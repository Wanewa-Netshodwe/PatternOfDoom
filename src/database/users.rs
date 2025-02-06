
use std::fmt::{write, Display};

use futures_util::{StreamExt, TryStreamExt};
use serde::{Serialize, Deserialize};
use mongodb::{bson::{self, doc, from_bson, Bson, Document}, Collection};

pub enum LoginError{
    Message(String)
}
impl Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f,"Error Occured {}", msg)

        }
    }
}

use super::get_connection;
#[derive(Serialize, Deserialize)]
pub struct UserAccout {
    pub name: String,
    pub ip_address: String,
    pub password: String,
    pub rank: String,
    pub file_path: String,
    pub patterns_solved: Vec<PatternInfo>,
    pub incomplete_pattern: Pattern,
    pub num_attempts:String
}

impl  Display for UserAccout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        
        write!(f,"name : {}\nip_address:{}\npassword:{}\nrank:{}\nfile_path:{}\nincomplete_patter:{}\npatterns_solved:{:#?}",
    self.name,self.ip_address,self.password,self.rank,self.file_path,self.incomplete_pattern,self.patterns_solved
    )
        
    }
}
impl  Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"general_rule:{}\nlevel:{}\npattern:{:?}", self.general_rule,self.level,self.pattern)
    }
}
impl  Display for PatternInfo {
fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f,"pattern:{}\ntime_taken:{}",self.pattern,self.time_taken)
}    
}


#[derive(Serialize, Deserialize,Debug)]
pub struct PatternInfo {
    pub   pattern: Pattern,
    pub  time_taken: String,
}

#[derive(Serialize, Deserialize,Debug)]
#[derive(Clone)]
pub struct Pattern  {
   pub general_rule: String,
   pub pattern: Vec<u64>,
   pub level: String,
   pub time_taken:String
}
pub struct CurrentPlayer{
    pub user_account :UserAccout
}



pub async  fn create_user_account(user_details: UserAccout) {
    // Construct a BSON document using the `doc!` macro
    let doc = doc! {
        "num_attempts":"0",
        "name": &user_details.name,
        "ip_address": &user_details.ip_address,
        "rank": &user_details.rank,
        "file_path": &user_details.file_path,
        "incomplete_pattern": to_bson(&user_details.incomplete_pattern),
        "patterns_solved": to_bson(&user_details.patterns_solved),
        "password": &user_details.password
    };
    let mut collection :Option<Collection<Document>> = Option::None;
    if let Ok(data) = super::get_connection().await{
        collection =Some(data.0);
    }
   
     save_document(&Ok(collection.unwrap()), &doc).await;

}
fn formatter(value:&str,user:&Document)->String{
    user.get(value).unwrap().to_string().replace("\"", "").trim().to_string()  
}

pub fn find_user(users:&Vec<Document>,username:&String,ip_address:&String)->Option<(String)>{
    for user in users{
        let doc_username =user.get("name").unwrap().to_string().replace("\"", "").trim().to_string();
        let doc_ip_adress =user.get("ip_address").unwrap().to_string().replace("\"", "").trim().to_string();
        if  doc_username.eq(username) || doc_ip_adress.eq(ip_address) {
            return Some(doc_username)
        }
      }
      return None
}
pub fn find_logged_in_user(users:& Vec<Document>,ip_address:&String)->  Option<UserAccout>{

    for user in users{
        let doc_ip_adress =user.get("ip_address").unwrap().to_string().replace("\"", "").trim().to_string();
        if doc_ip_adress.eq(ip_address) {
            let user_account = UserAccout{
                num_attempts:formatter("num_attempts", user),
                password:formatter("password", user),
                file_path: formatter("file_path", user),
                incomplete_pattern: match user.get("incomplete_pattern") {
                    Some(val) => {
                        match from_bson::<Pattern>(val.clone()) {
                            Ok(val)=>{val},
                            Err(err)=>{
                                eprintln!("Failed to parse PatternInfo: {}", err);
                                continue;
                            }
                        }
                    },
                    None => {
                    continue;
                }
                },
                 ip_address: formatter("ip_address", user),
                name: formatter("name", user),
                rank: formatter("rank", user),
                patterns_solved: user.get("patterns_solved").unwrap().as_array().unwrap().iter()
                .filter_map(|item| match from_bson::<PatternInfo>(item.clone()) {
                    Ok(pattern_info) => Some(pattern_info),
                    Err(e) => {
                        eprintln!("Failed to parse PatternInfo: {}", e);
                        None
                    }
                })
                .collect(),   
            };
                return Some(user_account)
        }
      }
      None

}
pub fn login(users:&Vec<Document>,username:&String,password:&String)->Result<bool,LoginError>{
        for user in users{
            println!("comapring {} with {}",formatter("name", user),username);
            println!("comapring {} password with {} password",formatter("password", user),password);
            if formatter("name", user) ==(*username){
                if formatter("password", user) == (*password){
                    
                    return  Ok(true)
                }
            }
        }
        
        return Err(LoginError::Message("Incorrect Password".to_string())) 
}


pub fn get_all_usernames(docs: &Vec<Document>) -> Vec<String> {
    if docs.is_empty() {
        let empty_list :Vec<String> = Vec::new();
        return empty_list
    }

    let usernames: Vec<String> = docs.iter()
        .filter_map(|doc| doc.get("name"))
        .map(|name| name.to_string().replace("\"", "").trim().to_string())
        .collect();

    usernames
}


// Helper function to convert values to Bson
fn to_bson<T>(value: &T) -> Bson
where
    T: Serialize,
{
    bson::to_bson(value).expect("Failed to convert to BSON")
}
pub enum DifficultyLevel{
    Impossible,
    Hard,
    Medium,
    Easy
}
    
async fn save_document(collection: &Result<Collection<Document>, mongodb::error::Error>, doc: &Document) {
    match collection {
        Ok(col) => {
            // Insert the document asynchronously
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
