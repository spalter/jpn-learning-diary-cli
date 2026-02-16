# jpn-learning-diary-cli

A command-line tool for managing a [Japanese learning diary](https://github.com/spalter/jpn-learning-diary).

## Usage

```bash
A command-line tool for managing a Japanese learning diary

Usage: jpn-learning-diary-cli [OPTIONS] <COMMAND>

Commands:
  add     Add a new diary entry interactively
  search  Search for diary entries matching a query
  delete  Delete a diary entry by ID
  update  Update a diary entry by ID interactively
  list    List all diary entries, optionally limited to a certain number
  dict    Search the dictionary for a query
  vacuum  Vacuum the database
  help    Print this message or the help of the given subcommand(s)

Options:
  -d, --db-path <DB_PATH>  Path to the SQLite database file [default: diary.db]
  -h, --help               Print help
  -V, --version            Print version
```

## Build

The build requires a dictionary file from the JPN Learning Diary app in `assets/jpn.db`. It's a converted version of the jmdict.

```bash
# Debug Builds
cargo build

# Release Builds
cargo build --release
```

## Credits

💜 The Kanji dictionary is based on [kanjiapi.dev](https://kanjiapi.dev/). Huge props to them for compiling so much data and making it available.<br>
💜 The Word dictionary is based on [JMdict](https://www.edrdg.org/jmdict/j_jmdict.html).
