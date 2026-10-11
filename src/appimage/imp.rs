use std::path::Path;

pub(super) fn handle_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("The selected path is not a file".into());
    }

    let is_appimage = path
        .extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x.eq_ignore_ascii_case("AppImage"));

    if !is_appimage {
        return Err("The selected path is not an AppImage".into());
    }

    println!("AppImage detected: {}", path.display());

    Ok(())
}
