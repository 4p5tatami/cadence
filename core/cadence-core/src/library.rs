use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
            CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5 (title, artist);
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
    pub fn index_directory(&self, dir: &Path) -> Result<usize> {
        let dir_str = dir.to_string_lossy().into_owned();
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO libraries (path) VALUES (?1)",
            params![dir_str],
        )?;
        let library_id: i64 = conn.query_row(
            "SELECT id FROM libraries WHERE path = ?1",
            params![dir_str],
            |row| row.get(0),
        )?;

        // Clean up stale tracks for this library (FTS first, then tracks).
        let stale_ids: Vec<i64> = {
            let mut statement = conn.prepare("SELECT id FROM tracks WHERE library_id = ?1")?;
            let ids: Vec<i64> =
            statement.query_map(params![library_id], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            ids
        };
        for id in &stale_ids {
            conn.execute("DELETE FROM tracks_fts WHERE rowid = ?1", params![id])?;
        }
        conn.execute("DELETE FROM tracks WHERE library_id = ?1", params![library_id])?;

        let mut count = 0usize;

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

            // Filename fallback for title.
            let title = title.unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Track")
                    .to_string()
            });
            let artist = artist.unwrap_or_else(|| UNKNOWN_ARTIST.to_string());

            // Upsert artist, get its id.
            conn.execute(
                "INSERT OR IGNORE INTO artists (name) VALUES (?1)",
                params![artist],
            )?;
            let artist_id: i64 = conn.query_row(
                "SELECT id FROM artists WHERE name = ?1",
                params![artist],
                |r| r.get(0),
            )?;

            let rows = conn.execute(
                "INSERT OR IGNORE INTO tracks (library_id, path, title, artist_id, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    library_id,
                    path.to_string_lossy().as_ref(),
                    title,
                    artist_id,
                    duration_ms as i64,
                ],
            )?;

            if rows > 0 {
                let track_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO tracks_fts(rowid, title, artist) VALUES (?1, ?2, ?3)",
                    params![track_id, title, artist],
                )?;
                count += 1;
            }
        }

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
        let conn = self.conn.lock().unwrap();
        // Clean up FTS entries before cascading delete removes the track rows.
        let stale_ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM tracks WHERE library_id = ?1")?;
            let ids: Vec<i64> = stmt.query_map(params![id], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            ids
        };
        for track_id in &stale_ids {
            conn.execute("DELETE FROM tracks_fts WHERE rowid = ?1", params![track_id])?;
        }
        conn.execute("DELETE FROM libraries WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn probe_track(path: &Path) -> (Option<String>, Option<String>, u64) {
    use lofty::prelude::*;

    let Some(tagged) = lofty::probe::Probe::open(path).ok()
        .and_then(|p| p.guess_file_type().ok())
        .and_then(|p| p.read().ok())
    else {
        return (None, None, 0);
    };

    let duration_ms = tagged.properties().duration().as_millis() as u64;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let title = tag.and_then(|t| t.title().as_deref().map(String::from));
    let artist = tag.and_then(|t| t.artist().as_deref().map(String::from));

    (title, artist, duration_ms)
}
