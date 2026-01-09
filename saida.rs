////// Arquivo: ./src/routes.rs
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


////// Arquivo: ./src/models.rs
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

////// Arquivo: ./src/main.rs
mod routes;
mod handlers;
mod models;
mod database;

use std::env;

use axum::Router;
use mongodb::{Database, GridFsBucket};
use tokio::net::TcpListener;
use crate::{
    database::get_db,
    models::AppState,
    routes::get_router
};


#[tokio::main]
async fn main() {
    let db: Database = get_db("files_db").await;
    let fs: GridFsBucket = db.gridfs_bucket(None);

    handlers::create_ttl_index(&db).await;

    let state: AppState = AppState::new(db, fs);
    const SIZE_100_MB: usize = 100 * 1024 * 1024;
    let router: Router = get_router(state, SIZE_100_MB);
    let port = env::var("PORT")
        .unwrap_or(String::from("7777"));

    let addr = format!("0.0.0.0:{}", port);
    let listener: TcpListener =
        TcpListener::bind(&addr)
        .await
        .unwrap();

    println!("Servidor rodando em {}", &addr);

    axum::serve(listener, router).await.unwrap();
}


////// Arquivo: ./src/database.rs
use dotenvy::dotenv;
use std::env;
use mongodb::{Client, Database, options::ClientOptions, Collection};


pub async fn get_db(database_name: &str) -> Database {
    dotenv().ok();

    let mongo_uri = env::var("MONGO_URI")
        .expect("MONGO_URI não definida");

    let client_options =
        ClientOptions::parse(mongo_uri)
            .await
            .expect("Falha ao parsear URI do MongoDB");

    let client =
        Client::with_options(client_options)
            .expect("Falha ao criar cliente MongoDB");

    let db = client.database(database_name);

    db
}


pub fn get_collection(db: &Database) -> Collection<bson::Document> {
    let col: Collection<bson::Document> = db.collection::<bson::Document>("fs.files");

    col
}

////// Arquivo: ./src/handlers.rs
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::{AsyncReadExt, AsyncWriteExt};
use bson::{doc, oid::ObjectId, DateTime as BsonDateTime};
use futures::TryStreamExt;
use chrono::{Duration, Utc};
use mongodb::{
    Database,
    IndexModel,
    options::{
        IndexOptions,
        GridFsUploadOptions
    }
};


use crate::{database::get_collection, models::{FileInfo, AppError, AppState, UploadResponse}};




// Criar índice TTL para expiração automática
pub async fn create_ttl_index(db: &Database) {
    let collection = get_collection(db);

    let index = IndexModel::builder()
        .keys(doc! { "expireAt": 1 })
        .options(
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
        )
        .build();

    collection
        .create_index(index, None)
        .await
        .expect("Falha ao criar índice TTL");

    println!("Índice TTL criado com sucesso");
}

// Handler de upload
pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::UploadError(e.to_string()))?
    {
        let filename = field
            .file_name()
            .unwrap_or("unknown")
            .to_string();

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        // Ler dados do arquivo
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::UploadError(e.to_string()))?;

        let options = GridFsUploadOptions::builder()
            .metadata(doc! { "content_type": content_type })
            .build();

        // Upload para GridFS
        let mut upload_stream = state
            .fs
            .open_upload_stream(filename.clone(), options);

        upload_stream
            .write_all(&data)
            .await
            .map_err(|e| AppError::UploadError(e.to_string()))?;

        upload_stream
            .close()
            .await
            .map_err(|e| AppError::UploadError(e.to_string()))?;

        let file_id = upload_stream.id();

        // Adicionar campo TTL (expira em 1 dia)
        let expire_at = Utc::now() + Duration::days(1);
        let bson_expire_at = BsonDateTime::from_millis(expire_at.timestamp_millis());

        let collection = state.db.collection::<bson::Document>("fs.files");
        collection
            .update_one(
                doc! { "_id": file_id.clone() },
                doc! { "$set": { "expireAt": bson_expire_at } },
                None,
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let file_id_str = file_id.as_object_id().unwrap().to_hex();
        let download_url = format!("/download/{}", file_id_str);

        return Ok(Json(UploadResponse {
            file_id: file_id_str,
            download_url,
            expires_at: expire_at.to_rfc3339(),
        }));
    }

    Err(AppError::UploadError("Nenhum arquivo enviado".to_string()))
}

// Handler de download
pub async fn download_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Response, AppError> {
    // Validar ObjectId
    let object_id = ObjectId::parse_str(&file_id).map_err(|_| AppError::InvalidObjectId)?;

    let files = state.db.collection::<bson::Document>("fs.files");

    let file_doc = files
        .find_one(doc! { "_id": &object_id }, None)
        .await
        .map_err(|_| AppError::FileNotFound)?
        .ok_or(AppError::FileNotFound)?;

    let content_type = file_doc
        .get_document("metadata")
        .ok()
        .and_then(|m| m.get_str("content_type").ok())
        .unwrap_or("application/octet-stream");

    let filename = file_doc
        .get_str("filename")
        .unwrap_or("download");

    let mut download_stream = state
        .fs
        .open_download_stream(object_id.into())
        .await
        .map_err(|_| AppError::FileNotFound)?;

    // Ler todo o conteúdo
    let mut buffer = Vec::new();
    download_stream
        .read_to_end(&mut buffer)
        .await
        .map_err(|_| AppError::FileNotFound)?;

    // Retornar resposta com headers apropriados
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", content_type),
            (
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        buffer,
    )
        .into_response())
}



/// Handler que lista todos os arquivos armazenados em GridFS
pub async fn list_files(State(state): State<AppState>) -> Result<Json<Vec<FileInfo>>, AppError> {
    let collection = state.db.collection::<bson::Document>("fs.files");

    let mut cursor = collection
        .find(None, None)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut files = Vec::new();

    while let Some(file_doc) = cursor
        .try_next()
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
    {
        let id_str = file_doc
            .get_object_id("_id")
            .map(|oid| oid.to_hex())
            .unwrap_or_else(|_| "unknown".to_string());

        let filename = file_doc
            .get_str("filename")
            .unwrap_or("unknown")
            .to_string();

        let expire_at = file_doc
            .get_datetime("expireAt")
            .map(|dt| {
                let chrono_dt = chrono::DateTime::<Utc>::from(*dt);
                chrono_dt.to_rfc3339()
            })
            .unwrap_or_else(|_| "unknown".to_string());

        files.push(FileInfo {
            _id: id_str,
            filename,
            expire_at,
        });
    }

    Ok(Json(files))
}

////// Arquivo: ./saida.rs


