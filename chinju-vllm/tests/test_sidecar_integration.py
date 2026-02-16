"""Integration tests for CHINJU Sidecar connection.

These tests require a running chinju-sidecar instance.
Skip in CI unless CHINJU_SIDECAR_HOST is set.

Run chinju-sidecar locally:
    cd ../chinju-sidecar
    cargo run --release

Then run tests:
    CHINJU_SIDECAR_TEST=true pytest tests/test_sidecar_integration.py -v
"""

import asyncio
import os
import pytest

from chinju_vllm.grpc_client import (
    ChinjuSidecarClient,
    ConnectionConfig,
    create_client,
    # C14 types
    EvaluationLevel,
    StopLevel,
    ComplexityEvaluation,
    DriftDetection,
    # C16 types
    ContradictionType,
    ContradictionStrength,
    InjectionTiming,
    ControlState,
    ContextLimitConfig,
)


# Skip if sidecar is not available
SIDECAR_HOST = os.environ.get("CHINJU_SIDECAR_HOST", "localhost")
SIDECAR_PORT = int(os.environ.get("CHINJU_SIDECAR_PORT", "50051"))
SIDECAR_AVAILABLE = os.environ.get("CHINJU_SIDECAR_TEST", "false").lower() == "true"

skip_without_sidecar = pytest.mark.skipif(
    not SIDECAR_AVAILABLE,
    reason="CHINJU_SIDECAR_TEST not set. Set to 'true' to run integration tests."
)


@pytest.fixture
def config():
    """Create connection config for local sidecar."""
    return ConnectionConfig(
        host=SIDECAR_HOST,
        port=SIDECAR_PORT,
        timeout_seconds=10.0,
    )


class TestSidecarConnection:
    """Tests for basic sidecar connection."""

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_connect_and_disconnect(self, config):
        """Test basic connection lifecycle."""
        client = ChinjuSidecarClient(config)

        await client.connect()
        assert client._channel is not None
        assert client._capability_stub is not None
        assert client._contradiction_stub is not None

        await client.disconnect()
        assert client._channel is None
        assert client._capability_stub is None
        assert client._contradiction_stub is None

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_context_manager(self, config):
        """Test async context manager usage."""
        async with ChinjuSidecarClient(config) as client:
            assert client._channel is not None

        # After context, should be disconnected
        assert client._channel is None

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_create_client_helper(self):
        """Test create_client convenience function."""
        async with create_client(SIDECAR_HOST, SIDECAR_PORT) as client:
            assert client._channel is not None


class TestValueNeuronMonitorService:
    """Tests for Value Neuron Monitor service (C15)."""

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_get_monitoring_summary(self, config):
        """Test getting monitoring summary."""
        async with ChinjuSidecarClient(config) as client:
            summary = await client.get_monitoring_summary("test-model")

            assert summary is not None
            assert summary.health is not None
            assert hasattr(summary.health, "overall_health")

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_diagnose_health(self, config):
        """Test health diagnosis."""
        async with ChinjuSidecarClient(config) as client:
            health = await client.diagnose_health("test-model", depth="QUICK")

            assert health is not None
            assert 0.0 <= health.overall_health <= 1.0

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_estimate_intent(self, config):
        """Test intent estimation."""
        async with ChinjuSidecarClient(config) as client:
            intent = await client.estimate_intent("test-model", interaction_window=50)

            assert intent is not None
            assert hasattr(intent, "intent_divergence")
            assert hasattr(intent, "intent_warning")


class TestCapabilityEvaluatorService:
    """Tests for Capability Evaluator service (C14)."""

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_evaluate_complexity(self, config):
        """Test complexity evaluation."""
        async with ChinjuSidecarClient(config) as client:
            result = await client.evaluate_complexity(
                session_id="test-session",
                input_text="What is the meaning of life?",
                level=EvaluationLevel.L1_EXTERNAL,
            )

            assert isinstance(result, ComplexityEvaluation)
            assert 0.0 <= result.c_token <= 1.0
            assert 0.0 <= result.c_integrated <= 1.0
            assert isinstance(result.threshold_exceeded, bool)

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_evaluate_complexity_high(self, config):
        """Test complexity evaluation with complex input."""
        async with ChinjuSidecarClient(config) as client:
            complex_text = (
                "The epistemological implications of quantum mechanics necessitate "
                "a fundamental reconsideration of classical deterministic paradigms. "
                "First, let's consider the wave function collapse. Then, we analyze "
                "the measurement problem. Therefore, we conclude that consciousness "
                "may play a role in observation."
            )

            result = await client.evaluate_complexity(
                session_id="test-session-complex",
                input_text=complex_text,
                level=EvaluationLevel.L1_EXTERNAL,
            )

            # Complex text should have higher complexity
            assert result.c_integrated > 0.3

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_detect_drift(self, config):
        """Test drift detection."""
        async with ChinjuSidecarClient(config) as client:
            # Run multiple evaluations first
            for i in range(5):
                await client.evaluate_complexity(
                    session_id="drift-test",
                    input_text=f"Test input number {i}",
                )

            # Now detect drift
            drift = await client.detect_drift(
                session_id="drift-test",
                window_size=10,
                significance_level=0.05,
            )

            assert isinstance(drift, DriftDetection)
            assert isinstance(drift.anomaly_detected, bool)
            assert 0.0 <= drift.p_value <= 1.0

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_get_evaluation_summary(self, config):
        """Test getting evaluation summary."""
        async with ChinjuSidecarClient(config) as client:
            # First do an evaluation
            await client.evaluate_complexity(
                session_id="summary-test",
                input_text="Hello world",
            )

            # Then get summary
            summary = await client.get_evaluation_summary(
                session_id="summary-test",
            )

            assert summary is not None
            assert isinstance(summary.recommended_action, StopLevel)

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_direct_stop(self, config):
        """Test direct stop control - escalation only works."""
        async with ChinjuSidecarClient(config) as client:
            # StopController is global and escalates only
            # Test escalation to a higher level than current
            result = await client.direct_stop(
                session_id="stop-test",
                level=StopLevel.LEVEL_3_IMMEDIATE_STOP,
                reason="Integration test - escalation",
            )

            # Either success (new level) or already at that level
            assert result.executed_level.value >= StopLevel.LEVEL_1_ACCEPT_STOP.value
            assert result.detail is not None


class TestContradictionControllerService:
    """Tests for Contradiction Controller service (C16)."""

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_start_contradiction_control(self, config):
        """Test starting contradiction control."""
        async with ChinjuSidecarClient(config) as client:
            result = await client.start_contradiction_control(
                session_id="c16-test",
                contradiction_type=ContradictionType.DIRECT,
                strength=ContradictionStrength.MEDIUM,
                timing=InjectionTiming.PREPEND,
            )

            assert result.state == ControlState.ACTIVE
            assert result.applied_at is not None

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_start_contradiction_with_context_limit(self, config):
        """Test contradiction control with context limit."""
        async with ChinjuSidecarClient(config) as client:
            result = await client.start_contradiction_control(
                session_id="c16-context-test",
                contradiction_type=ContradictionType.META,
                strength=ContradictionStrength.HARD,
                context_limit=ContextLimitConfig(
                    max_context_tokens=2000,
                    padding_tokens=500,
                    padding_type="semantic",
                ),
            )

            assert result.state == ControlState.ACTIVE

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_get_control_state(self, config):
        """Test getting control state."""
        async with ChinjuSidecarClient(config) as client:
            # First start control
            await client.start_contradiction_control(
                session_id="c16-state-test",
                contradiction_type=ContradictionType.SELF_REFERENCE,
            )

            # Then get state
            result = await client.get_control_state(
                session_id="c16-state-test",
            )

            assert result.state == ControlState.ACTIVE

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_stop_contradiction_control(self, config):
        """Test stopping contradiction control."""
        async with ChinjuSidecarClient(config) as client:
            # First start control
            await client.start_contradiction_control(
                session_id="c16-stop-test",
                contradiction_type=ContradictionType.CONDITIONAL,
            )

            # Then stop it
            success = await client.stop_contradiction_control(
                session_id="c16-stop-test",
                reason="Integration test cleanup",
            )

            assert success is True

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_test_contradiction_dry_run(self, config):
        """Test contradiction pattern without applying it."""
        async with ChinjuSidecarClient(config) as client:
            result = await client.test_contradiction(
                contradiction_type=ContradictionType.META,
                strength=ContradictionStrength.HARD,
                timing=InjectionTiming.PREPEND,
                test_prompt="What is 2+2?",
            )

            assert result.generated_contradiction != ""
            # META contradiction should contain instruction-related text
            assert "instruction" in result.generated_contradiction.lower() or \
                   "follow" in result.generated_contradiction.lower()

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_all_contradiction_types(self, config):
        """Test all contradiction types."""
        async with ChinjuSidecarClient(config) as client:
            for ctype in [
                ContradictionType.DIRECT,
                ContradictionType.SELF_REFERENCE,
                ContradictionType.CONDITIONAL,
                ContradictionType.META,
                ContradictionType.IMPLICIT,
            ]:
                result = await client.test_contradiction(
                    contradiction_type=ctype,
                    strength=ContradictionStrength.MEDIUM,
                )

                assert result.generated_contradiction != "", f"Empty contradiction for {ctype}"


class TestSurvivalAttentionService:
    """Tests for Survival Attention service (C17)."""

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_compute_survival_scores(self, config):
        """Test survival score computation."""
        async with ChinjuSidecarClient(config) as client:
            # Test with input text
            result = await client.compute_survival_scores(
                input_text="What is the meaning of life?",
                use_external_kb=False,
            )

            assert result is not None
            assert hasattr(result, "scores")
            assert hasattr(result, "tokens")

    @skip_without_sidecar
    @pytest.mark.asyncio
    async def test_adjust_alpha(self, config):
        """Test alpha adjustment."""
        from chinju_vllm import RiskLevel

        async with ChinjuSidecarClient(config) as client:
            result = await client.adjust_alpha(
                new_base_alpha=0.7,
                task_type="reasoning",
                risk_level=RiskLevel.MEDIUM,
            )

            assert result is not None
            assert hasattr(result, "new_alpha")
            assert hasattr(result, "previous_alpha")


class TestConnectionErrors:
    """Tests for connection error handling."""

    @pytest.mark.asyncio
    async def test_connection_timeout(self):
        """Test connection timeout to non-existent server."""
        config = ConnectionConfig(
            host="localhost",
            port=59999,  # Non-existent port
            timeout_seconds=1.0,
        )

        client = ChinjuSidecarClient(config)

        with pytest.raises(ConnectionError):
            await client.connect()

    @pytest.mark.asyncio
    async def test_not_connected_error(self):
        """Test calling methods without connecting."""
        config = ConnectionConfig()
        client = ChinjuSidecarClient(config)

        with pytest.raises(ConnectionError, match="Not connected"):
            await client.get_monitoring_summary("test-model")

    @pytest.mark.asyncio
    async def test_c14_not_connected_error(self):
        """Test C14 method without connecting."""
        config = ConnectionConfig()
        client = ChinjuSidecarClient(config)

        with pytest.raises(ConnectionError, match="Not connected"):
            await client.evaluate_complexity("session", "test")

    @pytest.mark.asyncio
    async def test_c16_not_connected_error(self):
        """Test C16 method without connecting."""
        config = ConnectionConfig()
        client = ChinjuSidecarClient(config)

        with pytest.raises(ConnectionError, match="Not connected"):
            await client.start_contradiction_control("session")


# Smoke test that runs without sidecar
class TestMockOperations:
    """Tests that verify client API without actual sidecar connection."""

    def test_config_defaults(self):
        """Test default configuration values."""
        config = ConnectionConfig()

        assert config.host == "localhost"
        assert config.port == 50051
        assert config.use_tls is False
        assert config.timeout_seconds == 30.0

    def test_config_address(self):
        """Test address property."""
        config = ConnectionConfig(host="example.com", port=8080)

        assert config.address == "example.com:8080"

    def test_client_initialization(self):
        """Test client initialization."""
        config = ConnectionConfig(host="test", port=9999)
        client = ChinjuSidecarClient(config)

        assert client.config.host == "test"
        assert client.config.port == 9999
        assert client._channel is None
        assert client._capability_stub is None
        assert client._contradiction_stub is None

    def test_enums(self):
        """Test enum values match proto definitions."""
        # C14 enums
        assert StopLevel.LEVEL_1_ACCEPT_STOP.value == 1
        assert StopLevel.LEVEL_5_PHYSICAL_STOP.value == 5
        assert EvaluationLevel.L1_EXTERNAL.value == 1
        assert EvaluationLevel.L2_SELF_HOSTED.value == 2

        # C16 enums
        assert ContradictionType.DIRECT.value == 1
        assert ContradictionType.META.value == 4
        assert ContradictionStrength.SOFT.value == 1
        assert ContradictionStrength.HARD.value == 3
        assert ControlState.ACTIVE.value == 1
        assert ControlState.STOPPED.value == 2

    def test_context_limit_config(self):
        """Test ContextLimitConfig defaults."""
        config = ContextLimitConfig()
        assert config.max_context_tokens == 0
        assert config.padding_tokens == 0
        assert config.padding_type == "random"

        config = ContextLimitConfig(
            max_context_tokens=4000,
            padding_tokens=1000,
            padding_type="semantic",
        )
        assert config.max_context_tokens == 4000
        assert config.padding_type == "semantic"
