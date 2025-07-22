use crate::{context::Context, dx, utils::DotEyre};
use manganis::AssetOptions;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs;

#[derive(Debug)]
pub struct BundledAsset {
    pub absolute_source_path: PathBuf,
    pub output_path: PathBuf,
    pub options: AssetOptions,
}

pub async fn collect_assets(
    context: &Context,
    output_location: Option<PathBuf>,
    out_dir: &Path,
) -> color_eyre::Result<Vec<BundledAsset>> {
    if !context.config.build.assets_manganis {
        return Ok(Vec::new());
    }
    let exe = output_location.unwrap();
    let manifest = dx::assets::extract_assets_from_file(exe)?;
    let mut assets = vec![];

    for bundled in manifest.assets() {
        let absolute_source_path = PathBuf::from_str(bundled.absolute_source_path())?;
        if !fs::try_exists(&absolute_source_path).await? {
            // TODO
            continue;
        }
        if absolute_source_path.is_dir() {
            // TODO
        } else {
            let output_path = out_dir.join(bundled.bundled_path());
            dioxus_cli_opt::process_file_to(bundled.options(), &absolute_source_path, &output_path)
                .dot_eyre()?;
            assets.push(BundledAsset {
                absolute_source_path,
                output_path,
                options: *bundled.options(),
            });
        }
    }

    Ok(assets)
}

pub fn asset_subset<'a>(
    assets: &'a [BundledAsset],
    paths: &Vec<PathBuf>,
) -> Option<Vec<&'a BundledAsset>> {
    let mut subset = vec![];
    for path in paths {
        let asset = assets
            .iter()
            .find(|asset| &asset.absolute_source_path == path)?;
        subset.push(asset);
    }
    Some(subset)
}
