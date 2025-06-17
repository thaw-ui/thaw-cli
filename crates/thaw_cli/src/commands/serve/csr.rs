use super::watch;
use crate::{commands::build::BuildCommands, context::Context};
use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{self, WebSocket},
    },
    response::Response,
    routing::{get, get_service},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    runtime::Builder,
    sync::{broadcast, mpsc},
    task::{self, JoinHandle},
};
use tower_http::services::{ServeDir, ServeFile};

pub fn run(context: Context) -> color_eyre::Result<()> {
    let rt = Builder::new_multi_thread().enable_io().build()?;
    rt.block_on(async {
        let context = Arc::new(context);
        let (build_tx, mut build_rx) = mpsc::channel::<()>(1);
        let (serve_tx, mut serve_rx) = mpsc::channel::<ServeEvent>(1);

        task::spawn({
            let context = context.clone();
            async move {
                while let Some(_) = build_rx.recv().await {
                    BuildCommands::Csr.run(&context, true).unwrap();
                    serve_tx.send(ServeEvent::RefreshPage).await.unwrap();
                }
            }
        });

        task::spawn({
            let context = context.clone();
            async move {
                let mut handle = None::<(broadcast::Sender<()>, JoinHandle<()>)>;

                while let Some(event) = serve_rx.recv().await {
                    event.run(&mut handle, context.clone());
                }
            }
        });

        build_tx.send(()).await.unwrap();

        watch::watch(context, build_tx).await.unwrap();

        color_eyre::Result::Ok(())
    })
}

#[derive(Debug)]
enum ServeEvent {
    Restart,
    RefreshPage,
}

impl ServeEvent {
    fn run(
        self,
        handle: &mut Option<(broadcast::Sender<()>, JoinHandle<()>)>,
        context: Arc<Context>,
    ) {
        match self {
            ServeEvent::Restart => {
                if let Some((_, jh)) = handle.take() {
                    jh.abort();
                }

                let (tx, _) = broadcast::channel(10);
                let thaw_cli_ws = ThawCliWs { tx: tx.clone() };
                let jh = task::spawn(async {
                    run_serve(context, thaw_cli_ws).await.unwrap();
                });

                *handle = Some((tx, jh));
            }
            ServeEvent::RefreshPage => {
                if let Some((tx, _)) = &handle {
                    let _ = tx.send(());
                } else {
                    ServeEvent::Restart.run(handle, context);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ThawCliWs {
    tx: broadcast::Sender<()>,
}

async fn run_serve(context: Arc<Context>, state: ThawCliWs) -> color_eyre::Result<()> {
    let out_dir = context
        .current_dir
        .join(context.config.build.out_dir.clone());

    let serve_dir =
        ServeDir::new(out_dir.clone()).fallback(ServeFile::new(out_dir.join("index.html")));

    let app = Router::new()
        .route("/__thaw_cli__", get(thaw_cli_ws))
        .fallback_service(get_service(serve_dir))
        .with_state(state);

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

async fn thaw_cli_ws(ws: WebSocketUpgrade, State(state): State<ThawCliWs>) -> Response {
    ws.on_upgrade(|socket| handle_thaw_cli_ws(socket, state))
}

async fn handle_thaw_cli_ws(mut socket: WebSocket, state: ThawCliWs) {
    let _ = socket.send(WsMessage::Connected.into());
    let mut rx = state.tx.subscribe();
    task::spawn(async move {
        while let Ok(_) = rx.recv().await {
            let _ = socket.send(WsMessage::RefreshPage.into()).await;
        }
    });
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsMessage {
    Connected,
    RefreshPage,
}

impl From<WsMessage> for ws::Message {
    fn from(value: WsMessage) -> Self {
        let value = serde_json::to_string(&value).unwrap();
        Self::text(value)
    }
}
