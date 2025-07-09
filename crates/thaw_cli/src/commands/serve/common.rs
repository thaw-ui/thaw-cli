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
    sync::broadcast,
    task::{self, JoinHandle},
};

pub trait RunServe {
    fn run(&self, page_tx: broadcast::Sender<()>) -> Vec<JoinHandle<color_eyre::Result<()>>>;
}

pub struct RunServeData {
    join_handle: Option<Vec<JoinHandle<color_eyre::Result<()>>>>,
    pub page_tx: Option<broadcast::Sender<()>>,
    serve: Box<dyn RunServe>,
}

impl RunServeData {
    pub fn new(serve: impl RunServe + 'static) -> Self {
        Self {
            join_handle: None,
            page_tx: None,
            serve: Box::new(serve),
        }
    }

    fn abort(&mut self) {
        if let Some(jh_list) = self.join_handle.take() {
            jh_list.into_iter().for_each(|jh| {
                jh.abort();
            });
            self.page_tx.take();
        }
    }

    pub fn run_serve(&mut self) {
        self.abort();

        let (tx, _) = broadcast::channel(10);
        let jh = self.serve.run(tx.clone());

        self.join_handle = Some(jh);
        self.page_tx = Some(tx);
    }
}

#[derive(Debug)]
pub enum ServeEvent {
    // Restart,
    RefreshPage,
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
    ws.on_upgrade(move |socket| handle_thaw_cli_ws(socket, state.tx.clone()))
}

pub async fn handle_thaw_cli_ws(mut socket: WebSocket, tx: broadcast::Sender<()>) {
    let _ = socket.send(WsMessage::Connected.into()).await;
    let mut rx = tx.subscribe();
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
