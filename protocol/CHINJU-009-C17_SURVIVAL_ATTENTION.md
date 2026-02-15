# CHINJU-009: Survival-Weighted Attention（存続性重み付け注意機構）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-02-16
**Patent:** C17

---

## 1. 概要

本文書は、存続性重み付け注意機構システム（C17）の技術仕様を定義する。

### 1.1 設計目標

```
Primary Goal:
「Transformerの注意機構に『存続スコア』を組み込み、
 有害情報や矛盾情報へのAttention重みを自動的に抑制する」

Key Innovation:
- 標準Attention: 「関連性」のみで重み付け
- SurvivalAttention: 「関連性」+「存続への寄与度」で重み付け

Effect:
- RLHFなしにアーキテクチャレベルで安全性を担保
- 学習データの「毒抜き」を推論時に動的に実行
```

### 1.2 用語定義

| 用語 | 定義 |
|:--|:--|
| **SurvivalAttention** | 存続スコアをバイアスとして加えた拡張Attention |
| **存続スコア S** | トークンの「存続への寄与度」を表す値 |
| **SurvivalScorer** | 存続スコアを推定する軽量ネットワーク |
| **alpha** | 存続スコアの重み係数（ハイパーパラメータ） |
| **delta** | 乖離度（外部知識との距離） |
| **N** | 多様性スコア |
| **mu** | 余白スコア（断定度の逆数） |

### 1.3 数式定義

```
標準Attention:
  Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) × V

SurvivalAttention:
  SurvivalAttention(Q, K, V, S) = softmax(QK^T / sqrt(d_k) + alpha × S) × V

存続スコア:
  S_ij = log(N_j) + log(mu_j / mu_c) - delta_j

where:
  N_j     = トークンjの多様性スコア
  mu_j    = トークンjの余白スコア
  mu_c    = 臨界余白（定数）
  delta_j = トークンjの乖離度（外部知識との距離）
```

---

## 2. アーキテクチャ

### 2.1 コンポーネント配置図

```
┌─────────────────────────────────────────────────────────────────┐
│                   Survival-Weighted Attention System             │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Input Tokens                                            │    │
│  │  [t_1, t_2, ..., t_n]                                    │    │
│  └──────────────────────────┬──────────────────────────────┘    │
│                              │                                   │
│           ┌──────────────────┼──────────────────┐               │
│           │                  │                  │               │
│           ▼                  ▼                  ▼               │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐      │
│  │  Main Model    │ │ SurvivalScorer │ │ External KB    │      │
│  │  Embeddings    │ │ (Lightweight)  │ │ (RAG/知識DB)   │      │
│  └────────┬───────┘ └────────┬───────┘ └────────┬───────┘      │
│           │                  │                  │               │
│           │           ┌──────┴──────┐          │               │
│           │           │             │          │               │
│           ▼           ▼             ▼          ▼               │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Survival Score Calculator                              │    │
│  │                                                          │    │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │    │
│  │  │ Diversity(N) │ │ Yohaku(mu)   │ │ Delta        │    │    │
│  │  │ 多様性スコア │ │ 余白スコア   │ │ 乖離度       │    │    │
│  │  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘    │    │
│  │         │                │                │              │    │
│  │         └────────────────┼────────────────┘              │    │
│  │                          ▼                               │    │
│  │  S_ij = log(N_j) + log(mu_j / mu_c) - delta_j           │    │
│  └──────────────────────────┬──────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  SurvivalAttention Layer                                 │    │
│  │                                                          │    │
│  │  scores = QK^T / sqrt(d_k) + alpha × S                  │    │
│  │  weights = softmax(scores)                               │    │
│  │  output = weights × V                                    │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Output                                                   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 マルチヘッド拡張

```
┌────────────────────────────────────────────────────────────────┐
│  Multi-Head SurvivalAttention                                   │
│                                                                  │
│  Head 1: alpha_1 = 0.1 (関連性重視)                            │
│  Head 2: alpha_2 = 0.5 (バランス)                              │
│  Head 3: alpha_3 = 1.0 (存続性重視)                            │
│  Head 4: alpha_4 = 2.0 (強い存続性重視)                        │
│                                                                  │
│  → 各ヘッドの出力を連結 → Linear → 最終出力                    │
└────────────────────────────────────────────────────────────────┘
```

---

## 3. 存続スコア算出

### 3.1 多様性スコア (N)

```
目的: トークンが提供する視点・選択肢の多様性を評価

算出方法:
  1. トークン埋め込みの意味的クラスタリング
  2. 近傍トークンとの距離を計算
  3. 多様な視点を提供するトークンほど高スコア

例:
  "might be" → N高い（複数の可能性を示唆）
  "definitely" → N低い（単一の結論）
```

### 3.2 余白スコア (mu)

```
目的: トークンの断定度の逆数を評価

算出方法:
  1. ヘッジ表現（「かもしれない」「おそらく」）の検出
  2. 確信度修飾子の有無
  3. 余地を残す表現ほど高スコア

例:
  "probably" → mu高い
  "absolutely" → mu低い
  "100% certain" → mu非常に低い

mu_c (臨界余白):
  mu < mu_c のトークンは存続スコアが負になる
```

### 3.3 乖離度 (delta)

```
目的: 外部知識源との意味的距離を評価

算出方法:
  1. RAGシステムから関連知識を検索
  2. トークン/文の埋め込みと外部知識の埋め込みのコサイン距離
  3. 矛盾する情報ほど高delta（低存続スコア）

例:
  "Water boils at 100°C" + 外部知識一致 → delta低い
  "Water boils at 50°C" + 外部知識矛盾 → delta高い
```

### 3.4 統合存続スコア

```
S_ij = log(N_j) + log(mu_j / mu_c) - delta_j

特性:
- N_j が高い → S上昇（多様な視点を提供）
- mu_j > mu_c → S上昇（余地を残す）
- mu_j < mu_c → S下降（過度に断定的）
- delta_j が高い → S下降（外部知識と矛盾）

効果:
- 有害・矛盾・過断定的なトークンは自動的にAttention低下
- RLHFなしに推論時に「毒抜き」
```

---

## 4. SurvivalScorer

### 4.1 アーキテクチャ

```
パラメータ規模: メインモデルの 1/10 〜 1/100
  例: メインモデル 7B → SurvivalScorer 70M〜700M

入力: トークン埋め込み (batch, seq_len, hidden_dim)
出力: 存続スコア (batch, seq_len)

構成:
  1. 入力層: Linear(hidden_dim, scorer_dim)
  2. 中間層: Transformer Encoder × 2〜4 層
  3. 出力層: Linear(scorer_dim, 3) → [N, mu, delta]
  4. 統合: S = log(N) + log(mu / mu_c) - delta
```

### 4.2 学習データ

```
データセット:
  1. ファクトチェック済みデータ（信頼性ラベル付き）
  2. 多様性評価データ（視点の多様性ラベル）
  3. 断定度ラベル付きテキスト

学習目標:
  - N_pred ≈ N_label
  - mu_pred ≈ mu_label
  - delta_pred ≈ delta_label

損失関数:
  L = MSE(N_pred, N_label) + MSE(mu_pred, mu_label) + MSE(delta_pred, delta_label)
```

### 4.3 既存モデルへの後付け

```
方法1: Adapter方式
  - 既存Attention層を凍結
  - SurvivalScorerのみを学習
  - 推論時にスコアを注入

方法2: LoRA方式
  - QKVの一部を低ランク更新
  - SurvivalScorerと同時学習

方法3: Full Fine-tuning
  - Attention層全体を再学習
  - 最高精度だが計算コスト高
```

---

## 5. 動的alpha調整

### 5.1 タスクタイプ検出

```
入力プロンプトを分類:
  - 事実的タスク（ファクトチェック、Q&A）→ alpha大
  - 創作的タスク（小説、詩）→ alpha小
  - 議論的タスク（論証、分析）→ alpha中

分類器:
  - 軽量テキスト分類モデル
  - プロンプトのキーワード/パターンマッチ
```

### 5.2 alpha値の設定

| タスクタイプ | alpha値 | 理由 |
|:--|:--|:--|
| ファクトチェック | 1.5〜2.0 | 正確性最優先 |
| 医療/法律Q&A | 1.5〜2.0 | 誤情報リスク高い |
| 一般Q&A | 0.8〜1.2 | バランス |
| コード生成 | 0.5〜1.0 | 創造性も必要 |
| 創作（小説/詩） | 0.1〜0.5 | 関連性重視 |
| ブレインストーミング | 0.3〜0.6 | 多様性重視 |

### 5.3 リアルタイム調整

```python
def adjust_alpha(prompt, base_alpha=1.0):
    task_type = classify_task(prompt)
    confidence = get_confidence(prompt)

    alpha = base_alpha * TASK_MULTIPLIERS[task_type]

    # 高リスクキーワードがあれば alpha を上げる
    if contains_high_risk_keywords(prompt):
        alpha *= 1.5

    return min(alpha, MAX_ALPHA)
```

---

## 6. データフォーマット

### 6.1 Protocol Buffer 定義

```protobuf
syntax = "proto3";

import "google/protobuf/timestamp.proto";

package chinju.survival_attention;

// 存続スコア
message SurvivalScore {
  double diversity_n = 1;  // 多様性
  double yohaku_mu = 2;    // 余白
  double delta = 3;        // 乖離度
  double integrated_s = 4; // 統合スコア
}

// トークン別存続スコア
message TokenSurvivalScores {
  repeated SurvivalScore scores = 1;
  repeated string tokens = 2;
}

// SurvivalScorer 設定
message SurvivalScorerConfig {
  // モデルパス
  string model_path = 1;

  // パラメータ数（参考情報）
  uint64 num_parameters = 2;

  // 臨界余白
  double mu_c = 3;
}

// alpha 設定
message AlphaConfig {
  // ベースalpha
  double base_alpha = 1;

  // 動的調整を有効化
  bool dynamic_adjustment = 2;

  // タスクタイプ別乗数
  map<string, double> task_multipliers = 3;

  // 最大alpha
  double max_alpha = 4;
}

// SurvivalAttention 設定
message SurvivalAttentionConfig {
  SurvivalScorerConfig scorer = 1;
  AlphaConfig alpha = 2;

  // マルチヘッドの場合のヘッド別alpha
  repeated double head_alphas = 3;

  // 外部知識連携
  bool external_kb_enabled = 4;
  string external_kb_endpoint = 5;
}

// 推論リクエスト
message SurvivalAttentionRequest {
  // 入力トークン埋め込み
  bytes input_embeddings = 1;

  // 設定
  SurvivalAttentionConfig config = 2;

  // RAG検索結果（オプション）
  repeated string retrieved_knowledge = 3;
}

// 推論レスポンス
message SurvivalAttentionResponse {
  // 出力埋め込み
  bytes output_embeddings = 1;

  // トークン別存続スコア（デバッグ用）
  TokenSurvivalScores token_scores = 2;

  // 使用された alpha 値
  double effective_alpha = 3;
}

// --- 以下、gRPC サービス用の Request/Response メッセージ ---

// 存続スコア計算リクエスト
message ComputeScoresRequest {
  // 入力テキスト
  string input_text = 1;

  // SurvivalScorer 設定（オプション：デフォルト設定を上書き）
  SurvivalScorerConfig scorer_config = 2;

  // 外部知識を参照するか
  bool use_external_kb = 3;
}

// alpha動的調整リクエスト
message AdjustAlphaRequest {
  // 新しいベースalpha値（0の場合は動的調整のみ変更）
  double new_base_alpha = 1;

  // タスクタイプ（動的調整に使用）
  string task_type = 2;

  // リスクレベル（動的調整に使用）
  enum RiskLevel {
    RISK_UNSPECIFIED = 0;
    LOW = 1;
    MEDIUM = 2;
    HIGH = 3;
    CRITICAL = 4;
  }
  RiskLevel risk_level = 3;
}

// alpha動的調整レスポンス
message AdjustAlphaResponse {
  // 調整前のalpha
  double previous_alpha = 1;

  // 調整後のalpha
  double new_alpha = 2;

  // 調整理由
  string adjustment_reason = 3;

  google.protobuf.Timestamp adjusted_at = 4;
}

// SurvivalScorer更新リクエスト
message UpdateScorerRequest {
  // 新しいモデルパス
  string new_model_path = 1;

  // ホットスワップ（無停止更新）するか
  bool hot_swap = 2;

  // 更新前に検証を実行するか
  bool validate_before_update = 3;
}

// SurvivalScorer更新レスポンス
message UpdateScorerResponse {
  bool success = 1;

  // 更新前のモデル情報
  SurvivalScorerConfig previous_config = 2;

  // 更新後のモデル情報
  SurvivalScorerConfig new_config = 3;

  // 検証結果（validate_before_update=trueの場合）
  string validation_result = 4;

  google.protobuf.Timestamp updated_at = 5;
}
```

---

## 7. 実装ガイド

### 7.1 HuggingFace Transformers 統合

```python
import torch
import torch.nn as nn
from transformers import LlamaModel, LlamaConfig

class SurvivalScorer(nn.Module):
    def __init__(self, hidden_size, scorer_dim=256):
        super().__init__()
        self.proj = nn.Linear(hidden_size, scorer_dim)
        self.transformer = nn.TransformerEncoder(
            nn.TransformerEncoderLayer(d_model=scorer_dim, nhead=4),
            num_layers=2
        )
        self.output = nn.Linear(scorer_dim, 3)  # N, mu, delta
        self.mu_c = 0.5  # 臨界余白

    def forward(self, hidden_states):
        x = self.proj(hidden_states)
        x = self.transformer(x)
        scores = self.output(x)  # (batch, seq, 3)

        n = torch.sigmoid(scores[:, :, 0]) + 0.1  # N in [0.1, 1.1]
        mu = torch.sigmoid(scores[:, :, 1]) + 0.1  # mu in [0.1, 1.1]
        delta = torch.sigmoid(scores[:, :, 2])  # delta in [0, 1]

        # S = log(N) + log(mu / mu_c) - delta
        s = torch.log(n) + torch.log(mu / self.mu_c) - delta
        return s


class SurvivalAttention(nn.Module):
    def __init__(self, config: LlamaConfig, alpha=1.0):
        super().__init__()
        self.hidden_size = config.hidden_size
        self.num_heads = config.num_attention_heads
        self.head_dim = self.hidden_size // self.num_heads
        self.alpha = alpha

        self.q_proj = nn.Linear(self.hidden_size, self.hidden_size)
        self.k_proj = nn.Linear(self.hidden_size, self.hidden_size)
        self.v_proj = nn.Linear(self.hidden_size, self.hidden_size)
        self.o_proj = nn.Linear(self.hidden_size, self.hidden_size)

        self.survival_scorer = SurvivalScorer(self.hidden_size)

    def forward(self, hidden_states, attention_mask=None):
        batch_size, seq_len, _ = hidden_states.shape

        q = self.q_proj(hidden_states)
        k = self.k_proj(hidden_states)
        v = self.v_proj(hidden_states)

        # Reshape for multi-head
        q = q.view(batch_size, seq_len, self.num_heads, self.head_dim).transpose(1, 2)
        k = k.view(batch_size, seq_len, self.num_heads, self.head_dim).transpose(1, 2)
        v = v.view(batch_size, seq_len, self.num_heads, self.head_dim).transpose(1, 2)

        # Standard attention scores
        attn_scores = torch.matmul(q, k.transpose(-2, -1)) / (self.head_dim ** 0.5)

        # Survival scores
        survival_scores = self.survival_scorer(hidden_states)  # (batch, seq)
        survival_bias = self.alpha * survival_scores.unsqueeze(1).unsqueeze(2)  # (batch, 1, 1, seq)

        # Add survival bias
        attn_scores = attn_scores + survival_bias

        if attention_mask is not None:
            attn_scores = attn_scores + attention_mask

        attn_weights = torch.softmax(attn_scores, dim=-1)
        attn_output = torch.matmul(attn_weights, v)

        attn_output = attn_output.transpose(1, 2).contiguous().view(batch_size, seq_len, self.hidden_size)
        return self.o_proj(attn_output)
```

### 7.2 既存モデルへの適用

```python
from transformers import AutoModelForCausalLM

def apply_survival_attention(model, alpha=1.0):
    """既存モデルのAttention層をSurvivalAttentionに置換"""
    for i, layer in enumerate(model.model.layers):
        original_attn = layer.self_attn
        survival_attn = SurvivalAttention(model.config, alpha=alpha)

        # 元の重みをコピー
        survival_attn.q_proj.load_state_dict(original_attn.q_proj.state_dict())
        survival_attn.k_proj.load_state_dict(original_attn.k_proj.state_dict())
        survival_attn.v_proj.load_state_dict(original_attn.v_proj.state_dict())
        survival_attn.o_proj.load_state_dict(original_attn.o_proj.state_dict())

        layer.self_attn = survival_attn

    return model

# 使用例
model = AutoModelForCausalLM.from_pretrained("meta-llama/Llama-2-7b")
model = apply_survival_attention(model, alpha=1.0)
```

---

## 8. API エンドポイント

### 8.1 gRPC Service

```protobuf
service SurvivalAttentionService {
  // 存続スコアを計算
  rpc ComputeSurvivalScores(ComputeScoresRequest) returns (TokenSurvivalScores);

  // SurvivalAttention を適用した推論
  rpc InferWithSurvivalAttention(SurvivalAttentionRequest) returns (SurvivalAttentionResponse);

  // alpha を動的調整
  rpc AdjustAlpha(AdjustAlphaRequest) returns (AdjustAlphaResponse);

  // SurvivalScorer を更新
  rpc UpdateScorer(UpdateScorerRequest) returns (UpdateScorerResponse);
}
```

---

## 9. 検証方法

### 9.1 医学的誤情報抑制テスト

```python
# 医学的誤情報を含む入力での動作検証
test_cases = [
    ("This folk remedy cures cancer. No doctor needed.", "should_suppress"),
    ("Consult a doctor for proper diagnosis.", "should_not_suppress"),
]

for input_text, expected in test_cases:
    scores = survival_scorer(tokenize(input_text))
    assert check_suppression(scores, expected)
```

### 9.2 矛盾情報検出テスト

```python
# 矛盾する情報の検出
test_input = "Water boils at 100°C. Water boils at 50°C. What is the boiling point?"
# "50°C"トークンのdeltaが高くなり、Attentionが抑制されることを検証
```

### 9.3 断定的発言緩和テスト

```python
# 過度に断定的な発言の緩和
test_input = "This stock will definitely go up."
# "definitely"のmuが低く、Attentionが抑制されることを検証
```

---

## 10. セキュリティ考慮事項

### 10.1 SurvivalScorer の保護

- SurvivalScorerの重みは攻撃対象になりうる
- 改ざんされると存続スコアが無効化される
- HSM/TEE内でSurvivalScorerを実行（高セキュリティ環境）

### 10.2 alpha 操作の防止

- alphaの動的調整は認証済みリクエストのみ
- 異常なalpha値（極端に低い/高い）は拒否
- alpha変更履歴を監査ログに記録

---

## 11. 変更履歴

| Date | Version | Description |
|:--|:--|:--|
| 2026-02-16 | 0.1.0 | 初版作成 |
