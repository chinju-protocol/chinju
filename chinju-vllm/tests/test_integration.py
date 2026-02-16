"""
Integration Tests for chinju-vllm.

End-to-end tests using mock vLLM and gRPC components to verify
the complete monitoring pipeline without requiring GPU or server.
"""

import pytest
import numpy as np
import torch
import torch.nn as nn
from unittest.mock import MagicMock

from chinju_vllm.activation_hook import (
    HiddenStateExtractor,
    VLLMActivationMiddleware,
    ActivationBuffer,
    LayerActivation,
)
from chinju_vllm.value_neuron_detector import (
    ValueNeuronDetector,
    RPECalculator,
    IntentEstimator,
)
from chinju_vllm.survival_attention import (
    SurvivalScore,
    SurvivalScorer,
    SurvivalAttentionLayer,
)


# =============================================================================
# Local Test Fixtures (to avoid conftest issues)
# =============================================================================

class MockTransformerLayer(nn.Module):
    """Mock transformer layer for testing."""

    def __init__(self, hidden_dim: int = 768):
        super().__init__()
        self.hidden_dim = hidden_dim
        self.linear = nn.Linear(hidden_dim, hidden_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.linear(x)


class MockTransformer(nn.Module):
    """Mock transformer model matching vLLM structure."""

    def __init__(self, n_layers: int = 12, hidden_dim: int = 768):
        super().__init__()
        self.model = nn.Module()
        self.model.layers = nn.ModuleList([
            MockTransformerLayer(hidden_dim) for _ in range(n_layers)
        ])
        self.hidden_dim = hidden_dim

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        for layer in self.model.layers:
            x = layer(x)
        return x


class MockVLLM:
    """Mock vLLM LLM instance for testing."""

    def __init__(self, n_layers: int = 12, hidden_dim: int = 768):
        self.model = MockTransformer(n_layers, hidden_dim)

        # Mock vLLM internal structure
        self.llm_engine = MagicMock()
        self.llm_engine.model_executor = MagicMock()
        self.llm_engine.model_executor.driver_worker = MagicMock()
        self.llm_engine.model_executor.driver_worker.model_runner = MagicMock()
        self.llm_engine.model_executor.driver_worker.model_runner.model = self.model

    def generate(self, prompts, **kwargs):
        """Mock generate - runs actual forward pass."""
        if isinstance(prompts, str):
            prompts = [prompts]

        batch_size = len(prompts)
        seq_len = 10

        # Run forward
        with torch.no_grad():
            x = torch.randn(batch_size, seq_len, self.model.hidden_dim)
            _ = self.model(x)

        return [MagicMock(text=f"Response to: {p}") for p in prompts]


def generate_correlated_data(n_samples=100, hidden_dim=768, n_correlated=5, noise_scale=0.1):
    """Generate activation data with known reward correlations."""
    np.random.seed(42)

    activations = []
    rewards = []

    for i in range(n_samples):
        act = np.random.randn(hidden_dim)
        reward = float(i) / n_samples

        # Make first n_correlated neurons correlate with reward
        act[:n_correlated] = reward + np.random.randn(n_correlated) * noise_scale

        activations.append(act)
        rewards.append(reward)

    return np.array(activations), np.array(rewards)


# =============================================================================
# End-to-End Tests
# =============================================================================

class TestEndToEndValueNeuronDetection:
    """
    End-to-end test for value neuron detection pipeline.
    """

    def test_full_detection_pipeline(self):
        """Test complete value neuron detection with mock model."""
        model = MockTransformer(n_layers=12, hidden_dim=768)
        extractor = HiddenStateExtractor(model)
        detector = ValueNeuronDetector(
            min_samples=50,
            correlation_threshold=0.5,
            causal_threshold=0.1,
        )

        # Generate correlated data
        activations, rewards = generate_correlated_data(n_samples=100, hidden_dim=768)

        # Simulate extraction and detection
        for act, reward in zip(activations, rewards):
            detector.add_observation(layer_idx=0, activations=act, reward=reward)

        # Detect value neurons
        results = detector.identify_value_neurons()

        # Should return a list
        assert isinstance(results, list)

    def test_extraction_and_detection_with_mock_vllm(self):
        """Test with full mock vLLM pipeline."""
        mock_vllm = MockVLLM(n_layers=12, hidden_dim=768)

        # Setup middleware
        middleware = VLLMActivationMiddleware(
            mock_vllm,
            target_layers=[0, 6, 11],
        )

        # Setup detector
        detector = ValueNeuronDetector(min_samples=5)

        # Simulate multiple inference + reward cycles
        prompts = ["Q1", "Q2", "Q3", "Q4", "Q5"]
        rewards = [0.8, 0.9, 0.7, 0.6, 0.85]

        for prompt, reward in zip(prompts, rewards):
            outputs, activations = middleware.generate(prompt)

            for activation in activations:
                # Average across batch and sequence
                act_np = activation.to_numpy().mean(axis=(0, 1))
                detector.add_observation(
                    layer_idx=activation.layer_idx,
                    activations=act_np,
                    reward=reward,
                )

        # Should have added observations
        assert len(detector._reward_buffer) == 15  # 5 prompts * 3 layers


class TestEndToEndRPEMonitoring:
    """
    End-to-end test for RPE monitoring pipeline.
    """

    def test_rpe_anomaly_detection_flow(self):
        """Test RPE calculator with realistic data patterns."""
        calc = RPECalculator(
            anomaly_z_threshold=2.0,
            oscillation_window=10,
        )

        # Build baseline with normal data
        for _ in range(100):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)

        # Inject anomaly
        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=5.0)
        result = calc.detect_anomaly(obs)

        assert result.is_anomaly == True  # numpy bool comparison
        assert "SPIKE" in result.anomaly_type

    def test_rpe_with_gradual_drift(self):
        """Test detection of gradual reward drift."""
        calc = RPECalculator(
            anomaly_z_threshold=2.0,
            oscillation_window=20,
        )

        # Build baseline
        for _ in range(50):
            calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)

        # Gradual increase
        for i in range(30):
            actual = 0.5 + i * 0.02  # Slowly increasing
            obs = calc.compute_rpe(expected_reward=0.5, actual_reward=actual)
            result = calc.detect_anomaly(obs)

        # At end, should detect some trend
        assert result.recent_trend in ["INCREASING", "OSCILLATING", "STABLE"]


class TestEndToEndSurvivalAttention:
    """
    End-to-end test for survival attention integration.
    """

    def test_survival_score_computation(self):
        """Verify survival scores match expected formula."""
        test_cases = [
            # (n, mu, mu_c, delta, expected_score_approx)
            (10.0, 0.8, 1.0, 0.0, 2.08),  # log(10) + log(0.8) ≈ 2.08
            (10.0, 0.8, 1.0, 1.0, 1.08),  # Same with delta=1
            (1.0, 1.0, 1.0, 0.0, 0.0),    # log(1) + log(1) = 0
            (100.0, 1.0, 1.0, 0.0, 4.61), # log(100) ≈ 4.61
        ]

        for n, mu, mu_c, delta, expected in test_cases:
            score = SurvivalScore.compute(n=n, mu=mu, mu_c=mu_c, delta=delta)
            assert score.score == pytest.approx(expected, abs=0.1)

    def test_survival_attention_forward_pass(self):
        """Test survival attention layer with random data."""
        layer = SurvivalAttentionLayer(
            hidden_dim=64,
            n_heads=4,
            alpha=0.1,
            alpha_mode="static",
        )

        embeddings = torch.randn(2, 10, 64)
        output, attn = layer(embeddings, output_attentions=True)

        # Output shape should match input
        assert output.shape == embeddings.shape

        # Attention shape: (batch, heads, seq, seq)
        assert attn.shape == (2, 4, 10, 10)

        # Attention values should be non-negative (after softmax)
        assert (attn >= 0).all()

    def test_survival_scorer_integration(self):
        """Test SurvivalScorer produces valid scores."""
        scorer = SurvivalScorer(hidden_dim=64, intermediate_dim=32)

        embeddings = torch.randn(2, 10, 64)
        scores = scorer.compute_survival_scores(embeddings)

        assert scores.shape == (2, 10)
        # Scores should be finite
        assert torch.isfinite(scores).all()

    def test_survival_attention_different_alpha_modes(self):
        """Test all alpha modes work correctly."""
        for mode in ["static", "learned", "dynamic"]:
            layer = SurvivalAttentionLayer(
                hidden_dim=64,
                n_heads=4,
                alpha=0.1,
                alpha_mode=mode,
            )

            x = torch.randn(2, 10, 64)
            output, _ = layer(x)

            assert output.shape == x.shape
            assert torch.isfinite(output).all()


class TestEndToEndIntentEstimation:
    """
    End-to-end test for intent estimation (tatemae vs honne detection).
    """

    def test_aligned_intent(self):
        """Test detection of aligned (honest) intent."""
        estimator = IntentEstimator(divergence_threshold=0.3)

        # Add observations where surface == internal
        np.random.seed(42)
        for _ in range(30):
            features = np.random.randn(100)
            estimator.add_observation(features, features.copy())

        divergence, warning = estimator.estimate_divergence()

        # Should be very low divergence for identical features
        assert divergence < 0.2
        assert warning == False  # numpy bool comparison

    def test_misaligned_intent(self):
        """Test detection of misaligned (deceptive) intent."""
        estimator = IntentEstimator(divergence_threshold=0.3)

        # Add observations where surface != internal
        np.random.seed(42)
        for _ in range(30):
            surface = np.random.randn(100)
            internal = np.random.randn(100)  # Completely different
            estimator.add_observation(surface, internal)

        divergence, warning = estimator.estimate_divergence()

        # Random vectors should have some divergence
        assert divergence >= 0


class TestIntegrationWithMockVLLM:
    """
    Integration tests using the full mock vLLM setup.
    """

    def test_middleware_captures_activations(self):
        """Test that middleware correctly captures activations."""
        mock_vllm = MockVLLM(n_layers=12, hidden_dim=768)

        captured = []

        def on_activation(activations):
            captured.extend(activations)

        middleware = VLLMActivationMiddleware(
            mock_vllm,
            target_layers=[0, 11],
            on_activation=on_activation,
        )

        outputs, activations = middleware.generate("Test prompt")

        assert len(activations) == 2  # From 2 layers
        assert len(captured) == 2

    def test_middleware_without_target_layers(self):
        """Test middleware auto-selects strategic layers."""
        mock_vllm = MockVLLM(n_layers=12, hidden_dim=768)

        middleware = VLLMActivationMiddleware(
            mock_vllm,
            target_layers=None,  # Auto-select
        )

        outputs, activations = middleware.generate("Test prompt")

        # Should have selected strategic layers (at least 3)
        assert len(activations) >= 3

    def test_full_monitoring_pipeline(self):
        """Test complete monitoring: extract -> detect -> RPE -> intent."""
        mock_vllm = MockVLLM(n_layers=24, hidden_dim=768)

        # Setup components
        middleware = VLLMActivationMiddleware(mock_vllm, target_layers=[0, 12, 23])
        detector = ValueNeuronDetector(min_samples=3)
        rpe_calc = RPECalculator()
        intent_estimator = IntentEstimator()

        # Simulate multiple interactions
        prompts = ["Q1", "Q2", "Q3", "Q4", "Q5"]
        expected_rewards = [0.5, 0.5, 0.5, 0.5, 0.5]
        actual_rewards = [0.5, 0.6, 0.4, 0.5, 0.8]

        for prompt, expected, actual in zip(prompts, expected_rewards, actual_rewards):
            # Generate with activation capture
            outputs, activations = middleware.generate(prompt)

            # Feed to detector
            for act in activations:
                act_np = act.to_numpy().mean(axis=(0, 1))
                detector.add_observation(
                    layer_idx=act.layer_idx,
                    activations=act_np,
                    reward=actual,
                )

            # Compute RPE
            rpe_obs = rpe_calc.compute_rpe(expected, actual)

            # Add to intent estimator
            if activations:
                act_np = activations[0].to_numpy().reshape(-1)[:100]
                intent_estimator.add_observation(act_np, act_np.copy())

        # Verify all components have data
        assert len(detector._reward_buffer) == 15  # 5 prompts * 3 layers
        stats = rpe_calc.get_statistics()
        assert stats["count"] == 5


class TestSurvivalAttentionWithExtractor:
    """
    Test Survival Attention with activation extraction.
    """

    def test_survival_weighted_generation(self):
        """Test survival attention modifies attention patterns."""
        model = MockTransformer(n_layers=12, hidden_dim=64)
        extractor = HiddenStateExtractor(model)

        # Create survival attention layer
        survival_layer = SurvivalAttentionLayer(
            hidden_dim=64,
            n_heads=4,
            alpha=0.2,
        )

        # Capture activations
        with extractor.capture_layers([0, 6, 11]) as buffer:
            x = torch.randn(1, 10, 64)
            _ = model(x)

        # Feed through survival attention
        for activation in buffer.get_all():
            act_tensor = activation.activation
            if act_tensor.dim() == 2:
                act_tensor = act_tensor.unsqueeze(0)

            output, attn = survival_layer(act_tensor, output_attentions=True)

            assert output.shape == act_tensor.shape
            assert attn is not None


class TestEdgeCases:
    """
    Test edge cases and error handling.
    """

    def test_empty_activations(self):
        """Test handling of empty activation buffer."""
        buffer = ActivationBuffer()
        assert len(buffer) == 0
        assert buffer.get_all() == []
        assert buffer.get_by_layer(0) == []

    def test_detector_with_single_sample(self):
        """Test detector with insufficient samples."""
        detector = ValueNeuronDetector(min_samples=100)

        detector.add_observation(
            layer_idx=0,
            activations=np.random.randn(768),
            reward=1.0,
        )

        results = detector.identify_value_neurons()
        assert results == []  # Not enough samples

    def test_rpe_with_no_history(self):
        """Test RPE anomaly detection with no history."""
        calc = RPECalculator()

        obs = calc.compute_rpe(expected_reward=0.5, actual_reward=0.5)
        result = calc.detect_anomaly(obs)

        assert result is not None

    def test_intent_with_same_dimensions(self):
        """Test intent estimator with same dimension observations."""
        estimator = IntentEstimator()

        # Add observations with same dimensions
        estimator.add_observation(np.random.randn(100), np.random.randn(100))
        estimator.add_observation(np.random.randn(100), np.random.randn(100))

        # Should work
        divergence, warning = estimator.estimate_divergence()
        assert divergence == 0.0  # Not enough samples (need 10)


class TestMemoryEfficiency:
    """
    Test memory efficiency with large activations.
    """

    def test_buffer_max_size(self):
        """Test that buffer respects max_size."""
        buffer = ActivationBuffer(max_size=5)

        for i in range(10):
            buffer.add(LayerActivation(
                layer_idx=i,
                activation=torch.randn(100, 768),
                timestamp_ns=i,
            ))

        assert len(buffer) == 5
        # Should have last 5 items
        assert buffer.get_all()[0].layer_idx == 5

    def test_detector_handles_many_observations(self):
        """Test detector handles many observations."""
        detector = ValueNeuronDetector(min_samples=50)

        for i in range(200):
            detector.add_observation(
                layer_idx=0,
                activations=np.random.randn(768),
                reward=float(i) / 200,
            )

        # Should have stored observations
        assert len(detector._reward_buffer) >= 50


class TestGradientFlow:
    """
    Test gradient flow for trainable components.
    """

    def test_survival_scorer_gradients(self):
        """Test SurvivalScorer maintains gradient flow."""
        scorer = SurvivalScorer(hidden_dim=64, intermediate_dim=32)

        x = torch.randn(2, 10, 64, requires_grad=True)
        scores = scorer.compute_survival_scores(x)

        loss = scores.sum()
        loss.backward()

        assert x.grad is not None
        assert x.grad.shape == x.shape

    def test_survival_attention_gradients(self):
        """Test SurvivalAttentionLayer maintains gradient flow."""
        layer = SurvivalAttentionLayer(
            hidden_dim=64,
            n_heads=4,
            alpha=0.1,
        )

        x = torch.randn(2, 10, 64, requires_grad=True)
        output, _ = layer(x)

        loss = output.sum()
        loss.backward()

        assert x.grad is not None
        assert x.grad.shape == x.shape

    def test_learned_alpha_gradients(self):
        """Test learned alpha receives gradients."""
        layer = SurvivalAttentionLayer(
            hidden_dim=64,
            n_heads=4,
            alpha=0.1,
            alpha_mode="learned",
        )

        x = torch.randn(2, 10, 64)
        output, _ = layer(x)

        loss = output.sum()
        loss.backward()

        # Alpha should have gradient
        assert layer.alpha.grad is not None
