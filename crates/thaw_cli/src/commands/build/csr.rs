use crate::context::Context;
use color_eyre::eyre::eyre;
use std::{fs, io::Write};

pub fn build_index_html(context: &Context, serve: bool) -> color_eyre::Result<()> {
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
