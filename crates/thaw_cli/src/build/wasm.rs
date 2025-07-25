use crate::{
    context::Context,
    logger,
    utils::{DotEyre, fs::copy_dir_all, wasm_opt_bin_path},
};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use wasm_bindgen_cli_support::Bindgen;

fn build_wasm_path(context: &Context) -> color_eyre::Result<PathBuf> {
    let wasm_path = context.target_dir.join(format!(
        "wasm32-unknown-unknown/{}/{}.wasm",
        if context.config.release {
            "release"
        } else {
            "debug"
        },
        context.cargo_package_name()?
    ));
    Ok(wasm_path)
}

pub async fn wasm_bindgen(
    context: &Context,
    input_path: Option<PathBuf>,
    out_dir: &Path,
) -> color_eyre::Result<()> {
    context
        .logger
        .send(logger::Message::Build(
            "Generating JS/WASM with wasm-bindgen".to_string(),
        ))
        .await?;
    if tokio::fs::try_exists(&context.wasm_bindgen_dir).await? {
        tokio::fs::remove_dir_all(&context.wasm_bindgen_dir).await?;
    }
    let input_path = if let Some(input_path) = input_path {
        input_path
    } else {
        build_wasm_path(context)?
    };

    let mut bindgen = Bindgen::new();
    let bindgen = bindgen.input_path(input_path).web(true).dot_eyre()?;
    bindgen.generate(&context.wasm_bindgen_dir).dot_eyre()?;

    copy_dir_all(&context.wasm_bindgen_dir, out_dir).await?;

    let package_name = context.cargo_package_name()?;
    let wasm_name = format!("{package_name}_bg.wasm");

    let wasm_path = context.wasm_bindgen_dir.join(&wasm_name);
    let mut out_wasm_path = out_dir.join(wasm_name);
    if context.ssr {
        tokio::fs::remove_file(&out_wasm_path).await?;
        out_wasm_path = out_dir.join(format!("{package_name}.wasm"))
    }
    wasm_opt(context, &wasm_path, &out_wasm_path).await?;

    Ok(())
}

async fn wasm_opt(context: &Context, input_path: &Path, out_path: &Path) -> color_eyre::Result<()> {
    context
        .logger
        .send(logger::Message::Build("Optimize WASM".to_string()))
        .await?;
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
