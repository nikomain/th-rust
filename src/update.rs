use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const GITHUB_API_URL: &str = "https://api.github.com/repos/YouLend/th-rust/releases/latest";
const UPDATE_CHECK_FILE: &str = ".th_update_check";
const CHECK_INTERVAL_SECONDS: u64 = 24 * 60 * 60; // 24 hours

// Test mode for simulating updates
const TEST_MODE: bool = true; // Set to false for production
const TEST_CHECK_INTERVAL_SECONDS: u64 = 5; // 5 seconds for testing

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    body: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCache {
    pub last_check: u64,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

pub struct UpdateChecker {
    cache_path: std::path::PathBuf,
    current_version: String,
}

impl UpdateChecker {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let cache_path = home.join(UPDATE_CHECK_FILE);
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        
        Ok(Self {
            cache_path,
            current_version,
        })
    }

    /// Check for updates in background (non-blocking)
    pub async fn check_for_updates_background(&self) -> Result<()> {
        let checker = self.clone_for_background()?;
        
        tokio::spawn(async move {
            let _ = checker.check_for_updates_internal().await;
        });
        
        Ok(())
    }

    /// Check if update notification should be shown
    pub async fn should_show_update_notification(&self) -> Result<Option<String>> {
        let cache = self.load_cache().await.unwrap_or_default();
        
        if cache.update_available {
            if let Some(latest_version) = cache.latest_version {
                return Ok(Some(format!(
                    "📦 Update available: {} → {} (run `th update` or `th changelog` for details)",
                    self.current_version,
                    latest_version
                )));
            }
        }
        
        Ok(None)
    }

    /// Force check for updates (for `th update` command)
    pub async fn check_for_updates_now(&self) -> Result<bool> {
        self.check_for_updates_internal().await
    }

    /// Get current version
    pub fn get_current_version(&self) -> String {
        self.current_version.clone()
    }

    /// Get update cache for external access
    pub async fn get_update_cache(&self) -> Result<UpdateCache> {
        self.load_cache().await
    }

    /// Fetch changelog from GitHub releases
    pub async fn fetch_changelog(&self) -> Result<String> {
        let release = self.fetch_latest_release().await?;
        let current_version = semver::Version::parse(&self.current_version)?;
        let latest_version = semver::Version::parse(&release.tag_name.trim_start_matches('v'))?;
        
        if latest_version > current_version {
            Ok(format!("## {} → {}\n\n{}", 
                self.current_version, 
                release.tag_name, 
                release.body))
        } else {
            Ok(format!("## Current Version: {}\n\nYou're running the latest version!\n\n{}", 
                self.current_version,
                release.body))
        }
    }

    /// Download and install update
    pub async fn install_update(&self) -> Result<()> {
        println!("🔄 Checking for updates...");
        
        let release = self.fetch_latest_release().await?;
        let current_version = semver::Version::parse(&self.current_version)?;
        let latest_version = semver::Version::parse(&release.tag_name.trim_start_matches('v'))?;
        
        if latest_version <= current_version {
            println!("✅ You're already running the latest version ({})", self.current_version);
            return Ok(());
        }
        
        println!("📦 Found new version: {} → {}", current_version, latest_version);
        println!("📝 Release notes:\n{}\n", release.body);
        
        if TEST_MODE {
            println!("🧪 TEST MODE: Simulating update process...");
            
            // Find the appropriate binary asset
            let asset = self.find_binary_asset(&release.assets)?;
            
            println!("⬇️  [SIMULATED] Downloading {}...", asset.name);
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
            
            println!("🔧 [SIMULATED] Installing update...");
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            
            println!("✅ [SIMULATED] Successfully updated to version {}", latest_version);
            println!("🔄 [SIMULATED] Restart your terminal or run `source th.sh` to use the new version");
            println!("\n⚠️  NOTE: This was a test simulation. No actual update was performed.");
            return Ok(());
        }
        
        // Real update process for production
        let asset = self.find_binary_asset(&release.assets)?;
        
        println!("⬇️  Downloading {}...", asset.name);
        let binary_data = self.download_asset(&asset.browser_download_url).await?;
        
        println!("🔧 Installing update...");
        self.install_binary(binary_data).await?;
        
        println!("✅ Successfully updated to version {}", latest_version);
        println!("🔄 Restart your terminal or run `source th.sh` to use the new version");
        
        Ok(())
    }

    // Internal implementation methods
    fn clone_for_background(&self) -> Result<Self> {
        Ok(Self {
            cache_path: self.cache_path.clone(),
            current_version: self.current_version.clone(),
        })
    }

    async fn check_for_updates_internal(&self) -> Result<bool> {
        let cache = self.load_cache().await.unwrap_or_default();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // Use shorter interval for testing
        let interval = if TEST_MODE { TEST_CHECK_INTERVAL_SECONDS } else { CHECK_INTERVAL_SECONDS };
        
        // Check if we need to update (cache expired)
        if now - cache.last_check < interval && cache.last_check > 0 {
            return Ok(cache.update_available);
        }
        
        // Fetch latest release info
        let release = match self.fetch_latest_release().await {
            Ok(release) => release,
            Err(_) => {
                // Network error, use cached info if available
                return Ok(cache.update_available);
            }
        };
        
        // Compare versions
        let current_version = semver::Version::parse(&self.current_version)?;
        let latest_version = semver::Version::parse(&release.tag_name.trim_start_matches('v'))?;
        let update_available = latest_version > current_version;
        
        // Update cache
        let new_cache = UpdateCache {
            last_check: now,
            latest_version: Some(release.tag_name.clone()),
            update_available,
        };
        
        let _ = self.save_cache(&new_cache).await;
        
        Ok(update_available)
    }

    async fn fetch_latest_release(&self) -> Result<GitHubRelease> {
        // In test mode, return a mock response with a newer version
        if TEST_MODE {
            return Ok(GitHubRelease {
                tag_name: "v1.6.0".to_string(),
                name: "Version 1.6.0 - Enhanced Features".to_string(),
                body: "🚀 New Features:\n• Autoupdate functionality\n• Improved error handling\n• Better performance\n\n🐛 Bug Fixes:\n• Fixed credential sourcing issues\n• Improved connection stability".to_string(),
                html_url: "https://github.com/YouLend/th-rust/releases/tag/v1.6.0".to_string(),
                assets: vec![
                    GitHubAsset {
                        name: "th-aarch64-apple-darwin".to_string(),
                        browser_download_url: "https://github.com/YouLend/th-rust/releases/download/v1.6.0/th-aarch64-apple-darwin".to_string(),
                    },
                    GitHubAsset {
                        name: "th-x86_64-apple-darwin".to_string(),
                        browser_download_url: "https://github.com/YouLend/th-rust/releases/download/v1.6.0/th-x86_64-apple-darwin".to_string(),
                    },
                    GitHubAsset {
                        name: "th-x86_64-unknown-linux-gnu".to_string(),
                        browser_download_url: "https://github.com/YouLend/th-rust/releases/download/v1.6.0/th-x86_64-unknown-linux-gnu".to_string(),
                    },
                ],
            });
        }
        
        let client = reqwest::Client::builder()
            .user_agent("th-cli")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        let response = client
            .get(GITHUB_API_URL)
            .send()
            .await?
            .json::<GitHubRelease>()
            .await?;
        
        Ok(response)
    }

    fn find_binary_asset<'a>(&self, assets: &'a [GitHubAsset]) -> Result<&'a GitHubAsset> {
        // Look for platform-specific binary
        let target_os = std::env::consts::OS;
        let target_arch = std::env::consts::ARCH;
        
        let pattern = match target_os {
            "macos" => format!("th-{}-apple-darwin", if target_arch == "aarch64" { "aarch64" } else { "x86_64" }),
            "linux" => format!("th-{}-unknown-linux-gnu", target_arch),
            "windows" => format!("th-{}.exe", target_arch),
            _ => return Err(anyhow::anyhow!("Unsupported platform: {}", target_os)),
        };
        
        assets.iter()
            .find(|asset| asset.name.contains(&pattern))
            .or_else(|| assets.iter().find(|asset| asset.name == "th" || asset.name == "th.exe"))
            .ok_or_else(|| anyhow::anyhow!("No suitable binary found for platform"))
    }

    async fn download_asset(&self, url: &str) -> Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let binary_data = response.bytes().await?;
        Ok(binary_data.to_vec())
    }

    async fn install_binary(&self, binary_data: Vec<u8>) -> Result<()> {
        // Get current binary path
        let current_exe = std::env::current_exe()?;
        let backup_path = current_exe.with_extension("backup");
        
        // Create backup of current binary
        fs::copy(&current_exe, &backup_path).await?;
        
        // Write new binary to temporary file first
        let temp_path = current_exe.with_extension("tmp");
        fs::write(&temp_path, binary_data).await?;
        
        // Make it executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_path, perms).await?;
        }
        
        // Atomic replace (rename is atomic on most filesystems)
        fs::rename(&temp_path, &current_exe).await?;
        
        // Remove backup on success
        let _ = fs::remove_file(&backup_path).await;
        
        Ok(())
    }

    async fn load_cache(&self) -> Result<UpdateCache> {
        let content = fs::read_to_string(&self.cache_path).await?;
        let cache: UpdateCache = serde_json::from_str(&content)?;
        Ok(cache)
    }

    async fn save_cache(&self, cache: &UpdateCache) -> Result<()> {
        let content = serde_json::to_string_pretty(cache)?;
        fs::write(&self.cache_path, content).await?;
        Ok(())
    }
}

impl Default for UpdateCache {
    fn default() -> Self {
        Self {
            last_check: 0,
            latest_version: None,
            update_available: false,
        }
    }
}