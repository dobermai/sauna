use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "klafs";
const KEYRING_SERVICE: &str = "klafs-cli";

/// Configuration stored in the config file
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Email/username for the Klafs account
    pub username: Option<String>,

    /// Default sauna ID to use
    pub sauna_id: Option<String>,
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "klafs", APP_NAME)
            .context("Could not determine config directory")?;

        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;

        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config =
            toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Store the password securely in the system keyring
    pub fn store_password(username: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, username)
            .context("Failed to create keyring entry")?;

        entry
            .set_password(password)
            .context("Failed to store password in keyring")?;

        Ok(())
    }

    /// Retrieve the password from the system keyring
    pub fn get_password(username: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, username)
            .context("Failed to create keyring entry")?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("Failed to retrieve password from keyring"),
        }
    }

    /// Delete the password from the system keyring
    pub fn delete_password(username: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, username)
            .context("Failed to create keyring entry")?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(e).context("Failed to delete password from keyring"),
        }
    }

    /// Store the PIN securely in the system keyring
    pub fn store_pin(sauna_id: &str, pin: &str) -> Result<()> {
        let key = format!("pin:{}", sauna_id);
        let entry =
            keyring::Entry::new(KEYRING_SERVICE, &key).context("Failed to create keyring entry")?;

        entry
            .set_password(pin)
            .context("Failed to store PIN in keyring")?;

        Ok(())
    }

    /// Retrieve the PIN from the system keyring
    pub fn get_pin(sauna_id: &str) -> Result<Option<String>> {
        let key = format!("pin:{}", sauna_id);
        let entry =
            keyring::Entry::new(KEYRING_SERVICE, &key).context("Failed to create keyring entry")?;

        match entry.get_password() {
            Ok(pin) => Ok(Some(pin)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("Failed to retrieve PIN from keyring"),
        }
    }
}
