use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::server::state::{AppState, Job};

/// GET /api/jobs/{id}
/// Returns details about a single job
pub(in crate::server) async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Job>, StatusCode> {
    state
        .jobs
        .read()
        .await
        .jobs
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
