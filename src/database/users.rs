use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc, from_bson, Bson, Document},
    Collection,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::{write, Display}};


pub enum LoginError {
    Message(String),
}
impl Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "Error Occured {}", msg),
        }
    }
}

use super::{cache, get_connection};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserAccout {
    pub name: String,
    pub ip_address: String,
    pub password: String,
    pub rank: String,
    pub file_path: String,
    pub patterns_solved: Vec<PatternInfo>,
    pub incomplete_pattern: Pattern,
    pub num_attempts: i32,
}

impl Display for UserAccout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"name : {}\nip_address:{}\npassword:{}\nrank:{}\nfile_path:{}\nincomplete_patter:{}\npatterns_solved:{:#?}",
    self.name,self.ip_address,self.password,self.rank,self.file_path,self.incomplete_pattern,self.patterns_solved
    )
    }
}
impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "general_rule:{}\nlevel:{}\npattern:{:?}",
            self.general_rule, self.level, self.pattern
        )
    }
}
impl Display for PatternInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pattern:{}\ntime_taken:{}",
            self.pattern, self.time_taken
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatternInfo {
    pub pattern: Pattern,
    pub time_taken: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pattern {
    pub general_rule: String,
    pub pattern: Vec<i32>,
    pub level: String,
    pub time_taken: i32,
    pub term_to_solve: i32,
    pub solved: bool,
    pub jeopardy: i32,
}

pub struct CurrentPlayer {
    pub user_account: UserAccout,
}
pub enum DifficultyLevel {
    Impossible,
    Hard,
    Medium,
    Easy,
}

pub async fn create_user_account(user_details: UserAccout) {
    let doc = doc! {
        "num_attempts":0,
        "solved":false,
        "name": &user_details.name,
        "ip_address": &user_details.ip_address,
        "rank": &user_details.rank,
        "file_path": &user_details.file_path,
        "incomplete_pattern": to_bson(&user_details.incomplete_pattern),
        "patterns_solved": to_bson(&user_details.patterns_solved),
        "password": &user_details.password,
       

    };
    let mut collection: Option<Collection<Document>> = Option::None;
    if let Ok(data) = super::get_connection().await {
        collection = Some(data.0);
    }

    save_document(&Ok(collection.unwrap()), &doc).await;
}
pub async fn update_user_account(user_details: UserAccout) {
    let filter = doc! { "ip_address": &user_details.ip_address };
    
    let update = doc! {
        "$set": {
            "file_path": &user_details.file_path,
            "incomplete_pattern":to_bson(&user_details.incomplete_pattern),
            "rank": &user_details.rank,
            "patterns_solved": to_bson(&user_details.patterns_solved),
            "num_attempts": &user_details.num_attempts,
        }
    };
   
   let cache = cache::GLOBAL_CACHE.lock().await;
   if !cache.is_empty(){
    let collection = cache.get_collection();
    if let Some(col) =collection{
     let res = col.update_one(filter, update, None).await;
     match res {
         Ok(_)=>{
            //  println!("Successfully updated the document.");
            // println!(".");
            },
         Err(_)=>{println!("No matching document found.");}
     }
    }
   }else {
       println!("cache is empty cant save ")
   }
     
}

fn formatter(value: &str, user: &Document) -> String {
    user.get(value)
        .unwrap()
        .to_string()
        .replace("\"", "")
        .trim()
        .to_string()
}

pub fn find_user(
    users: &Vec<Document>,
    username: &String,
    ip_address: &String,
) -> Option<(String)> {
    for user in users {
        let doc_username = user
            .get("name")
            .unwrap()
            .to_string()
            .replace("\"", "")
            .trim()
            .to_string();
        let doc_ip_adress = user
            .get("ip_address")
            .unwrap()
            .to_string()
            .replace("\"", "")
            .trim()
            .to_string();
        if doc_username.eq(username) || doc_ip_adress.eq(ip_address) {
            return Some(doc_username);
        }
    }
    return None;
}
pub fn find_logged_in_user(users: &Vec<Document>, ip_address: &String) -> Option<UserAccout> {
    for user in users {
        let doc_ip_adress = user
            .get("ip_address")
            .unwrap()
            .to_string()
            .replace("\"", "")
            .trim()
            .to_string();
        if doc_ip_adress.eq(ip_address) {
            let mut user_account = UserAccout {
                num_attempts:user.get("num_attempts").unwrap().as_i32().unwrap(),
                password: formatter("password", user),
                file_path: formatter("file_path", user),
                incomplete_pattern: match user.get("incomplete_pattern") {
                    Some(val) => match from_bson::<Pattern>(val.clone()) {
                        Ok(val) => val,
                        Err(err) => {
                            eprintln!("Failed to parse PatternInfo: {}", err);
                            continue;
                        }
                    },
                    None => {
                        continue;
                    }
                },
                ip_address: formatter("ip_address", user),
                name: formatter("name", user),
                rank: formatter("rank", user),
                patterns_solved: user
                    .get("patterns_solved")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|item| match from_bson::<PatternInfo>(item.clone()) {
                        Ok(pattern_info) => Some(pattern_info),
                        Err(e) => {
                            eprintln!("Failed to parse PatternInfo: {}", e);
                            None
                        }
                    })
                    .collect(),
            };
            return Some(user_account);
        }
    }
    None
}
pub fn login(
    users: &Vec<Document>,
    username: &String,
    password: &String,
) -> Result<bool, LoginError> {
    for user in users {
        // println!("comapring {} with {}", formatter("name", user), username);
        // println!(
        //     "comapring {} password with {} password",
        //     formatter("password", user),
        //     password
        // );
        if formatter("name", user) == (*username) {
            if formatter("password", user) == (*password) {
                return Ok(true);
            }
        }
    }

    return Err(LoginError::Message("Incorrect Password".to_string()));
}

pub fn get_all_usernames(docs: &Vec<Document>) -> Vec<String> {
    if docs.is_empty() {
        let empty_list: Vec<String> = Vec::new();
        return empty_list;
    }

    let usernames: Vec<String> = docs
        .iter()
        .filter_map(|doc| doc.get("name"))
        .map(|name| name.to_string().replace("\"", "").trim().to_string())
        .collect();

    usernames
}

fn to_bson<T>(value: &T) -> Bson
where
    T: Serialize,
{
    bson::to_bson(value).expect("Failed to convert to BSON")
}

async fn save_document(
    collection: &Result<Collection<Document>, mongodb::error::Error>,
    doc: &Document,
) {
    match collection {
        Ok(col) => {
            if let Err(error) = col.insert_one(doc, None).await {
                eprintln!("Error inserting document: {}", error);
            } else {
                println!("Account Created");
            }
        }
        Err(error) => {
            println!("Error retrieving collection: {}", error);
        }
    }
}
pub async fn update_user_document(
    collection: &Collection<Document>,
    user_account: &UserAccout,
) -> mongodb::error::Result<()> {
    let filter = doc! { "ip_address": &user_account.ip_address };

    // Update operation (set new password)
    let update = doc! {
        "$set": {
            "file_path": &user_account.file_path,
            "incomplete_pattern":to_bson(&user_account.incomplete_pattern),
            "num_attempts": &user_account.num_attempts,
            "rank": &user_account.rank,
            "patterns_solved": to_bson(&user_account.patterns_solved)
        }
    };

    // Perform the update
    let update_result = collection.update_one(filter, update, None).await?;

    if update_result.matched_count > 0 {
        println!("Successfully updated the document.");
    } else {
        println!("No matching document found.");
    }

    Ok(())
}
