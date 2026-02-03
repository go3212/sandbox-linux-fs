use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn health() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn status(State(state): State<AppState>) -> Json<Value> {
    let repo_count = state.repos.len();
    let total_size: u64 = state
        .repos
        .iter()
        .map(|r| r.value().current_size_bytes)
        .sum();
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.start_time)
        .num_seconds();

    Json(json!({
        "data": {
            "repo_count": repo_count,
            "total_size_bytes": total_size,
            "uptime_seconds": uptime,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "error": null
    }))
}
