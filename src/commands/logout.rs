use crate::cli::LogoutArgs;
use crate::config::Config;
use crate::display;
use anyhow::Result;

pub async fn execute(args: LogoutArgs, _config: &Config) -> Result<()> {
    // Show help if requested
    if args.help {
        show_help();
        return Ok(());
    }

    // Use the exact bash th_kill function
    display::th_kill().await
}

async fn cleanup_kubectl_contexts() -> Result<()> {
    // Get current kubectl contexts
    let output = crate::process::execute_command("kubectl", &["config", "get-contexts", "-o", "name"]).await;
    
    if let Ok(contexts) = output {
        for context in contexts.lines() {
            // Remove contexts that look like Teleport contexts
            if context.contains("teleport") || context.contains("tsh-") {
                let _ = crate::process::execute_command_silent(
                    "kubectl",
                    &["config", "delete-context", context]
                ).await;
            }
        }
    }
    
    Ok(())
}

fn show_help() {
    crate::display::clear_screen().unwrap();
    crate::display::create_header("th logout | l");
    println!("Logout from all proxies, accounts & clusters.\n");
    println!("This command will:");
    println!("  • Logout from Teleport");
    println!("  • Logout from all AWS applications");
    println!("  • Terminate background proxy processes");
    println!("  • Remove temporary credential files");
    println!("  • Clean up shell profile entries");
    println!("  • Remove kubectl contexts");
}