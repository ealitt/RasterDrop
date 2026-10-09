use directories::UserDirs;
use std::io;
use std::path::{Path, PathBuf};

pub fn downloads_directory() -> Option<PathBuf> {
    UserDirs::new().and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
}

pub fn reveal_path(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()?
            .wait()?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let target = path.parent().unwrap_or(path);
        open::that(target).map_err(io::Error::other)
    }
}

pub fn open_directory(path: &Path) -> io::Result<()> {
    open::that(path).map_err(io::Error::other)
}
