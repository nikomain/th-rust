use clap::{Parser, Subcommand, Args};

/// Teleport Helper - CLI for managing Teleport logins
#[derive(Parser)]
#[command(name = "th")]
#[command(version = "1.5.0")]
#[command(about = "A CLI for managing Teleport logins and cloud resources")]
#[command(disable_help_flag = true)]
pub struct Cli {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
    
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Login to Kubernetes clusters
    #[command(alias = "k")]
    Kube(KubeArgs),
    
    /// Login to AWS accounts
    #[command(alias = "a")]
    Aws(AwsArgs),
    
    /// Connect to databases
    #[command(alias = "d")]
    Database(DatabaseArgs),
    
    /// Login to Terraform/Terragrunt
    #[command(alias = "t")]
    Terra(TerraArgs),
    
    /// Basic Teleport login
    #[command(alias = "li")]
    Login(LoginArgs),
    
    /// Cleanup all sessions
    #[command(alias = "l")]
    Logout(LogoutArgs),
    
    /// Show version information  
    #[command(short_flag = 'v')]
    Version,
    
    /// Open documentation
    Docs,
    
    /// Open quickstart guide
    #[command(alias = "qs")]
    Quickstart,
    
    /// Display animations
    Animate(AnimateArgs),
    
    /// Run loader animation
    Loader,
    
    /// Update th to the latest version
    Update,
    
    /// Show changelog for recent updates
    Changelog,
    
    /// Clear update cache (for testing)
    #[command(hide = true)]
    ClearUpdateCache,
}

#[derive(Args)]
pub struct KubeArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
    
    /// Environment to connect to (dev, staging, prod, etc.)
    pub environment: Option<String>,
}

#[derive(Args)]
pub struct AwsArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
    
    /// Environment to connect to (dev, staging, prod, etc.)
    pub environment: Option<String>,
    
    /// Sudo flag - pass "s" to use sudo role (exactly like bash version)
    pub sudo_flag: Option<String>,
}

#[derive(Args)]
pub struct DatabaseArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
    
    /// Database identifier or environment
    pub target: Option<String>,
}

#[derive(Args)]
pub struct TerraArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
}

#[derive(Args)]
pub struct LoginArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
}

#[derive(Args)]
pub struct LogoutArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
}

#[derive(Args)]
pub struct AnimateArgs {
    /// Show help information
    #[arg(short = 'h', long = "help")]
    pub help: bool,
    
    /// Animation type (yl, th)
    pub animation: Option<String>,
}