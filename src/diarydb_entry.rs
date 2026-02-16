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
