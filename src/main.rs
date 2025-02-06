mod database;
use std::io::{self, Write};
use std::net::IpAddr;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration};

use ctrlc;
use database::users::{find_user, Pattern, PatternInfo, UserAccout};
use figlet_rs::FIGfont;
use mongodb::bson::Document;
use indicatif::ProgressStyle;
use indicatif::ProgressBar;
use database::users::DifficultyLevel;
use rand::Rng;
enum LoadingIndicator {
    ProgressBar,
    Spinner,
}
fn generate_sequence(level: &DifficultyLevel) -> Vec<i32> {
    let mut rng = rand::thread_rng();
    
    match level {
        DifficultyLevel::Easy => {
            let mut pattern: Vec<i32> = Vec::new();
            let mut n:i32 = rng.gen_range(2..9);
            let operand_num = rng.gen_range(1..5);
            let operand_string = match operand_num {
                1 => "+",
                2 => "-",
                3 => "/",
                4 => "*",
                _ => panic!("Unexpected number"),
            };
            let complementary_num= rng.gen_range(1..15);

           
            for num in 1..=5 {
                match operand_string {
                    "+" => pattern.push((n * num + complementary_num).into()),
                    "-" => pattern.push((n * num - complementary_num).into()),
                    "/" => {
                        
                        while n < complementary_num {
                            n = rng.gen_range(complementary_num..complementary_num*2);
                        }
                        pattern.push(((n * num) / complementary_num).into())
                    }
                    "*" => pattern.push((n * num * complementary_num).into()),
                    _ => unreachable!(),
                }
            }

            println!("general rule : {}n{}{}", n, operand_string, complementary_num);
            println!("Array is {:?}", pattern);
            pattern
        },
        
        DifficultyLevel::Medium => vec![1, 2, 3, 4, 5],  
        DifficultyLevel::Hard => vec![10, 20, 30, 40, 50],  
        DifficultyLevel::Impossible => vec![2, 4, 8, 16, 32],  
    }
}

fn determine_ip_address(ip_addr_clone: Arc<Mutex<Option<IpAddr>>>) {
    thread::spawn(move || {
        if let Some(addr) = database::ip_address::get_local_ip() {
            let mut client_address_lock = ip_addr_clone.lock().unwrap();
            *client_address_lock = Some(addr);
        } else {
            println!("failed to get IP address");
        }
    })
    .join()
    .unwrap();
}

fn loading_indicator(loading_completed: Arc<AtomicBool>) -> JoinHandle<()> {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template("{bar:40.white} {percent}% {elapsed} ({eta})")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
    thread::spawn(move || {
        let mut progress = 0;
        while !loading_completed.load(Ordering::SeqCst) {
            pb.inc(1);
            progress += 1;
            thread::sleep(Duration::from_millis(570));
        }
        if progress < 100 {
            for _ in progress..101 {
                pb.inc(1);
                progress += 1;
                thread::sleep(Duration::from_micros(100));
            }
        }
        pb.finish_with_message("Connected to database!");
    })
}
fn build_user_acount( ip_address: String,username: String,password: String) -> UserAccout{
    let pattern = Pattern{
        general_rule:"".to_string(),
        level:"".to_string(),
        pattern:vec![],
        time_taken:"".to_string(),
    };
    let pattern_clone =pattern.clone();
    let time = pattern.time_taken.clone();
    let pattern_info  = PatternInfo{
        pattern:pattern,
        time_taken:time,
    };
    let user_account =UserAccout{
        file_path:"".to_string(),
        incomplete_pattern:pattern_clone,
        password:password,
        ip_address:ip_address,
        name:username,
        num_attempts:"0".to_string(),
        rank:"noob".to_string(),
        patterns_solved:vec![pattern_info]
    };
    user_account

}
fn spinner_indicator(loading_completed: Arc<AtomicBool>, message: String) -> JoinHandle<()> {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("|/-\\"),
    );
    pb.set_message(message);
    thread::spawn(move || {
        while !loading_completed.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
            pb.tick();
        }
    })
}

/// This function handles account creation. If the user enters "#" when prompted for
/// a password (to continue an existing account), the function returns early and control
/// goes back to the login menu.
async fn handle_account_creation(
    usernames: &Vec<String>,
    users: &Vec<Document>,
    ip_address: &String,
) {
    let mut username = String::new();
    let mut password = String::new();

    // Get a unique username
    loop {
        print!("\nEnter User Name: ");
        io::stdout().flush().unwrap();
        username.clear();
        io::stdin()
            .read_line(&mut username)
            .expect("Error reading username");
        if !usernames.contains(&username.trim().to_string()) {
            break;
        }
        println!("Username '{}' is already taken. Please try another one.", username.trim());
    }

    // Get initial password (this might be used for account creation)
    print!("\nEnter Password: ");
    io::stdout().flush().unwrap();
    password.clear();
    io::stdin()
        .read_line(&mut password)
        .expect("Error reading password");

   
    let user_doc = database::users::find_user(&users, &username, ip_address);

    match user_doc {
        // No account exists for this username and IP address; create a new account.
        None => {
            let account_creation = Arc::new(AtomicBool::new(false));
            let ac = account_creation.clone();
            let join_handle = spinner_indicator(ac.clone(), "Creating Account...".to_string());
            let user_details = build_user_acount(ip_address.clone(), username.clone(), password.clone());
            database::users::create_user_account(user_details).await;
            account_creation.store(true, Ordering::SeqCst);
            join_handle.join().unwrap();
            println!("Account Created");
            
            // Insert new account creation logic here.
        }
        // An account was found.
        Some(data) => {
            println!("Account found for {}.", data);
            println!("Enter Password to continue or enter '#' to go back to the main menu: ");
            password.clear();
            io::stdout().flush().unwrap();
            io::stdin()
                .read_line(&mut password)
                .expect("Error reading password");

            if password.trim() == "#" {
                println!("Returning to the login menu...");
                return; // Early return takes you back to the login menu.
            } else {
                match database::users::login(&users, &data.trim().to_string(), &password.trim().to_string()) {
                    Ok(_) => {
                       let mut  game_option = String::new();
                       if let Some(account) =  database::users::find_logged_in_user(&users, &ip_address){
                        println!("+---------------------------------------------------+");
                        println!("|                          Gen-1(Easy Pattern)-350MB|");
                        println!("|                        Gen-2(Medium Pattern)-750MB|");
                        println!("|                            Gen-3(Hard Pattern)-1GB|");
                        println!("|                      Gen-4(Impossible Pattern)-2GB|");
                        println!("|                                                   |");
                        println!("| rank:{}                                           |",account.rank);
                        println!("| patterns solved:{}                                |",account.patterns_solved.len());
                        println!("| current_pattern:{:?}                              |",account.incomplete_pattern.pattern);
                        println!("| pattern_difficulity:{}                            |",account.incomplete_pattern.level);
                        println!("| time_elapsed:{}                                   |",account.incomplete_pattern.time_taken);
                        println!("|                                                   |");
                        println!("| Choose One To Determine Your Fate                 |");
                        println!("|                                                   |");
                        println!("| 1-> Show Leaderboard                              |");
                        println!("| 2-> Show Space Eaten                              |");
                        println!("| Type Gen-1 (Solve Easy Pattern)                   |");
                        println!("| Type Ans-(Answer of {} Term of pattern)           |",{8});
                        println!("|                                                   |");
                        println!("| NB:each incorrect answer comes with a cost        |");
                        println!("|                                                   |");
                        println!("|                                                   |");
                        println!("+---------------------------------------------------+");
                        print!("\nChoose an option: ");
                        io::stdout().flush().unwrap();
                        io::stdin().read_line(&mut game_option).unwrap();
                       if game_option == "1" {

                       }else if game_option == "2" {
                           
                       }else if game_option == "Gen-1" {
                           
                       }else if game_option == "Gen-2" {
                           
                       }else if game_option == "Gen-3" {
                           
                       }else if game_option == "Gen-4" {
                           
                       }else if game_option.starts_with("Ans-") {
                           
                       }

                   

                       }  
                      
                    }
                    Err(err) => {
                        println!("{}", err);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Set up user account & variables
    let client_address = Arc::new(Mutex::new(None::<IpAddr>));
    let client_address_clone = Arc::clone(&client_address);
    let _player_account: Option<UserAccout> = None;
    let logged_in = false;
    let mut ip_address = String::new();
    let mut users: Vec<Document> = Vec::new();
    let mut usernames: Vec<String> = Vec::new();

    // Terminal text
    let standard_font = FIGfont::standard().unwrap();

    // Exit the terminal gracefully on Ctrl+C
    ctrlc::set_handler(|| {
        println!("\nCtrl+C pressed! Exiting...");
        thread::sleep(Duration::from_millis(500));
        process::exit(256);
    })
    .unwrap();

    // Game Title
    let game_title = standard_font.convert("Pattern of Doom").unwrap();
    println!("{}", game_title);

    // Start Game Menu
    loop {
        println!("+------------------------+");
        println!("|  1. Start Game         |");
        println!("|  2. Exit               |");
        println!("+------------------------+");
        print!("\nChoose an option: ");
        io::stdout().flush().unwrap();

        let mut start_menu_option = String::new();
        io::stdin()
            .read_line(&mut start_menu_option)
            .expect("Error reading input");

        match start_menu_option.trim().parse::<u8>() {
            Ok(1) => break, // Proceed to next steps
            Ok(2) => {
                println!("Exiting game...");
                process::exit(0);
            }
            _ => println!("Enter a valid number."),
        }
    }

    // Get IP address
    determine_ip_address(client_address_clone);
    {
        let client_address_lock = client_address.lock().unwrap();
        if let Some(addr) = *client_address_lock {
            ip_address = addr.to_string();
        }
    }

    // Connect to DB
    println!("Connecting To Database");
    let connection_complete = Arc::new(AtomicBool::new(false));
    let loading_completed = connection_complete.clone();
    let join_handle = loading_indicator(loading_completed);
    if let Ok(data) = database::get_connection().await {
        connection_complete.store(true, Ordering::SeqCst);
        join_handle.join().unwrap();
        println!("Connected To Database");
        users = data.1;
    }

    // Populate all usernames from the database
    usernames = database::users::get_all_usernames(&users);

    // Login Menu Loop
    loop {
        println!("\n+------------------------+");
        println!("|  1. Login              |");
        println!("|  2. Create Account     |");
        println!("+------------------------+");
        print!("\nChoose an option: ");
        io::stdout().flush().unwrap();

        let mut login_menu_option = String::new();
        io::stdin()
            .read_line(&mut login_menu_option)
            .expect("Error reading input");

        match login_menu_option.trim().parse::<u8>() {
            Ok(1) => {
                // Placeholder for the login functionality.
                // You can add your login code here.
                println!("Login functionality not yet implemented.");
            }
            Ok(2) => {
                // Call the account creation function. If the user enters '#' during the process,
                // they will be returned to this login menu.
                handle_account_creation(&usernames, &users, &ip_address).await;
            }
            _ => {
                println!("Enter a valid number.");
                continue;
            }
        }

        // Optional: if you want to exit the login loop after a successful login,
        // you can break here. Otherwise, the loop continues.
        // For example, if logged in:
        if logged_in {
            break;
        }
    }

    // Wait for user input to quit the program.
    let mut quit = String::new();
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut quit)
        .expect("Error reading input");
    if quit.trim() == "q" {
        process::exit(256);
    }
}
