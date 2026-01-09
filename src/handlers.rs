use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::{AsyncReadExt, AsyncWriteExt};
use bson::{doc, oid::ObjectId, DateTime as BsonDateTime};
use chrono::{Duration, Utc};
use mongodb::{
    Database,
    IndexModel,
    options::{
        IndexOptions,
        GridFsUploadOptions
    }
};


use crate::{database::get_collection, models::{AppError, AppState, UploadResponse}};




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
