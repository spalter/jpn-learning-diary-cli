use rusqlite::{Connection, OptionalExtension, Result, params, functions::{FunctionFlags, Context}};
use std::path::Path;
use regex::Regex;
use std::sync::OnceLock;

fn strip_markdown_furigana(text: &str) -> String {
    // Compile regex only once
    static RE_MARKDOWN: OnceLock<Regex> = OnceLock::new();
    static RE_JP: OnceLock<Regex> = OnceLock::new();
    static RE_PUNCT: OnceLock<Regex> = OnceLock::new();
    
    // [Kanji](Reading) -> Kanji
    let re_md = RE_MARKDOWN.get_or_init(|| Regex::new(r"\[([^]]+)\]\([^)]+\)").unwrap());
    // [Kanji]（Reading） -> Kanji (Japanese parens)
    let re_jp = RE_JP.get_or_init(|| Regex::new(r"\[([^]]+)\]（[^）]+）").unwrap());
    // Strip simple punctuation brackets: 「 」 （ ）
    let re_punct = RE_PUNCT.get_or_init(|| Regex::new(r"[「」（）]").unwrap());

    // 1. Strip complex furigana first
    let temp1 = re_md.replace_all(text, "$1");
    let temp2 = re_jp.replace_all(&temp1, "$1");
    
    // 2. Then remove just the target punctuation characters
    let result = re_punct.replace_all(&temp2, "");
    result.to_string()
}

fn normalize_umlauts(text: &str) -> String {
    text.replace('ü', "ue")
        .replace('ö', "oe")
        .replace('ä', "ae")
        .replace('ß', "ss")
}

/// Data model for a single diary entry row.
#[derive(Debug, Clone)]
pub struct DiaryDBEntry {
    /// Unique identifier for the entry.
    pub id: i32,
    /// Japanese text of the diary entry.
    pub japanese: String,
    /// Romaji transcription of the Japanese text.
    pub romaji: String,
    /// English meaning of the diary entry.
    pub meaning: String,
    /// Optional notes about the diary entry.
    pub notes: Option<String>,
    /// Timestamp when the diary entry was added (milliseconds since epoch).
    pub date_added: i64,
}

/// SQLite-backed database for diary entries.
pub struct DiaryDB {
    conn: Connection,
}

impl DiaryDB {
    fn open_and_setup<P: AsRef<Path>>(path: P) -> Result<Connection> {
        let conn = Connection::open(path)?;

        conn.create_scalar_function(
            "STRIP_FURIGANA",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            move |ctx: &Context| {
                let text = ctx.get::<String>(0)?;
                Ok(strip_markdown_furigana(&text))
            },
        )?;

        conn.create_scalar_function(
            "NORMALIZE_UMLAUTS",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            move |ctx: &Context| {
                let text = ctx.get::<String>(0)?;
                Ok(normalize_umlauts(&text))
            },
        )?;

        Ok(conn)
    }

    /// Open an existing diary database.
    /// Fails if the table 'diary_entries' does not exist.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Self::open_and_setup(path)?;

        // Ensure the table exists, but do not create it automatically.
        // This fails if the table is missing, preventing operations on invalid DBs.
        let table_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='diary_entries')",
            [],
            |row| row.get(0),
        )?;

        if !table_exists {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some("Database table 'diary_entries' does not exist.".to_string()),
            ));
        }

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
            Some(l) => format!(
                "SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries LIMIT {}",
                l
            ),
            None => "SELECT id, japanese, romaji, meaning, notes, date_added FROM diary_entries"
                .to_string(),
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
                  WHERE LOWER(NORMALIZE_UMLAUTS(STRIP_FURIGANA(japanese))) LIKE ?1
                  OR LOWER(NORMALIZE_UMLAUTS(japanese)) LIKE ?1
                  OR LOWER(NORMALIZE_UMLAUTS(romaji)) LIKE ?1
                  OR LOWER(NORMALIZE_UMLAUTS(meaning)) LIKE ?1
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
