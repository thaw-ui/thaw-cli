use super::watch;
use crate::{commands::build::BuildCommands, context::Context};
use axum::{Router, routing::get_service};
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    runtime::Builder,
    sync::mpsc::channel,
    task::{self, JoinHandle},
};
use tower_http::services::{ServeDir, ServeFile};

pub fn run(context: Context) -> color_eyre::Result<()> {
    let rt = Builder::new_multi_thread().enable_io().build()?;
    rt.block_on(async {
        let context = Arc::new(context);
        let (build_tx, mut build_rx) = channel::<()>(1);
        let (serve_tx, mut serve_rx) = channel::<ServeEvent>(1);

        task::spawn({
            let context = context.clone();
            async move {
                while let Some(_) = build_rx.recv().await {
                    BuildCommands::Csr.run(&context).unwrap();
                    serve_tx.send(ServeEvent::Restart).await.unwrap();
                }
            }
        });

        task::spawn({
            let context = context.clone();
            async move {
                let mut handle = None::<JoinHandle<()>>;

                while let Some(event) = serve_rx.recv().await {
                    match event {
                        ServeEvent::Restart => {
                            if let Some(h) = handle.take() {
                                h.abort();
                            }
                            let context = context.clone();
                            let jh = task::spawn(async {
                                run_serve(context).await.unwrap();
                            });
                            handle = Some(jh);
                        }
                    }
                }
            }
        });

        build_tx.send(()).await.unwrap();

        watch::watch(context, build_tx).await.unwrap();

        color_eyre::Result::Ok(())
    })
}

enum ServeEvent {
    Restart,
}

async fn run_serve(context: Arc<Context>) -> color_eyre::Result<()> {
    let out_dir = context
        .current_dir
        .join(context.config.build.out_dir.clone());

    let serve_dir =
        ServeDir::new(out_dir.clone()).fallback(ServeFile::new(out_dir.join("index.html")));

    let app = Router::new().fallback_service(get_service(serve_dir));

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
