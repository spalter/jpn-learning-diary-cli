/// Build script to copy the jpn.db file to the target directory for use at runtime.
use std::{fs, path::PathBuf};

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let profile = std::env::var("PROFILE").unwrap();

    let src = PathBuf::from(&manifest_dir).join("assets/jpn.db");
    let dest = PathBuf::from(&manifest_dir)
        .join("target")
        .join(&profile)
        .join("jpn.db");

    fs::copy(&src, &dest).expect("Failed to copy jpn.db");

    println!(
        "cargo:info=Copied jpn.db to build directory at: {:?}",
        dest
    );
}
