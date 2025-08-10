use anyhow::Result;
use crate::display::{clear_screen, create_header};
use dirs;

pub async fn execute() -> Result<()> {
    clear_screen()?;
    create_header("th update");
    
    let update_checker = crate::update::UpdateChecker::new()?;
    
    println!("🔄 Checking for updates...");
    
    match update_checker.install_update().await {
        Ok(()) => {
            println!("\n✅ Update completed successfully!");
        }
        Err(e) => {
            eprintln!("❌ Update failed: {}", e);
            eprintln!("You can try again later or update manually from GitHub.");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

pub async fn clear_cache() -> Result<()> {
    println!("🧹 Clearing update cache...");
    
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let cache_path = home.join(".th_update_check");
    
    if cache_path.exists() {
        tokio::fs::remove_file(cache_path).await?;
        println!("✅ Update cache cleared. Next command will check for updates.");
    } else {
        println!("ℹ️  No update cache found.");
    }
    
    Ok(())
}