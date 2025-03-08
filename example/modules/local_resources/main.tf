resource "random_pet" "name" {
  length    = var.name_length
  separator = var.separator
  prefix    = var.prefix
}

resource "local_file" "config" {
  content  = jsonencode({
    name       = random_pet.name.id
    created_at = timestamp()
    tags       = var.tags
    metadata   = var.metadata
  })
  filename = "${var.output_dir}/${random_pet.name.id}.json"
  
  provisioner "local-exec" {
    command = "mkdir -p ${var.output_dir}"
  }
}

resource "local_file" "readme" {
  content  = <<-EOT
# ${random_pet.name.id}

このファイルは Terraform によって自動生成されました。

## 詳細情報

- 作成時刻: ${timestamp()}
- タグ: ${jsonencode(var.tags)}
- メタデータ: ${jsonencode(var.metadata)}

## 使用方法

このリソースは自動的に管理され、手動での編集は推奨されません。
  EOT
  filename = "${var.output_dir}/${random_pet.name.id}.md"
} 