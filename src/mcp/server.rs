//! RMCP-based MCP server implementation for tfmcp.

use crate::core::tfmcp::TfMcp;
use crate::mcp::types::*;
use crate::registry::fallback::RegistryClientWithFallback;
use crate::registry::provider::ProviderResolver;
use crate::shared::logging;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, CallToolRequestParam, CallToolResult, Content, Implementation, InitializeResult,
        ListResourcesResult, ListToolsResult, PaginatedRequestParam, PromptsCapability,
        ProtocolVersion, RawResource, ReadResourceRequestParam, ReadResourceResult,
        ResourceContents, ResourcesCapability, ServerCapabilities, ToolsCapability,
    },
    service::{RequestContext, RoleServer, ServiceExt},
    tool, tool_router,
};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

// Resource content for MCP resources
use super::resources::{
    TERRAFORM_BEST_PRACTICES, TERRAFORM_MODULE_DEVELOPMENT, TERRAFORM_STYLE_GUIDE,
};

/// RMCP-based MCP server for Terraform operations.
#[derive(Clone)]
pub struct TfMcpServer {
    tfmcp: Arc<RwLock<TfMcp>>,
    registry_client: Arc<RegistryClientWithFallback>,
    provider_resolver: Arc<ProviderResolver>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TfMcpServer {
    /// Create a new TfMcpServer instance.
    pub fn new(tfmcp: TfMcp) -> Self {
        Self {
            tfmcp: Arc::new(RwLock::new(tfmcp)),
            registry_client: Arc::new(RegistryClientWithFallback::new()),
            provider_resolver: Arc::new(ProviderResolver::new()),
            tool_router: Self::tool_router(),
        }
    }

    /// Serve the MCP server over stdio.
    pub async fn serve_stdio(tfmcp: TfMcp) -> anyhow::Result<()> {
        use tokio::io::{stdin, stdout};

        let server = Self::new(tfmcp);
        let transport = (stdin(), stdout());

        logging::info("Starting tfmcp MCP server via stdio...");
        let service = server.serve(transport).await?;

        // Wait for the server to finish (keep it alive)
        service.waiting().await?;

        Ok(())
    }

    // ============ Core Terraform Operations ============

    #[tool(
        description = "List all resources defined in the Terraform project",
        annotations(title = "List Terraform Resources", read_only_hint = true)
    )]
    async fn list_terraform_resources(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing list_terraform_resources tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.list_resources().await {
            Ok(resources) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "resources": resources
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to list resources: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Execute 'terraform plan' and return the output",
        annotations(title = "Get Terraform Plan", read_only_hint = true)
    )]
    async fn get_terraform_plan(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_terraform_plan tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.get_terraform_plan().await {
            Ok(output) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "plan": output
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get plan: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Apply Terraform configuration (WARNING: Makes actual infrastructure changes)",
        annotations(title = "Apply Terraform", destructive_hint = true)
    )]
    async fn apply_terraform(
        &self,
        params: Parameters<AutoApproveInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing apply_terraform tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.apply_terraform(params.0.auto_approve).await {
            Ok(output) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "output": output
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to apply: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Destroy all Terraform resources (requires TFMCP_ALLOW_DANGEROUS_OPS=true)",
        annotations(title = "Destroy Terraform", destructive_hint = true)
    )]
    async fn destroy_terraform(
        &self,
        params: Parameters<AutoApproveInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing destroy_terraform tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.destroy_terraform(params.0.auto_approve).await {
            Ok(output) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "output": output
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to destroy: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Initialize a Terraform project (downloads providers and modules)",
        annotations(
            title = "Initialize Terraform",
            open_world_hint = true,
            idempotent_hint = true
        )
    )]
    async fn init_terraform(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing init_terraform tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.init_terraform().await {
            Ok(output) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "output": output
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to init: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Validate Terraform configuration files",
        annotations(title = "Validate Terraform", read_only_hint = true)
    )]
    async fn validate_terraform(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing validate_terraform tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.validate_configuration().await {
            Ok(result) => {
                let valid = !result.contains("Error:");
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "valid": valid,
                    "message": result
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Validation failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Perform detailed validation with diagnostics and best practice checks",
        annotations(title = "Validate Terraform (Detailed)", read_only_hint = true)
    )]
    async fn validate_terraform_detailed(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing validate_terraform_detailed tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.validate_configuration_detailed().await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Detailed validation failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get the current Terraform state",
        annotations(title = "Get Terraform State", read_only_hint = true)
    )]
    async fn get_terraform_state(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_terraform_state tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.get_state().await {
            Ok(state) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "state": state
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get state: {}",
                e
            ))])),
        }
    }

    // ============ Configuration & Analysis ============

    #[tool(
        description = "Analyze Terraform configuration and return detailed analysis including provider version checks",
        annotations(title = "Analyze Terraform", read_only_hint = true)
    )]
    async fn analyze_terraform(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing analyze_terraform tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.get_terraform_analysis().await {
            Ok(analysis) => {
                // Run guideline checks for additional provider version info
                let guideline_summary = match tfmcp.run_security_scan().await {
                    Ok(checks) => {
                        serde_json::json!({
                            "compliance_score": checks.compliance_score,
                            "providers_missing_version": checks.providers_missing_version,
                            "variables_missing_type": checks.variables_missing_type.len(),
                            "variables_missing_description": checks.variables_missing_description.len(),
                            "outputs_missing_description": checks.outputs_missing_description.len()
                        })
                    }
                    Err(_) => serde_json::json!(null),
                };

                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "project_directory": analysis.project_directory,
                    "file_count": analysis.file_count,
                    "resources": analysis.resources,
                    "variables": analysis.variables,
                    "outputs": analysis.outputs,
                    "providers": analysis.providers,
                    "guideline_summary": guideline_summary
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Change the current Terraform project directory",
        annotations(title = "Set Terraform Directory", idempotent_hint = true)
    )]
    async fn set_terraform_directory(
        &self,
        params: Parameters<DirectoryInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing set_terraform_directory tool");
        // This is the only tool that needs a write lock
        let mut tfmcp = self.tfmcp.write().await;
        match tfmcp.change_project_directory(params.0.directory.clone()) {
            Ok(()) => {
                let dir = tfmcp.get_project_directory().to_string_lossy().to_string();
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "directory": dir,
                    "message": format!("Changed to: {}", dir)
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to change directory: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get the current security status, policy information, and secret detection scan results",
        annotations(title = "Get Security Status", read_only_hint = true)
    )]
    async fn get_security_status(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_security_status tool");

        // Get environment-based policy settings
        let allow_dangerous = std::env::var("TFMCP_ALLOW_DANGEROUS_OPS")
            .map(|v| v == "true")
            .unwrap_or(false);
        let allow_auto_approve = std::env::var("TFMCP_ALLOW_AUTO_APPROVE")
            .map(|v| v == "true")
            .unwrap_or(false);

        // Run security scan for secret detection and compliance
        let tfmcp = self.tfmcp.read().await;
        let scan_result = tfmcp.run_security_scan().await;

        let (secrets_detected, compliance_score, scan_status) = match scan_result {
            Ok(checks) => {
                let secrets: Vec<_> = checks
                    .hardcoded_secrets
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "file": s.file,
                            "line": s.line,
                            "pattern": s.pattern,
                            "severity": s.severity
                        })
                    })
                    .collect();
                (secrets, checks.compliance_score, "completed")
            }
            Err(e) => {
                logging::error(&format!("Security scan failed: {}", e));
                (vec![], 0, "failed")
            }
        };

        let json = serde_json::to_string_pretty(&serde_json::json!({
            "policy": {
                "allow_dangerous_operations": allow_dangerous,
                "allow_auto_approve": allow_auto_approve
            },
            "permissions": {
                "apply": allow_dangerous,
                "destroy": allow_dangerous,
                "init": true,
                "plan": true,
                "validate": true
            },
            "audit_enabled": true,
            "security_scan": {
                "status": scan_status,
                "secrets_detected": secrets_detected,
                "secrets_count": secrets_detected.len(),
                "compliance_score": compliance_score
            }
        }))
        .unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Analyze module health with cohesion, coupling metrics, and variable quality checks",
        annotations(title = "Analyze Module Health", read_only_hint = true)
    )]
    async fn analyze_module_health(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing analyze_module_health tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.analyze_module_health().await {
            Ok(health) => {
                // Run guideline checks for variable quality info
                let variable_quality = match tfmcp.run_security_scan().await {
                    Ok(checks) => {
                        serde_json::json!({
                            "variables_missing_type": checks.variables_missing_type,
                            "variables_missing_description": checks.variables_missing_description,
                            "any_type_usage": checks.any_type_usage,
                            "outputs_missing_description": checks.outputs_missing_description
                        })
                    }
                    Err(_) => serde_json::json!(null),
                };

                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "module_path": health.module_path,
                    "health_score": health.health_score,
                    "metrics": health.metrics,
                    "issues": health.issues,
                    "recommendations": health.recommendations,
                    "cohesion_analysis": health.cohesion_analysis,
                    "coupling_analysis": health.coupling_analysis,
                    "variable_quality": variable_quality
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Module health analysis failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get the resource dependency graph",
        annotations(title = "Get Resource Dependency Graph", read_only_hint = true)
    )]
    async fn get_resource_dependency_graph(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_resource_dependency_graph tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.get_dependency_graph().await {
            Ok(graph) => {
                let json = serde_json::to_string_pretty(&graph).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get dependency graph: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get module refactoring suggestions",
        annotations(title = "Suggest Module Refactoring", read_only_hint = true)
    )]
    async fn suggest_module_refactoring(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing suggest_module_refactoring tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.suggest_refactoring().await {
            Ok(suggestions) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "suggestions": suggestions
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get refactoring suggestions: {}",
                e
            ))])),
        }
    }

    // ============ Registry Tools ============

    #[tool(
        description = "Search for Terraform providers in the official registry",
        annotations(
            title = "Search Terraform Providers",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_terraform_providers(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing search_terraform_providers tool");
        match self
            .provider_resolver
            .search_providers(&params.0.query)
            .await
        {
            Ok(providers) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "providers": providers
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Provider search failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get detailed information about a specific provider",
        annotations(
            title = "Get Provider Info",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_info(
        &self,
        params: Parameters<ProviderInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_provider_info tool");
        match self
            .registry_client
            .get_provider_info(&params.0.provider_name, params.0.namespace.as_deref())
            .await
        {
            Ok(info) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "provider": info
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get provider info: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get documentation for a specific provider resource or data source",
        annotations(
            title = "Get Provider Docs",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_docs(
        &self,
        params: Parameters<ProviderDocsInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_provider_docs tool");
        let namespace = params.0.namespace.as_deref().unwrap_or("hashicorp");
        let data_type = params.0.data_type.as_deref().unwrap_or("resources");
        match self
            .registry_client
            .primary
            .search_docs(
                &params.0.provider_name,
                namespace,
                &params.0.service_slug,
                data_type,
            )
            .await
        {
            Ok(docs) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "documentation": docs
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get provider docs: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Search for Terraform modules in the registry",
        annotations(
            title = "Search Terraform Modules",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_terraform_modules(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing search_terraform_modules tool");
        match self
            .registry_client
            .primary
            .search_modules(&params.0.query)
            .await
        {
            Ok(modules) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "modules": modules
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Module search failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get detailed information about a specific module",
        annotations(
            title = "Get Module Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_module_details(
        &self,
        params: Parameters<ModuleInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_module_details tool");
        match self
            .registry_client
            .primary
            .get_module_details(
                &params.0.namespace,
                &params.0.name,
                &params.0.provider,
                params.0.version.as_deref(),
            )
            .await
        {
            Ok(details) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "module": details
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get module details: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get the latest version of a module",
        annotations(
            title = "Get Latest Module Version",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_latest_module_version(
        &self,
        params: Parameters<ModuleVersionInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_latest_module_version tool");
        match self
            .registry_client
            .primary
            .get_latest_module_version(&params.0.namespace, &params.0.name, &params.0.provider)
            .await
        {
            Ok(version) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "version": version,
                    "module_id": format!("{}/{}/{}", params.0.namespace, params.0.name, params.0.provider)
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get latest module version: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get the latest version of a provider",
        annotations(
            title = "Get Latest Provider Version",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_latest_provider_version(
        &self,
        params: Parameters<ProviderInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_latest_provider_version tool");
        match self
            .registry_client
            .get_provider_version(&params.0.provider_name, params.0.namespace.as_deref())
            .await
        {
            Ok((version, namespace)) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "version": version,
                    "namespace": namespace,
                    "provider_id": format!("{}/{}", namespace, params.0.provider_name)
                }))
                .unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get latest provider version: {}",
                e
            ))])),
        }
    }

    // ============ v0.1.9 New Tools ============

    #[tool(
        description = "Analyze terraform plan with risk scoring and recommendations",
        annotations(title = "Analyze Plan", read_only_hint = true)
    )]
    async fn analyze_plan(
        &self,
        params: Parameters<AnalyzePlanInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing analyze_plan tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.analyze_plan(params.0.include_risk).await {
            Ok(analysis) => {
                let json = serde_json::to_string_pretty(&analysis).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Plan analysis failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Analyze terraform state with optional drift detection",
        annotations(title = "Analyze State", read_only_hint = true)
    )]
    async fn analyze_state(
        &self,
        params: Parameters<AnalyzeStateInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing analyze_state tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp
            .analyze_state(params.0.resource_type.as_deref(), params.0.detect_drift)
            .await
        {
            Ok(analysis) => {
                let json = serde_json::to_string_pretty(&analysis).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "State analysis failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Manage terraform workspaces (list, show, new, select, delete)",
        annotations(title = "Terraform Workspace", idempotent_hint = true)
    )]
    async fn terraform_workspace(
        &self,
        params: Parameters<WorkspaceInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_workspace tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp
            .workspace(&params.0.action, params.0.name.as_deref())
            .await
        {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Workspace operation failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Import existing resources into Terraform state",
        annotations(title = "Terraform Import", destructive_hint = true)
    )]
    async fn terraform_import(
        &self,
        params: Parameters<ImportInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_import tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp
            .import_resource(
                &params.0.resource_type,
                &params.0.resource_id,
                &params.0.name,
                params.0.execute,
            )
            .await
        {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Import failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Format Terraform configuration files",
        annotations(title = "Terraform Format", idempotent_hint = true)
    )]
    async fn terraform_fmt(
        &self,
        params: Parameters<FmtInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_fmt tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp
            .fmt(params.0.check, params.0.diff, params.0.file.as_deref())
            .await
        {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Format failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Generate Terraform dependency graph in DOT format",
        annotations(title = "Terraform Graph", read_only_hint = true)
    )]
    async fn terraform_graph(
        &self,
        params: Parameters<GraphInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_graph tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.graph(params.0.graph_type.as_deref()).await {
            Ok(graph) => {
                let json = serde_json::to_string_pretty(&graph).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Graph generation failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get Terraform output values",
        annotations(title = "Terraform Output", read_only_hint = true)
    )]
    async fn terraform_output(
        &self,
        params: Parameters<OutputInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_output tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.output(params.0.name.as_deref()).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Output retrieval failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Taint or untaint a resource (deprecated: use -replace instead)",
        annotations(title = "Terraform Taint", destructive_hint = true)
    )]
    async fn terraform_taint(
        &self,
        params: Parameters<TaintInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_taint tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.taint(&params.0.action, &params.0.address).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Taint operation failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Refresh Terraform state to match real infrastructure",
        annotations(title = "Terraform Refresh", destructive_hint = true)
    )]
    async fn terraform_refresh(
        &self,
        params: Parameters<RefreshInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_refresh tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.refresh_state(params.0.target.as_deref()).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Refresh failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Get information about Terraform providers and lock file",
        annotations(title = "Terraform Providers", read_only_hint = true)
    )]
    async fn terraform_providers(
        &self,
        params: Parameters<ProvidersInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing terraform_providers tool");
        let tfmcp = self.tfmcp.read().await;
        match tfmcp.get_providers(params.0.include_lock).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Provider info failed: {}",
                e
            ))])),
        }
    }
}

// The ServerHandler trait requires this specific impl Future pattern
#[allow(clippy::manual_async_fn)]
impl ServerHandler for TfMcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                prompts: Some(PromptsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "tfmcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "tfmcp is a Terraform Model Context Protocol server. Use the tools to manage Terraform configurations.".into(),
            ),
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            Ok(ListResourcesResult {
                resources: vec![
                    Annotated {
                        raw: RawResource {
                            uri: "terraform://style-guide".into(),
                            name: "Terraform Style Guide".into(),
                            description: Some(
                                "Best practices for HCL formatting and code style".into(),
                            ),
                            mime_type: Some("text/markdown".into()),
                            title: None,
                            size: None,
                            icons: None,
                            meta: None,
                        },
                        annotations: None,
                    },
                    Annotated {
                        raw: RawResource {
                            uri: "terraform://module-development".into(),
                            name: "Module Development Guide".into(),
                            description: Some(
                                "Guide for developing reusable Terraform modules".into(),
                            ),
                            mime_type: Some("text/markdown".into()),
                            title: None,
                            size: None,
                            icons: None,
                            meta: None,
                        },
                        annotations: None,
                    },
                    Annotated {
                        raw: RawResource {
                            uri: "terraform://best-practices".into(),
                            name: "Terraform Best Practices".into(),
                            description: Some("Security and operational best practices".into()),
                            mime_type: Some("text/markdown".into()),
                            title: None,
                            size: None,
                            icons: None,
                            meta: None,
                        },
                        annotations: None,
                    },
                ],
                ..Default::default()
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let content = match request.uri.as_str() {
                "terraform://style-guide" => TERRAFORM_STYLE_GUIDE,
                "terraform://module-development" => TERRAFORM_MODULE_DEVELOPMENT,
                "terraform://best-practices" => TERRAFORM_BEST_PRACTICES,
                _ => {
                    return Err(McpError::resource_not_found(
                        format!("Unknown resource: {}", request.uri),
                        None,
                    ));
                }
            };

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, request.uri)],
            })
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            let tools = self.tool_router.list_all();
            Ok(ListToolsResult {
                tools,
                ..Default::default()
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let tool_context =
                rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            self.tool_router.call(tool_context).await
        }
    }
}
