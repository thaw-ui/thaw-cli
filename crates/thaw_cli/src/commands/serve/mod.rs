mod csr;
mod watch;

use crate::context::Context;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub fn run(self, context: Context) -> color_eyre::Result<()> {
        match self {
            ServeCommands::Csr => csr::run(context)?,
            ServeCommands::Ssr => todo!(),
        }
        color_eyre::Result::Ok(())
    }
}
