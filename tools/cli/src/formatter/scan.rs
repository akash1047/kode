use crate::presenter::scan::ScanView;

pub fn format(view: &ScanView) -> String {
    format!(
        "Repository\n\
         \x20 Root: {root}\n\
         \x20 Workspace: {workspace}\n\
         \n\
         Discovery\n\
         \x20 Files: {files}\n\
         \x20 Directories: {dirs}\n\
         \x20 Languages: {langs}\n\
         \x20 Manifests: {manifests}\n\
         \n\
         Parsing\n\
         \x20 Parsed: {parsed}\n\
         \x20 Recovered: {recovered}\n\
         \x20 Skipped: {skipped}\n\
         \x20 Failed: {failed}\n\
         \n\
         Elapsed: {elapsed:.3}s\n",
        root = view.repository_root,
        workspace = view.workspace,
        files = view.files_discovered,
        dirs = view.directories,
        langs = view.languages,
        manifests = view.manifests,
        parsed = view.parsed,
        recovered = view.recovered,
        skipped = view.skipped,
        failed = view.failed,
        elapsed = view.elapsed_secs,
    )
}
