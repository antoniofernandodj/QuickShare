use axum::{Router, extract::DefaultBodyLimit, routing::{get, post}};
use tower_http::cors::{Any, CorsLayer};
use crate::{handlers, models::AppState};


pub fn get_router(state: AppState, limit_size: usize) -> Router {

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/upload", post(handlers::upload_file))
        .route("/download/:file_id", get(handlers::download_file))
        .route("/download/", get(handlers::list_files))
        .layer(DefaultBodyLimit::max(limit_size)) // 100MB
        .with_state(state)
        .layer(cors);

    router
}
