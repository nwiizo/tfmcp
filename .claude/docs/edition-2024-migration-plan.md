# Rust Edition 2024 移行計画書

## 概要

tfmcpプロジェクトをRust Edition 2021からEdition 2024へ移行するための計画書です。

| 項目 | 現状 | 移行後 |
|------|------|--------|
| Edition | 2021 | 2024 |
| rust-version (MSRV) | 1.78.0 | 1.85.0 |
| Rustリリース日 | - | 2025年2月20日 |

## 移行が必要な変更点

### 1. `std::env::set_var`のunsafe化 ⚠️ 必須修正

**影響箇所**: `src/core/tfmcp.rs:361`

```rust
// 現在のコード
std::env::set_var(
    "TERRAFORM_DIR",
    project_directory.to_string_lossy().to_string(),
);
```

**理由**: マルチスレッド環境での環境変数の設定は、OSレベルでスレッドセーフではないため、Edition 2024からunsafe関数として扱われます。

**対応方法**:
1. `unsafe`ブロックで囲む（推奨: シングルスレッド初期化時のみ使用されるため）
2. 環境変数の代わりに設定構造体を使用するようリファクタリング

```rust
// 修正案1: unsafe使用
// SAFETY: この関数は初期化時にシングルスレッドで呼び出される
unsafe {
    std::env::set_var(
        "TERRAFORM_DIR",
        project_directory.to_string_lossy().to_string(),
    );
}

// 修正案2: 環境変数を使わない設計へ変更
// TerraformServiceの内部状態として保持
```

### 2. Tail Expression Drop Order ⚠️ 動作確認必要

**影響箇所**: 8箇所

| ファイル | 行番号 | 内容 |
|----------|--------|------|
| `src/registry/client.rs` | 470 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 611 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 722 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 880 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 1118 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 1291 | debug!マクロ内の一時変数 |
| `src/registry/client.rs` | 1422 | debug!マクロ内の一時変数 |
| `src/terraform/service.rs` | 526 | match式内のエラーハンドリング |

**理由**: Edition 2024では、ブロックの末尾式の一時変数がローカル変数より先にドロップされるようになります。

**対応方法**:
- debug!マクロ内の警告: 動作に影響なし（ログ出力のみ）
- `src/terraform/service.rs:526`: anyhow::Errorのドロップ順序変更だが、実際の動作に影響なし

```rust
// 現在のコード（警告あり）
debug!(
    "Search response (first 1000 chars): {}",
    &response_text.chars().take(1000).collect::<String>()
);

// 修正案（明示的に一時変数を保持）
let preview: String = response_text.chars().take(1000).collect();
debug!("Search response (first 1000 chars): {}", preview);
```

### 3. Preludeの変更 ✅ 影響なし

Edition 2024では`Future`と`IntoFuture`がpreludeに追加されます。

**確認結果**: tfmcpでは同名のトレイトを定義していないため、影響なし。

### 4. `IntoIterator for Box<[T]>` ✅ 影響なし

**確認結果**: `Box<[T]>`に対する`.into_iter()`呼び出しはありません。

### 5. `gen`キーワードの予約 ✅ 影響なし

**確認結果**: `gen`という識別子は使用されていません。

### 6. `macro_rules!`の`expr`フラグメント変更 ✅ 影響なし

**確認結果**: プロジェクト内でマクロ定義はありません。

## 移行手順

### Phase 1: 準備（見積もり: 小規模）

1. **依存クレートの確認**
   ```bash
   cargo update
   ```
   現在の依存クレートはEdition 2024対応済み

2. **現環境でのテスト実行**
   ```bash
   cargo test --locked --all-features
   ```

### Phase 2: コード修正

1. **`std::env::set_var`の修正**
   - `src/core/tfmcp.rs:361`を`unsafe`ブロックで囲む
   - SAFETYコメントを追加

2. **Tail expression警告の解消（オプション）**
   - debug!マクロ内の一時変数を明示的にローカル変数に格納
   - 8箇所すべてを修正

### Phase 3: Edition更新

1. **Cargo.tomlの更新**
   ```toml
   edition = "2024"
   rust-version = "1.85.0"
   ```

2. **cargo fixによる自動修正**
   ```bash
   cargo fix --edition --allow-dirty
   ```

3. **ビルド確認**
   ```bash
   cargo fmt --all
   RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --all-features
   cargo test --locked --all-features
   ```

### Phase 4: CI/CD更新

1. **GitHub Actions更新**: Rust 1.85.0以上を使用するよう設定

2. **MSRVテストの追加**: rust-version = "1.85.0"のテスト追加

## リスク評価

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|----------|------|
| `set_var` unsafe化によるバグ | 低 | 低 | 初期化時のみ使用、シングルスレッド |
| Drop順序変更によるバグ | 低 | 低 | debug出力のみで動作に影響なし |
| 依存クレートの非互換 | 低 | 低 | 主要クレートは対応済み |

## 修正が必要なファイル一覧

| ファイル | 変更種別 | 優先度 |
|----------|----------|--------|
| `Cargo.toml` | edition, rust-version更新 | 必須 |
| `src/core/tfmcp.rs` | unsafe追加 | 必須 |
| `src/registry/client.rs` | tail-expr警告解消 | 任意 |
| `src/terraform/service.rs` | tail-expr警告解消 | 任意 |
| `.github/workflows/*.yml` | Rust version更新 | 必須 |

## 参考資料

- [Announcing Rust 1.85.0 and Rust 2024](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
- [Rust 2024 Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
- [Newly Unsafe Functions](https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html)
- [Tail Expression Temporary Scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html)
- [Migration Guide](https://doc.rust-lang.org/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html)
- [Tracking Issue: set_var unsafe](https://github.com/rust-lang/rust/issues/124866)

## 結論

tfmcpのEdition 2024への移行は比較的シンプルです。必須の変更は2点のみ：

1. `Cargo.toml`のeditionとrust-version更新
2. `std::env::set_var`を`unsafe`ブロックで囲む

その他の警告はログ出力に関するもので、動作に影響しないため任意対応です。
