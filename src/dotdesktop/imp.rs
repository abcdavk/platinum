use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| "HOME is not set to an absolute path".into())
}

pub(super) fn copy_appimage(path: &Path, name: &str) -> Result<PathBuf, String> {
    let destination = home_dir()?
        .join("Applications")
        .join(format!("{name}.AppImage"));

    fs::create_dir_all(
        destination
            .parent()
            .ok_or_else(|| "Invalid AppImage destination path".to_owned())?,
    )
    .map_err(|e| format!("Failed to create ~/Applications: {e}"))?;

    let source = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve AppImage path: {e}"))?;
    let same_file = destination
        .canonicalize()
        .is_ok_and(|existing| existing == source);

    if !same_file {
        fs::copy(path, &destination)
            .map_err(|e| format!("Failed to copy AppImage to {}: {e}", destination.display()))?;
    }

    Ok(destination)
}

pub(super) fn install(
    appimage_path: &Path,
    desktop_filename: &str,
    desktop_contents: &str,
    icon_name: &str,
    icon_contents: &[u8],
) -> Result<(), String> {
    let home = home_dir()?;
    let applications_dir = home.join(".local/share/applications");
    let icons_dir = home.join("Applications/.icons");
    fs::create_dir_all(&applications_dir)
        .map_err(|e| format!("Failed to create {}: {e}", applications_dir.display()))?;
    fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Failed to create {}: {e}", icons_dir.display()))?;

    let appimage_path = appimage_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve copied AppImage path: {e}"))?;
    let icon_name = Path::new(icon_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Icon name is not valid UTF-8".to_owned())?;
    let icon_path = icons_dir.join(icon_name);
    let desktop_path = applications_dir.join(desktop_filename);
    let updated_desktop = update_desktop(desktop_contents, &appimage_path, &icon_path)?;

    fs::write(&icon_path, icon_contents)
        .map_err(|e| format!("Failed to write icon to {}: {e}", icon_path.display()))?;
    fs::write(&desktop_path, updated_desktop).map_err(|e| {
        format!(
            "Failed to write desktop file to {}: {e}",
            desktop_path.display()
        )
    })
}

fn update_desktop(
    contents: &str,
    appimage_path: &Path,
    icon_path: &Path,
) -> Result<String, String> {
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let desktop_start = lines
        .iter()
        .position(|line| line.trim() == "[Desktop Entry]")
        .ok_or_else(|| "The AppImage .desktop file has no [Desktop Entry] section".to_owned())?;
    let desktop_end = lines
        .iter()
        .enumerate()
        .skip(desktop_start + 1)
        .find(|(_, line)| line.trim().starts_with('['))
        .map_or(lines.len(), |(index, _)| index);

    let exec = quote_exec_path(&appimage_path.to_string_lossy());
    let icon = icon_path.to_string_lossy().into_owned();
    let mut has_exec = false;
    let mut has_icon = false;

    for line in &mut lines[desktop_start + 1..desktop_end] {
        if line.trim_start().starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key.trim() {
            "Exec" => {
                *line = format!("Exec={}{}", exec, exec_arguments(value));
                has_exec = true;
            }
            "TryExec" => {
                *line = format!("TryExec={exec}");
            }
            "Icon" => {
                *line = format!("Icon={icon}");
                has_icon = true;
            }
            _ => {}
        }
    }

    let mut additions = Vec::new();
    if !has_exec {
        additions.push(format!("Exec={exec}"));
    }
    if !has_icon {
        additions.push(format!("Icon={icon}"));
    }
    lines.splice(desktop_end..desktop_end, additions);

    let mut result = lines.join("\n");
    if contents.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn quote_exec_path(path: &str) -> String {
    let escaped = path
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{escaped}\"")
}

fn exec_arguments(exec: &str) -> &str {
    let mut in_quotes = false;
    let mut escaped = false;

    for (index, character) in exec.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            in_quotes = !in_quotes;
        } else if character.is_whitespace() && !in_quotes {
            return &exec[index..];
        }
    }

    ""
}

#[cfg(test)]
mod tests {
    use super::{quote_exec_path, update_desktop};
    use std::path::Path;

    #[test]
    fn updates_exec_icon_and_try_exec_in_desktop_entry() {
        let contents =
            "[Desktop Entry]\nName=Example\nExec=AppRun %U\nTryExec=AppRun\nIcon=example\n";
        let updated = update_desktop(
            contents,
            Path::new("/home/user/Applications/Example App.AppImage"),
            Path::new("/home/user/Applications/.icons/Example App"),
        )
        .unwrap();

        assert!(updated.contains("Exec=\"/home/user/Applications/Example App.AppImage\" %U"));
        assert!(updated.contains("TryExec=\"/home/user/Applications/Example App.AppImage\""));
        assert!(updated.contains("Icon=/home/user/Applications/.icons/Example App"));
        assert!(updated.ends_with('\n'));
    }

    #[test]
    fn escapes_exec_paths() {
        assert_eq!(
            quote_exec_path("/path/with $pecial/`chars`\\name"),
            "\"/path/with \\$pecial/\\`chars\\`\\\\name\""
        );
    }
}
