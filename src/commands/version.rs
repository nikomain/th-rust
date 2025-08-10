use anyhow::Result;

pub async fn execute() -> Result<()> {
    println!("{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}