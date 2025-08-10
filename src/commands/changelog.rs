use anyhow::Result;
use crate::display::{clear_screen, create_header};

pub async fn execute() -> Result<()> {
    clear_screen()?;
    create_header("th changelog");
    
    println!("📝 Recent Changes:");
    println!();
    
    let update_checker = crate::update::UpdateChecker::new()?;
    
    match update_checker.fetch_changelog().await {
        Ok(changelog) => {
            println!("{}", changelog);
        }
        Err(_) => {
            println!("❌ Could not fetch changelog from GitHub.");
            println!("📖 You can view the full changelog at:");
            println!("   https://github.com/YouLend/th-rust/releases");
        }
    }
    
    Ok(())
}