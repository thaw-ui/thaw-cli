mod default;

use default::{build, default_public_dir, server};
use serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected},
};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Build artifacts in release mode, with optimizations.
    ///
    /// Default: false
    #[serde(default)]
    pub release: bool,

    /// Directory to serve as plain static assets. Files in this directory are
    /// served at / during dev and copied to the root of outDir during build,
    /// and are always served or copied as-is without transform.
    ///
    /// Default: "public"
    #[serde(default = "default_public_dir")]
    pub public_dir: String,

    /// Server configuration.
    #[serde(default = "ServerConfig::default")]
    pub server: ServerConfig,

    /// Build configuration.
    #[serde(default = "BuildConfig::default")]
    pub build: BuildConfig,

    /// The directory from which .env files are loaded.
    /// Can be an absolute path, or a path relative to the project root.
    /// false will disable the .env file loading.
    ///
    /// Default: Root directory
    #[serde(default = "EnvDir::default")]
    pub env_dir: EnvDir,
}

impl Config {
    pub fn parse(path: PathBuf, user_input: bool) -> color_eyre::Result<Self> {
        let config = if user_input {
            std::fs::read_to_string(path)?
        } else {
            std::fs::read_to_string(path).unwrap_or_default()
        };
        let config: Self = toml::from_str(&config)?;
        Ok(config)
    }
}

#[derive(Debug)]
pub enum EnvDir {
    Path(String),
    False,
}

impl Default for EnvDir {
    fn default() -> Self {
        Self::Path(String::new())
    }
}

impl<'de> Deserialize<'de> for EnvDir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawInput {
            String(String),
            Bool(bool),
        }

        match RawInput::deserialize(deserializer)? {
            RawInput::String(s) => Ok(Self::Path(s)),
            RawInput::Bool(false) => Ok(Self::False),
            RawInput::Bool(true) => Err(de::Error::invalid_value(
                Unexpected::Bool(true),
                &"String or false",
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// Specify which IP addresses the server should listen on.
    ///
    /// Default: "localhost"
    #[serde(default = "server::default_host")]
    pub host: String,

    /// Specify server port.
    ///
    /// Default: 6321
    #[serde(default = "server::default_port")]
    pub port: u32,

    /// Automatically open the app in the browser on server start.
    ///
    /// Default: false
    #[serde(default = "server::default_open")]
    pub open: bool,

    /// Configure custom proxy rules for the dev server.
    #[serde(default = "Default::default")]
    pub proxy: Vec<Proxy>,

    /// File system watcher options to pass on to
    /// [notify-debouncer-full](https://github.com/notify-rs/notify/tree/main/notify-debouncer-full).
    #[serde(default = "Watch::default")]
    pub watch: Watch,

    /// Whether to enable erased components mode.
    ///
    /// erase_components mode offers a signifigant compile time speedup by type
    /// erasing the types in your app. This is similar to adding `.into_any()`
    /// to your entire app. It can also solve some issues with compilation in debug mode.
    ///
    /// Default: false
    #[serde(default = "server::default_erase_components")]
    pub erase_components: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: server::default_host(),
            port: server::default_port(),
            open: server::default_open(),
            proxy: Default::default(),
            watch: Watch::default(),
            erase_components: server::default_erase_components(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Proxy {
    pub proxy: String,
    pub target: String,
    #[serde(default = "Default::default")]
    pub change_origin: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct Watch {
    /// Watch folder or file changes.
    #[serde(default = "Default::default")]
    pub paths: Vec<WatchPath>,
    /// Defines files/paths to be ignored.
    /// Use [globset](https://github.com/BurntSushi/ripgrep/tree/master/crates/globset) matching.
    #[serde(default = "Default::default")]
    pub ignored: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WatchPath {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    /// Specify the output directory (relative to project root).
    ///
    /// Default: "dist"
    #[serde(default = "build::default_out_dir")]
    pub out_dir: String,

    /// Specify the directory to nest generated assets under (relative to build.outDir).
    ///
    /// Default: "assets"
    #[serde(default = "build::default_assets_dir")]
    pub assets_dir: String,

    /// Whether to enable manganis to collect assets from dependencies.
    ///
    /// Default: false
    #[serde(default = "build::default_assets_manganis")]
    pub assets_manganis: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: build::default_out_dir(),
            assets_dir: build::default_assets_dir(),
            assets_manganis: build::default_assets_manganis(),
        }
    }
}
