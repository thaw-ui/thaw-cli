use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{self, WebSocket},
    },
    response::Response,
};
use serde::Serialize;
use std::fmt::Debug;
use tokio::{
    sync::{broadcast, mpsc},
    task::{self, JoinHandle},
};

struct RunServeData {
    join_handle: Option<JoinHandle<()>>,
    tx: Option<broadcast::Sender<()>>,
    serve: Box<dyn Fn(broadcast::Sender<()>) -> JoinHandle<()> + Send + 'static>,
}

impl RunServeData {
    fn new(serve: impl Fn(broadcast::Sender<()>) -> JoinHandle<()> + Send + 'static) -> Self {
        Self {
            join_handle: None,
            tx: None,
            serve: Box::new(serve),
        }
    }

    fn abort(&mut self) {
        if let Some(jh) = self.join_handle.take() {
            jh.abort();
            self.tx.take();
        }
    }

    fn run_serve(&mut self) {
        self.abort();

        let (tx, _) = broadcast::channel(10);
        let jh = (self.serve)(tx.clone());

        self.join_handle = Some(jh);
        self.tx = Some(tx);
    }
}

#[derive(Debug)]
pub enum ServeEvent {
    Restart,
    RefreshPage,
}

impl ServeEvent {
    fn run(self, data: &mut RunServeData) {
        match self {
            ServeEvent::Restart => {
                data.run_serve();
            }
            ServeEvent::RefreshPage => {
                if let Some(tx) = &data.tx {
                    let _ = tx.send(());
                } else {
                    ServeEvent::Restart.run(data);
                }
            }
        }
    }
}

pub fn run_serve(
    serve: impl Fn(broadcast::Sender<()>) -> JoinHandle<()> + Send + 'static,
    mut serve_rx: mpsc::Receiver<ServeEvent>,
) {
    task::spawn(async move {
        let mut data = RunServeData::new(serve);
        while let Some(event) = serve_rx.recv().await {
            event.run(&mut data);
        }
    });
}

#[derive(Debug, Clone)]
pub struct ThawCliWs {
    tx: broadcast::Sender<()>,
}

impl ThawCliWs {
    pub fn new(tx: broadcast::Sender<()>) -> Self {
        Self { tx }
    }
}

pub static THAW_CLI_WS_PATH: &str = "/__thaw_cli__";

pub async fn thaw_cli_ws(ws: WebSocketUpgrade, State(state): State<ThawCliWs>) -> Response {
    ws.on_upgrade(|socket| handle_thaw_cli_ws(socket, state))
}

async fn handle_thaw_cli_ws(mut socket: WebSocket, state: ThawCliWs) {
    let _ = socket.send(WsMessage::Connected.into()).await;
    let mut rx = state.tx.subscribe();
    task::spawn(async move {
        while (rx.recv().await).is_ok() {
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
