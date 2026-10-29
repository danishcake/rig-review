use std::{sync::atomic::Ordering, time::SystemTime};

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    server::state::{AppState, Job, JobStatus},
    source::gitlab::gitlab_review_source::GitlabReviewSource,
};

#[derive(Deserialize)]
pub(in crate::server) struct SubmitJobRequest {
    merge_request_url: String,
}

#[derive(Serialize)]
struct EnqueuedResponse {
    job_id: u64,
    status: JobStatus,
}

/// POST /api/jobs
/// Queues a review for a manually submitted merge request URL.
pub(in crate::server) async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<SubmitJobRequest>,
) -> Response {
    let merge_request_url = payload.merge_request_url.trim();
    if !GitlabReviewSource::is_valid_mr_url(merge_request_url, &state.gitlab_config) {
        return (StatusCode::BAD_REQUEST, "Invalid merge request URL").into_response();
    }

    let id = state.next_job_id.fetch_add(1, Ordering::Relaxed);
    let job = Job {
        id,
        merge_request_url: merge_request_url.to_owned(),
        status: JobStatus::Queued,
        created_at: SystemTime::now(),
        started_at: None,
        completed_at: None,
        error: None,
    };

    let mut store = state.jobs.write().await;
    store.jobs.insert(id, job);
    match state.sender.try_send(id) {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(EnqueuedResponse {
                job_id: id,
                status: JobStatus::Queued,
            }),
        )
            .into_response(),
        Err(_) => {
            store.jobs.remove(&id);
            (StatusCode::SERVICE_UNAVAILABLE, "Review queue is full").into_response()
        }
    }
}
