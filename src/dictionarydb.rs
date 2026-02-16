use rusqlite::{params, Connection, Result};

/// Search result entry for JMdict records.
#[derive(Debug, Clone)]
pub struct JmdictEntry {
    /// Kanji spellings (keb values).
    pub kanji: Vec<String>,
    /// Kana readings (reb values).
    pub readings: Vec<String>,
    /// English glosses.
    pub glosses: Vec<String>,
}

/// SQLite-backed dictionary database for JMdict data.
pub struct DictionaryDB {
    conn: Connection,
}

impl DictionaryDB {
    /// Create a new DictionaryDB instance by loading the database next to the executable.
    ///
    /// # Returns
    ///
    /// A ready-to-use `DictionaryDB` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the executable path is unavailable or the database cannot be opened.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::current_exe()?.parent().unwrap().join("jpn.db");
        let conn = Connection::open(path)?;
        Ok(DictionaryDB { conn })
    }

    /// Search for entries by kanji, reading, or meaning
    ///
    /// # Arguments
    ///
    /// - `query`: The search term (can be kanji, reading, or English meaning)
    ///
    /// # Returns
    ///
    /// A vector of matching `JmdictEntry` entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn search(&self, query: &str) -> Result<Vec<JmdictEntry>> {
        let like_query = format!("%{}%", query);
        
        // Search across kanji, readings, and glosses
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT je.id 
             FROM jmdict_entries je
             LEFT JOIN jmdict_kanji jk ON je.id = jk.entry_id
             LEFT JOIN jmdict_readings jr ON je.id = jr.entry_id
             LEFT JOIN jmdict_senses js ON je.id = js.entry_id
             LEFT JOIN jmdict_glosses jg ON js.id = jg.sense_id
             WHERE jk.keb LIKE ?1 
                OR jr.reb LIKE ?1 
                OR jg.gloss LIKE ?1
             LIMIT 50"
        )?;
        
        let entry_ids: Vec<i32> = stmt
            .query_map(params![like_query], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        let mut entries = Vec::new();
        for entry_id in entry_ids {
            if let Some(entry) = self.get_entry_by_id(entry_id)? {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    /// Get a single entry by its JMdict entry id with related data.
    ///
    /// # Arguments
    ///
    /// - `entry_id`: JMdict entry id to fetch.
    ///
    /// # Returns
    ///
    /// `Ok(Some(entry))` if found, `Ok(None)` if no entry exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the queries fail.
    fn get_entry_by_id(&self, entry_id: i32) -> Result<Option<JmdictEntry>> {
        // Check if entry exists
        let entry_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM jmdict_entries WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        )?;
        
        if !entry_exists {
            return Ok(None);
        }
        
        // Get all kanji variants
        let mut stmt = self.conn.prepare(
            "SELECT keb FROM jmdict_kanji WHERE entry_id = ?1"
        )?;
        let kanji: Vec<String> = stmt
            .query_map(params![entry_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        // Get all readings
        let mut stmt = self.conn.prepare(
            "SELECT reb FROM jmdict_readings WHERE entry_id = ?1"
        )?;
        let readings: Vec<String> = stmt
            .query_map(params![entry_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        // Get all glosses (English meanings)
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT jg.gloss 
             FROM jmdict_glosses jg
             JOIN jmdict_senses js ON jg.sense_id = js.id
             WHERE js.entry_id = ?1"
        )?;
        let glosses: Vec<String> = stmt
            .query_map(params![entry_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(Some(JmdictEntry {
            kanji,
            readings,
            glosses,
        }))
    }
}
