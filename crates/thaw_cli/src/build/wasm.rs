use crate::{
    context::Context,
    utils::{DotEyre, fs::copy_dir_all, wasm_opt_bin_path},
};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use wasm_bindgen_cli_support::Bindgen;

pub async fn wasm_bindgen(
    context: &Context,
    input_path: &PathBuf,
    out_dir: &Path,
) -> color_eyre::Result<()> {
    if tokio::fs::try_exists(&context.wasm_bindgen_dir).await? {
        tokio::fs::remove_dir_all(&context.wasm_bindgen_dir).await?;
    }
    let mut bindgen = Bindgen::new();
    let bindgen = bindgen.input_path(input_path).web(true).dot_eyre()?;
    bindgen.generate(&context.wasm_bindgen_dir).dot_eyre()?;

    copy_dir_all(&context.wasm_bindgen_dir, out_dir).await?;

    let package_name = context.cargo_package_name()?;
    let wasm_name = format!("{package_name}_bg.wasm");

    let wasm_path = context.wasm_bindgen_dir.join(&wasm_name);
    let out_wasm_path = out_dir.join(wasm_name);
    wasm_opt(&wasm_path, &out_wasm_path).await?;

    Ok(())
}

async fn wasm_opt(input_path: &Path, out_path: &Path) -> color_eyre::Result<()> {
    let path = wasm_opt_bin_path().await?;
    // wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively()
    let args = vec![
        input_path.to_str().unwrap(),
        "-o",
        out_path.to_str().unwrap(),
        "-Oz",
        "--enable-reference-types",
        "--enable-bulk-memory",
        "--enable-mutable-globals",
        "--enable-nontrapping-float-to-int",
        "--debuginfo",
    ];

    Command::new(path).args(args).spawn()?.wait().await?;

    Ok(())
}
