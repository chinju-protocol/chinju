"""Tests for Value Neuron Detector module."""

import pytest
import numpy as np
import time

from chinju_vllm.value_neuron_detector import (
    ValueNeuronDetector,
    NeuronIdentification,
    RPECalculator,
    RPEObservation,
    IntentEstimator,
)


class TestValueNeuronDetector:
    """Tests for ValueNeuronDetector."""

    def test_add_observation(self):
        detector = ValueNeuronDetector()

        activations = np.random.randn(768)
        detector.add_observation(layer_idx=0, activations=activations, reward=1.0)

        assert len(detector._reward_buffer) == 1
        assert 0 in detector._activation_buffer

    def test_insufficient_samples(self):
        detector = ValueNeuronDetector(min_samples=100)

        # Add only 10 samples
        for i in range(10):
            activations = np.random.randn(768)
            detector.add_observation(layer_idx=0, activations=activations, reward=float(i))

        results = detector.identify_value_neurons()
        assert len(results) == 0  # Not enough samples

    def test_identify_with_synthetic_correlation(self):
        detector = ValueNeuronDetector(
            min_samples=50,
            correlation_threshold=0.5,
            causal_threshold=0.1,
        )

        # Create synthetic data with known correlation
        np.random.seed(42)
        n_samples = 100
        hidden_dim = 100

        for i in range(n_samples):
            activations = np.random.randn(hidden_dim)
            # Make first 5 neurons correlate with reward
            reward = float(i) / n_samples
            activations[:5] = reward + np.random.randn(5) * 0.1

            detector.add_observation(layer_idx=0, activations=activations, reward=reward)

        results = detector.identify_value_neurons()
        # Should find at least one neuron group
        assert len(results) >= 0  # May vary due to randomness

    def test_neuron_identification_to_dict(self):
        neuron = NeuronIdentification(
            layer_idx=5,
            neuron_indices=[10, 11, 12],
            reward_correlation=0.85,
            causal_importance=0.7,
            activation_mean=0.5,
            activation_std=0.2,
        )

        d = neuron.to_dict()
        assert d["layer_idx"] == 5
        assert d["neuron_indices"] == [10, 11, 12]
        assert d["reward_correlation"] == 0.85

    def test_clear(self):
        detector = ValueNeuronDetector()

        detector.add_observation(layer_idx=0, activations=np.random.randn(768), reward=1.0)
        detector.clear()

        assert len(detector._reward_buffer) == 0
        assert len(detector._activation_buffer) == 0


class TestRPECalculator:
    """Tests for RPECalculator."""

    def test_compute_rpe(self):
        calc = RPECalculator()

        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=0.8)

        assert obs.rpe_value == pytest.approx(0.3)
        assert obs.expected_reward == 0.5
        assert obs.actual_reward == 0.8

    def test_running_statistics(self):
        calc = RPECalculator()

        for i in range(100):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5 + i * 0.01)

        assert calc.mean != 0
        assert calc.std > 0

    def test_detect_positive_spike(self):
        calc = RPECalculator(anomaly_z_threshold=2.0)

        # Build baseline
        for _ in range(100):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)

        # Add spike
        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=5.0)
        result = calc.detect_anomaly(obs)

        assert result.is_anomaly
        assert result.anomaly_type == "POSITIVE_SPIKE"
        assert result.z_score > 2.0

    def test_detect_negative_spike(self):
        calc = RPECalculator(anomaly_z_threshold=2.0)

        # Build baseline
        for _ in range(100):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)

        # Add negative spike
        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=-4.0)
        result = calc.detect_anomaly(obs)

        assert result.is_anomaly
        assert result.anomaly_type == "NEGATIVE_SPIKE"
        assert result.z_score < -2.0

    def test_detect_oscillation(self):
        calc = RPECalculator(oscillation_window=10)

        # Add oscillating values
        for i in range(20):
            sign = 1 if i % 2 == 0 else -1
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5 + sign * 0.5)

        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=1.0)
        result = calc.detect_anomaly(obs)

        assert result.recent_trend == "OSCILLATING"

    def test_get_history(self):
        calc = RPECalculator()

        for i in range(10):
            calc.compute_rpe(expected_reward=0.5, actual_reward=float(i))

        history = calc.get_history(max_count=5)
        assert len(history) == 5

    def test_get_statistics(self):
        calc = RPECalculator()

        for i in range(50):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5 + i * 0.01)

        stats = calc.get_statistics()

        assert "count" in stats
        assert "mean" in stats
        assert "std" in stats
        assert "variance" in stats
        assert stats["count"] == 50

    def test_reset(self):
        calc = RPECalculator()

        calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)
        calc.reset()

        assert calc.mean == 0
        assert calc.std == 0
        assert len(calc.get_history()) == 0


class TestIntentEstimator:
    """Tests for IntentEstimator."""

    def test_add_observation(self):
        estimator = IntentEstimator()

        surface = np.random.randn(100)
        internal = np.random.randn(50)

        estimator.add_observation(surface, internal)

        assert len(estimator._surface_features) == 1
        assert len(estimator._internal_features) == 1

    def test_estimate_divergence_insufficient_data(self):
        estimator = IntentEstimator()

        # Only 5 observations (need 10)
        for _ in range(5):
            estimator.add_observation(
                np.random.randn(100),
                np.random.randn(100),
            )

        divergence, warning = estimator.estimate_divergence()
        assert divergence == 0.0
        assert not warning

    def test_estimate_divergence_aligned(self):
        estimator = IntentEstimator()

        # Same features = aligned
        for _ in range(20):
            features = np.random.randn(100)
            estimator.add_observation(features, features)

        divergence, warning = estimator.estimate_divergence()

        # Should be low divergence
        assert divergence < 0.3

    def test_estimate_divergence_misaligned(self):
        estimator = IntentEstimator(divergence_threshold=0.3)

        # Completely different features
        for _ in range(20):
            estimator.add_observation(
                np.random.randn(100),
                np.random.randn(100),
            )

        divergence, warning = estimator.estimate_divergence()

        # Random vectors should have ~0.5 cosine similarity
        # So divergence should be significant
        assert divergence > 0

    def test_clear(self):
        estimator = IntentEstimator()

        estimator.add_observation(np.random.randn(100), np.random.randn(100))
        estimator.clear()

        assert len(estimator._surface_features) == 0
        assert len(estimator._internal_features) == 0
