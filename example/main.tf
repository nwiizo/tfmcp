terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.4.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.5.0"
    }
  }
  required_version = ">= 1.2.0"
}

# ローカルの変数を定義
locals {
  project_name = "tfmcp-demo"
  content      = "このファイルは tfmcp によって作成されました！\n現在時刻: ${timestamp()}"
  environments = ["dev", "staging", "prod"]
}

# ランダムペットネームジェネレーター
resource "random_pet" "server" {
  length    = var.pet_length
  separator = "-"
  prefix    = var.file_prefix
}

# ランダムな数値
resource "random_integer" "priority" {
  min = 1
  max = var.priority_max
}

# ローカルファイルを作成
resource "local_file" "example" {
  filename = "${path.module}/example_output.txt"
  content  = local.content
}

# ローカルディレクトリを作成
resource "local_file" "config_file" {
  filename = "${path.module}/config/${random_pet.server.id}.json"
  content  = jsonencode({
    name     = random_pet.server.id
    priority = random_integer.priority.result
    created  = timestamp()
    tags     = var.tags
  })

  # ディレクトリが存在することを確認
  provisioner "local-exec" {
    command = "mkdir -p ${path.module}/config"
  }
}

# 複数環境のリソースを作成
module "environments" {
  source = "./modules/local_resources"
  count  = length(local.environments)
  
  name_length = var.pet_length
  prefix      = local.environments[count.index]
  output_dir  = "${path.module}/environments/${local.environments[count.index]}"
  
  tags = merge(var.tags, {
    Environment = local.environments[count.index]
  })
  
  metadata = {
    description = "リソース for ${local.environments[count.index]} 環境"
    created_by  = "tfmcp"
    version     = "1.0.0"
  }
}

# マスターリソースを作成
module "master" {
  source = "./modules/local_resources"
  
  name_length = 2
  prefix      = "master"
  output_dir  = "${path.module}/master"
  
  tags = var.tags
  metadata = {
    is_master  = "true"
    created_by = "tfmcp"
    priority   = "high"
  }
}

# 出力を定義
output "pet_name" {
  value       = random_pet.server.id
  description = "生成されたペット名"
}

output "priority" {
  value       = random_integer.priority.result
  description = "ランダムな優先度"
}

output "file_path" {
  value       = local_file.example.filename
  description = "作成されたファイルのパス"
}

output "config_path" {
  value       = local_file.config_file.filename
  description = "作成された設定ファイルのパス"
}

output "environment_resources" {
  value = {
    for idx, env in local.environments : env => {
      name        = module.environments[idx].resource_name
      config_path = module.environments[idx].config_file_path
      readme_path = module.environments[idx].readme_file_path
    }
  }
  description = "各環境で作成されたリソース"
}

output "master_resource" {
  value = {
    name        = module.master.resource_name
    config_path = module.master.config_file_path
    readme_path = module.master.readme_file_path
  }
  description = "マスターリソース"
} 