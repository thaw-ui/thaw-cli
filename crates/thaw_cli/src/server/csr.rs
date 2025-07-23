use super::Event;
use crate::{
    build::{assets, clear_out_dir, collect_assets, csr, run_cargo_build, wasm_bindgen},
    cli,
    context::Context,
    utils::DotEyre,
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
    watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    assets: Vec<assets::BundledAsset>,
    event_rx: mpsc::Receiver<Event>,
    page_tx: Option<broadcast::Sender<()>>,
}

impl DevServer {
    pub fn new(context: Arc<Context>) -> color_eyre::Result<Self> {
        let (event_tx, event_rx) = mpsc::channel::<Event>(240);

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
                        event_tx.blocking_send(Event::Watch(paths)).unwrap();
                    }
                }
                Err(e) => println!("watch error: {e:?}"),
            },
        )?;

        Ok(Self {
            context,
            watcher,
            assets: Vec::new(),
            event_rx,
            page_tx: None,
        })
    }

    pub async fn run(mut self, assets: Vec<assets::BundledAsset>) -> color_eyre::Result<Self> {
        let src_dir = self.context.current_dir.join("src");
        self.watcher.watch(src_dir, RecursiveMode::Recursive)?;
        let index_html = self.context.current_dir.join("index.html");
        self.watcher.watch(index_html, RecursiveMode::Recursive)?;
        self.watch_assets(assets)?;

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
                    if paths.is_empty() {
                        continue;
                    }

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

    async fn rebuild(&mut self, paths: &Vec<PathBuf>) -> color_eyre::Result<()> {
        if paths.len() == 1 && paths[0] == self.context.current_dir.join("index.html") {
            csr::build_index_html(&self.context).await?;
        } else if let Some(asset_subset) = assets::asset_subset(&self.assets, paths) {
            for asset in asset_subset {
                fs::remove_file(&asset.output_path).await?;
                dioxus_cli_opt::process_file_to(
                    &asset.options,
                    &asset.absolute_source_path,
                    &asset.output_path,
                )
                .dot_eyre()?;
            }
        } else {
            let wasm_path =
                run_cargo_build(&self.context, csr::cargo_build_args(&self.context)).await?;
            clear_out_dir(&self.context).await?;
            fs::create_dir_all(&self.context.assets_dir).await?;
            let assets = collect_assets(&self.context, wasm_path, &self.context.assets_dir).await?;
            wasm_bindgen(&self.context, None, &self.context.assets_dir).await?;
            self.watch_assets(assets)?;
        }

        // When no page is open, this send will report an error.
        let _ = self.page_tx.as_ref().unwrap().send(());
        Ok(())
    }

    fn watch_assets(&mut self, assets: Vec<assets::BundledAsset>) -> color_eyre::Result<()> {
        for asset in &self.assets {
            self.watcher.unwatch(&asset.absolute_source_path)?;
        }
        for asset in &assets {
            self.watcher
                .watch(&asset.absolute_source_path, RecursiveMode::Recursive)?;
        }
        self.assets = assets;
        Ok(())
    }
}
