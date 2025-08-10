use crate::error::ThError;
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Shell integration for managing environment variables and profiles
pub struct ShellIntegration {
    shell_type: ShellType,
    profile_path: PathBuf,
}

#[derive(Debug, Clone)]
enum ShellType {
    Bash,
    Zsh,
    Fish,
    Unknown,
}

impl ShellIntegration {
    pub async fn new() -> Result<Self> {
        let shell_type = Self::detect_shell();
        let profile_path = Self::get_profile_path(&shell_type)?;
        
        Ok(Self {
            shell_type,
            profile_path,
        })
    }

    /// Detect the current shell
    fn detect_shell() -> ShellType {
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") {
                return ShellType::Zsh;
            } else if shell.contains("bash") {
                return ShellType::Bash;
            } else if shell.contains("fish") {
                return ShellType::Fish;
            }
        }
        ShellType::Unknown
    }

    /// Get the shell profile path
    fn get_profile_path(shell_type: &ShellType) -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| ThError::Shell("Could not determine home directory".to_string()))?;

        let profile_name = match shell_type {
            ShellType::Zsh => ".zshrc",
            ShellType::Bash => ".bash_profile",
            ShellType::Fish => ".config/fish/config.fish",
            ShellType::Unknown => ".profile",
        };

        Ok(home.join(profile_name))
    }

    /// Source AWS credentials in shell profile
    pub async fn source_aws_credentials(&self, credentials_file: &str) -> Result<()> {
        let source_line = match self.shell_type {
            ShellType::Fish => format!("source {}", credentials_file),
            _ => format!("source {}", credentials_file),
        };

        self.add_to_profile(&source_line).await
    }

    /// Add content to shell profile
    async fn add_to_profile(&self, content: &str) -> Result<()> {
        // Read existing profile
        let mut existing_content = String::new();
        if self.profile_path.exists() {
            let mut file = fs::File::open(&self.profile_path).await?;
            file.read_to_string(&mut existing_content).await?;
        }

        // Check if content already exists
        if existing_content.contains(content) {
            return Ok(());
        }

        // Append new content
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.profile_path)
            .await?;

        file.write_all(format!("\n# Added by th\n{}\n", content).as_bytes()).await?;
        file.sync_all().await?;

        Ok(())
    }

    /// Remove th-related content from shell profile
    pub async fn cleanup_profile(&self) -> Result<()> {
        if !self.profile_path.exists() {
            return Ok(());
        }

        let mut content = String::new();
        let mut file = fs::File::open(&self.profile_path).await?;
        file.read_to_string(&mut content).await?;
        drop(file);

        // Remove lines related to th
        let cleaned_content: String = content
            .lines()
            .filter(|line| !self.is_th_related_line(line))
            .map(|line| format!("{}\n", line))
            .collect();

        // Write cleaned content back
        let mut file = fs::File::create(&self.profile_path).await?;
        file.write_all(cleaned_content.as_bytes()).await?;
        file.sync_all().await?;

        Ok(())
    }

    /// Check if a line is related to th
    fn is_th_related_line(&self, line: &str) -> bool {
        line.contains("# Added by th") ||
        line.contains("yl_aws_credentials") ||
        line.contains("TH_AWS_") ||
        line.contains("export AWS_") && line.contains("teleport")
    }

    /// Clean up temporary credential files
    pub async fn cleanup_temp_files() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        
        // Find and remove th-related temp files
        let mut entries = fs::read_dir(&temp_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            if let Some(name) = file_name.to_str() {
                if name.starts_with("yl_") || 
                   name.starts_with("tsh_") || 
                   name.starts_with("admin_") {
                    let _ = fs::remove_file(entry.path()).await;
                }
            }
        }

        Ok(())
    }

    /// Execute a shell command with the current environment
    pub async fn execute_in_shell(&self, command: &str) -> Result<String> {
        let shell_cmd = match self.shell_type {
            ShellType::Zsh => "zsh",
            ShellType::Bash => "bash",
            ShellType::Fish => "fish",
            ShellType::Unknown => "sh",
        };

        let args = match self.shell_type {
            ShellType::Fish => vec!["-c", command],
            _ => vec!["-c", command],
        };

        crate::process::execute_command(shell_cmd, &args).await
    }

    /// Get environment variables from shell
    pub async fn get_shell_env(&self) -> Result<std::collections::HashMap<String, String>> {
        let output = self.execute_in_shell("env").await?;
        
        let mut env_vars = std::collections::HashMap::new();
        
        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                env_vars.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(env_vars)
    }

    /// Set environment variable in current shell session
    pub async fn set_env_var(&self, key: &str, value: &str) -> Result<()> {
        let export_cmd = match self.shell_type {
            ShellType::Fish => format!("set -x {} {}", key, value),
            _ => format!("export {}={}", key, value),
        };

        self.add_to_profile(&export_cmd).await
    }

    /// Unset environment variable
    pub async fn unset_env_var(&self, key: &str) -> Result<()> {
        let unset_cmd = match self.shell_type {
            ShellType::Fish => format!("set -e {}", key),
            _ => format!("unset {}", key),
        };

        self.execute_in_shell(&unset_cmd).await?;
        Ok(())
    }

    /// Check if shell supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "colors" => !matches!(self.shell_type, ShellType::Unknown),
            "completion" => matches!(self.shell_type, ShellType::Zsh | ShellType::Bash),
            "functions" => !matches!(self.shell_type, ShellType::Unknown),
            _ => false,
        }
    }
}