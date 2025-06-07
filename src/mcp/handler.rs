use crate::core::tfmcp::{JsonRpcErrorCode, TfMcp};
use crate::mcp::stdio::{Message, StdioTransport, Transport};
use crate::registry::fallback::RegistryClientWithFallback;
use crate::registry::provider::ProviderResolver;
use crate::shared::logging;
use futures::StreamExt;
use serde_json::{json, Value};
use std::path::PathBuf;

const TOOLS_JSON: &str = r#"{
  "tools": [
    {
      "name": "list_terraform_resources",
      "description": "List all resources defined in the Terraform project",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "resources": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "List of resource identifiers"
          }
        },
        "required": ["resources"]
      }
    },
    {
      "name": "destroy_terraform",
      "description": "Destroy all resources defined in the Terraform project (requires TFMCP_DELETE_ENABLED=true)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "auto_approve": {
            "type": "boolean",
            "description": "Whether to automatically approve the destroy operation without confirmation"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Output from the Terraform destroy command"
          }
        },
        "required": ["output"]
      }
    },
    {
      "name": "analyze_terraform",
      "description": "Analyze Terraform configuration files and provide detailed information",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Path to the Terraform configuration directory (optional)"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "analysis": {
            "type": "object",
            "properties": {
              "resources": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "type": {
                      "type": "string",
                      "description": "Terraform resource type"
                    },
                    "name": {
                      "type": "string",
                      "description": "Resource name"
                    },
                    "file": {
                      "type": "string",
                      "description": "File containing the resource definition"
                    }
                  }
                }
              }
            }
          }
        },
        "required": ["analysis"]
      }
    },
    {
      "name": "get_terraform_plan",
      "description": "Execute 'terraform plan' and return the output",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "plan": {
            "type": "string",
            "description": "Terraform plan output"
          }
        },
        "required": ["plan"]
      }
    },
    {
      "name": "apply_terraform",
      "description": "Apply Terraform configuration (WARNING: This will make actual changes to your infrastructure)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "auto_approve": {
            "type": "boolean",
            "description": "Whether to auto-approve changes without confirmation"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Terraform apply output"
          }
        },
        "required": ["output"]
      }
    },
    {
      "name": "validate_terraform",
      "description": "Validate Terraform configuration files",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "valid": {
            "type": "boolean",
            "description": "Whether the configuration is valid"
          },
          "message": {
            "type": "string",
            "description": "Validation message"
          }
        },
        "required": ["valid", "message"]
      }
    },
    {
      "name": "validate_terraform_detailed",
      "description": "Perform detailed validation of Terraform configuration files with best practice checks",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "valid": {
            "type": "boolean",
            "description": "Whether the configuration is valid"
          },
          "error_count": {
            "type": "integer",
            "description": "Number of validation errors"
          },
          "warning_count": {
            "type": "integer",
            "description": "Number of warnings including best practice violations"
          },
          "diagnostics": {
            "type": "array",
            "description": "List of validation diagnostics from Terraform",
            "items": {
              "type": "object",
              "properties": {
                "severity": {
                  "type": "string",
                  "description": "Severity level (error, warning)"
                },
                "summary": {
                  "type": "string",
                  "description": "Summary of the diagnostic"
                },
                "detail": {
                  "type": "string",
                  "description": "Detailed description"
                },
                "range": {
                  "type": "object",
                  "description": "Location of the issue in the file",
                  "properties": {
                    "filename": {
                      "type": "string"
                    },
                    "start": {
                      "type": "object",
                      "properties": {
                        "line": { "type": "integer" },
                        "column": { "type": "integer" }
                      }
                    },
                    "end": {
                      "type": "object",
                      "properties": {
                        "line": { "type": "integer" },
                        "column": { "type": "integer" }
                      }
                    }
                  }
                }
              }
            }
          },
          "additional_warnings": {
            "type": "array",
            "description": "Additional warnings from best practice analysis",
            "items": {
              "type": "string"
            }
          },
          "suggestions": {
            "type": "array",
            "description": "Suggestions for improving the configuration",
            "items": {
              "type": "string"
            }
          },
          "checked_files": {
            "type": "integer",
            "description": "Number of Terraform files checked"
          }
        },
        "required": ["valid", "error_count", "warning_count", "diagnostics", "additional_warnings", "suggestions", "checked_files"]
      }
    },
    {
      "name": "get_terraform_state",
      "description": "Get the current Terraform state",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "state": {
            "type": "string",
            "description": "Terraform state output"
          }
        },
        "required": ["state"]
      }
    },
    {
      "name": "init_terraform",
      "description": "Initialize a Terraform project",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Terraform init output"
          }
        },
        "required": ["output"]
      }
    },
    {
      "name": "get_security_status",
      "description": "Get current security policy and status",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "policy": {
            "type": "object",
            "description": "Current security policy configuration"
          },
          "permissions": {
            "type": "object",
            "description": "Current operation permissions"
          },
          "audit_enabled": {
            "type": "boolean",
            "description": "Whether audit logging is enabled"
          }
        },
        "required": ["policy", "permissions", "audit_enabled"]
      }
    },
    {
      "name": "search_terraform_providers",
      "description": "Search for Terraform providers in the official registry",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "Search query for provider names"
          }
        },
        "required": ["query"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "providers": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string" },
                "namespace": { "type": "string" },
                "version": { "type": "string" },
                "description": { "type": "string" }
              }
            }
          }
        },
        "required": ["providers"]
      }
    },
    {
      "name": "get_provider_info",
      "description": "Get detailed information about a specific Terraform provider",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider_name": {
            "type": "string",
            "description": "Name of the provider"
          },
          "namespace": {
            "type": "string",
            "description": "Provider namespace (optional, will try common namespaces)"
          }
        },
        "required": ["provider_name"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "provider": {
            "type": "object",
            "description": "Provider information including versions and documentation"
          }
        },
        "required": ["provider"]
      }
    },
    {
      "name": "get_provider_docs",
      "description": "Get documentation for specific provider resources",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider_name": {
            "type": "string",
            "description": "Name of the provider"
          },
          "namespace": {
            "type": "string",
            "description": "Provider namespace (optional)"
          },
          "service_slug": {
            "type": "string",
            "description": "Service or resource name to search for"
          },
          "data_type": {
            "type": "string",
            "description": "Type of documentation (resources, data-sources)",
            "enum": ["resources", "data-sources"]
          }
        },
        "required": ["provider_name", "service_slug"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "documentation": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "content": { "type": "string" }
              }
            }
          }
        },
        "required": ["documentation"]
      }
    },
    {
      "name": "set_terraform_directory",
      "description": "Change the current Terraform project directory",
      "inputSchema": {
        "type": "object",
        "properties": {
          "directory": {
            "type": "string",
            "description": "Path to the new Terraform project directory"
          }
        },
        "required": ["directory"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "success": {
            "type": "boolean",
            "description": "Whether the directory change was successful"
          },
          "directory": {
            "type": "string",
            "description": "The new Terraform project directory path"
          },
          "message": {
            "type": "string",
            "description": "Status message"
          }
        },
        "required": ["success", "directory", "message"]
      }
    }
  ]
}"#;

pub struct McpHandler<'a> {
    tfmcp: &'a mut TfMcp,
    initialized: bool,
    registry_client: RegistryClientWithFallback,
    provider_resolver: ProviderResolver,
}

impl<'a> McpHandler<'a> {
    pub fn new(tfmcp: &'a mut TfMcp) -> Self {
        Self {
            tfmcp,
            initialized: false,
            registry_client: RegistryClientWithFallback::new(),
            provider_resolver: ProviderResolver::new(),
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
            
            // Debug: Log what type of message we received
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
                        if let Err(err) = self.handle_initialize(transport, id).await {
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

                    // Skip calling handle_request for methods already handled above
                    if method != "initialize" {
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
            "resources/list" => self.handle_resources_list(transport, id).await?,
            "prompts/list" => self.handle_prompts_list(transport, id).await?,
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

    async fn handle_initialize(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling initialize request");

        // Create a properly structured capabilities response
        let response = Message::Response {
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
                    "version": "0.1.0"
                }
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending initialize response: {}", json_str));
        }

        // Send the response
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
            "list_terraform_resources" => {
                self.handle_list_terraform_resources(transport, id).await?;
            }
            "analyze_terraform" => {
                self.handle_analyze_terraform(transport, id, &params_val)
                    .await?;
            }
            "get_terraform_plan" => {
                self.handle_get_terraform_plan(transport, id).await?;
            }
            "apply_terraform" => {
                self.handle_apply_terraform(transport, id, &params_val)
                    .await?;
            }
            "destroy_terraform" => {
                self.handle_destroy_terraform(transport, id, &params_val)
                    .await?;
            }
            "validate_terraform" => {
                self.handle_validate_terraform(transport, id).await?;
            }
            "validate_terraform_detailed" => {
                self.handle_validate_terraform_detailed(transport, id)
                    .await?;
            }
            "get_terraform_state" => {
                self.handle_get_terraform_state(transport, id).await?;
            }
            "init_terraform" => {
                self.handle_init_terraform(transport, id).await?;
            }
            "set_terraform_directory" => {
                self.handle_set_terraform_directory(transport, id, &params_val)
                    .await?;
            }
            "get_security_status" => {
                self.handle_get_security_status(transport, id).await?;
            }
            "search_terraform_providers" => {
                self.handle_search_terraform_providers(transport, id, &params_val)
                    .await?;
            }
            "get_provider_info" => {
                self.handle_get_provider_info(transport, id, &params_val)
                    .await?;
            }
            "get_provider_docs" => {
                self.handle_get_provider_docs(transport, id, &params_val)
                    .await?;
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

    async fn handle_list_terraform_resources(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.list_resources().await {
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

    async fn handle_analyze_terraform(
        &mut self,
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
        match self.tfmcp.analyze_terraform().await {
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

    async fn handle_get_terraform_plan(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.get_terraform_plan().await {
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

    async fn handle_apply_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match self.tfmcp.apply_terraform(auto_approve).await {
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

    async fn handle_validate_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.validate_configuration().await {
            Ok(result) => {
                // If validation succeeded, result will contain a success message
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

    async fn handle_validate_terraform_detailed(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.validate_configuration_detailed().await {
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

    async fn handle_get_terraform_state(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.get_state().await {
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

    async fn handle_init_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.init_terraform().await {
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

    async fn handle_resources_list(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling resources/list request");

        // Create a response with an empty resources list
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": []
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending resources/list response: {}", json_str));
        }

        // Send the response
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

    async fn handle_prompts_list(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling prompts/list request");

        // Create a response with an empty prompts list
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "prompts": []
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending prompts/list response: {}", json_str));
        }

        // Send the response
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

    async fn send_text_response(
        &self,
        transport: &StdioTransport,
        id: u64,
        text: &str,
    ) -> anyhow::Result<()> {
        logging::info(&format!("Sending text response for id {}", id));

        // Create a properly structured text response
        let response = Message::Response {
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

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending text response: {}", json_str));
        }

        // Send the response
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

        // Create a properly structured error response
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": code as i32,
                "message": message
            })),
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending error response: {}", json_str));
        }

        // Send the response
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

    // 新しいハンドラー: Terraformディレクトリを変更する
    async fn handle_set_terraform_directory(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        logging::info("Handling set_terraform_directory request");

        // パラメータから新しいディレクトリパスを取得
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

        // ディレクトリを変更
        match self.tfmcp.change_project_directory(directory) {
            Ok(()) => {
                // 現在のディレクトリを取得して応答
                let current_dir = self.tfmcp.get_project_directory();
                let current_dir_str = current_dir.to_string_lossy().to_string();

                let response = Message::Response {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "success": true,
                        "directory": current_dir_str,
                        "message": format!("Successfully changed to Terraform project directory: {}", current_dir_str)
                    })),
                    error: None,
                };

                // レスポンスをログに記録
                if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                    logging::debug(&format!(
                        "Sending set_terraform_directory response: {}",
                        json_str
                    ));
                }

                // レスポンスを送信
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

    async fn handle_destroy_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        // Check for auto_approve parameter
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Execute destroy operation
        match self.tfmcp.destroy_terraform(auto_approve).await {
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

    async fn handle_get_security_status(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        // Get security policy from TerraformService
        // Since TfMcp doesn't expose the service directly, we need to add a method
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
            "current_directory": self.tfmcp.get_project_directory().to_string_lossy(),
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

    // Registry-related handlers
    async fn handle_search_terraform_providers(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let query = match params_val.pointer("/arguments/query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: query".to_string(),
                    )
                    .await;
            }
        };

        match self.provider_resolver.search_providers(query).await {
            Ok(providers) => {
                let result_json = json!({ "providers": providers });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to search providers: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_provider_info(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let provider_name = match params_val.pointer("/arguments/provider_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider_name".to_string(),
                    )
                    .await;
            }
        };

        let namespace = params_val.pointer("/arguments/namespace").and_then(|v| v.as_str());

        match self.registry_client.get_provider_info(provider_name, namespace).await {
            Ok(provider_info) => {
                // Also get versions for comprehensive information
                match self.registry_client.get_provider_version(provider_name, namespace).await {
                    Ok((version, used_namespace)) => {
                        let result_json = json!({
                            "provider": {
                                "name": provider_info.name,
                                "namespace": used_namespace,
                                "latest_version": version,
                                "description": provider_info.description,
                                "downloads": provider_info.downloads,
                                "published_at": provider_info.published_at,
                                "info": provider_info
                            }
                        });
                        let obj_as_str = serde_json::to_string(&result_json)?;
                        self.send_text_response(transport, id, &obj_as_str).await?;
                    }
                    Err(err) => {
                        // Return basic info even if version lookup fails
                        let result_json = json!({
                            "provider": {
                                "info": provider_info,
                                "version_error": err.to_string()
                            }
                        });
                        let obj_as_str = serde_json::to_string(&result_json)?;
                        self.send_text_response(transport, id, &obj_as_str).await?;
                    }
                }
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get provider info: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_provider_docs(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let provider_name = match params_val.pointer("/arguments/provider_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider_name".to_string(),
                    )
                    .await;
            }
        };

        let service_slug = match params_val.pointer("/arguments/service_slug").and_then(|v| v.as_str()) {
            Some(slug) => slug,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: service_slug".to_string(),
                    )
                    .await;
            }
        };

        let namespace = params_val.pointer("/arguments/namespace").and_then(|v| v.as_str());
        let data_type = params_val.pointer("/arguments/data_type").and_then(|v| v.as_str()).unwrap_or("resources");

        match self.registry_client.search_docs_with_fallback(
            provider_name,
            namespace,
            service_slug,
            data_type,
        ).await {
            Ok((doc_ids, used_namespace)) => {
                // Fetch content for each documentation ID
                let mut documentation = Vec::new();
                
                for doc_id in doc_ids {
                    match self.provider_resolver.get_provider_docs(&doc_id.id).await {
                        Ok(content) => {
                            documentation.push(json!({
                                "id": doc_id.id,
                                "title": doc_id.title,
                                "description": doc_id.description,
                                "category": doc_id.category,
                                "content": content
                            }));
                        }
                        Err(err) => {
                            // Include the doc entry even if content fetch fails
                            documentation.push(json!({
                                "id": doc_id.id,
                                "title": doc_id.title,
                                "description": doc_id.description,
                                "category": doc_id.category,
                                "content_error": err.to_string()
                            }));
                        }
                    }
                }

                let result_json = json!({
                    "documentation": documentation,
                    "namespace": used_namespace,
                    "provider": provider_name,
                    "service": service_slug,
                    "type": data_type
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get provider documentation: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }
}
