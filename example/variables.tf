variable "file_prefix" {
  description = "作成されるファイルの接頭辞"
  type        = string
  default     = "tfmcp"
}

variable "pet_length" {
  description = "ペット名の長さ"
  type        = number
  default     = 2
  validation {
    condition     = var.pet_length > 1 && var.pet_length < 5
    error_message = "ペット名の長さは2-4の間でなければなりません。"
  }
}

variable "priority_max" {
  description = "優先度の最大値"
  type        = number
  default     = 100
}

variable "tags" {
  description = "リソースに追加するタグ"
  type        = map(string)
  default     = {
    Environment = "development"
    Project     = "tfmcp-demo"
    Owner       = "tfmcp-user"
  }
} 