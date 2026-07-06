/// Severity level for a parse diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A diagnostic message produced during parsing.
///
/// Carries the severity, human-readable message, and source location span.
/// Diagnostics are the primary mechanism for communicating parse quality
/// to downstream consumers.
///
/// # Invariants
///
/// - Positions are 1-indexed (line 1, column 1 is the first character).
/// - The span is inclusive on both ends.
/// - An error with `Severity::Error` means the syntax tree may be incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: Severity,
    message: String,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn error(
        message: impl Into<String>,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self::new(
            Severity::Error,
            message,
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
