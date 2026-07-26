pub mod config;
pub mod core;
pub mod formatters;
pub mod mcp;
pub mod prompts;
pub mod registry;
pub mod shared;
pub mod terraform;
pub mod tfe;

// Re-export commonly used types for easier testing and external use
pub use core::tfmcp::TfMcp;
pub use mcp::server::TfMcpServer;
pub use registry::cache::CacheManager;
pub use registry::provider::ProviderResolver;
pub use terraform::model;
pub use terraform::service::TerraformService;
