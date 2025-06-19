mod common;
mod csr;
mod ssr;
mod watch;

use self::common::ServeEvent;
use crate::context::Context;
use clap::Subcommand;
use std::sync::Arc;
use tokio::{sync::mpsc, task};

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        let context = Arc::new(context);
        let (build_tx, build_rx) = mpsc::channel::<()>(1);
        let (serve_tx, serve_rx) = mpsc::channel::<ServeEvent>(1);

        match self {
            ServeCommands::Csr => {
                csr::build(context.clone(), build_rx, serve_tx);
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
            }
            ServeCommands::Ssr => {
                ssr::build(context.clone(), build_rx, serve_tx);
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
            }
        }
        build_tx.send(()).await.unwrap();
        watch::watch(context, build_tx).await.unwrap();

        color_eyre::Result::Ok(())
    }
}
