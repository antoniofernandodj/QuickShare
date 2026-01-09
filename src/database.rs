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