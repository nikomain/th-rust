use crate::config::Config;
use crate::error::ThError;
use crate::process::{execute_command, execute_command_silent, execute_command_json, wait_for_condition};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct TeleportStatus {
    pub logged_in: bool,
    pub user: Option<String>,
    pub cluster: Option<String>,
    pub expires: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KubernetesCluster {
    pub name: String,
    pub accessible: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AwsApp {
    pub name: String,
    pub description: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    pub accessible: bool,
}

#[derive(Clone)]
pub struct TeleportClient {
    config: Config,
}

impl TeleportClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Validate that required tools are available
    pub async fn validate_tools(&self) -> Result<()> {
        // Check if tsh is available
        let tsh_check = tokio::process::Command::new(&self.config.paths.tsh)
            .arg("version")
            .output()
            .await;

        match tsh_check {
            Ok(output) if output.status.success() => {
                // tsh is available
            }
            _ => {
                return Err(crate::error::ThError::NotFound(
                    format!("tsh command not found at: {}", self.config.paths.tsh)
                ).into());
            }
        }

        Ok(())
    }

    /// Check if user is logged into Teleport
    pub async fn is_logged_in(&self) -> Result<bool> {
        let status = self.get_status().await?;
        Ok(status.logged_in)
    }

    /// Get current Teleport status
    pub async fn get_status(&self) -> Result<TeleportStatus> {
        let output = execute_command(&self.config.paths.tsh, &["status"]).await;
        
        match output {
            Ok(status_text) => {
                // Check for different login indicators that tsh status might show
                if status_text.contains("Logged in as:") || 
                   status_text.contains("logged in as") ||
                   status_text.contains("User:") {
                    
                    // Try multiple field names that tsh might use
                    let user = self.extract_field(&status_text, "Logged in as:")
                        .or_else(|| self.extract_field(&status_text, "User:"));
                    let cluster = self.extract_field(&status_text, "Cluster:")
                        .or_else(|| self.extract_field(&status_text, "Proxy:"));
                    let expires = self.extract_field(&status_text, "Valid until:")
                        .or_else(|| self.extract_field(&status_text, "Expires:"));
                    
                    Ok(TeleportStatus {
                        logged_in: true,
                        user,
                        cluster,
                        expires,
                    })
                } else if status_text.contains("Not logged in") || status_text.is_empty() {
                    Ok(TeleportStatus {
                        logged_in: false,
                        user: None,
                        cluster: None,
                        expires: None,
                    })
                } else {
                    // If we can't determine status clearly, assume not logged in
                    Ok(TeleportStatus {
                        logged_in: false,
                        user: None,
                        cluster: None,
                        expires: None,
                    })
                }
            }
            Err(e) => {
                // Log the error but don't fail - might just mean tsh not found or not logged in
                eprintln!("Warning: Could not check Teleport status: {}", e);
                Ok(TeleportStatus {
                    logged_in: false,
                    user: None,
                    cluster: None,
                    expires: None,
                })
            }
        }
    }

    /// Login to Teleport
    pub async fn login(&self) -> Result<()> {
        // Check if already logged in
        if self.is_logged_in().await? {
            return Ok(());
        }

        // Start login process (this will be interactive)
        let args = vec![
            "login",
            "--auth",
            &self.config.teleport.auth_type,
            "--proxy",
            &self.config.teleport.proxy,
        ];

        // Use background execution for non-blocking login
        let mut child = tokio::process::Command::new(&self.config.paths.tsh)
            .args(&args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        // Wait for the login process to complete
        let status = child.wait().await?;
        if !status.success() {
            return Err(ThError::AuthFailed("Teleport login failed".to_string()).into());
        }

        // Wait for login to be fully established
        wait_for_condition(
            || {
                // Use a blocking call in the closure
                tokio::task::block_in_place(|| {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        self.is_logged_in().await.unwrap_or(false)
                    })
                })
            },
            self.config.teleport.timeout_seconds,
            500,
        ).await?;

        Ok(())
    }

    /// Logout from Teleport
    pub async fn logout(&self) -> Result<()> {
        execute_command_silent(&self.config.paths.tsh, &["logout"]).await?;
        Ok(())
    }

    /// List available Kubernetes clusters - exactly like bash version
    pub async fn list_kube_clusters(&self) -> Result<Vec<KubernetesCluster>> {
        let json = execute_command_json(&self.config.paths.tsh, &["kube", "ls", "-f", "json"]).await?;
        
        let mut clusters = Vec::new();
        let mut test_cluster: Option<String> = None;
        
        // First pass: collect all cluster names and find first prod cluster
        if let Value::Array(items) = json {
            for item in items {
                if let Some(name) = item.get("kube_cluster_name").and_then(|n| n.as_str()) {
                    clusters.push(name.to_string());
                    
                    // Find first prod cluster to test with
                    if test_cluster.is_none() && name.contains("prod") {
                        test_cluster = Some(name.to_string());
                    }
                }
            }
        }
        
        // Test access with one prod cluster if we found one
        let prod_access_status = if let Some(ref test_cluster) = test_cluster {
            self.check_prod_cluster_access(test_cluster).await.unwrap_or(false)
        } else {
            false
        };
        
        // Second pass: apply status based on cluster type
        let result: Vec<KubernetesCluster> = clusters.into_iter().map(|name| {
            let accessible = if name.contains("prod") {
                prod_access_status
            } else {
                true // Non-prod clusters are always accessible (n/a case)
            };
            
            KubernetesCluster {
                name,
                accessible,
            }
        }).collect();
        
        Ok(result)
    }

    /// Check prod cluster access - exactly like bash version
    pub async fn check_prod_cluster_access(&self, cluster_name: &str) -> Result<bool> {
        // Try to login to the prod cluster
        let login_result = execute_command_silent(
            &self.config.paths.tsh,
            &["kube", "login", cluster_name],
        ).await;
        
        if login_result.is_err() {
            return Ok(false);
        }
        
        // Test if we can create pods (write access)
        let kubectl_result = execute_command_silent(
            "kubectl",
            &["auth", "can-i", "create", "pod"],
        ).await;
        
        Ok(kubectl_result.unwrap_or(false))
    }

    /// Login to a Kubernetes cluster
    pub async fn kube_login(&self, cluster_name: &str) -> Result<()> {
        execute_command(&self.config.paths.tsh, &["kube", "login", cluster_name]).await?;
        Ok(())
    }

    /// List available AWS applications
    pub async fn list_aws_apps(&self) -> Result<Vec<AwsApp>> {
        let json = execute_command_json(&self.config.paths.tsh, &["apps", "ls", "--format=json"]).await?;
        
        let mut apps = Vec::new();
        
        if let Value::Array(items) = json {
            for item in items {
                if let Some(name) = item.get("metadata").and_then(|m| m.get("name")).and_then(|n| n.as_str()) {
                    let description = item.get("metadata")
                        .and_then(|m| m.get("description"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());
                    
                    let uri = item.get("spec")
                        .and_then(|s| s.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    apps.push(AwsApp {
                        name: name.to_string(),
                        description,
                        uri,
                    });
                }
            }
        }
        
        Ok(apps)
    }

    /// Login to an AWS application
    /// Login to AWS app with specific role 
    pub async fn aws_login(&self, app_name: &str, role_name: &str) -> Result<()> {
        execute_command(&self.config.paths.tsh, &["apps", "login", app_name, "--aws-role", role_name]).await?;
        Ok(())
    }
    
    /// Login to AWS app without role (to discover available roles)
    pub async fn aws_login_discover_roles(&self, app_name: &str) -> Result<String> {
        // This will fail but return the available roles in the error output
        let result = crate::process::execute_command_with_output(&self.config.paths.tsh, &["apps", "login", app_name]).await;
        
        match result {
            Ok(output) => Ok(output.stderr), // Should not happen, but return stderr if it does
            Err(e) => {
                // Extract the error message which contains available roles
                Ok(e.to_string())
            }
        }
    }

    /// Logout from AWS applications
    pub async fn aws_logout(&self) -> Result<()> {
        execute_command_silent(&self.config.paths.tsh, &["apps", "logout"]).await?;
        Ok(())
    }

    /// List available databases
    pub async fn list_databases(&self) -> Result<Vec<Value>> {
        let json = execute_command_json(&self.config.paths.tsh, &["db", "ls", "--format=json"]).await?;
        
        if let Value::Array(items) = json {
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// Login to a database
    pub async fn db_login(&self, db_name: &str) -> Result<()> {
        execute_command(&self.config.paths.tsh, &["db", "login", db_name]).await?;
        Ok(())
    }

    /// Get proxy information for a database
    pub async fn get_db_proxy(&self, db_name: &str) -> Result<String> {
        execute_command(&self.config.paths.tsh, &["proxy", "db", db_name]).await
    }

    /// Helper to extract field from status text
    fn extract_field(&self, text: &str, field: &str) -> Option<String> {
        text.lines()
            .find(|line| line.contains(field))
            .and_then(|line| line.split(':').nth(1))
            .map(|value| value.trim().to_string())
    }

    /// List RDS databases with access checking - exactly like bash check_rds_login
    pub async fn list_rds_databases(&self) -> Result<Vec<DatabaseInfo>> {
        let json = execute_command_json(&self.config.paths.tsh, &["db", "ls", "--format=json"]).await?;
        
        let mut databases = Vec::new();
        
        if let Value::Array(items) = json {
            for item in items {
                if let Some(name) = item.get("metadata").and_then(|m| m.get("name")).and_then(|n| n.as_str()) {
                    // Check if it's an RDS database (has rds label or postgres/mysql protocol)
                    let is_rds = item.get("metadata")
                        .and_then(|m| m.get("labels"))
                        .and_then(|l| l.get("db_type"))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "rds")
                        .unwrap_or(false);
                    
                    if is_rds {
                        // For now, assume all RDS databases are accessible
                        // This would need to be implemented based on actual access checking logic
                        databases.push(DatabaseInfo {
                            name: name.to_string(),
                            accessible: true,
                        });
                    }
                }
            }
        }
        
        Ok(databases)
    }

    /// List MongoDB databases with access checking - exactly like bash check_atlas_access
    pub async fn list_mongodb_databases(&self) -> Result<(Vec<String>, bool)> {
        // Check if user has atlas access
        let status_output = execute_command_json(&self.config.paths.tsh, &["status", "--format=json"]).await?;
        
        let has_atlas_access = status_output.get("active_requests")
            .and_then(|requests| requests.as_array())
            .map(|arr| arr.iter().any(|req| 
                req.get("id").and_then(|id| id.as_str()).map(|s| s.contains("atlas-can-read")).unwrap_or(false)
            ))
            .unwrap_or(false);
        
        // Get MongoDB databases (filter out RDS)
        let json = execute_command_json(&self.config.paths.tsh, &["db", "ls", "--format=json"]).await?;
        
        let mut databases = Vec::new();
        
        if let Value::Array(items) = json {
            for item in items {
                if let Some(name) = item.get("metadata").and_then(|m| m.get("name")).and_then(|n| n.as_str()) {
                    // Check if it's NOT an RDS database
                    let is_rds = item.get("metadata")
                        .and_then(|m| m.get("labels"))
                        .and_then(|l| l.get("db_type"))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "rds")
                        .unwrap_or(false);
                    
                    if !is_rds {
                        databases.push(name.to_string());
                    }
                }
            }
        }
        
        Ok((databases, has_atlas_access))
    }
}