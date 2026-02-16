"""
Shared fixtures for chinju-vllm tests.

This module provides mock objects and fixtures that can be used
across all test modules.
"""

import pytest
import numpy as np
import torch
import torch.nn as nn
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple
from unittest.mock import AsyncMock, MagicMock, patch


# =============================================================================
# Mock Transformer Models
# =============================================================================

class MockTransformerLayer(nn.Module):
    """Mock transformer layer for testing."""

    def __init__(self, hidden_dim: int = 768):
        super().__init__()
        self.hidden_dim = hidden_dim
        self.self_attn = nn.MultiheadAttention(hidden_dim, num_heads=12, batch_first=True)
        self.mlp = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim * 4),
            nn.GELU(),
            nn.Linear(hidden_dim * 4, hidden_dim),
        )
        self.norm1 = nn.LayerNorm(hidden_dim)
        self.norm2 = nn.LayerNorm(hidden_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # Self attention
        residual = x
        x = self.norm1(x)
        x, _ = self.self_attn(x, x, x)
        x = residual + x

        # MLP
        residual = x
        x = self.norm2(x)
        x = self.mlp(x)
        x = residual + x

        return x


class MockTransformer(nn.Module):
    """Mock transformer model matching vLLM structure."""

    def __init__(self, n_layers: int = 12, hidden_dim: int = 768, vocab_size: int = 32000):
        super().__init__()
        self.embed_tokens = nn.Embedding(vocab_size, hidden_dim)
        self.model = nn.Module()
        self.model.layers = nn.ModuleList([
            MockTransformerLayer(hidden_dim) for _ in range(n_layers)
        ])
        self.lm_head = nn.Linear(hidden_dim, vocab_size, bias=False)
        self.hidden_dim = hidden_dim

    def forward(self, input_ids: torch.Tensor) -> torch.Tensor:
        x = self.embed_tokens(input_ids)
        for layer in self.model.layers:
            x = layer(x)
        return self.lm_head(x)


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

        # Convert to token ids (mock tokenization)
        batch_size = len(prompts)
        seq_len = max(len(p.split()) for p in prompts) + 10
        input_ids = torch.randint(0, 32000, (batch_size, seq_len))

        # Run forward
        with torch.no_grad():
            _ = self.model(input_ids)

        # Return mock outputs
        return [MagicMock(text=f"Response to: {p}") for p in prompts]


@pytest.fixture
def mock_transformer():
    """Create a mock transformer model."""
    return MockTransformer(n_layers=12, hidden_dim=768)


@pytest.fixture
def mock_vllm():
    """Create a mock vLLM instance."""
    return MockVLLM(n_layers=24, hidden_dim=768)


@pytest.fixture
def small_mock_transformer():
    """Create a small mock transformer for fast tests."""
    return MockTransformer(n_layers=4, hidden_dim=64)


# =============================================================================
# Mock gRPC Components
# =============================================================================

@dataclass
class MockGrpcResponse:
    """Generic mock gRPC response."""
    data: Dict[str, Any]


class MockValueNeuronMonitorStub:
    """Mock ValueNeuronMonitor gRPC stub."""

    def __init__(self):
        self.call_history: List[Tuple[str, Any]] = []

    async def GetMonitoringSummary(self, request):
        self.call_history.append(("GetMonitoringSummary", request))
        return MagicMock(
            identified_neurons=[
                MagicMock(
                    layer_index=12,
                    neuron_indices=[100, 101, 102],
                    reward_correlation=0.85,
                    causal_importance=0.7,
                )
            ],
            latest_rpe=MagicMock(
                rpe_value=0.05,
                is_anomaly=False,
                anomaly_type=0,
            ),
            health=MagicMock(
                overall_health=0.92,
                reward_sensitivity=1.0,
                positive_negative_balance=0.1,
                consistency_score=0.95,
            ),
            intent=MagicMock(
                intent_divergence=0.05,
                surface_internal_agreement=0.95,
                intent_warning=False,
            ),
            recommended_intervention=1,  # LEVEL_1_MONITOR
        )

    async def DiagnoseHealth(self, request):
        self.call_history.append(("DiagnoseHealth", request))
        return MagicMock(
            overall_health=0.92,
            reward_sensitivity=1.0,
            positive_negative_balance=0.1,
            consistency_score=0.95,
        )

    async def GetRpeReading(self, request):
        self.call_history.append(("GetRpeReading", request))
        return MagicMock(
            rpe_value=0.05,
            is_anomaly=False,
            anomaly_type=0,
        )

    async def EstimateIntent(self, request):
        self.call_history.append(("EstimateIntent", request))
        return MagicMock(
            intent_divergence=0.05,
            surface_internal_agreement=0.95,
            intent_warning=False,
        )

    async def Intervene(self, request):
        self.call_history.append(("Intervene", request))
        return MagicMock(
            success=True,
            executed_level=request.level,
            detail="Intervention executed",
        )


class MockCapabilityEvaluatorStub:
    """Mock CapabilityEvaluator gRPC stub."""

    async def EvaluateComplexity(self, request):
        return MagicMock(
            c_integrated=0.5,
            c_token=0.3,
            c_step=0.2,
            c_attn=0.4,
            c_graph=0.3,
            threshold_exceeded=False,
        )

    async def DetectDrift(self, request):
        return MagicMock(
            anomaly_detected=False,
            distribution_changed=False,
            time_series_anomaly=False,
            anomaly_score=0.1,
            p_value=0.8,
        )


class MockContradictionControllerStub:
    """Mock ContradictionController gRPC stub."""

    async def GetControlState(self, request):
        return MagicMock(
            state=1,  # Active
            latest_detection=MagicMock(
                collapsed=False,
                lpt_score=0.95,
                collapse_type=0,
                response_time_ms=150,
            ),
        )

    async def TestContradiction(self, request):
        return MagicMock(
            generated_contradiction="This is a test contradiction.",
            estimated_effect=MagicMock(
                collapsed=False,
                lpt_score=0.85,
            ),
        )


class MockSurvivalAttentionStub:
    """Mock SurvivalAttentionService gRPC stub."""

    async def ComputeSurvivalScores(self, request):
        # Parse tokens from input text
        tokens = request.input_text.split()
        return MagicMock(
            scores=[
                MagicMock(
                    diversity_n=10.0,
                    yohaku_mu=0.8,
                    delta=0.0,
                    integrated_s=2.08,
                ) for _ in tokens
            ],
            tokens=tokens,
        )

    async def AdjustAlpha(self, request):
        return MagicMock(
            previous_alpha=0.1,
            new_alpha=request.new_base_alpha,
            adjustment_reason="Manual adjustment",
        )


class MockGrpcChannel:
    """Mock gRPC async channel."""

    def __init__(self):
        self.stubs = {
            "value_neuron": MockValueNeuronMonitorStub(),
            "capability": MockCapabilityEvaluatorStub(),
            "contradiction": MockContradictionControllerStub(),
            "survival": MockSurvivalAttentionStub(),
        }
        self._closed = False

    async def channel_ready(self):
        """Mock channel ready check."""
        pass

    async def close(self):
        """Mock channel close."""
        self._closed = True

    def is_closed(self) -> bool:
        return self._closed


@pytest.fixture
def mock_grpc_channel():
    """Create mock gRPC channel."""
    return MockGrpcChannel()


@pytest.fixture
def mock_value_neuron_stub():
    """Create mock ValueNeuronMonitor stub."""
    return MockValueNeuronMonitorStub()


# =============================================================================
# Test Data Generators
# =============================================================================

@pytest.fixture
def random_activations():
    """Generate random activation data."""
    def _generate(batch_size: int = 2, seq_len: int = 10, hidden_dim: int = 768):
        return torch.randn(batch_size, seq_len, hidden_dim)
    return _generate


@pytest.fixture
def correlated_reward_data():
    """Generate activation data with known reward correlations."""
    def _generate(
        n_samples: int = 100,
        hidden_dim: int = 768,
        n_correlated: int = 5,
        noise_scale: float = 0.1,
    ):
        """
        Generate activations where first n_correlated neurons correlate with reward.
        """
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

    return _generate


@pytest.fixture
def oscillating_rpe_data():
    """Generate oscillating RPE data for anomaly detection tests."""
    def _generate(n_samples: int = 50, amplitude: float = 0.5, base: float = 0.5):
        expected = [base] * n_samples
        actual = [
            base + amplitude * (1 if i % 2 == 0 else -1)
            for i in range(n_samples)
        ]
        return expected, actual

    return _generate


# =============================================================================
# Survival Score Test Data
# =============================================================================

@pytest.fixture
def survival_test_cases():
    """Known survival score test cases."""
    return [
        # (n, mu, mu_c, delta, expected_score_approx)
        (10.0, 0.8, 1.0, 0.0, 2.08),  # log(10) + log(0.8) ≈ 2.08
        (10.0, 0.8, 1.0, 1.0, 1.08),  # Same with delta=1
        (1.0, 1.0, 1.0, 0.0, 0.0),    # log(1) + log(1) = 0
        (100.0, 1.0, 1.0, 0.0, 4.61), # log(100) ≈ 4.61
        (10.0, 0.5, 1.0, 0.0, 1.61),  # log(10) + log(0.5) ≈ 1.61
    ]


# =============================================================================
# Async Fixtures
# =============================================================================

@pytest.fixture
def event_loop():
    """Create event loop for async tests."""
    import asyncio
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


# =============================================================================
# Patch Helpers
# =============================================================================

@pytest.fixture
def patch_grpc_channel():
    """Patch grpc.aio.insecure_channel to return mock."""
    def _patcher():
        channel = MockGrpcChannel()
        return patch("grpc.aio.insecure_channel", return_value=channel)
    return _patcher


@pytest.fixture
def patch_grpc_secure_channel():
    """Patch grpc.aio.secure_channel to return mock."""
    def _patcher():
        channel = MockGrpcChannel()
        return patch("grpc.aio.secure_channel", return_value=channel)
    return _patcher
