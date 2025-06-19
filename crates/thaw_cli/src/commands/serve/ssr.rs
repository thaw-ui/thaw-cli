use super::common::{ServeEvent, THAW_CLI_WS_PATH, ThawCliWs, thaw_cli_ws};
use crate::{
    commands::build::{BuildCommands, BuildSsrArgs},
    context::Context,
};
use axum::{
    Router,
    routing::{get, get_service},
};
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc},
    task,
};
use tower_http::{compression::CompressionLayer, services::ServeDir};

pub fn build(
    context: Arc<Context>,
    mut build_rx: mpsc::Receiver<()>,
    serve_tx: mpsc::Sender<ServeEvent>,
) {
    task::spawn(async move {
        while (build_rx.recv().await).is_some() {
            BuildCommands::Ssr(BuildSsrArgs { no_hydrate: false })
                .run(&context, true)
                .await
                .unwrap();
            serve_tx.send(ServeEvent::RefreshPage).await.unwrap();
        }
    });
}

pub async fn run_serve(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let state = ThawCliWs::new(tx);
    let out_dir = &context.out_dir;

    let serve_dir = ServeDir::new(out_dir.clone())
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
