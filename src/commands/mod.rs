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
    guard_home(root);
}

fn guard_home(root: &Path) {
    let canonical = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return,
    };

    // Block scanning the filesystem root
    if canonical.parent().is_none() {
        eprintln!("Error: scanning the filesystem root '/' is not allowed");
        process::exit(1);
    }

    // Block scanning the home directory itself
    if let Some(home) = dirs::home_dir() {
        if canonical == home {
            eprintln!(
                "Error: scanning your home directory '{}' is not allowed",
                home.display()
            );
            process::exit(1);
        }
    }
}
