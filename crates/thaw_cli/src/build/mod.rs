pub mod assets;
pub mod csr;
pub mod hydrate;
mod wasm;

pub use assets::collect_assets;
pub use wasm::wasm_bindgen;

use crate::{
    cli,
    context::Context,
    utils::fs::{clear_dir, copy_dir_all},
};
use color_eyre::eyre::eyre;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[inline]
pub async fn clear_out_dir(context: &Context) -> color_eyre::Result<()> {
    // The server stage clears temporary folders in the `target` directory,
    // so no prompt is needed to avoid misleading users.
    if !context.serve {
        context
            .cli_tx
            .send(cli::Message::Build(
                "Cleaning up the out_dir directory".to_string(),
            ))
            .await?;
    }
    clear_dir(&context.out_dir).await?;
    Ok(())
}

pub async fn copy_public_dir(context: &Context, out_dir: &Path) -> color_eyre::Result<()> {
    if context.serve {
        return Ok(());
    }

    if context.config.public_dir.is_empty() {
        return Ok(());
    }
    let public_dir = context.current_dir.join(context.config.public_dir.clone());
    if !fs::try_exists(&public_dir).await? {
        return Ok(());
    }

    context
        .cli_tx
        .send(cli::Message::Build(
            "Copying public_dir directory".to_string(),
        ))
        .await?;
    copy_dir_all(public_dir, out_dir).await?;
    Ok(())
}

pub fn cargo_build_exe_name(context: &Context) -> color_eyre::Result<String> {
    let mut exe_name = context.cargo_package_name()?;
    if cfg!(windows) {
        exe_name.push_str(".exe");
    }
    Ok(exe_name)
}

pub async fn run_cargo_build(
    context: &Context,
    args: Vec<&'static str>,
) -> color_eyre::Result<Option<PathBuf>> {
    let mut cmd = Command::new("cargo");

    cmd.arg("build");
    cmd.args(args);
    if context.config.release {
        cmd.arg("--release");
    }
    cmd.arg("--message-format=json-diagnostic-rendered-ansi");

    if context.serve && context.config.server.erase_components {
        cmd.env("RUSTFLAGS", "--cfg erase_components");
    }

    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    let mut stdout = stdout.lines();
    let mut stderr = stderr.lines();
    let mut output_location: Option<PathBuf> = None;

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
            Message::CompilerArtifact(artifact) => {
                // TODO

                output_location = artifact.executable.map(Into::into);
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
                    if context.serve {
                        // todo!()
                        return Err(eyre!("Cargo build failed"));
                    } else {
                        return Err(eyre!("Cargo build failed"));
                    }
                }
                Some(cli::Message::CargoBuildFinished)
            }
            _ => None,
        };

        if let Some(message) = message {
            let _ = context.cli_tx.send(message).await;
        }
    }

    Ok(output_location)
}
