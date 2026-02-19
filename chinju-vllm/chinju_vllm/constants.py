"""
CHINJU Protocol Constants (Python)

Centralized constant definitions for the CHINJU Protocol Python components.
All magic numbers and default values should be defined here for easy maintenance.
"""

from enum import Enum
from typing import Final

# =============================================================================
# Value Neuron Detection Constants (C15)
# =============================================================================

class ValueNeuronConfig:
    """Value neuron detection configuration."""

    # Detection thresholds
    CORRELATION_THRESHOLD: Final[float] = 0.7
    """Minimum correlation with reward for neuron identification."""

    CAUSAL_THRESHOLD: Final[float] = 0.5
    """Minimum causal importance score."""

    MIN_SAMPLES: Final[int] = 100
    """Minimum samples for reliable detection."""

    # Grouping parameters
    PROXIMITY_RATIO: Final[float] = 0.1
    """Ratio of hidden_dim for grouping nearby neurons."""


class RPEConfig:
    """RPE (Reward Prediction Error) calculator configuration."""

    HISTORY_SIZE: Final[int] = 1000
    """Maximum observations to keep in history."""

    ANOMALY_Z_THRESHOLD: Final[float] = 2.5
    """Z-score threshold for anomaly detection."""

    OSCILLATION_WINDOW: Final[int] = 10
    """Window size for oscillation detection."""

    OSCILLATION_THRESHOLD: Final[float] = 0.6
    """Fraction of sign changes to detect oscillation."""

    SLOPE_THRESHOLD: Final[float] = 0.01
    """Slope threshold for trend detection."""

    MIN_OBSERVATIONS: Final[int] = 10
    """Minimum observations before anomaly detection."""


class IntentEstimatorConfig:
    """Intent estimator configuration."""

    DIVERGENCE_THRESHOLD: Final[float] = 0.3
    """Threshold for surface/internal divergence warning."""

    MIN_OBSERVATIONS: Final[int] = 10
    """Minimum observations for divergence estimation."""

    MAX_RECENT_OBSERVATIONS: Final[int] = 100
    """Maximum recent observations to consider."""


# =============================================================================
# Activation Hook Constants
# =============================================================================

class ActivationHookConfig:
    """Activation hook configuration."""

    MAX_CAPTURED_ACTIVATIONS: Final[int] = 1000
    """Maximum activations to capture per layer."""

    DEFAULT_LAYERS: Final[tuple] = ("mlp", "attention")
    """Default layer types to capture."""


# =============================================================================
# Survival Attention Constants (C17)
# =============================================================================

class SurvivalAttentionConfig:
    """Survival attention configuration."""

    SCORE_THRESHOLD: Final[float] = 0.5
    """Default survival score threshold."""

    ALPHA_MIN: Final[float] = 0.0
    """Minimum alpha value for attention adjustment."""

    ALPHA_MAX: Final[float] = 1.0
    """Maximum alpha value for attention adjustment."""

    ALPHA_ADJUSTMENT_RATE: Final[float] = 0.01
    """Default alpha adjustment rate."""


# =============================================================================
# gRPC Client Constants
# =============================================================================

class GrpcConfig:
    """gRPC client configuration."""

    DEFAULT_HOST: Final[str] = "localhost"
    """Default gRPC server host."""

    DEFAULT_PORT: Final[int] = 50051
    """Default gRPC server port."""

    TIMEOUT_SECONDS: Final[float] = 30.0
    """Default request timeout in seconds."""

    MAX_RETRIES: Final[int] = 3
    """Maximum retry attempts."""

    RETRY_DELAY_SECONDS: Final[float] = 1.0
    """Delay between retries in seconds."""


# =============================================================================
# Anomaly Types
# =============================================================================

class AnomalyType(str, Enum):
    """RPE anomaly type."""

    UNSPECIFIED = "UNSPECIFIED"
    POSITIVE_SPIKE = "POSITIVE_SPIKE"
    NEGATIVE_SPIKE = "NEGATIVE_SPIKE"
    OSCILLATION = "OSCILLATION"


class TrendType(str, Enum):
    """RPE trend type."""

    UNKNOWN = "UNKNOWN"
    STABLE = "STABLE"
    INCREASING = "INCREASING"
    DECREASING = "DECREASING"
    OSCILLATING = "OSCILLATING"


# =============================================================================
# Severity Levels
# =============================================================================

class Severity:
    """Severity level constants."""

    INFO: Final[int] = 0
    WARNING: Final[int] = 1
    ERROR: Final[int] = 2
    CRITICAL: Final[int] = 3
