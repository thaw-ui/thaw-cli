use super::{build, build_wasm_path, common::wasm_bindgen};
use crate::context::Context;

pub async fn run(context: &Context) -> color_eyre::Result<()> {
    let mut cargo_args = vec![
        "--target=wasm32-unknown-unknown",
        "--lib",
        "--features=hydrate",
    ];
    if context.config.release {
        cargo_args.push("--release");
    }
    build(cargo_args)?;
    wasm_bindgen(context, &build_wasm_path(context)?, &context.out_dir).await?;
    color_eyre::Result::Ok(())
}
