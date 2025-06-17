use crate::config::Config;
use cargo_manifest::Manifest;
use cargo_metadata::MetadataCommand;
use color_eyre::eyre::eyre;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Context {
    pub(crate) config: Config,
    pub(crate) current_dir: PathBuf,
    pub(crate) target_dir: PathBuf,
    pub(crate) out_dir: PathBuf,
    cargo_manifest: Manifest,
}

impl Context {
    pub fn new(config: Config, current_dir: PathBuf, serve: bool) -> color_eyre::Result<Self> {
        let cargo_manifest = Manifest::from_path(current_dir.join("Cargo.toml"))?;
        let metadata = MetadataCommand::new().exec()?;
        let target_dir = metadata.target_directory.into_std_path_buf();

        let out_dir = if serve {
            let package_name = Self::package_name(&cargo_manifest, &current_dir)?;
            target_dir
                .join("thaw-cli")
                .join(if config.release { "release" } else { "debug" })
                .join(package_name)
        } else {
            current_dir.join(config.build.out_dir.clone())
        };
        color_eyre::Result::Ok(Self {
            config,
            current_dir,
            target_dir,
            out_dir,
            cargo_manifest,
        })
    }

    pub(crate) fn cargo_package_name(&self) -> color_eyre::Result<String> {
        Self::package_name(&self.cargo_manifest, &self.current_dir)
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

    pub(crate) fn cargo_features_contains_key(&self, key: &str) -> color_eyre::Result<bool> {
        if let Some(features) = &self.cargo_manifest.features {
            color_eyre::Result::Ok(features.contains_key(key))
        } else {
            color_eyre::Result::Err(eyre!("Cargo.toml file not found"))
        }
    }
}
