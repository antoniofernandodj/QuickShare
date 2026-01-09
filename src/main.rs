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
