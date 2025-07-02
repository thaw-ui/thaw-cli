use crate::{cli, context::Context};
use color_eyre::eyre::eyre;
use std::{fs, io::Write, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

pub fn build_index_html(context: &Context, serve: bool) -> color_eyre::Result<()> {
    let html_path = context.current_dir.join("index.html");
    let mut html_str = fs::read_to_string(html_path)?;
    let Some(body_end_index) = html_str.find("</body>") else {
        return color_eyre::Result::Err(eyre!("No end tag found for body"));
    };

    let package_name = context.cargo_package_name()?;
    let assets_path = &context.config.build.assets_dir;
    let mut import_script = format!(
        r#"<script type="module">import init from '/{assets_path}/{package_name}.js';await init({{ module_or_path: '/{assets_path}/{package_name}_bg.wasm' }})</script>"#,
    );

    if serve {
        import_script.push_str(r#"<script src="/__thaw_cli__.js"></script>"#);
    }

    html_str.insert_str(body_end_index, &import_script);

    let out_dir = &context.out_dir;

    let new_html_path = out_dir.join("index.html");
    let mut file = fs::File::create_new(new_html_path)?;
    file.write_all(html_str.as_bytes())?;

    if serve {
        let path = out_dir.join("__thaw_cli__.js");
        let mut file = fs::File::create_new(path)?;
        file.write_all(include_str!("./__thaw_cli__.js").as_bytes())?;
    }

    color_eyre::Result::Ok(())
}

pub async fn build_wasm(context: &Context, serve: bool) -> color_eyre::Result<()> {
    let mut cmd = Command::new("cargo");

    cmd.arg("build").arg("--target=wasm32-unknown-unknown");
    if context.cargo_features_contains_key("csr") {
        cmd.arg("--features=csr");
    }
    if context.config.release {
        cmd.arg("--release");
    }

    cmd.arg("--message-format=json-diagnostic-rendered-ansi");

    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    let mut stdout = stdout.lines();
    let mut stderr = stderr.lines();

    loop {
        use cargo_metadata::Message;

        let line = tokio::select! {
            Ok(Some(line)) = stdout.next_line() => line,
            Ok(Some(line)) = stderr.next_line() => line,
            else => break,
        };

        let Some(Ok(message)) = Message::parse_stream(std::io::Cursor::new(line)).next() else {
            continue;
        };

        let message = match message {
            Message::CompilerArtifact(_artifact) => {
                // TODO
                None
            }
            Message::BuildScriptExecuted(_build_script) => {
                // TODO
                None
            }
            Message::CompilerMessage(compiler_message) => Some(cli::Message::CargoPackaging(
                compiler_message.message.into(),
            )),
            Message::TextLine(value) => Some(cli::Message::CargoPackaging(value.into())),
            Message::BuildFinished(build_finished) => {
                if !build_finished.success {
                    if serve {
                        todo!()
                    } else {
                        return Err(eyre!("Cargo build failed"));
                    }
                }
                Some(cli::Message::CargoBuildFinished)
            }
            _ => None,
        };

        if let Some(message) = message {
            let _ = context.cli_tx.clone().unwrap().send(message).await;
        }
    }

    Ok(())
}
