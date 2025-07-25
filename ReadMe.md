# monthly\_file\_diff

## 概要

`monthly_file_diff` は、指定したテンプレートフォルダから年月フォルダを検出し、各フォルダ内のファイル一覧および作成日時（created）・更新日時（modified）をCSV出力するRust製コマンドラインツールです。
秒数≥30秒で分を繰り上げるExplorer形式のタイムスタンプ表示や、出力エンコーディングの指定に対応しています。

## 特徴

* `{yyyy}`, `{mm}`, `{dd}` プレースホルダ対応のフォルダテンプレート
* ファイルの作成日時・更新日時をExplorer形式（秒≥30で分繰り上げ）で出力
* CSV出力のエンコーディングを `utf8`（デフォルト）、`shift_jis`、`utf16le` から選択可能
* HTML形式のグラフ付きレポート出力に対応（Teraテンプレートエンジン使用）
* サブフォルダの最大探索深さを調整可能

## 前提条件

* Rust 1.56 以上
* Cargo
* Windows/macOS/Linux 対応
* HTMLレポート生成時は `templates/report.html` テンプレートファイルが必要

## インストール

```bash
git clone https://github.com/youruser/monthly_file_diff.git
cd monthly_file_diff
cargo build --release
```

## 使い方

```powershell
# 1) UTF-8 で出力（デフォルト）
.\target\release\monthly_file_diff.exe `
  --template "D:\data\参照{yyyy}_{mm}月データ\Main" > output.csv

# 2) SHIFT_JIS で出力
.\target\release\monthly_file_diff.exe `
  --template "D:\data\参照{yyyy}_{mm}月データ\Main" `
  --encoding shift_jis > output_sjis.csv

# 3) UTF-16LE で出力
.\target\release\monthly_file_diff.exe `
  --template "D:\data\参照{yyyy}_{mm}月データ\Main" `
  --encoding utf16le > output_utf16le.csv

# 4) 特定日付リストを指定
.\target\release\monthly_file_diff.exe `
  --template "..." `
  --dates 2025-06-01,2025-07-01 > output.csv

# 5) HTMLレポートを生成（デフォルトUTF-8）
.\target\release\monthly_file_diff.exe `
  --template "D:\data\参照{yyyy}_{mm}月データ\Main" `
  --html-file report.html

# 6) サブフォルダ探索の深さを指定
.\target\release\monthly_file_diff.exe `
  --template "..." `
  --max-depth 3 > output.csv
```

## コマンドライン引数

| オプション                       | 説明                                                       |
| --------------------------- | -------------------------------------------------------- |
| `-t, --template <TEMPLATE>` | フォルダテンプレートパス。`{yyyy}`, `{mm}`, `{dd}` プレースホルダを使用可能       |
| `-d, --dates <DATES>`       | カンマ区切りの日付リスト（例: `2025-06-01,2025-07-01`）。指定がない場合は自動検出    |
| `-e, --encoding <ENC>`      | 出力CSVのエンコーディング。`utf8`（デフォルト）、`shift_jis`、`utf16le` のいずれか |
| `--html-file <PATH>`        | HTMLレポート出力ファイル名。空文字列の場合はCSV出力のみ                           |
| `--max-depth <N>`           | サブディレクトリの最大探索深さ（デフォルト: 2）                                |


## サンプルCSV出力

```csv
normalized_rel_path,date,actual_name,size,created,modified,rel_path
InTheBox{mm}-{yyyy}.xlsx,2025-07,InTheBox12-2025.xlsx,10240,2025/07/23 10:31,2025/07/23 10:45,InTheBox12-2025.xlsx
Sub/InTheBox{mm}-{yyyy}.xlsx,2024-12,InTheBox12-2024.xlsx,8192,2024/12/15 14:22,2024/12/15 14:30,Sub/InTheBox12-2024.xlsx
```

## 出力について

### CSV出力
- `normalized_rel_path`: ファイル名部分のみ年月をプレースホルダに正規化した相対パス
- `date`: 対象年月 (YYYY-MM形式)
- `actual_name`: 実際のファイル名
- `size`: ファイルサイズ（バイト）
- `created`/`modified`: 作成日時・更新日時（Explorer形式）
- `rel_path`: 実際の相対パス

### HTML出力
テンプレート `templates/report.html` を使用してインタラクティブなチャートを生成します。ファイルごとに時系列でサイズや日時の変化をグラフ表示できます。

## ライセンス

MIT License
