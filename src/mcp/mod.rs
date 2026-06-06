pub mod agent;
pub mod agent_tools;
pub mod evidence;
pub mod http;
pub mod protocol;
pub mod server;
pub mod stdio;
pub mod tools;

/// Shared server state and entry-point for starting the MCP server.
pub use server::{McpState, boot};
