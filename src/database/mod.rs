use futures_util::TryStreamExt;
use mongodb::{bson::Document, error::Error, options::ClientOptions, Client, Collection};
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub mod ip_address;
pub mod users;
static COLLECTION: Lazy<Mutex<Option<(Collection<Document>, Vec<Document>)>>> =
    Lazy::new(|| Mutex::new(None));

pub struct Col {
    collection: Collection<Document>,
}

async fn connection() -> Result<(Collection<Document>, Vec<Document>), Error> {
    let mongodb_uri = "mongodb+srv://wanewa:Wanewa%4012@cluster0.atsji.mongodb.net/?retryWrites=true&w=majority&appName=Cluster0";
    let client_options = ClientOptions::parse(mongodb_uri).await?;
    let client = Client::with_options(client_options)?;
    let database = client.database("GameStats");
    let collection = database.collection("GameStats");
    let mut docs: Vec<Document> = Vec::new();
    let mut cursor = collection.find(None, None).await?;
    while let Some(doc) = cursor.try_next().await? {
        docs.push(doc);
    }
    Ok((collection, docs))
}

pub async fn get_all_docs() -> Option<Vec<Document>> {
    if let Ok(con) = get_connection().await {
        let collection = con.0;
        let mut docs: Vec<Document> = Vec::new();

        let mut cursor = match collection.find(None, None).await {
            Ok(cursor) => cursor,
            Err(e) => {
                eprintln!("Error querying collection: {}", e);
                return None;
            }
        };
        while let Ok(Some(doc)) = cursor.try_next().await {
            docs.push(doc);
        }

        return Some(docs);
    }
    None
}
pub async fn get_connection() -> Result<(Collection<Document>, Vec<Document>), Error> {
    // let mut lock = COLLECTION.lock().unwrap();

    // if let Some((collection, docs)) = &*lock {
    //     return Ok((collection.clone(), docs.clone()));
    // }
    let (collection, docs) = connection().await?;

    // *lock = Some((collection.clone(), docs.clone()));
    Ok((collection, docs))
}
