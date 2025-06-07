use std::collections::HashMap;

/// Builder for creating structured tool descriptions with usage guides and constraints
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ToolDescription {
    pub summary: String,
    pub usage_guide: String,
    pub constraints: Vec<String>,
    pub error_hints: HashMap<String, String>,
    pub examples: Vec<ToolExample>,
    pub security_notes: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ToolExample {
    pub title: String,
    pub description: String,
    pub input: serde_json::Value,
    pub expected_output: String,
}

impl ToolDescription {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            usage_guide: String::new(),
            constraints: Vec::new(),
            error_hints: HashMap::new(),
            examples: Vec::new(),
            security_notes: Vec::new(),
        }
    }

    pub fn with_usage_guide(mut self, guide: impl Into<String>) -> Self {
        self.usage_guide = guide.into();
        self
    }

    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    pub fn with_error_hint(mut self, error_type: &str, hint: impl Into<String>) -> Self {
        self.error_hints.insert(error_type.to_string(), hint.into());
        self
    }

    pub fn with_example(mut self, example: ToolExample) -> Self {
        self.examples.push(example);
        self
    }

    pub fn with_security_note(mut self, note: impl Into<String>) -> Self {
        self.security_notes.push(note.into());
        self
    }

    /// Build a comprehensive prompt string for the tool
    pub fn build_prompt(&self) -> String {
        let mut prompt = self.summary.clone();

        if !self.usage_guide.is_empty() {
            prompt.push_str(&format!("\n\n## Usage Guide\n{}", self.usage_guide));
        }

        if !self.constraints.is_empty() {
            prompt.push_str("\n\n## Constraints");
            for constraint in &self.constraints {
                prompt.push_str(&format!("\n- {}", constraint));
            }
        }

        if !self.security_notes.is_empty() {
            prompt.push_str("\n\n## Security Notes");
            for note in &self.security_notes {
                prompt.push_str(&format!("\n- ⚠️ {}", note));
            }
        }

        if !self.examples.is_empty() {
            prompt.push_str("\n\n## Examples");
            for (i, example) in self.examples.iter().enumerate() {
                prompt.push_str(&format!(
                    "\n\n### Example {}: {}\n{}\n\n**Input:**\n```json\n{}\n```\n\n**Expected Output:**\n{}",
                    i + 1,
                    example.title,
                    example.description,
                    serde_json::to_string_pretty(&example.input).unwrap_or_else(|_| "{}".to_string()),
                    example.expected_output
                ));
            }
        }

        if !self.error_hints.is_empty() {
            prompt.push_str("\n\n## Troubleshooting");
            for (error_type, hint) in &self.error_hints {
                prompt.push_str(&format!("\n- **{}**: {}", error_type, hint));
            }
        }

        prompt
    }

    /// Build a shorter prompt for space-constrained contexts
    pub fn build_compact_prompt(&self) -> String {
        let mut prompt = self.summary.clone();

        if !self.constraints.is_empty() {
            prompt.push_str(" Constraints: ");
            prompt.push_str(&self.constraints.join(", "));
        }

        if !self.security_notes.is_empty() {
            prompt.push_str(" ⚠️ Security: ");
            prompt.push_str(&self.security_notes.join("; "));
        }

        prompt
    }
}

/// Builder for creating comprehensive MCP tool definitions
#[allow(dead_code)]
pub struct McpToolBuilder {
    name: String,
    description: ToolDescription,
    input_schema: serde_json::Value,
    output_schema: Option<serde_json::Value>,
}

impl McpToolBuilder {
    pub fn new(name: impl Into<String>, description: ToolDescription) -> Self {
        Self {
            name: name.into(),
            description,
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            output_schema: None,
        }
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Build the complete MCP tool definition
    pub fn build(self) -> serde_json::Value {
        let mut tool = serde_json::json!({
            "name": self.name,
            "description": self.description.build_prompt(),
            "inputSchema": self.input_schema
        });

        if let Some(output_schema) = self.output_schema {
            tool["outputSchema"] = output_schema;
        }

        tool
    }

    /// Build a compact version of the tool definition
    pub fn build_compact(self) -> serde_json::Value {
        let mut tool = serde_json::json!({
            "name": self.name,
            "description": self.description.build_compact_prompt(),
            "inputSchema": self.input_schema
        });

        if let Some(output_schema) = self.output_schema {
            tool["outputSchema"] = output_schema;
        }

        tool
    }
}

/// Helper function to create common constraint messages
pub fn common_constraints() -> Vec<String> {
    vec![
        "Ensure Terraform is initialized before running operations".to_string(),
        "Validate directory permissions before executing commands".to_string(),
        "Check security policy settings for dangerous operations".to_string(),
        "Use proper namespace format (e.g., 'hashicorp/aws' or auto-fallback)".to_string(),
        "Provider search queries should be specific but not overly narrow".to_string(),
    ]
}

/// Helper function to create common error hints
pub fn common_error_hints() -> HashMap<String, String> {
    let mut hints = HashMap::new();
    hints.insert(
        "Init Required".to_string(),
        "Run 'terraform init' first to initialize the working directory".to_string(),
    );
    hints.insert(
        "Permission Denied".to_string(),
        "Check if TFMCP_ALLOW_DANGEROUS_OPS environment variable is set for apply/destroy operations".to_string(),
    );
    hints.insert(
        "Provider Not Found".to_string(),
        "Verify provider name and namespace, or try without specifying namespace for auto-fallback"
            .to_string(),
    );
    hints.insert(
        "Invalid Configuration".to_string(),
        "Run validation tools to check Terraform configuration syntax and semantics".to_string(),
    );
    hints
}

/// Helper function to create common security notes
pub fn common_security_notes() -> Vec<String> {
    vec![
        "Apply and destroy operations are disabled by default for safety".to_string(),
        "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable infrastructure modifications".to_string(),
        "All operations are logged to ~/.tfmcp/audit.log for security monitoring".to_string(),
        "Production directory patterns are automatically blocked".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_description_builder() {
        let desc = ToolDescription::new("Test tool for demonstrations")
            .with_usage_guide("Use this tool when you need to test something")
            .with_constraint("Only use in development environments")
            .with_error_hint("TestError", "This is how you fix test errors")
            .with_security_note("Test operations are safe");

        let prompt = desc.build_prompt();
        assert!(prompt.contains("Test tool for demonstrations"));
        assert!(prompt.contains("Usage Guide"));
        assert!(prompt.contains("Constraints"));
        assert!(prompt.contains("Security Notes"));
        assert!(prompt.contains("Troubleshooting"));
    }

    #[test]
    fn test_tool_description_compact() {
        let desc = ToolDescription::new("Test tool")
            .with_constraint("Dev only")
            .with_security_note("Safe operation");

        let compact = desc.build_compact_prompt();
        assert!(compact.contains("Test tool"));
        assert!(compact.contains("Constraints: Dev only"));
        assert!(compact.contains("⚠️ Security: Safe operation"));
    }

    #[test]
    fn test_mcp_tool_builder() {
        let desc = ToolDescription::new("Test MCP tool");
        let builder = McpToolBuilder::new("test_tool", desc);
        let tool = builder.build();

        assert_eq!(tool["name"], "test_tool");
        assert!(tool["description"]
            .as_str()
            .unwrap()
            .contains("Test MCP tool"));
        assert!(tool["inputSchema"].is_object());
    }

    #[test]
    fn test_common_helpers() {
        let constraints = common_constraints();
        assert!(!constraints.is_empty());

        let hints = common_error_hints();
        assert!(hints.contains_key("Init Required"));

        let security_notes = common_security_notes();
        assert!(!security_notes.is_empty());
    }
}
