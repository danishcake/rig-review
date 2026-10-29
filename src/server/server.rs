//! HTTP server, GitLab webhook handler, and in-memory review job queue.

use std::{
    sync::{Arc, atomic::AtomicU64},
    time::SystemTime,
};

use axum::{
    Router,
    routing::{get, post},
};
use tokio::{
    net::TcpListener,
    sync::{Mutex, RwLock, mpsc},
};

use crate::{
    config::{Config, NetworkConfig},
    error::RigReviewError,
    review::{
        base::ReviewEngine,
        rig::{config::RigReviewEngineConfig, engine::RigReviewEngine},
    },
    server::{
        routes::{
            get_job::get_job, index::index, list_jobs::list_jobs, submit_job::submit_job,
            webhook::gitlab_webhook,
        },
        state::{AppState, JobStatus, JobStore},
    },
    sink::base::ReviewSink,
    source::{
        base::ReviewSource,
        gitlab::{config::GitlabReviewSourceConfig, gitlab_review_source::GitlabReviewSource},
    },
};

/// Start the HTTP server and its review workers.
pub async fn run(config: Config) -> Result<(), RigReviewError> {
    let Some(server_config) = config.server else {
        return Err(RigReviewError::MissingConfiguration("server".into()));
    };
    let Some(gitlab_config) = config.gitlab else {
        return Err(RigReviewError::MissingConfiguration("gitlab".into()));
    };

    let webhook_secret = server_config.webhook_secret.to_owned();

    // Require that the host is specified
    if gitlab_config.host.is_none() {
        return Err(RigReviewError::MissingConfiguration("gitlab.host".into()));
    };

    let jobs = Arc::new(RwLock::new(JobStore::default()));
    let (sender, receiver) = mpsc::channel(server_config.queue_capacity.get());
    let receiver = Arc::new(Mutex::new(receiver));

    // Spawn workers for performing the job reviews
    for _ in 0..server_config.worker_count.get() {
        tokio::spawn(run_worker(
            config.rig_review_engine.clone(),
            gitlab_config.clone(),
            config.network.clone(),
            jobs.clone(),
            receiver.clone(),
            server_config.job_history_limit.get(),
        ));
    }

    let state = AppState {
        gitlab_config: gitlab_config.clone(),
        jobs,
        sender,
        next_job_id: Arc::new(AtomicU64::new(1)),
        webhook_secret: Arc::from(webhook_secret),
    };

    // Define the routes
    let app = Router::new()
        .route("/", get(index))
        .route("/api/jobs", get(list_jobs).post(submit_job))
        .route("/api/jobs/{id}", get(get_job))
        .route("/webhooks/gitlab", post(gitlab_webhook))
        .with_state(state);

    // Start the server
    let listener =
        TcpListener::bind(format!("{}:{}", server_config.host, server_config.port)).await?;
    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutdown signal received");
        })
        .await?;
    Ok(())
}

async fn run_worker(
    review_engine_config: RigReviewEngineConfig,
    gitlab_config: GitlabReviewSourceConfig,
    network_config: NetworkConfig,
    jobs: Arc<RwLock<JobStore>>,
    receiver: Arc<Mutex<mpsc::Receiver<u64>>>,
    history_limit: usize,
) {
    loop {
        let id = {
            let mut receiver = receiver.lock().await;
            receiver.recv().await
        };
        let Some(id) = id else {
            break;
        };

        let merge_request_url = {
            let mut store = jobs.write().await;
            let Some(job) = store.jobs.get_mut(&id) else {
                continue;
            };
            job.status = JobStatus::Running;
            job.started_at = Some(SystemTime::now());
            job.merge_request_url.clone()
        };

        let result = perform_review(
            &review_engine_config,
            &gitlab_config,
            &network_config,
            &merge_request_url,
        )
        .await;
        if let Err(error) = &result {
            tracing::error!(job_id = id, error = %error, "review job failed");
        }

        let mut store = jobs.write().await;
        if let Some(job) = store.jobs.get_mut(&id) {
            job.completed_at = Some(SystemTime::now());
            match result {
                Ok(()) => job.status = JobStatus::Succeeded,
                Err(_) => {
                    job.status = JobStatus::Failed;
                    job.error = Some("Review failed; see server logs for details.".to_owned());
                }
            }
        }
        trim_job_history(&mut store, history_limit);
    }
}

async fn perform_review(
    review_engine_config: &RigReviewEngineConfig,
    gitlab_config: &GitlabReviewSourceConfig,
    network_config: &NetworkConfig,
    merge_request_url: &str,
) -> Result<(), RigReviewError> {
    let source = GitlabReviewSource::new(merge_request_url, gitlab_config, network_config).await?;
    let sink: Arc<Mutex<dyn ReviewSink>> = Arc::new(Mutex::new(source.create_sink()));
    let source: Arc<Mutex<dyn ReviewSource>> = Arc::new(Mutex::new(source));

    let mut engine = RigReviewEngine::new(review_engine_config, network_config);
    engine.review(source, vec![sink]).await?;
    Ok(())
}

fn trim_job_history(store: &mut JobStore, history_limit: usize) {
    while store.jobs.len() > history_limit {
        let oldest_finished_job = store
            .jobs
            .iter()
            .find(|(_, job)| matches!(job.status, JobStatus::Succeeded | JobStatus::Failed))
            .map(|(id, _)| *id);
        let Some(id) = oldest_finished_job else {
            break;
        };
        store.jobs.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::Job;

    fn job(id: u64, status: JobStatus) -> Job {
        Job {
            id,
            merge_request_url: format!(
                "https://gitlab.example.com/group/project/-/merge_requests/{id}"
            ),
            status,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    #[test]
    fn history_pruning_keeps_active_jobs_and_recent_finished_jobs() {
        let mut store = JobStore::default();
        store.jobs.insert(1, job(1, JobStatus::Succeeded));
        store.jobs.insert(2, job(2, JobStatus::Running));
        store.jobs.insert(3, job(3, JobStatus::Failed));

        trim_job_history(&mut store, 2);

        assert!(!store.jobs.contains_key(&1));
        assert!(store.jobs.contains_key(&2));
        assert!(store.jobs.contains_key(&3));
    }
}
