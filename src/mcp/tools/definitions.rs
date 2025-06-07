/// Tool definitions for MCP protocol
/// This module contains the JSON schema definitions for all available tools
pub const TOOLS_JSON: &str = r#"{
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