use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub kube: HashMap<String, String>,
    pub aws: HashMap<String, String>,
    pub teleport: TeleportConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TeleportConfig {
    pub proxy: String,
    pub auth_type: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PathsConfig {
    pub tsh: String,
    pub kubectl: String,
    pub aws_cli: String,
    pub temp_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let mut kube = HashMap::new();
        kube.insert("dev".to_string(), "aslive-dev-eks-blue".to_string());
        kube.insert("sandbox".to_string(), "aslive-sandbox-eks-blue".to_string());
        kube.insert("staging".to_string(), "aslive-staging-eks-blue".to_string());
        kube.insert("usstaging".to_string(), "aslive-usstaging-eks-blue".to_string());
        kube.insert("admin".to_string(), "headquarter-admin-eks-green".to_string());
        kube.insert("prod".to_string(), "live-prod-eks-blue".to_string());
        kube.insert("usprod".to_string(), "live-usprod-eks-blue".to_string());
        kube.insert("corepgblue".to_string(), "platform-corepgblue-eks-blue".to_string());
        kube.insert("corepggreen".to_string(), "platform-corepggreen-eks-green".to_string());

        let mut aws = HashMap::new();
        aws.insert("dev".to_string(), "yl-development".to_string());
        aws.insert("sandbox".to_string(), "yl-sandbox".to_string());
        aws.insert("staging".to_string(), "yl-staging".to_string());
        aws.insert("usstaging".to_string(), "yl-usstaging".to_string());
        aws.insert("admin".to_string(), "yl-admin".to_string());
        aws.insert("prod".to_string(), "yl-production".to_string());
        aws.insert("usprod".to_string(), "yl-usproduction".to_string());
        aws.insert("corepgblue".to_string(), "yl-corepgblue".to_string());
        aws.insert("corepggreen".to_string(), "yl-corepggreen".to_string());
        aws.insert("corepg".to_string(), "yl-coreplayground".to_string());

        Self {
            kube,
            aws,
            teleport: TeleportConfig {
                proxy: "youlend.teleport.sh:443".to_string(),
                auth_type: "ad".to_string(),
                timeout_seconds: 15,
            },
            paths: PathsConfig {
                tsh: "tsh".to_string(),
                kubectl: "kubectl".to_string(),
                aws_cli: "aws".to_string(),
                temp_dir: std::env::temp_dir(),
            },
        }
    }
}

impl Config {
    /// Load configuration from file, creating default if not exists
    pub async fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save().await?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content).await?;
        
        Ok(())
    }

    /// Get the path to the configuration file
    pub fn get_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        
        Ok(home.join(".config").join("th").join("config.toml"))
    }

    /// Get Kubernetes cluster name for environment
    pub fn get_kube_cluster(&self, env: &str) -> Option<&String> {
        self.kube.get(env)
    }

    /// Get AWS account name for environment
    pub fn get_aws_account(&self, env: &str) -> Option<&String> {
        self.aws.get(env)
    }

    /// List all available Kubernetes environments
    pub fn list_kube_envs(&self) -> Vec<&String> {
        self.kube.keys().collect()
    }

    /// List all available AWS environments
    pub fn list_aws_envs(&self) -> Vec<&String> {
        self.aws.keys().collect()
    }
}