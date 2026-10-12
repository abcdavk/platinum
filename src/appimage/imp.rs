use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
    process::Command,
};

use backhand::{FilesystemReader, InnerNode};

use crate::dotdesktop;

pub(super) fn handle_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("The selected path is not a file".into());
    }

    if !super::is_appimage(path) {
        return Err("The selected path is not an AppImage".into());
    }

    println!("AppImage detected: {}", path.display());

    // Find the SquashFS offset using Gear Lever's script.
    let script_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/get_appimage_offset.sh");

    let output = Command::new(&script_path)
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to run offset script: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to calculate AppImage offset: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let offset = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Invalid AppImage offset: {e}"))?;

    println!("SquashFS offset: {offset}");

    // Verify the SquashFS magic bytes.
    let mut file = File::open(path).map_err(|e| format!("Failed to open AppImage: {e}"))?;

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Failed to seek to SquashFS: {e}"))?;

    let mut magic = [0u8; 4];

    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read SquashFS magic: {e}"))?;

    println!("Magic bytes: {magic:?}");

    if magic != *b"hsqs" {
        return Err(format!(
            "Invalid SquashFS magic at offset {offset}: {magic:?}"
        ));
    }

    // Reopen the file from the beginning.
    let file =
        BufReader::new(File::open(path).map_err(|e| format!("Failed to open AppImage: {e}"))?);

    let read_fs = FilesystemReader::from_reader_with_offset(file, offset)
        .map_err(|e| format!("Failed to read SquashFS: {e}"))?;

    println!("SquashFS loaded successfully.");

    let mut desktop_entries = Vec::new();

    for node in read_fs.files() {
        if node
            .fullpath
            .extension()
            .is_some_and(|ext| ext == "desktop")
        {
            if let InnerNode::File(file) = &node.inner {
                let mut reader = read_fs.file(file).reader();
                let mut contents = String::new();

                reader
                    .read_to_string(&mut contents)
                    .map_err(|e| format!("Failed to read desktop file: {e}"))?;

                println!("Desktop file: {}", node.fullpath.display());
                println!("{contents}");
                desktop_entries.push((node.fullpath.to_path_buf(), contents));
            }
        }
    }

    if desktop_entries.is_empty() {
        return Err("No .desktop file found in AppImage".into());
    }

    desktop_entries.sort_by(|(left_path, _), (right_path, _)| {
        left_path
            .components()
            .count()
            .cmp(&right_path.components().count())
            .then_with(|| left_path.cmp(right_path))
    });
    let primary_entry = &desktop_entries[0];
    let source_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "AppImage filename is not valid UTF-8".to_owned())?;
    let app_name = appimage_name(&primary_entry.1, source_stem);
    let appimage_copy = dotdesktop::copy_appimage(path, &app_name)?;
    let (desktop_contents, icon_name, icon_stem) = if desktop_entries.len() == 1 {
        let (desktop_path, desktop_contents) = primary_entry;
        (
            desktop_contents.clone(),
            desktop_property(desktop_contents, "Icon")
                .unwrap_or(&app_name)
                .to_owned(),
            desktop_path.file_stem().map(std::ffi::OsStr::to_owned),
        )
    } else {
        let (desktop_path, desktop_contents) = primary_entry;
        (
            single_desktop_entry(desktop_contents),
            desktop_property(desktop_contents, "Icon")
                .unwrap_or(&app_name)
                .to_owned(),
            desktop_path.file_stem().map(std::ffi::OsStr::to_owned),
        )
    };
    let desktop_filename = format!("{app_name}.desktop");
    let (_, icon_contents) = find_icon(&read_fs, &icon_name, icon_stem.as_deref())?
        .ok_or_else(|| format!("No application icon found for {desktop_filename}"))?;
    dotdesktop::install(
        &appimage_copy,
        &desktop_filename,
        &desktop_contents,
        &app_name,
        &icon_contents,
    )?;

    Ok(())
}

fn appimage_name(contents: &str, fallback: &str) -> String {
    let original = desktop_property(contents, "Name")
        .unwrap_or(fallback)
        .trim();
    let name = original
        .strip_prefix('"')
        .and_then(|name| name.strip_suffix('"'))
        .unwrap_or(original);
    let name: String = name
        .chars()
        .map(|character| {
            if character == '/' || character == '\\' || character.is_control() {
                '_'
            } else {
                character
            }
        })
        .collect();
    let name = name.trim().trim_matches('.');
    if name.is_empty() {
        fallback.to_owned()
    } else {
        name.to_owned()
    }
}

fn single_desktop_entry(contents: &str) -> String {
    let name = desktop_property(contents, "Name").unwrap_or("AppImage");
    let appimage_name = desktop_property(contents, "X-AppImage-Name").unwrap_or(name);
    let arch = desktop_property(contents, "X-AppImage-Arch").unwrap_or("");
    let version = desktop_property(contents, "X-AppImage-Version").unwrap_or("");
    let categories = desktop_property(contents, "Categories").unwrap_or("");
    let terminal = desktop_property(contents, "Terminal").unwrap_or("false");
    let comment = desktop_property(contents, "Comment").unwrap_or("");

    format!(
        "[Desktop Entry]\n\
         X-AppImage-Arch={arch}\n\
         X-AppImage-Version={version}\n\
         X-AppImage-Name={appimage_name}\n\
         Name={name}\n\
         Exec=\n\
         Icon=\n\
         Type=Application\n\
         Terminal={terminal}\n\
         Categories={categories}\n\
         Comment={comment}\n"
    )
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

fn find_icon(
    filesystem: &FilesystemReader<'_>,
    icon_name: &str,
    desktop_stem: Option<&std::ffi::OsStr>,
) -> Result<Option<(std::path::PathBuf, Vec<u8>)>, String> {
    let icon_stem = Path::new(icon_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(icon_name);
    let desktop_stem = desktop_stem.and_then(|stem| stem.to_str()).unwrap_or("");
    let mut best: Option<(u8, usize, std::path::PathBuf)> = None;

    for node in filesystem.files() {
        let Some(extension) = node.fullpath.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !["png", "svg", "xpm", "jpg", "jpeg"]
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
        {
            continue;
        }
        let Some(stem) = node.fullpath.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };

        let in_known_icon_dir = node.fullpath.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some("apps" | "pixmaps" | "icons")
            )
        });
        let rank = if stem.eq_ignore_ascii_case(icon_stem) {
            0
        } else if stem.eq_ignore_ascii_case(desktop_stem) {
            1
        } else if in_known_icon_dir {
            2
        } else {
            continue;
        };

        let candidate = (
            rank,
            node.fullpath.as_os_str().len(),
            node.fullpath.to_path_buf(),
        );
        if best.as_ref().is_none_or(|best| candidate < *best) {
            best = Some(candidate);
        }
    }

    let Some((_, _, icon_path)) = best else {
        return Ok(None);
    };
    for node in filesystem.files() {
        if node.fullpath == icon_path {
            if let InnerNode::File(file) = &node.inner {
                let mut reader = filesystem.file(file).reader();
                let mut contents = Vec::new();
                reader
                    .read_to_end(&mut contents)
                    .map_err(|e| format!("Failed to read icon {}: {e}", icon_path.display()))?;
                return Ok(Some((icon_path, contents)));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{appimage_name, single_desktop_entry};

    #[test]
    fn uses_desktop_name_for_appimage_and_icon_filename() {
        assert_eq!(
            appimage_name("[Desktop Entry]\nName=\"Krita\"\n", "original"),
            "Krita"
        );
    }

    #[test]
    fn sanitizes_name_as_a_single_filename_component() {
        assert_eq!(
            appimage_name("[Desktop Entry]\nName=Bad/Name\n", "original"),
            "Bad_Name"
        );
        assert_eq!(
            appimage_name("[Desktop Entry]\nName=...\n", "original"),
            "original"
        );
    }

    #[test]
    fn retains_version_when_selecting_primary_desktop_entry() {
        let reduced = single_desktop_entry(
            "[Desktop Entry]\nName=Example\nX-AppImage-Version=1.2\nX-AppImage-Publisher=Example Inc.\nX-AppImage-Creator=Creator\n",
        );
        assert!(reduced.contains("X-AppImage-Version=1.2"));
        assert!(!reduced.contains("X-AppImage-Publisher"));
        assert!(!reduced.contains("X-AppImage-Creator"));
    }
}
