use crate::core::tfmcp::JsonRpcErrorCode;
use crate::mcp::stdio::{StdioTransport, Transport};
use crate::registry::fallback::RegistryClientWithFallback;
use crate::registry::provider::ProviderResolver;
use crate::shared::logging;
use serde_json::json;

/// Handler for Terraform Registry-related tool operations
pub struct RegistryToolsHandler {
    registry_client: RegistryClientWithFallback,
    provider_resolver: ProviderResolver,
}

impl RegistryToolsHandler {
    pub fn new() -> Self {
        Self {
            registry_client: RegistryClientWithFallback::new(),
            provider_resolver: ProviderResolver::new(),
        }
    }

    /// Handle search_terraform_providers tool call
    pub async fn handle_search_terraform_providers(
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

    /// Handle get_provider_info tool call
    pub async fn handle_get_provider_info(
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

    /// Handle get_provider_docs tool call
    pub async fn handle_get_provider_docs(
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

impl Default for RegistryToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}