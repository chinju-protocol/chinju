"""
CHINJU Protocol vLLM Integration

This package provides integration between vLLM and the CHINJU Protocol
for AI safety monitoring, including:

- C14: Capability Evaluator (多次元能力評価)
- C15: Value Neuron Monitoring (価値ニューロン監視)
- C16: Contradiction Controller (構造的矛盾注入)
- C17: Survival Attention (存続性重み付け注意機構)

Requires self-hosted LLM environment with access to hidden states.
"""

from chinju_vllm.activation_hook import (
    ActivationHook,
    ActivationBuffer,
    HiddenStateExtractor,
)
from chinju_vllm.value_neuron_detector import (
    ValueNeuronDetector,
    NeuronIdentification,
    RPECalculator,
)
from chinju_vllm.grpc_client import (
    # Client
    ChinjuSidecarClient,
    ConnectionConfig,
    create_client,
    # C14: Capability Evaluator
    StopLevel,
    EvaluationLevel,
    ComplexityEvaluation,
    IntegrityVerification,
    DriftDetection,
    DirectStopResult,
    CapabilityEvaluationSummary,
    # C15: Value Neuron Monitor
    RpeAnomalyType,
    InterventionLevel,
    DiagnosisDepth,
    ValueNeuronInfo,
    RpeReading,
    IntentEstimation,
    RewardSystemHealth,
    MonitoringSummary,
    InterventionResult,
    # C16: Contradiction Controller
    ContradictionType,
    ContradictionStrength,
    InjectionTiming,
    ControlState,
    CollapseType,
    ContextLimitConfig,
    ContradictionConfig,
    CollapseDetectionResult,
    ContradictionControlResult,
    TestContradictionResult,
    # C17: Survival Attention
    RiskLevel,
    SurvivalScore,
    TokenSurvivalScores,
    SurvivalScorerConfig,
    AlphaConfig,
    AdjustAlphaResult,
    UpdateScorerResult,
)
from chinju_vllm.survival_attention import (
    SurvivalScorer,
    SurvivalAttentionLayer,
)

__version__ = "0.1.0"
__all__ = [
    # Activation Hook
    "ActivationHook",
    "ActivationBuffer",
    "HiddenStateExtractor",
    # Value Neuron Detection
    "ValueNeuronDetector",
    "NeuronIdentification",
    "RPECalculator",
    # gRPC Client
    "ChinjuSidecarClient",
    "ConnectionConfig",
    "create_client",
    # C14: Capability Evaluator
    "StopLevel",
    "EvaluationLevel",
    "ComplexityEvaluation",
    "IntegrityVerification",
    "DriftDetection",
    "DirectStopResult",
    "CapabilityEvaluationSummary",
    # C15: Value Neuron Monitor
    "RpeAnomalyType",
    "InterventionLevel",
    "DiagnosisDepth",
    "ValueNeuronInfo",
    "RpeReading",
    "IntentEstimation",
    "RewardSystemHealth",
    "MonitoringSummary",
    "InterventionResult",
    # C16: Contradiction Controller
    "ContradictionType",
    "ContradictionStrength",
    "InjectionTiming",
    "ControlState",
    "CollapseType",
    "ContextLimitConfig",
    "ContradictionConfig",
    "CollapseDetectionResult",
    "ContradictionControlResult",
    "TestContradictionResult",
    # C17: Survival Attention
    "RiskLevel",
    "SurvivalScore",
    "TokenSurvivalScores",
    "SurvivalScorerConfig",
    "AlphaConfig",
    "AdjustAlphaResult",
    "UpdateScorerResult",
    # Survival Attention Layer
    "SurvivalScorer",
    "SurvivalAttentionLayer",
]
