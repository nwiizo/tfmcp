#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const glob = require('glob');

// mdcファイルとmdディレクトリの対応関係の定義
const mdcConfigurations = [
  {
    output: ".cursor/rules/000_general.mdc",
    sourceDir: "rules/general",
    header: "---\ndescription: General project guidelines for tfmcp. Always consider these guidelines when working on the project.\nglobs: *\nalwaysApply: true\n---\n\n",
    filePattern: "*.md",
    sortBy: "name"
  },
  {
    output: ".cursor/rules/001_rust.mdc",
    sourceDir: "rules/rust",
    header: "---\ndescription: Rust coding standards and architecture guidelines for tfmcp.\nglobs: **/*.rs\nalwaysApply: false\n---\n\n",
    filePattern: "*.md",
    sortBy: "name"
  },
  {
    output: ".cursor/rules/002_terraform.mdc",
    sourceDir: "rules/terraform",
    header: "---\ndescription: Terraform standards and best practices for tfmcp.\nglobs: **/*.tf, **/*.tfvars, **/*.hcl\nalwaysApply: false\n---\n\n",
    filePattern: "*.md",
    sortBy: "name"
  },
  {
    output: ".cursor/rules/003_mcp.mdc",
    sourceDir: "rules/mcp",
    header: "---\ndescription: MCP protocol implementation guidelines and standards.\nglobs: **/mcp/**/*.rs\nalwaysApply: false\n---\n\n",
    filePattern: "*.md",
    sortBy: "name"
  }
];

// ファイル名から数字プレフィックスを抽出してソートするための関数
function extractNumberPrefix(filename) {
  const match = filename.match(/^(\d+)_/);
  return match ? parseInt(match[1], 10) : Infinity;
}

// mdファイルを検索して結合する関数
async function buildMdcFile(config) {
  // ルートディレクトリの取得（スクリプトの実行場所から相対パスで計算）
  const rootDir = process.cwd();
  
  // mdファイルのパターンを作成
  const pattern = path.join(rootDir, config.sourceDir, config.filePattern);
  
  // mdファイルを検索
  const files = glob.sync(pattern);
  
  // ファイル名でソート
  files.sort((a, b) => {
    const numA = extractNumberPrefix(path.basename(a));
    const numB = extractNumberPrefix(path.basename(b));
    return numA - numB;
  });
  
  // コンテンツの初期化
  let content = '';
  
  // ヘッダー情報を追加
  content += config.header;
  
  // 各mdファイルの内容を結合
  for (const file of files) {
    console.log(`Processing file: ${file}`);
    const fileContent = fs.readFileSync(file, 'utf8');
    content += fileContent + '\n\n';
  }
  
  // mdcファイルを出力
  const outputPath = path.join(rootDir, config.output);
  
  // 出力ディレクトリが存在することを確認
  const outputDir = path.dirname(outputPath);
  try {
    fs.mkdirSync(outputDir, { recursive: true });
  } catch (error) {
    // ディレクトリが既に存在する場合は無視
  }
  
  // ファイルに書き込み
  fs.writeFileSync(outputPath, content);
  
  console.log(`Generated ${config.output} from ${files.length} files in ${config.sourceDir}`);
}

// 既存のMDCファイルの中身を空にする関数
function cleanMdcFiles() {
  const rootDir = process.cwd();
  
  // .cursor/rules ディレクトリの存在確認
  const rulesDir = path.join(rootDir, '.cursor/rules');
  try {
    fs.accessSync(rulesDir);
  } catch (error) {
    // ディレクトリが存在しない場合は作成
    fs.mkdirSync(rulesDir, { recursive: true });
    return;
  }
  
  // .mdc ファイルを検索して中身を空にする
  const mdcFiles = glob.sync(path.join(rulesDir, '*.mdc'));
  for (const file of mdcFiles) {
    console.log(`Clearing content of MDC file: ${file}`);
    fs.writeFileSync(file, ''); // ファイルの中身を空にする
  }
}

// メイン処理
async function main() {
  try {
    // 既存のMDCファイルを削除
    cleanMdcFiles();
    
    // 各設定に対してmdcファイルを生成
    for (const config of mdcConfigurations) {
      await buildMdcFile(config);
    }
    console.log('All mdc files have been successfully generated!');
  } catch (error) {
    console.error('Error generating mdc files:', error);
    process.exit(1);
  }
}

// スクリプトの実行
main(); 