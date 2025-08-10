use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod config;
mod display;
mod error;
mod process;
mod shell;
mod teleport;
mod update;

use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration
    let config = Config::load().await?;
    
    // Check for updates in background (non-blocking)
    if let Ok(update_checker) = crate::update::UpdateChecker::new() {
        let _ = update_checker.check_for_updates_background().await;
    }
    
    // Validate that required tools are available (for commands that need them)
    let cli = Cli::parse();
    
    // Only validate teleport tools for commands that need them
    match &cli.command {
        Some(Commands::Kube(_)) | Some(Commands::Aws(_)) | Some(Commands::Database(_)) | Some(Commands::Login(_)) | Some(Commands::Logout(_)) => {
            let client = crate::teleport::TeleportClient::new(config.clone());
            if let Err(e) = client.validate_tools().await {
                eprintln!("Error: {}", e);
                eprintln!("Make sure Teleport (tsh) is installed and in your PATH");
                std::process::exit(1);
            }
        }
        _ => {
            // Commands like version, docs, quickstart don't need teleport, or no command (help)
        }
    }
    
    // Check if help flag was provided
    if cli.help {
        display::print_help("1.5.0");
        return Ok(());
    }
    
    // Execute command  
    let result = match cli.command {
        None => {
            // No command provided, show original help screen (like bash version)
            display::print_help("1.5.0");
            Ok(())
        }
        Some(Commands::Kube(kube_args)) => {
            commands::kube::execute(kube_args, &config).await
        }
        Some(Commands::Aws(aws_args)) => {
            commands::aws::execute(aws_args, &config).await
        }
        Some(Commands::Database(db_args)) => {
            commands::database::execute(db_args, &config).await
        }
        Some(Commands::Terra(terra_args)) => {
            commands::terraform::execute(terra_args, &config).await
        }
        Some(Commands::Login(login_args)) => {
            commands::login::execute(login_args, &config).await
        }
        Some(Commands::Logout(logout_args)) => {
            commands::logout::execute(logout_args, &config).await
        }
        Some(Commands::Version) => {
            commands::version::execute().await
        }
        Some(Commands::Docs) => {
            commands::docs::execute().await
        }
        Some(Commands::Quickstart) => {
            commands::quickstart::execute().await
        }
        Some(Commands::Animate(animate_args)) => {
            commands::animate::execute(animate_args).await
        }
        Some(Commands::Loader) => {
            crate::display::demo_wave_loader(None).await;
            Ok(())
        }
        Some(Commands::Update) => {
            commands::update::execute().await
        }
        Some(Commands::Changelog) => {
            commands::changelog::execute().await
        }
        Some(Commands::ClearUpdateCache) => {
            commands::update::clear_cache().await
        }
    };
    
    // Show update notification after command completion (end-of-flow)
    if let Ok(update_checker) = crate::update::UpdateChecker::new() {
        if let Ok(Some(_notification)) = update_checker.should_show_update_notification().await {
            // Extract version numbers from the cache
            if let Ok(cache) = update_checker.get_update_cache().await {
                if cache.update_available {
                    if let Some(latest_version) = cache.latest_version {
                        display::create_update_notification(&update_checker.get_current_version(), &latest_version);
                    }
                }
            }
        }
    }
    
    result
}