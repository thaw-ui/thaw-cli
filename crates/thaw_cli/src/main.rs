use std::env;

use clap::Parser;
use thaw_cli::{cli::Cli, config::Config, context::Context};

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();
    println!("{:#?}", cli);

    let current_dir = env::current_dir()?;

    let config_path = current_dir.join("Thaw.toml");
    let config = Config::parse(config_path)?;
    println!("{:#?}", config);

    let context = Context::new(config, current_dir)?;
    println!("{:#?}", context);

    cli.run(context)
}
