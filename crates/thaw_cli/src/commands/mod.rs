mod build;
mod serve;

use crate::{cli, context::Context};
use build::BuildCommands;
use clap::Subcommand;
use crossterm::style::Stylize;
use serve::ServeCommands;
use std::sync::Arc;
use tokio::{task, time};

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Build(BuildCommands),
    #[command(subcommand)]
    Serve(ServeCommands),
}

impl Commands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        match self {
            Self::Build(subcommmands) => {
                let context = Arc::new(context);
                build(context.clone(), async { subcommmands.run(&context).await }).await
            }
            Self::Serve(subcommmands) => subcommmands.run(context).await,
        }
    }
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
