use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use crate::get_tagged_file;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "wav", "aac", "m4a", "opus", "wv", "ape",
];

const UNKNOWN_ARTIST: &str = "Unknown Artist";

pub struct Library {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackRecord {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryRecord {
    pub id: i64,
    pub path: String,
}

impl Library {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open DB at {:?}", db_path))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS libraries (
                id   INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL
            );
            CREATE TABLE IF NOT EXISTS artists (
                id   INTEGER PRIMARY KEY,
                name TEXT UNIQUE NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tracks (
                id          INTEGER PRIMARY KEY,
                library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
                path        TEXT UNIQUE NOT NULL,
                title       TEXT NOT NULL,
                artist_id   INTEGER NOT NULL REFERENCES artists(id),
                duration_ms INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5 (title, artist, filename);
        ")?;

        // Ensure the sentinel artist always exists.
        conn.execute(
            "INSERT OR IGNORE INTO artists (name) VALUES (?1)",
            params![UNKNOWN_ARTIST],
        )?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Walk `dir`, probe every audio file for tags + duration, and upsert into DB.
    /// Re-indexes from scratch on repeated calls (stale tracks are removed first).
    /// Returns the number of tracks inserted.
    ///
    /// `on_progress` is called with the running count of audio files probed so far.
    ///
    /// The slow part (walking the tree and reading tags from disk) runs without
    /// holding the DB lock, so searches stay responsive while a scan is in progress.
    /// All writes then happen in a single transaction: a failed re-index leaves the
    /// previous contents of the library untouched.
    pub fn index_directory(&self, dir: &Path, mut on_progress: impl FnMut(usize)) -> Result<usize> {
        let scanned = scan_directory(dir, &mut on_progress);

        let dir_str = dir.to_string_lossy().into_owned();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "INSERT OR IGNORE INTO libraries (path) VALUES (?1)",
            params![dir_str],
        )?;
        let library_id: i64 = tx.query_row(
            "SELECT id FROM libraries WHERE path = ?1",
            params![dir_str],
            |row| row.get(0),
        )?;

        // Clean up stale tracks for this library (FTS first, then tracks).
        tx.execute(
            "DELETE FROM tracks_fts WHERE rowid IN (SELECT id FROM tracks WHERE library_id = ?1)",
            params![library_id],
        )?;
        tx.execute("DELETE FROM tracks WHERE library_id = ?1", params![library_id])?;

        let mut count = 0usize;
        {
            let mut insert_artist = tx.prepare("INSERT OR IGNORE INTO artists (name) VALUES (?1)")?;
            let mut select_artist = tx.prepare("SELECT id FROM artists WHERE name = ?1")?;
            let mut insert_track = tx.prepare(
                "INSERT OR IGNORE INTO tracks (library_id, path, title, artist_id, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            let mut insert_fts = tx.prepare(
                "INSERT INTO tracks_fts(rowid, title, artist, filename) VALUES (?1, ?2, ?3, ?4)",
            )?;

            for track in &scanned {
                insert_artist.execute(params![track.artist])?;
                let artist_id: i64 = select_artist.query_row(params![track.artist], |r| r.get(0))?;

                let rows = insert_track.execute(params![
                    library_id,
                    track.path,
                    track.title,
                    artist_id,
                    track.duration_ms as i64,
                ])?;

                if rows > 0 {
                    let track_id = tx.last_insert_rowid();
                    insert_fts.execute(params![track_id, track.title, track.artist, track.filename])?;
                    count += 1;
                }
            }
        }

        tx.commit()?;
        Ok(count)
    }

    /// Full-text search over title and artist. Supports prefix matching.
    pub fn search(&self, query: &str) -> Result<Vec<TrackRecord>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        // Split on any non-alphanumeric character so apostrophes, dashes, etc.
        // don't produce tokens that FTS5 never indexed (it uses the same split rule).
        let fts_query = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| format!("{}*", w))
            .collect::<Vec<_>>()
            .join(" ");

        let conn = self.conn.lock().unwrap();
        let mut statement =
            conn.prepare(
            "SELECT t.id, t.path, t.title, a.name, t.duration_ms
                 FROM tracks_fts f
                 JOIN tracks t ON t.id = f.rowid
                 JOIN artists a ON a.id = t.artist_id
                 WHERE tracks_fts MATCH ?1
                 ORDER BY rank
                 LIMIT 50",
            )?;

        let tracks = statement
            .query_map(params![fts_query], |row| {
                Ok(TrackRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    duration_ms: row.get::<_, i64>(4)? as u64,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tracks)
    }

    pub fn all_track_paths(&self) -> Result<Vec<PathBuf>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM tracks")?;
        let paths = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(PathBuf::from)
            .collect();
        Ok(paths)
    }

    pub fn list_libraries(&self) -> Result<Vec<LibraryRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare("SELECT id, path FROM libraries ORDER BY path")?;
        let records = statement
            .query_map([], |row| {
                Ok(LibraryRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(records)
    }

    pub fn delete_library(&self, id: i64) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        // Clean up FTS entries before cascading delete removes the track rows.
        tx.execute(
            "DELETE FROM tracks_fts WHERE rowid IN (SELECT id FROM tracks WHERE library_id = ?1)",
            params![id],
        )?;
        tx.execute("DELETE FROM libraries WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }
}

/// A track read from disk, ready to be written to the DB.
struct ScannedTrack {
    path: String,
    filename: String,
    title: String,
    artist: String,
    duration_ms: u64,
}

/// Walk `dir` and probe every audio file. Touches only the filesystem, never the DB.
fn scan_directory(dir: &Path, on_progress: &mut impl FnMut(usize)) -> Vec<ScannedTrack> {
    let mut tracks = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let (title, artist, duration_ms) = probe_track(path);

        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Filename fallback for title.
        let title = title.unwrap_or_else(|| {
            if filename.is_empty() { "Unknown Track".to_string() } else { filename.clone() }
        });
        let artist = artist.unwrap_or_else(|| UNKNOWN_ARTIST.to_string());

        tracks.push(ScannedTrack {
            path: path.to_string_lossy().into_owned(),
            filename,
            title,
            artist,
            duration_ms,
        });
        on_progress(tracks.len());
    }

    tracks
}

fn probe_track(path: &Path) -> (Option<String>, Option<String>, u64) {
    use lofty::prelude::*;

    let Some(tagged) = 
        get_tagged_file(path)
    else {
        return (None, None, 0);
    };

    let duration_ms = tagged.properties().duration().as_millis() as u64;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let title = tag.and_then(|t| t.title().as_deref().map(String::from));
    let artist = tag.and_then(|t| t.artist().as_deref().map(String::from));

    (title, artist, duration_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cadence-library-{}-{}",
            name,
            std::process::id(),
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn index_reports_progress_reindexes_and_deletes() {
        let root = scratch_dir("index");
        let music = root.join("music");
        std::fs::create_dir_all(music.join("album")).unwrap();
        // Untaggable "audio" files still get indexed with a filename title.
        std::fs::write(music.join("first song.flac"), b"not really flac").unwrap();
        std::fs::write(music.join("album/second song.MP3"), b"not really mp3").unwrap();
        std::fs::write(music.join("cover.jpg"), b"not audio").unwrap();

        let library = Library::open(&root.join("cadence.db")).unwrap();

        let mut progress = Vec::new();
        let count = library.index_directory(&music, |n| progress.push(n)).unwrap();
        assert_eq!(count, 2);
        assert_eq!(progress, vec![1, 2]);

        let hits = library.search("second").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "second song");
        assert_eq!(hits[0].artist, UNKNOWN_ARTIST);

        // Re-indexing replaces stale rows instead of duplicating or dropping them.
        std::fs::remove_file(music.join("first song.flac")).unwrap();
        assert_eq!(library.index_directory(&music, |_| {}).unwrap(), 1);
        assert!(library.search("first").unwrap().is_empty());
        assert_eq!(library.search("second").unwrap().len(), 1);
        assert_eq!(library.list_libraries().unwrap().len(), 1);

        let id = library.list_libraries().unwrap()[0].id;
        library.delete_library(id).unwrap();
        assert!(library.list_libraries().unwrap().is_empty());
        assert!(library.search("second").unwrap().is_empty());
        assert!(library.all_track_paths().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
