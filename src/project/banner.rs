use std::path::Path;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const BRIGHT_CYAN: &str = "\x1b[96m";
const MAGENTA: &str = "\x1b[35m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

pub fn print(root: &Path) {
    let info = super::info(root);
    let name = info.name.as_deref().unwrap_or_else(|| {
        root.file_name().and_then(|s| s.to_str()).unwrap_or("?")
    });
    let authors = if info.authors.is_empty() {
        format!("{}-{}", DIM, RESET)
    } else {
        format!("{}{}{}", MAGENTA, info.authors.join(", "), RESET)
    };
    let remote = info
        .remote_url
        .as_deref()
        .map(|s| format!("{}{}{}", YELLOW, s, RESET))
        .unwrap_or_else(|| format!("{}-{}", DIM, RESET));
    let web = info
        .web_url
        .as_deref()
        .map(|s| format!("{}{}{}", YELLOW, s, RESET))
        .unwrap_or_else(|| format!("{}-{}", DIM, RESET));

    let yes_no = |b: bool| {
        if b {
            format!("{}yes{}", GREEN, RESET)
        } else {
            format!("{}no{}", DIM, RESET)
        }
    };

    println!("{}{}┌─ project{}", BOLD, BRIGHT_CYAN, RESET);
    println!("{}│{} {}name{}    {}{}{}", BRIGHT_CYAN, RESET, GRAY, RESET, BOLD, name, RESET);
    println!("{}│{} {}authors{} {}", BRIGHT_CYAN, RESET, GRAY, RESET, authors);
    println!("{}│{} {}path{}    {}{}{}", BRIGHT_CYAN, RESET, GRAY, RESET, CYAN, info.abs_path.display(), RESET);
    println!("{}│{} {}git{}     {}", BRIGHT_CYAN, RESET, GRAY, RESET, yes_no(info.git_init));
    println!("{}│{} {}cloud{}   {}", BRIGHT_CYAN, RESET, GRAY, RESET, yes_no(info.remote_url.is_some()));
    println!("{}│{} {}remote{}  {}", BRIGHT_CYAN, RESET, GRAY, RESET, remote);
    println!("{}│{} {}url{}     {}", BRIGHT_CYAN, RESET, GRAY, RESET, web);
    println!("{}{}└─{}", BOLD, BRIGHT_CYAN, RESET);
    println!();
}
