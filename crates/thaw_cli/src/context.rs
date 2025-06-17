use crate::config::Config;
use cargo_manifest::Manifest;
use color_eyre::eyre::eyre;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Context {
    pub config: Config,
    pub current_dir: PathBuf,
    cargo_manifest: Manifest,
}

impl Context {
    pub fn new(config: Config, current_dir: PathBuf) -> color_eyre::Result<Self> {
        let cargo_manifest = Manifest::from_path(current_dir.join("Cargo.toml"))?;

        color_eyre::Result::Ok(Self {
            config,
            current_dir,
            cargo_manifest,
        })
    }

    pub(crate) fn cargo_package_name(&self) -> color_eyre::Result<String> {
        if let Some(package) = &self.cargo_manifest.package {
            color_eyre::Result::Ok(package.name.clone())
        } else {
            color_eyre::Result::Err(eyre!("Cargo.toml file not found"))
        }
    }

    pub(crate) fn cargo_features_contains_key(&self, key: &str) -> color_eyre::Result<bool> {
        if let Some(features) = &self.cargo_manifest.features {
            color_eyre::Result::Ok(features.contains_key(key))
        } else {
            color_eyre::Result::Err(eyre!("Cargo.toml file not found"))
        }
    }

    pub fn target_dir(&self) -> color_eyre::Result<PathBuf> {
        Self::get_target_dir(&self.current_dir)
    }

    fn get_target_dir(dir: &Path) -> color_eyre::Result<PathBuf> {
        let target_dir = dir.join("target");
        if fs::exists(dir.join("Cargo.toml"))?
            && fs::exists(target_dir.clone())?
            && target_dir.is_dir()
        {
            return color_eyre::Result::Ok(target_dir);
        }

        if let Some(parent) = dir.parent() {
            Self::get_target_dir(parent)
        } else {
            color_eyre::Result::Err(eyre!("target directory not found"))
        }
    }
}
