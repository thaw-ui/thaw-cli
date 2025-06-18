mod build;
mod serve;

use crate::context::Context;
use build::BuildCommands;
use clap::Subcommand;
use serve::ServeCommands;
use tokio::runtime;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Build(BuildCommands),
    #[command(subcommand)]
    Serve(ServeCommands),
}

impl Commands {
    pub fn run(self, context: Context) -> color_eyre::Result<()> {
        let rt = runtime::Builder::new_multi_thread().enable_io().build()?;
        rt.block_on(async {
            match self {
                Self::Build(subcommmands) => {
                    subcommmands.run(&context, false).await?;
                }
                Self::Serve(subcommmands) => {
                    subcommmands.run(context).await?;
                }
            }
            color_eyre::Result::Ok(())
        })
    }
}
