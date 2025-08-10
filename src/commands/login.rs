use crate::cli::LoginArgs;
use crate::config::Config;
use crate::display;
use anyhow::Result;

pub async fn execute(args: LoginArgs, _config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    // Use the exact bash th_login function
    display::th_login().await
}

fn show_help() {
    crate::display::clear_screen().unwrap();
    crate::display::create_header("th login | li");
    println!("Log in to Teleport.\n");
    println!("This command performs a basic Teleport authentication using:");
    println!("  • Active Directory (AD) authentication");
    println!("  • Configured Teleport proxy server");
    println!("  • Interactive browser-based login\n");
    println!("Once logged in, you can access:");
    println!("  • Kubernetes clusters via 'th kube'");
    println!("  • AWS accounts via 'th aws'");
    println!("  • Database connections via 'th database'");
    println!("  • Administrative tools via 'th terra'");
}