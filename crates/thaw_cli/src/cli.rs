use crate::{commands::Commands, context::Context};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(self, context: Context) -> color_eyre::Result<()> {
        match self.commands {
            Commands::Build(subcommmands) => {
                subcommmands.run(&context)?;
            }
            Commands::Serve(subcommmands) => {
                subcommmands.run(context)?;
            }
        }
        color_eyre::Result::Ok(())
    }
}
