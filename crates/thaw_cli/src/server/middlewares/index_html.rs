use crate::{
    constants::CLIENT_PUBLIC_PATH,
    context::Context,
    plugins::html::{
        HtmlTagDescriptor, HtmlTagInjectTo, IndexHtmlTransformResult, apply_html_transform,
    },
};
use std::collections::HashMap;
use tokio::{fs, io::AsyncWriteExt};

async fn dev_html_hook(context: &Context) -> color_eyre::Result<IndexHtmlTransformResult> {
    let out_dir = &context.out_dir;
    let path = out_dir.join(format!(".{CLIENT_PUBLIC_PATH}.js"));
    if !fs::try_exists(&path).await? {
        fs::create_dir_all(&path.parent().unwrap()).await?;
        let mut file = fs::File::create_new(path).await?;
        file.write_all(include_str!("../../client/client.js").as_bytes())
            .await?;
    }

    Ok(IndexHtmlTransformResult {
        tags: vec![HtmlTagDescriptor {
            tag: "script",
            attrs: HashMap::from([
                ("type", "module".to_string()),
                ("src", format!("{CLIENT_PUBLIC_PATH}.js")),
            ]),
            inject_to: HtmlTagInjectTo::HeadPrepend,
        }],
    })
}

pub async fn dev_html_transform_fn(context: &Context, html: &mut String) -> color_eyre::Result<()> {
    let res = dev_html_hook(context).await?;
    apply_html_transform(html, res.tags);
    Ok(())
}
