terraform {
  required_version = ">= 1.6.0"
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.5"
    }
  }
}

variable "release" {
  type    = string
  default = "v1"
  validation {
    condition     = can(regex("^v[0-9]+$", var.release))
    error_message = "Release must be v followed by a number."
  }
}

resource "local_file" "page" {
  filename        = "${path.module}/site/index.html"
  file_permission = "0644"
  content = templatefile("${path.module}/index.html.tftpl", {
    release = var.release
  })
}

output "release" {
  value = var.release
}

output "page_path" {
  value = local_file.page.filename
}
