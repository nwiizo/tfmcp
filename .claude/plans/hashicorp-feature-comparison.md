# HashiCorp terraform-mcp-server機能差分分析と実装計画

## 参照
- [HashiCorp terraform-mcp-server](https://github.com/hashicorp/terraform-mcp-server)
- [MCP Server Reference](https://developer.hashicorp.com/terraform/docs/tools/mcp-server/reference)

## 現状比較

### tfmcp (21ツール)
| カテゴリ | ツール |
|---------|--------|
| ローカルTerraform | init, plan, apply, destroy, validate, state |
| 分析 | analyze, module_health, dependency_graph, refactoring |
| セキュリティ | security_status, secret_detection, guideline_checks |
| レジストリ | search_providers, get_provider_info/docs, search_modules, get_module_details |

### HashiCorp (30+ツール)
| カテゴリ | ツール |
|---------|--------|
| Provider | search_providers, get_provider_details, get_latest_provider_version |
| Module | search_modules, get_module_details, get_latest_module_version |
| **Policy (NEW)** | search_policies, get_policy_details |
| **HCP Terraform (NEW)** | list_orgs, list_projects, workspaces CRUD, runs, variables |
| **Variable Sets (NEW)** | list, create, attach/detach variable sets |
| **Tags (NEW)** | create_workspace_tags, read_workspace_tags |

## 機能差分

### tfmcp独自機能 ✅
1. **ローカルTerraform実行** - plan/apply/destroy/init
2. **モジュールヘルス分析** - cohesion/coupling metrics
3. **リソース依存グラフ** - 可視化
4. **リファクタリング提案** - 具体的な改善策
5. **ガイドライン準拠チェック** - Future Architect準拠
6. **シークレット検出** - AWS keys, API tokens
7. **セキュリティステータス** - 監査ログ

### HashiCorp独自機能 (未実装)
1. **Sentinel Policy Tools** - ポリシー検索・詳細取得
2. **HCP Terraform統合** - ワークスペース/Run管理
3. **Variable Sets** - 変数セット管理
4. **タグ管理** - ワークスペースタグ
5. **Private Registry** - プライベートモジュール/プロバイダー

---

## 実装計画

### Phase 1: Sentinel Policy Tools (優先度: 高)
**理由**: Registry APIへの追加のみでHCP不要

#### 新規ツール
1. `search_policies` - Sentinelポリシー検索
2. `get_policy_details` - ポリシー詳細取得

#### 実装内容
```rust
// src/registry/policy.rs (新規)
pub struct PolicyClient {
    client: reqwest::Client,
}

impl PolicyClient {
    pub async fn search(&self, query: &str) -> Result<Vec<Policy>> { ... }
    pub async fn get_details(&self, id: &str) -> Result<PolicyDetails> { ... }
}
```

#### 見積もり
- 新規ファイル: 1 (policy.rs)
- 変更ファイル: 3 (mod.rs, server.rs, types.rs)
- 追加行数: ~200行

---

### Phase 2: Provider Doc ID Resolution (優先度: 中)
**理由**: HashiCorpと同様のdoc検索パターン

#### 改善内容
`get_provider_docs`を2段階に分割:
1. `resolve_provider_doc_id` - doc ID一覧取得
2. `get_provider_docs` - 既存(doc IDで詳細取得)

#### 見積もり
- 変更ファイル: 2 (server.rs, types.rs)
- 追加行数: ~50行

---

### Phase 3: HCP Terraform統合 (優先度: 低)
**理由**: HCPアカウント必須、認証実装必要

#### 新規ツール (段階的実装)
1. `list_terraform_orgs` - 組織一覧
2. `list_terraform_projects` - プロジェクト一覧
3. `list_workspaces` - ワークスペース一覧
4. `get_workspace_details` - 詳細取得

#### 前提条件
- HCP_TOKEN環境変数
- OAuth認証フロー

#### 見積もり
- 新規ファイル: 2 (hcp/client.rs, hcp/mod.rs)
- 追加行数: ~500行

---

## 推奨実装順序

```
v0.1.9: Phase 1 (Sentinel Policy Tools)
  - search_policies
  - get_policy_details

v0.1.10: Phase 2 (Provider Doc Resolution)
  - resolve_provider_doc_id

v0.2.0: Phase 3 (HCP Terraform - 基本)
  - list_terraform_orgs
  - list_workspaces
  - get_workspace_details
```

---

## 見送り機能

| 機能 | 理由 |
|------|------|
| Variable Sets管理 | HCP専用、書き込み操作 |
| ワークスペースCRUD | 破壊的操作、HCP専用 |
| Run管理 | HCP専用、複雑な状態管理 |
| Tags管理 | HCP専用 |
| Private Registry | HCP専用 |

---

## 差別化戦略

tfmcpは以下で差別化:
1. **ローカルファースト** - Terraform CLI直接実行
2. **分析重視** - モジュールヘルス、依存グラフ
3. **セキュリティ重視** - シークレット検出、ガイドライン準拠
4. **軽量** - HCP不要で即使用可能

HashiCorpはHCP/TFE統合に特化しているため、ローカル開発ワークフローで差別化。
