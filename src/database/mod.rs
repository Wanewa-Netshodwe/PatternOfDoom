use futures_util::TryStreamExt;
use mongodb::{bson::Document, error::Error, options::ClientOptions, Client, Collection};
use once_cell::sync::Lazy;
use std::sync::Mutex;  // Required for thread-safe locking

pub mod users;
pub mod ip_address;

// Static variables to hold the collection and documents.
static COLLECTION: Lazy<Mutex<Option<(Collection<Document>, Vec<Document>)>>> = Lazy::new(|| Mutex::new(None));

pub struct Col {
    collection: Collection<Document>,
}

/// Asynchronously connects to MongoDB, inserts a test document, and returns the `Collection` object.
async fn connection() -> Result<(Collection<Document>, Vec<Document>), Error> {
    // MongoDB connection URL with URL-encoded password
    let mongodb_uri = "mongodb+srv://wanewa:Wanewa%4012@cluster0.atsji.mongodb.net/?retryWrites=true&w=majority&appName=Cluster0";

    // Parse the MongoDB URI to configure the client
    let client_options = ClientOptions::parse(mongodb_uri).await?;

    // Create a MongoDB client instance
    let client = Client::with_options(client_options)?;

    // Get a reference to a database
    let database = client.database("GameStats"); // Replace with your database name
    let collection = database.collection("GameStats");

    // Create a vector to store the documents
    let mut docs: Vec<Document> = Vec::new();

    // Fetch all documents from the collection
    let mut cursor = collection.find(None, None).await?;

    // Iterate over the cursor using `try_next()`
    while let Some(doc) = cursor.try_next().await? {
        docs.push(doc);
    }

    // Return the collection and documents
    Ok((collection, docs))
}

/// A function to get the MongoDB collection and documents. It establishes the connection only once.
pub async fn get_connection() -> Result<(Collection<Document>, Vec<Document>), Error> {
    // Check if the connection has already been established and stored
    let mut lock = COLLECTION.lock().unwrap();

    if let Some((collection, docs)) = &*lock {
        // If already initialized, return the stored collection and documents
        return Ok((collection.clone(), docs.clone()));
    }

    // If not initialized, establish a connection
    let (collection, docs) = connection().await?;

    // Store the established connection and documents in the static variable
    *lock = Some((collection.clone(), docs.clone()));

    // Return the newly established collection and documents
    Ok((collection, docs))
}
