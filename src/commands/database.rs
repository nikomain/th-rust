use crate::cli::DatabaseArgs;
use crate::config::Config;
use crate::display::{clear_screen, create_header, create_menu, load_content, print_success, print_error, print_info, display_code};
use crate::teleport::TeleportClient;
use anyhow::Result;
use colored::*;
use serde_json::Value;

pub async fn execute(args: DatabaseArgs, config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    let client = TeleportClient::new(config.clone());

    // Call th_login at start like bash version does
    crate::display::th_login().await?;

    // Direct connection if target specified
    if let Some(target) = args.target {
        return direct_connect(&client, &target).await;
    }

    // Interactive database selection - exactly like bash db_login
    db_login(&client).await
}

async fn direct_connect(client: &TeleportClient, target: &str) -> Result<()> {
    clear_screen()?;
    create_header("Database Connection");
    
    print_info(&format!("Connecting to database: {}", target));
    
    let target_owned = target.to_string();
    let client_clone = client.clone();
    load_content(
        &format!("Logging into {}...", target),
        async move { client_clone.db_login(&target_owned).await }
    ).await?;
    
    print_success(&format!("Successfully connected to {}", target));
    
    // Get connection details and show options
    show_connection_options(client, target).await
}

async fn db_login(client: &TeleportClient) -> Result<()> {
    use std::io::{self, Write};
    
    clear_screen()?;
    create_header("DB");
    
    println!("Which database would you like to connect to?");
    println!("\n1. \x1b[1mRDS\x1b[0m");
    println!("2. \x1b[1mMongoDB\x1b[0m");
    
    loop {
        print!("\nSelect option (number): ");
        io::stdout().flush().unwrap();
        
        let mut db_choice = String::new();
        io::stdin().read_line(&mut db_choice).unwrap();
        
        match db_choice.trim() {
            "1" => {
                println!("\n\x1b[1mRDS\x1b[0m selected.");
                return handle_rds_selection(client).await;
            },
            "2" => {
                println!("\n\x1b[1mMongoDB\x1b[0m selected.");
                return handle_mongodb_selection(client).await;
            },
            _ => {
                println!("\n\x1b[31mInvalid selection here2\x1b[0m");
                continue;
            }
        }
    }
}

async fn handle_rds_selection(client: &TeleportClient) -> Result<()> {
    clear_screen()?;
    create_header("Available Databases");
    
    // Get RDS databases and check access - exactly like bash check_rds_login
    let client_clone = client.clone();
    let databases = load_content(
        "Checking cluster access...",
        async move { client_clone.list_rds_databases().await }
    ).await?;

    if databases.is_empty() {
        print_error("No RDS databases available");
        return Ok(());
    }

    // Display databases with bash-style formatting
    for (i, db) in databases.iter().enumerate() {
        if db.accessible {
            println!("{:2}. {}", i + 1, db.name);
        } else {
            println!("\x1b[90m{:2}. {}\x1b[0m", i + 1, db.name);
        }
    }

    // Get user choice - exactly like bash
    use std::io::{self, Write};
    print!("\n\x1b[1mSelect database (number):\x1b[0m ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    if choice.trim().is_empty() {
        println!("No selection made. Exiting.");
        return Ok(());
    }
    
    let selected_index: usize = choice.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
        
    if selected_index == 0 || selected_index > databases.len() {
        println!("\n\x1b[31mInvalid selection here\x1b[0m");
        return Ok(());
    }
    
    let selected_db = &databases[selected_index - 1];
    
    // Handle elevated access case
    if !selected_db.accessible {
        db_elevated_login(client, "sudo_teleport_rds_read_role", &selected_db.name).await?;
        return Ok(());
    }
    
    println!("\n\x1b[1;32m{}\x1b[0m selected.", selected_db.name);
    
    // Connect to RDS
    rds_connect(client, &selected_db.name).await
}

async fn handle_mongodb_selection(client: &TeleportClient) -> Result<()> {
    clear_screen()?;
    create_header("Available Databases");
    
    let client_clone = client.clone();
    let (databases, has_atlas_access) = load_content(
        "Checking MongoDB access...",
        async move { client_clone.list_mongodb_databases().await }
    ).await?;

    if databases.is_empty() {
        print_error("No MongoDB databases available");
        return Ok(());
    }

    // Display databases with color coding based on access
    for (i, db) in databases.iter().enumerate() {
        print!("{:2}. ", i + 1);
        if has_atlas_access {
            println!("{}", db);
        } else {
            println!("\x1b[90m{}\x1b[0m", db);
        }
    }

    // Get user choice - exactly like bash
    use std::io::{self, Write};
    print!("\n\x1b[1mSelect database (number):\x1b[0m ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    if choice.trim().is_empty() {
        println!("No selection made. Exiting.");
        return Ok(());
    }
    
    let selected_index: usize = choice.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
        
    if selected_index == 0 || selected_index > databases.len() {
        println!("\n\x1b[31mInvalid selection\x1b[0m");
        return Ok(());
    }
    
    let selected_db = &databases[selected_index - 1];
    
    // If user doesn't have atlas access, trigger elevated login
    if !has_atlas_access {
        db_elevated_login(client, "atlas-read-only", selected_db).await?;
        return Ok(());
    }
    
    println!("\n\x1b[1;32m{}\x1b[0m selected.", selected_db);
    
    // Connect to MongoDB
    mongo_connect(client, selected_db).await
}

async fn db_elevated_login(_client: &TeleportClient, role: &str, db_name: &str) -> Result<()> {
    use std::io::{self, Write};
    
    let display_name = if db_name.is_empty() {
        "Mongo databases"
    } else {
        db_name
    };
    
    loop {
        clear_screen()?;
        create_header("Privilege Request");
        
        println!("You don't have access to \x1b[4m{}\x1b[0m", display_name);
        print!("\n\nWould you like to raise a request? (y/n): ");
        io::stdout().flush().unwrap();
        
        let mut elevated = String::new();
        io::stdin().read_line(&mut elevated).unwrap();
        
        match elevated.trim().to_lowercase().as_str() {
            "y" | "yes" => {
                print!("\n\x1b[1mEnter your reason for request: \x1b[0m");
                io::stdout().flush().unwrap();
                
                let mut reason = String::new();
                io::stdin().read_line(&mut reason).unwrap();
                let reason = reason.trim();
                
                println!();
                
                // Execute tsh request create command
                let request_output = std::process::Command::new("tsh")
                    .args([
                        "request", "create", 
                        "--roles", role,
                        "--max-duration", "6h",
                        "--reason", reason
                    ])
                    .output()?;
                
                let output_text = String::from_utf8_lossy(&request_output.stdout);
                let error_text = String::from_utf8_lossy(&request_output.stderr);
                
                // Print output to user (like bash tee /dev/tty)
                print!("{}{}", output_text, error_text);
                io::stdout().flush().unwrap();
                
                // Extract request ID from output
                let full_output = format!("{}{}", output_text, error_text);
                if let Some(request_id_line) = full_output.lines().find(|line| line.contains("Request ID:")) {
                    if let Some(request_id) = request_id_line.split_whitespace().nth(2) {
                        println!("\nRequest ID: {}", request_id);
                    }
                }
                
                // Set reauth_db flag equivalent (would need to be handled by calling function)
                println!("\nElevated access request submitted. You will need to re-authenticate.");
                
                return Ok(());
            },
            "n" | "no" => {
                println!("\nRequest creation skipped.");
                // Set exit_db flag equivalent (would need to be handled by calling function)
                return Ok(());
            },
            _ => {
                println!("\n\x1b[31mInvalid input. Please enter y or n.\x1b[0m");
                continue;
            }
        }
    }
}

async fn rds_connect(client: &TeleportClient, rds: &str) -> Result<()> {
    use std::io::{self, Write};
    
    clear_screen()?;
    create_header("Connect");
    
    println!("How would you like to connect?\n");
    println!("1. Via \x1b[1mPSQL\x1b[0m");
    println!("2. Via \x1b[1mDBeaver\x1b[0m");
    print!("\nSelect option (number): ");
    io::stdout().flush().unwrap();
    
    let mut option = String::new();
    io::stdin().read_line(&mut option).unwrap();
    
    if option.trim().is_empty() {
        println!("No selection made. Exiting.");
        return Ok(());
    }
    
    match option.trim() {
        "1" => {
            println!("\nConnecting via \x1b[1;32mPSQL\x1b[0m...");
            check_psql().await?;
            let database = list_postgres_databases(client, rds).await?;
            let db_user = check_admin(client).await?;
            connect_db(client, rds, &database, &db_user).await
        },
        "2" => {
            println!("\nConnecting via \x1b[1;32mDBeaver\x1b[0m...");
            let database = list_postgres_databases(client, rds).await?;
            let db_user = check_admin(client).await?;
            open_dbeaver(client, rds, &database, &db_user).await
        },
        _ => {
            println!("Invalid selection. Exiting.");
            Ok(())
        }
    }
}

async fn check_psql() -> Result<()> {
    use std::io::{self, Write};
    
    // Check if psql command exists
    let psql_check = std::process::Command::new("which")
        .arg("psql")
        .output();
    
    if psql_check.map(|output| output.status.success()).unwrap_or(false) {
        return Ok(());
    }
    
    println!("\n\x1b[1m=============== PSQL not found ===============\x1b[0m");
    println!("\n❌ PSQL client not found. It is required to connect to PostgreSQL databases.");
    
    loop {
        print!("\nWould you like to install it via brew? (y/n): ");
        io::stdout().flush().unwrap();
        
        let mut install = String::new();
        io::stdin().read_line(&mut install).unwrap();
        
        match install.trim().to_lowercase().as_str() {
            "y" | "yes" => {
                println!();
                let output = std::process::Command::new("brew")
                    .args(["install", "postgresql@14"])
                    .output()?;
                    
                if output.status.success() {
                    println!("\n✅ \x1b[1;32mPSQL client installed successfully!\x1b[0m");
                } else {
                    println!("\n❌ Failed to install PSQL client");
                }
                break;
            },
            "n" | "no" => {
                println!("\nPSQL installation skipped.");
                break;
            },
            _ => {
                println!("\n\x1b[31mInvalid input. Please enter y or n.\x1b[0m");
                continue;
            }
        }
    }
    
    Ok(())
}

async fn list_postgres_databases(_client: &TeleportClient, rds: &str) -> Result<String> {
    use std::io::{self, Write};
    
    // Find available port
    let port = crate::display::find_available_port();
    
    // Start proxy tunnel
    let mut child = std::process::Command::new("tsh")
        .args(["proxy", "db", rds, "--db-user=tf_teleport_rds_read_user", "--db-name=postgres", &format!("--port={}", port), "--tunnel"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    
    // Wait for proxy to be ready (up to 10 seconds)
    let mut proxy_ready = false;
    for _ in 0..10 {
        let nc_check = std::process::Command::new("nc")
            .args(["-z", "localhost", &port.to_string()])
            .output();
            
        if nc_check.map(|output| output.status.success()).unwrap_or(false) {
            proxy_ready = true;
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    if !proxy_ready {
        println!("\n\x1b[31m❌ Failed to establish tunnel to database.\x1b[0m");
        let _ = child.kill();
        return Err(anyhow::anyhow!("Failed to establish tunnel"));
    }
    
    clear_screen()?;
    create_header("Available Databases");
    
    // Fetch databases
    let db_list = load_content(
        "Fetching databases...",
        async move { fetch_postgres_databases(port).await }
    ).await?;
    
    // Kill proxy
    let _ = child.kill();
    
    if db_list.is_empty() {
        println!("\x1b[31m❌ No databases found or connection failed.\x1b[0m");
        return Err(anyhow::anyhow!("No databases found"));
    }
    
    // Display databases
    for (i, db) in db_list.iter().enumerate() {
        println!("{:2}. {}", i + 1, db);
    }
    
    print!("\n\x1b[1mSelect database (number):\x1b[0m ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    if choice.trim().is_empty() {
        println!("No selection made. Exiting.");
        return Ok("postgres".to_string()); // Default to postgres
    }
    
    let selected_index: usize = choice.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
    
    if selected_index == 0 || selected_index > db_list.len() {
        println!("\n\x1b[31mInvalid selection\x1b[0m");
        return Ok("postgres".to_string()); // Default to postgres
    }
    
    Ok(db_list[selected_index - 1].clone())
}

async fn fetch_postgres_databases(port: u16) -> Result<Vec<String>> {
    let output = std::process::Command::new("psql")
        .arg(&format!("postgres://tf_teleport_rds_read_user@localhost:{}/postgres", port))
        .args(["-t", "-A", "-c", "SELECT datname FROM pg_database WHERE datistemplate = false;"])
        .output()?;
    
    if !output.status.success() {
        return Ok(vec![]);
    }
    
    let db_list = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    
    Ok(db_list)
}

async fn check_admin(client: &TeleportClient) -> Result<String> {
    use std::io::{self, Write};
    
    // Check if user has admin role
    let status_output = std::process::Command::new("tsh")
        .args(["status"])
        .output()?;
    
    let status_text = String::from_utf8_lossy(&status_output.stdout);
    
    if status_text.contains("sudo_teleport_rds_write_role") {
        print!("\nConnecting as admin? (y/n): ");
        io::stdout().flush().unwrap();
        
        let mut admin = String::new();
        io::stdin().read_line(&mut admin).unwrap();
        
        if admin.trim().to_lowercase().starts_with('y') {
            return Ok("tf_sudo_teleport_rds_user".to_string());
        }
    }
    
    Ok("tf_teleport_rds_read_user".to_string())
}

async fn connect_db(_client: &TeleportClient, rds: &str, database: &str, db_user: &str) -> Result<()> {
    use std::io::Write;
    
    println!("\n\x1b[1mConnecting to \x1b[1;32m{}\x1b[0m in \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m...", database, rds, db_user);
    
    for i in (1..=3).rev() {
        print!("\x1b[1;32m. \x1b[0m");
        std::io::stdout().flush().unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    println!();
    
    clear_screen()?;
    
    // Execute tsh db connect
    std::process::Command::new("tsh")
        .args(["db", "connect", rds, &format!("--db-user={}", db_user), &format!("--db-name={}", database)])
        .status()?;
    
    Ok(())
}

async fn open_dbeaver(_client: &TeleportClient, rds: &str, database: &str, db_user: &str) -> Result<()> {
    use std::io::Write;
    
    let port = crate::display::find_available_port();
    
    println!("\n\x1b[1mConnecting to \x1b[1;32m{}\x1b[0m in \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m...\n", database, rds, db_user);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Start proxy in background
    std::process::Command::new("tsh")
        .args(["proxy", "db", rds, &format!("--db-name={}", database), &format!("--port={}", port), "--tunnel", &format!("--db-user={}", db_user)])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    
    clear_screen()?;
    create_header("DBeaver");
    
    println!("\x1b[1mTo connect to the database, follow these steps: \x1b[0m\n");
    println!("1. Once DBeaver opens click create a new connection in the very top left.");
    println!("2. Select \x1b[1mPostgreSQL\x1b[0m as the database type.");
    println!("3. Use the following connection details:");
    println!(" - Host:      \x1b[1mlocalhost\x1b[0m");
    println!(" - Port:      \x1b[1m{}\x1b[0m", port);
    println!(" - Database:  \x1b[1m{}\x1b[0m", database);
    println!(" - User:      \x1b[1m{}\x1b[0m", db_user);
    println!(" - Password:  \x1b[1m(leave blank)\x1b[0m");
    println!("4. Optionally, select show all databases.");
    println!("5. Click 'Test Connection' to ensure everything is set up correctly.");
    println!("6. If the test is successful, click 'Finish' to save the connection.");
    
    for _i in (1..=3).rev() {
        print!("\x1b[1;32m. \x1b[0m");
        std::io::stdout().flush().unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    print!("\r\x1b[K\n");
    std::io::stdout().flush().unwrap();
    
    // Open DBeaver
    std::process::Command::new("open")
        .args(["-a", "DBeaver"])
        .status()?;
    
    Ok(())
}

async fn mongo_connect(_client: &TeleportClient, db_name: &str) -> Result<()> {
    use std::io::{self, Write};
    
    // Determine db_user based on database name
    let db_user = match db_name {
        "mongodb-YLUSProd-Cluster-1" => "teleport-usprod",
        "mongodb-YLProd-Cluster-1" => "teleport-prod", 
        "mongodb-YLSandbox-Cluster-1" => "teleport-sandbox",
        _ => "teleport-default", // fallback
    };
    
    clear_screen()?;
    create_header("MongoDB");
    
    println!("How would you like to connect?\n");
    println!("1. Via \x1b[1mMongoCLI\x1b[0m");
    println!("2. Via \x1b[1mAtlasGUI\x1b[0m");
    print!("\nSelect option (number): ");
    io::stdout().flush().unwrap();
    
    let mut option = String::new();
    io::stdin().read_line(&mut option).unwrap();
    
    loop {
        match option.trim() {
            "1" => {
                // Check if mongosh is available
                let mongosh_check = std::process::Command::new("command")
                    .args(["-v", "mongosh"])
                    .output();
                
                if mongosh_check.map(|output| output.status.success()).unwrap_or(false) {
                    // MongoDB client found, connect
                    println!("\n\x1b[1mConnecting to \x1b[1;32m{}\x1b[0m...", db_name);
                    
                    for _i in (1..=3).rev() {
                        print!("\x1b[1;32m. \x1b[0m");
                        io::stdout().flush().unwrap();
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                    
                    clear_screen()?;
                    
                    // Execute tsh db connect
                    std::process::Command::new("tsh")
                        .args(["db", "connect", db_name, &format!("--db-user={}", db_user), "--db-name=admin"])
                        .status()?;
                        
                    return Ok(());
                } else {
                    // MongoDB client not found
                    println!("\n❌ MongoDB client not found. MongoSH is required to connect to MongoDB databases.");
                    
                    loop {
                        print!("\nWould you like to install it via brew? (y/n): ");
                        io::stdout().flush().unwrap();
                        
                        let mut install = String::new();
                        io::stdin().read_line(&mut install).unwrap();
                        
                        match install.trim().to_lowercase().as_str() {
                            "y" | "yes" => {
                                println!();
                                let output = std::process::Command::new("brew")
                                    .args(["install", "mongosh"])
                                    .output()?;
                                    
                                if output.status.success() {
                                    println!("\n✅ \x1b[1;32mMongoDB client installed successfully!\x1b[0m");
                                    println!("\n\x1b[1mConnecting to \x1b[1;32m{}\x1b[0m...", db_name);
                                    println!();
                                    
                                    std::process::Command::new("tsh")
                                        .args(["db", "connect", db_name])
                                        .status()?;
                                } else {
                                    println!("\n❌ Failed to install MongoDB client");
                                }
                                return Ok(());
                            },
                            "n" | "no" => {
                                println!("\nMongoDB client installation skipped.");
                                return Ok(());
                            },
                            _ => {
                                println!("\n\x1b[31mInvalid input. Please enter y or n.\x1b[0m");
                                continue;
                            }
                        }
                    }
                }
            },
            "2" => {
                // Atlas GUI connection
                clear_screen()?;
                create_header("Atlas GUI");
                
                println!("Logging into: \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m", db_name, db_user);
                
                // Login to database
                let _login_result = std::process::Command::new("tsh")
                    .args(["db", "login", db_name, &format!("--db-user={}", db_user), "--db-name=admin"])
                    .output()?;
                
                println!("\n✅ \x1b[1;32mLogged in successfully!\x1b[0m");
                
                // Create proxy
                println!("\nCreating proxy for \x1b[1;32m{}\x1b[0m...", db_name);
                let mongo_port = crate::display::find_available_port();
                
                std::process::Command::new("tsh")
                    .args(["proxy", "db", "--tunnel", &format!("--port={}", mongo_port), db_name])
                    .spawn()?;
                
                // Open MongoDB Compass
                println!("\nOpening MongoDB compass...");
                std::process::Command::new("open")
                    .arg(&format!("mongodb://localhost:{}/?directConnection=true", mongo_port))
                    .status()?;
                
                return Ok(());
            },
            _ => {
                println!("\n\x1b[31mInvalid selection. Please enter 1 or 2.\x1b[0m");
                print!("\nSelect option (number): ");
                io::stdout().flush().unwrap();
                
                option.clear();
                io::stdin().read_line(&mut option).unwrap();
                continue;
            }
        }
    }
}

async fn show_connection_options(client: &TeleportClient, db_name: &str) -> Result<()> {
    print_info("Connection options:");
    
    // Try to get proxy information
    match client.get_db_proxy(db_name).await {
        Ok(proxy_info) => {
            println!("\n1. {} Connect via proxy:", "🔗".bright_blue());
            print_info(&format!("Proxy details: {}", proxy_info));
        }
        Err(_) => {
            print_info("Proxy information not available");
        }
    }
    
    // Show direct connection commands
    println!("\n2. {} Direct connection commands:", "💻".bright_blue());
    
    // PostgreSQL/MySQL commands
    println!("   PostgreSQL: {}", display_code(&format!("tsh db connect {}", db_name)));
    println!("   MySQL: {}", display_code(&format!("tsh db connect {} --db-user=root", db_name)));
    
    // MongoDB commands
    println!("   MongoDB: {}", display_code(&format!("tsh db connect {} --db-name=admin", db_name)));
    
    println!("\n3. {} GUI Tools:", "🖥️".bright_blue());
    println!("   You can also connect using GUI tools like:");
    println!("   - DBeaver");
    println!("   - pgAdmin (PostgreSQL)");
    println!("   - MongoDB Compass");
    
    println!("\n4. {} Environment variables:", "🔧".bright_blue());
    println!("   Use {} to get connection details", display_code("tsh db env"));

    Ok(())
}

fn show_help() {
    clear_screen().unwrap();
    create_header("th database | d");
    println!("Connect to databases (RDS/MongoDB).\n");
    println!("Usage: {} | {}", "th database [options]".bold(), "d".bold());
    println!(" ╚═ {}                     : Open interactive database selection.", "th d".bold());
    println!(" ╚═ {}         : Connect directly to specified database.\n", "th d <database>.".bold());
}