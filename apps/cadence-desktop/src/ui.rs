use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cadence_core::{library::IndexCancellation, Library, LibraryRecord};
use fltk::{
    app,
    button::Button,
    dialog::{FileDialog, FileDialogAction, FileDialogType},
    draw::{self, LineStyle},
    enums::{Align, Color, Font, FrameType},
    frame::Frame,
    group::{Group, Scroll, ScrollType},
    image::PngImage,
    prelude::*,
    window::Window,
};

const MARGIN: i32 = 30;
const LIST_TOP: i32 = 134;
const ROW_HEIGHT: i32 = 76;
const SCROLLBAR_WIDTH: i32 = 14;
const TEXT: Color = Color::from_hex(0x141518);
const MUTED: Color = Color::from_hex(0x595f69);
const BORDER: Color = Color::from_hex(0xc9cdd3);
const DIVIDER: Color = Color::from_hex(0xdadce1);
const BLUE: Color = Color::from_hex(0x1265e4);
const DISCONNECTED: &str = "No mobile device connected";
/// Shown above the folder list where library drives may need mounting.
#[cfg(target_os = "linux")]
const MOUNT_NOTE: Option<&str> =
    Some("Make sure the drives holding your library folders are mounted before playing.");
#[cfg(all(unix, not(target_os = "linux")))]
const MOUNT_NOTE: Option<&str> =
    Some("On Unix systems, external or network drives with library folders must be mounted.");
#[cfg(not(unix))]
const MOUNT_NOTE: Option<&str> = None;

#[derive(Clone)]
pub(crate) enum Message {
    AddFolder,
    RemoveFolder(i64),
    ScanProgress(usize),
    Finished(Result<(), String>),
    Playback(String),
}

enum Operation {
    Index(PathBuf),
    Remove(i64),
}

pub(crate) fn run(
    app: &app::App,
    library: Arc<Library>,
    address: &str,
    sender: app::Sender<Message>,
    receiver: app::Receiver<Message>,
) {
    let mut view = LibraryView::new(sender, address);
    view.refresh(&library, sender);
    view.window.show();

    let mut size = (view.window.w(), view.window.h());
    let mut busy = false;
    let mut indexing: Option<IndexDialog> = None;
    while app.wait() {
        let current_size = (view.window.w(), view.window.h());
        if current_size != size {
            size = current_size;
            view.layout();
        }
        while let Some(message) = receiver.recv() {
            let operation = match message {
                Message::Playback(label) => {
                    view.playback.set_label(&label);
                    view.playback.set_tooltip(&label);
                    view.playback.redraw();
                    continue;
                }
                Message::AddFolder if !busy => {
                    let mut picker = FileDialog::new(FileDialogType::BrowseDir);
                    picker.set_title("Add Music Folder");
                    match picker.try_show() {
                        Ok(FileDialogAction::Success) => Operation::Index(picker.filename()),
                        Ok(FileDialogAction::Cancelled) => continue,
                        Err(error) => {
                            view.set_status(
                                &format!("Could not open folder picker. {error}"),
                                true,
                            );
                            continue;
                        }
                    }
                }
                Message::RemoveFolder(id) if !busy => Operation::Remove(id),
                Message::ScanProgress(count) => {
                    if let Some(dialog) = &mut indexing {
                        dialog.update_progress(count);
                    }
                    continue;
                }
                Message::Finished(result) => {
                    if let Some(mut dialog) = indexing.take() {
                        dialog.window.hide();
                        app::delete_widget(dialog.window);
                        if let Err(error) = result {
                            fltk::dialog::alert_default(&error);
                        }
                    } else {
                        match result {
                            Ok(()) => view.set_status("", false),
                            Err(error) => view.set_status(&error, true),
                        }
                    }
                    busy = false;
                    view.set_busy(false);
                    view.refresh(&library, sender);
                    continue;
                }
                Message::AddFolder | Message::RemoveFolder(_) => continue,
            };

            busy = true;
            view.set_busy(true);
            let library = Arc::clone(&library);
            match operation {
                Operation::Index(path) => {
                    view.set_status("", false);
                    let cancellation = Arc::new(IndexCancellation::default());
                    indexing = Some(IndexDialog::show(
                        &view.window,
                        &path,
                        Arc::clone(&cancellation),
                    ));
                    std::thread::spawn(move || {
                        let mut last_update = Instant::now();
                        let result = library
                            .index_directory_cancellable(&path, &cancellation, |scanned| {
                                if last_update.elapsed() >= Duration::from_millis(100) {
                                    sender.send(Message::ScanProgress(scanned));
                                    last_update = Instant::now();
                                }
                            })
                            .map(|_| ())
                            .map_err(|error| format!("Indexing failed. {error:#}"));
                        sender.send(Message::Finished(result));
                    });
                }
                Operation::Remove(id) => {
                    view.set_status("Removing folder from the library…", false);
                    std::thread::spawn(move || {
                        let result = library
                            .delete_library(id)
                            .map_err(|error| format!("Could not remove folder. {error:#}"));
                        sender.send(Message::Finished(result));
                    });
                }
            }
        }
    }
}

struct IndexDialog {
    window: Window,
    progress: Frame,
    cancel: Button,
}

impl IndexDialog {
    fn show(parent: &Window, path: &Path, cancellation: Arc<IndexCancellation>) -> Self {
        let (width, height) = (440, 146);
        let x = parent.x() + (parent.w() - width) / 2;
        let y = parent.y() + (parent.h() - height) / 2;
        Group::set_current(None::<&Group>);
        let mut window = Window::new(x, y, width, height, "Indexing Music Folder");
        window.make_modal(true);

        let mut folder = Frame::new(20, 18, 400, 24, None);
        folder.set_label(&path.to_string_lossy());
        folder.set_tooltip(&path.to_string_lossy());
        folder.set_align(Align::Left | Align::Inside | Align::Clip);
        let mut progress = Frame::new(20, 49, 400, 24, "Scanning for tracks…");
        progress.set_align(Align::Left | Align::Inside);

        let mut cancel = Button::new(320, 93, 100, 33, "Cancel");
        let mut progress_label = progress.clone();
        cancel.set_callback(move |button| {
            if button.active() {
                let label = if cancellation.cancel() {
                    "Cancelling…"
                } else {
                    "Finishing…"
                };
                progress_label.set_label(label);
                button.deactivate();
            }
        });
        let mut cancel_button = cancel.clone();
        window.set_callback(move |_| cancel_button.do_callback());
        window.end();
        window.show();

        Self {
            window,
            progress,
            cancel,
        }
    }

    fn update_progress(&mut self, count: usize) {
        if self.cancel.active() {
            self.progress
                .set_label(&format!("{} scanned", track_count(count)));
        }
    }
}

pub(crate) fn playback_label(
    connected: bool,
    status: Option<&crate::player::StatusResponse>,
) -> String {
    if !connected {
        return DISCONNECTED.into();
    }
    let Some(status) = status else {
        return "Stopped".into();
    };
    let name = status
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
    let state = if status.is_playing() {
        "Now playing"
    } else {
        "Paused"
    };
    format!("{state}: {name} by {artist}").replace(['\n', '\r'], " ")
}

fn track_count(count: usize) -> String {
    let noun = if count == 1 { "track" } else { "tracks" };
    format!("{count} {noun}")
}

struct LibraryView {
    window: Window,
    heading: Frame,
    add: Button,
    mount_note: Option<Frame>,
    outline: Frame,
    scroll: Scroll,
    rows: Vec<Group>,
    status: Frame,
    playback: Frame,
    address: Frame,
}

impl LibraryView {
    fn new(sender: app::Sender<Message>, address: &str) -> Self {
        let mut window = Window::new(0, 0, 770, 530, "cadence desktop").center_screen();
        window.set_color(Color::White);
        window.size_range(540, 320, 0, 0);
        if let Ok(icon) = PngImage::from_data(include_bytes!("../assets/cadence.png")) {
            window.set_icon(Some(icon));
        }

        let mut heading = Frame::default().with_label("Music Library Folders");
        heading.set_align(Align::Left | Align::Inside);
        heading.set_label_font(Font::HelveticaBold);
        heading.set_label_size(34);
        heading.set_label_color(TEXT);

        let mut add = Button::default().with_label("Add Folder");
        add.set_frame(FrameType::NoBox);
        add.emit(sender, Message::AddFolder);
        add.draw(|button| {
            let color = if !button.active_r() {
                Color::from_hex(0x7293c3)
            } else if button.value() {
                Color::from_hex(0x0d51b9)
            } else {
                BLUE
            };
            draw::set_draw_color(color);
            draw::draw_rounded_rectf(button.x(), button.y(), button.w(), button.h(), 11);
            draw::set_draw_color(Color::White);
            draw::set_font(Font::Helvetica, 22);
            draw::draw_text2(
                "Add Folder",
                button.x(),
                button.y(),
                button.w(),
                button.h(),
                Align::Center,
            );
            draw_focus(button, Color::White);
        });

        let mount_note = MOUNT_NOTE.map(|note| {
            let mut frame = Frame::default().with_label(note);
            frame.set_align(Align::Left | Align::Inside | Align::Clip);
            frame.set_label_size(14);
            frame.set_label_color(MUTED);
            frame.set_tooltip(note);
            frame
        });

        let mut scroll = Scroll::default().with_type(ScrollType::Vertical);
        scroll.set_color(Color::White);
        scroll.set_scrollbar_size(SCROLLBAR_WIDTH);
        scroll.end();

        let mut outline = Frame::default();
        outline.draw(|frame| {
            draw::set_draw_color(BORDER);
            draw::draw_rounded_rect(frame.x(), frame.y(), frame.w(), frame.h(), 12);
        });
        let mut status = Frame::default();
        status.set_align(Align::Left | Align::Inside | Align::Wrap | Align::Clip);
        status.set_label_size(15);
        let mut playback = Frame::default().with_label(DISCONNECTED);
        playback.set_align(Align::Left | Align::Inside | Align::Clip);
        playback.set_label_size(16);
        playback.set_label_color(TEXT);
        let mut address = Frame::default().with_label(address);
        address.set_align(Align::Left | Align::Inside | Align::Clip);
        address.set_label_size(15);
        address.set_label_color(MUTED);
        window.end();
        window.resizable(&window);

        let mut view = Self {
            window,
            heading,
            add,
            mount_note,
            outline,
            scroll,
            rows: Vec::new(),
            status,
            playback,
            address,
        };
        view.layout();
        view
    }

    fn refresh(&mut self, library: &Library, sender: app::Sender<Message>) {
        match library.list_libraries() {
            Ok(folders) => self.show_folders(&folders, sender),
            Err(error) => self.set_status(&format!("Could not load folders. {error:#}"), true),
        }
    }

    fn show_folders(&mut self, folders: &[LibraryRecord], sender: app::Sender<Message>) {
        let offset = self.scroll.yposition();
        self.scroll.scroll_to(0, 0);
        for row in self.rows.drain(..) {
            self.scroll.remove(&row);
            app::delete_widget(row);
        }
        self.scroll.begin();
        let width = self.window.w() - MARGIN * 2 - 2;
        let x = MARGIN + 1;
        let y = LIST_TOP + 1;
        if folders.is_empty() {
            let row = Group::new(x, y, width, ROW_HEIGHT, None);
            let mut empty = Frame::new(
                x + 26,
                y,
                width - 52,
                ROW_HEIGHT,
                "No library folders added yet",
            );
            empty.set_align(Align::Left | Align::Inside);
            empty.set_label_size(19);
            empty.set_label_color(MUTED);
            row.end();
            row.resizable(&empty);
            self.rows.push(row);
        } else {
            for (index, folder) in folders.iter().enumerate() {
                let top = y + index as i32 * ROW_HEIGHT;
                self.rows.push(folder_row(
                    x,
                    top,
                    width,
                    folder,
                    index + 1 < folders.len(),
                    sender,
                ));
            }
        }
        self.scroll.end();
        self.layout();
        let max_offset = (self.rows.len() as i32 * ROW_HEIGHT - self.scroll.h()).max(0);
        self.scroll.scroll_to(0, offset.min(max_offset));
    }

    fn layout(&mut self) {
        let (width, height) = (self.window.w(), self.window.h());
        let inner_width = width - 2 * MARGIN;
        let button_width = if width >= 650 { 181 } else { 142 };
        self.heading
            .set_label_size(if width >= 650 { 34 } else { 27 });
        self.heading
            .resize(MARGIN + 7, 43, inner_width - button_width - 19, 52);
        self.add
            .resize(width - MARGIN - button_width - 2, 43, button_width, 52);
        if let Some(note) = &mut self.mount_note {
            // Sits in the gap between the heading row and the folder list.
            note.resize(MARGIN + 7, LIST_TOP - 32, inner_width - 14, 26);
        }

        let rows_height = self.rows.len().max(1) as i32 * ROW_HEIGHT;
        let card_height = (rows_height + 2).min(height - LIST_TOP - 115);
        let scrollable = rows_height > card_height - 2;
        let row_width = inner_width - 2 - if scrollable { SCROLLBAR_WIDTH } else { 0 };
        self.outline
            .resize(MARGIN, LIST_TOP, inner_width, card_height);
        self.scroll
            .resize(MARGIN + 1, LIST_TOP + 1, inner_width - 2, card_height - 2);
        let offset = self
            .scroll
            .yposition()
            .min((rows_height - self.scroll.h()).max(0));
        self.scroll.scroll_to(0, offset);
        for (index, row) in self.rows.iter_mut().enumerate() {
            let y = LIST_TOP + 1 + index as i32 * ROW_HEIGHT - offset;
            row.resize(MARGIN + 1, y, row_width, ROW_HEIGHT);
        }
        let below_list = LIST_TOP + card_height;
        self.playback
            .resize(MARGIN + 6, below_list + 8, inner_width - 12, 24);
        self.status
            .resize(MARGIN + 6, below_list + 32, inner_width - 12, 49);
        self.address
            .resize(MARGIN + 6, height - 33, inner_width - 12, 22);
        self.window.redraw();
    }

    fn set_busy(&mut self, busy: bool) {
        if busy {
            self.add.deactivate();
            self.scroll.deactivate();
        } else {
            self.add.activate();
            self.scroll.activate();
        }
    }

    fn set_status(&mut self, message: &str, error: bool) {
        self.status.set_label(message);
        self.status.set_tooltip(message);
        self.status.set_label_color(if error {
            Color::from_hex(0xa41a2a)
        } else {
            MUTED
        });
        self.status.redraw();
    }
}

fn folder_row(
    x: i32,
    y: i32,
    width: i32,
    folder: &LibraryRecord,
    divider: bool,
    sender: app::Sender<Message>,
) -> Group {
    let row = Group::new(x, y, width, ROW_HEIGHT, None);
    let mut icon = Frame::new(x + 28, y, 32, ROW_HEIGHT, None);
    icon.draw(|icon| draw_folder(icon.x(), icon.y() + (icon.h() - 24) / 2));

    let mut path = Frame::new(x + 92, y, width - 168, ROW_HEIGHT, folder.path.as_str());
    path.set_align(Align::Left | Align::Inside | Align::Clip);
    path.set_label_size(23);
    path.set_label_color(TEXT);
    path.set_tooltip(&folder.path);

    let mut remove = Button::new(x + width - 64, y + 16, 43, 44, None);
    remove.set_frame(FrameType::NoBox);
    remove.set_tooltip("Remove folder from the library");
    remove.emit(sender, Message::RemoveFolder(folder.id));
    remove.draw(|button| {
        if button.value() {
            draw::set_draw_color(Color::from_hex(0xe9edf3));
            draw::draw_rounded_rectf(button.x(), button.y(), button.w(), button.h(), 8);
        }
        draw_trash(
            button.x() + 9,
            button.y() + 9,
            if button.active_r() { TEXT } else { MUTED },
        );
        draw_focus(button, BLUE);
    });

    if divider {
        let mut line = Frame::new(x + 1, y + ROW_HEIGHT - 1, width - 2, 1, None);
        line.set_frame(FrameType::FlatBox);
        line.set_color(DIVIDER);
    }
    row.end();
    row.resizable(&path);
    row
}

fn draw_focus(button: &Button, color: Color) {
    if button.has_focus() {
        draw::set_draw_color(color);
        draw::draw_rounded_rect(
            button.x() + 3,
            button.y() + 3,
            button.w() - 6,
            button.h() - 6,
            7,
        );
    }
}

fn draw_folder(x: i32, y: i32) {
    draw::set_draw_color(TEXT);
    draw::set_line_style(LineStyle::Solid, 2);
    draw::draw_rounded_rect(x, y + 5, 30, 19, 3);
    draw::draw_line(x, y + 9, x, y + 1);
    draw::draw_line(x, y + 1, x + 11, y + 1);
    draw::draw_line(x + 11, y + 1, x + 16, y + 5);
    draw::set_line_style(LineStyle::Solid, 0);
}

fn draw_trash(x: i32, y: i32, color: Color) {
    draw::set_draw_color(color);
    draw::set_line_style(LineStyle::Solid, 2);
    draw::draw_rounded_rect(x + 4, y + 6, 17, 20, 3);
    draw::draw_line(x + 1, y + 6, x + 24, y + 6);
    draw::draw_rounded_rect(x + 8, y + 1, 9, 5, 2);
    draw::draw_line(x + 10, y + 11, x + 10, y + 21);
    draw::draw_line(x + 15, y + 11, x + 15, y + 21);
    draw::set_line_style(LineStyle::Solid, 0);
}

#[cfg(test)]
mod tests {
    use super::playback_label;
    use crate::player::StatusResponse;
    use cadence_core::{OutputState, PlayerMode};

    #[test]
    fn playback_line_reflects_connection_and_player_state() {
        let mut status = StatusResponse {
            path: "track.wav".into(),
            duration_ms: 1000,
            position_ms: 0,
            paused: false,
            output_state: OutputState::Ready,
            playback_error: None,
            title: Some("Title".into()),
            artist: Some("Artist".into()),
            file_format: None,
            sample_rate: None,
            bitrate: None,
            mode: PlayerMode::Default,
        };
        assert_eq!(
            playback_label(false, Some(&status)),
            "No mobile device connected"
        );
        assert_eq!(playback_label(true, None), "Stopped");
        assert_eq!(
            playback_label(true, Some(&status)),
            "Now playing: Title by Artist"
        );
        status.paused = true;
        assert_eq!(
            playback_label(true, Some(&status)),
            "Paused: Title by Artist"
        );
        status.title = None;
        status.artist = None;
        assert_eq!(
            playback_label(true, Some(&status)),
            "Paused: track by Unknown Artist"
        );
    }
}
