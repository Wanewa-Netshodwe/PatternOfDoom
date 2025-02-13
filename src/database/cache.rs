use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use mongodb::Collection;
use mongodb::bson::Document;

pub struct Cache {
    pub collection: HashMap<String, Collection<Document>>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            collection: HashMap::new(),
        }
    }

    pub fn set(&mut self, collection: Collection<Document>) {
        self.collection.insert("collection".to_string(), collection);
    }

    pub fn is_empty(&self) -> bool {
        self.collection.is_empty()
    }

    pub fn get_collection(&self) -> Option<&Collection<Document>> {
        self.collection.get("collection")
    }
}

pub static GLOBAL_CACHE: Lazy<Mutex<Cache>> = Lazy::new(|| Mutex::new(Cache::new()));
