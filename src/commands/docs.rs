use anyhow::Result;

pub async fn execute() -> Result<()> {
    let url = "https://youlend.atlassian.net/wiki/spaces/ISS/pages/1378517027/TH+-+Teleport+Helper+Docs";
    
    match webbrowser::open(url) {
        Ok(_) => println!("Opening documentation in your default browser..."),
        Err(e) => {
            eprintln!("Failed to open browser: {}", e);
            println!("Please visit: {}", url);
        }
    }
    
    Ok(())
}