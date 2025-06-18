mod hydrate;

use crate::{
    context::Context,
    utils::{DotEyre, copy_dir_all, wasm_opt_bin_path},
};
use clap::{Args, Subcommand};
use color_eyre::eyre::eyre;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use wasm_bindgen_cli_support::Bindgen;
use xshell::{Shell, cmd};

#[derive(Debug, Subcommand)]
pub enum BuildCommands {
    Csr,
    Ssr(BuildSsrArgs),
    Hydrate,
}

impl BuildCommands {
    pub async fn run(self, context: &Context, serve: bool) -> color_eyre::Result<()> {
        Self::clear_out_dir(context)?;
        Self::copy_public_dir(context)?;
        match self {
            Self::Csr => {
                Self::build_index_html(context, serve)?;
                let mut cargo_args = vec!["--target=wasm32-unknown-unknown"];
                if context.cargo_features_contains_key("csr")? {
                    cargo_args.push("--features=csr");
                }
                if context.config.release {
                    cargo_args.push("--release");
                }
                Self::build(cargo_args)?;
                wasm_bindgen(context, &build_wasm_path(context)?, &context.out_dir).await?;
            }
            Self::Ssr(build_ssr_args) => {
                if !build_ssr_args.no_hydrate {
                    hydrate::run(context).await?;
                }

                let mut cargo_args = vec!["--features=ssr"];
                if context.config.release {
                    cargo_args.push("--release");
                }
                Self::build(cargo_args)?;
            }
            Self::Hydrate => {
                hydrate::run(context).await?;
            }
        }
        color_eyre::Result::Ok(())
    }

    fn clear_out_dir(context: &Context) -> color_eyre::Result<()> {
        let out_dir = &context.out_dir;
        if fs::exists(out_dir.clone())? {
            fs::remove_dir_all(out_dir.clone())?;
        }
        fs::create_dir_all(out_dir)?;
        color_eyre::Result::Ok(())
    }

    fn copy_public_dir(context: &Context) -> color_eyre::Result<()> {
        if context.config.public_dir.is_empty() {
            return color_eyre::Result::Ok(());
        }
        let new_public_dir = &context.out_dir;

        let public_dir = context.current_dir.join(context.config.public_dir.clone());

        if fs::exists(public_dir.clone())? {
            copy_dir_all(public_dir, new_public_dir)?;
        }

        color_eyre::Result::Ok(())
    }

    fn build_index_html(context: &Context, serve: bool) -> color_eyre::Result<()> {
        let html_path = context.current_dir.join("index.html");
        let mut html_str = fs::read_to_string(html_path)?;
        let Some(body_end_index) = html_str.find("</body>") else {
            return color_eyre::Result::Err(eyre!("No end tag found for body"));
        };

        let package_name = context.cargo_package_name()?;
        let mut import_script = format!(
            r#"<script type="module">import init from '/{package_name}.js';await init({{ module_or_path: '/{package_name}_bg.wasm' }})</script>"#,
        );

        if serve {
            import_script.push_str(r#"<script src="/__thaw_cli__.js"></script>"#);
        }

        html_str.insert_str(body_end_index, &import_script);

        let out_dir = &context.out_dir;

        let new_html_path = out_dir.join("index.html");
        let mut file = fs::File::create_new(new_html_path)?;
        file.write_all(html_str.as_bytes())?;

        if serve {
            let path = out_dir.join("__thaw_cli__.js");
            let mut file = fs::File::create_new(path)?;
            file.write_all(include_str!("./__thaw_cli__.js").as_bytes())?;
        }

        color_eyre::Result::Ok(())
    }

    fn build(args: Vec<&'static str>) -> color_eyre::Result<()> {
        let sh = Shell::new()?;
        cmd!(sh, "cargo build {args...}").run()?;

        color_eyre::Result::Ok(())
    }
}

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
    color_eyre::Result::Ok(wasm_path)
}

async fn wasm_bindgen(
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

#[derive(Debug, Args)]
pub struct BuildSsrArgs {
    #[arg(long, action=clap::ArgAction::SetTrue, default_value_t=false, default_missing_value="true")]
    pub no_hydrate: bool,
}
