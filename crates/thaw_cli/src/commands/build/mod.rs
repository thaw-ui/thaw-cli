use crate::{
    context::Context,
    utils::{DotEyre, copy_dir_all},
};
use clap::{Args, Subcommand};
use color_eyre::eyre::eyre;
use std::{fs, io::Write};
use wasm_bindgen_cli_support::Bindgen;
use xshell::{Shell, cmd};

#[derive(Debug, Subcommand)]
pub enum BuildCommands {
    Csr,
    Ssr(BuildSsrArgs),
    Hydrate,
}

impl BuildCommands {
    pub fn run(self, context: &Context, serve: bool) -> color_eyre::Result<()> {
        Self::clear_out_dir(context)?;
        Self::copy_public_dir(context)?;
        match self {
            Self::Csr => {
                Self::build_index_html(context, serve)?;
                let cargo_args = vec!["build", "--target=wasm32-unknown-unknown", "--features=csr"];
                Self::build(cargo_args)?;
                Self::wasm_bindgen(context)?;
            }
            Self::Ssr(build_ssr_args) => {
                if !build_ssr_args.no_hydrate {
                    BuildCommands::Hydrate.run(context, serve)?;
                }

                let cargo_args = vec!["build", "--features=ssr"];
                Self::build(cargo_args)?;
            }
            Self::Hydrate => {
                let cargo_args = vec![
                    "build",
                    "--target=wasm32-unknown-unknown",
                    "--features=hydrate",
                ];
                Self::build(cargo_args)?;
                Self::wasm_bindgen(context)?;
            }
        }
        color_eyre::Result::Ok(())
    }

    fn clear_out_dir(context: &Context) -> color_eyre::Result<()> {
        let out_dir = context
            .current_dir
            .join(context.config.build.out_dir.clone());
        if fs::exists(out_dir.clone())? {
            fs::remove_dir_all(out_dir.clone())?;
        }
        fs::create_dir_all(out_dir)?;
        color_eyre::Result::Ok(())
    }

    fn copy_public_dir(context: &Context) -> color_eyre::Result<()> {
        if context.config.public_dir.is_empty() {
            return color_eyre::Result::Ok(());
        }
        let out_dir = context
            .current_dir
            .join(context.config.build.out_dir.clone());
        let new_public_dir = out_dir.join(context.config.public_dir.clone());

        let public_dir = context.current_dir.join(context.config.public_dir.clone());

        if fs::exists(public_dir.clone())? {
            copy_dir_all(public_dir, new_public_dir)?;
        }

        color_eyre::Result::Ok(())
    }

    fn build_index_html(context: &Context, serve: bool) -> color_eyre::Result<()> {
        let html_path = context.current_dir.join("index.html");
        let mut html_str = fs::read_to_string(html_path)?;
        let Some(body_end_index) = html_str.find("</body>") else {
            return color_eyre::Result::Err(eyre!("No end tag found for body"));
        };

        let package_name = context.cargo_package_name()?;
        let mut import_script = format!(
            r#"<script type="module">import init from '/{package_name}.js';await init({{ module_or_path: '/{package_name}_bg.wasm' }})</script>"#,
        );

        if serve {
            import_script.push_str(r#"<script src="/__thaw_cli__.js"></script>"#);
        }

        html_str.insert_str(body_end_index, &import_script);

        let out_dir = context
            .current_dir
            .join(context.config.build.out_dir.clone());

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

    fn build(args: Vec<&'static str>) -> color_eyre::Result<()> {
        let sh = Shell::new()?;
        cmd!(sh, "cargo {args...}").run()?;

        color_eyre::Result::Ok(())
    }

    fn wasm_bindgen(context: &Context) -> color_eyre::Result<()> {
        let mut bindgen = Bindgen::new();

        let target_dir = context.target_dir.join(format!(
            "wasm32-unknown-unknown/{}/{}.wasm",
            if context.config.release {
                "release"
            } else {
                "debug"
            },
            context.cargo_package_name()?
        ));
        let bindgen = bindgen.input_path(target_dir).web(true).dot_eyre()?;

        let out_dir = context
            .current_dir
            .join(context.config.build.out_dir.clone());
        bindgen.generate(out_dir).dot_eyre()?;

        color_eyre::Result::Ok(())
    }
}

#[derive(Debug, Args)]
pub struct BuildSsrArgs {
    pub no_hydrate: bool,
}
