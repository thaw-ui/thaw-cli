mod build;
mod serve;

use build::BuildCommands;
use clap::Subcommand;
use serve::ServeCommands;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Build(BuildCommands),
    #[command(subcommand)]
    Serve(ServeCommands),
}
