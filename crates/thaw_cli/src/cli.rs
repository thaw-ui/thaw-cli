mod message;

pub use message::{Message, PrintMessage};

use crate::{commands::Commands, context::Context};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(self, context: Context) -> color_eyre::Result<()> {
        self.commands.run(context)
    }

    #[inline]
    pub fn is_serve(&self) -> bool {
        matches!(self.commands, Commands::Serve(_))
    }
}
