use crate::diarydb_entry::DiaryDBEntry;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::path::Path;

/// SQLite-backed database for diary entries.
pub struct DiaryDB {
    conn: Connection,
}

impl DiaryDB {
    /// Open or create a diary database at the given path and ensure the schema exists.
    ///
    /// # Arguments
    ///
    /// - `path`: Filesystem path to the SQLite database.
    ///
    /// # Returns
    ///
    /// A ready-to-use `DiaryDB` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or the schema creation fails.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS diary_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                japanese TEXT NOT NULL,
                romaji TEXT NOT NULL,
                meaning TEXT NOT NULL,
                notes TEXT,
                date_added INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(DiaryDB { conn })
    }

    /// Insert a new diary entry and return the created record.
    ///
    /// # Arguments
    ///
    /// - `japanese`: Japanese text of the entry.
    /// - `romaji`: Romaji transcription of the entry.
    /// - `meaning`: English meaning of the entry.
    /// - `notes`: Optional notes for the entry.
    ///
    /// # Returns
    ///
    /// The created `DiaryDBEntry` with the assigned id and timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn create_entry(
        &self,
        japanese: &str,
        romaji: &str,
        meaning: &str,
        notes: Option<&str>,
    ) -> Result<DiaryDBEntry> {
        let date_added = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        self.conn.execute(
            "INSERT INTO diary_entries (japanese, romaji, meaning, notes, date_added) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![japanese, romaji, meaning, notes, date_added],
        )?;

        let id = self.conn.last_insert_rowid() as i32;

        Ok(DiaryDBEntry {
            id,
            japanese: japanese.to_string(),
            romaji: romaji.to_string(),
            meaning: meaning.to_string(),
            notes: notes.map(|s| s.to_string()),
            date_added,
        })
    }

    /// Fetch a single entry by its id.
    ///
    /// # Arguments
    ///
    /// - `id`: Entry id to look up.
    ///
    /// # Returns
    ///
    /// `Ok(Some(entry))` if found, `Ok(None)` if no entry exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn get_entry(&self, id: i32) -> Result<Option<DiaryDBEntry>> {
        self.conn.query_row(
            "SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries WHERE id = ?1",
            params![id],
            |row| {
                Ok(DiaryDBEntry {
                    id: row.get(0)?,
                    japanese: row.get(1)?,
                    romaji: row.get(2)?,
                    meaning: row.get(3)?,
                    notes: row.get(4)?,
                    date_added: row.get(5)?,
                })
            },
        ).optional()
    }

    /// Fetch all entries, optionally limited by count.
    ///
    /// # Arguments
    ///
    /// - `limit`: Maximum number of entries to return.
    ///
    /// # Returns
    ///
    /// A vector of `DiaryDBEntry` records.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn get_all_entries(&self, limit: Option<usize>) -> Result<Vec<DiaryDBEntry>> {
        let query = match limit {
            Some(l) => format!("SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries LIMIT {}", l),
            None => "SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries".to_string(),
        };
        let mut stmt = self.conn.prepare(&query)?;
        let entry_iter = stmt.query_map([], |row| {
            Ok(DiaryDBEntry {
                id: row.get(0)?,
                japanese: row.get(1)?,
                romaji: row.get(2)?,
                meaning: row.get(3)?,
                notes: row.get(4)?,
                date_added: row.get(5)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }

    /// Update an existing entry by id.
    ///
    /// # Arguments
    ///
    /// - `entry`: Entry data to persist.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub fn update_entry(&self, entry: &DiaryDBEntry) -> Result<()> {
        self.conn.execute(
            "UPDATE diary_entries SET japanese = ?1, romaji = ?2, meaning = ?3, notes = ?4, date_added = ?5 WHERE id = ?6",
            params![entry.japanese, entry.romaji, entry.meaning, entry.notes, entry.date_added, entry.id],
        )?;
        Ok(())
    }

    /// Delete an entry by id.
    ///
    /// # Arguments
    ///
    /// - `id`: Entry id to delete.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the delete succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if the delete fails.
    pub fn delete_entry(&self, id: i32) -> Result<()> {
        self.conn
            .execute("DELETE FROM diary_entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Search entries by Japanese, romaji, or meaning using a LIKE query.
    ///
    /// # Arguments
    ///
    /// - `query`: Search term to match.
    ///
    /// # Returns
    ///
    /// A vector of matching entries ordered by newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn search_entries(&self, query: &str) -> Result<Vec<DiaryDBEntry>> {
        let like_query = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries 
             WHERE japanese LIKE ?1 OR romaji LIKE ?1 OR meaning LIKE ?1
             ORDER BY date_added DESC",
        )?;
        let entry_iter = stmt.query_map(params![like_query], |row| {
            Ok(DiaryDBEntry {
                id: row.get(0)?,
                japanese: row.get(1)?,
                romaji: row.get(2)?,
                meaning: row.get(3)?,
                notes: row.get(4)?,
                date_added: row.get(5)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }

    /// Run a VACUUM on the database to reclaim space.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the vacuum succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if the vacuum fails.
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }
}
