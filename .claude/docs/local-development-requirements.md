# ローカルTerraform開発で必要な機能

調査日: 2026-10-02。tfmcp v0.2.3と、v0.2.4に向けた作業中の実装を対象にした機能評価。
優先度と機能案は、以下の資料とコードを照合した本プロジェクトでの判断であり、書籍の機能一覧ではない。

## 調査対象と結論

ローカルに取得済みの次の章を確認した。書籍本文は転載していない。

| 資料 | 確認箇所 | 機能検討への示唆 |
|---|---|---|
| terraform-in-depth、`66bcabd11cba5bcbfcfdda4fd8b26ab222c60b0b` | `content/en/05.md`、`06.md` | 実行入力、planの種類、ロック、stateと実環境の差分を扱う |
| 同上 | `content/en/07.md` §7.2–7.6 | 開発環境とCIで同じ検査を再現し、整形・構文検証・lint・セキュリティ検査を区別する |
| 同上 | `content/en/09.md` §9.2、ネイティブテスト、リファクタリング | examplesを試験対象にし、準備・検証・片付けまで観測する |
| terraform-at-scale、`669ad7b437d2832027c109e2a945ec0f8410b0f7` | `content/en/ch05.md`、`ch06.md`、`ch08.md` | root module、共有module、state境界と変更の責任範囲を意識する |
| 同上 | `content/en/ch10.md`、`ch11.md` | 静的検査と実環境での動作検証を区別し、整理・分割後に意図しない再作成がないことを確認する |

特に優先したいのは、**変更前に開発環境を準備し、短い検査を繰り返し、必要な試験だけを実行できること**。
tfmcpには既にplan/applyや解析の機能があるが、検証専用の初期化とTerraformネイティブテストの実行支援が不足している。
書籍の版による差を避けるため、CLIの具体的な動作はHashiCorp公式資料と照合した。今回の実行検証の基準はTerraform 1.15.8。

## ジョブと必要な機能

P0は実行結果と秘密情報の扱いに関する前提、P1は日常のローカル開発に優先して追加する機能、P2は規模や運用に応じて追加する機能。
「既存」はv0.2.3、「今回」はv0.2.4向けの変更、「追加候補」は未実装を表す。

| 優先度 | 開発者が達成したいこと | 必要な機能 | tfmcpの現状 | 完了を判断する観測結果 |
|---|---|---|---|---|
| P1 | 初めて開いたリポジトリで、どこを検証すればよいか知る | root/module/examples/tests、required_version、provider/module依存、選択中のworkspaceの一覧 | entrypoint検出・project inspectionは既存。試験対象と依存の対応付けは追加候補 | 複数rootを勝手に選ばず、対象と検出根拠を返す |
| P1 | backendの認証前でもmoduleを検証する | 検証用init、独立したTF_DATA_DIR、非対話実行、依存取得失敗の診断 | 通常のinitは既存。検証専用モードは追加候補 | 既存のbackend設定とstateを変更せずvalidateを実行できる |
| P1 | 編集直後に修正箇所を知る | fmtの確認、validateの構造化診断、ファイルと行番号、未実行・失敗・成功の区別 | fmtとvalidateは個別に既存。集約品質チェックはfmtを実行しない | 整形不一致・構文エラー・初期化不足を別々に返し、確認モードでファイルを変えない |
| P1 | moduleの変更が利用者を壊さないか確かめる | `.tftest.hcl`/`.tftest.json`検出、対象選択、terraform test、assertion結果、cleanup結果 | ネイティブテスト実行は追加候補 | 成功・assertion失敗・実行失敗・片付け失敗を区別し、残存物を報告する |
| P1 | 手元とCIで同じ結果を得る | 実行対象・CLI版・検査設定・終了状態の記録、既存MakefileやCIとの対応、任意のlint連携 | 品質レポートは既存。既存ツールとの実行条件の統一は追加候補 | 同じcommit・入力・依存で同じ検査が走り、未導入ツールを合格扱いしない |
| P1 | どの入力と依存で実行するか把握する | tfvars・環境変数・指定var-fileの出所、優先順位、不足入力、lockfileとmodule versionの確認 | var_files指定、lockfile検査は既存。入力全体の出所表示は追加候補 | 値を漏らさず出所を説明し、依存更新を通常のinitと区別する |
| P0 | AIへ実行結果を渡しても秘密情報を漏らさない | sensitive outputの伏せ字、単一outputでも機密性保持、診断・state・planの情報制限 | plan解析の伏せ字は既存。output取得の漏えいは今回修正対象 | all/name指定の両経路で機密値が応答に現れない |
| P0 | 確認した変更だけを適用・削除する | 保存planのID共有、対象照合、明示的な実行許可、destroy planの確認 | 保存planは既存。destroy対応は今回 | plan生成後の設定編集が適用内容を変えず、削除権限不足なら両実行経路で拒否する |
| P0 | 失敗後に二重実行せず状況を確かめる | failedとoutcome_unknownの区別、結果保持、stateと実環境を確認する案内、再実行防止 | 今回追加。保持はサーバーの生存期間内 | 部分適用・timeout・中断を成功扱いせず、同じIDを再適用しない |
| P1 | 不要なplanを整理して作業を続ける | 保存planの一覧・破棄、容量制限の説明 | 今回追加 | 破棄後のIDが使えず、stateと他のplanを変えずに保存枠を空ける |
| P2 | リファクタリングで既存リソースを作り直さない | moved/import/removedの変更支援、アドレス変更の確認、移行前後planの評価 | importやrefactoring提案は既存。宣言的移行の一連の支援は追加候補 | 意図しないdelete/replaceを検出し、「変更なし」や意図した移動を確認できる |
| P2 | 実環境とのずれを確認して修正方針を選ぶ | refresh-only plan、設定・state・実環境の比較、設定反映か実環境修正かの判断材料 | refresh-only保存planは既存。drift候補の推測と実際の観測を区別する説明を強化する | stateを読むだけで「実環境と一致」と断定せず、観測時点と未確認箇所を返す |
| P2 | 複数rootの変更影響と長時間実行を追う | module利用先と試験対象の対応、進捗・中断・再起動後の結果照会 | 一部の依存解析は既存。横断的影響分析、永続ジョブ管理は追加候補 | 変更したmoduleの利用先を説明し、中断後も残存処理・結果不明を追跡できる |

## ローカル開発の流れ

1. **対象を選ぶ。** root module、再利用module、examples、testsを確認する。CLI workspaceの切替を、認証や環境隔離の代わりにしない。
2. **検証環境を準備する。** 専用のTF_DATA_DIRで`terraform init -backend=false -input=false`を実行する。provider/module取得には通信が必要な場合がある。既存のlockfileを維持する場合はreadonly方針を明示する。
3. **短い検査を繰り返す。** fmtの確認、validate、プロジェクト採用済みのlint・セキュリティ検査を行う。validateだけで実際の入力・権限・接続先の妥当性が確認できたとは扱わない。
4. **必要なテストを実行する。** tests/examplesから対象を絞り、planだけの試験かapplyを伴う試験かを確認する。apply試験は専用の認証・命名・実行範囲で行い、最後に片付けの結果を確認する。
5. **実行環境のplanを確認する。** 対象のbackend・workspace・入力で保存planを作り、確認後に同じIDを適用する。失敗時は先にstate・実環境・残存処理を照合し、新しいplanで次の変更を判断する。

`-backend=false`はbackend初期化を省略する指定で、既存の初期化情報を消去したり、以降の全コマンドをオフラインにする機能ではない。
検証用と実行用の作業データを分ける提案は、この混同を防ぐためのtfmcpの設計判断。
公式資料: [validate](https://developer.hashicorp.com/terraform/cli/commands/validate)、[init](https://developer.hashicorp.com/terraform/cli/commands/init)、[backend](https://developer.hashicorp.com/terraform/language/backend)。

Terraformのtestは既定でapplyを伴う。`command = plan`でもproviderやdata sourceに応じた接続・処理があり得るため、単純に「無副作用」と表示しない。
testの成功とcleanupの成功は分けて記録する。tfmcp自身の回帰テストでは、プロジェクト方針に従ってモックを使わず、組み込みterraform_dataや実Terraformと一時ディレクトリを使う。
公式資料: [testコマンド](https://developer.hashicorp.com/terraform/cli/commands/test)、[testの構成](https://developer.hashicorp.com/terraform/language/tests)。

依存の再現性ではproviderとmoduleを分ける。`.terraform.lock.hcl`が固定するのはprovider依存であり、remote moduleの選択はsource/version指定も確認する。
公式資料: [Dependency Lock File](https://developer.hashicorp.com/terraform/language/files/dependency-lock)。

リファクタリングでは、利用中のmodule versionからの移行を考慮し、movedブロックの保持とplanでの差分確認を優先する。
state mvを無条件に実行する機能だけでは、利用者側の移行確認を満たせない。
公式資料: [Refactor modules](https://developer.hashicorp.com/terraform/language/modules/develop/refactoring)。

`terraform output -json`はsensitive値も平文で出力するため、MCPの応答前に伏せる必要がある。
公式資料: [output](https://developer.hashicorp.com/terraform/cli/commands/output)。

## HashiCorp MCPとの役割分担

HashiCorp terraform-mcp-serverのtool登録をcommit
`d8fd44d71426ccdc8f90d907208a71d2c864f1f6`で確認した。
Registryのprovider/module/policy検索、Private Registry、HCP Terraform/TFEのorganization・workspace・run・variables・policies・stacksなどを扱う。
確認したtool登録にはローカルTerraform CLIのfmt/validate/test/plan/apply実行はない。
stdioでサーバーをローカル起動できることと、ローカルのTerraformコードを実行できることは区別する。
出典: [tool登録の実装](https://github.com/hashicorp/terraform-mcp-server/blob/d8fd44d71426ccdc8f90d907208a71d2c864f1f6/pkg/toolsets/registry.go)。

したがってtfmcpでは、ローカル開発の準備・検査・試験・確認済みplanの実行を優先する。
HCPのteam管理や全管理APIの追従は、このジョブを満たすための必須条件ではない。
また、独自HCL評価器、全検査ツールの強制導入、常用の`-target`、自動force-unlockやstate pushは今回の追加候補に含めない。

## 実装順序と範囲

今回のリリースでは、合意済みの保存destroy plan、保存planの一覧・破棄、適用結果と復旧案内を仕上げ、調査で判明したoutputの機密値漏えいを修正する。
新たに抽出した候補は、次の順で進めるのが妥当と判断した。

1. 検証専用initと、既存fmt/validateを活用する品質チェック。
2. Terraformネイティブテストの対象選択・実行・cleanup結果の報告。
3. 入力の出所、依存固定、ローカルとCIの実行条件の記録。
4. moved/import/removedによる移行と実際のdrift確認。
5. 複数rootの影響分析と永続的な実行結果管理。

今回の結果保持はメモリ内であり、サーバー再起動後の復旧機能ではない。
適用後の`state_verified`は管理リソースのアドレス照合で、属性値・output・アプリケーションの正常性を検証するものではない。
品質レポート内の独自ルールやmodule healthは助言であり、TFLint・セキュリティスキャナ・terraform testの実行結果として表示しない。
