# TH (Teleport Helper) - Rust Architecture Design

## Overview

This document outlines the Rust architecture for the "th" (Teleport Helper) CLI tool, designed to replicate and enhance the existing bash implementation while leveraging Rust's safety, performance, and ecosystem.

## Project Structure

```
src/
├── main.rs              # Entry point and CLI dispatcher
├── cli.rs               # Command-line argument parsing (Clap)
├── config.rs            # Configuration management
├── display.rs           # Terminal UI and formatting
├── error.rs             # Custom error types
├── process.rs           # External process management
├── shell.rs             # Shell integration and environment
├── teleport.rs          # Teleport API client
└── commands/            # Command implementations
    ├── mod.rs
    ├── aws.rs           # AWS account login
    ├── database.rs      # Database connections
    ├── kube.rs          # Kubernetes cluster login
    ├── terraform.rs     # Terraform/Terragrunt
    ├── login.rs         # Basic Teleport login
    ├── logout.rs        # Session cleanup
    ├── version.rs       # Version information
    ├── docs.rs          # Documentation links
    ├── quickstart.rs    # Quickstart guide
    └── animate.rs       # Visual effects
```

## Core Dependencies

### CLI Framework
- **clap** (4.5): Modern CLI parsing with derive macros
- **dialoguer** (0.11): Interactive prompts and menus
- **console** (0.15): Terminal detection and control

### Async Runtime
- **tokio** (1.0): Async runtime for I/O operations
- **tokio-process**: Process management

### Error Handling
- **anyhow**: Simplified error handling
- **thiserror**: Custom error type definitions

### Display & UI
- **colored** (2.0): Terminal colors and styling
- **crossterm** (0.27): Cross-platform terminal control
- **indicatif** (0.17): Progress bars and spinners

### Data Handling
- **serde** + **serde_json**: JSON parsing for tsh output
- **toml**: Configuration file format
- **config** (0.14): Configuration management

### System Integration
- **dirs** (5.0): Standard directories
- **which** (6.0): Executable discovery
- **tempfile** (3.0): Temporary file management
- **shellwords** (1.1): Shell command parsing

## Key Architectural Patterns

### 1. Error Handling Strategy

```rust
// Custom error types with context
#[derive(Error, Debug)]
pub enum ThError {
    #[error("Teleport authentication failed: {0}")]
    AuthFailed(String),
    
    #[error("Process execution failed: {0}")]
    Process(String),
    
    #[error("Timeout waiting for operation: {0}")]
    Timeout(String),
}

// Usage throughout the codebase
pub async fn execute_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output().await?;
    
    if !output.status.success() {
        return Err(ThError::Process(format!(
            "Command '{}' failed: {}",
            program, String::from_utf8_lossy(&output.stderr)
        )).into());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 2. Configuration Management

```rust
// Type-safe configuration with defaults
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub kube: HashMap<String, String>,    // env -> cluster mapping
    pub aws: HashMap<String, String>,     // env -> account mapping
    pub teleport: TeleportConfig,
    pub paths: PathsConfig,
}

// Automatic loading with fallback to defaults
impl Config {
    pub async fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            Ok(toml::from_str(&content)?)
        } else {
            let config = Config::default();
            config.save().await?;  // Create default config
            Ok(config)
        }
    }
}
```

### 3. Process Management

```rust
// Safe async process execution with timeout
pub async fn execute_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<String> {
    let future = execute_command(program, args);
    
    timeout(Duration::from_secs(timeout_secs), future)
        .await
        .map_err(|_| ThError::Timeout(format!("Command '{}' timed out", program)))?
}

// Background process management
pub async fn execute_background(program: &str, args: &[&str]) -> Result<Child> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    Ok(child)
}
```

### 4. Interactive UI

```rust
// Loading animations for long operations
pub async fn show_loading<F, R>(message: &str, operation: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));

    let result = operation.await;
    
    spinner.finish_and_clear();
    result
}

// Interactive menus
pub fn create_menu(title: &str, items: &[String]) -> io::Result<usize> {
    clear_screen()?;
    create_header(title);
    
    for (i, item) in items.iter().enumerate() {
        println!("{:2}. {}", i + 1, item);
    }
    
    // Handle user input with validation
    // ...
}
```

### 5. Shell Integration

```rust
pub struct ShellIntegration {
    shell_type: ShellType,
    profile_path: PathBuf,
}

impl ShellIntegration {
    // Automatic shell detection
    fn detect_shell() -> ShellType {
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") { ShellType::Zsh }
            else if shell.contains("bash") { ShellType::Bash }
            else if shell.contains("fish") { ShellType::Fish }
            else { ShellType::Unknown }
        } else {
            ShellType::Unknown
        }
    }
    
    // Safe profile modification
    pub async fn source_aws_credentials(&self, credentials_file: &str) -> Result<()> {
        let source_line = format!("source {}", credentials_file);
        self.add_to_profile(&source_line).await
    }
    
    // Cleanup on logout
    pub async fn cleanup_profile(&self) -> Result<()> {
        // Remove th-related entries from shell profile
        // ...
    }
}
```

## Specific Rust Approaches for Complex Parts

### 1. Interactive Menus with Error Recovery

```rust
pub async fn create_menu(title: &str, items: &[String]) -> io::Result<usize> {
    loop {
        clear_screen()?;
        create_header(title);
        
        for (i, item) in items.iter().enumerate() {
            println!("{:2}. {}", i + 1, item);
        }
        
        print!("Select option (number): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim().parse::<usize>() {
            Ok(choice) if choice > 0 && choice <= items.len() => {
                return Ok(choice - 1);
            }
            _ => {
                print_error("Invalid selection. Please try again.");
                sleep(Duration::from_secs(1)).await;
                // Loop continues for retry
            }
        }
    }
}
```

### 2. Background Process Management

```rust
pub struct ProcessManager {
    children: Vec<Child>,
}

impl ProcessManager {
    pub async fn spawn_proxy(&mut self, cluster: &str) -> Result<()> {
        let child = Command::new("tsh")
            .args(&["proxy", "kube", cluster])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
            
        self.children.push(child);
        Ok(())
    }
    
    pub async fn cleanup_all(&mut self) -> Result<()> {
        for child in &mut self.children {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.children.clear();
        Ok(())
    }
}
```

### 3. Shell Integration with Cross-Platform Support

```rust
impl ShellIntegration {
    pub async fn execute_in_shell(&self, command: &str) -> Result<String> {
        let (shell_cmd, args) = match self.shell_type {
            ShellType::Zsh => ("zsh", vec!["-c", command]),
            ShellType::Bash => ("bash", vec!["-c", command]),
            ShellType::Fish => ("fish", vec!["-c", command]),
            ShellType::Unknown => ("sh", vec!["-c", command]),
        };

        crate::process::execute_command(shell_cmd, &args).await
    }
    
    pub async fn set_env_var(&self, key: &str, value: &str) -> Result<()> {
        let export_cmd = match self.shell_type {
            ShellType::Fish => format!("set -x {} {}", key, value),
            _ => format!("export {}={}", key, value),
        };

        self.add_to_profile(&export_cmd).await
    }
}
```

## Command-Specific Architectures

### Kubernetes (`commands/kube.rs`)

```rust
pub async fn execute(args: KubeArgs, config: &Config) -> Result<()> {
    let client = TeleportClient::new(config.clone());
    
    // Ensure authenticated
    show_loading("Checking Teleport login...", client.login()).await?;
    
    match args.environment {
        Some(env) => quick_login(&client, config, &env).await,
        None => interactive_login(&client, config).await,
    }
}

async fn interactive_login(client: &TeleportClient, config: &Config) -> Result<()> {
    // Get clusters with access status
    let clusters = show_loading(
        "Checking cluster access...",
        client.list_kube_clusters()
    ).await?;
    
    // Create status-aware menu
    let menu_items: Vec<String> = clusters
        .iter()
        .map(|cluster| {
            if cluster.accessible {
                format!("{} ✅", cluster.name)
            } else {
                format!("{} ❌ (No access)", cluster.name)
            }
        })
        .collect();
    
    // Interactive selection and connection
    // ...
}
```

### AWS (`commands/aws.rs`)

```rust
pub async fn execute(args: AwsArgs, config: &Config) -> Result<()> {
    let client = TeleportClient::new(config.clone());
    
    // Clear existing sessions
    let _ = client.aws_logout().await;
    
    // Handle role selection (regular vs sudo)
    let role = if args.sudo {
        select_sudo_role(&client, account, env).await?
    } else {
        select_regular_role(&client, account, env).await?
    };
    
    // Setup credentials and shell integration
    setup_aws_credentials(&client, account, &role, env).await?;
}

async fn setup_aws_credentials(/* ... */) -> Result<()> {
    // Generate temporary credentials via tsh
    let aws_env = get_aws_credentials(client, account, role).await?;
    
    // Write to temporary file
    let temp_file = write_temp_credentials(&aws_env).await?;
    
    // Integrate with shell
    let shell = ShellIntegration::new().await?;
    shell.source_aws_credentials(&temp_file).await?;
    
    Ok(())
}
```

## Challenges and Solutions

### 1. **Challenge**: Interactive UI in async context
**Solution**: Use `dialoguer` for menus and `indicatif` for progress indicators, with proper async integration.

### 2. **Challenge**: Shell integration across platforms
**Solution**: Auto-detect shell type and use appropriate syntax for each shell (bash/zsh/fish).

### 3. **Challenge**: Process management and cleanup
**Solution**: RAII pattern with Drop trait for automatic cleanup, plus explicit cleanup methods.

### 4. **Challenge**: Teleport JSON parsing
**Solution**: Use `serde_json` for robust parsing with proper error handling for malformed data.

### 5. **Challenge**: Background proxy management
**Solution**: Tokio child processes with proper lifecycle management and timeout handling.

### 6. **Challenge**: Configuration management
**Solution**: Layered configuration with defaults, file-based overrides, and automatic creation.

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_config_loading() {
        let config = Config::default();
        assert!(config.get_kube_cluster("dev").is_some());
    }
    
    #[tokio::test]
    async fn test_process_execution() {
        let result = execute_command("echo", &["test"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "test");
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_full_kube_workflow() {
    // Mock teleport client
    // Test full kubernetes login flow
    // Verify kubectl config changes
}
```

### Benchmarks
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_config_loading(c: &mut Criterion) {
    c.bench_function("config_load", |b| {
        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                Config::load().await.unwrap()
            })
        })
    });
}

criterion_group!(benches, benchmark_config_loading);
criterion_main!(benches);
```

## Migration Path

1. **Phase 1**: Core infrastructure (config, process, display)
2. **Phase 2**: Basic commands (login, logout, version)
3. **Phase 3**: Interactive commands (kube, aws, database)
4. **Phase 4**: Advanced features (terraform, animations)
5. **Phase 5**: Performance optimization and polish

## Performance Considerations

- **Lazy loading**: Only load resources when needed
- **Concurrent operations**: Use async for I/O-bound operations  
- **Efficient JSON parsing**: Stream processing for large responses
- **Memory management**: Minimal allocations in hot paths
- **Process reuse**: Cache tsh client state when possible

This architecture provides a solid foundation for a production-quality Rust implementation that maintains feature parity with the bash version while providing better error handling, performance, and maintainability.