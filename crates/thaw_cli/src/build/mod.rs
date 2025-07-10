use crate::{
    context::Context,
    utils::fs::{clear_dir, copy_dir_all},
};

#[inline]
pub async fn clear_out_dir(context: &Context) -> color_eyre::Result<()> {
    clear_dir(&context.out_dir).await?;
    Ok(())
}

pub async fn copy_public_dir(context: &Context) -> color_eyre::Result<()> {
    if context.config.public_dir.is_empty() {
        return Ok(());
    }

    let public_dir = context.current_dir.join(context.config.public_dir.clone());
    copy_dir_all(public_dir, &context.out_dir).await?;
    Ok(())
}
