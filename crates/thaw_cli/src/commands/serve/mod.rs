mod common;
mod csr;
mod ssr;
mod watch;

use self::common::ServeEvent;
use crate::{cli, context::Context};
use clap::Subcommand;
use color_eyre::owo_colors::OwoColorize;
use crossterm::style::Stylize;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::mpsc, task, time};

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        let context = Arc::new(context);
        let (build_tx, build_rx) = mpsc::channel::<Vec<PathBuf>>(10);
        let (serve_tx, serve_rx) = mpsc::channel::<ServeEvent>(10);

        match self {
            ServeCommands::Csr => {
                common::run_serve(
                    {
                        let context = context.clone();
                        move |tx| {
                            let context = context.clone();
                            let handle = task::spawn(async {
                                csr::run_serve(context, tx).await.unwrap();
                            })
                            .abort_handle();

                            vec![handle]
                        }
                    },
                    serve_rx,
                );
                csr::build(&context, &serve_tx).await?;
                csr::watch_build(context.clone(), build_rx, serve_tx);
            }
            ServeCommands::Ssr => {
                common::run_serve(
                    {
                        let context = context.clone();
                        move |tx| {
                            let exe_handle = task::spawn({
                                let context = context.clone();
                                async {
                                    ssr::run_ssr_exe(context).unwrap();
                                }
                            })
                            .abort_handle();

                            let context = context.clone();
                            let serve_handle = task::spawn(async {
                                ssr::run_serve(context, tx).await.unwrap();
                            })
                            .abort_handle();

                            vec![serve_handle, exe_handle]
                        }
                    },
                    serve_rx,
                );
                ssr::build(&context, &serve_tx).await?;
                ssr::watch_build(context.clone(), build_rx, serve_tx);
            }
        };

        context.cli_tx.send(cli::Message::InitBuildFinished).await?;
        let time = ((time::Instant::now().elapsed().as_secs_f32()
            - context.init_start_time.elapsed().as_secs_f32())
            * 100.0)
            .abs()
            .round()
            / 100.0;
        println!(
            "\n\n  {}  ready in {}s\n",
            format!("Thaw CLI v{}", context.create_version).green(),
            time.bold()
        );
        println!(
            "  {}  {}: {}",
            "➜".green(),
            "Local".bold(),
            format!(
                "http://{}:{}",
                context.config.server.host, context.config.server.port,
            )
            .cyan()
        );

        watch::watch(context, build_tx).await?;
        Ok(())
    }
}
