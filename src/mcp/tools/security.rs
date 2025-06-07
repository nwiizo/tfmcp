use crate::core::tfmcp::TfMcp;
use crate::mcp::stdio::{StdioTransport, Transport};
use crate::shared::logging;
use serde_json::json;

/// Handler for security-related tool operations
pub struct SecurityToolsHandler;

impl SecurityToolsHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle get_security_status tool call
    pub async fn handle_get_security_status(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        let security_info = json!({
            "policy": {
                "allow_dangerous_operations": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "allow_auto_approve": std::env::var("TFMCP_ALLOW_AUTO_APPROVE").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "max_resource_limit": std::env::var("TFMCP_MAX_RESOURCES").ok().and_then(|v| v.parse::<usize>().ok()),
                "audit_enabled": std::env::var("TFMCP_AUDIT_ENABLED").map(|v| v.to_lowercase() == "true").unwrap_or(true),
                "blocked_patterns": ["**/prod*/**", "**/production*/**", "**/*prod*.tf", "**/*production*.tf", "**/*secret*"]
            },
            "permissions": {
                "apply": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "destroy": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "plan": true,
                "validate": true,
                "init": true,
                "state": true
            },
            "audit_enabled": std::env::var("TFMCP_AUDIT_ENABLED").map(|v| v.to_lowercase() == "true").unwrap_or(true),
            "current_directory": tfmcp.get_project_directory().to_string_lossy(),
            "security_notes": [
                "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable apply/destroy operations",
                "Set TFMCP_ALLOW_AUTO_APPROVE=true to enable auto-approve for dangerous operations",
                "Set TFMCP_MAX_RESOURCES=N to limit maximum resource count",
                "Audit logs are stored in ~/.tfmcp/audit.log by default"
            ]
        });

        let obj_as_str = serde_json::to_string(&security_info)?;
        self.send_text_response(transport, id, &obj_as_str).await?;
        Ok(())
    }

    // Helper methods for sending responses
    async fn send_text_response(
        &self,
        transport: &StdioTransport,
        id: u64,
        text: &str,
    ) -> anyhow::Result<()> {
        logging::info(&format!("Sending text response for id {}", id));

        let response = crate::mcp::stdio::Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": text
                }]
            })),
            error: None,
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending text response: {}", json_str));
        }

        match transport.send(response).await {
            Ok(_) => {
                logging::info("Text response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send text response: {}", e));
                Err(anyhow::anyhow!("Failed to send text response: {}", e))
            }
        }
    }
}

impl Default for SecurityToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}