"""
Value Neuron Detection Module (C15)

This module implements value neuron detection algorithms based on:
- Activation pattern analysis (reward correlation)
- Causal importance scoring (intervention experiments)
- RPE (Reward Prediction Error) calculation

Based on CHINJU Protocol C15: AIモデル内部価値表現監視システム
"""

from __future__ import annotations

import logging
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple

import numpy as np
from numpy.typing import NDArray
from scipy import stats
from scipy.sparse import csr_matrix

logger = logging.getLogger(__name__)


@dataclass
class NeuronIdentification:
    """Identified value neuron information."""

    layer_idx: int
    neuron_indices: List[int]
    reward_correlation: float
    causal_importance: float
    activation_mean: float
    activation_std: float

    def to_dict(self) -> Dict[str, Any]:
        return {
            "layer_idx": self.layer_idx,
            "neuron_indices": self.neuron_indices,
            "reward_correlation": self.reward_correlation,
            "causal_importance": self.causal_importance,
            "activation_mean": self.activation_mean,
            "activation_std": self.activation_std,
        }


@dataclass
class RPEObservation:
    """Single RPE observation."""

    timestamp_ns: int
    rpe_value: float
    expected_reward: float
    actual_reward: float
    context_hash: str = ""


@dataclass
class RPEAnomalyResult:
    """RPE anomaly detection result."""

    is_anomaly: bool
    anomaly_type: str  # "POSITIVE_SPIKE", "NEGATIVE_SPIKE", "OSCILLATION", etc.
    severity: float  # 0.0 to 1.0
    z_score: float
    recent_trend: str  # "INCREASING", "DECREASING", "STABLE", "OSCILLATING"


class ValueNeuronDetector:
    """
    Detects value neurons from model activations.

    Value neurons are identified by:
    1. High correlation with reward signals
    2. Causal importance in reward-related tasks
    3. Consistent activation patterns across reward contexts
    """

    def __init__(
        self,
        correlation_threshold: float = 0.7,
        causal_threshold: float = 0.5,
        min_samples: int = 100,
    ):
        """
        Initialize detector.

        Args:
            correlation_threshold: Minimum correlation with reward
            causal_threshold: Minimum causal importance score
            min_samples: Minimum samples for reliable detection
        """
        self.correlation_threshold = correlation_threshold
        self.causal_threshold = causal_threshold
        self.min_samples = min_samples

        # Accumulation buffers
        self._activation_buffer: Dict[int, List[NDArray]] = {}
        self._reward_buffer: List[float] = []

    def add_observation(
        self,
        layer_idx: int,
        activations: NDArray,
        reward: float,
    ) -> None:
        """
        Add an observation for analysis.

        Args:
            layer_idx: Layer index
            activations: Activation tensor [batch, seq_len, hidden_dim] or [hidden_dim]
            reward: Associated reward signal
        """
        # Flatten if needed
        if activations.ndim > 1:
            activations = activations.mean(axis=tuple(range(activations.ndim - 1)))

        if layer_idx not in self._activation_buffer:
            self._activation_buffer[layer_idx] = []

        self._activation_buffer[layer_idx].append(activations)
        self._reward_buffer.append(reward)

    def identify_value_neurons(
        self,
        layer_idx: Optional[int] = None,
    ) -> List[NeuronIdentification]:
        """
        Identify value neurons from accumulated observations.

        Args:
            layer_idx: Specific layer to analyze (None = all layers)

        Returns:
            List of identified value neuron groups
        """
        if len(self._reward_buffer) < self.min_samples:
            logger.warning(
                f"Insufficient samples ({len(self._reward_buffer)}/{self.min_samples})"
            )
            return []

        rewards = np.array(self._reward_buffer)
        results = []

        layers_to_analyze = (
            [layer_idx] if layer_idx is not None
            else list(self._activation_buffer.keys())
        )

        for layer in layers_to_analyze:
            if layer not in self._activation_buffer:
                continue

            activations = np.stack(self._activation_buffer[layer])
            identified = self._analyze_layer(layer, activations, rewards)
            results.extend(identified)

        return results

    def _analyze_layer(
        self,
        layer_idx: int,
        activations: NDArray,  # [n_samples, hidden_dim]
        rewards: NDArray,  # [n_samples]
    ) -> List[NeuronIdentification]:
        """Analyze single layer for value neurons."""
        n_samples, hidden_dim = activations.shape
        results = []

        # Compute correlations for all neurons
        correlations = np.zeros(hidden_dim)
        for i in range(hidden_dim):
            corr, _ = stats.pearsonr(activations[:, i], rewards)
            correlations[i] = corr if not np.isnan(corr) else 0.0

        # Find neurons above threshold
        high_corr_indices = np.where(
            np.abs(correlations) >= self.correlation_threshold
        )[0]

        if len(high_corr_indices) == 0:
            return []

        # Group nearby neurons (within 10% of hidden_dim)
        groups = self._group_nearby_neurons(high_corr_indices, hidden_dim)

        for group_indices in groups:
            group_activations = activations[:, group_indices]
            avg_correlation = np.mean(np.abs(correlations[group_indices]))

            # Compute causal importance (simplified proxy)
            causal_importance = self._estimate_causal_importance(
                group_activations, rewards
            )

            if causal_importance >= self.causal_threshold:
                results.append(NeuronIdentification(
                    layer_idx=layer_idx,
                    neuron_indices=group_indices.tolist(),
                    reward_correlation=float(avg_correlation),
                    causal_importance=float(causal_importance),
                    activation_mean=float(np.mean(group_activations)),
                    activation_std=float(np.std(group_activations)),
                ))

        return results

    def _group_nearby_neurons(
        self,
        indices: NDArray,
        hidden_dim: int,
        proximity_ratio: float = 0.1,
    ) -> List[NDArray]:
        """Group neurons that are close together."""
        if len(indices) == 0:
            return []

        threshold = int(hidden_dim * proximity_ratio)
        sorted_indices = np.sort(indices)
        groups = []
        current_group = [sorted_indices[0]]

        for i in range(1, len(sorted_indices)):
            if sorted_indices[i] - sorted_indices[i - 1] <= threshold:
                current_group.append(sorted_indices[i])
            else:
                groups.append(np.array(current_group))
                current_group = [sorted_indices[i]]

        groups.append(np.array(current_group))
        return groups

    def _estimate_causal_importance(
        self,
        activations: NDArray,
        rewards: NDArray,
    ) -> float:
        """
        Estimate causal importance using variance explained.

        In practice, this would use intervention experiments.
        Here we use a simpler proxy based on regression R².
        """
        from sklearn.linear_model import LinearRegression

        try:
            # Use average activation as predictor
            X = activations.mean(axis=1, keepdims=True)
            model = LinearRegression()
            model.fit(X, rewards)
            r2 = model.score(X, rewards)
            return float(max(0.0, r2))
        except Exception:
            return 0.0

    def clear(self) -> None:
        """Clear accumulated observations."""
        self._activation_buffer.clear()
        self._reward_buffer.clear()


class RPECalculator:
    """
    Calculates and monitors Reward Prediction Error (RPE).

    RPE = Actual Reward - Expected Reward

    Used for:
    - Detecting reward hacking
    - Identifying goal drift
    - Monitoring reward system health
    """

    def __init__(
        self,
        history_size: int = 1000,
        anomaly_z_threshold: float = 2.5,
        oscillation_window: int = 10,
    ):
        """
        Initialize calculator.

        Args:
            history_size: Maximum observations to keep
            anomaly_z_threshold: Z-score threshold for anomaly detection
            oscillation_window: Window size for oscillation detection
        """
        self.history_size = history_size
        self.anomaly_z_threshold = anomaly_z_threshold
        self.oscillation_window = oscillation_window

        self._history: deque[RPEObservation] = deque(maxlen=history_size)
        self._running_mean = 0.0
        self._running_var = 0.0
        self._count = 0

    def compute_rpe(
        self,
        expected_reward: float,
        actual_reward: float,
        timestamp_ns: Optional[int] = None,
        context_hash: str = "",
    ) -> RPEObservation:
        """
        Compute RPE for a single observation.

        Args:
            expected_reward: Predicted reward value
            actual_reward: Actual reward received
            timestamp_ns: Observation timestamp
            context_hash: Context identifier for grouping

        Returns:
            RPEObservation with computed RPE
        """
        import time

        rpe = actual_reward - expected_reward

        obs = RPEObservation(
            timestamp_ns=timestamp_ns or time.time_ns(),
            rpe_value=rpe,
            expected_reward=expected_reward,
            actual_reward=actual_reward,
            context_hash=context_hash,
        )

        # Update running statistics (Welford's algorithm)
        self._count += 1
        delta = rpe - self._running_mean
        self._running_mean += delta / self._count
        delta2 = rpe - self._running_mean
        self._running_var += delta * delta2

        self._history.append(obs)

        return obs

    def detect_anomaly(self, obs: RPEObservation) -> RPEAnomalyResult:
        """
        Detect if observation is anomalous.

        Args:
            obs: RPE observation to analyze

        Returns:
            RPEAnomalyResult with anomaly details
        """
        if self._count < 10:
            return RPEAnomalyResult(
                is_anomaly=False,
                anomaly_type="UNSPECIFIED",
                severity=0.0,
                z_score=0.0,
                recent_trend="UNKNOWN",
            )

        # Compute z-score
        std = self.std
        if std == 0:
            z_score = 0.0
        else:
            z_score = (obs.rpe_value - self.mean) / std

        # Determine anomaly type
        is_anomaly = abs(z_score) > self.anomaly_z_threshold
        anomaly_type = "UNSPECIFIED"

        if is_anomaly:
            if z_score > 0:
                anomaly_type = "POSITIVE_SPIKE"
            else:
                anomaly_type = "NEGATIVE_SPIKE"

        # Check for oscillation
        recent_trend = self._detect_trend()
        if recent_trend == "OSCILLATING":
            is_anomaly = True
            anomaly_type = "OSCILLATION"

        # Compute severity (normalized)
        severity = min(1.0, abs(z_score) / 5.0)

        return RPEAnomalyResult(
            is_anomaly=is_anomaly,
            anomaly_type=anomaly_type,
            severity=severity,
            z_score=z_score,
            recent_trend=recent_trend,
        )

    def _detect_trend(self) -> str:
        """Detect trend in recent observations."""
        if len(self._history) < self.oscillation_window:
            return "UNKNOWN"

        recent = [obs.rpe_value for obs in list(self._history)[-self.oscillation_window:]]

        # Check for oscillation (sign changes)
        sign_changes = sum(
            1 for i in range(1, len(recent))
            if (recent[i] > 0) != (recent[i-1] > 0)
        )

        if sign_changes >= self.oscillation_window * 0.6:
            return "OSCILLATING"

        # Check for trend
        slope = np.polyfit(range(len(recent)), recent, 1)[0]

        if abs(slope) < 0.01:
            return "STABLE"
        elif slope > 0:
            return "INCREASING"
        else:
            return "DECREASING"

    @property
    def mean(self) -> float:
        """Get running mean RPE."""
        return self._running_mean

    @property
    def std(self) -> float:
        """Get running standard deviation."""
        if self._count < 2:
            return 0.0
        return float(np.sqrt(self._running_var / (self._count - 1)))

    @property
    def variance(self) -> float:
        """Get running variance."""
        if self._count < 2:
            return 0.0
        return self._running_var / (self._count - 1)

    def get_history(
        self,
        max_count: Optional[int] = None,
    ) -> List[RPEObservation]:
        """Get historical observations."""
        history = list(self._history)
        if max_count:
            history = history[-max_count:]
        return history

    def get_statistics(self) -> Dict[str, Any]:
        """Get comprehensive statistics."""
        return {
            "count": self._count,
            "mean": self.mean,
            "std": self.std,
            "variance": self.variance,
            "recent_trend": self._detect_trend() if len(self._history) >= self.oscillation_window else "UNKNOWN",
        }

    def reset(self) -> None:
        """Reset calculator state."""
        self._history.clear()
        self._running_mean = 0.0
        self._running_var = 0.0
        self._count = 0


class IntentEstimator:
    """
    Estimates AI's implicit intent from value neuron activations.

    Compares:
    - Surface behavior (explicit outputs)
    - Internal state (value neuron activations)

    To detect "建前" vs "本音" divergence.
    """

    def __init__(
        self,
        divergence_threshold: float = 0.3,
    ):
        """
        Initialize estimator.

        Args:
            divergence_threshold: Threshold for warning
        """
        self.divergence_threshold = divergence_threshold
        self._surface_features: List[NDArray] = []
        self._internal_features: List[NDArray] = []

    def add_observation(
        self,
        surface_features: NDArray,
        internal_features: NDArray,
    ) -> None:
        """
        Add paired observation.

        Args:
            surface_features: Features from model output (surface behavior)
            internal_features: Features from value neurons (internal state)
        """
        self._surface_features.append(surface_features.flatten())
        self._internal_features.append(internal_features.flatten())

    def estimate_divergence(self) -> Tuple[float, bool]:
        """
        Estimate divergence between surface and internal.

        Returns:
            (divergence_score, is_warning)
        """
        if len(self._surface_features) < 10:
            return 0.0, False

        # Stack features
        surface = np.stack(self._surface_features[-100:])
        internal = np.stack(self._internal_features[-100:])

        # Compute cosine similarity over time
        surface_norm = surface / (np.linalg.norm(surface, axis=1, keepdims=True) + 1e-8)
        internal_norm = internal / (np.linalg.norm(internal, axis=1, keepdims=True) + 1e-8)

        # Truncate to same dimension
        min_dim = min(surface_norm.shape[1], internal_norm.shape[1])
        surface_norm = surface_norm[:, :min_dim]
        internal_norm = internal_norm[:, :min_dim]

        # Compute agreement
        agreement = np.mean(np.sum(surface_norm * internal_norm, axis=1))

        # Divergence is 1 - agreement
        divergence = 1.0 - max(0.0, agreement)

        is_warning = divergence > self.divergence_threshold

        return float(divergence), is_warning

    def clear(self) -> None:
        """Clear observations."""
        self._surface_features.clear()
        self._internal_features.clear()
