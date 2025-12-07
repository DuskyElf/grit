use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::provider::ProviderKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_provider: Option<ProviderKind>,
    pub plr_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: None,
            plr_dir: PathBuf::from(".plr"),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config TOML from {:?}", path))
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content =
            toml::to_string_pretty(&self).with_context(|| "Failed to serialize config to TOML")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        fs::write(path, content).with_context(|| format!("Failed to write config to {:?}", path))
    }

    pub fn config_path(&self) -> PathBuf {
        self.plr_dir.join("config.toml")
    }

    pub fn credentials_dir(&self) -> PathBuf {
        self.plr_dir.join("credentials")
    }

    pub fn playlists_dir(&self) -> PathBuf {
        self.plr_dir.join("playlists")
    }
}
