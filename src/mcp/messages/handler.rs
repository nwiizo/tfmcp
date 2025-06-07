use crate::mcp::stdio::{StdioTransport, Transport};
use crate::shared::logging;
use serde_json::json;

/// Handler for MCP message operations (initialization, prompts, etc.)
pub struct MessagesHandler;

impl MessagesHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle initialize request
    pub async fn handle_initialize(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling initialize request");

        let response = crate::mcp::stdio::Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "capabilities": {
                    "experimental": {},
                    "prompts": { "listChanged": false },
                    "resources": { "listChanged": false, "subscribe": false },
                    "tools": { "listChanged": false }
                },
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "tfmcp",
                    "version": "0.1.3"
                }
            })),
            error: None,
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending initialize response: {}", json_str));
        }

        match transport.send(response).await {
            Ok(_) => {
                logging::info("Initialize response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send initialize response: {}", e));
                Err(anyhow::anyhow!("Failed to send initialize response: {}", e))
            }
        }
    }

    /// Handle prompts/list request
    pub async fn handle_prompts_list(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling prompts/list request");

        let response = crate::mcp::stdio::Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "prompts": []
            })),
            error: None,
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending prompts/list response: {}", json_str));
        }

        match transport.send(response).await {
            Ok(_) => {
                logging::info("Prompts list response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send prompts/list response: {}", e));
                Err(e.into())
            }
        }
    }
}

impl Default for MessagesHandler {
    fn default() -> Self {
        Self::new()
    }
}