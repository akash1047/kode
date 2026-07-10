use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM provider error: {0}")]
    Provider(String),

    #[error("Tool execution error: {0}")]
    Tool(String),

    #[error("Max turns ({0}) exceeded")]
    MaxTurns(usize),

    #[error("Query engine error: {0}")]
    Query(#[from] kode_query::QueryError),

    #[error("Request timed out after {0}s")]
    Timeout(u64),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
