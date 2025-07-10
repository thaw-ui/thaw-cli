use super::{
    common::{self, ServeEvent, handle_thaw_cli_ws},
    watch,
};
use crate::{
    commands::build::{BuildCommands, BuildSsrArgs, build_exe_name},
    context::Context,
};
use axum::{
    Router,
    body::Body,
    extract::{Request, State, WebSocketUpgrade},
    http::uri::Uri,
    response::{IntoResponse, Response},
    routing::get,
};
use hyper::{Method, StatusCode};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs,
    net::TcpListener,
    sync::{broadcast, mpsc},
    task::{self, JoinHandle},
};
use tower::ServiceExt;
use tower_http::{compression::CompressionLayer, services::ServeDir};
use xshell::{Shell, cmd};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

pub async fn build(
    context: &Arc<Context>,
    serve_tx: &mpsc::Sender<ServeEvent>,
) -> color_eyre::Result<()> {
    BuildCommands::Ssr(BuildSsrArgs { no_hydrate: false })
        .run(context, true)
        .await?;
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
        vec![
            task::spawn({
                let context = self.0.clone();
                async { run_ssr_exe(context) }
            }),
            task::spawn({
                let context = self.0.clone();
                async { run_serve(context, page_tx).await }
            }),
        ]
    }
}

pub fn run_ssr_exe(context: Arc<Context>) -> color_eyre::Result<()> {
    let sh = Shell::new()?;
    let exe_path = context
        .out_dir
        .join("server")
        .join(build_exe_name(&context)?);
    sh.set_var("LEPTOS_OUTPUT_NAME", context.cargo_package_name()?);
    sh.set_var("LEPTOS_SITE_PKG_DIR", "assets");
    sh.set_var("LEPTOS_WATCH", "");
    sh.set_var(
        "LEPTOS_RELOAD_EXTERNAL_PORT",
        context.config.server.port.to_string(),
    );
    cmd!(sh, "{exe_path}").run()?;

    color_eyre::Result::Ok(())
}

#[derive(Debug, Clone)]
pub struct AppState {
    tx: broadcast::Sender<()>,
    static_file_service: ServeDir,
    backend_url: String,
    client_dir: PathBuf,
    client: Client,
}

async fn cargo_leptos_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_thaw_cli_ws(socket, state.tx.clone(), true))
}

pub async fn run_serve(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let client_dir = context.out_dir.join("client");

    let static_file_service = ServeDir::new(&client_dir)
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate();

    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let state = AppState {
        tx,
        static_file_service,
        backend_url: "http://127.0.0.1:3000".to_string(),
        client_dir,
        client,
    };

    let app = Router::new()
        .route("/live_reload", get(cargo_leptos_ws))
        .fallback(handler)
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

async fn handler(State(state): State<AppState>, request: Request) -> Response {
    if request.method() == Method::GET {
        let mut path = request.uri().path().to_string();
        if path.starts_with("/") {
            path.remove(0);
        }
        let file_path = state.client_dir.join(path);
        if fs::metadata(&file_path).await.is_ok_and(|f| f.is_file()) {
            return match state.static_file_service.clone().oneshot(request).await {
                Ok(response) => response.into_response(),
                Err(_) => (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response(),
            };
        }
    }

    proxy_to_backend(state, request).await
}

async fn proxy_to_backend(state: AppState, mut request: Request) -> Response {
    let path = request.uri().path();
    let path_query = request
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);
    let backend_uri = format!("{}{}", state.backend_url, path_query);
    println!("{backend_uri}");
    *request.uri_mut() = match Uri::try_from(backend_uri) {
        Ok(url) => url,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response();
        }
    };
    match state.client.request(request).await {
        Ok(response) => response.into_response(),
        Err(_) => (StatusCode::BAD_GATEWAY, "Backend service unavailable").into_response(),
    }
}
