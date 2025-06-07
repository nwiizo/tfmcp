use crate::core::tfmcp::{JsonRpcErrorCode, TfMcp};
use crate::mcp::stdio::{StdioTransport, Transport};
use crate::shared::logging;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Handler for Terraform-related tool operations
pub struct TerraformToolsHandler;

impl TerraformToolsHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle list_terraform_resources tool call
    pub async fn handle_list_terraform_resources(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.list_resources().await {
            Ok(resources) => {
                let result_json = json!({ "resources": resources });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to list Terraform resources: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle analyze_terraform tool call
    pub async fn handle_analyze_terraform(
        &mut self,
        tfmcp: &mut TfMcp,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        // Get optional path parameter
        let _path = params_val
            .pointer("/arguments/path")
            .and_then(Value::as_str)
            .map(PathBuf::from);

        // Analyze Terraform configurations
        match tfmcp.analyze_terraform().await {
            Ok(analysis) => {
                let result_json = json!({ "analysis": analysis });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to analyze Terraform configuration: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle get_terraform_plan tool call
    pub async fn handle_get_terraform_plan(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.get_terraform_plan().await {
            Ok(plan) => {
                let result_json = json!({ "plan": plan });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get Terraform plan: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle apply_terraform tool call
    pub async fn handle_apply_terraform(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match tfmcp.apply_terraform(auto_approve).await {
            Ok(result) => {
                let result_json = json!({ "result": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to apply Terraform configuration: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle destroy_terraform tool call
    pub async fn handle_destroy_terraform(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match tfmcp.destroy_terraform(auto_approve).await {
            Ok(result) => {
                let result_json = json!({ "output": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                logging::error(&format!("Failed to destroy Terraform resources: {}", err));
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to destroy Terraform resources: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle validate_terraform tool call
    pub async fn handle_validate_terraform(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.validate_configuration().await {
            Ok(result) => {
                let valid = !result.contains("Error:");
                let result_json = json!({
                    "valid": valid,
                    "message": result
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to validate Terraform configuration: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle validate_terraform_detailed tool call
    pub async fn handle_validate_terraform_detailed(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.validate_configuration_detailed().await {
            Ok(result) => {
                let result_json = json!({
                    "valid": result.valid,
                    "error_count": result.error_count,
                    "warning_count": result.warning_count,
                    "diagnostics": result.diagnostics,
                    "additional_warnings": result.additional_warnings,
                    "suggestions": result.suggestions,
                    "checked_files": result.checked_files
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to perform detailed validation: {}", err),
                )
                .await?
            }
        }
        Ok(())
    }

    /// Handle get_terraform_state tool call
    pub async fn handle_get_terraform_state(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.get_state().await {
            Ok(state) => {
                let result_json = json!({ "state": state });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get Terraform state: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle init_terraform tool call
    pub async fn handle_init_terraform(
        &self,
        tfmcp: &TfMcp,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match tfmcp.init_terraform().await {
            Ok(result) => {
                let result_json = json!({ "result": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to initialize Terraform: {}", err),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Handle set_terraform_directory tool call
    pub async fn handle_set_terraform_directory(
        &mut self,
        tfmcp: &mut TfMcp,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        logging::info("Handling set_terraform_directory request");

        let directory = match params_val.pointer("/arguments/directory").and_then(|v| v.as_str()) {
            Some(dir) => dir.to_string(),
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: directory".to_string(),
                    )
                    .await;
            }
        };

        match tfmcp.change_project_directory(directory) {
            Ok(()) => {
                let current_dir = tfmcp.get_project_directory();
                let current_dir_str = current_dir.to_string_lossy().to_string();

                let response = crate::mcp::stdio::Message::Response {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "success": true,
                        "directory": current_dir_str,
                        "message": format!("Successfully changed to Terraform project directory: {}", current_dir_str)
                    })),
                    error: None,
                };

                if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                    logging::debug(&format!(
                        "Sending set_terraform_directory response: {}",
                        json_str
                    ));
                }

                match transport.send(response).await {
                    Ok(_) => {
                        logging::info("Set terraform directory response sent successfully");
                        Ok(())
                    }
                    Err(e) => {
                        logging::error(&format!(
                            "Failed to send set_terraform_directory response: {}",
                            e
                        ));
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to change Terraform directory: {}", e),
                )
                .await
            }
        }
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

    async fn send_error_response(
        &self,
        transport: &StdioTransport,
        id: u64,
        code: JsonRpcErrorCode,
        message: String,
    ) -> anyhow::Result<()> {
        logging::warn(&format!(
            "Sending error response for id {}: {}",
            id, message
        ));

        let response = crate::mcp::stdio::Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": code as i32,
                "message": message
            })),
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending error response: {}", json_str));
        }

        match transport.send(response).await {
            Ok(_) => {
                logging::info("Error response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send error response: {}", e));
                Err(anyhow::anyhow!("Failed to send error response: {}", e))
            }
        }
    }
}

impl Default for TerraformToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}