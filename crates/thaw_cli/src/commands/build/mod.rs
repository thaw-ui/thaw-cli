mod common;
mod csr;
mod hydrate;

use self::common::{clear_out_dir, copy_public_dir, wasm_bindgen};
use crate::context::Context;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tokio::fs;

use xshell::{Shell, cmd};

#[derive(Debug, Subcommand)]
pub enum BuildCommands {
    Csr,
    Ssr(BuildSsrArgs),
    Hydrate,
}

impl BuildCommands {
    pub async fn run(self, context: &Context, serve: bool) -> color_eyre::Result<()> {
        match self {
            Self::Csr => {
                let wasm_path = csr::build_wasm(context, serve).await?;

                clear_out_dir(&context.out_dir)?;
                copy_public_dir(context, &context.out_dir)?;
                csr::build_index_html(context, serve)?;
                fs::create_dir_all(&context.assets_dir).await?;
                common::build_assets(context, wasm_path, &context.assets_dir).await?;
                wasm_bindgen(context, &build_wasm_path(context)?, &context.assets_dir).await?;
            }
            Self::Ssr(build_ssr_args) => {
                clear_out_dir(&context.out_dir)?;

                let client_out_dir = context.out_dir.join("client");
                let server_out_dir = context.out_dir.join("server");
                let assets_dir = client_out_dir.join(&context.config.build.assets_dir);

                fs::create_dir_all(&assets_dir).await?;
                copy_public_dir(context, &client_out_dir)?;

                if !build_ssr_args.no_hydrate {
                    hydrate::run(context, &assets_dir).await?;
                }

                let mut cargo_args = vec!["--features=ssr"];
                if context.config.release {
                    cargo_args.push("--release");
                }
                build(cargo_args)?;

                fs::create_dir_all(&server_out_dir).await?;
                fs::copy(
                    build_exe_path(context)?,
                    server_out_dir.join(build_exe_name(context)?),
                )
                .await?;
            }
            Self::Hydrate => {
                clear_out_dir(&context.out_dir)?;
                hydrate::run(context, &context.out_dir).await?;
            }
        }
        color_eyre::Result::Ok(())
    }
}

fn build(args: Vec<&'static str>) -> color_eyre::Result<()> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo build {args...}").run()?;

    color_eyre::Result::Ok(())
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

fn build_exe_path(context: &Context) -> color_eyre::Result<PathBuf> {
    let exe_name = build_exe_name(context)?;
    let exe_path = context.target_dir.join(format!(
        "{}/{}",
        if context.config.release {
            "release"
        } else {
            "debug"
        },
        exe_name
    ));
    color_eyre::Result::Ok(exe_path)
}

pub fn build_exe_name(context: &Context) -> color_eyre::Result<String> {
    let mut exe_name = context.cargo_package_name()?;
    if cfg!(windows) {
        exe_name.push_str(".exe");
    }
    color_eyre::Result::Ok(exe_name)
}

#[derive(Debug, Args)]
pub struct BuildSsrArgs {
    #[arg(long, action=clap::ArgAction::SetTrue, default_value_t=false, default_missing_value="true")]
    pub no_hydrate: bool,
}
