pub mod cache;
pub mod chat;
pub mod config;
pub mod info;
pub mod mcp;
pub mod scan;

use std::path::Path;
use std::process;

pub fn ensure_dir(root: &Path) {
    if !root.is_dir() {
        eprintln!("Error: '{}' is not a directory", root.display());
        process::exit(1);
    }
}
