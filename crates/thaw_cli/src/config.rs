mod default;

use default::{build, default_public_dir, serve};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub release: bool,
    #[serde(default = "default_public_dir")]
    pub public_dir: String,

    #[serde(default = "ServeConfig::default")]
    pub serve: ServeConfig,

    #[serde(default = "BuildConfig::default")]
    pub build: BuildConfig,
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
pub struct ServeConfig {
    #[serde(default = "serve::default_host")]
    pub host: String,
    #[serde(default = "serve::default_port")]
    pub port: u32,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: serve::default_host(),
            port: serve::default_port(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildConfig {
    #[serde(default = "build::default_out_dir")]
    pub out_dir: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: build::default_out_dir(),
        }
    }
}
