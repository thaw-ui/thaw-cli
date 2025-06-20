use super::common::{ServeEvent, THAW_CLI_WS_PATH, handle_thaw_cli_ws};
use crate::{
    commands::build::{BuildCommands, BuildSsrArgs, build_exe_name},
    context::Context,
};
use axum::{
    Router, body,
    extract::{Request, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::get,
};
use http_body_util::BodyExt;
use hyper::Method;
use reqwest::StatusCode;
use std::{path::PathBuf, sync::Arc, usize};
use tokio::{
    fs,
    net::TcpListener,
    sync::{broadcast, mpsc},
    task,
};
use tower::ServiceExt;
use tower_http::{compression::CompressionLayer, services::ServeDir};
use xshell::{Shell, cmd};

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

pub fn run_ssr_exe(context: Arc<Context>) -> color_eyre::Result<()> {
    let sh = Shell::new()?;
    let exe_path = context
        .out_dir
        .join("server")
        .join(build_exe_name(&context)?);
    cmd!(sh, "{exe_path}").run()?;

    color_eyre::Result::Ok(())
}

#[derive(Debug, Clone)]
pub struct AppState {
    tx: broadcast::Sender<()>,
    static_file_service: ServeDir,
    backend_url: String,
    client_dir: PathBuf,
    client: reqwest::Client,
}

async fn thaw_cli_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_thaw_cli_ws(socket, state.tx.clone()))
}

pub async fn run_serve(context: Arc<Context>, tx: broadcast::Sender<()>) -> color_eyre::Result<()> {
    let client_dir = context.out_dir.join("client");

    let static_file_service = ServeDir::new(&client_dir)
        .precompressed_br()
        .precompressed_zstd()
        .precompressed_gzip()
        .precompressed_deflate();

    let state = AppState {
        tx,
        static_file_service,
        backend_url: "http://127.0.0.1:3000".to_string(),
        client_dir,
        client: reqwest::Client::new(),
    };

    let app = Router::new()
        .route(THAW_CLI_WS_PATH, get(thaw_cli_ws))
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

async fn handler(State(state): State<AppState>, request: Request<body::Body>) -> Response {
    if request.method() == Method::GET {
        let mut path = request.uri().path().to_string();
        if path.starts_with("/") {
            path.remove(0);
        }
        let file_path = state.client_dir.join(path);
        if fs::metadata(&file_path)
            .await
            .map_or(false, |f| f.is_file())
        {
            let rt = match state.static_file_service.clone().oneshot(request).await {
                Ok(response) => response.into_response(),
                Err(_) => (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response(),
            };
            return rt;
        }
    }

    proxy_to_backend(state, request).await
}

async fn proxy_to_backend(state: AppState, request: Request<body::Body>) -> Response {
    use reqwest::header::{HeaderMap, HeaderValue};

    let (parts, body) = request.into_parts();

    let backend_url = format!(
        "{}{}",
        state.backend_url,
        parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("")
    );
    let url: reqwest::Url = match backend_url.parse() {
        Ok(url) => url,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid backend URL").into_response();
        }
    };

    let body_bytes = match body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut headers = HeaderMap::new();
    for (name, value) in parts.headers.into_iter() {
        let Some(name) = name else {
            continue;
        };
        if is_hop_by_hop_header(&name) {
            continue;
        }
        headers.insert(name, value);
    }

    let request = state.client.request(parts.method, url).headers(headers);
    let request = if body_bytes.is_empty() {
        request
    } else {
        request.body(body_bytes)
    };
    // .header("X-Proxy-Request", HeaderValue::from_static("true"));
    println!("request {:#?}", request);
    match request.send().await {
        Ok(response) => {
            let res: Response<reqwest::Body> = response.into();
            let (headers, body) = res.into_parts();
            println!("send ok {}", headers.status);
            let body = body.collect().await.unwrap().to_bytes().into();
            Response::from_parts(headers, body)
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "Backend service unavailable").into_response(),
    }
}

fn is_hop_by_hop_header(name: &reqwest::header::HeaderName) -> bool {
    // match name.as_str() {
    //     "connection"
    //     | "keep-alive"
    //     | "proxy-authenticate"
    //     | "proxy-authorization"
    //     | "te"
    //     | "trailer"
    //     | "transfer-encoding"
    //     | "upgrade" => true,
    //     _ => false,
    // }
    false
}
