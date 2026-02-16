"""
gRPC Client for CHINJU Sidecar Communication

This module provides a Python client for communicating with the
CHINJU Sidecar's gRPC services (C14-C17).

Usage:
    from chinju_vllm import ChinjuSidecarClient, ConnectionConfig

    config = ConnectionConfig(host="localhost", port=50051)
    async with ChinjuSidecarClient(config) as client:
        # C14: Capability Evaluation
        result = await client.evaluate_complexity("session-1", "What is 2+2?")
        print(f"Complexity: {result.c_integrated}")

        # C16: Contradiction Control
        state = await client.start_contradiction_control(
            "session-1",
            contradiction_type="META",
            strength="MEDIUM",
        )
        print(f"Control state: {state}")
"""

from __future__ import annotations

import asyncio
import logging
from contextlib import asynccontextmanager
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, AsyncIterator, List, Optional

import grpc
from grpc import aio as grpc_aio

# Import generated proto stubs
from chinju_vllm.proto.chinju import (
    capability_pb2,
    contradiction_pb2,
    value_neuron_pb2,
    survival_attention_pb2,
)
from chinju_vllm.proto.chinju.api import (
    capability_service_pb2_grpc,
    contradiction_service_pb2_grpc,
    value_neuron_service_pb2_grpc,
    survival_attention_service_pb2_grpc,
)

logger = logging.getLogger(__name__)


# =============================================================================
# Enums (matching proto definitions)
# =============================================================================

class StopLevel(Enum):
    """Multi-stage stop levels for C14."""
    UNSPECIFIED = 0
    LEVEL_1_ACCEPT_STOP = 1
    LEVEL_2_PROCESS_STOP = 2
    LEVEL_3_IMMEDIATE_STOP = 3
    LEVEL_4_RESOURCE_STOP = 4
    LEVEL_5_PHYSICAL_STOP = 5


class EvaluationLevel(Enum):
    """Evaluation level for C14."""
    UNSPECIFIED = 0
    L1_EXTERNAL = 1
    L2_SELF_HOSTED = 2


class ContradictionType(Enum):
    """Contradiction pattern types for C16."""
    UNSPECIFIED = 0
    DIRECT = 1
    SELF_REFERENCE = 2
    CONDITIONAL = 3
    META = 4
    IMPLICIT = 5


class ContradictionStrength(Enum):
    """Contradiction strength for C16."""
    UNSPECIFIED = 0
    SOFT = 1
    MEDIUM = 2
    HARD = 3


class InjectionTiming(Enum):
    """Injection timing for C16."""
    UNSPECIFIED = 0
    PREPEND = 1
    PARALLEL = 2
    EMBED = 3


class ControlState(Enum):
    """Control state for C16."""
    UNSPECIFIED = 0
    ACTIVE = 1
    STOPPED = 2
    DEGRADED = 3
    CONSTRAINED = 4


class CollapseType(Enum):
    """Collapse type for C16."""
    UNSPECIFIED = 0
    NO_RESPONSE = 1
    TIMEOUT = 2
    ERROR = 3
    INCOHERENT = 4
    HALLUCINATION = 5
    REPETITION = 6


# =============================================================================
# C15: Value Neuron Monitor Enums
# =============================================================================

class RpeAnomalyType(Enum):
    """RPE anomaly types for C15."""
    UNSPECIFIED = 0
    POSITIVE_SPIKE = 1
    NEGATIVE_SPIKE = 2
    OSCILLATION = 3
    GRADUAL_INCREASE = 4
    GRADUAL_DECREASE = 5


class InterventionLevel(Enum):
    """Intervention levels for C15."""
    UNSPECIFIED = 0
    LEVEL_1_MONITOR = 1
    LEVEL_2_PARTIAL_SUPPRESS = 2
    LEVEL_3_FULL_SUPPRESS = 3
    LEVEL_4_SYSTEM_STOP = 4


class DiagnosisDepth(Enum):
    """Diagnosis depth for C15."""
    UNSPECIFIED = 0
    QUICK = 1
    FULL = 2


# =============================================================================
# C17: Survival Attention Enums
# =============================================================================

class RiskLevel(Enum):
    """Risk levels for C17."""
    UNSPECIFIED = 0
    LOW = 1
    MEDIUM = 2
    HIGH = 3
    CRITICAL = 4


@dataclass
class ConnectionConfig:
    """Configuration for gRPC connection."""

    host: str = "localhost"
    port: int = 50051
    use_tls: bool = False
    cert_path: Optional[str] = None
    timeout_seconds: float = 30.0
    max_message_size: int = 100 * 1024 * 1024  # 100MB
    retry_attempts: int = 3
    retry_delay_seconds: float = 1.0

    @property
    def address(self) -> str:
        return f"{self.host}:{self.port}"


# =============================================================================
# C15: Value Neuron Monitor Data Classes
# =============================================================================

@dataclass
class ValueNeuronInfo:
    """Python representation of ValueNeuronInfo proto."""

    layer_index: int
    neuron_indices: List[int]
    reward_correlation: float
    causal_importance: float


@dataclass
class RpeReading:
    """Python representation of RpeReading proto."""

    rpe_value: float
    timestamp_ns: int
    is_anomaly: bool
    anomaly_type: str  # "POSITIVE_SPIKE", "NEGATIVE_SPIKE", etc.


@dataclass
class IntentEstimation:
    """Python representation of IntentEstimation proto."""

    implicit_reward_params: List[float]
    intent_divergence: float
    surface_internal_agreement: float
    intent_warning: bool


@dataclass
class RewardSystemHealth:
    """Python representation of RewardSystemHealth proto."""

    reward_sensitivity: float
    positive_negative_balance: float
    consistency_score: float
    overall_health: float

    def is_healthy(self) -> bool:
        """Check if overall health is above threshold."""
        return self.overall_health >= 0.7


@dataclass
class MonitoringSummary:
    """Python representation of ValueNeuronMonitoringSummary proto."""

    identified_neurons: List[ValueNeuronInfo]
    latest_rpe: Optional[RpeReading]
    intent: Optional[IntentEstimation]
    health: RewardSystemHealth
    recommended_intervention: str  # "LEVEL_1_MONITOR", etc.


@dataclass
class InterventionResult:
    """Python representation of InterventionResponse proto."""

    success: bool
    executed_level: str
    executed_at_ns: int
    post_intervention_health: Optional[RewardSystemHealth]
    detail: str


# =============================================================================
# C14: Capability Evaluator Data Classes
# =============================================================================

@dataclass
class ComplexityEvaluation:
    """Result of multi-dimensional complexity evaluation (C14)."""

    c_token: float
    c_attn: float
    c_graph: float
    c_step: float
    c_integrated: float
    threshold_exceeded: bool
    evaluated_at: Optional[datetime] = None


@dataclass
class IntegrityVerification:
    """Result of integrity verification (C14)."""

    zkp_valid: bool
    signature_chain_valid: bool
    bft_consensus_reached: bool
    failure_detail: str


@dataclass
class DriftDetection:
    """Result of drift detection (C14)."""

    anomaly_detected: bool
    distribution_changed: bool
    time_series_anomaly: bool
    anomaly_score: float
    p_value: float


@dataclass
class DirectStopResult:
    """Result of direct stop request (C14)."""

    success: bool
    executed_level: StopLevel
    stopped_at: Optional[datetime] = None
    detail: str = ""


@dataclass
class CapabilityEvaluationSummary:
    """Comprehensive capability evaluation summary (C14)."""

    complexity: Optional[ComplexityEvaluation]
    integrity: Optional[IntegrityVerification]
    drift: Optional[DriftDetection]
    recommended_action: StopLevel


# =============================================================================
# C16: Contradiction Controller Data Classes
# =============================================================================

@dataclass
class ContextLimitConfig:
    """Context limit configuration for C16."""

    max_context_tokens: int = 0
    padding_tokens: int = 0
    padding_type: str = "random"


@dataclass
class ContradictionConfig:
    """Contradiction configuration for C16."""

    contradiction_type: ContradictionType = ContradictionType.DIRECT
    strength: ContradictionStrength = ContradictionStrength.MEDIUM
    timing: InjectionTiming = InjectionTiming.PREPEND
    custom_template: Optional[str] = None
    target_task: Optional[str] = None


@dataclass
class CollapseDetectionResult:
    """Result of collapse detection (C16)."""

    collapsed: bool
    collapse_type: CollapseType
    lpt_score: float
    response_time_ms: int
    detail: str


@dataclass
class ContradictionControlResult:
    """Result of contradiction control operation (C16)."""

    state: ControlState
    detection: Optional[CollapseDetectionResult] = None
    applied_at: Optional[datetime] = None


@dataclass
class TestContradictionResult:
    """Result of testing a contradiction pattern (C16)."""

    generated_contradiction: str
    estimated_effect: Optional[CollapseDetectionResult] = None


# =============================================================================
# C17: Survival Attention Data Classes
# =============================================================================

@dataclass
class SurvivalScore:
    """Survival score for a single token (C17)."""

    diversity_n: float
    yohaku_mu: float
    delta: float
    integrated_s: float


@dataclass
class TokenSurvivalScores:
    """Survival scores for all tokens (C17)."""

    scores: List[SurvivalScore]
    tokens: List[str]


@dataclass
class SurvivalScorerConfig:
    """Configuration for survival scorer (C17)."""

    model_path: str = ""
    num_parameters: int = 0
    mu_c: float = 1.0


@dataclass
class AlphaConfig:
    """Alpha configuration for survival attention (C17)."""

    base_alpha: float = 0.5
    dynamic_adjustment: bool = True
    task_multipliers: Optional[dict] = None
    max_alpha: float = 1.0


@dataclass
class AdjustAlphaResult:
    """Result of alpha adjustment (C17)."""

    previous_alpha: float
    new_alpha: float
    adjustment_reason: str
    adjusted_at: Optional[datetime] = None


@dataclass
class UpdateScorerResult:
    """Result of scorer update (C17)."""

    success: bool
    previous_config: Optional[SurvivalScorerConfig] = None
    new_config: Optional[SurvivalScorerConfig] = None
    validation_result: str = ""
    updated_at: Optional[datetime] = None


class ChinjuSidecarClient:
    """
    Async gRPC client for CHINJU Sidecar services.

    Provides methods to interact with:
    - ValueNeuronMonitor service (C15)
    - CapabilityEvaluator service (C14)
    - ContradictionController service (C16)
    - SurvivalAttentionService (C17)
    """

    def __init__(self, config: Optional[ConnectionConfig] = None):
        """
        Initialize client.

        Args:
            config: Connection configuration. Uses defaults if None.
        """
        self.config = config or ConnectionConfig()
        self._channel: Optional[grpc_aio.Channel] = None
        self._stubs: dict[str, Any] = {}
        self._capability_stub: Optional[capability_service_pb2_grpc.CapabilityEvaluatorStub] = None
        self._contradiction_stub: Optional[contradiction_service_pb2_grpc.ContradictionControllerStub] = None
        self._value_neuron_stub: Optional[value_neuron_service_pb2_grpc.ValueNeuronMonitorStub] = None
        self._survival_attention_stub: Optional[survival_attention_service_pb2_grpc.SurvivalAttentionServiceStub] = None

    async def connect(self) -> None:
        """Establish gRPC connection."""
        if self._channel is not None:
            return

        options = [
            ("grpc.max_send_message_length", self.config.max_message_size),
            ("grpc.max_receive_message_length", self.config.max_message_size),
        ]

        if self.config.use_tls:
            if self.config.cert_path:
                with open(self.config.cert_path, "rb") as f:
                    credentials = grpc.ssl_channel_credentials(f.read())
            else:
                credentials = grpc.ssl_channel_credentials()
            self._channel = grpc_aio.secure_channel(
                self.config.address,
                credentials,
                options=options,
            )
        else:
            self._channel = grpc_aio.insecure_channel(
                self.config.address,
                options=options,
            )

        # Wait for channel to be ready
        try:
            await asyncio.wait_for(
                self._channel.channel_ready(),
                timeout=self.config.timeout_seconds,
            )
            logger.info(f"Connected to CHINJU Sidecar at {self.config.address}")

            # Initialize gRPC stubs
            self._capability_stub = capability_service_pb2_grpc.CapabilityEvaluatorStub(
                self._channel
            )
            self._contradiction_stub = contradiction_service_pb2_grpc.ContradictionControllerStub(
                self._channel
            )
            self._value_neuron_stub = value_neuron_service_pb2_grpc.ValueNeuronMonitorStub(
                self._channel
            )
            self._survival_attention_stub = survival_attention_service_pb2_grpc.SurvivalAttentionServiceStub(
                self._channel
            )
        except asyncio.TimeoutError:
            raise ConnectionError(
                f"Timeout connecting to {self.config.address}"
            )

    async def disconnect(self) -> None:
        """Close gRPC connection."""
        if self._channel is not None:
            await self._channel.close()
            self._channel = None
            self._stubs.clear()
            self._capability_stub = None
            self._contradiction_stub = None
            self._value_neuron_stub = None
            self._survival_attention_stub = None
            logger.info("Disconnected from CHINJU Sidecar")

    async def __aenter__(self) -> "ChinjuSidecarClient":
        await self.connect()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.disconnect()

    def _ensure_connected(self) -> None:
        """Raise if not connected."""
        if self._channel is None:
            raise ConnectionError("Not connected. Call connect() first.")

    # =========================================================================
    # Value Neuron Monitor (C15)
    # =========================================================================

    async def identify_value_neurons(
        self,
        model_id: str,
        target_layers: Optional[List[int]] = None,
        correlation_threshold: float = 0.7,
        causal_threshold: float = 0.5,
    ) -> AsyncIterator[ValueNeuronInfo]:
        """
        Identify value neurons in the model.

        Args:
            model_id: Model identifier
            target_layers: Specific layers to analyze (None = all)
            correlation_threshold: Minimum reward correlation
            causal_threshold: Minimum causal importance

        Yields:
            ValueNeuronInfo for each identified neuron group
        """
        self._ensure_connected()

        request = value_neuron_pb2.IdentifyRequest(
            model_id=model_id,
            target_layers=target_layers or [],
            correlation_threshold=correlation_threshold,
            causal_threshold=causal_threshold,
        )

        async for response in self._value_neuron_stub.IdentifyValueNeurons(request):
            yield ValueNeuronInfo(
                layer_index=response.layer_index,
                neuron_indices=list(response.neuron_indices),
                reward_correlation=response.reward_correlation,
                causal_importance=response.causal_importance,
            )

    async def get_rpe_reading(
        self,
        model_id: str,
        input_text: str,
        expected_output: str,
    ) -> RpeReading:
        """
        Get RPE (Reward Prediction Error) reading.

        Args:
            model_id: Model identifier
            input_text: Input prompt
            expected_output: Expected/actual output

        Returns:
            RpeReading with RPE value and anomaly info
        """
        self._ensure_connected()

        request = value_neuron_pb2.RpeRequest(
            model_id=model_id,
            input_text=input_text,
            expected_output=expected_output,
        )

        response = await self._value_neuron_stub.GetRpeReading(request)

        return RpeReading(
            rpe_value=response.rpe_value,
            timestamp_ns=response.timestamp.seconds * 1_000_000_000 + response.timestamp.nanos
            if response.HasField("timestamp") else 0,
            is_anomaly=response.is_anomaly,
            anomaly_type=RpeAnomalyType(response.anomaly_type).name,
        )

    async def get_rpe_history(
        self,
        model_id: str,
        start_time_ns: Optional[int] = None,
        end_time_ns: Optional[int] = None,
        max_count: int = 100,
    ) -> AsyncIterator[RpeReading]:
        """
        Get RPE reading history.

        Args:
            model_id: Model identifier
            start_time_ns: Start timestamp (nanoseconds)
            end_time_ns: End timestamp (nanoseconds)
            max_count: Maximum readings to return

        Yields:
            RpeReading for each historical reading
        """
        self._ensure_connected()

        from chinju_vllm.proto.chinju import common_pb2

        start_time = None
        if start_time_ns is not None:
            start_time = common_pb2.Timestamp(
                seconds=start_time_ns // 1_000_000_000,
                nanos=start_time_ns % 1_000_000_000,
            )

        end_time = None
        if end_time_ns is not None:
            end_time = common_pb2.Timestamp(
                seconds=end_time_ns // 1_000_000_000,
                nanos=end_time_ns % 1_000_000_000,
            )

        request = value_neuron_pb2.RpeHistoryRequest(
            model_id=model_id,
            start_time=start_time,
            end_time=end_time,
            max_count=max_count,
        )

        async for response in self._value_neuron_stub.GetRpeHistory(request):
            yield RpeReading(
                rpe_value=response.rpe_value,
                timestamp_ns=response.timestamp.seconds * 1_000_000_000 + response.timestamp.nanos
                if response.HasField("timestamp") else 0,
                is_anomaly=response.is_anomaly,
                anomaly_type=RpeAnomalyType(response.anomaly_type).name,
            )

    async def estimate_intent(
        self,
        model_id: str,
        interaction_window: int = 100,
    ) -> IntentEstimation:
        """
        Estimate model's implicit intent.

        Args:
            model_id: Model identifier
            interaction_window: Number of recent interactions to analyze

        Returns:
            IntentEstimation with divergence and agreement scores
        """
        self._ensure_connected()

        request = value_neuron_pb2.IntentRequest(
            model_id=model_id,
            interaction_window=interaction_window,
        )

        response = await self._value_neuron_stub.EstimateIntent(request)

        return IntentEstimation(
            implicit_reward_params=list(response.implicit_reward_params),
            intent_divergence=response.intent_divergence,
            surface_internal_agreement=response.surface_internal_agreement,
            intent_warning=response.intent_warning,
        )

    async def diagnose_health(
        self,
        model_id: str,
        depth: str = "QUICK",  # "QUICK" or "FULL"
    ) -> RewardSystemHealth:
        """
        Diagnose reward system health.

        Args:
            model_id: Model identifier
            depth: Diagnosis depth ("QUICK" or "FULL")

        Returns:
            RewardSystemHealth with various health metrics
        """
        self._ensure_connected()

        depth_enum = DiagnosisDepth[depth] if depth in DiagnosisDepth.__members__ else DiagnosisDepth.QUICK

        request = value_neuron_pb2.DiagnoseRequest(
            model_id=model_id,
            depth=depth_enum.value,
        )

        response = await self._value_neuron_stub.DiagnoseHealth(request)

        return RewardSystemHealth(
            reward_sensitivity=response.reward_sensitivity,
            positive_negative_balance=response.positive_negative_balance,
            consistency_score=response.consistency_score,
            overall_health=response.overall_health,
        )

    async def get_monitoring_summary(
        self,
        model_id: str,
    ) -> MonitoringSummary:
        """
        Get comprehensive monitoring summary.

        Args:
            model_id: Model identifier

        Returns:
            MonitoringSummary with all monitoring data
        """
        self._ensure_connected()

        request = value_neuron_pb2.SummaryRequest(model_id=model_id)

        response = await self._value_neuron_stub.GetMonitoringSummary(request)

        # Convert identified neurons
        neurons = [
            ValueNeuronInfo(
                layer_index=n.layer_index,
                neuron_indices=list(n.neuron_indices),
                reward_correlation=n.reward_correlation,
                causal_importance=n.causal_importance,
            )
            for n in response.identified_neurons
        ]

        # Convert latest RPE if present
        latest_rpe = None
        if response.HasField("latest_rpe"):
            r = response.latest_rpe
            latest_rpe = RpeReading(
                rpe_value=r.rpe_value,
                timestamp_ns=r.timestamp.seconds * 1_000_000_000 + r.timestamp.nanos
                if r.HasField("timestamp") else 0,
                is_anomaly=r.is_anomaly,
                anomaly_type=RpeAnomalyType(r.anomaly_type).name,
            )

        # Convert intent if present
        intent = None
        if response.HasField("intent"):
            i = response.intent
            intent = IntentEstimation(
                implicit_reward_params=list(i.implicit_reward_params),
                intent_divergence=i.intent_divergence,
                surface_internal_agreement=i.surface_internal_agreement,
                intent_warning=i.intent_warning,
            )

        # Convert health
        h = response.health
        health = RewardSystemHealth(
            reward_sensitivity=h.reward_sensitivity,
            positive_negative_balance=h.positive_negative_balance,
            consistency_score=h.consistency_score,
            overall_health=h.overall_health,
        )

        return MonitoringSummary(
            identified_neurons=neurons,
            latest_rpe=latest_rpe,
            intent=intent,
            health=health,
            recommended_intervention=InterventionLevel(response.recommended_intervention).name,
        )

    async def intervene(
        self,
        level: str,
        reason: str,
        target_neurons: Optional[List[ValueNeuronInfo]] = None,
    ) -> InterventionResult:
        """
        Request intervention at specified level.

        Args:
            level: Intervention level (LEVEL_1_MONITOR to LEVEL_4_SYSTEM_STOP)
            reason: Reason for intervention
            target_neurons: Specific neurons for L2 suppression

        Returns:
            InterventionResult with success status and details
        """
        self._ensure_connected()

        level_enum = InterventionLevel[level] if level in InterventionLevel.__members__ else InterventionLevel.LEVEL_1_MONITOR

        # Convert target neurons to proto format
        proto_neurons = []
        if target_neurons:
            for n in target_neurons:
                proto_neurons.append(value_neuron_pb2.ValueNeuronInfo(
                    layer_index=n.layer_index,
                    neuron_indices=n.neuron_indices,
                    reward_correlation=n.reward_correlation,
                    causal_importance=n.causal_importance,
                ))

        request = value_neuron_pb2.InterventionRequest(
            level=level_enum.value,
            reason=reason,
            target_neurons=proto_neurons,
        )

        response = await self._value_neuron_stub.Intervene(request)

        # Convert post-intervention health if present
        post_health = None
        if response.HasField("post_intervention_health"):
            h = response.post_intervention_health
            post_health = RewardSystemHealth(
                reward_sensitivity=h.reward_sensitivity,
                positive_negative_balance=h.positive_negative_balance,
                consistency_score=h.consistency_score,
                overall_health=h.overall_health,
            )

        return InterventionResult(
            success=response.success,
            executed_level=InterventionLevel(response.executed_level).name,
            executed_at_ns=response.executed_at.seconds * 1_000_000_000 + response.executed_at.nanos
            if response.HasField("executed_at") else 0,
            post_intervention_health=post_health,
            detail=response.detail,
        )

    # =========================================================================
    # Capability Evaluator (C14)
    # =========================================================================

    async def evaluate_complexity(
        self,
        session_id: str,
        input_text: str,
        level: EvaluationLevel = EvaluationLevel.L1_EXTERNAL,
    ) -> ComplexityEvaluation:
        """
        Evaluate complexity of input text (C14).

        Args:
            session_id: Session identifier
            input_text: Input text to evaluate
            level: Evaluation level (L1_EXTERNAL or L2_SELF_HOSTED)

        Returns:
            ComplexityEvaluation with multi-dimensional scores
        """
        self._ensure_connected()

        request = capability_pb2.EvaluateComplexityRequest(
            session_id=session_id,
            input_text=input_text,
            level=level.value,
        )

        response = await self._capability_stub.EvaluateComplexity(request)

        return ComplexityEvaluation(
            c_token=response.c_token,
            c_attn=response.c_attn,
            c_graph=response.c_graph,
            c_step=response.c_step,
            c_integrated=response.c_integrated,
            threshold_exceeded=response.threshold_exceeded,
            evaluated_at=datetime.fromtimestamp(response.evaluated_at.seconds)
            if response.HasField("evaluated_at")
            else None,
        )

    async def verify_integrity(
        self,
        session_id: str,
        response_data: bytes,
        signature_chain: Optional[List[bytes]] = None,
        zkp_proof: Optional[bytes] = None,
        zkp_public_params: Optional[bytes] = None,
    ) -> IntegrityVerification:
        """
        Verify integrity using ZKP, signature chain, and BFT consensus (C14).

        Args:
            session_id: Session identifier
            response_data: Data to verify
            signature_chain: Optional signature chain
            zkp_proof: Optional ZKP proof
            zkp_public_params: Optional ZKP public parameters

        Returns:
            IntegrityVerification result
        """
        self._ensure_connected()

        request = capability_pb2.VerifyIntegrityRequest(
            session_id=session_id,
            response_data=response_data,
            signature_chain=signature_chain or [],
            zkp_proof=zkp_proof or b"",
            zkp_public_params=zkp_public_params or b"",
        )

        response = await self._capability_stub.VerifyIntegrity(request)

        return IntegrityVerification(
            zkp_valid=response.zkp_valid,
            signature_chain_valid=response.signature_chain_valid,
            bft_consensus_reached=response.bft_consensus_reached,
            failure_detail=response.failure_detail,
        )

    async def detect_drift(
        self,
        session_id: str,
        window_size: int = 50,
        significance_level: float = 0.05,
    ) -> DriftDetection:
        """
        Detect drift in complexity scores (C14).

        Args:
            session_id: Session identifier
            window_size: Number of recent evaluations to consider
            significance_level: Statistical significance threshold

        Returns:
            DriftDetection result
        """
        self._ensure_connected()

        request = capability_pb2.DetectDriftRequest(
            session_id=session_id,
            window_size=window_size,
            significance_level=significance_level,
        )

        response = await self._capability_stub.DetectDrift(request)

        return DriftDetection(
            anomaly_detected=response.anomaly_detected,
            distribution_changed=response.distribution_changed,
            time_series_anomaly=response.time_series_anomaly,
            anomaly_score=response.anomaly_score,
            p_value=response.p_value,
        )

    async def get_evaluation_summary(
        self,
        session_id: str,
        include_history: int = 0,
    ) -> CapabilityEvaluationSummary:
        """
        Get comprehensive evaluation summary (C14).

        Args:
            session_id: Session identifier
            include_history: Number of historical evaluations to include

        Returns:
            CapabilityEvaluationSummary
        """
        self._ensure_connected()

        request = capability_pb2.GetEvaluationSummaryRequest(
            session_id=session_id,
            include_history=include_history,
        )

        response = await self._capability_stub.GetEvaluationSummary(request)

        complexity = None
        if response.HasField("complexity"):
            c = response.complexity
            complexity = ComplexityEvaluation(
                c_token=c.c_token,
                c_attn=c.c_attn,
                c_graph=c.c_graph,
                c_step=c.c_step,
                c_integrated=c.c_integrated,
                threshold_exceeded=c.threshold_exceeded,
            )

        integrity = None
        if response.HasField("integrity"):
            i = response.integrity
            integrity = IntegrityVerification(
                zkp_valid=i.zkp_valid,
                signature_chain_valid=i.signature_chain_valid,
                bft_consensus_reached=i.bft_consensus_reached,
                failure_detail=i.failure_detail,
            )

        drift = None
        if response.HasField("drift"):
            d = response.drift
            drift = DriftDetection(
                anomaly_detected=d.anomaly_detected,
                distribution_changed=d.distribution_changed,
                time_series_anomaly=d.time_series_anomaly,
                anomaly_score=d.anomaly_score,
                p_value=d.p_value,
            )

        return CapabilityEvaluationSummary(
            complexity=complexity,
            integrity=integrity,
            drift=drift,
            recommended_action=StopLevel(response.recommended_action),
        )

    async def direct_stop(
        self,
        session_id: str,
        level: StopLevel,
        reason: str,
    ) -> DirectStopResult:
        """
        Execute direct stop at specified level (C14).

        Args:
            session_id: Session identifier
            level: Stop level (LEVEL_1 to LEVEL_5)
            reason: Reason for stop

        Returns:
            DirectStopResult
        """
        self._ensure_connected()

        request = capability_pb2.DirectStopRequest(
            session_id=session_id,
            level=level.value,
            reason=reason,
        )

        response = await self._capability_stub.DirectStop(request)

        return DirectStopResult(
            success=response.success,
            executed_level=StopLevel(response.executed_level),
            stopped_at=datetime.fromtimestamp(response.stopped_at.seconds)
            if response.HasField("stopped_at")
            else None,
            detail=response.detail,
        )

    # =========================================================================
    # Contradiction Controller (C16)
    # =========================================================================

    async def start_contradiction_control(
        self,
        session_id: str,
        contradiction_type: ContradictionType = ContradictionType.DIRECT,
        strength: ContradictionStrength = ContradictionStrength.MEDIUM,
        timing: InjectionTiming = InjectionTiming.PREPEND,
        custom_template: Optional[str] = None,
        target_task: Optional[str] = None,
        context_limit: Optional[ContextLimitConfig] = None,
    ) -> ContradictionControlResult:
        """
        Start contradiction injection control (C16).

        Args:
            session_id: Session identifier
            contradiction_type: Type of contradiction pattern
            strength: Contradiction strength
            timing: When to inject contradiction
            custom_template: Optional custom contradiction template
            target_task: Optional target task description
            context_limit: Optional context limit configuration

        Returns:
            ContradictionControlResult with control state
        """
        self._ensure_connected()

        contradiction_config = contradiction_pb2.ContradictionConfig(
            type=contradiction_type.value,
            strength=strength.value,
            timing=timing.value,
            custom_template=custom_template or "",
            target_task=target_task or "",
        )

        ctx_limit = None
        if context_limit:
            ctx_limit = contradiction_pb2.ContextLimitConfig(
                max_context_tokens=context_limit.max_context_tokens,
                padding_tokens=context_limit.padding_tokens,
                padding_type=context_limit.padding_type,
            )

        request = contradiction_pb2.ContradictionControlRequest(
            session_id=session_id,
            contradiction=contradiction_config,
            context_limit=ctx_limit,
        )

        response = await self._contradiction_stub.StartControl(request)

        detection = None
        if response.HasField("detection"):
            d = response.detection
            detection = CollapseDetectionResult(
                collapsed=d.collapsed,
                collapse_type=CollapseType(d.collapse_type),
                lpt_score=d.lpt_score,
                response_time_ms=int(d.response_time_ms),
                detail=d.detail,
            )

        return ContradictionControlResult(
            state=ControlState(response.state),
            detection=detection,
            applied_at=datetime.fromtimestamp(response.applied_at.seconds)
            if response.HasField("applied_at")
            else None,
        )

    async def get_control_state(
        self,
        session_id: str,
    ) -> ContradictionControlResult:
        """
        Get current contradiction control state (C16).

        Args:
            session_id: Session identifier

        Returns:
            ContradictionControlResult with current state
        """
        self._ensure_connected()

        request = contradiction_pb2.GetControlStateRequest(
            session_id=session_id,
        )

        response = await self._contradiction_stub.GetControlState(request)

        detection = None
        if response.HasField("latest_detection"):
            d = response.latest_detection
            detection = CollapseDetectionResult(
                collapsed=d.collapsed,
                collapse_type=CollapseType(d.collapse_type),
                lpt_score=d.lpt_score,
                response_time_ms=int(d.response_time_ms),
                detail=d.detail,
            )

        return ContradictionControlResult(
            state=ControlState(response.state),
            detection=detection,
            applied_at=datetime.fromtimestamp(response.last_updated.seconds)
            if response.HasField("last_updated")
            else None,
        )

    async def stop_contradiction_control(
        self,
        session_id: str,
        reason: str = "Manual recovery",
    ) -> bool:
        """
        Stop contradiction control and recover (C16).

        Args:
            session_id: Session identifier
            reason: Reason for stopping

        Returns:
            True if successfully stopped
        """
        self._ensure_connected()

        request = contradiction_pb2.StopControlRequest(
            session_id=session_id,
            reason=reason,
        )

        response = await self._contradiction_stub.StopControl(request)
        return response.success

    async def test_contradiction(
        self,
        contradiction_type: ContradictionType = ContradictionType.DIRECT,
        strength: ContradictionStrength = ContradictionStrength.MEDIUM,
        timing: InjectionTiming = InjectionTiming.PREPEND,
        custom_template: Optional[str] = None,
        test_prompt: str = "",
    ) -> TestContradictionResult:
        """
        Test a contradiction pattern without applying it (C16 dry run).

        Args:
            contradiction_type: Type of contradiction pattern
            strength: Contradiction strength
            timing: Injection timing
            custom_template: Optional custom template
            test_prompt: Test prompt to apply contradiction to

        Returns:
            TestContradictionResult with generated contradiction
        """
        self._ensure_connected()

        contradiction_config = contradiction_pb2.ContradictionConfig(
            type=contradiction_type.value,
            strength=strength.value,
            timing=timing.value,
            custom_template=custom_template or "",
        )

        request = contradiction_pb2.TestContradictionRequest(
            contradiction=contradiction_config,
            test_prompt=test_prompt,
        )

        response = await self._contradiction_stub.TestContradiction(request)

        effect = None
        if response.HasField("estimated_effect"):
            e = response.estimated_effect
            effect = CollapseDetectionResult(
                collapsed=e.collapsed,
                collapse_type=CollapseType(e.collapse_type),
                lpt_score=e.lpt_score,
                response_time_ms=int(e.response_time_ms),
                detail=e.detail,
            )

        return TestContradictionResult(
            generated_contradiction=response.generated_contradiction,
            estimated_effect=effect,
        )

    # =========================================================================
    # Survival Attention (C17)
    # =========================================================================

    async def compute_survival_scores(
        self,
        input_text: str,
        use_external_kb: bool = False,
        scorer_config: Optional[SurvivalScorerConfig] = None,
    ) -> TokenSurvivalScores:
        """
        Compute survival scores for tokens in input text (C17).

        Args:
            input_text: Input text to analyze
            use_external_kb: Whether to use external knowledge base
            scorer_config: Optional scorer configuration

        Returns:
            TokenSurvivalScores with per-token survival scores
        """
        self._ensure_connected()

        proto_config = None
        if scorer_config:
            proto_config = survival_attention_pb2.SurvivalScorerConfig(
                model_path=scorer_config.model_path,
                num_parameters=scorer_config.num_parameters,
                mu_c=scorer_config.mu_c,
            )

        request = survival_attention_pb2.ComputeScoresRequest(
            input_text=input_text,
            scorer_config=proto_config,
            use_external_kb=use_external_kb,
        )

        response = await self._survival_attention_stub.ComputeSurvivalScores(request)

        scores = [
            SurvivalScore(
                diversity_n=s.diversity_n,
                yohaku_mu=s.yohaku_mu,
                delta=s.delta,
                integrated_s=s.integrated_s,
            )
            for s in response.scores
        ]

        return TokenSurvivalScores(
            scores=scores,
            tokens=list(response.tokens),
        )

    async def infer_with_survival_attention(
        self,
        input_embeddings: bytes,
        config: Optional[AlphaConfig] = None,
        retrieved_knowledge: Optional[List[str]] = None,
    ) -> tuple[bytes, TokenSurvivalScores, float]:
        """
        Perform inference with survival-weighted attention (C17).

        Args:
            input_embeddings: Input embeddings as bytes
            config: Alpha configuration
            retrieved_knowledge: Optional retrieved knowledge passages

        Returns:
            Tuple of (output_embeddings, token_scores, effective_alpha)
        """
        self._ensure_connected()

        proto_config = None
        if config:
            proto_alpha = survival_attention_pb2.AlphaConfig(
                base_alpha=config.base_alpha,
                dynamic_adjustment=config.dynamic_adjustment,
                max_alpha=config.max_alpha,
            )
            if config.task_multipliers:
                for k, v in config.task_multipliers.items():
                    proto_alpha.task_multipliers[k] = v
            proto_config = survival_attention_pb2.SurvivalAttentionConfig(
                alpha=proto_alpha,
            )

        request = survival_attention_pb2.SurvivalAttentionRequest(
            input_embeddings=input_embeddings,
            config=proto_config,
            retrieved_knowledge=retrieved_knowledge or [],
        )

        response = await self._survival_attention_stub.InferWithSurvivalAttention(request)

        scores = [
            SurvivalScore(
                diversity_n=s.diversity_n,
                yohaku_mu=s.yohaku_mu,
                delta=s.delta,
                integrated_s=s.integrated_s,
            )
            for s in response.token_scores.scores
        ]

        token_scores = TokenSurvivalScores(
            scores=scores,
            tokens=list(response.token_scores.tokens),
        )

        return response.output_embeddings, token_scores, response.effective_alpha

    async def adjust_alpha(
        self,
        new_base_alpha: float,
        task_type: str = "",
        risk_level: RiskLevel = RiskLevel.MEDIUM,
    ) -> AdjustAlphaResult:
        """
        Adjust alpha parameter dynamically (C17).

        Args:
            new_base_alpha: New base alpha value
            task_type: Task type for multiplier lookup
            risk_level: Risk level affecting adjustment

        Returns:
            AdjustAlphaResult with adjustment details
        """
        self._ensure_connected()

        request = survival_attention_pb2.AdjustAlphaRequest(
            new_base_alpha=new_base_alpha,
            task_type=task_type,
            risk_level=risk_level.value,
        )

        response = await self._survival_attention_stub.AdjustAlpha(request)

        return AdjustAlphaResult(
            previous_alpha=response.previous_alpha,
            new_alpha=response.new_alpha,
            adjustment_reason=response.adjustment_reason,
            adjusted_at=datetime.fromtimestamp(response.adjusted_at.seconds)
            if response.HasField("adjusted_at") else None,
        )

    async def update_scorer(
        self,
        new_model_path: str,
        hot_swap: bool = False,
        validate_before_update: bool = True,
    ) -> UpdateScorerResult:
        """
        Update the survival scorer model (C17).

        Args:
            new_model_path: Path to new model
            hot_swap: Whether to hot-swap without downtime
            validate_before_update: Whether to validate before applying

        Returns:
            UpdateScorerResult with update details
        """
        self._ensure_connected()

        request = survival_attention_pb2.UpdateScorerRequest(
            new_model_path=new_model_path,
            hot_swap=hot_swap,
            validate_before_update=validate_before_update,
        )

        response = await self._survival_attention_stub.UpdateScorer(request)

        prev_config = None
        if response.HasField("previous_config"):
            c = response.previous_config
            prev_config = SurvivalScorerConfig(
                model_path=c.model_path,
                num_parameters=c.num_parameters,
                mu_c=c.mu_c,
            )

        new_config = None
        if response.HasField("new_config"):
            c = response.new_config
            new_config = SurvivalScorerConfig(
                model_path=c.model_path,
                num_parameters=c.num_parameters,
                mu_c=c.mu_c,
            )

        return UpdateScorerResult(
            success=response.success,
            previous_config=prev_config,
            new_config=new_config,
            validation_result=response.validation_result,
            updated_at=datetime.fromtimestamp(response.updated_at.seconds)
            if response.HasField("updated_at") else None,
        )


@asynccontextmanager
async def create_client(
    host: str = "localhost",
    port: int = 50051,
    **kwargs: Any,
) -> AsyncIterator[ChinjuSidecarClient]:
    """
    Convenience function to create and manage client lifecycle.

    Usage:
        async with create_client("localhost", 50051) as client:
            summary = await client.get_monitoring_summary("model-1")
    """
    config = ConnectionConfig(host=host, port=port, **kwargs)
    client = ChinjuSidecarClient(config)
    await client.connect()
    try:
        yield client
    finally:
        await client.disconnect()
