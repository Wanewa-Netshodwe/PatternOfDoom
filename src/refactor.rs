mod database;
use std::io::{self};
use std::net::IpAddr;
use ctrlc;
use database::users::{find_user, Pattern, PatternInfo, UserAccout};
use mongodb::bson::Document;
mod refactor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
mod ip_address;
use indicatif::ProgressStyle;
use indicatif::ProgressBar;


fn main() {
   
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let logged_in = false;

    
    ctrlc::set_handler(move || {
        println!("\nCtrl+C pressed! Exiting...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");
    let client_address = Arc::new(Mutex::new(None::<IpAddr>));
    let client_address_clone = Arc::clone(&client_address);
        thread::spawn(move || {
            if let Some(addr)=ip_address::get_local_ip(){
                let mut client_address_lock = client_address_clone.lock().unwrap();
                *client_address_lock = Some(addr);

            }else{
                println!("failed to get ip address");
            }
        }).join().expect("msg");
        let client_address_lock = client_address.lock().unwrap();
        let mut  final_client_address =String::new();
        match *client_address_lock {
           Some(addr)=>{
            final_client_address = addr.to_string();
           },
           None =>()
            
        }
       
    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        if(logged_in){
            todo!();
        }else{
            println!("Enter 1: to connect");
            let mut choice = String::new();
            io::stdin().read_line(&mut choice).expect("Error reading input");
            let choice: u8 = match choice.trim().parse() {
                Ok(num) => num,
                Err(_) => {
                    println!("Invalid input. Please enter a number.");
                    continue;
                }
            };
    
            if choice == 1 {
               
                let connection_complete = Arc::new(AtomicBool::new(false));
                let cc = connection_complete.clone();
    
                
                 let pb = ProgressBar::new(100);
                 pb.set_style(
                     ProgressStyle::with_template("{bar:40.white} {percent}% {elapsed} ({eta})")
                         .unwrap()
                         .progress_chars("█▓▒░"), 
                 );
    
     
               
                 let spinner_thread = thread::spawn(move || {
                     let mut progress = 0;
     
                     while !cc.load(Ordering::SeqCst) {
                         pb.inc(1); 
                         progress += 1;
                         thread::sleep(time::Duration::from_millis(390)); 
                     }
                     if progress < 100 {
                        for _ in progress..101{
                            pb.inc(1); 
                             progress+= 1;
                            thread::sleep(time::Duration::from_micros(100));
                           
                        } 
                     }
                     pb.finish_with_message("Connected to database!"); 
                 });
    
              
                let connection_result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(database::connection());
    
                // Signal that the connection is complete
                connection_complete.store(true, Ordering::SeqCst);
    
                spinner_thread.join().unwrap();
                println!("Connected To Database ");
                let account_creation = Arc::new(AtomicBool::new(false));
                let ac = account_creation.clone();
                let pb2 = ProgressBar::new(100);
    
                               
                                 pb2.set_style(
                                     ProgressStyle::with_template("{spinner:.green} {msg}")
                                         .unwrap()
                                         .tick_chars("|/-\\") 
                                 );
                             
                                
                                 pb2.set_message("Loading Please Wait ...");
                                 let spinner_thread_names = thread::spawn(move||{
                                    while !ac.load(Ordering::SeqCst) {
                                        thread::sleep(time::Duration::from_millis(100)); 
                                        pb2.tick(); 
                                     }
                                 });
              let mut users :Vec<Document> = Vec::new();

                if let Ok(users_docs) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(database::users::get_all_users()){
                        users = users_docs
                    }else{
                        let empty_list :Vec<Document> = Vec::new();
                        users = empty_list;
                    }
               
                let usernames = database::users::get_all_usernames(&users);
                account_creation.store(true, Ordering::SeqCst);
                spinner_thread_names.join().unwrap();
                println!("list of usernames in the db {usernames:?}");
                let mut choice = String::new();
                println!("Enter 1 to Login in \n Enter 2 to Creat new Account ");
                io::stdin().read_line(&mut choice).expect("Error reading Line");
                let choice:u64 = choice.trim().parse().expect("eroor parsing");
                match  choice {
                    
                    1=>{
                        println!("Login to be implemented");
                       
                    }
                    2=>{
                        let additional_info = ||->(&Vec<Document>,&String){
                            (&users,&final_client_address)
                        };
                        let account_creation = Arc::new(AtomicBool::new(false));
                        let ac = account_creation.clone();
                        
                        let mut username = String::new();
                        let (user_docs, client_address) = additional_info();
                println!("Enter Username ");
                let mut username_db = String::new();
                io::stdin().read_line(&mut username).expect("Error reading Line");
                if let Some(name) =find_user(user_docs, &username, client_address){
                    username_db = name;
                }
                let mut valid = usernames.contains(&username.trim().to_string()) ;
                        while valid {
                            username.clear();
                            print!("Username {}",username);
                            print!("is  already taken try another one ");
                            println!("Enter Username ");
                            io::stdin().read_line(&mut username).expect("Error reading Line");
                            valid = usernames.contains(&username.trim().to_string());
                        }
                        
                let mut password = String::new();
                println!("Enter Password ");
                io::stdin().read_line(&mut password).expect("Error reading Line");
                println!("Processing");
    
                        match connection_result {
                            Ok(_) =>{
                                 
                                 let pat =Pattern{
                                    general_rule:String::new(),
                                    level:String::new(),
                                    pattern:vec![]
                                 };
                                 let pat_clone = pat.clone();
                                 
                                 let pi =PatternInfo{
                                    pattern:pat,
                                    time_taken:String::new()
                                 };
                                 let user =UserAccout{
                                    file_path:String::new(),
                                    incomplete_pattern:pat_clone,
                                    ip_address:final_client_address.clone(),
                                    name:username.clone(),
                                    password:password.clone(),
                                    patterns_solved:vec![pi],
                                    rank:String::from("Starterpack")
                                 };
                                 let pb = ProgressBar::new(100);
    
                               
                                 pb.set_style(
                                     ProgressStyle::with_template("{spinner:.green} {msg}")
                                         .unwrap()
                                         .tick_chars("|/-\\") 
                                 );
                             
                                
                                 pb.set_message("Loading...");
                                 let spinner_thread = thread::spawn(move||{
                                    while !ac.load(Ordering::SeqCst) {
                                        thread::sleep(time::Duration::from_millis(100)); 
                                        pb.tick(); 
                                     }
                                 });
                                 let find_ip = ||->&String{
                                        &final_client_address
                                 }();
                                 let existing_user = ||->String{
                                    username_db
                                 }();
                                
                                let user_doc=database::users::find_user(&users, &existing_user, find_ip);
                                match user_doc {
                                    None=>{
                                        tokio::runtime::Runtime::new().unwrap().block_on(database::users::create_user_account(&user));
                                        account_creation.store(true, Ordering::SeqCst);
                                        spinner_thread.join().unwrap();  
                                    },
                                    Some(username)=>{
                                        account_creation.store(true, Ordering::SeqCst);
                                        spinner_thread.join().unwrap();  
                                        print!("Account Found For ");
                                        print!("{}",username);
                                        print!(" Enter Password To Countinue : ");
                                        let mut password2 = String::new();
                                        io::stdin().read_line(&mut password2).expect("Error reading Line");
                                        let res = database::users::login(&users, &username, &password2);
                                        
                                       
                            
                                    }
                                    
                                }
                                 
                                
                                }
                            
                            Err(e) => println!("An error occurred: {}", e),
                        }
                    }
                    _=>()
                    
                }
    
    
            } else {
                println!("Database connection rejected");
            }
        }
      
    }
}