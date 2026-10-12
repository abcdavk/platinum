use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct ManagedApp {
    pub desktop_path: PathBuf,
    pub appimage_path: Option<PathBuf>,
    pub name: String,
    pub version: String,
    pub icon_path: Option<PathBuf>,
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| "HOME is not set to an absolute path".into())
}

pub(super) fn managed_apps() -> Result<Vec<ManagedApp>, String> {
    let applications_dir = home_dir()?.join(".local/share/applications");
    if !applications_dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&applications_dir)
        .map_err(|e| format!("Failed to read {}: {e}", applications_dir.display()))?;
    let mut apps = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read desktop entry: {e}"))?;
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
        {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if !desktop_property(&contents, "X-Platinum-Managed")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            continue;
        }

        let name = desktop_property(&contents, "Name")
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "AppImage".to_owned());
        let version = desktop_property(&contents, "X-AppImage-Version")
            .filter(|value| !value.is_empty())
            .unwrap_or("Unknown")
            .to_owned();
        let icon_path = desktop_property(&contents, "Icon").map(PathBuf::from);
        let appimage_path = desktop_property(&contents, "Exec").and_then(exec_program_path);

        apps.push(ManagedApp {
            desktop_path: path,
            appimage_path,
            name,
            version,
            icon_path,
        });
    }

    apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(apps)
}

pub(super) fn remove_managed_app(desktop_path: &Path) -> Result<(), String> {
    let home = home_dir()?;
    let desktop_dir = home.join(".local/share/applications");
    let applications_dir = home.join("Applications");
    if desktop_path.parent() != Some(desktop_dir.as_path())
        || !desktop_path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
    {
        return Err(
            "Refusing to remove a desktop file outside Platinum's applications directory".into(),
        );
    }

    let contents = fs::read_to_string(desktop_path)
        .map_err(|e| format!("Failed to read {}: {e}", desktop_path.display()))?;
    if !desktop_property(&contents, "X-Platinum-Managed")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return Err("Refusing to remove an application not managed by Platinum".into());
    }

    let exec = desktop_property(&contents, "Exec")
        .ok_or_else(|| "Managed desktop entry has no Exec path".to_owned())?;
    let appimage_path = exec_program_path(exec)
        .ok_or_else(|| "Managed desktop entry has an invalid Exec path".to_owned())?;
    if appimage_path.parent() != Some(applications_dir.as_path())
        || !appimage_path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("AppImage"))
    {
        return Err("Refusing to remove an AppImage outside ~/Applications".into());
    }

    match fs::remove_file(&appimage_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to remove AppImage {}: {error}",
                appimage_path.display()
            ));
        }
    }

    fs::remove_file(desktop_path).map_err(|e| {
        format!(
            "AppImage was removed, but failed to remove desktop entry {}: {e}",
            desktop_path.display()
        )
    })
}

fn exec_program_path(exec: &str) -> Option<PathBuf> {
    let exec = exec.trim_start();
    if let Some(quoted) = exec.strip_prefix('"') {
        let mut program = String::new();
        let mut escaped = false;
        for character in quoted.chars() {
            if escaped {
                program.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                return (!program.is_empty()).then(|| PathBuf::from(program));
            } else {
                program.push(character);
            }
        }
        None
    } else {
        let program = exec.split_whitespace().next()?;
        (!program.is_empty()).then(|| PathBuf::from(program))
    }
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
    let mut has_managed_marker = false;

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
            "X-Platinum-Managed" => {
                *line = "X-Platinum-Managed=true".to_owned();
                has_managed_marker = true;
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
    if !has_managed_marker {
        additions.push("X-Platinum-Managed=true".to_owned());
    }
    lines.splice(desktop_end..desktop_end, additions);

    let mut result = lines.join("\n");
    if contents.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn desktop_property<'a>(contents: &'a str, property: &str) -> Option<&'a str> {
    let mut in_desktop_entry = false;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == property
        {
            return Some(value.trim());
        }
    }
    None
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
    use std::path::PathBuf;

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
        assert!(updated.contains("X-Platinum-Managed=true"));
        assert!(updated.ends_with('\n'));
    }

    #[test]
    fn parses_quoted_exec_path_with_spaces_and_escapes() {
        assert_eq!(
            super::exec_program_path(r#""/home/user/Applications/My \"App\".AppImage" %U"#),
            Some(PathBuf::from("/home/user/Applications/My \"App\".AppImage"))
        );
    }

    #[test]
    fn escapes_exec_paths() {
        assert_eq!(
            quote_exec_path("/path/with $pecial/`chars`\\name"),
            "\"/path/with \\$pecial/\\`chars\\`\\\\name\""
        );
    }
}
