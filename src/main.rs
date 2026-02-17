/// Main entry point for the jpn-learning-diary-cli application.
/// This application provides a command-line interface for managing a Japanese learning diary, allowing users to add, search, update, and delete diary entries, as well as search a Japanese dictionary database.
/// It uses SQLite for data storage and includes a build script to ensure the dictionary database is available at runtime.
/// The app is meant to be a CLI companion to the jpn-learning-diary desktop application.
mod diarydb;
mod dictionarydb;

use clap::Parser;
use clap::Subcommand;
use std::io::{self, BufRead, Write};

/// Command-line arguments structure for the application.
#[derive(Parser)]
#[command(name = "jpn-learning-diary-cli", version, about = "A command-line tool for managing a Japanese learning diary", long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "diary.db",
        help = "Path to the SQLite database file"
    )]
    pub db_path: String, // Path to the SQLite database file

    #[command(subcommand)]
    pub command: Commands, // The command to execute (add, search, delete, update, list)
}

/// Enumeration of supported commands for the CLI application.
#[derive(Subcommand, Clone)]
pub enum Commands {
    #[command(about = "Add a new diary entry interactively")]
    Add {},

    #[command(about = "Search for diary entries matching a query")]
    Search {
        query: String, // The search query to match against the Japanese text, romaji, or meaning
    },

    #[command(about = "Delete a diary entry by ID")]
    Delete {
        id: i32, // The unique identifier of the entry to delete
    },

    #[command(about = "Update a diary entry by ID interactively")]
    Update {
        id: i32, // The unique identifier of the entry to update
    },

    #[command(about = "List all diary entries, optionally limited to a certain number")]
    List {
        limit: Option<usize>, // Optional limit for the number of entries to list
    },

    #[command(about = "Search the dictionary for a query")]
    Dict {
        query: String, // The search query to match against the dictionary (kanji, reading, or meaning)
    },

    #[command(about = "Vacuum the database")]
    Vacuum {}, // Command to vacuum the database
}

/// Main function that parses command-line arguments, initializes the database connections, and dispatches to the appropriate command handler based on user input.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize the database connection
    let db = diarydb::DiaryDB::new(&args.db_path)?;
    let dict = dictionarydb::DictionaryDB::new()?;

    match args.command {
        Commands::Add {} => {
            handle_add(&db)?;
        }
        Commands::Search { query } => {
            handle_search(&db, query)?;
        }
        Commands::Delete { id } => {
            handle_delete(&db, id)?;
        }
        Commands::Update { id } => {
            handle_update(&db, id)?;
        }
        Commands::List { limit } => {
            handle_list(&db, limit)?;
        }
        Commands::Dict { query } => {
            handle_dict(&dict, query)?;
        }
        Commands::Vacuum {} => {
            handle_vacuum(db)?;
        }
    }

    Ok(())
}

/// Handle the add command.
///
/// # Arguments
///
/// - `db`: Database handle used for persistence.
/// - `japanese`: Japanese text of the entry.
/// - `romaji`: Romaji transcription of the entry.
/// - `meaning`: English meaning of the entry.
/// - `notes`: Optional notes for the entry.
///
/// # Returns
///
/// Returns `Ok(())` if the entry is created successfully, or an error otherwise.
fn handle_add(db: &diarydb::DiaryDB) -> Result<(), Box<dyn std::error::Error>> {
    println!("Adding entry...");
    let japanese = get_interactive_input("Japanese", "")?;
    let romaji = get_interactive_input("Romaji", "")?;
    let meaning = get_interactive_input("Meaning", "")?;
    let notes = get_interactive_input_optional("Notes", &None)?;
    db.create_entry(&japanese, &romaji, &meaning, notes.as_deref())?;
    Ok(())
}

/// Handle the search command.
///
/// # Arguments
///
/// - `db`: Database handle used for queries.
/// - `query`: Search term to match against entries.
///
/// # Returns
///
/// Returns `Ok(())` after printing matching entries, or an error otherwise.
fn handle_search(db: &diarydb::DiaryDB, query: String) -> Result<(), Box<dyn std::error::Error>> {
    let entries = db.search_entries(&query)?;
    if entries.is_empty() {
        println!("No entries found matching query: '{}'", query);
        return Ok(());
    }
    for entry in entries {
        let japanese = replace_brackets(&entry.japanese);
        println!("{}", japanese);
        println!("{}", entry.romaji);
        println!("{}", entry.meaning);
        if let Some(notes) = &entry.notes {
            println!("{}", notes);
        }
        println!();
    }

    Ok(())
}

/// Handle the delete command.
///
/// # Arguments
///
/// - `db`: Database handle used for deletion.
/// - `id`: Identifier of the entry to delete.
///
/// # Returns
///
/// Returns `Ok(())` if the entry is deleted successfully, or an error otherwise.
fn handle_delete(db: &diarydb::DiaryDB, id: i32) -> Result<(), Box<dyn std::error::Error>> {
    db.delete_entry(id)?;
    println!("Deleted entry with ID: {}", id);
    Ok(())
}

/// Handle the update command.
///
/// # Arguments
///
/// - `db`: Database handle used for updates.
/// - `id`: Identifier of the entry to update.
/// - `japanese`: Optional replacement Japanese text.
/// - `romaji`: Optional replacement romaji.
/// - `meaning`: Optional replacement meaning.
/// - `notes`: Optional replacement notes.
///
/// Returns `Ok(())` if the entry is updated successfully, or an error otherwise.
fn handle_update(db: &diarydb::DiaryDB, id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut entry = db
        .get_entry(id)?
        .ok_or_else(|| format!("Entry with ID {} not found", id))?;

    entry.japanese = get_interactive_input("Japanese", &entry.japanese)?;
    entry.romaji = get_interactive_input("Romaji", &entry.romaji)?;
    entry.meaning = get_interactive_input("Meaning", &entry.meaning)?;
    entry.notes = get_interactive_input_optional("Notes", &entry.notes)?;

    db.update_entry(&entry)?;
    println!("Updated entry with ID: {}", id);
    Ok(())
}

/// Handle the list command.
///
/// # Arguments
///
/// - `db`: Database handle used for queries.
/// - `limit`: Optional limit for the number of entries.
///
/// # Returns
///
/// Returns `Ok(())` after printing entries, or an error otherwise.
fn handle_list(
    db: &diarydb::DiaryDB,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = db.get_all_entries(limit)?;
    let length = entries.len();
    let digits = length.to_string().len();
    for entry in entries {
        let notes = entry
            .notes
            .as_deref()
            .unwrap_or("No notes")
            .replace("\n", " ");
        let japanese = replace_brackets(&entry.japanese);

        println!(
            "[{:width$}]: {} ({}), {}, {}",
            entry.id,
            japanese,
            entry.romaji,
            entry.meaning,
            notes,
            width = digits
        );
    }
    println!("Total entries: {}", length);
    Ok(())
}

/// Replace parentheses and brackets in a string with their full-width counterparts.
///
/// This is useful for ensuring that the text displays correctly in environments that may not handle half-width characters well.
///
/// # Arguments
///
/// - `s`: The input string to process.
///
/// # Returns
///
/// A new string with parentheses and brackets replaced by their full-width versions.
fn replace_brackets(s: &str) -> String {
    s.replace("(", "（")
        .replace(")", "）")
        .replace("[", "「")
        .replace("]", "」")
}

/// Get interactive input with a pre-filled default value.
///
/// Displays a prompt with the current value and allows the user to edit it or keep it.
///
/// # Arguments
///
/// - `prompt`: The field name to display.
/// - `current_value`: The current/default value.
///
/// # Returns
///
/// The user's input, or the current value if they just press Enter.
fn get_interactive_input(
    prompt: &str,
    current_value: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    print!("{} [{}]: ", prompt, current_value);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    let trimmed = input.trim();
    Ok(if trimmed.is_empty() {
        current_value.to_string()
    } else {
        trimmed.to_string()
    })
}

/// Get interactive input for an optional field.
///
/// # Arguments
///
/// - `prompt`: The field name to display.
/// - `current_value`: The current/default value (if any).
///
/// # Returns
///
/// The user's input as an Option, or the current value if they just press Enter.
fn get_interactive_input_optional(
    prompt: &str,
    current_value: &Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let default_display = current_value.as_deref().unwrap_or("(empty)");
    print!("{} [{}]: ", prompt, default_display);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    let trimmed = input.trim();
    if input.len() > 0 && trimmed.is_empty() {
        return Ok(None);
    }

    Ok(if trimmed.is_empty() {
        current_value.clone()
    } else {
        Some(trimmed.to_string())
    })
}

/// Handle the dictionary search command.
/// 
/// # Arguments
/// 
/// - `dict`: The dictionary database instance to perform the search on.
/// - `query`: The search term to match against kanji, readings, or meanings.
/// 
/// # Returns
/// 
/// Returns `Ok(())` after printing matching entries, or an error otherwise.
fn handle_dict(dict: &dictionarydb::DictionaryDB, query: String) -> Result<(), Box<dyn std::error::Error>> {
    let entries = dict.search(&query)?;
    if entries.is_empty() {
        println!("No dictionary entries found matching query: '{}'", query);
        return Ok(());
    }
    for entry in entries {
        println!("{} ({}) - {})", entry.kanji.join(", "), entry.readings.join(", "), entry.glosses.join("; "));
    }
    Ok(())
}

/// Handle the vacuum command to optimize the database file.
///
/// # Arguments
///
/// - `db`: The database instance to perform the vacuum operation on.
///
/// # Returns
/// 
/// Returns `Ok(())` after vacuuming the database, or an error otherwise.
fn handle_vacuum(db: diarydb::DiaryDB) -> Result<(), Box<dyn std::error::Error>> {
    db.vacuum()?;
    println!("Database vacuumed successfully.");
    Ok(())
}
