mod imp;

use std::path::{Path, PathBuf};

pub use imp::ManagedApp;

pub fn managed_apps() -> Result<Vec<ManagedApp>, String> {
    imp::managed_apps()
}

pub fn remove_managed_app(desktop_path: &Path) -> Result<(), String> {
    imp::remove_managed_app(desktop_path)
}

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
