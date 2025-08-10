use crate::cli::KubeArgs;
use crate::config::Config;
use crate::display::{clear_screen, create_header, create_menu, show_loading, print_success, print_error, print_info, display_code};
use crate::teleport::TeleportClient;
use anyhow::Result;
use colored::*;

pub async fn execute(args: KubeArgs, config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    let client = TeleportClient::new(config.clone());

    // Ensure logged in to Teleport
    show_loading("Checking Teleport login...", client.login()).await?;

    // Direct login if environment specified
    if let Some(env) = args.environment {
        return quick_login(&client, config, &env).await;
    }

    // Interactive cluster selection
    interactive_login(&client, config).await
}

async fn quick_login(client: &TeleportClient, config: &Config, env: &str) -> Result<()> {
    if let Some(cluster_name) = config.get_kube_cluster(env) {
        clear_screen()?;
        create_header("Kube Login");
        
        println!("Logging you into: \x1b[1;32m{}\x1b[0m", cluster_name);
        
        client.kube_login(cluster_name).await?;
        
        println!("\n✅ Logged in successfully!\n");
        
        Ok(())
    } else {
        println!("\n\x1b[31mUnknown environment: {}\x1b[0m", env);
        println!("Available environments: dev, sandbox, staging, usstaging, admin, prod, usprod, corepgblue, corepggreen");
        Ok(())
    }
}

async fn interactive_login(client: &TeleportClient, _config: &Config) -> Result<()> {
    clear_screen()?;
    create_header("Available Clusters");
    
    // Get available clusters
    let clusters = show_loading(
        "Checking cluster access...",
        client.list_kube_clusters()
    ).await?;

    if clusters.is_empty() {
        print_error("No Kubernetes clusters available");
        return Ok(());
    }

    // Display clusters with bash-style formatting
    for (i, cluster) in clusters.iter().enumerate() {
        if cluster.accessible && !cluster.name.contains("prod") {
            // n/a case - normal display
            println!("{:2}. {}", i + 1, cluster.name);
        } else if cluster.accessible && cluster.name.contains("prod") {
            // ok case - normal display
            println!("{:2}. {}", i + 1, cluster.name);
        } else {
            // fail case - grayed out
            println!("\x1b[90m{:2}. {}\x1b[0m", i + 1, cluster.name);
        }
    }

    // Get user choice - exactly like bash
    use std::io::{self, Write};
    print!("\n\x1b[1mSelect cluster (number):\x1b[0m ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    if choice.trim().is_empty() {
        println!("No selection made. Exiting.");
        return Ok(());
    }
    
    let selected_index: usize = choice.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
        
    if selected_index == 0 || selected_index > clusters.len() {
        println!("\n\x1b[31mInvalid selection\x1b[0m");
        return Ok(());
    }
    
    let selected_cluster = &clusters[selected_index - 1];
    
    // Handle elevated access case for prod clusters
    if !selected_cluster.accessible && selected_cluster.name.contains("prod") {
        kube_elevated_login(client, &selected_cluster.name).await?;
        return Ok(());
    }
    
    // Normal login
    println!("\n\x1b[1mLogging you into:\x1b[0m \x1b[1;32m{}\x1b[0m", selected_cluster.name);
    client.kube_login(&selected_cluster.name).await?;
    println!("\n✅ \x1b[1mLogged in successfully!\x1b[0m\n");

    Ok(())
}

async fn kube_elevated_login(client: &TeleportClient, cluster: &str) -> Result<()> {
    use std::io::{self, Write};
    
    loop {
        clear_screen()?;
        create_header("Privilege Request");
        
        println!("\n\nYou don't have write access to \x1b[1m{}\x1b[0m.", cluster);
        println!("\n\x1b[1mWould you like to raise a request?\x1b[0m");
        println!("\n\x1b[1mNote:\x1b[0m Entering (N/n) will log you in as a read-only user.");
        print!("\n(Yy/Nn): ");
        io::stdout().flush().unwrap();
        
        let mut elevated = String::new();
        io::stdin().read_line(&mut elevated).unwrap();
        let elevated = elevated.trim().to_lowercase();
        
        match elevated.as_str() {
            "y" | "yes" => {
                print!("\n\x1b[1mEnter your reason for request:\x1b[0m ");
                io::stdout().flush().unwrap();
                
                let mut reason = String::new();
                io::stdin().read_line(&mut reason).unwrap();
                let reason = reason.trim();
                
                let role = match cluster {
                    "live-prod-eks-blue" => "sudo_prod_eks_cluster",
                    "live-usprod-eks-blue" => "sudo_usprod_eks_cluster", 
                    _ => {
                        println!("\nCluster doesn't exist");
                        return Ok(());
                    }
                };
                
                let output = std::process::Command::new("tsh")
                    .args(["request", "create", "--roles", role, "--reason", reason])
                    .output()?;
                    
                let output_text = String::from_utf8_lossy(&output.stdout);
                println!("{}", output_text);
                
                // Extract request ID
                let _request_id = output_text.lines()
                    .find(|line| line.contains("Request ID:"))
                    .and_then(|line| line.split_whitespace().nth(2));
                
                println!("\n\n✅ \x1b[1;32mAccess request sent!\x1b[0m\n\n");
                return Ok(());
            },
            "n" | "no" => {
                println!("\nRequest creation skipped.");
                return Ok(());
            },
            _ => {
                println!("\n\x1b[31mInvalid input. Please enter y or n.\x1b[0m");
                continue;
            }
        }
    }
}

fn show_help() {
    clear_screen().unwrap();
    create_header("th kube | k");
    println!("Login to our Kubernetes clusters.\n");
    println!("Usage: {} | {}", "th kube [options]".bold(), "k".bold());
    println!(" ╚═ {}                 : Open interactive login.", "th k".bold());
    println!(" ╚═ {}       : Quick kube log-in, Where {} = dev, staging, etc..\n", "th k <account>".bold(), "<account>".bold());
    println!("Examples:");
    println!(" ╚═ {}             : logs you into {}.", display_code("th k dev"), "aslive-dev-eks-blue".green());
}