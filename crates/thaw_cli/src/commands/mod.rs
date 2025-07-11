mod build;
mod serve;

use crate::context::Context;
use build::BuildCommands;
use clap::Subcommand;
use serve::ServeCommands;
use std::sync::Arc;

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
                subcommmands.run(&context, false).await?;
            }
            Self::Serve(subcommmands) => {
                subcommmands.run(context).await?;
            }
        }
        Ok(())
    }
}
