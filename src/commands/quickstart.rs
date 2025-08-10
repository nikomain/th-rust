use anyhow::Result;

pub async fn execute() -> Result<()> {
    let url = "https://youlend.atlassian.net/wiki/spaces/ISS/pages/1384972392/TH+-+Teleport+Helper+Quick+Start";
    
    match webbrowser::open(url) {
        Ok(_) => println!("Opening quickstart guide in your default browser..."),
        Err(e) => {
            eprintln!("Failed to open browser: {}", e);
            println!("Please visit: {}", url);
        }
    }
    
    Ok(())
}