use std::path::PathBuf;

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub(crate) fn choose_music_folder() -> Result<Option<PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("Add Music Folder")
        .pick_folder())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn choose_music_folder() -> Result<Option<PathBuf>, String> {
    use std::{ffi::OsString, io::ErrorKind, os::unix::ffi::OsStringExt, process::Command};

    if ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .all(|name| std::env::var_os(name).is_none_or(|value| value.is_empty()))
    {
        return Err("A desktop session is required to open the folder picker.".into());
    }

    let output = Command::new("zenity")
        .args([
            "--file-selection",
            "--directory",
            "--title",
            "Add Music Folder",
        ])
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => "Install Zenity to open the folder picker.".into(),
            _ => format!("Could not open folder picker. {error}"),
        })?;

    match output.status.code() {
        Some(0) => {
            let mut path = output.stdout;
            if path.last() == Some(&b'\n') {
                path.pop();
            }
            let folder = PathBuf::from(OsString::from_vec(path));
            if folder.is_dir() {
                Ok(Some(folder))
            } else {
                Err("The selected folder is unavailable.".into())
            }
        }
        Some(1) => Ok(None),
        _ => Err(format!("Folder picker exited with {}.", output.status)),
    }
}
