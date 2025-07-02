mod build;
mod serve;

use crate::{cli, context::Context};
use build::BuildCommands;
use clap::Subcommand;
use serve::ServeCommands;
use tokio::{runtime, sync::mpsc, task};

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Build(BuildCommands),
    #[command(subcommand)]
    Serve(ServeCommands),
}

impl Commands {
    pub fn run(self, mut context: Context) -> color_eyre::Result<()> {
        let rt = runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()?;
        rt.block_on(async {
            let (message_tx, mut message_rx) = mpsc::channel(1);
            context.cli_tx = Some(message_tx);

            task::spawn(async move {
                let mut print_message = cli::PrintMessage::new();
                while let Some(message) = message_rx.recv().await {
                    print_message.print(message).unwrap();
                }
            });

            match self {
                Self::Build(subcommmands) => {
                    subcommmands.run(&context, false).await?;
                }
                Self::Serve(subcommmands) => {
                    subcommmands.run(context).await?;
                }
            }
            Ok(())
        })
    }
}
