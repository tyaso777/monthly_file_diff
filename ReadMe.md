# monthly\_file\_diff

## 概要

`monthly_file_diff` は、指定したテンプレートフォルダから年月フォルダを検出し、各フォルダ内のファイル一覧および作成日時（created）・更新日時（modified）をCSV出力するRust製コマンドラインツールです。
秒数≥30秒で分を繰り上げるExplorer形式のタイムスタンプ表示や、出力エンコーディングの指定に対応しています。

## 特徴

* `{yyyy}`, `{mm}`, `{dd}` プレースホルダ対応のフォルダテンプレート
* ファイルの作成日時・更新日時をExplorer形式（秒≥30で分繰り上げ）で出力
* CSV出力のエンコーディングを `utf8`（デフォルト）、`shift_jis`、`utf16le` から選択可能

## 前提条件

* Rust 1.56 以上
* Cargo
* Windows/macOS/Linux 対応

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
```

## コマンドライン引数

| オプション                       | 説明                                                            |
| --------------------------- | ------------------------------------------------------------- |
| `-t, --template <TEMPLATE>` | フォルダテンプレートパス。`{yyyy}`, `{mm}`, `{dd}` プレースホルダを使用可能            |
| `-d, --dates <DATES>`       | カンマ区切りの日付リスト（例: `2025-06-01,2025-07-01`）。指定がない場合はテンプレートから自動検出 |
| `-e, --encoding <ENC>`      | 出力CSVのエンコーディング。`utf8`（デフォルト）、`shift_jis`、`utf16le` のいずれか      |

## サンプルCSV出力

```csv
normalized_name,date,actual_name,size,created,modified
InTheBox{mm}-{yyyy}.xlsx,2025-07,InTheBox12-2025.xlsx,10240,2025/07/23 10:31,2025/07/23 10:45
```

## ライセンス

MIT License
