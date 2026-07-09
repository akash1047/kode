use crate::presenter::symbols::SymbolsView;

pub fn format(view: &SymbolsView) -> String {
    if view.entries.is_empty() {
        return "No symbols found.\n".to_string();
    }
    let mut out = String::from("Symbols\n");
    for (name, kind, file, line, verified) in &view.entries {
        let status = if *verified { "\u{2713}" } else { "?" };
        out.push_str(&format!("  {status} {name:<20} {kind:<12} {file}:{line}\n"));
    }
    out
}
