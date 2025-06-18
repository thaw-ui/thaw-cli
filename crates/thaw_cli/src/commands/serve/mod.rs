mod csr;
mod watch;

use crate::context::Context;
use clap::Subcommand;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub async fn run(self, context: Context) -> color_eyre::Result<()> {
        let context = Arc::new(context);

        match self {
            ServeCommands::Csr => csr::run(context).await?,
            ServeCommands::Ssr => todo!(),
        }
        color_eyre::Result::Ok(())
    }
}
