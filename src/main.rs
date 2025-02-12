mod database;
use database::get_all_docs;
use database::users::DifficultyLevel;
use database::users::{update_user_account, Pattern, PatternInfo, UserAccout};
use figlet_rs::FIGfont;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use mongodb::bson::Document;
use rand::Rng;
use std::fs;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use sysinfo::{Disk, Disks};
use tokio::sync::mpsc::{channel, Sender};

fn generate_sequence(level: &DifficultyLevel) -> (Vec<u32>, String) {
    let mut rng = rand::thread_rng();

    match level {
        DifficultyLevel::Easy => {
            let mut pattern: Vec<u32> = Vec::new();
            let n: u32 = rng.gen_range(2..9);
            let operand_num = rng.gen_range(1..4);
            let operand_string = match operand_num {
                1 => "+",
                2 => "-",
                3 => "*",
                _ => panic!("Unexpected number"),
            };
            let complementary_num = rng.gen_range(1..15);

            for num in 1..=5 {
                match operand_string {
                    "+" => pattern.push((n * num + complementary_num as u32)),
                    "-" => pattern.push((n * num - complementary_num as u32)),
                    "*" => pattern.push((n * num * complementary_num)),
                    _ => unreachable!(),
                }
            }

            let rule = format!("{},{},{}", n, operand_string, complementary_num);
            (pattern, rule)
        }

        DifficultyLevel::Medium => todo!(),
        DifficultyLevel::Hard => todo!(),
        DifficultyLevel::Impossible => todo!(),
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
            thread::sleep(Duration::from_millis(700));
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

fn build_user_acount(ip_address: String, username: String, password: String) -> UserAccout {
    let pattern = Pattern {
        jeopardy: 0,
        general_rule: "".to_string(),
        level: "".to_string(),
        pattern: vec![],
        time_taken: 0,
        term_to_solve: 0,
        solved: true,
    };
    let pattern_clone = pattern.clone();
    let time = pattern.time_taken.clone();
    let pattern_info = PatternInfo {
        pattern: pattern,
        time_taken: time,
    };
    let user_account = UserAccout {
        file_path: "".to_string(),
        incomplete_pattern: pattern_clone,
        password: password,
        ip_address: ip_address,
        name: username,
        num_attempts: "0".to_string(),
        rank: "noob".to_string(),
        patterns_solved: vec![pattern_info],
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
fn counter(seconds: Arc<AtomicI32>, flag_clone: Arc<AtomicBool>) {
    let counter_handle = thread::spawn(move || {
        while !flag_clone.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(1));
            seconds.fetch_add(1, Ordering::SeqCst);
            // print!("\r| time_elapsed:{:02}:{:02}:{:02}",
            //        seconds.load(Ordering::SeqCst) / 3600,
            //        (seconds.load(Ordering::SeqCst) / 60) % 60,
            //        seconds.load(Ordering::SeqCst) % 60);
            // io::stdout().flush().unwrap();
        }
    });
}
fn is_correct_answer(answer: &i32, term: &i32, rule: &String) -> (bool, f32) {
    let rule_parts: Vec<&str> = rule.split(",").collect();
    let mut correct_answer = 0;
    let first: i32 = match rule_parts[0].parse() {
        Ok(v) => v,
        Err(err) => {
            println!("error partsing num {}", err);
            0
        }
    };
    let last: i32 = match rule_parts[2].parse() {
        Ok(v) => v,
        Err(err) => {
            println!("error partsing num {}", err);
            0
        }
    };
    match rule_parts[1] {
        "/" => correct_answer = (first * term) / last,
        "*" => correct_answer = (first * term) * last,
        "-" => correct_answer = (first * term) - last,
        "+" => correct_answer = (first * term) + last,
        _ => (),
    }
    println!("correct_answer : {}", correct_answer);
    let diff = correct_answer - answer;
    let off_percentage: f32 = (diff as f32 / correct_answer as f32) * 100.0;
    if correct_answer == *answer {
        return (true, off_percentage);
    } else {
        return (false, off_percentage);
    }
}
async fn account_login(
    sys: &mut Disks,
    users: &mut Vec<Document>,
    ip_address: &String,
    data: String,
    password: String,
    tx: Sender<Option<UserAccout>>,
) {
    let mut exit = false;
    let mut c_drive: Option<&mut Disk> = Option::None;
    let counter_flag = Arc::new(AtomicBool::new(false));
    let mut hint: Option<f32> = Option::None;
    let mut message: Option<&str> = Option::None;
    let mut jeopardy: Option<i32> = Option::None;
    let mut user_account: Option<UserAccout> = None;
    let size = Arc::new(AtomicI32::new(0));
    let size_for_thread = Arc::clone(&size);
    if let Some(account) = database::users::find_logged_in_user(&users, &ip_address) {
        size.fetch_add(account.incomplete_pattern.jeopardy, Ordering::SeqCst);
        user_account = Some(account);
    }

    for disk in sys {
        println!("{}", disk.mount_point().display().to_string());
        if disk.mount_point().display().to_string().starts_with("C:") {
            println!("found disk ");
            c_drive = Some(disk);
            break;
        }
    }
    jeopardy = Some(size.load(Ordering::SeqCst));
    let shared_account = Arc::new(Mutex::new(user_account.unwrap()));
    let shared_account_ref = Arc::clone(&shared_account);
    let mut shared_account_ref_lock = shared_account_ref.lock().unwrap();
    let seconds = Arc::new(AtomicI32::new(
        shared_account_ref_lock.incomplete_pattern.time_taken,
    ));
    let seconds_clone = Arc::clone(&seconds);
    let second_ref = Arc::new(seconds_clone);

    let path = Path::new(r"C:\Temp\test\file.txt");
    if shared_account_ref_lock.file_path.len() < 2 {
        shared_account_ref_lock.file_path = path.to_str().unwrap().to_string();
        update_user_account(shared_account_ref_lock.clone()).await;
    }
    drop(shared_account_ref_lock);
    let lock2 = shared_account_ref.lock().unwrap();
    if lock2.incomplete_pattern.solved == false {
        counter(seconds, counter_flag);
    }
    drop(lock2);

    while !exit {
        match database::users::login(
            &users,
            &data.trim().to_string(),
            &password.trim().to_string(),
        ) {
            Ok(_) => {
                let mut game_option = String::new();
                let mut lock = shared_account_ref.lock().unwrap();

                println!("+---------------------------------------------------");
                println!("|                          Gen-1(Easy Pattern)-350MB");
                println!("|                        Gen-2(Medium Pattern)-750MB");
                println!("|                            Gen-3(Hard Pattern)-1GB");
                println!("|                      Gen-4(Impossible Pattern)-2GB");
                println!("|                                                   ");
                println!(
                    "| rank:{}                                           ",
                    lock.rank
                );
                println!(
                    "| patterns solved:{}                                ",
                    lock.patterns_solved.len()
                );
                println!(
                    "| current_pattern:{:?}                              ",
                    lock.incomplete_pattern.pattern
                );
                println!(
                    "| pattern_difficulity:{}                            ",
                    lock.incomplete_pattern.level
                );
                println!("|                                                   ");
                println!("| Choose One To Determine Your Fate                 ");
                println!("|                                                   ");
                println!("| 1-> Show Leaderboard                              ");
                println!("| 2-> Show Space Eaten                              ");
                println!("| Type Gen-1 (Solve Easy Pattern)                   ");
                println!("| Type Ans-(Answer of {} Term of pattern)           ", {
                    lock.incomplete_pattern.term_to_solve
                });
                println!("| Type quit (To Exit)                               ");
                println!("|                                                   ");
                println!("| NB:each incorrect answer comes with a cost        ");
                println!("|                                                   ");
                println!("|                                                   ");
                println!("+---------------------------------------------------");
                if let Some(disk) = c_drive.as_mut() {
                    disk.refresh();
                    if disk.available_space() > 1_111_741_824 {
                        println!(
                            "Disc Space Available : {} GB ",
                            disk.available_space() / 1_073_741_824
                        );
                    } else {
                        println!(
                            "Disc Space Available : {} MB ",
                            disk.available_space() / 1_048_576
                        );
                    }
                }
                if let Some(data) = jeopardy {
                    if data > 1024 {
                        println!("jeopardy : {} GB ", data);
                    } else {
                        println!("jeopardy : {} MB ", data);
                    }
                }
                if let Some(mess) = message {
                    println!("{}", mess);
                }
                if let Some(value) = hint {
                    println!("Answer Incorrect\nAnswer is {}% off", value);
                }

                println!();
                print!("\nChoose an option: ");

                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut game_option).unwrap();
                if game_option.trim() == "1" {
                    hint = None;
                    message = None;

                    println!("seconds : {}", second_ref.load(Ordering::SeqCst));
                } else if game_option.contains("quit") {
                    hint = None;
                    // user_details = Some(account);
                    exit = true
                } else if game_option == "2" {
                    hint = None;
                    message = None;
                } else if game_option.trim().contains("Gen-1") {
                    hint = None;

                    if lock.incomplete_pattern.solved {
                        let mut rng = rand::thread_rng();
                        let term_to_solve = rng.gen_range(6..12);
                        let seq: (Vec<u32>, String) = generate_sequence(&DifficultyLevel::Easy);
                        let seq_creation = Arc::new(AtomicBool::new(false));
                        let sc = seq_creation.clone();
                        lock.incomplete_pattern.pattern = seq.0;
                        lock.incomplete_pattern.general_rule = seq.1;
                        lock.incomplete_pattern.term_to_solve = term_to_solve;
                        lock.incomplete_pattern.level = "Easy".to_string();
                        lock.incomplete_pattern.time_taken = second_ref.load(Ordering::SeqCst);
                        lock.incomplete_pattern.solved = false;

                        let join_handle =
                            spinner_indicator(sc.clone(), "Generating Sequence...".to_string());
                        update_user_account(lock.clone()).await;
                        seq_creation.store(true, Ordering::SeqCst);
                        join_handle.join().unwrap();
                        println!("seq_creacted!");
                    } else {
                        message = Some("solve current problem firsttttt ");
                    }
                } else if game_option == "Gen-2" {
                    hint = None;
                } else if game_option == "Gen-3" {
                    hint = None;
                } else if game_option == "Gen-4" {
                    hint = None;
                } else if game_option.starts_with("Ans-") {
                    message = None;
                    let str_answer = &game_option[4..];
                    println!("answer_str : {}", str_answer);
                    let answer: i32 = str_answer.trim().parse().expect("error panicking now");
                    println!("answer : {}", answer);
                    let correct = is_correct_answer(
                        &answer,
                        &lock.incomplete_pattern.term_to_solve,
                        &lock.incomplete_pattern.general_rule,
                    );
                    if correct.0 {
                        println!("Answer correct");
                        lock.incomplete_pattern.jeopardy = 0;
                        lock.incomplete_pattern.time_taken = second_ref.load(Ordering::SeqCst);
                        update_user_account(lock.clone()).await;
                        let file_path = Path::new(r"C:\Temp\test\file.txt");
                        match fs::remove_file(file_path) {
                            Ok(_) => {
                                if jeopardy.unwrap() > 500 {
                                    println!("Alight U survived (barely)")
                                } else {
                                    println!("Nobody likes a Show off ")
                                }
                            }

                            Err(err) => println!("error while deleting {}", { err }),
                        }
                    } else {
                        println!("Answer incorrect");
                        println!("Answer is {}%Off", correct.1);
                        hint = Some(correct.1);
                        let size_thread = size_for_thread.load(Ordering::SeqCst);
                        let acc_path = lock.file_path.clone();
                        thread::spawn(move || {
                            let content = "A".repeat(1024);
                            let path = Path::new(acc_path.as_str().trim());
                            if let Some(parent) = path.parent() {
                                if let Err(e) = create_dir_all(parent) {
                                    println!("Failed to create directories: {}", e);
                                    return;
                                }
                            }

                            let mut file =
                                match OpenOptions::new().append(true).create(true).open(path) {
                                    Ok(file) => file,
                                    Err(err) => {
                                        println!("Error opening file: {}", err);
                                        return;
                                    }
                                };

                            for _ in 1..size_thread * 1024 {
                                let res = file.write_all(content.as_bytes());
                                if let Err(err) = res {
                                    println!("Error writing to file: {}", err);
                                    break;
                                } else {
                                }
                            }
                        });
                        size.fetch_add(150, Ordering::SeqCst);
                        jeopardy = Some(size.load(Ordering::SeqCst));
                        let mut jeopardy_clone = 90;
                        if let Some(jeopardy) = jeopardy {
                            jeopardy_clone = jeopardy;
                        }
                        lock.incomplete_pattern.jeopardy = jeopardy_clone;
                        lock.incomplete_pattern.time_taken = second_ref.load(Ordering::SeqCst);
                    }
                }

                let _ = tx.send(Some(lock.clone())).await;

                drop(lock);
            }
            Err(err) => {
                println!("{}", err);
                break;
            }
        }
    }
}
async fn handle_account_creation(
    usernames: &Vec<String>,
    users: &mut Vec<Document>,
    ip_address: &String,
    sys: &mut Disks,
    tx: Sender<Option<UserAccout>>,
) {
    let mut user_details: Option<UserAccout> = Option::None;
    let mut username = String::new();
    let mut password = String::new();

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
        println!(
            "Username '{}' is already taken. Please try another one.",
            username.trim()
        );
    }

    print!("\nEnter Password: ");
    io::stdout().flush().unwrap();
    password.clear();
    io::stdin()
        .read_line(&mut password)
        .expect("Error reading password");

    let user_doc = database::users::find_user(&users, &username, ip_address);

    match user_doc {
        None => {
            let account_creation = Arc::new(AtomicBool::new(false));
            let ac = account_creation.clone();
            let join_handle = spinner_indicator(ac.clone(), "Creating Account...".to_string());
            user_details = Some(build_user_acount(
                ip_address.clone(),
                username.clone(),
                password.clone(),
            ));
            if let Some(details) = user_details {
                database::users::create_user_account(details).await;
            }
            account_creation.store(true, Ordering::SeqCst);
            join_handle.join().unwrap();
            println!("Account Created");
            // show the game screen
            let docs = get_all_docs().await;

            match docs {
                None => {
                    println!("No docs found ");
                    return;
                }
                Some(data) => {
                    *users = data;
                }
            }
            account_login(sys, users, ip_address, username, password, tx).await;
        }

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
                return;
            } else {
                account_login(sys, users, ip_address, data, password, tx).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Set up user account & variables
    let mut sys = Disks::new_with_refreshed_list();
    let client_address = Arc::new(Mutex::new(None::<IpAddr>));
    let client_address_clone = Arc::clone(&client_address);
    let player_account = Arc::new(Mutex::new(None::<UserAccout>));
    let player_account_clone = Arc::clone(&player_account);
    let player_account_clone_2 = Arc::clone(&player_account);
    let logged_in = false;
    let mut ip_address = String::new();
    let mut users: Vec<Document> = Vec::new();
    let mut usernames: Vec<String> = Vec::new();

    // Terminal text
    let standard_font = FIGfont::standard().unwrap();

    // set up the signals
    let (tx, mut rx) = channel::<Option<UserAccout>>(1);
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(account) = rx.recv().await {
            let acc_clone = account.clone();
            let mut lock = player_account_clone.lock().unwrap();
            *lock = account;

            println!("Account state saved");
            println!("Account : {:?}", acc_clone);
            drop(lock);
        }
    });

   
    tokio::spawn(async move {
       
        tokio::signal::ctrl_c().await.unwrap();
        println!("\nCtrl+C pressed! Exiting...");

        let account = match player_account_clone_2.lock() {
            Ok(lock) => lock.clone(),
            Err(poisoned) => {
                eprintln!("Mutex poisoned, using possibly inconsistent state");
                poisoned.into_inner().clone()
            }
        };

       
        if let Some(user_account) = account {
            println!("account ->>{:?}",&user_account);
           
            update_user_account(user_account).await;
        }

        // Async-friendly delay for message visibility
        // tokio::time::sleep(Duration::from_millis(500)).await;

        // Ensure proper cleanup before exit
        process::exit(0);
    });

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
                let mut username = String::new();
                let mut password = String::new();
                print!("\nEnter User Name: ");
                io::stdout().flush().unwrap();
                io::stdin()
                    .read_line(&mut username)
                    .expect("Error reading username");
                print!("\nEnter Password: ");
                io::stdout().flush().unwrap();
                password.clear();
                io::stdin()
                    .read_line(&mut password)
                    .expect("Error reading password");
                let user_doc = database::users::find_user(&users, &username, &ip_address);
                match user_doc {
                    None => println!("No Account Found Create a new Account "),
                    Some(data) => {
                        account_login(
                            &mut sys,
                            &mut users,
                            &ip_address,
                            data,
                            password,
                            tx_clone.clone(),
                        )
                        .await;
                    }
                }
            }
            Ok(2) => {
                handle_account_creation(&usernames, &mut users, &ip_address, &mut sys, tx.clone())
                    .await;
            }
            _ => {
                println!("Enter a valid number.");
                continue;
            }
        }

        if logged_in {
            break;
        }
    }

    let mut quit = String::new();
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut quit)
        .expect("Error reading input");
    if quit.trim() == "q" {
        process::exit(256);
    }
}
