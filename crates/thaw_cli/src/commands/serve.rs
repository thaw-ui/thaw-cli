use crate::{commands::build::BuildCommands, context::Context};
use axum::{Router, routing::get_service};
use clap::Subcommand;
use tokio::{net::TcpListener, runtime::Builder};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Subcommand)]
pub enum ServeCommands {
    Csr,
    Ssr,
}

impl ServeCommands {
    pub fn run(self, context: Context) -> color_eyre::Result<()> {
        match self {
            ServeCommands::Csr => Self::run_csr(context)?,
            ServeCommands::Ssr => todo!(),
        }
        color_eyre::Result::Ok(())
    }

    pub fn run_csr(context: Context) -> color_eyre::Result<()> {
        BuildCommands::Csr.run(&context)?;

        let out_dir = context
            .current_dir
            .join(context.config.build.out_dir.clone());

        let serve_dir =
            ServeDir::new(out_dir.clone()).fallback(ServeFile::new(out_dir.join("index.html")));

        let app = Router::new().fallback_service(get_service(serve_dir));

        let rt = Builder::new_multi_thread().enable_io().build()?;
        rt.block_on(async {
            let addr = format!(
                "{}:{}",
                context.config.server.host, context.config.server.port
            );

            let listener = match TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => return color_eyre::Result::Err(err.into()),
            };

            match axum::serve(listener, app).await {
                Ok(_) => color_eyre::Result::Ok(()),
                Err(err) => color_eyre::Result::Err(err.into()),
            }
        })
    }
}
