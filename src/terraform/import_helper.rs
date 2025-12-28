//! Terraform import helper for importing existing resources.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Import preview information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub resource_address: String,
    pub resource_id: String,
    pub resource_type: String,
    pub suggested_config: String,
    pub warnings: Vec<String>,
}

/// Import result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub resource_address: String,
    pub resource_id: String,
    pub message: String,
    pub output: Option<String>,
}

/// Generate a preview of what would be imported
pub fn preview_import(
    resource_type: &str,
    resource_id: &str,
    name: &str,
) -> anyhow::Result<ImportPreview> {
    let resource_address = format!("{}.{}", resource_type, name);

    // Generate suggested configuration based on resource type
    let suggested_config = generate_suggested_config(resource_type, name);
    let warnings = generate_import_warnings(resource_type);

    Ok(ImportPreview {
        resource_address,
        resource_id: resource_id.to_string(),
        resource_type: resource_type.to_string(),
        suggested_config,
        warnings,
    })
}

/// Execute the import
pub fn execute_import(
    terraform_path: &Path,
    project_dir: &Path,
    resource_type: &str,
    resource_id: &str,
    name: &str,
) -> anyhow::Result<ImportResult> {
    let resource_address = format!("{}.{}", resource_type, name);

    // Run terraform import
    let output = Command::new(terraform_path)
        .arg("import")
        .arg(&resource_address)
        .arg(resource_id)
        .current_dir(project_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(ImportResult {
            success: true,
            resource_address,
            resource_id: resource_id.to_string(),
            message: "Resource imported successfully".to_string(),
            output: Some(stdout.to_string()),
        })
    } else {
        // Parse common error messages
        let message = if stderr.contains("Cannot import non-existent remote object") {
            format!(
                "Resource with ID '{}' does not exist in the cloud",
                resource_id
            )
        } else if stderr.contains("Resource already managed by Terraform") {
            format!(
                "Resource '{}' is already managed by Terraform",
                resource_address
            )
        } else if stderr.contains("configuration for") && stderr.contains("is not present") {
            format!(
                "No configuration found for '{}'. Add a resource block before importing.",
                resource_address
            )
        } else {
            format!("Import failed: {}", stderr)
        };

        Ok(ImportResult {
            success: false,
            resource_address,
            resource_id: resource_id.to_string(),
            message,
            output: Some(stderr.to_string()),
        })
    }
}

/// Generate suggested configuration for a resource type
fn generate_suggested_config(resource_type: &str, name: &str) -> String {
    // Provider-specific templates
    match resource_type {
        // AWS resources
        "aws_instance" => format!(
            r#"resource "{}" "{}" {{
  # Required attributes after import:
  ami           = "ami-xxxxxxxx"  # Update with actual AMI ID
  instance_type = "t3.micro"      # Update with actual instance type

  # Optional: Add tags, VPC settings, etc.
  tags = {{
    Name = "{}"
  }}
}}"#,
            resource_type, name, name
        ),
        "aws_s3_bucket" => format!(
            r#"resource "{}" "{}" {{
  # The bucket name will be imported from the resource ID
  # Add any additional configuration as needed

  tags = {{
    Name = "{}"
  }}
}}"#,
            resource_type, name, name
        ),
        "aws_security_group" => format!(
            r#"resource "{}" "{}" {{
  name        = "{}"
  description = "Imported security group"

  # Ingress and egress rules will need to be added manually
  # after running 'terraform plan' to see the current state
}}"#,
            resource_type, name, name
        ),
        "aws_vpc" => format!(
            r#"resource "{}" "{}" {{
  cidr_block = "10.0.0.0/16"  # Update with actual CIDR

  tags = {{
    Name = "{}"
  }}
}}"#,
            resource_type, name, name
        ),
        "aws_subnet" => format!(
            r#"resource "{}" "{}" {{
  vpc_id     = aws_vpc.main.id  # Update with actual VPC reference
  cidr_block = "10.0.1.0/24"    # Update with actual CIDR

  tags = {{
    Name = "{}"
  }}
}}"#,
            resource_type, name, name
        ),
        "aws_db_instance" => format!(
            r#"resource "{}" "{}" {{
  identifier        = "{}"
  instance_class    = "db.t3.micro"  # Update with actual instance class
  engine            = "mysql"         # Update with actual engine
  allocated_storage = 20              # Update with actual storage

  # Password must be specified or use manage_master_user_password
  # username = "admin"
  # password = "..."

  skip_final_snapshot = true  # Set to false in production
}}"#,
            resource_type, name, name
        ),

        // Google Cloud resources
        "google_compute_instance" => format!(
            r#"resource "{}" "{}" {{
  name         = "{}"
  machine_type = "e2-medium"  # Update with actual machine type
  zone         = "us-central1-a"

  boot_disk {{
    initialize_params {{
      image = "debian-cloud/debian-11"
    }}
  }}

  network_interface {{
    network = "default"
  }}
}}"#,
            resource_type, name, name
        ),
        "google_storage_bucket" => format!(
            r#"resource "{}" "{}" {{
  name     = "{}"
  location = "US"  # Update with actual location

  force_destroy = false
}}"#,
            resource_type, name, name
        ),

        // Azure resources
        "azurerm_virtual_machine"
        | "azurerm_linux_virtual_machine"
        | "azurerm_windows_virtual_machine" => format!(
            r#"resource "{}" "{}" {{
  name                = "{}"
  resource_group_name = "my-resource-group"  # Update
  location            = "East US"             # Update
  size                = "Standard_B1s"        # Update

  # Additional required attributes depend on the VM type
}}"#,
            resource_type, name, name
        ),
        "azurerm_storage_account" => format!(
            r#"resource "{}" "{}" {{
  name                     = "{}"
  resource_group_name      = "my-resource-group"  # Update
  location                 = "East US"             # Update
  account_tier             = "Standard"
  account_replication_type = "LRS"
}}"#,
            resource_type, name, name
        ),

        // Generic template for unknown resource types
        _ => format!(
            r#"resource "{}" "{}" {{
  # Add required attributes for this resource type
  # Run 'terraform plan' after import to see the current state
  # and identify any required attributes that are missing
}}"#,
            resource_type, name
        ),
    }
}

/// Generate warnings for import based on resource type
fn generate_import_warnings(resource_type: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Common warning
    warnings.push(
        "After import, run 'terraform plan' to see if your configuration matches the imported state"
            .to_string(),
    );

    // Resource-specific warnings
    match resource_type {
        "aws_db_instance" | "google_sql_database_instance" | "azurerm_sql_database" => {
            warnings.push(
                "Database passwords are not imported - you may need to update your configuration"
                    .to_string(),
            );
        }
        "aws_instance" | "google_compute_instance" | "azurerm_virtual_machine" => {
            warnings.push("Instance user data / startup scripts are not imported".to_string());
        }
        "aws_security_group" | "google_compute_firewall" | "azurerm_network_security_group" => {
            warnings.push("Review all ingress/egress rules after import for security".to_string());
        }
        "aws_iam_role" | "aws_iam_policy" | "google_project_iam_binding" => {
            warnings.push(
                "IAM resources are security-sensitive - review all permissions carefully"
                    .to_string(),
            );
        }
        "aws_s3_bucket" | "google_storage_bucket" | "azurerm_storage_account" => {
            warnings.push("Bucket policies and ACLs may need separate import".to_string());
        }
        _ => {}
    }

    // Check for data resources
    if resource_type.starts_with("data.") {
        warnings.push("Data sources cannot be imported - they are read-only".to_string());
    }

    warnings
}

/// Get import ID format hint for common resource types
#[allow(dead_code)]
pub fn get_import_id_hint(resource_type: &str) -> String {
    match resource_type {
        // AWS
        "aws_instance" => "Instance ID (e.g., i-1234567890abcdef0)".to_string(),
        "aws_s3_bucket" => "Bucket name (e.g., my-bucket-name)".to_string(),
        "aws_security_group" => "Security group ID (e.g., sg-1234567890abcdef0)".to_string(),
        "aws_vpc" => "VPC ID (e.g., vpc-1234567890abcdef0)".to_string(),
        "aws_subnet" => "Subnet ID (e.g., subnet-1234567890abcdef0)".to_string(),
        "aws_db_instance" => "DB instance identifier (e.g., my-database)".to_string(),
        "aws_iam_role" => "Role name (e.g., my-role)".to_string(),
        "aws_iam_policy" => "Policy ARN (e.g., arn:aws:iam::123456789012:policy/my-policy)".to_string(),
        "aws_lambda_function" => "Function name (e.g., my-function)".to_string(),

        // Google Cloud
        "google_compute_instance" => "projects/{project}/zones/{zone}/instances/{name}".to_string(),
        "google_storage_bucket" => "Bucket name (e.g., my-bucket)".to_string(),
        "google_compute_network" => "projects/{project}/global/networks/{name}".to_string(),

        // Azure
        "azurerm_resource_group" => "/subscriptions/{subscription_id}/resourceGroups/{name}".to_string(),
        "azurerm_virtual_machine" => "/subscriptions/{subscription_id}/resourceGroups/{rg}/providers/Microsoft.Compute/virtualMachines/{name}".to_string(),
        "azurerm_storage_account" => "/subscriptions/{subscription_id}/resourceGroups/{rg}/providers/Microsoft.Storage/storageAccounts/{name}".to_string(),

        _ => "Resource-specific ID format - check Terraform provider documentation".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_import() {
        let preview = preview_import("aws_instance", "i-12345", "web").unwrap();
        assert_eq!(preview.resource_address, "aws_instance.web");
        assert_eq!(preview.resource_id, "i-12345");
        assert!(!preview.suggested_config.is_empty());
        assert!(!preview.warnings.is_empty());
    }

    #[test]
    fn test_import_id_hint() {
        let hint = get_import_id_hint("aws_instance");
        assert!(hint.contains("Instance ID"));

        let hint = get_import_id_hint("aws_s3_bucket");
        assert!(hint.contains("Bucket name"));
    }

    #[test]
    fn test_generate_warnings() {
        let warnings = generate_import_warnings("aws_db_instance");
        assert!(warnings.iter().any(|w| w.contains("password")));

        let warnings = generate_import_warnings("aws_security_group");
        assert!(warnings.iter().any(|w| w.contains("security")));
    }
}
