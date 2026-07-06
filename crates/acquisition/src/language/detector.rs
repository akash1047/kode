use std::path::Path;

use crate::language::model::Language;

pub trait LanguageDetector {
    fn detect(&self, path: &Path) -> Option<Language>;
}
