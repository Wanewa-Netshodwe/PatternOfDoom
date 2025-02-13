use futures_util::TryStreamExt;
use mongodb::{bson::Document, error::Error, options::ClientOptions, Client, Collection};
use once_cell::sync::Lazy;
use std::sync::Mutex;
pub mod ip_address;
pub mod users;
pub mod cache;

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
    let cache = cache::GLOBAL_CACHE.lock().await;
   if !cache.is_empty(){
    let collection = cache.get_collection();
    if let Some(col) =collection{
    
        let collection = col;
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
    
   }else {
       return None;
   }
   }else {
    return None
}
}
    
pub async fn get_connection() -> Result<(Collection<Document>, Vec<Document>), Error> {
    let (collection, docs) = connection().await?;
    let mut  cache = cache::GLOBAL_CACHE.lock().await;
    cache.set(collection.clone());
    drop(cache);
    Ok((collection, docs))
}
