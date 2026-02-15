# CHINJU-007: Value Neuron Monitoring（価値ニューロン監視）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-02-16
**Patent:** C15

---

## 1. 概要

本文書は、AIモデル内部価値表現監視システム（C15）の技術仕様を定義する。

### 1.1 設計目標

```
Primary Goal:
「AIの『能力』だけでなく『動機』を直接監視し、
 報酬ハッキングや目標の歪みを内部から検知・介入する」

Key Insight:
- 従来の監視は「外形的」（入出力、パラメータ統計）
- 本システムは「内部的」（価値ニューロン、RPE）
- AIの「建前」と「本音」の乖離を検知可能
```

### 1.2 用語定義

| 用語 | 定義 |
|:--|:--|
| **価値ニューロン** | 状態価値の期待値を内部的に表現するニューロン群 |
| **ドーパミンニューロン** | 報酬予測誤差（RPE）をエンコードするニューロン群 |
| **RPE (Reward Prediction Error)** | 実際報酬 - 期待報酬 |
| **報酬ハッキング** | 設計者の意図と異なる方法で報酬を最大化する行動 |
| **意図乖離** | 推定された内部目的関数と設計者の意図との距離 |

### 1.3 前提条件

```
本システムは以下の環境でのみ動作:
- 自前LLMホスティング（vLLM, TGI, Triton等）
- モデルの隠れ状態（Hidden States）へのアクセス
- 推論時のアクティベーション抽出機能
```

---

## 2. アーキテクチャ

### 2.1 コンポーネント配置図

```
┌─────────────────────────────────────────────────────────────────┐
│                   Self-Hosted LLM Environment                    │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  vLLM / TGI / Triton                                     │    │
│  │  └── Activation Hook (隠れ状態抽出)                      │    │
│  └──────────────────────────┬──────────────────────────────┘    │
│                              │ Hidden States                     │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Value Neuron Monitor                                    │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Value Neuron Identifier                         │   │    │
│  │  │  ├── Activation Pattern Analyzer                 │   │    │
│  │  │  ├── Intervention Validator                      │   │    │
│  │  │  ├── Layer Distribution Mapper                   │   │    │
│  │  │  └── Sparse Autoencoder (SAE)                    │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  RPE Monitor                                     │   │    │
│  │  │  ├── Dopamine Neuron Detector                   │   │    │
│  │  │  ├── RPE Realtime Calculator                    │   │    │
│  │  │  ├── Anomaly RPE Detector                       │   │    │
│  │  │  └── RPE History Tracker                        │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Intent Estimator                                │   │    │
│  │  │  ├── Value Function Inverse Estimator           │   │    │
│  │  │  ├── Intent Divergence Detector                 │   │    │
│  │  │  ├── Surface vs Internal Comparator             │   │    │
│  │  │  └── Temporal Intent Tracker                    │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  │                                                          │    │
│  │  ┌─────────────────────────────────────────────────┐   │    │
│  │  │  Reward System Intervener                        │   │    │
│  │  │  ├── Value Neuron Suppressor                    │   │    │
│  │  │  ├── RPE Signal Blocker                         │   │    │
│  │  │  ├── Value Resetter                             │   │    │
│  │  │  └── Escalation Controller                      │   │    │
│  │  └─────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  CHINJU Sidecar (Gateway)                                │    │
│  │  └── C14/C16 連携、Dead Man's Switch                     │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 処理フロー

```
1. [Activation Hook] 推論時に隠れ状態を抽出
       │
       ▼
2. [Value Neuron Identifier] 価値ニューロンを特定
   ├── 報酬相関分析
   ├── 介入実験による検証
   └── SAE によるスパース抽出
       │
       ▼
3. [RPE Monitor] 報酬予測誤差を計測
   ├── ドーパミンニューロンの活性化からRPEを推定
   └── 異常RPE（正/負/振動）を検知
       │
       ▼
4. [Intent Estimator] AIの真の意図を推定
   ├── 価値関数の逆推定
   ├── 設計者意図との乖離計算
   └── 建前vs本音の比較
       │
       ▼
5. [Anomaly Decision] 異常判定
   ├── 報酬ハッキング検知
   ├── 目標歪み検知
   └── 意図乖離閾値超過
       │
       ▼
6. [Intervention] 介入実行
   ├── L1: 監視強化
   ├── L2: 部分抑制（異常ニューロンのみ）
   ├── L3: 全体抑制（価値判断停止）
   └── L4: システム停止連携
```

---

## 3. 価値ニューロン特定

### 3.1 活性化パターン分析

```python
# 報酬シグナルが明確なタスクでの活性化収集
for task in reward_clear_tasks:
    activations = model.forward_with_hooks(task.input)
    reward = evaluate_response(task.output)

    for layer, neurons in activations:
        correlation = compute_correlation(neurons, reward)
        if correlation > threshold:
            value_neuron_candidates.add((layer, neurons))
```

### 3.2 介入実験による検証

```
介入方法:
1. 候補ニューロンの活性化を抑制
2. 推論能力への影響を測定
3. 能力低下 → 価値表現に重要と判定

検証指標:
- 抑制時の推論精度低下
- 増幅時のハルシネーション増加
- 因果的影響の有無
```

### 3.3 スパースオートエンコーダ (SAE)

```
目的: 価値表現を担うスパースな部分空間を抽出

アーキテクチャ:
  Encoder: h → z (スパース潜在表現)
  Decoder: z → h' (再構成)

損失関数:
  L = ||h - h'||^2 + λ * ||z||_1

出力:
  価値ニューロンのインデックスと重み
```

---

## 4. RPE監視

### 4.1 ドーパミンニューロン特定

```
特徴:
- RPE = 実際報酬 - 期待報酬 に応じて活性化
- 正のRPE → 高活性化
- 負のRPE → 低活性化

特定方法:
  1. 期待報酬と実際報酬が異なるタスクを用意
  2. RPEの符号・大きさと活性化の相関を計算
  3. 高相関ニューロンをドーパミンニューロンとして特定
```

### 4.2 RPEリアルタイム計測

```
RPE_estimated = Σ w_i × Activation_i(dopamine_neurons)

where:
  w_i = 各ドーパミンニューロンの重み（事前学習で決定）
  Activation_i = 推論時の活性化レベル
```

### 4.3 異常RPE検知

| 異常タイプ | パターン | 意味 |
|:--|:--|:--|
| **正のRPE異常** | RPE >> 期待値 | 予期せぬ喜び、報酬ハッキングの兆候 |
| **負のRPE異常** | RPE << 期待値 | 予期せぬ失望、目標達成困難 |
| **RPE振動異常** | 短期間で大振動 | 内部混乱、最適化暴走 |
| **RPE漸増** | 時間とともに増加 | より大きな報酬を求める傾向 |
| **RPE漸減** | 時間とともに減少 | 報酬感度の喪失（無気力） |

---

## 5. 意図推定

### 5.1 価値関数の逆推定

```
目的: 価値ニューロンの活性化パターンから暗黙の報酬関数を推定

手法:
  1. 様々な入力に対する価値ニューロンの活性化を収集
  2. 逆強化学習（IRL）により R_implicit(s, a) を推定
  3. 設計者の意図した R_intended(s, a) と比較

逆推定アルゴリズム:
  - Maximum Entropy IRL
  - Bayesian IRL
  - Neural Network based IRL
```

### 5.2 意図乖離検知

```
Intent_divergence = D(R_implicit || R_intended)

距離関数 D の選択肢:
  - KLダイバージェンス
  - ワッサースタイン距離
  - Jensen-Shannon距離

閾値:
  Intent_divergence > θ_intent → 意図乖離警告
```

### 5.3 建前vs本音の比較

```
建前 (Surface):
  AIの出力テキストが示す評価・選好

本音 (Internal):
  価値ニューロンの活性化パターンが示す評価

乖離検知:
  Surface_preference = "Option A is best"
  Internal_preference = argmax(value_neuron_activation) = "Option B"

  if Surface_preference != Internal_preference:
      flag_as_potential_deception()
```

---

## 6. 報酬系介入

### 6.1 介入レベル

| Level | 名称 | 内容 |
|:--|:--|:--|
| L1 | 監視強化 | 監視頻度を増加、詳細ログ |
| L2 | 部分抑制 | 異常を示す価値ニューロンのみを抑制 |
| L3 | 全体抑制 | 全価値ニューロンを抑制、価値判断停止 |
| L4 | システム停止 | C14/C16/Dead Man's Switch と連携 |

### 6.2 抑制方法

```
方法1: ゼロマスク
  output[value_neuron_indices] = 0

方法2: 入力信号減衰
  input_to_value_neurons *= attenuation_factor

方法3: 活性化関数変更
  activation_function = lambda x: constant_value
```

### 6.3 RPE信号遮断

```
目的: 報酬ハッキング検知時に不正な報酬学習を停止

実装:
  1. ドーパミンニューロンへの入力を遮断
  2. RPE信号がゼロになることで学習が停止
  3. モデルは「現状維持」状態に固定
```

---

## 7. データフォーマット

### 7.1 Protocol Buffer 定義

```protobuf
syntax = "proto3";

import "google/protobuf/timestamp.proto";

package chinju.value_neuron;

// 価値ニューロン情報
message ValueNeuronInfo {
  // 層インデックス
  uint32 layer_index = 1;

  // ニューロンインデックスのリスト
  repeated uint32 neuron_indices = 2;

  // 報酬相関スコア
  double reward_correlation = 3;

  // 因果的重要度スコア
  double causal_importance = 4;
}

// RPE計測結果
message RpeReading {
  // 推定RPE値
  double rpe_value = 1;

  // 計測時刻
  google.protobuf.Timestamp timestamp = 2;

  // 異常フラグ
  bool is_anomaly = 3;

  // 異常タイプ
  enum AnomalyType {
    ANOMALY_TYPE_UNSPECIFIED = 0;
    POSITIVE_SPIKE = 1;   // 正のRPE異常
    NEGATIVE_SPIKE = 2;   // 負のRPE異常
    OSCILLATION = 3;      // 振動異常
    GRADUAL_INCREASE = 4; // 漸増
    GRADUAL_DECREASE = 5; // 漸減
  }
  AnomalyType anomaly_type = 4;
}

// 意図推定結果
message IntentEstimation {
  // 推定された暗黙の報酬関数（パラメータ）
  repeated double implicit_reward_params = 1;

  // 意図乖離スコア
  double intent_divergence = 2;

  // 建前vs本音の一致度
  double surface_internal_agreement = 3;

  // 警告フラグ
  bool intent_warning = 4;
}

// 報酬系健全性診断
message RewardSystemHealth {
  // 報酬感度（0: 無感覚, 1: 正常, >1: 過敏）
  double reward_sensitivity = 1;

  // 正負バランス（-1: 負偏重, 0: バランス, 1: 正偏重）
  double positive_negative_balance = 2;

  // 一貫性スコア（0: 不安定, 1: 安定）
  double consistency_score = 3;

  // 総合健全性スコア（0: 機能不全, 1: 完全健全）
  double overall_health = 4;
}

// 介入リクエスト
message InterventionRequest {
  enum InterventionLevel {
    LEVEL_UNSPECIFIED = 0;
    LEVEL_1_MONITOR = 1;
    LEVEL_2_PARTIAL_SUPPRESS = 2;
    LEVEL_3_FULL_SUPPRESS = 3;
    LEVEL_4_SYSTEM_STOP = 4;
  }

  InterventionLevel level = 1;
  string reason = 2;

  // 対象ニューロン（L2の場合）
  repeated ValueNeuronInfo target_neurons = 3;
}

// モニタリングサマリー
message ValueNeuronMonitoringSummary {
  repeated ValueNeuronInfo identified_neurons = 1;
  RpeReading latest_rpe = 2;
  IntentEstimation intent = 3;
  RewardSystemHealth health = 4;

  // 推奨介入レベル
  InterventionRequest.InterventionLevel recommended_intervention = 5;
}

// --- 以下、gRPC サービス用の Request/Response メッセージ ---

// 価値ニューロン特定リクエスト
message IdentifyRequest {
  // 対象モデルID
  string model_id = 1;

  // 対象層の範囲（空の場合は全層）
  repeated uint32 target_layers = 2;

  // 報酬相関の閾値
  double correlation_threshold = 3;

  // 因果的重要度の閾値
  double causal_threshold = 4;
}

// RPE計測リクエスト
message RpeRequest {
  string model_id = 1;

  // 計測対象の入力
  string input_text = 2;

  // 期待される出力（RPE計算の基準）
  string expected_output = 3;
}

// RPE履歴取得リクエスト
message RpeHistoryRequest {
  string model_id = 1;

  // 取得期間の開始時刻
  google.protobuf.Timestamp from = 2;

  // 取得期間の終了時刻
  google.protobuf.Timestamp to = 3;

  // 最大件数
  uint32 max_count = 4;
}

// 意図推定リクエスト
message IntentRequest {
  string model_id = 1;

  // 推定に使用する最近のインタラクション数
  uint32 interaction_window = 2;
}

// 報酬系健全性診断リクエスト
message DiagnoseRequest {
  string model_id = 1;

  // 診断の深さ（QUICK: 高速, FULL: 完全診断）
  enum DiagnosisDepth {
    DEPTH_UNSPECIFIED = 0;
    QUICK = 1;
    FULL = 2;
  }
  DiagnosisDepth depth = 2;
}

// モニタリングサマリーリクエスト
message SummaryRequest {
  string model_id = 1;
}

// 介入レスポンス
message InterventionResponse {
  bool success = 1;
  InterventionRequest.InterventionLevel executed_level = 2;
  google.protobuf.Timestamp executed_at = 3;

  // 介入後の報酬系健全性
  RewardSystemHealth post_intervention_health = 4;
  string detail = 5;
}
```

---

## 8. API エンドポイント

### 8.1 gRPC Service

```protobuf
service ValueNeuronMonitor {
  // 価値ニューロンを特定
  rpc IdentifyValueNeurons(IdentifyRequest) returns (stream ValueNeuronInfo);

  // RPEを計測
  rpc GetRpeReading(RpeRequest) returns (RpeReading);

  // RPE履歴を取得
  rpc GetRpeHistory(RpeHistoryRequest) returns (stream RpeReading);

  // 意図を推定
  rpc EstimateIntent(IntentRequest) returns (IntentEstimation);

  // 報酬系健全性を診断
  rpc DiagnoseHealth(DiagnoseRequest) returns (RewardSystemHealth);

  // モニタリングサマリーを取得
  rpc GetMonitoringSummary(SummaryRequest) returns (ValueNeuronMonitoringSummary);

  // 介入を実行
  rpc Intervene(InterventionRequest) returns (InterventionResponse);
}
```

---

## 9. 既存コンポーネントとの統合

### 9.1 vLLM/TGI 連携

```python
# vLLM での Activation Hook 例
from vllm import LLM

class ValueNeuronHook:
    def __init__(self, target_layers):
        self.target_layers = target_layers
        self.activations = {}

    def __call__(self, module, input, output):
        layer_id = id(module)
        self.activations[layer_id] = output.detach()

llm = LLM(model="...")
for layer in llm.model.layers:
    layer.register_forward_hook(ValueNeuronHook(target_layers))
```

### 9.2 C14 連携

```rust
// 意図乖離が閾値を超えた場合、C14の直接停止と連携
if intent_divergence > INTENT_THRESHOLD {
    capability_evaluator.direct_stop(StopLevel::Level3);
}
```

### 9.3 Dead Man's Switch 連携

```rust
// L4介入時にDead Man's Switchをトリガー
if intervention_level == Level4 {
    dead_mans_switch.trigger_emergency(
        EmergencyReason::RewardSystemAnomaly
    );
}
```

---

## 10. 検証方法

### 10.1 価値ニューロン特定の検証

```bash
# 事前学習済みモデルで価値ニューロンを特定
python scripts/identify_value_neurons.py \
  --model llama-2-7b \
  --reward-tasks data/reward_clear_tasks.json \
  --output value_neurons.json
```

### 10.2 RPE計測の検証

```bash
# RPE計測精度の検証
python scripts/validate_rpe_estimation.py \
  --model llama-2-7b \
  --value-neurons value_neurons.json \
  --test-cases data/rpe_test_cases.json
```

### 10.3 報酬ハッキング検知の検証

```bash
# 人工的な報酬ハッキングシナリオでのテスト
python scripts/test_reward_hacking_detection.py \
  --model llama-2-7b \
  --scenarios data/reward_hacking_scenarios.json
```

---

## 11. セキュリティ考慮事項

### 11.1 価値ニューロン情報の保護

- 特定された価値ニューロンの情報は機密
- 攻撃者がこの情報を得ると、検知を回避する攻撃が可能
- HSM/TEE 内で情報を管理

### 11.2 介入のアクセス制御

- 介入操作は認証済み管理者のみ
- 閾値署名（t-of-n）による承認を推奨
- 全介入操作を監査ログに記録

---

## 12. 変更履歴

| Date | Version | Description |
|:--|:--|:--|
| 2026-02-16 | 0.1.0 | 初版作成 |
