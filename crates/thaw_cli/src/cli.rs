mod message;

pub use message::{Message, PrintMessage};

use crate::{
    commands::{BuildCommands, Commands, ServeCommands},
    context::Context,
};
use clap::Parser;
use std::path::PathBuf;
use tokio::{sync::mpsc, task};

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

    pub fn watch_message(current_dir: PathBuf) -> mpsc::Sender<Message> {
        let (message_tx, mut message_rx) = mpsc::channel(50);
        let mut print_message = PrintMessage::new(current_dir);

        task::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                print_message.print(message).unwrap();
            }
        });

        message_tx
    }
}
