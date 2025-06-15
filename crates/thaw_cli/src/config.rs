use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub release: bool,
    #[serde(default = "default_public_dir")]
    pub public_dir: String,
    #[serde(default = "BuildConfig::default")]
    pub build: BuildConfig,
}

fn default_public_dir() -> String {
    "public".to_string()
}

impl Config {
    pub fn parse(path: PathBuf) -> color_eyre::Result<Self> {
        let config = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&config)?;
        color_eyre::Result::Ok(config)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildConfig {
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: default_out_dir(),
        }
    }
}

fn default_out_dir() -> String {
    "dist".to_string()
}
