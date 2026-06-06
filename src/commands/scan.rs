use std::fs;
use std::path::Path;

use crate::fs as project_fs;
use crate::project::banner;

pub async fn run_default(root: &Path) {
    super::ensure_dir(root);
    banner::print(root);
    run(root);
}

pub fn run(root: &Path) {
    for result in project_fs::walker(root) {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);

        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                match fs::metadata(path) {
                    Ok(m) => {
                        let size = m.len();
                        if size == 0 {
                            println!("{} 0 [empty]", rel.display());
                        } else {
                            println!("{} {} [binary]", rel.display(), size);
                        }
                    }
                    Err(e) => eprintln!("Error reading {}: {}", rel.display(), e),
                }
                continue;
            }
        };

        let chars = content.chars().count();
        if chars == 0 {
            println!("{} 0 [empty]", rel.display());
        } else {
            println!("{} {}", rel.display(), chars);
        }
    }
}
