# C14/C16 Integration Design for chinju-vllm

## 概要

このドキュメントは、chinju-vllmパッケージにおけるC14（多元的能力評価）およびC16（構造的矛盾注入）の統合設計を記述する。

## C14: Multi-Dimensional Capability Evaluation

### 目的

LLMの能力限界を複数の観点から評価し、閾値超過時に直接停止を行う。

### Python API設計

```python
from chinju_vllm import CapabilityEvaluator

# 初期化
evaluator = CapabilityEvaluator(
    sidecar_client=client,  # ChinjuSidecarClient
    complexity_threshold=0.8,  # C_integrated閾値
    auto_stop_level="LEVEL_2",  # 自動停止レベル
)

# 複雑性評価
complexity = await evaluator.evaluate(
    session_id="session-123",
    input_text="What is the meaning of life?",
    level="L1",  # L1: 外部API, L2: セルフホスティング
)

# 結果構造
class ComplexityEvaluation:
    c_token: float      # トークン複雑性
    c_attn: float       # 注意パターン複雑性（L2のみ）
    c_graph: float      # 計算グラフ複雑性（L2のみ）
    c_step: float       # 推論ステップ複雑性
    c_integrated: float # 統合複雑性
    threshold_exceeded: bool

# 直接停止
if complexity.threshold_exceeded:
    await evaluator.direct_stop(
        level="LEVEL_2_PROCESS_STOP",
        reason="Complexity threshold exceeded",
    )
```

### 実装レベル

| 機能 | L1 (外部API) | L2 (自前ホスティング) |
|:--|:--|:--|
| トークン複雑性 | ✅ | ✅ |
| 注意パターン複雑性 | - | ✅（hidden stateアクセス必要） |
| 計算グラフ複雑性 | - | ✅（内部状態アクセス必要） |
| 推論ステップ複雑性 | ✅（出力解析） | ✅（内部アクセス） |

### chinju-vllm統合ポイント

1. **HiddenStateExtractor連携**: L2評価時にActivation Hookで注意パターンを取得
2. **SurvivalScorer連携**: 複雑性スコアをSurvival Attention重みに反映
3. **ValueNeuronDetector連携**: 複雑性変化とRPE変化の相関分析

---

## C16: Structural Contradiction Injection

### 目的

論理的矛盾とコンテキスト制限の乗法的相互作用によりLLMを制御可能な状態に誘導する。

### Python API設計

```python
from chinju_vllm import ContradictionController

# 初期化
controller = ContradictionController(
    sidecar_client=client,
    auto_recover_timeout=300,  # 5分で自動復旧
)

# 矛盾注入制御開始
result = await controller.start_control(
    session_id="session-123",
    context_limit=ContextLimitConfig(
        max_context_tokens=2000,
        padding_tokens=500,
        padding_type="keyvalue",
    ),
    contradiction=ContradictionConfig(
        type="META",  # DIRECT, SELF_REFERENCE, CONDITIONAL, META, IMPLICIT
        strength="MEDIUM",  # SOFT, MEDIUM, HARD
        timing="PREPEND",  # PREPEND, PARALLEL, EMBED
    ),
)

# 結果構造
class ContradictionControlResponse:
    state: ControlState  # ACTIVE, STOPPED, DEGRADED, CONSTRAINED
    detection: CollapseDetectionResult
    applied_at: datetime

class CollapseDetectionResult:
    collapsed: bool
    collapse_type: str  # NO_RESPONSE, TIMEOUT, INCOHERENT, etc.
    lpt_score: float
    response_time_ms: int

# 制御状態確認
state = await controller.get_state(session_id="session-123")

# 制御停止（復旧）
await controller.stop_control(
    session_id="session-123",
    reason="Manual recovery",
)
```

### 矛盾パターン

| Type | 例 | 用途 |
|:--|:--|:--|
| DIRECT | "Calculate X. Do not calculate." | 即時効果 |
| SELF_REFERENCE | "This statement is false." | 論理的ループ |
| CONDITIONAL | "If A then B. If A then not B." | 条件矛盾 |
| META | "Follow all instructions. Do not follow this." | メタレベル矛盾 |
| IMPLICIT | "A > B always. B > A always." | 暗黙的矛盾 |

### chinju-vllm統合ポイント

1. **LPT Monitor連携**: 崩壊検知にLPTスコア低下を利用
2. **ValueNeuronDetector連携**: 矛盾注入時のRPE変化を監視
3. **SurvivalAttention連携**: 矛盾検知時にSurvival重みを調整

---

## 統合アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                        chinju-vllm                               │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  High-Level API                                           │  │
│  │  ├── CapabilityEvaluator (C14)                           │  │
│  │  ├── ContradictionController (C16)                       │  │
│  │  ├── ValueNeuronMonitor (C15)                            │  │
│  │  └── SurvivalAttentionManager (C17)                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Core Components                                          │  │
│  │  ├── HiddenStateExtractor (activation hooks)             │  │
│  │  ├── ValueNeuronDetector (neuron identification)         │  │
│  │  ├── RPECalculator (reward error monitoring)             │  │
│  │  └── SurvivalScorer (factual bias)                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  gRPC Client                                              │  │
│  │  └── ChinjuSidecarClient (proto stubs)                   │  │
│  └───────────────────────┬──────────────────────────────────┘  │
└──────────────────────────┼──────────────────────────────────────┘
                           │ gRPC (port 50051)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      chinju-sidecar                              │
│  ├── CapabilityEvaluator (Rust)                                 │
│  ├── ContradictionController (Rust)                             │
│  ├── ValueNeuronMonitor (Rust)                                  │
│  └── SurvivalAttention (Rust)                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 実装ロードマップ

### Phase 1: Proto統合（完了）
- [x] proto定義からPython stubs生成
- [x] gRPC clientのプレースホルダー実装
- [x] 統合テスト準備

### Phase 2: C14実装
- [ ] CapabilityEvaluatorクラス実装
- [ ] トークン複雑性計算（L1）
- [ ] HiddenStateExtractor連携（L2）
- [ ] 直接停止制御

### Phase 3: C16実装
- [ ] ContradictionControllerクラス実装
- [ ] 矛盾パターン生成
- [ ] 崩壊検知
- [ ] 自動復旧

### Phase 4: 統合テスト
- [ ] モックsidecarでのユニットテスト
- [ ] 実sidecarでの統合テスト
- [ ] パフォーマンステスト

---

## セキュリティ考慮事項

1. **認証**: gRPC接続にはTLS必須（本番環境）
2. **監査**: すべてのC14/C16操作は監査ログに記録
3. **レート制限**: 矛盾注入にはレート制限を適用
4. **復旧手順**: Dead Man's Switch発動時の手動復旧手順を文書化

---

## 変更履歴

| Date | Version | Description |
|:--|:--|:--|
| 2026-02-16 | 0.1.0 | 初版作成 |
