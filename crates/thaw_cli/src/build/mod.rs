use crate::{
    cli,
    context::Context,
    utils::fs::{clear_dir, copy_dir_all},
};
use tokio::fs;

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

pub async fn copy_public_dir(context: &Context) -> color_eyre::Result<()> {
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
    copy_dir_all(public_dir, &context.out_dir).await?;
    Ok(())
}
