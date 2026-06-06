pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";

pub const CYAN: &str = "\x1b[36m";
pub const BRIGHT_CYAN: &str = "\x1b[96m";
pub const MAGENTA: &str = "\x1b[35m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const GRAY: &str = "\x1b[90m";

pub fn cache_summary(
    scanned: usize,
    added: usize,
    changed: usize,
    _unchanged: usize,
    deleted: usize,
    manifests: usize,
    run_configs: usize,
    readmes: usize,
    symbols_files: usize,
    symbols_total: usize,
    dirs_hashed: usize,
    summaries_cached: usize,
) {
    let symbols_label = format!("{symbols_total} in {symbols_files} files");
    let parts: Vec<(&str, String, bool)> = vec![
        ("scanned", scanned.to_string(), true),
        ("new", added.to_string(), added > 0),
        ("changed", changed.to_string(), changed > 0),
        ("removed", deleted.to_string(), deleted > 0),
        ("manifests", manifests.to_string(), manifests > 0),
        ("run-configs", run_configs.to_string(), run_configs > 0),
        ("readmes", readmes.to_string(), readmes > 0),
        ("symbols", symbols_label, symbols_total > 0),
        ("dirs", dirs_hashed.to_string(), dirs_hashed > 0),
        ("summaries", summaries_cached.to_string(), summaries_cached > 0),
    ];
    let joined = parts
        .iter()
        .filter(|(_, _, show)| *show)
        .map(|(k, v, _)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("{DIM}{GRAY}└─ cache: {joined}{RESET}");
}

pub fn banner(model: &str) {
    println!(
        "{BOLD}{BRIGHT_CYAN}┌─ kode chat ─{RESET}{DIM} model: {RESET}{MAGENTA}{model}{RESET}"
    );
    println!(
        "{DIM}{ITALIC}└─ /help for commands · ctrl-c cancels · ctrl-d exits{RESET}"
    );
}

pub fn assistant_prefix() -> String {
    format!("{BOLD}{GREEN}◆{RESET} ")
}

pub fn thinking_prefix() -> String {
    format!("{DIM}{ITALIC}💭 {RESET}")
}

pub fn tool_completed(
    name: &str,
    args: &serde_json::Value,
    response: &serde_json::Value,
    elapsed: std::time::Duration,
) {
    let args_str = format_args(args);
    let (sigil, sigil_color, summary, summary_color) = summarize(name, response);
    let elapsed_str = format_elapsed(elapsed);

    let mut line = format!("  {sigil_color}{sigil}{RESET} {CYAN}{name}{RESET}");
    if !args_str.is_empty() {
        line.push_str(&format!(" {DIM}{args_str}{RESET}"));
    }
    line.push_str(&format!(
        "  {summary_color}{summary}{RESET} {DIM}{GRAY}· {elapsed_str}{RESET}"
    ));
    eprintln!("{line}");
}

fn summarize(name: &str, resp: &serde_json::Value) -> (&'static str, &'static str, String, &'static str) {
    if let Some(e) = resp.get("error").and_then(|v| v.as_str()) {
        let msg = format!("error: {}", truncate(e, 70));
        return ("✗", RED, msg, RED);
    }
    if let Some(n) = resp.get("count").and_then(|v| v.as_u64()) {
        let unit = match name {
            "list_project_files" => "files",
            "search_project" => {
                if resp.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
                    "hits (truncated)"
                } else {
                    "hits"
                }
            }
            "read_project_files" => "results",
            _ => "items",
        };
        return ("✓", GREEN, format!("{n} {unit}"), GRAY);
    }
    if name == "get_project_metadata" {
        let name_field = resp
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        return ("✓", GREEN, format!("project={name_field}"), GRAY);
    }
    ("✓", GREEN, "ok".to_string(), GRAY)
}

fn format_elapsed(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

pub fn error_line(msg: &str) {
    eprintln!("{BOLD}{RED}✖ error:{RESET} {RED}{msg}{RESET}");
}

pub fn info_line(msg: &str) {
    eprintln!("{DIM}{YELLOW}ℹ {msg}{RESET}");
}

pub fn cancel_line() {
    eprintln!("{DIM}{YELLOW}⨯ cancelled{RESET}");
}

fn format_args(args: &serde_json::Value) -> String {
    let Some(obj) = args.as_object() else {
        return String::new();
    };
    if obj.is_empty() {
        return String::new();
    }
    let noise_keys: &[&str] = &["case_insensitive", "max_hits"];
    let mut parts = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        if noise_keys.contains(&k.as_str()) {
            continue;
        }
        if v.is_null() {
            continue;
        }
        let val = match v {
            serde_json::Value::String(s) => format!("\"{}\"", truncate(s, 40)),
            serde_json::Value::Array(arr) => format!("[{}]", arr.len()),
            serde_json::Value::Bool(b) => b.to_string(),
            other => truncate(&other.to_string(), 40),
        };
        parts.push(format!("{k}={val}"));
    }
    parts.join(" ")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}
