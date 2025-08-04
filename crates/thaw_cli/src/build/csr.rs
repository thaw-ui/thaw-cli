use crate::{context::Context, logger, plugins::html::BuildHtml};
use color_eyre::eyre::eyre;
use std::{fs, io::Write};

pub fn cargo_build_args(context: &Context) -> Vec<&'static str> {
    let mut args = vec!["--target=wasm32-unknown-unknown"];
    if context.cargo_features_contains_key("csr") {
        args.push("--features=csr");
    }
    args
}

pub async fn build_index_html(context: &Context) -> color_eyre::Result<()> {
    context
        .logger
        .send(logger::Message::Build(
            "Packaging index.html file".to_string(),
        ))
        .await?;

    let html_path = context.current_dir.join("index.html");
    if !context.serve && !fs::exists(&html_path)? {
        return Err(eyre!(
            "No index.html file was found in the root directory. Location: {html_path:?}"
        ));
    }
    let mut html = fs::read_to_string(html_path)?;

    html = BuildHtml::transform(context, html).await?;

    let out_dir = &context.out_dir;

    let new_html_path = out_dir.join("index.html");
    let mut file = fs::File::create(new_html_path)?;
    file.write_all(html.as_bytes())?;

    Ok(())
}
