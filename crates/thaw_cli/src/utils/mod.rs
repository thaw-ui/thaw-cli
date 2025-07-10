mod dot_eyre;
pub mod fs;
mod wasm_opt;

pub use dot_eyre::DotEyre;
pub use wasm_opt::wasm_opt_bin_path;

use std::path::PathBuf;

pub fn thaw_cli_home_dir() -> PathBuf {
    dirs::data_local_dir()
        .map(|f| f.join("thaw-cli"))
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".thaw-cli"))
}

#[inline]
pub fn thaw_cli_cache_dir() -> PathBuf {
    thaw_cli_home_dir().join("cache")
}
