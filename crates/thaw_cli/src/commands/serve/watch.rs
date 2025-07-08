use crate::context::Context;
use notify_debouncer_full::{
    DebounceEventResult, new_debouncer,
    notify::{EventKind, RecursiveMode},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::mpsc::{Sender, channel};

pub async fn watch(
    context: Arc<Context>,
    build_tx: Sender<Vec<PathBuf>>,
) -> color_eyre::Result<()> {
    let (tx, mut rx) = channel(10);

    let mut watcher = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            tx.blocking_send(result).unwrap();
        },
    )?;

    let src_dir = context.current_dir.join("src");
    watcher.watch(&src_dir, RecursiveMode::Recursive)?;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(events) => {
                let paths = events
                    .into_iter()
                    .filter(|e| matches!(e.kind, EventKind::Create(_) | EventKind::Modify(_)))
                    .map(|e| e.event.paths)
                    .flatten()
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    build_tx.send(paths).await.unwrap();
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        }
    }

    Ok(())
}
