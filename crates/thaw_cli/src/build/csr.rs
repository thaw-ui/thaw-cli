use crate::{cli, context::Context};
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
        .cli_tx
        .send(cli::Message::Build("Packaging index.html file".to_string()))
        .await?;

    let html_path = context.current_dir.join("index.html");
    if !context.serve && !fs::exists(&html_path)? {
        return Err(eyre!(
            "No index.html file was found in the root directory. Location: {html_path:?}"
        ));
    }
    let mut html_str = fs::read_to_string(html_path)?;
    let Some(body_end_index) = html_str.find("</body>") else {
        return color_eyre::Result::Err(eyre!("No end tag found for body"));
    };

    let package_name = context.cargo_package_name()?;
    let assets_path = &context.config.build.assets_dir;
    let mut import_script = format!(
        r#"<script type="module">import init from '/{assets_path}/{package_name}.js';await init({{ module_or_path: '/{assets_path}/{package_name}_bg.wasm' }})</script>"#,
    );

    if context.serve {
        import_script.push_str(r#"<script src="/__thaw_cli__.js"></script>"#);
    }

    html_str.insert_str(body_end_index, &import_script);

    let out_dir = &context.out_dir;

    let new_html_path = out_dir.join("index.html");
    let mut file = fs::File::create(new_html_path)?;
    file.write_all(html_str.as_bytes())?;

    if context.serve {
        let path = out_dir.join("__thaw_cli__.js");
        if !tokio::fs::try_exists(&path).await? {
            let mut file = fs::File::create_new(path)?;
            file.write_all(include_str!("./__thaw_cli__.js").as_bytes())?;
        }
    }

    Ok(())
}
