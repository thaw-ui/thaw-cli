mod build;
mod serve;

use build::BuildCommands;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Build(BuildCommands),
}
