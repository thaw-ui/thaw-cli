mod common;
mod csr;
mod ssr;
mod watch;

use self::common::{RunServeData, ServeEvent};
use crate::{cli, context::Context};
use clap::Subcommand;
use color_eyre::owo_colors::OwoColorize;
use crossterm::style::Stylize;
use notify_debouncer_full::notify::RecursiveMode;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        let context = Arc::new(context);
        let (serve_tx, mut serve_rx) = mpsc::channel::<ServeEvent>(10);

        match &self {
            ServeCommands::Csr => csr::build(&context, &serve_tx).await?,
            ServeCommands::Ssr => ssr::build(&context, &serve_tx).await?,
        }

        let mut watcher = match &self {
            ServeCommands::Csr => watch::watch_file_and_rebuild(
                context.clone(),
                csr::WatchBuild {
                    context: context.clone(),
                    serve_tx,
                },
            ),
            ServeCommands::Ssr => watch::watch_file_and_rebuild(
                context.clone(),
                ssr::WatchBuild {
                    context: context.clone(),
                    serve_tx,
                },
            ),
        }?;
        let src_dir = context.current_dir.join("src");
        watcher.watch(&src_dir, RecursiveMode::Recursive)?;

        init_build_finished(&context).await?;

        let mut data = match &self {
            ServeCommands::Csr => RunServeData::new(csr::RunServe(context)),
            ServeCommands::Ssr => RunServeData::new(ssr::RunServe(context)),
        };
        while let Some(event) = serve_rx.recv().await {
            match event {
                // ServeEvent::Restart => {
                //     data.run_serve();
                // }
                ServeEvent::RefreshPage => {
                    if let Some(tx) = &data.page_tx {
                        let _ = tx.send(());
                    } else {
                        data.run_serve();
                    }
                }
            }
        }
        Ok(())
    }
}

async fn init_build_finished(context: &Arc<Context>) -> color_eyre::Result<()> {
    context.cli_tx.send(cli::Message::InitBuildFinished).await?;

    let time = (context.init_start_time.elapsed().as_secs_f32() * 100.0).round() / 100.0;

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
    Ok(())
}
