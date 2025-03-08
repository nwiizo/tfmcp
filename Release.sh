#!/bin/bash

# バージョン番号を引数として受け取る
VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v1.0.0"
    exit 1
fi

# 前回のタグを取得
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

# リリースノートを生成
generate_release_notes() {
    local from_tag="$1"
    local to_ref="HEAD"
    local notes=""

    if [ -z "$from_tag" ]; then
        notes="## 🎉 Initial Release"
    else
        notes="## 🚀 Changes since $from_tag\n\n"
        
        # コミットを分類してリリースノートを生成
        notes+="### ✨ New Features\n"
        notes+=$(git log "$from_tag..$to_ref" --pretty=format:"- %s" --grep="^feat:" || echo "None")
        notes+="\n\n### 🐛 Bug Fixes\n"
        notes+=$(git log "$from_tag..$to_ref" --pretty=format:"- %s" --grep="^fix:" || echo "None")
        notes+="\n\n### 📚 Documentation\n"
        notes+=$(git log "$from_tag..$to_ref" --pretty=format:"- %s" --grep="^docs:" || echo "None")
        notes+="\n\n### 🔧 Maintenance\n"
        notes+=$(git log "$from_tag..$to_ref" --pretty=format:"- %s" --grep="^chore:" || echo "None")
    fi

    echo -e "$notes"
}

# Cargoにログインしているか確認
if ! cargo login --help &>/dev/null; then
    echo "Error: Please login to crates.io first using 'cargo login'"
    echo "You can find your API token at https://crates.io/me"
    exit 1
fi

# Cargo.tomlのバージョンを更新
# sed -i "s/^version = .*/version = \"${VERSION#v}\"/" Cargo.toml
# mac os では
sed -i '' "s/^version = .*/version = \"${VERSION#v}\"/" Cargo.toml

# cargo fmt を実行してフォーマットを整える
cargo fmt || exit 1

# cargo clippyを実行してコードをチェック
cargo clippy --all-targets --all-features -- -D warnings || exit 1

# cargo updateを実行してCargo.lockを更新
cargo update || exit 1

# 変更をコミット
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION"

# ビルドとテストを実行
cargo build --release || exit 1
cargo test || exit 1

# Cargoのタグを作成
cargo package --allow-dirty || exit 1

# リリースノートを生成
RELEASE_NOTES=$(generate_release_notes "$LAST_TAG")

# gitタグを作成
git tag -a "$VERSION" -m "Release $VERSION"

# GitHubリリースを作成
gh release create "$VERSION" \
    --title "Release $VERSION" \
    --notes "$RELEASE_NOTES" \
    --draft \
    target/package/*

# crates.ioにパブリッシュ
echo "Publishing to crates.io..."
cargo publish --allow-dirty || {
    echo "Failed to publish to crates.io"
    exit 1
}

# リモートにプッシュ
git push origin main
git push origin "$VERSION"

echo "Successfully:"
echo "- Updated version to $VERSION"
echo "- Created GitHub release with auto-generated notes"
echo "- Published to crates.io"
echo "- Pushed tags to origin"

