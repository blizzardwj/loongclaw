pub mod acp;
pub mod channel;
pub mod chat;
pub mod config;
pub mod context;
pub mod conversation;
#[cfg(feature = "feishu-integration")]
pub mod feishu;
pub mod memory;
pub mod migration;
pub mod prompt;
pub mod provider;
pub mod tools;

#[cfg(test)]
mod process_env;

pub use context::KernelContext;
/// Result type for MVP CLI operations.
pub type CliResult<T> = Result<T, String>;
