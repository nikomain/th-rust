use crate::cli::AwsArgs;
use crate::config::Config;
use crate::display::{clear_screen, create_header, create_menu, show_loading, print_success, print_error, print_info, display_code, create_note};
use crate::shell::ShellIntegration;
use crate::teleport::TeleportClient;
use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn execute(args: AwsArgs, config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    let client = TeleportClient::new(config.clone());

    // Call th_login at start like bash version does
    crate::display::th_login().await?;

    // Logout from any existing AWS sessions
    let _ = client.aws_logout().await;

    // Direct login if environment specified
    if let Some(env) = args.environment {
        let use_sudo = args.sudo_flag.as_deref() == Some("s");
        return quick_login(&client, config, &env, use_sudo).await;
    }

    // Interactive AWS app selection  
    let use_sudo = args.sudo_flag.as_deref() == Some("s");
    interactive_login(&client, config, use_sudo).await
}

async fn quick_login(client: &TeleportClient, config: &Config, env: &str, use_sudo: bool) -> Result<()> {
    if let Some(account_name) = config.get_aws_account(env) {
        clear_screen()?;
        create_header("AWS Login");
        
        // Handle role selection
        let role = if use_sudo {
            select_sudo_role(client, account_name, env).await?
        } else {
            select_regular_role(client, account_name, env).await?
        };
        
        // Display exactly like bash version
        println!("Logging you into: \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m", account_name, role);
        
        // Logout first, then login with role - exactly like bash version
        let _ = client.aws_logout().await;
        client.aws_login(account_name, &role).await?;
        
        println!("\n✅ Logged in successfully!");
        
        
        // Create proxy and source credentials - exactly like bash create_proxy function
        create_proxy(account_name, &role).await?;
        
        Ok(())
    } else {
        print_error(&format!("Environment '{}' not found in configuration", env));
        print_info("Available environments:");
        for env in config.list_aws_envs() {
            println!("  - {}", env);
        }
        Ok(())
    }
}

async fn interactive_login(client: &TeleportClient, config: &Config, use_sudo: bool) -> Result<()> {
    clear_screen()?;
    create_header("AWS Accounts");
    
    // Get available AWS apps
    let apps = show_loading(
        "Fetching AWS applications...",
        client.list_aws_apps()
    ).await?;

    if apps.is_empty() {
        print_error("No AWS applications available");
        return Ok(());
    }

    // Create menu items
    let menu_items: Vec<String> = apps
        .iter()
        .map(|app| app.name.clone())
        .collect();

    // Show interactive menu
    let selection = create_menu("Available Accounts", &menu_items).await?;
    let selected_app = &apps[selection];

    clear_screen()?;
    create_header("AWS Login");
    
    print_info(&format!("Connecting to AWS account: {}", selected_app.name));
    
    // Logout to force fresh AWS role output - exactly like bash
    let _ = client.aws_logout().await;
    
    // Run tsh apps login to capture AWS roles (will error but shows roles) - exactly like bash
    let login_output = std::process::Command::new("tsh")
        .args(["apps", "login", &selected_app.name])
        .output();
        
    let output_text = match login_output {
        Ok(output) => {
            // Combine both stdout and stderr like the bash version does with 2>&1
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("{}{}", stdout, stderr)
        },
        Err(_) => return Err(anyhow::anyhow!("Failed to get AWS roles")),
    };
    
    // Extract AWS roles section - exactly like bash awk command
    let role_section = extract_roles_section(&output_text);

    // Extract default role from ARN - exactly like bash
    let default_role = extract_default_role(&output_text);
    
    if role_section.is_empty() {
        // Handle case with only default role - exactly like bash aws_elevated_login
        if let Some(role) = default_role {
            // Always show elevated login prompt when there's only one role available
            return aws_elevated_login(client, &selected_app.name, &role).await;
        }
        return Err(anyhow::anyhow!("No AWS roles available"));
    }
    
    // Parse roles list - skip first 2 lines (headers) exactly like bash
    let roles_lines: Vec<&str> = role_section.lines().skip(2).collect();
    let roles_list: Vec<String> = roles_lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.contains("---------")) // Skip separator lines
        .map(|line| line.split_whitespace().next().unwrap_or(line).to_string())
        .filter(|role| !role.is_empty())
        .collect();
    
    if roles_list.is_empty() {
        return Err(anyhow::anyhow!("No roles found in output"));
    }
    
    // Display role selection menu - exactly like bash
    clear_screen()?;
    create_header("Available Roles");
    
    for (i, role) in roles_list.iter().enumerate() {
        println!("{}. {}", i + 1, role);
    }
    
    // Get user role choice - exactly like bash
    use std::io::{self, Write};
    print!("\nSelect role (number): ");
    io::stdout().flush().unwrap();
    
    let mut role_input = String::new();
    io::stdin().read_line(&mut role_input).unwrap();
    
    let role_choice: usize = role_input.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
        
    if role_choice == 0 || role_choice > roles_list.len() {
        return Err(anyhow::anyhow!("Invalid selection"));
    }
    
    let selected_role = &roles_list[role_choice - 1];
    
    // Login with selected role - exactly like bash
    println!("\nLogging you into \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m", selected_app.name, selected_role);
    client.aws_login(&selected_app.name, selected_role).await?;
    println!("\n✅\x1b[1;32m Logged in successfully!\x1b[0m");
    
    create_proxy(&selected_app.name, selected_role).await?;
    Ok(())
}

async fn select_regular_role(_client: &TeleportClient, _account: &str, env: &str) -> Result<String> {
    // Map environment to role value - exactly like bash version
    let role_value = match env {
        "dev" => "dev",
        "corepg" => "coreplayground", 
        _ => env,
    };
    
    Ok(role_value.to_string())
}

async fn select_sudo_role(_client: &TeleportClient, _account: &str, env: &str) -> Result<String> {
    // Map environment to role value, then add sudo_ prefix - exactly like bash version
    let role_value = match env {
        "dev" => "dev",
        "corepg" => "coreplayground",
        _ => env,
    };
    
    Ok(format!("sudo_{}", role_value))
}

async fn aws_elevated_login(client: &TeleportClient, app: &str, default_role: &str) -> Result<()> {
    use std::io::{self, Write};
    
    clear_screen()?;
    create_header("Privilege Request");
    println!("No privileged roles found. Your only available role is: \x1b[1;32m{}\x1b[0m", default_role);
    
    loop {
        println!("\n\x1b[1mWould you like to raise a privilege request?\x1b[0m");
        create_note(&format!("Entering (N/n) will log you in as \x1b[1;32m{}\x1b[0m. ", default_role));
        print!("(Yy/Nn): ");
        io::stdout().flush().unwrap();
        
        let mut request = String::new();
        io::stdin().read_line(&mut request).unwrap();
        let request = request.trim().to_lowercase();
        
        match request.as_str() {
            "y" | "yes" => {
                print!("\n\x1b[1mEnter request reason:\x1b[0m ");
                io::stdout().flush().unwrap();
                
                let mut reason = String::new();
                io::stdin().read_line(&mut reason).unwrap();
                let reason = reason.trim();
                
                // Determine role based on app name - exactly like bash
                let role = if app == "yl-production" {
                    "sudo_prod_role"
                } else {
                    "sudo_usprod_role"
                };
                
                // Create privilege request
                let output = std::process::Command::new("tsh")
                    .args(["request", "create", "--roles", role, "--reason", reason])
                    .output()?;
                    
                let output_text = String::from_utf8_lossy(&output.stdout);
                println!("{}", output_text); // Show output to user like bash tee /dev/tty
                
                // Extract request ID
                let request_id = output_text.lines()
                    .find(|line| line.contains("Request ID:"))
                    .and_then(|line| line.split_whitespace().nth(2));
                
                if let Some(request_id) = request_id {
                    println!("\n\n✅ \x1b[1;32mAccess request sent!\x1b[0m\n\n");
                    
                    // Re-authenticate with request ID - exactly like bash
                    println!("\n\x1b[1mRe-Authenticating\x1b[0m\n");
                    let _ = std::process::Command::new("tsh").args(["logout"]).output();
                    let _ = std::process::Command::new("tsh")
                        .args(["login", "--auth=ad", "--proxy=youlend.teleport.sh:443", &format!("--request-id={}", request_id)])
                        .output();
                        
                    println!("✅ Re-authentication complete. Please run the command again to use elevated permissions.");
                    return Ok(());
                } else {
                    println!("Failed to extract request ID from output");
                    return Ok(());
                }
            },
            "n" | "no" => {
                // Log in with default role - exactly like bash
                println!("\nLogging you into \x1b[1;32m{}\x1b[0m as \x1b[1;32m{}\x1b[0m", app, default_role);
                client.aws_login(app, default_role).await?;
                println!("\n✅\x1b[1;32m Logged in successfully!\x1b[0m");
                create_proxy(app, default_role).await?;
                return Ok(());
            },
            _ => {
                println!("\n\x1b[31mInvalid input. Please enter y or n.\x1b[0m");
                continue;
            }
        }
    }
}


/// Create proxy & source credentials - exactly like bash create_proxy function
pub async fn create_proxy(app: &str, role_name: &str) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::fs;
    use regex::Regex;
    
    if app.is_empty() {
        return Err(anyhow::anyhow!("No active app found. Run 'tsh apps login <app>' first."));
    }

    let log_file = format!("/tmp/tsh_proxy_{}.log", app);

    // Clean up existing credential files - exactly like bash
    println!("Cleaned up existing credential files.");

    println!("\nStarting AWS proxy for \x1b[1;32m{}\x1b[0m...", app);

    // Start tsh proxy aws and redirect output to log file - exactly like bash
    use std::process::Stdio as StdStdio;
    let log_file_for_redirect = log_file.clone();
    let mut child = std::process::Command::new("tsh")
        .args(&["proxy", "aws", "--app", app])
        .stdout(std::fs::File::create(&log_file_for_redirect)?)
        .stderr(StdStdio::null())
        .spawn()?;

    // Wait up to 10 seconds for credentials to appear - exactly like bash
    let mut wait_time = 0;
    while wait_time < 20 {
        if let Ok(content) = fs::read_to_string(&log_file).await {
            // Look for the exact pattern the bash version uses - with leading spaces
            if content.contains("  export AWS_ACCESS_KEY_ID=") {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        wait_time += 1;
        
        if wait_time >= 20 {
            return Err(anyhow::anyhow!("Timed out waiting for AWS credentials."));
        }
    }

    // Filter to retain only export lines - exactly like bash
    if let Ok(content) = fs::read_to_string(&log_file).await {
        let export_regex = Regex::new(r"^[[:space:]]*export ").unwrap();
        let export_lines: Vec<&str> = content.lines()
            .filter(|line| export_regex.is_match(line))
            .collect();
            
        let filtered_content = export_lines.join("\n");
        fs::write(&log_file, &filtered_content).await?;
    }

    // Add ACCOUNT and ROLE exports - exactly like bash
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&log_file)
        .await?;
    
    file.write_all(b"\n").await?; // Add newline for proper formatting
    file.write_all(format!("export ACCOUNT={}\n", app).as_bytes()).await?;
    file.write_all(format!("export ROLE={}\n", role_name).as_bytes()).await?;

    // Set region based on app name - exactly like bash
    if app.starts_with("yl-us") {
        file.write_all(b"export AWS_DEFAULT_REGION=us-east-2\n").await?;
    } else {
        file.write_all(b"export AWS_DEFAULT_REGION=eu-west-1\n").await?;
    }

    // Set environment variables directly in current process AND add to shell profile
    if let Ok(content) = fs::read_to_string(&log_file).await {
        // Parse and set environment variables in current process
        for line in content.lines() {
            if line.trim().starts_with("export ") {
                if let Some(export_part) = line.trim().strip_prefix("export ") {
                    if let Some(eq_pos) = export_part.find('=') {
                        let key = &export_part[..eq_pos];
                        let value = &export_part[eq_pos + 1..].trim_matches('"');
                        std::env::set_var(key, value);
                    }
                }
            }
        }
    }

    // Add source line to shell profile - exactly like bash  
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = std::path::Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let shell_profile = match shell_name {
        "zsh" => format!("{}/.zshrc", std::env::var("HOME").unwrap_or_default()),
        "bash" => format!("{}/.bash_profile", std::env::var("HOME").unwrap_or_default()),
        _ => format!("{}/.profile", std::env::var("HOME").unwrap_or_default()),
    };

    // Remove existing tsh source lines and add new one - exactly like bash
    if let Ok(content) = fs::read_to_string(&shell_profile).await {
        let lines: Vec<&str> = content.lines()
            .filter(|line| !line.starts_with("source /tmp/tsh"))
            .collect();
        let mut new_content = lines.join("\n");
        new_content.push_str(&format!("\nsource {}\n", log_file));
        fs::write(&shell_profile, new_content).await?;
    }

    println!("\nCredentials exported, and made global, for app: \x1b[1;32m{}\x1b[0m\n", app);
    
    Ok(())
}

fn find_env_for_account(config: &Config, account_name: &str) -> String {
    for (env, name) in &config.aws {
        if name == account_name {
            return env.clone();
        }
    }
    "unknown".to_string()
}

// Extract roles section from tsh output - exactly like bash awk command
fn extract_roles_section(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut in_roles_section = false;
    let mut roles_section = Vec::new();
    
    for line in lines {
        if line.contains("Available AWS roles:") {
            in_roles_section = true;
            roles_section.push(line); // Include the header line
            continue;
        }
        if line.contains("ERROR: --aws-role flag is required") {
            break;
        }
        if in_roles_section && !line.contains("ERROR:") {
            roles_section.push(line);
        }
    }
    
    roles_section.join("\n")
}

// Extract default role from ARN - exactly like bash grep -o command
fn extract_default_role(output: &str) -> Option<String> {
    // Look for ARN pattern: arn:aws:iam::account:role/RoleName
    for line in output.lines() {
        if line.contains("arn:aws:iam::") {
            if let Some(arn_start) = line.find("arn:aws:iam::") {
                let arn_part = &line[arn_start..];
                // Handle both cases: ARN at end of line or ARN followed by whitespace
                let arn = if let Some(arn_end) = arn_part.find(char::is_whitespace) {
                    &arn_part[..arn_end]
                } else {
                    arn_part.trim()
                };
                if let Some(role_part) = arn.split('/').last() {
                    return Some(role_part.to_string());
                }
            }
        }
    }
    None
}


fn show_help() {
    clear_screen().unwrap();
    create_header("th aws | a");
    println!("Login to our AWS accounts.\n");
    println!("Usage: {} | {}", "th aws [options]".bold(), "a".bold());
    println!(" ╚═ {}                : Open interactive login.", "th a".bold());
    println!(" ╚═ {}  : Quick log-in, Where {} = dev, staging, etc..", "th a <account> <s>".bold(), "<account>".bold());
    println!("                          and {} is an optional arg which logs you in with", "<s>".bold());
    println!("                          the account's sudo role\n");
    println!("Examples:");
    println!(" ╚═ {}            : logs you into {} as {}", display_code("th a dev"), "yl-development".green(), "dev".underline().green());
    println!(" ╚═ {}          : logs you into {} as {}", display_code("th a dev s"), "yl-development".green(), "sudo_dev".underline().green());
}