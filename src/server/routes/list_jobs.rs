use axum::{Json, extract::State};

use crate::server::state::{AppState, Job};

/// GET /api/jobs
/// Returns the list of jobs
pub(in crate::server) async fn list_jobs(State(state): State<AppState>) -> Json<Vec<Job>> {
    let store = state.jobs.read().await;
    Json(store.jobs.values().rev().cloned().collect())
}
