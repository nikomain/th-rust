use crate::cli::AnimateArgs;
use crate::display::{animate_youlend, animate_th};
use anyhow::Result;

pub async fn execute(args: AnimateArgs) -> Result<()> {
    match args.animation.as_deref() {
        Some("yl") => {
            animate_youlend().await;
        }
        Some("th") | None => {
            animate_th().await;
        }
        Some(other) => {
            eprintln!("Unknown animation: {}", other);
            eprintln!("Available animations: yl, th");
        }
    }
    
    Ok(())
}