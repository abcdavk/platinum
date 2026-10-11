mod imp;

use std::path::Path;

pub fn handle_file(path: &Path) -> Result<(), String> {
    imp::handle_file(path)
}

pub(crate) fn is_appimage(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("AppImage"))
}
