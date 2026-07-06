use std::path::Path;

use ignore::WalkBuilder;

use crate::error::Error;
use crate::language::LanguageDetector;
use crate::snapshot::{FileMetadata, RepositoryDirectory, RepositoryFile};

fn extract_io_error(err: ignore::Error) -> std::io::Error {
    let msg = err.to_string();
    err.into_io_error()
        .unwrap_or_else(|| std::io::Error::other(msg))
}

pub(crate) fn walk_repository(
    root: &Path,
    language_detector: &dyn LanguageDetector,
) -> Result<(Vec<RepositoryFile>, Vec<RepositoryDirectory>), Error> {
    let mut files = Vec::new();
    let mut directories = Vec::new();

    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .build();

    for result in walker {
        let entry = result.map_err(|e| {
            let io_err = extract_io_error(e);
            Error::Traversal {
                path: root.to_path_buf(),
                source: io_err,
            }
        })?;

        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };

        if relative.as_os_str().is_empty() {
            continue;
        }

        match entry.file_type() {
            Some(ft) if ft.is_dir() => {
                directories.push(RepositoryDirectory {
                    relative_path: relative.to_path_buf(),
                });
            }
            Some(_) => {
                let meta = entry.metadata().ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta.and_then(|m| m.modified().ok());
                let detected_language = language_detector.detect(path);

                files.push(RepositoryFile {
                    relative_path: relative.to_path_buf(),
                    metadata: FileMetadata::new(size, modified),
                    language: detected_language,
                });
            }
            None => {}
        }
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    directories.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok((files, directories))
}
