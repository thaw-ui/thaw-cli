pub mod csr;
mod csr_app;
mod open_browser;
pub mod ssr;
mod ssr_app;
mod ws;

use crate::{context::Context, logger};
use color_eyre::owo_colors::OwoColorize;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
enum Event {
    Watch(Vec<PathBuf>),
}

pub async fn init_build_finished(context: &Arc<Context>) -> color_eyre::Result<()> {
    context
        .logger
        .send(logger::Message::InitBuildFinished)
        .await?;

    let time = (context.init_start_time.elapsed().as_secs_f32() * 100.0).round() / 100.0;

    println!(
        "\n\n  {}  ready in {} s\n",
        format!("Thaw CLI v{}", context.create_version).green(),
        time.bold()
    );
    println!(
        "  {}  {}: {}",
        "➜".green(),
        "Local".bold(),
        format!(
            "http://{}:{}",
            context.config.server.host, context.config.server.port,
        )
        .cyan()
    );
    Ok(())
}
