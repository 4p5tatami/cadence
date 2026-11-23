use std::path::PathBuf;

use fltk::dialog::{FileDialog, FileDialogAction, FileDialogType};

pub(crate) fn choose_music_folder() -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "linux")]
    if ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .all(|name| std::env::var_os(name).is_none_or(|value| value.is_empty()))
    {
        return Err("A desktop session is required to open the folder picker.".into());
    }

    if !fltk::app::is_initialized() {
        fltk::app::init_all();
    }
    let mut picker = FileDialog::new(FileDialogType::BrowseDir);
    picker.set_title("Add Music Folder");
    match picker.try_show() {
        Ok(FileDialogAction::Success) => Ok(Some(picker.filename())),
        Ok(FileDialogAction::Cancelled) => Ok(None),
        Err(error) => Err(format!("Could not open folder picker. {error}")),
    }
}
