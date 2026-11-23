use std::path::Path;

use crate::player::StatusResponse;

pub(crate) fn track_info(status: &StatusResponse) -> (&str, &str) {
    let title = status
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .or_else(|| {
            Path::new(&status.path)
                .file_stem()
                .and_then(|stem| stem.to_str())
        })
        .unwrap_or("Unknown Track");
    let artist = status
        .artist
        .as_deref()
        .filter(|artist| !artist.trim().is_empty())
        .unwrap_or("Unknown Artist");
    (title, artist)
}

pub(crate) fn format_time(ms: u64, minute_width: usize) -> String {
    let seconds = ms / 1000;
    format!("{:0minute_width$}:{:02}", seconds / 60, seconds % 60)
}
