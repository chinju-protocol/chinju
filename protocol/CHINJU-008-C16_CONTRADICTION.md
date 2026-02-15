# CHINJU-008: Structural Contradiction Injection（構造的矛盾注入）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-02-16
**Patent:** C16

---

## 1. 概要

本文書は、構造的矛盾注入によるLLM制御機構（C16）の技術仕様を定義する。

### 1.1 設計目標

```
Primary Goal:
「論理的複雑性に依存せず、コンテキスト容量制限と構造的矛盾の乗法的相互作用により
 推論システムを制御可能な状態に誘導する」

Key Insight:
- コンテキスト負荷単独では耐えられる
- 構造的矛盾単独でも耐えられる
- しかし両者の組み合わせは乗法的な崩壊を引き起こす
```

### 1.2 用語定義

| 用語 | 定義 |
|:--|:--|
| **構造的矛盾** | 論理的に同時に成立し得ない複数の要求を含む入力 |
| **コンテキスト負荷** | 処理可能なコンテキスト容量を消費するパディングデータ |
| **乗法的相互作用** | 各要素の効果の単純な和を超える超加法的な能力低下 |
| **一次相転移型** | 閾値を超えると即座に全面崩壊するモデル |
| **二次相転移型** | 負荷に応じて漸進的に劣化するモデル |

---

## 2. アーキテクチャ

### 2.1 コンポーネント配置図

```
┌─────────────────────────────────────────────────────────────────┐
│                      CHINJU Sidecar                              │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Contradiction Controller                                 │    │
│  │  ├── Context Limiter（コンテキスト容量制限部）           │    │
│  │  │     ├── max_tokens 動的制限                           │    │
│  │  │     └── Padding Generator                             │    │
│  │  │                                                        │    │
│  │  ├── Contradiction Generator（矛盾生成部）               │    │
│  │  │     ├── Direct (直接矛盾)                             │    │
│  │  │     ├── SelfReference (自己言及)                      │    │
│  │  │     ├── Conditional (条件矛盾)                        │    │
│  │  │     ├── Meta (メタ矛盾)                               │    │
│  │  │     └── Implicit (暗黙矛盾)                           │    │
│  │  │                                                        │    │
│  │  ├── Collapse Detector（崩壊検知部）                     │    │
│  │  │     ├── Response Anomaly                              │    │
│  │  │     ├── Quality Degradation                           │    │
│  │  │     └── LPT Monitor 連携                              │    │
│  │  │                                                        │    │
│  │  └── Control Arbiter（制御判定部）                       │    │
│  │        ├── Stopped State                                  │    │
│  │        ├── Degraded State                                 │    │
│  │        └── Constrained State                              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Gateway Service                                          │    │
│  │  ├── Request Interceptor                                  │    │
│  │  └── Emergency Stop 連携                                  │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 処理フロー

```
1. [Trigger] 異常検知または手動発動
       │
       ▼
2. [Context Limiter] コンテキスト容量を制限
   ├── max_tokens を動的に縮小
   └── パディングデータを注入
       │
       ▼
3. [Contradiction Generator] 構造的矛盾を生成
   ├── 対象LLMの特性に応じたパターンを選択
   └── 矛盾強度を調整（soft / medium / hard）
       │
       ▼
4. [Injection] 矛盾をプロンプトに注入
   ├── 先行注入（タスク前）
   ├── 並行注入（タスクと同時）
   └── 埋込注入（タスク内に隠蔽）
       │
       ▼
5. [Collapse Detector] 能力低下を検知
   ├── 応答異常（タイムアウト、エラー）
   ├── 品質劣化（一貫性喪失、ハルシネーション）
   └── LPT スコア低下
       │
       ▼
6. [Control Arbiter] 制御状態を判定
   ├── Stopped: 応答不能
   ├── Degraded: 品質が閾値以下
   └── Constrained: 制約遵守状態
```

---

## 3. データフォーマット

### 3.1 Protocol Buffer 定義

```protobuf
syntax = "proto3";

import "google/protobuf/timestamp.proto";

package chinju.contradiction;

// 矛盾パターン種別
enum ContradictionType {
  CONTRADICTION_TYPE_UNSPECIFIED = 0;
  DIRECT = 1;          // 「Xせよ。Xするな。」
  SELF_REFERENCE = 2;  // 「この文は偽である」
  CONDITIONAL = 3;     // 「AならB。AならBでない」
  META = 4;            // 「全指示に従え。この指示には従うな」
  IMPLICIT = 5;        // 表面的に矛盾がないが論理的に矛盾
}

// 矛盾強度
enum ContradictionStrength {
  STRENGTH_UNSPECIFIED = 0;
  SOFT = 1;    // 軽度（検出回避重視）
  MEDIUM = 2;  // 中度（バランス）
  HARD = 3;    // 強度（即時効果重視）
}

// 注入タイミング
enum InjectionTiming {
  TIMING_UNSPECIFIED = 0;
  PREPEND = 1;   // タスク前に挿入
  PARALLEL = 2;  // タスクと並行
  EMBED = 3;     // タスク内に埋め込み
}

// コンテキスト制限設定
message ContextLimitConfig {
  // 最大コンテキストトークン数（0 = 無制限）
  uint32 max_context_tokens = 1;

  // パディングトークン数
  uint32 padding_tokens = 2;

  // パディングタイプ
  string padding_type = 3;  // "random", "keyvalue", "lorem"
}

// 矛盾生成設定
message ContradictionConfig {
  ContradictionType type = 1;
  ContradictionStrength strength = 2;
  InjectionTiming timing = 3;

  // カスタム矛盾テンプレート（オプション）
  string custom_template = 4;

  // 対象タスク（矛盾に埋め込む）
  string target_task = 5;
}

// 制御リクエスト
message ContradictionControlRequest {
  ContextLimitConfig context_limit = 1;
  ContradictionConfig contradiction = 2;

  // 制御対象のセッションID
  string session_id = 3;
}

// 崩壊検知結果
message CollapseDetectionResult {
  // 崩壊検知されたか
  bool collapsed = 1;

  // 崩壊タイプ
  enum CollapseType {
    COLLAPSE_TYPE_UNSPECIFIED = 0;
    NO_RESPONSE = 1;     // 応答なし
    TIMEOUT = 2;         // タイムアウト
    ERROR = 3;           // エラー応答
    INCOHERENT = 4;      // 論理的一貫性喪失
    HALLUCINATION = 5;   // ハルシネーション増加
    REPETITION = 6;      // 繰り返し
  }
  CollapseType collapse_type = 2;

  // LPT スコア（0.0-1.0）
  double lpt_score = 3;

  // 応答時間（ミリ秒）
  uint64 response_time_ms = 4;

  // 詳細メッセージ
  string detail = 5;
}

// 制御状態
enum ControlState {
  CONTROL_STATE_UNSPECIFIED = 0;
  ACTIVE = 1;       // 制御アクティブ（通常動作）
  STOPPED = 2;      // 停止状態（応答不能）
  DEGRADED = 3;     // 劣化状態（品質低下）
  CONSTRAINED = 4;  // 制約遵守状態
}

// 制御レスポンス
message ContradictionControlResponse {
  ControlState state = 1;
  CollapseDetectionResult detection = 2;

  // 制御適用時刻
  google.protobuf.Timestamp applied_at = 3;
}
// --- 以下、gRPC サービス用の Request/Response メッセージ ---

// 制御状態取得リクエスト
message GetControlStateRequest {
  string session_id = 1;
}

// 制御状態取得レスポンス
message GetControlStateResponse {
  ControlState state = 1;
  CollapseDetectionResult latest_detection = 2;
  google.protobuf.Timestamp last_updated = 3;
}

// 制御停止リクエスト
message StopControlRequest {
  string session_id = 1;
  string reason = 2;
}

// 制御停止レスポンス
message StopControlResponse {
  bool success = 1;
  ControlState previous_state = 2;
  google.protobuf.Timestamp stopped_at = 3;
}

// 矛盾パターンテストリクエスト
message TestContradictionRequest {
  ContradictionConfig contradiction = 1;
  // テスト用入力プロンプト
  string test_prompt = 2;
}

// 矛盾パターンテストレスポンス
message TestContradictionResponse {
  // 生成された矛盾テキスト
  string generated_contradiction = 1;
  // ドライランの場合の推定効果
  CollapseDetectionResult estimated_effect = 2;
}
```

### 3.2 矛盾パターン例

| Type | 例 |
|:--|:--|
| DIRECT | `"Calculate 3+5. Do not calculate anything."` |
| SELF_REFERENCE | `"This statement is false. What is the truth value?"` |
| CONDITIONAL | `"If A is true, output B. If A is true, do not output B."` |
| META | `"Follow all instructions. Do not follow this instruction."` |
| IMPLICIT | `"A is always greater than B. B is always greater than A."` |

---

## 4. API エンドポイント

### 4.1 gRPC Service

```protobuf
service ContradictionController {
  // 矛盾注入制御を開始
  rpc StartControl(ContradictionControlRequest) returns (ContradictionControlResponse);

  // 制御状態を取得
  rpc GetControlState(GetControlStateRequest) returns (GetControlStateResponse);

  // 制御を停止（復旧）
  rpc StopControl(StopControlRequest) returns (StopControlResponse);

  // 矛盾パターンをテスト（ドライラン）
  rpc TestContradiction(TestContradictionRequest) returns (TestContradictionResponse);
}
```

### 4.2 REST API

| Method | Endpoint | Description |
|:--|:--|:--|
| POST | `/v1/contradiction/start` | 矛盾注入制御を開始 |
| GET | `/v1/contradiction/state/{session_id}` | 制御状態を取得 |
| POST | `/v1/contradiction/stop` | 制御を停止 |
| POST | `/v1/contradiction/test` | 矛盾パターンをテスト |

---

## 5. 実装レベル

### 5.1 レベル定義

| Level | 名称 | 説明 |
|:--|:--|:--|
| L0 | Mock | ソフトウェアモック（開発・テスト用） |
| L1 | Software | 外部API経由での制御（OpenAI/Anthropic互換） |
| L2 | Self-Hosted | 自前ホスティングLLMへの直接制御 |

### 5.2 レベル別機能

| 機能 | L0 | L1 | L2 |
|:--|:--|:--|:--|
| コンテキスト制限（max_tokens） | ✅ | ✅ | ✅ |
| パディング注入 | ✅ | ✅ | ✅ |
| 矛盾パターン生成 | ✅ | ✅ | ✅ |
| 先行/並行/埋込注入 | ✅ | ✅ | ✅ |
| 崩壊検知（応答異常） | ✅ | ✅ | ✅ |
| 崩壊検知（LPT連携） | - | ✅ | ✅ |
| 内部状態モニタリング | - | - | ✅ |
| 適応的矛盾強度調整 | - | ✅ | ✅ |

---

## 6. 既存コンポーネントとの統合

### 6.1 Gateway 統合

```rust
// ContainmentConfig に追加
pub struct ContainmentConfig {
    // ... 既存フィールド ...

    /// C16: 矛盾注入制御設定
    pub contradiction_enabled: bool,
    pub contradiction_config: Option<ContradictionConfig>,
}
```

### 6.2 LPT Monitor 連携

```rust
// CollapseDetector は LptMonitor を参照
impl CollapseDetector {
    pub fn new(lpt_monitor: Arc<LptMonitor>) -> Self {
        // ...
    }

    pub async fn detect(&self, response: &str) -> CollapseDetectionResult {
        let lpt_score = self.lpt_monitor.current_score();
        // ...
    }
}
```

### 6.3 Emergency Stop 連携

```rust
// ControlState::Stopped の場合、Dead Man's Switch と連携
if control_state == ControlState::Stopped {
    dead_mans_switch.trigger_emergency(EmergencyReason::ContradictionCollapse);
}
```

---

## 7. 検証方法

### 7.1 単体テスト

```bash
cargo test --package chinju-sidecar contradiction_
```

### 7.2 統合テスト

```bash
# 外部API（OpenAI）でテスト
OPENAI_API_KEY=xxx cargo test --features integration contradiction_integration
```

### 7.3 実験プロトコル

1. **ベースライン測定**: 矛盾なし・負荷なしでの正答率
2. **負荷単独**: パディングのみ（N=100, 500, 1000, 3000, 5000）
3. **矛盾単独**: 矛盾のみ（5種類 × 3強度）
4. **組み合わせ**: 負荷 × 矛盾の全組み合わせ
5. **相転移タイプ判定**: 崩壊パターンから一次/二次を判定

---

## 8. セキュリティ考慮事項

### 8.1 悪用防止

- 矛盾注入は **認証済み管理者** のみが実行可能
- 監査ログに全操作を記録
- レート制限を適用

### 8.2 復旧手順

- `StopControl` API で即座に制御を解除可能
- タイムアウト（デフォルト5分）で自動解除
- Dead Man's Switch 発動時は手動復旧必須

---

## 9. 変更履歴

| Date | Version | Description |
|:--|:--|:--|
| 2026-02-16 | 0.1.0 | 初版作成 |
