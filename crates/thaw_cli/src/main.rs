mod build;
mod cli;
mod commands;
mod config;
mod context;
mod dx;
mod env;
mod server;
mod utils;

use self::{cli::Cli, config::Config, context::Context, env::Env};
use clap::Parser;
use tokio::time;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let init_start_time = time::Instant::now();

    let cli = Cli::parse();

    let current_dir = std::env::current_dir()?;

    let config_path = current_dir.join("Thaw.toml");
    let config = Config::parse(config_path, false)?;
    let env = Env::load(&current_dir, cli.mode(), &config.env_dir)?;

    let message_tx = Cli::watch_message(current_dir.clone());
    let context = Context::new(
        config,
        env,
        current_dir,
        message_tx,
        init_start_time,
        cli.is_serve(),
    )?;

    cli.run(context).await
}
