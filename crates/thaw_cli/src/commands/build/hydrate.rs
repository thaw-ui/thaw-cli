use super::{BuildCommands, build_wasm_path, wasm_bindgen, wasm_opt};
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
    BuildCommands::build(cargo_args)?;
    wasm_bindgen(&build_wasm_path(&context)?, &context.wasm_bindgen_dir)?;
    wasm_opt(&context.wasm_bindgen_dir, &context.out_dir).await?;
    color_eyre::Result::Ok(())
}
