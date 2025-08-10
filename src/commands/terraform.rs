use crate::cli::TerraArgs;
use crate::config::Config;
use crate::display::{clear_screen, create_header};
use crate::teleport::TeleportClient;
use anyhow::Result;
use colored::*;

pub async fn execute(args: TerraArgs, config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    let client = TeleportClient::new(config.clone());

    // Call th_login at start like bash version does
    crate::display::th_login().await?;
    
    clear_screen()?;
    create_header("Terragrunt Login");
    
    // Logout from existing AWS sessions
    let _ = client.aws_logout().await;
    
    println!("\x1b[1mLogging into \x1b[1;32myl-admin\x1b[0m \x1b[1mas\x1b[0m \x1b[1;32msudo_admin\x1b[0m");
    
    // Login to yl-admin with sudo_admin role - silently like bash version
    client.aws_login("yl-admin", "sudo_admin").await?;
    
    // Create proxy like bash version - now using public create_proxy function
    crate::commands::aws::create_proxy("yl-admin", "sudo_admin").await?;

    Ok(())
}

fn show_help() {
    clear_screen().unwrap();
    create_header("th terra | t");
    println!("Login to Terraform/Terragrunt with admin privileges.\n");
    println!("Usage: {} | {}", "th terra".bold(), "t".bold());
    println!(" ╚═ {}                       : Login to yl-admin as sudo_admin\n", "th t".bold());
    println!("This command will:");
    println!("  • Login to yl-admin AWS account");
    println!("  • Configure elevated sudo_admin role");
    println!("  • Set up AWS credentials proxy");
    println!("  • Enable Terraform/Terragrunt operations");
}