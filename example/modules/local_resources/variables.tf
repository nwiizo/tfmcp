variable "name_length" {
  description = "生成するランダムペット名の長さ"
  type        = number
  default     = 2
}

variable "separator" {
  description = "ペット名の単語を区切る文字"
  type        = string
  default     = "-"
}

variable "prefix" {
  description = "ペット名の接頭辞"
  type        = string
  default     = null
}

variable "output_dir" {
  description = "ファイルを出力するディレクトリ"
  type        = string
  default     = "output"
}

variable "tags" {
  description = "リソースに追加するタグ"
  type        = map(string)
  default     = {}
}

variable "metadata" {
  description = "追加のメタデータ"
  type        = map(string)
  default     = {}
} 