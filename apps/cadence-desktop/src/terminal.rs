use std::{
    future::pending,
    io::{self, IsTerminal, Stdout, Write},
    pin::pin,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use cadence_core::{Library, LibraryRecord};
use crossterm::{
    cursor::{MoveTo, MoveToColumn, MoveUp},
    event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use tokio::{net::TcpListener, sync::watch, task::JoinHandle};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    display::{format_time, track_info},
    folder_picker::choose_music_folder,
    player::{PlayerMessage, StatusResponse},
    websocket,
};

pub(crate) async fn run(
    listener: TcpListener,
    player: mpsc::Sender<PlayerMessage>,
    library: Arc<Library>,
    address: &str,
) -> io::Result<()> {
    let mut view = TerminalView::open()?;
    let (tx, mut rx) = watch::channel((false, None));
    let mut server = tokio::spawn(websocket::serve(
        listener,
        player,
        Arc::clone(&library),
        move |connected, status| {
            tx.send_replace((connected, status.map(Track::from)));
        },
    ));
    let mut interrupt = pin!(tokio::signal::ctrl_c());
    let mut events = EventStream::new();
    let mut indexing: Option<JoinHandle<Result<usize, String>>> = None;
    let (progress_tx, mut progress_rx) = watch::channel(None);

    loop {
        let (connected, track) = rx.borrow_and_update().clone();
        view.draw(connected, track.as_ref(), address)?;

        tokio::select! {
            result = &mut interrupt => return result,
            result = &mut server => {
                result.map_err(io::Error::other)?;
                return Err(io::Error::other("Playback server stopped."));
            }
            result = rx.changed(), if matches!(view.screen, Screen::Playing) => {
                result.map_err(io::Error::other)?;
            }
            result = progress_rx.changed(), if indexing.is_some() => {
                result.map_err(io::Error::other)?;
                if let Some(scan) = &mut view.indexing {
                    scan.progress = *progress_rx.borrow_and_update();
                }
            }
            event = events.next() => {
                let Some(event) = event.transpose()? else { return Ok(()); };
                let Event::Key(key) = event else { continue; };
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                let Some(command) = view.read_command(key) else { continue; };
                let command = command.to_ascii_lowercase();
                let mut words = command.split_whitespace();
                match (words.next(), words.next(), words.next()) {
                    (None, _, _) => {}
                    (Some("help"), None, _) => view.show(Screen::Help),
                    (Some("list"), None, _) => {
                        view.notice = None;
                        view.show_libraries(&library);
                    }
                    (Some("play"), None, _) => view.show(Screen::Playing),
                    (Some("delete"), Some(number), None) => {
                        view.remove_library(&library, number, indexing.is_some());
                    }
                    (Some("delete"), _, _) => {
                        view.notice = Some("Use delete <index>. Type list to see the indexes.".into());
                    }
                    (Some("index"), None, _) if indexing.is_some() => {
                        view.notice = Some("A folder is already being indexed.".into());
                    }
                    (Some("index"), None, _) => {
                        view.notice = Some("Choose a folder in the desktop picker.".into());
                        view.draw(connected, track.as_ref(), address)?;
                        match choose_music_folder() {
                            Ok(Some(path)) => {
                                view.notice = None;
                                view.indexing = Some(IndexingStatus {
                                    path: path.to_string_lossy().into_owned(),
                                    progress: None,
                                });
                                progress_tx.send_replace(None);
                                let progress = progress_tx.clone();
                                let library = Arc::clone(&library);
                                indexing = Some(tokio::task::spawn_blocking(move || {
                                    let mut last_update = Instant::now();
                                    library.index_directory_with_progress(&path, |scanned, total| {
                                        let now = Instant::now();
                                        if scanned == 0
                                            || scanned == total
                                            || now.duration_since(last_update) >= Duration::from_millis(100)
                                        {
                                            progress.send_replace(Some((scanned, total)));
                                            last_update = now;
                                        }
                                    })
                                        .map_err(|error| format!("Could not index folder. {error:#}"))
                                }));
                            }
                            Ok(None) => view.notice = Some("Folder selection cancelled.".into()),
                            Err(error) => view.notice = Some(error),
                        }
                    }
                    _ => view.notice = Some("Unknown command. Type help to see available commands.".into()),
                }
            }
            result = async {
                match &mut indexing {
                    Some(job) => job.await,
                    None => pending().await,
                }
            } => {
                indexing = None;
                if let Some(scan) = &mut view.indexing {
                    scan.progress = *progress_rx.borrow_and_update();
                    view.draw(connected, track.as_ref(), address)?;
                }
                view.indexing = None;
                view.notice = Some(match result {
                    Ok(Ok(1)) => "Indexed 1 track.".into(),
                    Ok(Ok(count)) => format!("Indexed {count} tracks."),
                    Ok(Err(error)) => error,
                    Err(error) => format!("Indexing failed. {error}"),
                });
                if matches!(view.screen, Screen::Libraries(_)) {
                    view.show_libraries(&library);
                }
            }
        }
    }
}

#[derive(Clone)]
struct Track {
    label: String,
    position_ms: u64,
    duration_ms: u64,
    metadata: String,
}

impl From<&StatusResponse> for Track {
    fn from(status: &StatusResponse) -> Self {
        let (title, artist) = track_info(status);
        Self {
            label: format!("{title} - {artist}").replace(char::is_control, " "),
            position_ms: status.position_ms,
            duration_ms: status.duration_ms,
            metadata: audio_metadata(
                status.file_format.as_deref(),
                status.sample_rate,
                status.bitrate,
            ),
        }
    }
}

fn audio_metadata(
    file_format: Option<&str>,
    sample_rate: Option<u32>,
    bitrate: Option<u32>,
) -> String {
    let file_format = file_format.unwrap_or("--").replace(char::is_control, " ");
    let sample_rate = sample_rate.map_or_else(
        || "--".into(),
        |rate| {
            let khz = format!("{:.3}", f64::from(rate) / 1000.0);
            format!("{} kHz", khz.trim_end_matches('0').trim_end_matches('.'))
        },
    );
    let bitrate = bitrate.map_or_else(|| "--".into(), |rate| format!("{rate} kbps"));
    format!("{file_format}  |  {sample_rate}  |  {bitrate}")
}

enum Screen {
    Welcome,
    Help,
    Libraries(Vec<LibraryRecord>),
    Playing,
}

const WELCOME: &str = "Welcome to cadence! Type help to see available commands.";
const HELP: &[&str] = &[
    "list            List indexed library paths",
    "index           Choose and index a folder",
    "delete <index>  Remove a folder from the index",
    "play            Show Now Playing",
    "help            Show available commands",
    "",
    "Ctrl+C  Quit",
];

struct IndexingStatus {
    path: String,
    progress: Option<(usize, usize)>,
}

impl IndexingStatus {
    fn line(&self) -> String {
        let path = self.path.replace(char::is_control, " ");
        match self.progress {
            Some((scanned, total)) => {
                format!("Indexing library… ({scanned} out of {total} files) {path}")
            }
            None => format!("Indexing library… Counting files in {path}"),
        }
    }
}

struct TerminalView {
    stdout: Stdout,
    last_frame: String,
    screen_changed: bool,
    alternate_screen: bool,
    label: String,
    scrolling_since: Instant,
    screen: Screen,
    command: String,
    notice: Option<String>,
    indexing: Option<IndexingStatus>,
    command_progress_visible: bool,
}

impl TerminalView {
    fn open() -> io::Result<Self> {
        let stdout = io::stdout();
        if !stdout.is_terminal() || !io::stdin().is_terminal() {
            return Err(io::Error::other(
                "--terminal needs an interactive terminal.",
            ));
        }
        terminal::enable_raw_mode()?;
        Ok(Self {
            stdout,
            last_frame: String::new(),
            screen_changed: true,
            alternate_screen: false,
            label: String::new(),
            scrolling_since: Instant::now(),
            screen: Screen::Welcome,
            command: String::new(),
            notice: None,
            indexing: None,
            command_progress_visible: false,
        })
    }

    fn show(&mut self, screen: Screen) {
        self.screen = screen;
        self.screen_changed = true;
        self.notice = None;
    }

    fn show_libraries(&mut self, library: &Library) {
        match library.list_libraries() {
            Ok(mut folders) => {
                // Keep the order folders were added in this view. Display numbers come from their positions.
                folders.sort_unstable_by_key(|folder| folder.id);
                self.screen = Screen::Libraries(folders);
                self.screen_changed = true;
            }
            Err(error) => self.notice = Some(format!("Could not list libraries. {error:#}")),
        }
    }

    fn remove_library(&mut self, library: &Library, number: &str, indexing: bool) {
        let Some(index) = number.parse::<usize>().ok().and_then(|n| n.checked_sub(1)) else {
            self.notice = Some("Use delete <index> with a number from the list.".into());
            return;
        };
        if indexing {
            self.notice = Some("Wait for the current folder scan to finish.".into());
            return;
        }
        if !matches!(self.screen, Screen::Libraries(_)) {
            self.show_libraries(library);
        }
        let Screen::Libraries(folders) = &mut self.screen else {
            return;
        };
        let Some(folder) = folders.get(index) else {
            self.notice = Some(format!(
                "No folder at index {number}. Type list to see the indexes."
            ));
            return;
        };
        match library.delete_library(folder.id) {
            Ok(()) => {
                folders.remove(index);
                self.screen_changed = true;
                self.notice = Some("Folder removed from the index.".into());
            }
            Err(error) => self.notice = Some(format!("Could not remove folder. {error:#}")),
        }
    }

    fn read_command(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Enter => return Some(std::mem::take(&mut self.command)),
            KeyCode::Char(ch)
                if !ch.is_control() && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.command.push(ch);
            }
            KeyCode::Backspace => {
                self.command.pop();
            }
            _ => {}
        }
        None
    }

    fn draw(&mut self, connected: bool, track: Option<&Track>, address: &str) -> io::Result<()> {
        let playing = matches!(self.screen, Screen::Playing);
        if playing != self.alternate_screen {
            if playing {
                self.clear_command_area()?;
                execute!(self.stdout, EnterAlternateScreen)?;
            } else {
                execute!(self.stdout, LeaveAlternateScreen)?;
            }
            self.alternate_screen = playing;
            self.last_frame.clear();
        }
        let (columns, rows) = terminal::size()?;
        let width = usize::from(columns.saturating_sub(1));
        let prompt = format!("> {}", self.command);
        let prompt = clip_line(&prompt, width);
        let progress = self.indexing.as_ref().map(IndexingStatus::line);
        let progress = progress.as_deref().map(|line| clip_line(line, width));
        if !playing {
            return self.draw_commands(width, prompt, progress);
        }

        let label = track.map_or("", |track| track.label.as_str());
        if self.label != label {
            self.label = label.to_owned();
            self.scrolling_since = Instant::now();
        }

        let rows = usize::from(rows);
        let notice = self
            .notice
            .as_ref()
            .filter(|_| rows >= 3)
            .map(|notice| notice.replace(char::is_control, " "));
        let progress = progress.filter(|_| rows >= 3);
        let height = rows
            .saturating_sub(2 + usize::from(notice.is_some()) + usize::from(progress.is_some()));
        let mut visible = self.now_playing(connected, track, address, width);
        visible.truncate(height);
        if !visible.is_empty() {
            visible.push(String::new());
        }
        if let Some(notice) = notice {
            visible.push(clip_line(&notice, width).into());
        }
        if let Some(progress) = progress {
            visible.push(progress.into());
        }
        visible.push(prompt.into());
        let frame = visible.join("\r\n");

        if self.last_frame != frame {
            queue!(
                self.stdout,
                MoveTo(0, 0),
                Clear(ClearType::FromCursorDown),
                Print(&frame)
            )?;
            self.stdout.flush()?;
            self.last_frame = frame;
        }
        Ok(())
    }

    fn draw_commands(
        &mut self,
        width: usize,
        prompt: &str,
        progress: Option<&str>,
    ) -> io::Result<()> {
        let frame = progress.map_or_else(|| prompt.into(), |line| format!("{line}\r\n{prompt}"));
        if !self.screen_changed && self.notice.is_none() && self.last_frame == frame {
            return Ok(());
        }
        self.clear_command_area()?;
        if self.screen_changed {
            let lines = match &self.screen {
                Screen::Welcome => wrap_line(WELCOME, width),
                Screen::Help => HELP
                    .iter()
                    .flat_map(|line| wrap_line(line, width))
                    .collect(),
                Screen::Libraries(folders) if folders.is_empty() => {
                    wrap_line("No libraries indexed. Type index to add a folder.", width)
                }
                Screen::Libraries(folders) => library_table(folders, width),
                Screen::Playing => unreachable!(),
            };
            queue!(self.stdout, Print(lines.join("\r\n")), Print("\r\n\r\n"))?;
            self.screen_changed = false;
        }
        if let Some(notice) = self.notice.take() {
            let notice = notice.replace(char::is_control, " ");
            queue!(self.stdout, Print(notice), Print("\r\n"))?;
        }
        queue!(self.stdout, Print(&frame))?;
        self.stdout.flush()?;
        self.last_frame = frame;
        self.command_progress_visible = progress.is_some();
        Ok(())
    }

    fn clear_command_area(&mut self) -> io::Result<()> {
        let clear_type = if self.command_progress_visible {
            queue!(self.stdout, MoveUp(1))?;
            self.command_progress_visible = false;
            ClearType::FromCursorDown
        } else {
            ClearType::CurrentLine
        };
        queue!(self.stdout, MoveToColumn(0), Clear(clear_type))
    }

    fn now_playing(
        &self,
        connected: bool,
        track: Option<&Track>,
        address: &str,
        columns: usize,
    ) -> Vec<String> {
        let width = columns.saturating_sub(4).min(54);
        if width < 32 {
            return wrap_line("Widen the terminal to see Now Playing.", columns);
        }
        let (title, time, metadata) = if let Some(track) = track {
            let step = (self.scrolling_since.elapsed().as_millis() / 150) as usize;
            let time = format!(
                "{} / {}",
                format_time(track.position_ms, 2),
                format_time(track.duration_ms, 2)
            );
            (
                marquee(&track.label, width, step),
                time,
                track.metadata.as_str(),
            )
        } else {
            let idle = if connected {
                "Nothing playing"
            } else {
                "Waiting for phone"
            };
            (center_line(idle, width), String::new(), "")
        };
        let border = format!("+{}+", "-".repeat(width + 2));
        vec![
            border.clone(),
            format!("| {title} |"),
            format!("| {} |", center_line(&time, width)),
            format!("| {} |", center_line(metadata, width)),
            border,
            clip_line(address, columns).into(),
        ]
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        if self.alternate_screen {
            let _ = execute!(self.stdout, LeaveAlternateScreen);
        }
        let _ = self.clear_command_area();
        let _ = self.stdout.flush();
        let _ = terminal::disable_raw_mode();
    }
}

fn library_table(folders: &[LibraryRecord], width: usize) -> Vec<String> {
    let index_width = "Index".len().max(folders.len().to_string().len());
    let paths: Vec<_> = folders
        .iter()
        .map(|folder| folder.path.replace(char::is_control, " "))
        .collect();
    let path_width = paths
        .iter()
        .map(|path| path.width())
        .max()
        .unwrap_or(0)
        .max("Path".len())
        .min(width.saturating_sub(index_width + 7));
    if path_width < "Path".len() {
        return wrap_line("Widen the terminal to see library paths.", width);
    }
    let border = format!(
        "+{}+{}+",
        "-".repeat(path_width + 2),
        "-".repeat(index_width + 2)
    );
    let row = |path: &str, index: &str| {
        let padding = " ".repeat(path_width.saturating_sub(path.width()));
        format!("| {path}{padding} | {index:>index_width$} |")
    };
    let mut lines = vec![border.clone(), row("Path", "Index"), border.clone()];
    for (index, path) in paths.iter().enumerate() {
        let number = (index + 1).to_string();
        for (line, path) in wrap_line(path, path_width).iter().enumerate() {
            lines.push(row(path, if line == 0 { &number } else { "" }));
        }
    }
    lines.push(border);
    lines
}

fn clip_line(text: &str, mut width: usize) -> &str {
    for (index, glyph) in text.grapheme_indices(true) {
        let columns = glyph.width();
        if columns > width {
            return &text[..index];
        }
        width -= columns;
    }
    text
}

fn center_line(text: &str, width: usize) -> String {
    let text = clip_line(text, width);
    let padding = width.saturating_sub(text.width());
    format!(
        "{}{text}{}",
        " ".repeat(padding / 2),
        " ".repeat(padding - padding / 2)
    )
}

fn wrap_line(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for glyph in text.graphemes(true) {
        let columns = glyph.width();
        if columns > width {
            continue;
        }
        if used + columns > width {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push_str(glyph);
        used += columns;
    }
    lines.push(line);
    lines
}

fn marquee(label: &str, width: usize, step: usize) -> String {
    let gap = width.saturating_sub(label.width()).max(6);
    let mut cycle: Vec<&str> = label.graphemes(true).collect();
    cycle.extend(std::iter::repeat_n(" ", gap));
    let start = step % cycle.len();
    let mut remaining = width;
    let mut line = String::new();
    for glyph in cycle[start..].iter().chain(&cycle[..start]) {
        let columns = glyph.width();
        if columns > remaining {
            break;
        }
        line.push_str(glyph);
        remaining -= columns;
    }
    line.push_str(&" ".repeat(remaining));
    line
}

#[cfg(test)]
mod tests {
    use super::{audio_metadata, marquee};
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn marquee_wraps_without_splitting_wide_or_combining_characters() {
        assert_eq!(marquee("A", 4, 0), "A   ");
        assert_eq!(marquee("A", 4, 1), "    ");
        assert_eq!(marquee("A", 4, 4), "   A");
        assert_eq!(marquee("A", 4, 7), "A   ");
        assert_eq!(marquee("界e\u{301}", 3, 0), "界e\u{301}");
        assert_eq!(marquee("界e\u{301}", 3, 1), "e\u{301}  ");
        for step in 0..8 {
            assert_eq!(marquee("界e\u{301}", 3, step).width(), 3);
        }
    }

    #[test]
    fn audio_metadata_uses_khz_and_keeps_missing_values_in_place() {
        assert_eq!(
            audio_metadata(Some("FLAC"), Some(48_000), Some(957)),
            "FLAC  |  48 kHz  |  957 kbps"
        );
        assert_eq!(
            audio_metadata(Some("WAV"), Some(44_100), None),
            "WAV  |  44.1 kHz  |  --"
        );
        assert_eq!(audio_metadata(None, None, None), "--  |  --  |  --");
    }
}
