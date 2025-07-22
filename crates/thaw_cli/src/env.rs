use crate::config::EnvDir;
use std::{
    collections::{HashMap, hash_map::IntoIter},
    ops::Deref,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Clone)]
pub struct Env(HashMap<String, String>);

impl Env {
    pub fn load(
        current_dir: &Path,
        mode: &'static str,
        env_dir: &EnvDir,
    ) -> color_eyre::Result<Self> {
        let mut env = Self::default();
        let env_files = Self::get_env_files_for_mode(current_dir, mode, env_dir);
        for file_path in env_files {
            if !file_path.is_file() {
                continue;
            }

            for item in dotenvy::from_path_iter(file_path)? {
                let (key, value) = item?;
                env.0.insert(key, value);
            }
        }

        Ok(env)
    }

    fn get_env_files_for_mode(
        current_dir: &Path,
        mode: &'static str,
        env_dir: &EnvDir,
    ) -> Vec<PathBuf> {
        match env_dir {
            EnvDir::Path(path) => {
                let dir = current_dir.join(path);
                vec![
                    dir.join(".env"),
                    dir.join(".env.local"),
                    dir.join(format!(".env.{mode}")),
                    dir.join(format!(".env.{mode}.local")),
                ]
            }
            EnvDir::False => Vec::new(),
        }
    }

    pub fn set_default(&mut self, envs: Vec<(&'static str, String)>) {
        for (key, value) in envs {
            if !self.0.contains_key(key) {
                self.0.insert(key.to_string(), value);
            }
        }
    }

    pub fn cloned_into_iter(&self) -> IntoIter<String, String> {
        let env = self.clone();
        env.0.into_iter()
    }
}

impl Deref for Env {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
