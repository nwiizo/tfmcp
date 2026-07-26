//! MCP protocol metadata, resources, and request dispatch.

use super::{McpError, TfMcpServer};
use crate::mcp::resources::{
    SERVER_INSTRUCTIONS, TERRAFORM_BEST_PRACTICES, get_module_dev_content, get_style_guide_content,
};
use rmcp::{
    ServerHandler,
    model::{
        Annotated, CallToolRequestParams, CallToolResult, Implementation,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
        RawResource, RawResourceTemplate, ReadResourceRequestParams, ReadResourceResult,
        ResourceContents, ServerCapabilities, ServerInfo,
    },
    service::{RequestContext, RoleServer},
};
use std::future::Future;

fn terraform_resources() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated::new(
            RawResource::new("terraform://style-guide", "Terraform Style Guide")
                .with_description("Best practices for HCL formatting and code style")
                .with_mime_type("text/markdown"),
            None,
        ),
        Annotated::new(
            RawResource::new("/terraform/style-guide", "Terraform Style Guide")
                .with_description("HashiCorp-compatible Terraform style guide resource alias")
                .with_mime_type("text/markdown"),
            None,
        ),
        Annotated::new(
            RawResource::new("terraform://module-development", "Module Development Guide")
                .with_description("Guide for developing reusable Terraform modules")
                .with_mime_type("text/markdown"),
            None,
        ),
        Annotated::new(
            RawResource::new("/terraform/module-development", "Module Development Guide")
                .with_description("HashiCorp-compatible module development resource alias")
                .with_mime_type("text/markdown"),
            None,
        ),
        Annotated::new(
            RawResource::new("terraform://best-practices", "Terraform Best Practices")
                .with_description("Security and operational best practices")
                .with_mime_type("text/markdown"),
            None,
        ),
    ]
}

fn terraform_resource_templates() -> Vec<Annotated<RawResourceTemplate>> {
    vec![
        Annotated::new(
            RawResourceTemplate::new(
                "terraform://providers/{namespace}/{name}/{version}/docs",
                "Provider Documentation",
            )
            .with_description("Fetch documentation for a specific Terraform provider version"),
            None,
        ),
        Annotated::new(
            RawResourceTemplate::new(
                "/terraform/providers/{namespace}/name/{name}/version/{version}",
                "Provider Documentation",
            )
            .with_description("HashiCorp-compatible provider documentation resource template"),
            None,
        ),
    ]
}

fn list_resources_result() -> ListResourcesResult {
    ListResourcesResult::with_all_items(terraform_resources())
}

fn list_resource_templates_result() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(terraform_resource_templates())
}

// The ServerHandler trait requires this specific impl Future pattern.
#[allow(clippy::manual_async_fn)]
impl ServerHandler for TfMcpServer {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        let server_info = Implementation::new("tfmcp", env!("CARGO_PKG_VERSION"));
        ServerInfo::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(SERVER_INSTRUCTIONS)
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(list_resources_result()))
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(list_resource_templates_result()))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let provider_path = if request.uri.starts_with("terraform://providers/")
                && request.uri.ends_with("/docs")
            {
                Some(
                    request
                        .uri
                        .trim_start_matches("terraform://providers/")
                        .trim_end_matches("/docs")
                        .to_string(),
                )
            } else if request.uri.starts_with("/terraform/providers/") {
                let path = request.uri.trim_start_matches("/terraform/providers/");
                let parts = path.split('/').collect::<Vec<_>>();
                if parts.len() == 5 && parts[1] == "name" && parts[3] == "version" {
                    Some(format!("{}/{}/{}", parts[0], parts[2], parts[4]))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(path) = provider_path {
                let parts = path.split('/').collect::<Vec<_>>();
                if parts.len() == 3 {
                    let (namespace, name) = (parts[0], parts[1]);
                    match self
                        .registry_client
                        .primary
                        .get_provider_info(name, namespace)
                        .await
                    {
                        Ok(info) => {
                            let docs = info
                                .extra
                                .get("docs")
                                .and_then(|value| value.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let content = format!(
                                "# {} ({}/{})\n\nVersion: {}\n{}\n\n## Available Documentation ({} items)\n\n{}",
                                info.name,
                                namespace,
                                info.name,
                                info.version,
                                info.description,
                                docs.len(),
                                docs.iter()
                                    .map(|doc| {
                                        let category = doc
                                            .get("category")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or("other");
                                        let title = doc
                                            .get("title")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or("?");
                                        format!("- **[{category}]** {title}")
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            );
                            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                                content,
                                request.uri,
                            )]));
                        }
                        Err(error) => {
                            return Err(McpError::resource_not_found(
                                format!("Provider not found: {error}"),
                                None,
                            ));
                        }
                    }
                }
            }

            let content = match request.uri.as_str() {
                "terraform://style-guide" | "/terraform/style-guide" => {
                    get_style_guide_content().await
                }
                "terraform://module-development" | "/terraform/module-development" => {
                    get_module_dev_content().await
                }
                "terraform://best-practices" => TERRAFORM_BEST_PRACTICES.to_string(),
                _ => {
                    return Err(McpError::resource_not_found(
                        format!("Unknown resource: {}", request.uri),
                        None,
                    ));
                }
            };

            Ok(ReadResourceResult::new(vec![ResourceContents::text(
                content,
                request.uri,
            )]))
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(self.list_tools_result())
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            if !self.tool_filter.is_enabled(&request.name) {
                return Err(McpError::invalid_request(
                    format!(
                        "Tool '{}' is not enabled. Check --toolsets configuration.",
                        request.name
                    ),
                    None,
                ));
            }
            let tool_context =
                rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            self.tool_router.call(tool_context).await
        }
    }
}
