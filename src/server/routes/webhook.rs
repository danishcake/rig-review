use std::{sync::atomic::Ordering, time::SystemTime};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    server::state::{AppState, Job, JobStatus},
    source::gitlab::gitlab_review_source::GitlabReviewSource,
};

#[derive(Debug, Deserialize)]
pub(in crate::server) struct GitLabWebhook {
    object_kind: Option<String>,
    object_attributes: Option<MergeRequestAttributes>,
}

#[derive(Debug, Deserialize)]
struct MergeRequestAttributes {
    action: Option<String>,
    url: Option<String>,
}

#[derive(Serialize)]
struct EnqueuedResponse {
    job_id: u64,
    status: JobStatus,
}

/// POST /webhooks/gitlab
/// The webhook route to be called from Gitlab
pub(in crate::server) async fn gitlab_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<GitLabWebhook>,
) -> Response {
    // Check the webhook token is correct
    if !valid_webhook_token(&headers, &state.webhook_secret) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Get the merge request URL
    let Some(merge_request_url) = open_merge_request_url(&payload) else {
        return StatusCode::NO_CONTENT.into_response();
    };

    // Validate the URL so we don't go posting tokens at attacker controlled domains
    if !GitlabReviewSource::is_valid_mr_url(merge_request_url, &state.gitlab_config) {
        return StatusCode::BAD_REQUEST.into_response();
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
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

/// Extracts the URL from the request body
fn open_merge_request_url(payload: &GitLabWebhook) -> Option<&str> {
    if payload.object_kind.as_deref() != Some("merge_request") {
        return None;
    }

    let attributes = payload.object_attributes.as_ref()?;
    if attributes.action.as_deref() != Some("open") {
        return None;
    }

    attributes.url.as_deref()
}

/// Determines if the request headers contai the required webhook token
fn valid_webhook_token(headers: &HeaderMap, secret: &str) -> bool {
    headers
        .get("x-gitlab-token")
        .is_some_and(|token| token.as_bytes() == secret.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_only_accepts_open_merge_request_events() {
        let open = GitLabWebhook {
            object_kind: Some("merge_request".to_owned()),
            object_attributes: Some(MergeRequestAttributes {
                action: Some("open".to_owned()),
                url: Some("https://gitlab.example.com/group/project/-/merge_requests/1".to_owned()),
            }),
        };
        assert_eq!(
            open_merge_request_url(&open),
            Some("https://gitlab.example.com/group/project/-/merge_requests/1")
        );

        let closed = GitLabWebhook {
            object_kind: Some("merge_request".to_owned()),
            object_attributes: Some(MergeRequestAttributes {
                action: Some("close".to_owned()),
                url: Some("https://gitlab.example.com/group/project/-/merge_requests/1".to_owned()),
            }),
        };
        assert_eq!(open_merge_request_url(&closed), None);
    }

    #[test]
    fn webhook_requires_a_matching_gitlab_token_header() {
        let mut headers = HeaderMap::new();
        assert!(!valid_webhook_token(&headers, "expected"));

        headers.insert("x-gitlab-token", "incorrect".parse().unwrap());
        assert!(!valid_webhook_token(&headers, "expected"));

        headers.insert("x-gitlab-token", "expected".parse().unwrap());
        assert!(valid_webhook_token(&headers, "expected"));
    }
}
