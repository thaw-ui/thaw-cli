use crate::{
    build::{
        assets, cargo_build_exe_name, clear_out_dir, collect_assets, copy_public_dir, csr, hydrate,
        run_cargo_build, wasm_bindgen,
    },
    context::Context,
};
use clap::Subcommand;
use std::sync::Arc;
use tokio::fs;

#[derive(Debug, Subcommand)]
pub enum BuildCommands {
    /// Client-side rendering
    Csr,
    /// Server-side Rendering
    Ssr,
}

impl BuildCommands {
    pub async fn run(
        self,
        context: &Arc<Context>,
    ) -> color_eyre::Result<Vec<assets::BundledAsset>> {
        match self {
            Self::Csr => {
                let wasm_path = run_cargo_build(context, csr::cargo_build_args(context)).await?;
                clear_out_dir(context).await?;
                if !context.serve {
                    copy_public_dir(context, &context.out_dir).await?;
                }
                csr::build_index_html(context).await?;
                fs::create_dir_all(&context.assets_dir).await?;
                let assets = collect_assets(context, wasm_path, &context.assets_dir).await?;
                wasm_bindgen(context, None, &context.assets_dir).await?;
                Ok(assets)
            }
            Self::Ssr => {
                clear_out_dir(context).await?;

                let client_out_dir = context.out_dir.join("client");
                let server_out_dir = context.out_dir.join("server");
                let assets_dir = client_out_dir.join(&context.config.build.assets_dir);

                fs::create_dir_all(&assets_dir).await?;
                if !context.serve {
                    copy_public_dir(context, &client_out_dir).await?;
                }

                run_cargo_build(context, hydrate::cargo_build_args()).await?;
                wasm_bindgen(context, None, &assets_dir).await?;

                let exe_path = run_cargo_build(context, vec!["--features=ssr"])
                    .await?
                    .unwrap();
                let assets = collect_assets(context, Some(exe_path.clone()), &assets_dir).await?;
                fs::create_dir_all(&server_out_dir).await?;
                fs::copy(
                    exe_path,
                    server_out_dir.join(cargo_build_exe_name(context)?),
                )
                .await?;
                Ok(assets)
            }
        }
    }
}
