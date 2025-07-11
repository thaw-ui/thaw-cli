mod common;
mod csr;
mod hydrate;

use self::common::wasm_bindgen;
use crate::{
    build::{clear_out_dir, copy_public_dir},
    cli,
    context::Context,
};
use clap::Subcommand;
use crossterm::style::Stylize;
use std::{path::PathBuf, sync::Arc};
use tokio::{fs, task, time};

use xshell::{Shell, cmd};

#[derive(Debug, Subcommand)]
pub enum BuildCommands {
    Csr,
    Ssr,
}

impl BuildCommands {
    pub async fn run(self, context: &Arc<Context>, serve: bool) -> color_eyre::Result<()> {
        match self {
            Self::Csr => {
                init_build(context);
                let start = time::Instant::now();

                let wasm_path = csr::build_wasm(context, serve).await?;

                clear_out_dir(context).await?;
                copy_public_dir(context).await?;
                csr::build_index_html(context, serve).await?;
                fs::create_dir_all(&context.assets_dir).await?;
                common::build_assets(context, wasm_path, &context.assets_dir).await?;
                wasm_bindgen(context, &build_wasm_path(context)?, &context.assets_dir).await?;

                let time = start.elapsed().as_secs_f32();
                if context.serve {
                    context
                        .cli_tx
                        .send(cli::Message::Build(
                            format!("✓ built in {time:.2}s").green().to_string(),
                        ))
                        .await?;
                } else {
                    let context = context.clone();
                    task::spawn_blocking(move || {
                        context.cli_tx.blocking_send(cli::Message::Build(
                            format!("✓ built in {time:.2}s").green().to_string(),
                        ))
                    })
                    .await??;
                }
            }
            Self::Ssr => {
                clear_out_dir(context).await?;

                let client_out_dir = context.out_dir.join("client");
                let server_out_dir = context.out_dir.join("server");
                let assets_dir = client_out_dir.join(&context.config.build.assets_dir);

                fs::create_dir_all(&assets_dir).await?;
                copy_public_dir(context).await?;

                hydrate::run(context, &assets_dir).await?;

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

fn init_build(context: &Context) {
    if context.serve {
        return;
    }
    println!(
        "{} {}",
        format!("Thaw CLI v{}", context.create_version).cyan(),
        "building".green()
    );
}
