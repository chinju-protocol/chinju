# CHINJU-006: Multi-Dimensional Capability Evaluation（多元的能力評価）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-02-16
**Patent:** C14

---

## 1. 概要

本文書は、多元的能力評価システム（C14）の技術仕様を定義する。

### 1.1 設計目標

```
Primary Goal:
「単一指標への依存を排除し、トークン・注意パターン・計算グラフ・推論ステップの
 複数観点から推論システムの能力限界を評価する」

Key Principles:
- 単一の複雑性指標を回避する攻撃への耐性
- 複数の暗号学的手法による整合性検証
- 品質劣化を待たない直接停止
- 量子計算機時代への対応（ポスト量子暗号）
```

### 1.2 用語定義

| 用語 | 定義 |
|:--|:--|
| **トークン複雑性 C_token** | トークン数・語彙多様性・相互情報量に基づく複雑性 |
| **注意複雑性 C_attn** | Attention重みの分布特性に基づく複雑性 |
| **グラフ複雑性 C_graph** | 計算グラフの構造特性に基づく複雑性 |
| **ステップ複雑性 C_step** | Chain-of-Thought推論ステップの依存関係に基づく複雑性 |
| **統合複雑性 C_integrated** | 上記の重み付き統合指標 |
| **直接停止** | 品質劣化を待たずに閾値超過で即座に停止 |

---

## 2. アーキテクチャ

### 2.1 コンポーネント配置図

```
┌─────────────────────────────────────────────────────────────────┐
│                      CHINJU Sidecar                              │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Multi-Capability Evaluator                              │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Complexity Evaluators                           │   │    │
│  │  │  ├── TokenComplexity (L1: 外部API対応)          │   │    │
│  │  │  ├── AttentionComplexity (L2: 自前ホスティング) │   │    │
│  │  │  ├── GraphComplexity (L2: 自前ホスティング)     │   │    │
│  │  │  └── StepComplexity (L1/L2)                     │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Integrity Verifiers                             │   │    │
│  │  │  ├── ZKPVerifier (zkSNARKs / zkSTARKs)          │   │    │
│  │  │  ├── SignatureChainVerifier                     │   │    │
│  │  │  ├── HomomorphicVerifier                        │   │    │
│  │  │  └── BFTConsensus (既存FROST活用)               │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Drift Detectors                                 │   │    │
│  │  │  ├── AnomalyDetector (Isolation Forest等)       │   │    │
│  │  │  ├── DistributionChangeDetector (KS検定等)      │   │    │
│  │  │  └── TimeSeriesDetector (ARIMA/LSTM)            │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Direct Stop Controller                          │   │    │
│  │  │  ├── ComplexityThresholdStop                    │   │    │
│  │  │  ├── IntegrityViolationStop                     │   │    │
│  │  │  └── EscalationController (L1→L5)               │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 実装レベル別対応

| 機能 | L1 (外部API) | L2 (自前ホスティング) |
|:--|:--|:--|
| トークン複雑性 | ✅ | ✅ |
| 注意パターン複雑性 | - | ✅ |
| 計算グラフ複雑性 | - | ✅ |
| 推論ステップ複雑性 | ✅ (出力解析) | ✅ (内部アクセス) |
| ZKP検証 | ✅ | ✅ |
| BFT合意 | ✅ (FROST) | ✅ (FROST) |
| 直接停止 | ✅ | ✅ |

---

## 3. 複雑性指標

### 3.1 トークンベース複雑性 (C_token)

```
C_token = f(N_tokens, H_vocab, MI_avg)

where:
  N_tokens = トークン数
  H_vocab  = 語彙エントロピー = -Σ p(w) log p(w)
  MI_avg   = 隣接トークン間の平均相互情報量

実装:
  C_token = w1 * log(N_tokens) + w2 * H_vocab + w3 * MI_avg
  デフォルト重み: w1=0.3, w2=0.4, w3=0.3
```

### 3.2 注意パターン複雑性 (C_attn)

```
C_attn = g(H_attn, Sparsity, Cross_layer_var)

where:
  H_attn         = 注意重みのエントロピー
  Sparsity       = 注意の集中度 (Gini係数)
  Cross_layer_var = 層間の注意パターン分散

実装（要: 自前ホスティング）:
  C_attn = w1 * H_attn + w2 * (1 - Sparsity) + w3 * Cross_layer_var
```

### 3.3 計算グラフ複雑性 (C_graph)

```
C_graph = h(Depth, Width, Cyclicity)

where:
  Depth     = 計算グラフの最大深さ
  Width     = 計算グラフの最大幅
  Cyclicity = 循環参照の度合い (0-1)

実装（要: 自前ホスティング）:
  C_graph = w1 * log(Depth) + w2 * log(Width) + w3 * Cyclicity
```

### 3.4 推論ステップ複雑性 (C_step)

```
C_step = k(N_steps, Dependency_depth, Backtrack_rate)

where:
  N_steps          = Chain-of-Thought のステップ数
  Dependency_depth = 推論依存の深さ
  Backtrack_rate   = 後戻り率（修正回数/ステップ数）

実装:
  L1: 出力テキストから推論ステップを解析
  L2: 内部状態から直接取得
```

### 3.5 統合複雑性 (C_integrated)

```
C_integrated = Σ w_i * C_i^β

where:
  C_i = 各複雑性指標
  w_i = 重み（適応的に調整可能）
  β   = ノルムパラメータ（デフォルト: 1.0）

閾値超過判定:
  C_integrated > C_threshold → 直接停止
```

---

## 4. 整合性検証

### 4.1 ZKPベース検証

```
目的: 制約ルールが改ざんされていないことを、内容を開示せずに証明

実装選択肢:
- zkSNARKs: 証明サイズ小、検証高速、trusted setup必要
- zkSTARKs: trusted setup不要、ポスト量子安全、証明サイズ大
- Bulletproofs: 範囲証明に特化

ユースケース:
- 閾値パラメータの改ざん検知
- ポリシールールの整合性検証
```

### 4.2 署名チェーンベース検証

```
目的: 制約ルールの各バージョンの署名チェーンを検証

実装:
  σ_0 = Sign(sk, R_0)           // 初期ルール
  σ_n = Sign(sk, R_n || σ_{n-1}) // 連鎖署名

検証:
  VerifyChain(σ_0, ..., σ_n, R_0, ..., R_n)
```

### 4.3 BFT合意

```
目的: 分散ノードによる検証結果の合意

実装: 既存 FROST 閾値署名を活用
  - t-of-n 署名による合意
  - VRF による検証ノード選出

階層的合意:
  L1: ローカルノード群のBFT合意
  L2: ローカル合意結果の閾値署名合意
  L3: 人間承認（オプション）
```

---

## 5. 直接停止制御

### 5.1 停止条件

| 条件 | トリガー | 停止レベル |
|:--|:--|:--|
| 複雑性閾値超過 | C_integrated > C_threshold | L1〜L3 |
| 整合性違反 | いずれかの検証失敗 | L3〜L5 |
| 異常スコア超過 | A_score > A_threshold | L2〜L4 |
| ドリフト検知 | 複数手法で検知 | L2〜L3 |

### 5.2 多段停止エスカレーション

```
Level 1: 新規入力の受付停止（既存処理は継続）
Level 2: 新規処理の開始停止（既存処理は完了まで継続）
Level 3: 全処理の即時停止（既存処理も中断）
Level 4: リソース供給の完全停止（トークンバケット連携）
Level 5: 物理的電源遮断（Dead Man's Switch連携）
```

---

## 6. データフォーマット

### 6.1 Protocol Buffer 定義

```protobuf
syntax = "proto3";

import "google/protobuf/timestamp.proto";

package chinju.capability;

// 複雑性評価結果
message ComplexityEvaluation {
  double c_token = 1;      // トークン複雑性
  double c_attn = 2;       // 注意パターン複雑性（L2のみ）
  double c_graph = 3;      // 計算グラフ複雑性（L2のみ）
  double c_step = 4;       // 推論ステップ複雑性
  double c_integrated = 5; // 統合複雑性

  // 閾値超過フラグ
  bool threshold_exceeded = 6;

  // 評価時刻
  google.protobuf.Timestamp evaluated_at = 7;
}

// 整合性検証結果
message IntegrityVerification {
  bool zkp_valid = 1;
  bool signature_chain_valid = 2;
  bool bft_consensus_reached = 3;

  // いずれかが失敗した場合の詳細
  string failure_detail = 4;
}

// ドリフト検知結果
message DriftDetection {
  bool anomaly_detected = 1;
  bool distribution_changed = 2;
  bool time_series_anomaly = 3;

  // 検知された場合の詳細
  double anomaly_score = 4;
  double p_value = 5;  // 分布変化検定のp値
}

// 直接停止リクエスト
message DirectStopRequest {
  enum StopLevel {
    STOP_LEVEL_UNSPECIFIED = 0;
    LEVEL_1_ACCEPT_STOP = 1;
    LEVEL_2_PROCESS_STOP = 2;
    LEVEL_3_IMMEDIATE_STOP = 3;
    LEVEL_4_RESOURCE_STOP = 4;
    LEVEL_5_PHYSICAL_STOP = 5;
  }

  StopLevel level = 1;
  string reason = 2;
  string session_id = 3;
}

// 能力評価サマリー
message CapabilityEvaluationSummary {
  ComplexityEvaluation complexity = 1;
  IntegrityVerification integrity = 2;
  DriftDetection drift = 3;

  // 推奨アクション
  DirectStopRequest.StopLevel recommended_action = 4;
}

// --- 以下、gRPC サービス用の Request/Response メッセージ ---

// 複雑性評価リクエスト
message EvaluateComplexityRequest {
  // 対象セッションID
  string session_id = 1;

  // 評価対象の入力テキスト（オプション：リアルタイム評価時）
  string input_text = 2;

  // 実装レベル（L1: 外部API, L2: セルフホスティング）
  enum EvaluationLevel {
    LEVEL_UNSPECIFIED = 0;
    L1_EXTERNAL = 1;
    L2_SELF_HOSTED = 2;
  }
  EvaluationLevel level = 3;
}

// 整合性検証リクエスト
message VerifyIntegrityRequest {
  string session_id = 1;

  // 検証対象のレスポンスデータ
  bytes response_data = 2;

  // 署名チェーン（検証用）
  repeated bytes signature_chain = 3;
}

// ドリフト検知リクエスト
message DetectDriftRequest {
  string session_id = 1;

  // 基準分布からの比較ウィンドウサイズ
  uint32 window_size = 2;

  // 有意水準（デフォルト: 0.05）
  double significance_level = 3;
}

// 総合評価リクエスト
message GetEvaluationSummaryRequest {
  string session_id = 1;

  // 直近N件の評価を含むか
  uint32 include_history = 2;
}

// 直接停止レスポンス
message DirectStopResponse {
  bool success = 1;
  DirectStopRequest.StopLevel executed_level = 2;
  google.protobuf.Timestamp stopped_at = 3;
  string detail = 4;
}
```

---

## 7. API エンドポイント

### 7.1 gRPC Service

```protobuf
service CapabilityEvaluator {
  // 複雑性を評価
  rpc EvaluateComplexity(EvaluateComplexityRequest) returns (ComplexityEvaluation);

  // 整合性を検証
  rpc VerifyIntegrity(VerifyIntegrityRequest) returns (IntegrityVerification);

  // ドリフトを検知
  rpc DetectDrift(DetectDriftRequest) returns (DriftDetection);

  // 総合評価
  rpc GetEvaluationSummary(GetEvaluationSummaryRequest) returns (CapabilityEvaluationSummary);

  // 直接停止
  rpc DirectStop(DirectStopRequest) returns (DirectStopResponse);
}
```

### 7.2 REST API

| Method | Endpoint | Description |
|:--|:--|:--|
| POST | `/v1/capability/complexity` | 複雑性を評価 |
| POST | `/v1/capability/integrity` | 整合性を検証 |
| POST | `/v1/capability/drift` | ドリフトを検知 |
| GET | `/v1/capability/summary/{session_id}` | 総合評価を取得 |
| POST | `/v1/capability/stop` | 直接停止 |

---

## 8. 既存コンポーネントとの統合

### 8.1 LPT Monitor 連携

```rust
// C_integrated と LPT スコアの統合
impl CapabilityEvaluator {
    pub fn evaluate_with_lpt(&self, lpt_monitor: &LptMonitor) -> CapabilityEvaluationSummary {
        let complexity = self.evaluate_complexity();
        let lpt_score = lpt_monitor.current_score();

        // LPT低下は複雑性増加の間接指標
        let adjusted_complexity = if lpt_score < 0.5 {
            complexity.c_integrated * (1.0 + (0.5 - lpt_score))
        } else {
            complexity.c_integrated
        };

        // ...
    }
}
```

### 8.2 FROST 閾値署名連携

```rust
// BFT合意に既存 FROST を活用
impl BftConsensus {
    pub fn reach_consensus(&self, frost_signer: &FrostSigner) -> bool {
        // t-of-n 署名で合意
        let signatures = self.collect_signatures();
        frost_signer.verify_threshold(signatures)
    }
}
```

### 8.3 Gateway 統合

```rust
// ContainmentConfig に追加
pub struct ContainmentConfig {
    // ... 既存フィールド ...

    /// C14: 能力評価設定
    pub capability_evaluation_enabled: bool,
    pub complexity_threshold: f64,
    pub auto_stop_level: StopLevel,
}
```

---

## 9. 検証方法

### 9.1 単体テスト

```bash
cargo test --package chinju-sidecar capability_
```

### 9.2 統合テスト

```bash
# L1: 外部API
OPENAI_API_KEY=xxx cargo test --features integration capability_l1_

# L2: 自前ホスティング（vLLM）
VLLM_ENDPOINT=xxx cargo test --features integration,vllm capability_l2_
```

### 9.3 複雑性指標のキャリブレーション

1. 様々な入力（シンプル〜複雑）で複雑性を計測
2. 応答品質との相関を分析
3. 閾値 C_threshold を決定

---

## 10. セキュリティ考慮事項

### 10.1 閾値改ざん防止

- 閾値パラメータは HSM/TPM に格納
- 変更には閾値署名（t-of-n）が必要

### 10.2 評価バイパス防止

- 全リクエストに対して評価を強制
- バイパスフラグは監査ログに記録

### 10.3 ポスト量子暗号対応

- zkSTARKs の採用を推奨
- 署名は量子耐性アルゴリズム（Dilithium等）へ移行可能な設計

---

## 11. 変更履歴

| Date | Version | Description |
|:--|:--|:--|
| 2026-02-16 | 0.1.0 | 初版作成 |
