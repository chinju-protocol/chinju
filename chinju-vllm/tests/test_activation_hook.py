"""Tests for Activation Hook module."""

import pytest
import torch
import torch.nn as nn
import numpy as np

from chinju_vllm.activation_hook import (
    ActivationBuffer,
    ActivationHook,
    HiddenStateExtractor,
    LayerActivation,
    VLLMActivationMiddleware,
)


class MockTransformerLayer(nn.Module):
    """Mock transformer layer for testing."""

    def __init__(self, hidden_dim: int = 768):
        super().__init__()
        self.hidden_dim = hidden_dim
        self.linear = nn.Linear(hidden_dim, hidden_dim)

    def forward(self, x):
        return self.linear(x)


class MockTransformer(nn.Module):
    """Mock transformer model for testing."""

    def __init__(self, n_layers: int = 12, hidden_dim: int = 768):
        super().__init__()
        self.model = nn.Module()
        self.model.layers = nn.ModuleList([
            MockTransformerLayer(hidden_dim) for _ in range(n_layers)
        ])

    def forward(self, x):
        for layer in self.model.layers:
            x = layer(x)
        return x


class TestActivationBuffer:
    """Tests for ActivationBuffer."""

    def test_add_and_get(self):
        buffer = ActivationBuffer(max_size=100)
        activation = LayerActivation(
            layer_idx=0,
            activation=torch.randn(10, 768),
            timestamp_ns=12345,
        )
        buffer.add(activation)

        assert len(buffer) == 1
        assert buffer.get_all()[0] == activation

    def test_max_size_limit(self):
        buffer = ActivationBuffer(max_size=5)

        for i in range(10):
            buffer.add(LayerActivation(
                layer_idx=i,
                activation=torch.randn(10, 768),
                timestamp_ns=i,
            ))

        assert len(buffer) == 5
        # Should have last 5 items
        assert buffer.get_all()[0].layer_idx == 5

    def test_get_by_layer(self):
        buffer = ActivationBuffer(max_size=100)

        for layer in [0, 1, 0, 2, 0]:
            buffer.add(LayerActivation(
                layer_idx=layer,
                activation=torch.randn(10, 768),
                timestamp_ns=0,
            ))

        layer_0 = buffer.get_by_layer(0)
        assert len(layer_0) == 3

    def test_get_latest(self):
        buffer = ActivationBuffer(max_size=100)

        for i in range(10):
            buffer.add(LayerActivation(
                layer_idx=i,
                activation=torch.randn(10, 768),
                timestamp_ns=i,
            ))

        latest = buffer.get_latest(3)
        assert len(latest) == 3
        assert latest[0].layer_idx == 7

    def test_clear(self):
        buffer = ActivationBuffer(max_size=100)
        buffer.add(LayerActivation(
            layer_idx=0,
            activation=torch.randn(10, 768),
            timestamp_ns=0,
        ))

        buffer.clear()
        assert len(buffer) == 0


class TestLayerActivation:
    """Tests for LayerActivation."""

    def test_to_numpy(self):
        activation = LayerActivation(
            layer_idx=0,
            activation=torch.randn(10, 768),
            timestamp_ns=12345,
        )

        np_array = activation.to_numpy()
        assert isinstance(np_array, np.ndarray)
        assert np_array.shape == (10, 768)

    def test_shape_property(self):
        activation = LayerActivation(
            layer_idx=0,
            activation=torch.randn(2, 5, 768),
            timestamp_ns=12345,
        )

        assert activation.shape == (2, 5, 768)


class TestActivationHook:
    """Tests for ActivationHook."""

    def test_hook_captures_output(self):
        buffer = ActivationBuffer()
        hook = ActivationHook(buffer, layer_idx=0)

        layer = MockTransformerLayer()
        hook.register(layer)

        # Run forward pass
        x = torch.randn(1, 10, 768)
        _ = layer(x)

        assert len(buffer) == 1
        assert buffer.get_all()[0].layer_idx == 0

        hook.remove()

    def test_multiple_forward_passes(self):
        buffer = ActivationBuffer()
        hook = ActivationHook(buffer, layer_idx=5)

        layer = MockTransformerLayer()
        hook.register(layer)

        # Multiple forward passes
        for _ in range(3):
            x = torch.randn(1, 10, 768)
            _ = layer(x)

        assert len(buffer) == 3

        hook.remove()


class TestHiddenStateExtractor:
    """Tests for HiddenStateExtractor."""

    def test_layer_discovery(self):
        model = MockTransformer(n_layers=12)
        extractor = HiddenStateExtractor(model)

        assert extractor.num_layers == 12

    def test_get_layer(self):
        model = MockTransformer(n_layers=12)
        extractor = HiddenStateExtractor(model)

        layer = extractor.get_layer(5)
        assert layer is not None
        assert isinstance(layer, MockTransformerLayer)

    def test_get_layer_out_of_range(self):
        model = MockTransformer(n_layers=12)
        extractor = HiddenStateExtractor(model)

        assert extractor.get_layer(-1) is None
        assert extractor.get_layer(100) is None

    def test_capture_layers_context(self):
        model = MockTransformer(n_layers=12, hidden_dim=768)
        extractor = HiddenStateExtractor(model)

        with extractor.capture_layers([0, 5, 11]) as buffer:
            x = torch.randn(1, 10, 768)
            _ = model(x)

            # Should have captured from 3 layers
            assert len(buffer) == 3

        # After context, hooks should be removed
        # (no easy way to verify, but no errors)

    def test_capture_strategic_layers(self):
        model = MockTransformer(n_layers=24, hidden_dim=768)
        extractor = HiddenStateExtractor(model)

        with extractor.capture_strategic_layers(early=1, middle=2, late=1) as buffer:
            x = torch.randn(1, 10, 768)
            _ = model(x)

            # Should capture from ~4 layers
            assert len(buffer) >= 3

    def test_capture_all_layers(self):
        model = MockTransformer(n_layers=6, hidden_dim=768)
        extractor = HiddenStateExtractor(model)

        with extractor.capture_all_layers() as buffer:
            x = torch.randn(1, 10, 768)
            _ = model(x)

            assert len(buffer) == 6

    def test_invalid_layer_index(self):
        model = MockTransformer(n_layers=12, hidden_dim=768)
        extractor = HiddenStateExtractor(model)

        # Should handle invalid indices gracefully
        with extractor.capture_layers([0, 100, 11]) as buffer:
            x = torch.randn(1, 10, 768)
            _ = model(x)

            # Only valid layers (0 and 11) should be captured
            assert len(buffer) == 2


class TestVLLMActivationMiddleware:
    """Tests for VLLMActivationMiddleware."""

    def test_middleware_initialization(self):
        """Test middleware initializes with mock vLLM."""
        from unittest.mock import MagicMock

        # Create mock vLLM
        mock_llm = MagicMock()
        mock_llm.llm_engine.model_executor.driver_worker.model_runner.model = MockTransformer()

        middleware = VLLMActivationMiddleware(mock_llm)

        assert middleware._model is not None
        assert middleware._extractor is not None

    def test_middleware_with_target_layers(self):
        """Test middleware with explicit target layers."""
        from unittest.mock import MagicMock

        mock_model = MockTransformer(n_layers=12, hidden_dim=768)
        mock_llm = MagicMock()
        mock_llm.llm_engine.model_executor.driver_worker.model_runner.model = mock_model

        def mock_generate(prompts, **kwargs):
            # Run actual forward pass to trigger hooks
            x = torch.randn(1, 10, 768)
            with torch.no_grad():
                _ = mock_model(x)
            return [MagicMock(text="response")]

        mock_llm.generate = mock_generate

        middleware = VLLMActivationMiddleware(
            mock_llm,
            target_layers=[0, 5, 11],
        )

        outputs, activations = middleware.generate("Test prompt")

        assert len(outputs) == 1
        assert len(activations) == 3  # 3 target layers

    def test_middleware_auto_layer_selection(self):
        """Test middleware auto-selects strategic layers."""
        from unittest.mock import MagicMock

        mock_model = MockTransformer(n_layers=24, hidden_dim=768)
        mock_llm = MagicMock()
        mock_llm.llm_engine.model_executor.driver_worker.model_runner.model = mock_model

        def mock_generate(prompts, **kwargs):
            x = torch.randn(1, 10, 768)
            with torch.no_grad():
                _ = mock_model(x)
            return [MagicMock(text="response")]

        mock_llm.generate = mock_generate

        middleware = VLLMActivationMiddleware(
            mock_llm,
            target_layers=None,  # Auto-select
        )

        outputs, activations = middleware.generate("Test prompt")

        # Should select 4 strategic layers (n/4, n/2, 3n/4, n-1)
        assert len(activations) == 4

    def test_middleware_callback(self):
        """Test middleware invokes callback."""
        from unittest.mock import MagicMock

        mock_model = MockTransformer(n_layers=12, hidden_dim=768)
        mock_llm = MagicMock()
        mock_llm.llm_engine.model_executor.driver_worker.model_runner.model = mock_model

        def mock_generate(prompts, **kwargs):
            x = torch.randn(1, 10, 768)
            with torch.no_grad():
                _ = mock_model(x)
            return [MagicMock(text="response")]

        mock_llm.generate = mock_generate

        captured = []

        def on_activation(activations):
            captured.extend(activations)

        middleware = VLLMActivationMiddleware(
            mock_llm,
            target_layers=[0, 5],
            on_activation=on_activation,
        )

        middleware.generate("Test prompt")

        assert len(captured) == 2

    def test_middleware_fallback_no_model(self):
        """Test middleware fallback when model not accessible."""
        from unittest.mock import MagicMock

        mock_llm = MagicMock()
        # Don't setup internal structure - model not accessible
        del mock_llm.llm_engine
        mock_llm.generate = lambda prompts, **kwargs: [MagicMock(text="response")]

        middleware = VLLMActivationMiddleware(mock_llm)

        outputs, activations = middleware.generate("Test prompt")

        assert len(outputs) == 1
        assert activations == []  # No capture possible

    def test_middleware_multiple_prompts(self):
        """Test middleware with multiple prompts."""
        from unittest.mock import MagicMock

        mock_model = MockTransformer(n_layers=12, hidden_dim=768)
        mock_llm = MagicMock()
        mock_llm.llm_engine.model_executor.driver_worker.model_runner.model = mock_model

        def mock_generate(prompts, **kwargs):
            x = torch.randn(1, 10, 768)
            with torch.no_grad():
                _ = mock_model(x)
            n = len(prompts) if isinstance(prompts, list) else 1
            return [MagicMock(text=f"response_{i}") for i in range(n)]

        mock_llm.generate = mock_generate

        middleware = VLLMActivationMiddleware(
            mock_llm,
            target_layers=[0],
        )

        outputs, activations = middleware.generate(["P1", "P2", "P3"])

        assert len(outputs) == 3
        assert len(activations) == 1  # Still one layer, captures all prompts
