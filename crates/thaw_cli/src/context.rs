use crate::{cli, config::Config};
use cargo_manifest::Manifest;
use cargo_metadata::MetadataCommand;
use color_eyre::eyre::eyre;
use std::path::{Path, PathBuf};
use tokio::{sync::mpsc, time};

#[derive(Debug)]
pub struct Context {
    /// User-specified or default configuration files.
    pub(crate) config: Config,
    /// Run the current command in the directory.
    pub(crate) current_dir: PathBuf,
    /// The target directory of the package.
    pub(crate) target_dir: PathBuf,
    pub(crate) wasm_bindgen_dir: PathBuf,
    pub(crate) out_dir: PathBuf,
    pub(crate) assets_dir: PathBuf,
    cargo_manifest: Manifest,
    pub(crate) create_version: &'static str,
    pub(crate) cli_tx: mpsc::Sender<cli::Message>,
    pub(crate) init_start_time: time::Instant,
}

impl Context {
    pub fn new(
        config: Config,
        current_dir: PathBuf,
        cli_tx: mpsc::Sender<cli::Message>,
        init_start_time: time::Instant,
        serve: bool,
    ) -> color_eyre::Result<Self> {
        let cargo_manifest = Manifest::from_path(current_dir.join("Cargo.toml"))?;
        let package_name = Self::package_name(&cargo_manifest, &current_dir)?;

        let metadata = MetadataCommand::new().exec()?;
        let target_dir = metadata.target_directory.into_std_path_buf();
        let thaw_cli_dir = target_dir.join("thaw-cli");

        let wasm_bindgen_dir = thaw_cli_dir
            .join("wasm-bindgen")
            .join(if config.release { "release" } else { "debug" })
            .join(&package_name);

        let out_dir = if serve {
            thaw_cli_dir
                .join(if config.release { "release" } else { "debug" })
                .join(&package_name)
        } else {
            current_dir.join(config.build.out_dir.clone())
        };

        let assets_dir = out_dir.join(&config.build.assets_dir);

        color_eyre::Result::Ok(Self {
            config,
            current_dir,
            target_dir,
            wasm_bindgen_dir,
            out_dir,
            assets_dir,
            cargo_manifest,
            create_version: env!("CARGO_PKG_VERSION"),
            cli_tx,
            init_start_time,
        })
    }

    pub(crate) fn cargo_package_name(&self) -> color_eyre::Result<String> {
        if let Some(package) = &self.cargo_manifest.package {
            color_eyre::Result::Ok(package.name.clone())
        } else {
            color_eyre::Result::Err(eyre!("The Carog.toml file does not have a package name"))
        }
    }

    fn package_name(manifest: &Manifest, current_dir: &Path) -> color_eyre::Result<String> {
        if let Some(package) = &manifest.package {
            color_eyre::Result::Ok(package.name.clone())
        } else if let Some(dir_name) = current_dir.file_name().and_then(|name| name.to_str()) {
            color_eyre::Result::Ok(dir_name.to_string())
        } else {
            color_eyre::Result::Err(eyre!("The Carog.toml file does not have a package name"))
        }
    }

    pub(crate) fn cargo_features_contains_key(&self, key: &str) -> bool {
        if let Some(features) = &self.cargo_manifest.features {
            features.contains_key(key)
        } else {
            false
        }
    }
}
