#![deny(unsafe_code)]
#![allow(unused_crate_dependencies)]

//! Evidence-first repository agent with sandboxed tools and optional symbol index.

pub mod agent;
pub mod config;
pub mod error;
pub mod index;
pub mod provider;
pub mod tools;

pub use agent::{Agent, AgentEvent};
pub use config::{AgentConfig, ProviderKind};
pub use error::AgentError;
pub use index::{IndexedSymbol, SharedIndex, SymbolIndex};
pub use provider::openai::OpenAiCompatProvider;
pub use provider::{Message, Provider, Role};
