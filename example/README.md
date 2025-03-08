# Terraform サンプルプロジェクト for tfmcp

このプロジェクトは、tfmcp (Terraform Model Context Protocol) ツールのデモ用のサンプルTerraformプロジェクトです。
認証情報を必要とせずにローカル環境で動作し、基本的なTerraformの機能を実演します。

## 概要

このサンプルプロジェクトでは以下のTerraformの機能を使用しています：

- ローカルファイルの生成
- ランダム値の生成 (ペット名、数値)
- モジュールの使用
- 変数とローカル値
- 出力値
- プロビジョナー
- 条件付きリソース作成

## 内容

- **main.tf**: メインの構成ファイル
- **variables.tf**: 変数定義
- **terraform.tfvars**: 変数のデフォルト値
- **modules/**: カスタムモジュール
  - **local_resources/**: ファイル生成モジュール

## 使い方

このプロジェクトは、Claude Desktopとtfmcpを使ってTerraformプロジェクトを分析、計画、適用する方法を示します。

### 基本コマンド

```bash
# 初期化
terraform init

# 計画を表示
terraform plan

# 変更を適用
terraform apply

# リソースを削除
terraform destroy
```

### tfmcpでの利用

このサンプルは、tfmcpのMCPサーバーでTerraformプロジェクトの解析と操作を行うためのデモとして使用できます。
Claude Desktopと統合することで、自然言語でTerraformプロジェクトを操作できます。

## 出力されるファイル

このTerraformプロジェクトを適用すると、以下のファイルが生成されます：

1. `example_output.txt` - 基本的なテキストファイル
2. `config/[ランダム名].json` - ランダム生成された設定ファイル
3. `environments/dev/`, `environments/staging/`, `environments/prod/` - 各環境用のリソースファイル
4. `master/` - マスターリソースファイル

## 注意事項

このサンプルプロジェクトは教育目的のみを意図しており、実際の本番環境では使用しないでください。

## Directory Structure

This example directory contains various Terraform examples demonstrating different use cases and configuration patterns:

- `/demo` - Minimal demo configuration used by tfmcp in demo mode
- `/environments` - Environment-specific configurations (dev, staging, prod)
- `/master` - Main configuration templates
- `/modules` - Reusable Terraform modules
- `/config` - Configuration files

## Getting Started

The simplest example is in the `/demo` directory, which creates a single local file:

```bash
cd demo
terraform init
terraform apply
```

For more advanced usage, explore the other directories based on your needs. 