use crate::{context::Context, server::ssr::default_env};
use std::path::Path;
use tokio::{fs, io::AsyncWriteExt};

pub async fn build_env_file(context: &Context, out_dir: &Path) -> color_eyre::Result<()> {
    let envs = default_env(context)?;
    let envs = envs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let path = out_dir.join(".env");
    let mut file = fs::File::create(path).await?;
    file.write_all(envs.as_bytes()).await?;
    Ok(())
}
