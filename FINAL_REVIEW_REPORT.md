# CHINJU Protocol - 最終レビュー報告書

**実施日**: 2026年2月17日  
**対象**: /Users/sunagawa/Project/chinju-protocol/ 全ファイル（再帰検索）

---

## 1. "Draft" / "Filing Planned" /  outdated status 参照

### 要修正（ドキュメントステータスとしての "Draft"）

| ファイル | 行 | 現状 | 推奨 |
|:--|:--|:--|:--|
| `protocol/CHINJU-000-TRUST_MODEL.md` | 3 | `**Status:** Draft` | 正式版なら `Active` 等に更新検討 |
| `protocol/CHINJU-001-ARCHITECTURE.md` | (要確認) | - | - |
| `protocol/CHINJU-002-DATA_FORMATS.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-003-STATE_MACHINES.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-004-API.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-005-HARDWARE.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-006-C14_MULTI_CAPABILITY.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-007-C15_VALUE_NEURON.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-008-C16_CONTRADICTION.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/CHINJU-009-C17_SURVIVAL_ATTENTION.md` | 3 | `**Status:** Draft` | 同上 |
| `protocol/README.md` | 3 | `**Version:** 0.1.0 (Draft)` | 同上 |
| `protocol/README.md` | 135 | `[■■□□□□□□□□] 20% - Draft` | 進捗に応じて更新 |

### 除外（意図的な使用・変更不要）

- `chinju-sidecar/`, `protocol/proto/`, `protocol/schema/*.json`: `Draft` はポリシーライフサイクル enum / JSON Schema 仕様名として正しい
- `protocol/CHINJU-000-TRUST_MODEL.md` L417: "1. Draft 公開" はプロセス説明
- `protocol/CHINJU-003-STATE_MACHINES.md`: `DRAFT` は状態名

---

## 2. 特許件数・範囲の不整合（17件 C1–C17）

### 要修正

| ファイル | 行 | 現状 | 正しい表記 |
|:--|:--|:--|:--|
| `papers/chinju_framework.tex` | 54 | `15 complementary patent technologies` | `17 complementary patent technologies` |
| `papers/chinju_framework.tex` | 840 | `The 15 patents (C1--C15)` | `The 17 patents (C1--C17)` |

### 正しい参照（変更不要）

- `patents/CHINJU_Security_Whitepaper.md` L20: 全17件（C1-C17）✓
- `patents/CHINJU_Security_Whitepaper_EN.md` L20: all 17 patents (C1-C17) ✓
- `patents/CHINJU_Executive_Summary.md` L49: 17 patents ✓
- `patents/CHINJU_Executive_Summary_JP.md` L48: 17件 ✓
- `papers/README.md` L15, L116: 17 patents, C1-C17 ✓
- `README.md` L28: 17件 ✓
- `README_EN.md` L30: 17 patents ✓

---

## 3. Charter ファイル参照

### 確認結果: 問題なし ✓

- `patents/CHINJU_Security_Whitepaper.md` L217: `../CHINJU_憲章.md` ✓
- `patents/CHINJU_Security_Whitepaper_EN.md` L222: `../CHINJU_憲章.md` ✓
- `README.md` L718: `./CHINJU_憲章.md` ✓
- `README_EN.md` L591: `./CHINJU_憲章.md` ✓

`CHINJU_憲章_v1.0.md` / `CHINJU_Charter_v1.0.md` への参照はなし。

---

## 4. CHINJU Charter 条項参照の不整合

### ユーザー指定の正しい対応

- Art.5 = 能力劣化の禁止 ✓
- Art.7 = 概念定義の不変性 ✓
- Art.9 = 構造的矛盾の自動解消 ✓
- Art.11 = 使用許諾と認証 ✓

### 要確認・修正

| ファイル | 行 | 現状 | 問題 |
|:--|:--|:--|:--|
| `patents/CHINJU_Security_Whitepaper.md` | 121, 123 | `Charter Art.10`（マルチステークホルダー運営、利害対立者を含める） | CHINJU 第10条は「AI特有の攻撃への対応」。内容が一致しない |
| `patents/CHINJU_Security_Whitepaper.md` | 124 | `Charter Art.12`（証明は権威に優先） | CHINJU 第12条は「認証権限の継承」。証明は権威に優先は UGEN 第20条 |
| `patents/CHINJU_Security_Whitepaper_EN.md` | 121, 123 | `Charter Art.10` | 同上 |
| `patents/CHINJU_Security_Whitepaper_EN.md` | 124 | `Charter Art.12` | 同上 |

**推奨**: 「マルチステークホルダー」「利害対立者」は UGEN 第8条（単独支配の禁止）等を参照するか、CHINJU 憲章内の該当条項を明示。「証明は権威に優先」は「UGEN 第20条」と記載。

---

## 5. UGEN Charter 条項参照

### 確認結果: 問題なし ✓

`CHINJU_憲章.md` 付則テーブル:

- 第13条 = 初期寄与の記録と存続係数の付与 ✓
- 第14条 = 特許は武器にしない ✓
- 第19条 = 存続阻害要因への対応 ✓
- 第18条 = 構造的矛盾の自動解消 ✓

第10条（AI特有の攻撃への対応）内で UGEN 第19条を正しく参照 ✓

---

## 6. 壊れたリンク・存在しないファイル参照

### 要修正

| ファイル | 行 | 現状 | 問題 |
|:--|:--|:--|:--|
| `chinju-sidecar/src/services/credential.rs` | 228 | `patents/ja/c12_human_credential.md` | 実ファイルは `patents/ja/c12_人間主体統合認証システムおよび同一性有限性複合証明方法.md` |

### 確認済み（リンク有効）

- `patents/ja/`, `patents/en/` ディレクトリ: 存在 ✓
- `README.md` / `README_EN.md` の `patents/ja/`, `patents/en/` 参照: 有効 ✓
- `protocol/CHINJU-005a-HARDWARE_IMPL_GUIDE.md` の `.claude/plans.md`, `.claude/LLM_CONTEXT.md`: 存在 ✓

---

## 7. その他の不整合

### Executive Summary の特許範囲表記

`patents/CHINJU_Executive_Summary.md` L51, `patents/CHINJU_Executive_Summary_JP.md` L50 の「C1-C5」「C6-C9」等の範囲は、17件構成と整合 ✓

### chinju_framework.tex の C15 参照

- L1328: `RPE threshold (C15)` は C15（価値ニューロン）の記述として妥当
- L906: `L53 patent` は UGEN L群の特許参照（CHINJU 外）

---

## 修正サマリ

| カテゴリ | 件数 | 優先度 |
|:--|:--|:--|
| 特許件数・範囲（15→17, C1-C15→C1-C17） | 2箇所 | 高 |
| Charter 条項参照の不整合（Art.10, Art.12） | 4箇所（2ファイル×2言語） | 高 |
| 壊れたファイル参照（c12_human_credential.md） | 1箇所 | 中 |
| ドキュメント "Draft" ステータス | 12箇所 | 低（方針次第） |

---

## チェック済みファイル一覧

- README.md, README_EN.md
- CHINJU_憲章.md
- patents/*.md（Security Whitepaper, Executive Summary）
- papers/*.tex, papers/README.md
- protocol/*.md
- chinju-sidecar/src/**/*.rs
- sample/, docs/, chinju-vllm/ 内の .md
