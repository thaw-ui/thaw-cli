use crate::utils::copy_dir_all;

use super::{thaw_cli_cache_dir, thaw_cli_home_dir};
use color_eyre::eyre::eyre;
use flate2::read;
use std::path::PathBuf;
use tokio::fs;

pub async fn wasm_opt_bin_path() -> color_eyre::Result<PathBuf> {
    let existing_path = which::which("wasm-opt");

    if let Ok(path) = existing_path {
        color_eyre::Result::Ok(path)
    } else {
        let install_dir = binaryen_install_dir().await?;
        let install_path = install_dir.join("bin").join(binaryen_bin_name());
        if !install_path.exists() {
            let cache_dir = thaw_cli_cache_dir();
            let (dir_name, url) = find_latest_binaryen_download_url().await?;
            let bytes = reqwest::get(url).await?.bytes().await?;
            let mut archive = tar::Archive::new(read::GzDecoder::new(bytes.as_ref()));
            archive.unpack(cache_dir.clone())?;

            let dir_path = cache_dir.join(dir_name);
            copy_dir_all(dir_path.clone(), install_dir)?;
            fs::remove_dir_all(dir_path).await?;
        }
        color_eyre::Result::Ok(install_path)
    }
}

async fn binaryen_install_dir() -> color_eyre::Result<PathBuf> {
    let bindgen_dir = thaw_cli_home_dir().join("binaryen");
    fs::create_dir_all(&bindgen_dir).await?;
    color_eyre::Result::Ok(bindgen_dir)
}

fn binaryen_bin_name() -> &'static str {
    if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    }
}

async fn find_latest_binaryen_download_url() -> color_eyre::Result<(String, String)> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/WebAssembly/binaryen/releases/latest")
        .header("User-Agent", "thaw-cli")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    if let Some(message) = response.get("message").and_then(|message| message.as_str()) {
        return color_eyre::Result::Err(eyre!("wasm-opt installation: {message}"));
    }

    let tag_name = response
        .get("tag_name")
        .and_then(|tag_name| tag_name.as_str())
        .ok_or_else(|| eyre!("Failed to parse tag_name"))?;
    let assets = response
        .get("assets")
        .and_then(|assets| assets.as_array())
        .ok_or_else(|| eyre!("Failed to parse assets"))?;

    let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-windows"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-linux"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-linux"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-macos"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "arm64-macos"
    } else {
        return color_eyre::Result::Err(eyre!(
            "Unknown platform for wasm-opt installation. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH."
        ));
    };

    let asset = assets
        .iter()
        .find(|asset| {
            asset
                .get("name")
                .and_then(|name| name.as_str())
                .is_some_and(|name| name.contains(platform))
        })
        .ok_or_else(|| {
            eyre!(
                "No suitable wasm-opt binary found for platform: {}. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH.",
                platform
            )
        })?;

    let download_url = asset
        .get("browser_download_url")
        .and_then(|url| url.as_str())
        .ok_or_else(|| eyre!("Failed to get download URL for wasm-opt"))?;

    Ok((format!("binaryen-{tag_name}"), download_url.to_string()))
}
