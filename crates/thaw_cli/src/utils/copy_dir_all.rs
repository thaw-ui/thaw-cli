use std::fs;
use std::io;
use std::path::Path;

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = dst.as_ref().join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_all(entry_path, target_path)?;
        } else {
            fs::copy(entry_path, target_path)?;
        }
    }
    Ok(())
}
