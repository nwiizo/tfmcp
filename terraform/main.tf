# This is a sample Terraform file created by tfmcp
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

resource "local_file" "example" {
  content  = "Hello from tfmcp!"
  filename = "${path.module}/example.txt"
}
