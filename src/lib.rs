// Re-export modules for testing and external use
pub mod registry {
    pub mod batch;
    pub mod cache;
    pub mod client;
    pub mod fallback;
    pub mod provider;

    // Re-export commonly used items
    pub use batch::BatchFetcher;
    pub use cache::{CacheManager, SimpleCache};
    pub use client::{ProviderInfo, RegistryClient, RegistryError};
    pub use fallback::RegistryClientWithFallback;
    pub use provider::ProviderResolver;
}

pub mod formatters {
    pub mod output;

    pub use output::OutputFormatter;
}

pub mod prompts {
    pub mod builder;
    pub mod descriptions;

    pub use builder::{ToolDescription, ToolExample};
}

pub mod shared {
    pub mod logging;
    pub mod security;
    pub mod utils;
}

pub mod terraform {
    pub mod model;
    pub mod parser;
    pub mod service;
}

pub mod core {
    pub mod tfmcp;
}

pub mod mcp {
    pub mod handler;
    pub mod stdio;
}

pub mod config;

// Re-export commonly used types for easier testing and external use
pub use core::tfmcp::TfMcp;
pub use mcp::handler::McpHandler;
pub use mcp::stdio::{Message, StdioTransport};
pub use registry::cache::CacheManager;
pub use registry::provider::ProviderResolver;
pub use terraform::service::TerraformService;
