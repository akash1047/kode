use crate::presenter::status::StatusView;

pub fn format(view: &StatusView) -> String {
    format!(
        "Repository\n\
         \x20 Root: {root}\n\
         \x20 Workspace: {workspace}\n\
         \n\
         \x20 Files: {files}\n\
         \x20 Languages: {langs}\n\
         \n\
         \x20 Parsed:   {parsed:>6}\n\
         \x20 Skipped:  {skipped:>6}\n\
         \x20 Recovered:{recovered:>6}\n\
         \x20 Failed:   {failed:>6}\n",
        root = view.repository_root,
        workspace = view.workspace,
        files = view.files,
        langs = view.languages,
        parsed = view.parsed,
        recovered = view.recovered,
        skipped = view.skipped,
        failed = view.failed,
    )
}
