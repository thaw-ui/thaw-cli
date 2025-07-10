use std::path::Path;
use tokio::{fs, io};

pub async fn clear_dir(dir: impl AsRef<Path>) -> io::Result<()> {
    if fs::try_exists(&dir).await? {
        fs::remove_dir_all(&dir).await?;
    }
    fs::create_dir_all(dir).await?;
    Ok(())
}

pub async fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    if !fs::try_exists(&src).await? {
        return Ok(());
    }
    fs::create_dir_all(&dst).await?;

    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        let target_path = dst.as_ref().join(entry.file_name());
        if entry_path.is_dir() {
            Box::pin(copy_dir_all(entry_path, target_path)).await?;
        } else {
            fs::copy(entry_path, target_path).await?;
        }
    }
    Ok(())
}
