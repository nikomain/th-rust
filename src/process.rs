use crate::error::ThError;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Output, Stdio};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Execute a command and return stdout as string
pub async fn execute_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ThError::Process(format!(
            "Command '{}' failed: {}",
            program, stderr
        )).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute command and return full Output (including stderr)
pub async fn execute_command_with_output(program: &str, args: &[&str]) -> Result<ProcessOutput> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    })
}

/// Process output structure
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Execute command with timeout
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

/// Execute command silently (suppress output)
pub async fn execute_command_silent(program: &str, args: &[&str]) -> Result<bool> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await?;

    Ok(output.status.success())
}

/// Execute command and return JSON output
pub async fn execute_command_json(program: &str, args: &[&str]) -> Result<Value> {
    let output = execute_command(program, args).await?;
    let json: Value = serde_json::from_str(&output)?;
    Ok(json)
}

/// Execute command interactively (inherit stdio)
pub async fn execute_command_interactive(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        return Err(ThError::Process(format!(
            "Interactive command '{}' failed with exit code: {:?}",
            program, status.code()
        )).into());
    }

    Ok(())
}

/// Kill processes by pattern
pub async fn kill_processes_by_pattern(pattern: &str) -> Result<()> {
    let output = execute_command("pgrep", &["-f", pattern]).await;
    
    if let Ok(pids_str) = output {
        let pids: Vec<&str> = pids_str.trim().split('\n').collect();
        
        for pid in pids {
            if !pid.is_empty() {
                let _ = execute_command_silent("kill", &["-9", pid]).await;
            }
        }
    }
    
    Ok(())
}

/// Check if a command exists in PATH
pub async fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Get environment variables as HashMap
pub fn get_env_vars() -> HashMap<String, String> {
    std::env::vars().collect()
}

/// Set environment variable for child processes
pub async fn execute_with_env(
    program: &str,
    args: &[&str],
    env_vars: HashMap<String, String>,
) -> Result<String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
    
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ThError::Process(format!(
            "Command '{}' failed: {}",
            program, stderr
        )).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute command in background and return child process
pub async fn execute_background(program: &str, args: &[&str]) -> Result<tokio::process::Child> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    Ok(child)
}

/// Wait for a condition to be true with timeout
pub async fn wait_for_condition<F>(
    condition: F,
    timeout_secs: u64,
    check_interval_ms: u64,
) -> Result<()>
where
    F: Fn() -> bool,
{
    let timeout_duration = Duration::from_secs(timeout_secs);
    let check_interval = Duration::from_millis(check_interval_ms);
    let start_time = tokio::time::Instant::now();
    
    while start_time.elapsed() < timeout_duration {
        if condition() {
            return Ok(());
        }
        tokio::time::sleep(check_interval).await;
    }
    
    Err(ThError::Timeout("Condition not met within timeout".to_string()).into())
}