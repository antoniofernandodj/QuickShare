use bson::doc;
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use mongodb::{Database, gridfs::GridFsBucket,};
use serde::Serialize;


#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub fs: GridFsBucket,
}

impl AppState {
    pub fn new(db: Database, fs: GridFsBucket) -> Self {
        AppState { db, fs }
    }
}


#[derive(Serialize)]
pub struct UploadResponse {
    pub file_id: String,
    pub download_url: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub detail: String,
}

pub enum AppError {
    InvalidObjectId,
    FileNotFound,
    UploadError(String),
    DatabaseError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message): (StatusCode, String) = match self {
            AppError::InvalidObjectId =>
                (StatusCode::BAD_REQUEST, "ID inválido".to_string()),
            AppError::FileNotFound =>
                (StatusCode::NOT_FOUND, "Arquivo não encontrado ou expirado".to_string()),
            AppError::UploadError(msg) =>
                (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::DatabaseError(msg) =>
                (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(ErrorResponse { detail: message })).into_response()
    }
}


#[derive(Serialize)]
pub struct FileInfo {
    pub _id: String,
    pub filename: String,
    pub expire_at: String,
}