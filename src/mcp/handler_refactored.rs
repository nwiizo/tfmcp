use crate::core::tfmcp::{JsonRpcErrorCode, TfMcp};
use crate::mcp::stdio::{Message, StdioTransport, Transport};
use crate::mcp::tools::{TOOLS_JSON, TerraformToolsHandler, RegistryToolsHandler, SecurityToolsHandler};
use crate::mcp::resources::ResourcesHandler;
use crate::mcp::messages::MessagesHandler;
use crate::shared::logging;
use futures::StreamExt;
use serde_json::json;

pub struct McpHandler<'a> {
    tfmcp: &'a mut TfMcp,
    initialized: bool,
    terraform_handler: TerraformToolsHandler,
    registry_handler: RegistryToolsHandler,
    security_handler: SecurityToolsHandler,
    resources_handler: ResourcesHandler,
    messages_handler: MessagesHandler,
}

impl<'a> McpHandler<'a> {
    pub fn new(tfmcp: &'a mut TfMcp) -> Self {
        Self {
            tfmcp,
            initialized: false,
            terraform_handler: TerraformToolsHandler::new(),
            registry_handler: RegistryToolsHandler::new(),
            security_handler: SecurityToolsHandler::new(),
            resources_handler: ResourcesHandler::new(),
            messages_handler: MessagesHandler::new(),
        }
    }

    pub async fn launch_mcp(&mut self, transport: &StdioTransport) -> anyhow::Result<()> {
        logging::info("MCP stdio transport server started. Waiting for JSON messages on stdin...");
        
        // Create the stream receiver first, before sending any log messages
        let mut stream = transport.receive();
        
        // Add a small delay to ensure the receiver is ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        logging::send_log_message(
            transport,
            logging::LogLevel::Info,
            "tfmcp server initialized and ready",
        )
        .await?;

        // Add debug logging for stream creation
        logging::debug("Created message stream, starting to listen for messages...");

        while let Some(msg_result) = stream.next().await {
            logging::debug("Stream received a message, processing...");
            
            match &msg_result {
                Ok(msg) => {
                    logging::debug(&format!("Received message type: {:?}", std::any::type_name_of_val(msg)));
                }
                Err(e) => {
                    logging::debug(&format!("Received error: {:?}", e));
                }
            }
            
            match msg_result {
                Ok(Message::Request {
                    id, method, params, ..
                }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!(
                            "Got Request: id={}, method={}, params={:?}",
                            id, method, params
                        ),
                    )
                    .await?;

                    // Handle initialization request first
                    if method == "initialize" {
                        if let Err(err) = self.messages_handler.handle_initialize(transport, id).await {
                            logging::error(&format!("Error handling initialize request: {}", err));
                            self.send_error_response(
                                transport,
                                id,
                                JsonRpcErrorCode::InternalError,
                                format!("Failed to initialize: {}", err),
                            )
                            .await?;
                        } else {
                            self.initialized = true;
                            logging::info("MCP server successfully initialized");
                        }
                        continue;
                    }

                    // For all other requests, ensure we're initialized
                    if !self.initialized {
                        self.send_error_response(
                            transport,
                            id,
                            JsonRpcErrorCode::InvalidRequest,
                            "Server not initialized. Send 'initialize' request first.".to_string(),
                        )
                        .await?;
                        continue;
                    }

                    // Route requests to appropriate handlers
                    if let Err(err) = self.handle_request(transport, id, method, params).await {
                        logging::error(&format!("Error handling request: {:?}", err));
                        self.send_error_response(
                            transport,
                            id,
                            JsonRpcErrorCode::InternalError,
                            format!("Failed to handle request: {}", err),
                        )
                        .await?;
                    }
                }
                Ok(Message::Notification { method, params, .. }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!("Got Notification: method={}, params={:?}", method, params),
                    )
                    .await?;
                }
                Ok(Message::Response {
                    id, result, error, ..
                }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!(
                            "Got Response: id={}, result={:?}, error={:?}",
                            id, result, error
                        ),
                    )
                    .await?;
                }
                Err(e) => {
                    logging::error(&format!("Error receiving message: {:?}", e));
                }
            }
        }

        Ok(())
    }

    async fn handle_request(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        method: String,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        match &*method {
            "tools/list" => self.handle_tools_list(transport, id).await?,
            "tools/call" => {
                if let Some(params_val) = params {
                    self.handle_tools_call(transport, id, params_val).await?;
                }
            }
            "resources/list" => self.resources_handler.handle_resources_list(transport, id).await?,
            "prompts/list" => self.messages_handler.handle_prompts_list(transport, id).await?,
            _ => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::MethodNotFound,
                    format!("Method not found: {}", method),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn handle_tools_list(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        let tools_value: serde_json::Value =
            serde_json::from_str(TOOLS_JSON).expect("tools.json must be valid JSON");

        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(tools_value),
            error: None,
        };

        transport.send(response).await?;
        Ok(())
    }

    async fn handle_tools_call(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        params_val: serde_json::Value,
    ) -> anyhow::Result<()> {
        let name = params_val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        logging::info(&format!("Handling tools/call for tool: {}", name));

        match name {
            // Terraform tools
            "list_terraform_resources" => {
                self.terraform_handler.handle_list_terraform_resources(self.tfmcp, transport, id).await?;
            }
            "analyze_terraform" => {
                self.terraform_handler.handle_analyze_terraform(self.tfmcp, transport, id, &params_val).await?;
            }
            "get_terraform_plan" => {
                self.terraform_handler.handle_get_terraform_plan(self.tfmcp, transport, id).await?;
            }
            "apply_terraform" => {
                self.terraform_handler.handle_apply_terraform(self.tfmcp, transport, id, &params_val).await?;
            }
            "destroy_terraform" => {
                self.terraform_handler.handle_destroy_terraform(self.tfmcp, transport, id, &params_val).await?;
            }
            "validate_terraform" => {
                self.terraform_handler.handle_validate_terraform(self.tfmcp, transport, id).await?;
            }
            "validate_terraform_detailed" => {
                self.terraform_handler.handle_validate_terraform_detailed(self.tfmcp, transport, id).await?;
            }
            "get_terraform_state" => {
                self.terraform_handler.handle_get_terraform_state(self.tfmcp, transport, id).await?;
            }
            "init_terraform" => {
                self.terraform_handler.handle_init_terraform(self.tfmcp, transport, id).await?;
            }
            "set_terraform_directory" => {
                self.terraform_handler.handle_set_terraform_directory(self.tfmcp, transport, id, &params_val).await?;
            }
            
            // Registry tools
            "search_terraform_providers" => {
                self.registry_handler.handle_search_terraform_providers(transport, id, &params_val).await?;
            }
            "get_provider_info" => {
                self.registry_handler.handle_get_provider_info(transport, id, &params_val).await?;
            }
            "get_provider_docs" => {
                self.registry_handler.handle_get_provider_docs(transport, id, &params_val).await?;
            }
            
            // Security tools
            "get_security_status" => {
                self.security_handler.handle_get_security_status(self.tfmcp, transport, id).await?;
            }
            
            _ => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::MethodNotFound,
                    format!("Tool not found: {}", name),
                )
                .await?;
            }
        }

        Ok(())
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

        let response = Message::Response {
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