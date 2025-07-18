use crate::{context::Context, dx, utils::DotEyre};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs;

pub async fn collect_assets(
    context: &Context,
    output_location: Option<PathBuf>,
    out_dir: &Path,
) -> color_eyre::Result<()> {
    if !context.config.build.assets_manganis {
        return Ok(());
    }
    let exe = output_location.unwrap();
    let manifest = dx::assets::extract_assets_from_file(exe)?;

    for bundled in manifest.assets() {
        let absolute_source_path = PathBuf::from_str(bundled.absolute_source_path())?;
        if !fs::try_exists(&absolute_source_path).await? {
            // TODO
            continue;
        }
        if absolute_source_path.is_dir() {
            // TODO
        } else {
            let file_path = out_dir.join(bundled.bundled_path());
            dioxus_cli_opt::process_file_to(bundled.options(), &absolute_source_path, &file_path)
                .dot_eyre()?;
        }
    }

    Ok(())
}
