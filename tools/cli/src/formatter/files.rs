use crate::presenter::files::FilesView;

pub fn format(view: &FilesView) -> String {
    let mut s = format!("{:<12} PATH\n", "LANGUAGE");
    for (lang, path) in &view.entries {
        s.push_str(&format!("{:<12} {}\n", lang, path));
    }
    s
}
