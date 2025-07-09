use crate::{cli, context::Context};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, FileIdMap, new_debouncer,
    notify::{EventKind, ReadDirectoryChangesWatcher},
};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::mpsc, task};

pub trait WatchBuild {
    fn build(&self) -> impl Future<Output = color_eyre::Result<()>> + Send;
}

pub fn watch_file_and_rebuild(
    context: Arc<Context>,
    build: impl WatchBuild + Send + 'static,
) -> color_eyre::Result<Debouncer<ReadDirectoryChangesWatcher, FileIdMap>> {
    let (build_tx, mut build_rx) = mpsc::channel::<Vec<PathBuf>>(240);

    let watcher = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                let paths = events
                    .into_iter()
                    .filter(|e| matches!(e.kind, EventKind::Create(_) | EventKind::Modify(_)))
                    .flat_map(|e| e.event.paths)
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    build_tx.blocking_send(paths).unwrap();
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        },
    )?;

    task::spawn(async move {
        let mut paths_batch = vec![];
        while let Some(mut paths) = build_rx.recv().await {
            paths_batch.append(&mut paths);
            while let Ok(mut paths) = build_rx.try_recv() {
                paths_batch.append(&mut paths);
            }

            let build_result = build.build().await;

            let paths = paths_batch
                .drain(..)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            context
                .cli_tx
                .send(cli::Message::PageReload(paths, build_result))
                .await
                .unwrap();
        }
    });

    Ok(watcher)
}
