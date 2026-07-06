use std::path::PathBuf;

/// Severity level for an extraction diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A diagnostic produced during fact extraction.
///
/// Carries severity, message, and optional source location.
/// Extraction continues past diagnostics whenever possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionDiagnostic {
    severity: Severity,
    message: String,
    source_file: Option<PathBuf>,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl ExtractionDiagnostic {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        source_file: Option<PathBuf>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            source_file,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn error(
        message: impl Into<String>,
        source_file: Option<PathBuf>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self::new(
            Severity::Error,
            message,
            source_file,
            start_line,
            start_column,
            end_line,
            end_column,
        )
    }

    pub fn warning(
        message: impl Into<String>,
        source_file: Option<PathBuf>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self::new(
            Severity::Warning,
            message,
            source_file,
            start_line,
            start_column,
            end_line,
            end_column,
        )
    }

    pub fn severity(&self) -> &Severity {
        &self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_file(&self) -> Option<&std::path::Path> {
        self.source_file.as_deref()
    }

    pub fn start_line(&self) -> usize {
        self.start_line
    }

    pub fn start_column(&self) -> usize {
        self.start_column
    }

    pub fn end_line(&self) -> usize {
        self.end_line
    }

    pub fn end_column(&self) -> usize {
        self.end_column
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn diagnostic_creation() {
        let d = ExtractionDiagnostic::error(
            "unsupported syntax",
            Some(PathBuf::from("test.rs")),
            1,
            1,
            1,
            10,
        );
        assert_eq!(d.severity(), &Severity::Error);
        assert_eq!(d.message(), "unsupported syntax");
        assert_eq!(d.source_file(), Some(Path::new("test.rs")));
    }

    #[test]
    fn warning_diagnostic() {
        let d = ExtractionDiagnostic::warning(
            "partial extraction",
            None,
            0,
            0,
            0,
            0,
        );
        assert_eq!(d.severity(), &Severity::Warning);
        assert!(d.source_file().is_none());
    }
}
