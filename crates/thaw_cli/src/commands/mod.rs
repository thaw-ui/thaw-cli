mod build;
// pub mod serve;

use crate::{
    cli,
    context::Context,
    server::{csr, init_build_finished, ssr},
};
use build::BuildCommands;
use clap::Subcommand;
use crossterm::style::Stylize;
use std::sync::Arc;
use tokio::{task, time};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Build the Leptos project
    #[command(subcommand)]
    Build(BuildCommands),
    /// Start Thaw CLI dev server in the current directory
    #[command(subcommand)]
    Serve(ServeCommands),
}

impl Commands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        let context = Arc::new(context);

        match self {
            Self::Build(subcommmands) => {
                build(context.clone(), async { subcommmands.run(&context).await }).await
            }
            Self::Serve(subcommmands) => match subcommmands {
                ServeCommands::Csr => {
                    BuildCommands::Csr.run(&context).await?;
                    init_build_finished(&context).await?;
                    csr::DevServer::new(context)?
                        .run()
                        .await?
                        .wait_event()
                        .await?;
                    Ok(())
                }
                ServeCommands::Ssr => {
                    BuildCommands::Ssr.run(&context).await?;
                    init_build_finished(&context).await?;
                    ssr::DevServer::new(context)?
                        .run()
                        .await?
                        .wait_event()
                        .await?;
                    Ok(())
                }
            },
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    /// Client-side rendering
    Csr,
    /// Server-side Rendering
    Ssr,
}

async fn build(
    context: Arc<Context>,
    run: impl Future<Output = color_eyre::Result<()>>,
) -> color_eyre::Result<()> {
    println!(
        "{} {}",
        format!("Thaw CLI v{}", context.create_version).cyan(),
        "building".green()
    );
    let start = time::Instant::now();

    run.await?;

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

    Ok(())
}
