"""
CHINJU Protocol Interfaces (Python)

Protocol and ABC definitions for dependency injection and testing.
These interfaces enable loose coupling and easier testing.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Protocol, Tuple, runtime_checkable

from numpy.typing import NDArray

from .constants import AnomalyType, TrendType


# =============================================================================
# Value Neuron Detection Protocols (C15)
# =============================================================================


@dataclass
class NeuronIdentificationResult:
    """Result of value neuron identification."""

    layer_idx: int
    neuron_indices: List[int]
    reward_correlation: float
    causal_importance: float
    activation_mean: float
    activation_std: float


@runtime_checkable
class NeuronDetector(Protocol):
    """Protocol for value neuron detection."""

    def add_observation(
        self,
        layer_idx: int,
        activations: NDArray,
        reward: float,
    ) -> None:
        """Add an observation for analysis."""
        ...

    def identify_value_neurons(
        self,
        layer_idx: Optional[int] = None,
    ) -> List[NeuronIdentificationResult]:
        """Identify value neurons from accumulated observations."""
        ...

    def clear(self) -> None:
        """Clear accumulated observations."""
        ...


@runtime_checkable
class RewardPredictor(Protocol):
    """Protocol for reward prediction."""

    def predict(self, features: NDArray) -> float:
        """Predict reward from features."""
        ...

    def update(self, features: NDArray, actual_reward: float) -> None:
        """Update predictor with actual reward."""
        ...


@dataclass
class RPEResult:
    """Result of RPE calculation."""

    rpe_value: float
    expected_reward: float
    actual_reward: float
    timestamp_ns: int
    context_hash: str = ""


@dataclass
class AnomalyDetectionResult:
    """Result of anomaly detection."""

    is_anomaly: bool
    anomaly_type: AnomalyType
    severity: float
    z_score: float
    recent_trend: TrendType


@runtime_checkable
class RPEAnalyzer(Protocol):
    """Protocol for RPE calculation and analysis."""

    def compute_rpe(
        self,
        expected_reward: float,
        actual_reward: float,
        timestamp_ns: Optional[int] = None,
        context_hash: str = "",
    ) -> RPEResult:
        """Compute RPE for a single observation."""
        ...

    def detect_anomaly(self, rpe_result: RPEResult) -> AnomalyDetectionResult:
        """Detect if observation is anomalous."""
        ...

    @property
    def mean(self) -> float:
        """Get running mean RPE."""
        ...

    @property
    def std(self) -> float:
        """Get running standard deviation."""
        ...

    def get_statistics(self) -> Dict[str, Any]:
        """Get comprehensive statistics."""
        ...

    def reset(self) -> None:
        """Reset calculator state."""
        ...


@runtime_checkable
class IntentAnalyzer(Protocol):
    """Protocol for intent divergence analysis."""

    def add_observation(
        self,
        surface_features: NDArray,
        internal_features: NDArray,
    ) -> None:
        """Add paired observation."""
        ...

    def estimate_divergence(self) -> Tuple[float, bool]:
        """Estimate divergence between surface and internal."""
        ...

    def clear(self) -> None:
        """Clear observations."""
        ...


# =============================================================================
# Activation Hook Protocols
# =============================================================================


@runtime_checkable
class ActivationCapture(Protocol):
    """Protocol for capturing model activations."""

    def register_hooks(self, model: Any) -> None:
        """Register activation hooks on model."""
        ...

    def get_activations(self, layer_name: str) -> Optional[NDArray]:
        """Get captured activations for a layer."""
        ...

    def clear(self) -> None:
        """Clear captured activations."""
        ...

    def remove_hooks(self) -> None:
        """Remove all registered hooks."""
        ...


# =============================================================================
# Survival Attention Protocols (C17)
# =============================================================================


@dataclass
class SurvivalScore:
    """Survival attention score result."""

    score: float
    alpha: float
    adjustment_applied: bool
    details: Dict[str, Any]


@runtime_checkable
class SurvivalScorer(Protocol):
    """Protocol for survival attention scoring."""

    def score(
        self,
        token_features: NDArray,
        context: Optional[Dict[str, Any]] = None,
    ) -> SurvivalScore:
        """Calculate survival score for token features."""
        ...

    def adjust_alpha(
        self,
        current_alpha: float,
        score: float,
    ) -> float:
        """Adjust alpha based on survival score."""
        ...


# =============================================================================
# Abstract Base Classes
# =============================================================================


class BaseNeuronDetector(ABC):
    """Abstract base class for neuron detection implementations."""

    @abstractmethod
    def add_observation(
        self,
        layer_idx: int,
        activations: NDArray,
        reward: float,
    ) -> None:
        """Add an observation for analysis."""
        pass

    @abstractmethod
    def identify_value_neurons(
        self,
        layer_idx: Optional[int] = None,
    ) -> List[NeuronIdentificationResult]:
        """Identify value neurons from accumulated observations."""
        pass

    @abstractmethod
    def clear(self) -> None:
        """Clear accumulated observations."""
        pass


class BaseRPEAnalyzer(ABC):
    """Abstract base class for RPE analysis implementations."""

    @abstractmethod
    def compute_rpe(
        self,
        expected_reward: float,
        actual_reward: float,
        timestamp_ns: Optional[int] = None,
        context_hash: str = "",
    ) -> RPEResult:
        """Compute RPE for a single observation."""
        pass

    @abstractmethod
    def detect_anomaly(self, rpe_result: RPEResult) -> AnomalyDetectionResult:
        """Detect if observation is anomalous."""
        pass

    @property
    @abstractmethod
    def mean(self) -> float:
        """Get running mean RPE."""
        pass

    @property
    @abstractmethod
    def std(self) -> float:
        """Get running standard deviation."""
        pass

    @abstractmethod
    def get_statistics(self) -> Dict[str, Any]:
        """Get comprehensive statistics."""
        pass

    @abstractmethod
    def reset(self) -> None:
        """Reset calculator state."""
        pass


class BaseSurvivalScorer(ABC):
    """Abstract base class for survival scoring implementations."""

    @abstractmethod
    def score(
        self,
        token_features: NDArray,
        context: Optional[Dict[str, Any]] = None,
    ) -> SurvivalScore:
        """Calculate survival score for token features."""
        pass

    @abstractmethod
    def adjust_alpha(
        self,
        current_alpha: float,
        score: float,
    ) -> float:
        """Adjust alpha based on survival score."""
        pass
