use super::ws::handle_thaw_cli_ws;
use crate::context::Context;
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
use tokio::{fs, net::TcpListener, sync::broadcast};
use tower::ServiceExt;
use tower_http::{compression::CompressionLayer, services::ServeDir};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Debug, Clone)]
pub struct AppState {
    tx: broadcast::Sender<()>,
    public_dir: PathBuf,
    public_file_service: Option<ServeDir>,
    static_file_service: ServeDir,
    backend_url: String,
    client_dir: PathBuf,
    client: Client,
}

async fn cargo_leptos_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_thaw_cli_ws(socket, state.tx.clone(), true))
}

pub async fn run(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let client_dir = context.out_dir.join("client");

    let static_file_service = ServeDir::new(&client_dir)
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate();

    let public_dir = context.current_dir.join(context.config.public_dir.clone());
    let public_file_service = if fs::try_exists(&public_dir).await? {
        ServeDir::new(&public_dir)
            .precompressed_br()
            .precompressed_zstd()
            .precompressed_gzip()
            .precompressed_deflate()
            .into()
    } else {
        None
    };

    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let state = AppState {
        tx,
        public_dir,
        public_file_service,
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

    Ok(())
}

async fn handler(State(state): State<AppState>, request: Request) -> Response {
    if request.method() == Method::GET {
        let mut path = request.uri().path().to_string();
        if path.starts_with("/") {
            path.remove(0);
        }
        let file_path = state.client_dir.join(&path);
        if fs::metadata(&file_path).await.is_ok_and(|f| f.is_file()) {
            return match state.static_file_service.clone().oneshot(request).await {
                Ok(response) => response.into_response(),
                Err(_) => (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response(),
            };
        }
        if let Some(public_file_service) = state.public_file_service.clone() {
            let file_path = state.public_dir.join(&path);
            if fs::metadata(&file_path).await.is_ok_and(|f| f.is_file()) {
                return match public_file_service.oneshot(request).await {
                    Ok(response) => response.into_response(),
                    Err(_) => (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response(),
                };
            }
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
