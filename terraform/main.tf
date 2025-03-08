# This is a sample Terraform file created by tfmcp in demo mode
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

resource "local_file" "example" {
  content  = "Hello from tfmcp demo mode!"
  filename = "${path.module}/example.txt"
}
