use std::sync::Arc;

use crate::context::Context;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::{Sender, channel};

pub async fn watch(context: Arc<Context>, build_tx: Sender<()>) -> color_eyre::Result<()> {
    let (tx, mut rx) = channel(10);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            tx.blocking_send(res).unwrap();
        },
        Config::default(),
    )?;

    let src_dir = context.current_dir.join("src");
    watcher.watch(&src_dir, RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                let Event { kind, .. } = event;
                match kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        build_tx.send(()).await.unwrap();
                    }
                    _ => {}
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        }
    }

    color_eyre::Result::Ok(())
}
