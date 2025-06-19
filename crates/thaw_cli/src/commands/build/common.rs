use crate::{
    context::Context,
    utils::{DotEyre, copy_dir_all, wasm_opt_bin_path},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use wasm_bindgen_cli_support::Bindgen;
use xshell::{Shell, cmd};

pub fn clear_out_dir(out_dir: &Path) -> color_eyre::Result<()> {
    if fs::exists(out_dir)? {
        fs::remove_dir_all(out_dir)?;
    }
    fs::create_dir_all(out_dir)?;
    color_eyre::Result::Ok(())
}

pub fn copy_public_dir(context: &Context, out_dir: &Path) -> color_eyre::Result<()> {
    if context.config.public_dir.is_empty() {
        return color_eyre::Result::Ok(());
    }

    let public_dir = context.current_dir.join(context.config.public_dir.clone());

    if fs::exists(public_dir.clone())? {
        copy_dir_all(public_dir, out_dir)?;
    }

    color_eyre::Result::Ok(())
}

pub async fn wasm_bindgen(
    context: &Context,
    input_path: &PathBuf,
    out_path: &Path,
) -> color_eyre::Result<()> {
    let mut bindgen = Bindgen::new();
    let bindgen = bindgen.input_path(input_path).web(true).dot_eyre()?;
    bindgen
        .generate(context.wasm_bindgen_dir.clone())
        .dot_eyre()?;

    let package_name = context.cargo_package_name()?;
    let wasm_name = format!("{package_name}_bg.wasm");
    let js_name = format!("{package_name}.js");

    let wasm_path = context.wasm_bindgen_dir.join(wasm_name.clone());
    let out_wasm_path = out_path.join(wasm_name);
    wasm_opt(&wasm_path, &out_wasm_path).await?;

    let js_path = context.wasm_bindgen_dir.join(js_name.clone());
    let out_js_path = out_path.join(js_name);
    tokio::fs::copy(js_path, out_js_path).await?;

    color_eyre::Result::Ok(())
}

async fn wasm_opt(input_path: &Path, out_path: &Path) -> color_eyre::Result<()> {
    let path = wasm_opt_bin_path().await?;
    let sh = Shell::new()?;
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
    cmd!(sh, "{path} {args...}").run()?;
    color_eyre::Result::Ok(())
}
