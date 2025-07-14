use crate::context::Context;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

pub async fn collect_assets(
    context: &Context,
    output_location: Option<PathBuf>,
    out_dir: &Path,
) -> color_eyre::Result<()> {
    if !context.config.build.assets_manganis {
        return Ok(());
    }
    use tokio::fs;
    // use dioxus_cli_opt::process_file_to();
    let exe = output_location.unwrap();
    let manifest = crate::dx::assets::extract_assets_from_file(exe)?;

    for bundled in manifest.assets() {
        let absolute_source_path = PathBuf::from_str(bundled.absolute_source_path())?;
        if !fs::try_exists(&absolute_source_path).await? {
            // TODO
            continue;
        }
        if absolute_source_path.is_dir() {
            // TODO
        } else {
            fs::copy(absolute_source_path, out_dir.join(bundled.bundled_path())).await?;
        }
    }

    Ok(())
}
