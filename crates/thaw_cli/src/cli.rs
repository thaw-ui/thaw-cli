use crate::{
    commands::{BuildCommands, Commands, ServeCommands},
    context::Context,
};
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version)]
pub struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

impl Cli {
    pub async fn run(self, mut context: Context) -> color_eyre::Result<()> {
        context.ssr = self.is_ssr();

        self.commands.run(context).await
    }

    #[inline]
    pub fn is_serve(&self) -> bool {
        matches!(self.commands, Commands::Serve(_))
    }

    #[inline]
    pub fn is_ssr(&self) -> bool {
        match &self.commands {
            Commands::Build(build_commands) => matches!(build_commands, BuildCommands::Ssr),
            Commands::Serve(serve_commands) => matches!(serve_commands, ServeCommands::Ssr(_)),
        }
    }

    pub fn mode(&self) -> &'static str {
        match &self.commands {
            Commands::Build(_) => "production",
            Commands::Serve(_) => "development",
        }
    }
}
