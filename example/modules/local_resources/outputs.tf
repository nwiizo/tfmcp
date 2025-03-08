output "resource_name" {
  description = "生成されたリソース名"
  value       = random_pet.name.id
}

output "config_file_path" {
  description = "生成された設定ファイルのパス"
  value       = local_file.config.filename
}

output "readme_file_path" {
  description = "生成されたREADMEファイルのパス"
  value       = local_file.readme.filename
}

output "file_content_hash" {
  description = "設定ファイルの内容のハッシュ"
  value       = local_file.config.content_base64sha256
} 