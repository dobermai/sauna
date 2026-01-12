use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "klafs";

/// A saved profile containing sauna settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Operating mode: "sauna" or "sanarium"
    pub mode: String,

    /// Target temperature in °C
    pub temperature: i32,

    /// Humidity level (1-10, for sanarium mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub humidity: Option<i32>,
}

impl Profile {
    /// Create a new profile
    pub fn new(mode: &str, temperature: i32, humidity: Option<i32>) -> Result<Self> {
        // Validate mode
        let mode_lower = mode.to_lowercase();
        if !["sauna", "sanarium"].contains(&mode_lower.as_str()) {
            bail!("Invalid mode '{}'. Use: sauna or sanarium", mode);
        }

        // Validate temperature based on mode
        match mode_lower.as_str() {
            "sauna" => {
                if !(10..=100).contains(&temperature) {
                    bail!(
                        "Sauna temperature must be between 10 and 100°C, got {}",
                        temperature
                    );
                }
            }
            "sanarium" => {
                if !(40..=75).contains(&temperature) {
                    bail!(
                        "Sanarium temperature must be between 40 and 75°C, got {}",
                        temperature
                    );
                }
            }
            _ => {}
        }

        // Validate humidity (only valid for sanarium)
        if let Some(hum) = humidity {
            if mode_lower != "sanarium" {
                bail!("Humidity can only be set for sanarium mode");
            }
            if !(1..=10).contains(&hum) {
                bail!("Humidity level must be between 1 and 10, got {}", hum);
            }
        }

        Ok(Self {
            mode: mode_lower,
            temperature,
            humidity,
        })
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        let mut parts = vec![format!("{}°C", self.temperature), self.mode.clone()];

        if let Some(hum) = self.humidity {
            parts.push(format!("humidity {}", hum));
        }

        parts.join(", ")
    }
}

/// Collection of saved profiles
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profiles {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

impl Profiles {
    /// Get the profiles file path
    pub fn profiles_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "klafs", APP_NAME)
            .context("Could not determine config directory")?;

        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;

        Ok(config_dir.join("profiles.toml"))
    }

    /// Load profiles from disk
    pub fn load() -> Result<Self> {
        let path = Self::profiles_path()?;

        if !path.exists() {
            return Ok(Profiles::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read profiles file: {}", path.display()))?;

        let profiles: Profiles =
            toml::from_str(&content).context("Failed to parse profiles file")?;

        Ok(profiles)
    }

    /// Save profiles to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::profiles_path()?;

        let content = toml::to_string_pretty(self).context("Failed to serialize profiles")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write profiles file: {}", path.display()))?;

        Ok(())
    }

    /// Add or update a profile
    pub fn set(&mut self, name: &str, profile: Profile) {
        self.profiles.insert(name.to_string(), profile);
    }

    /// Get a profile by name
    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// Remove a profile
    pub fn remove(&mut self, name: &str) -> Option<Profile> {
        self.profiles.remove(name)
    }

    /// List all profile names
    pub fn list(&self) -> Vec<&String> {
        let mut names: Vec<_> = self.profiles.keys().collect();
        names.sort();
        names
    }

    /// Check if a profile exists
    pub fn exists(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }
}
