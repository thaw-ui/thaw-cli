use axum::{
    body::Bytes,
    extract::{
        State, WebSocketUpgrade,
        ws::{self, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
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
    ws.protocols(vec!["thaw-cli-ping"])
        .on_upgrade(move |socket| handle_thaw_cli_ws(socket, state.tx.clone(), false))
}

pub async fn handle_thaw_cli_ws(socket: WebSocket, tx: broadcast::Sender<()>, cargo_leptos: bool) {
    if let Some(protocol) = socket.protocol() {
        if let Ok(protocol) = protocol.to_str() {
            if protocol == "thaw-cli-ping" {
                return;
            }
        }
    }
    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let mut recv_task = task::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, ws::Message::Close(_)) {
                break;
            }
        }
    });
    let mut send_task = task::spawn(async move {
        let _ = sender.send(WsMessage::Connected.into(cargo_leptos)).await;
        while (rx.recv().await).is_ok() {
            let _ = sender.send(WsMessage::RefreshPage.into(cargo_leptos)).await;
        }
    });

    tokio::select! {
        _rv_a = (&mut send_task) => {
            recv_task.abort();
        },
        _rv_b = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsMessage {
    Connected,
    RefreshPage,
}

impl WsMessage {
    fn into(self, cargo_leptos: bool) -> ws::Message {
        if cargo_leptos {
            match self {
                WsMessage::Connected => ws::Message::Ping(Bytes::new()),
                WsMessage::RefreshPage => ws::Message::text(r#"{"all":""}"#),
            }
        } else {
            let value = serde_json::to_string(&self).unwrap();
            ws::Message::text(value)
        }
    }
}
