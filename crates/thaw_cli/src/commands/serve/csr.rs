use super::common::{ServeEvent, THAW_CLI_WS_PATH, ThawCliWs, thaw_cli_ws};
use crate::{cli, commands::build::BuildCommands, context::Context};
use axum::{
    Router,
    routing::{get, get_service},
};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc},
    task,
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};

pub async fn build(
    context: &Arc<Context>,
    serve_tx: &mpsc::Sender<ServeEvent>,
) -> color_eyre::Result<()> {
    BuildCommands::Csr.run(context, true).await?;
    serve_tx.send(ServeEvent::RefreshPage).await?;
    Ok(())
}

pub fn watch_build(
    context: Arc<Context>,
    mut build_rx: mpsc::Receiver<Vec<PathBuf>>,
    serve_tx: mpsc::Sender<ServeEvent>,
) {
    task::spawn({
        let context = context.clone();
        async move {
            let mut paths_batch = vec![];
            while let Some(mut paths) = build_rx.recv().await {
                paths_batch.append(&mut paths);
                while let Ok(mut paths) = build_rx.try_recv() {
                    paths_batch.append(&mut paths);
                }

                let build_result = build(&context, &serve_tx).await;

                let paths = paths_batch
                    .drain(..)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                context
                    .cli_tx
                    .send(cli::Message::PageReload(paths, build_result))
                    .await
                    .unwrap();
            }
        }
    });
}

pub async fn run_serve(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
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
