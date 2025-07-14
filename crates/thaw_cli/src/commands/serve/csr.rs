use super::{
    common::{self, ServeEvent, THAW_CLI_WS_PATH, ThawCliWs, thaw_cli_ws},
    watch,
};
use crate::{commands::build::BuildCommands, context::Context};
use axum::{
    Router,
    routing::{get, get_service},
};
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc},
    task::{self, JoinHandle},
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};

pub async fn build(
    context: &Arc<Context>,
    serve_tx: &mpsc::Sender<ServeEvent>,
) -> color_eyre::Result<()> {
    BuildCommands::Csr.run(context).await?;
    serve_tx.send(ServeEvent::RefreshPage).await?;
    Ok(())
}

pub struct WatchBuild {
    pub context: Arc<Context>,
    pub serve_tx: mpsc::Sender<ServeEvent>,
}

impl watch::WatchBuild for WatchBuild {
    async fn build(&self) -> color_eyre::Result<()> {
        build(&self.context, &self.serve_tx).await
    }
}

pub struct RunServe(pub Arc<Context>);

impl common::RunServe for RunServe {
    fn run(&self, page_tx: broadcast::Sender<()>) -> Vec<JoinHandle<color_eyre::Result<()>>> {
        vec![task::spawn({
            let context = self.0.clone();
            async { run_serve(context, page_tx).await }
        })]
    }
}

async fn run_serve(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let state = ThawCliWs::new(tx);
    let out_dir = &context.out_dir;

    let serve_dir = ServeDir::new(out_dir.clone())
        .fallback(ServeFile::new(out_dir.join("index.html")))
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate();

    let app = Router::new()
        .route(THAW_CLI_WS_PATH, get(thaw_cli_ws))
        .fallback_service(get_service(serve_dir))
        .with_state(state)
        .layer(CompressionLayer::new());

    let addr = format!(
        "{}:{}",
        context.config.server.host, context.config.server.port
    );

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => return color_eyre::Result::Err(err.into()),
    };

    axum::serve(listener, app).await?;

    color_eyre::Result::Ok(())
}
