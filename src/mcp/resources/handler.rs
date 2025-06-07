use crate::mcp::stdio::{StdioTransport, Transport};
use crate::shared::logging;
use serde_json::json;

/// Handler for MCP resources operations
pub struct ResourcesHandler;

impl ResourcesHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle resources/list request
    pub async fn handle_resources_list(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling resources/list request");

        let response = crate::mcp::stdio::Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": []
            })),
            error: None,
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending resources/list response: {}", json_str));
        }

        match transport.send(response).await {
            Ok(_) => {
                logging::info("Resources list response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send resources/list response: {}", e));
                Err(e.into())
            }
        }
    }
}

impl Default for ResourcesHandler {
    fn default() -> Self {
        Self::new()
    }
}