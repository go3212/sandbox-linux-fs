pub mod archive;
pub mod files;
pub mod health;
pub mod repos;
pub mod shell;

use axum::routing::{delete, get, head, patch, post};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::auth::ApiKeyLayer;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let api_key = state.config.api_key.clone();
    let max_upload = state.config.max_upload_size as usize;

    // Public routes (no auth)
    let public_routes = Router::new().route("/health", get(health::health));

    // Authenticated API routes
    let api_routes = Router::new()
        .route("/status", get(health::status))
        // Repos
        .route("/repos", post(repos::create_repo))
        .route("/repos", get(repos::list_repos))
        .route("/repos/{repo_id}", get(repos::get_repo))
        .route("/repos/{repo_id}", patch(repos::update_repo))
        .route("/repos/{repo_id}", delete(repos::delete_repo))
        // Files
        .route("/repos/{repo_id}/files", get(files::list_files))
        .route(
            "/repos/{repo_id}/files/{*file_path}",
            post(files::upload_file),
        )
        .route(
            "/repos/{repo_id}/files/{*file_path}",
            get(files::download_file),
        )
        .route(
            "/repos/{repo_id}/files/{*file_path}",
            head(files::head_file),
        )
        .route(
            "/repos/{repo_id}/files/{*file_path}",
            delete(files::delete_file),
        )
        .route("/repos/{repo_id}/files-move", post(files::move_file))
        .route("/repos/{repo_id}/files-copy", post(files::copy_file))
        // Shell
        .route("/repos/{repo_id}/exec", post(shell::exec_command))
        // Archive
        .route("/repos/{repo_id}/archive", post(archive::create_archive))
        .layer(ApiKeyLayer::new(api_key));

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Request ID
    let x_request_id = http::HeaderName::from_static("x-request-id");

    Router::new()
        .merge(public_routes)
        .nest("/api/v1", api_routes)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(max_upload))
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(
            x_request_id,
            MakeRequestUuid,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
