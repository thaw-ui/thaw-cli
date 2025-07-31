use super::{
    middlewares,
    open_browser::open_browser,
    ws::{THAW_CLI_WS_PATH, ThawCliWs, thaw_cli_ws},
};
use crate::context::Context;
use axum::{
    Router,
    routing::{get, get_service},
};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};

pub async fn run(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let state = ThawCliWs::new(tx);
    let out_dir = &context.out_dir;

    let public_dir = context.current_dir.join(context.config.public_dir.clone());
    let public_file_service = ServeDir::new(&public_dir)
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate()
        .fallback(ServeFile::new(out_dir.join("index.html")));

    let serve_dir = ServeDir::new(out_dir.clone())
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate()
        .fallback(public_file_service);

    let app = Router::new()
        .route(THAW_CLI_WS_PATH, get(thaw_cli_ws))
        .fallback_service(get_service(serve_dir))
        .with_state(state)
        .layer(middlewares::ProxyLayer::new(&context))
        .layer(CompressionLayer::new());

    let addr = format!(
        "{}:{}",
        context.config.server.host, context.config.server.port
    );

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => return color_eyre::Result::Err(err.into()),
    };

    if context.open {
        let url = format!(
            "http://{}:{}",
            context.config.server.host, context.config.server.port,
        );
        open_browser(&context, url)?;
    }

    axum::serve(listener, app).await?;

    Ok(())
}
