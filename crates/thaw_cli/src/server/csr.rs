use super::Event;
use crate::{
    build::{clear_out_dir, collect_assets, csr, run_cargo_build, wasm_bindgen},
    cli,
    context::Context,
};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, FileIdMap, new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs,
    sync::{broadcast, mpsc},
    task,
};

pub struct DevServer {
    context: Arc<Context>,
    _watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    event_rx: mpsc::Receiver<Event>,
    page_tx: Option<broadcast::Sender<()>>,
}

impl DevServer {
    pub fn new(context: Arc<Context>) -> color_eyre::Result<Self> {
        let (event_tx, event_rx) = mpsc::channel::<Event>(240);

        let mut watcher = new_debouncer(
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
                        event_tx.blocking_send(Event::Watch(paths)).unwrap();
                    }
                }
                Err(e) => println!("watch error: {e:?}"),
            },
        )?;
        let src_dir = context.current_dir.join("src");
        watcher.watch(src_dir, RecursiveMode::Recursive)?;
        let index_html = context.current_dir.join("index.html");
        watcher.watch(index_html, RecursiveMode::Recursive)?;

        Ok(Self {
            context,
            _watcher: watcher,
            event_rx,
            page_tx: None,
        })
    }

    pub async fn run(mut self) -> color_eyre::Result<Self> {
        let (page_tx, _) = broadcast::channel(10);
        task::spawn({
            let context = self.context.clone();
            let page_tx = page_tx.clone();
            async move { super::csr_app::run(context, page_tx).await }
        });
        self.page_tx = Some(page_tx);
        Ok(self)
    }

    pub async fn wait_event(mut self) -> color_eyre::Result<()> {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                Event::Watch(paths) => {
                    let build_result = self.rebuild(&paths).await;
                    self.context
                        .cli_tx
                        .send(cli::Message::PageReload(paths, build_result))
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn rebuild(&mut self, _paths: &Vec<PathBuf>) -> color_eyre::Result<()> {
        let wasm_path =
            run_cargo_build(&self.context, csr::cargo_build_args(&self.context)).await?;
        clear_out_dir(&self.context).await?;
        csr::build_index_html(&self.context).await?;
        fs::create_dir_all(&self.context.assets_dir).await?;
        collect_assets(&self.context, wasm_path, &self.context.assets_dir).await?;
        wasm_bindgen(&self.context, None, &self.context.assets_dir).await?;

        self.page_tx.as_ref().unwrap().send(())?;
        Ok(())
    }
}
