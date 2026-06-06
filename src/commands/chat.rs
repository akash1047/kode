use std::path::Path;
use std::process;

use crate::chat;
use crate::project::banner;

pub async fn run(root: &Path, message: Option<String>, model: Option<String>) {
    super::ensure_dir(root);
    if let Some(msg) = message {
        if let Err(e) = chat::run_one_shot(root, &msg, model).await {
            eprintln!("chat error: {:#}", e);
            process::exit(1);
        }
    } else {
        banner::print(root);
        if let Err(e) = chat::run_repl(root, model).await {
            eprintln!("chat error: {:#}", e);
            process::exit(1);
        }
    }
}
