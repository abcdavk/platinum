mod imp;

use std::path::{Path, PathBuf};

pub fn copy_appimage(path: &Path, name: &str) -> Result<PathBuf, String> {
    imp::copy_appimage(path, name)
}

pub fn install(
    appimage_path: &Path,
    desktop_filename: &str,
    desktop_contents: &str,
    icon_name: &str,
    icon_contents: &[u8],
) -> Result<(), String> {
    imp::install(
        appimage_path,
        desktop_filename,
        desktop_contents,
        icon_name,
        icon_contents,
    )
}
