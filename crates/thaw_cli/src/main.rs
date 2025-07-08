use clap::Parser;
use std::env;
use thaw_cli::{Cli, config::Config, context::Context};
use tokio::time;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let init_start_time = time::Instant::now();

    let cli = Cli::parse();

    let current_dir = env::current_dir()?;

    let config_path = current_dir.join("Thaw.toml");
    let config = Config::parse(config_path, false)?;

    let message_tx = Cli::watch_message(current_dir.clone());
    let context = Context::new(
        config,
        current_dir,
        message_tx,
        init_start_time,
        cli.is_serve(),
    )?;

    cli.run(context).await
}
