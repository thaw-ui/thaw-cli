use super::Event;
use crate::{
    build::{assets, cargo_build_exe_name, collect_assets, hydrate, run_cargo_build, wasm_bindgen},
    cli,
    context::Context,
    utils::{DotEyre, fs::clear_dir},
};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, FileIdMap, new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    fs,
    process::{Child, Command},
    sync::{broadcast, mpsc},
    task::{self, JoinHandle},
};
use tokio_util::sync::CancellationToken;

pub struct DevServer {
    context: Arc<Context>,
    watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    assets: Vec<assets::BundledAsset>,
    event_rx: mpsc::Receiver<Event>,

    ssr_exe_join_handle: Option<JoinHandle<color_eyre::Result<()>>>,
    cancellation_token: Option<CancellationToken>,

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

            ssr_exe_join_handle: None,
            cancellation_token: None,

            page_tx: None,
        })
    }

    pub async fn run(mut self, assets: Vec<assets::BundledAsset>) -> color_eyre::Result<Self> {
        let src_dir = self.context.current_dir.join("src");
        self.watcher.watch(src_dir, RecursiveMode::Recursive)?;
        self.watch_assets(assets)?;

        self.run_ssr_exe();

        let (page_tx, _) = broadcast::channel(10);
        task::spawn({
            let context = self.context.clone();
            let page_tx = page_tx.clone();
            async move { super::ssr_app::run(context, page_tx).await }
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

    async fn rebuild(&mut self, paths: &Vec<PathBuf>) -> color_eyre::Result<()> {
        if let Some(asset_subset) = assets::asset_subset(&self.assets, paths) {
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
            let client_out_dir = self.context.out_dir.join("client");
            let server_out_dir = self.context.out_dir.join("server");
            let assets_dir = client_out_dir.join(&self.context.config.build.assets_dir);

            run_cargo_build(&self.context, hydrate::cargo_build_args()).await?;
            clear_dir(&assets_dir).await?;
            wasm_bindgen(&self.context, None, &assets_dir).await?;

            let exe_path = run_cargo_build(&self.context, vec!["--features=ssr"])
                .await?
                .unwrap();
            let assets = collect_assets(&self.context, Some(exe_path.clone()), &assets_dir).await?;
            self.watch_assets(assets)?;

            self.abort_ssr_exe().await?;

            fs::copy(
                exe_path,
                server_out_dir.join(cargo_build_exe_name(&self.context)?),
            )
            .await?;

            self.run_ssr_exe();
        }
        self.page_tx.as_ref().unwrap().send(())?;
        Ok(())
    }

    fn run_ssr_exe(&mut self) {
        let cancellation_token = CancellationToken::new();
        let join_handle = task::spawn({
            let context = self.context.clone();
            let cancellation_token = cancellation_token.clone();
            async move {
                let mut child = run_ssr_exe(context)?;
                tokio::select! {
                    rt = child.wait() => {
                        let _ = rt?;
                        Ok(())
                    }
                    _ = cancellation_token.cancelled() => {
                        child.kill().await?;
                        child.wait().await?;
                        Ok(())
                    }
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.ssr_exe_join_handle = Some(join_handle);
    }

    async fn abort_ssr_exe(&mut self) -> color_eyre::Result<()> {
        if let Some(join_handle) = self.ssr_exe_join_handle.take() {
            let cancellation_token = self.cancellation_token.take().unwrap();
            cancellation_token.cancel();
            join_handle.await??;
        }
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

fn run_ssr_exe(context: Arc<Context>) -> color_eyre::Result<Child> {
    let exe_path = context
        .out_dir
        .join("server")
        .join(cargo_build_exe_name(&context)?);

    let mut cmd = Command::new(exe_path);
    cmd.envs(context.env.cloned_into_iter());
    // cmd.env("LEPTOS_OUTPUT_NAME", context.cargo_package_name()?);
    // cmd.env("LEPTOS_SITE_PKG_DIR", "assets");
    // cmd.env("LEPTOS_WATCH", "");
    // cmd.env(
    //     "LEPTOS_RELOAD_EXTERNAL_PORT",
    //     context.config.server.port.to_string(),
    // );

    let child = cmd.spawn()?;
    Ok(child)
}

pub fn default_env(context: &Context) -> color_eyre::Result<Vec<(&'static str, String)>> {
    Ok(vec![
        ("LEPTOS_OUTPUT_NAME", context.cargo_package_name()?),
        ("LEPTOS_SITE_PKG_DIR", "assets".to_string()),
        ("LEPTOS_WATCH", String::new()),
        (
            "LEPTOS_RELOAD_EXTERNAL_PORT",
            context.config.server.port.to_string(),
        ),
        ("LEPTOS_SITE_ADDR", "127.0.0.1:3000".to_string()),
    ])
}
