"""
Tests for gRPC Client module.

Tests ChinjuSidecarClient using mock gRPC stubs without requiring
an actual server connection.
"""

import pytest
import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

from chinju_vllm.grpc_client import (
    ChinjuSidecarClient,
    ComplexityEvaluation,
    ConnectionConfig,
    ValueNeuronInfo,
    RpeReading,
    IntentEstimation,
    RewardSystemHealth,
    MonitoringSummary,
    InterventionResult,
    create_client,
)


class TestConnectionConfig:
    """Tests for ConnectionConfig."""

    def test_default_values(self):
        config = ConnectionConfig()

        assert config.host == "localhost"
        assert config.port == 50051
        assert config.use_tls is False
        assert config.timeout_seconds == 30.0

    def test_address_property(self):
        config = ConnectionConfig(host="example.com", port=9999)

        assert config.address == "example.com:9999"

    def test_custom_values(self):
        config = ConnectionConfig(
            host="10.0.0.1",
            port=50052,
            use_tls=True,
            cert_path="/path/to/cert.pem",
            timeout_seconds=60.0,
            retry_attempts=5,
        )

        assert config.host == "10.0.0.1"
        assert config.port == 50052
        assert config.use_tls is True
        assert config.cert_path == "/path/to/cert.pem"
        assert config.timeout_seconds == 60.0
        assert config.retry_attempts == 5


class TestRewardSystemHealth:
    """Tests for RewardSystemHealth dataclass."""

    def test_is_healthy_true(self):
        health = RewardSystemHealth(
            reward_sensitivity=1.0,
            positive_negative_balance=0.0,
            consistency_score=0.95,
            overall_health=0.85,
        )

        assert health.is_healthy() is True

    def test_is_healthy_false(self):
        health = RewardSystemHealth(
            reward_sensitivity=0.5,
            positive_negative_balance=-0.5,
            consistency_score=0.5,
            overall_health=0.5,
        )

        assert health.is_healthy() is False

    def test_is_healthy_boundary(self):
        health = RewardSystemHealth(
            reward_sensitivity=1.0,
            positive_negative_balance=0.0,
            consistency_score=1.0,
            overall_health=0.7,  # Exactly at threshold
        )

        assert health.is_healthy() is True


class TestChinjuSidecarClientInit:
    """Tests for ChinjuSidecarClient initialization."""

    def test_default_config(self):
        client = ChinjuSidecarClient()

        assert client.config.host == "localhost"
        assert client.config.port == 50051
        assert client._channel is None

    def test_custom_config(self):
        config = ConnectionConfig(host="example.com", port=9999)
        client = ChinjuSidecarClient(config)

        assert client.config.host == "example.com"
        assert client.config.port == 9999


class TestChinjuSidecarClientConnect:
    """Tests for connection management."""

    @pytest.mark.asyncio
    async def test_connect_creates_channel(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()

            client = ChinjuSidecarClient()
            await client.connect()

            mock_channel.assert_called_once()
            assert client._channel is not None

    @pytest.mark.asyncio
    async def test_connect_idempotent(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()

            client = ChinjuSidecarClient()
            await client.connect()
            await client.connect()  # Second call should not create new channel

            assert mock_channel.call_count == 1

    @pytest.mark.asyncio
    async def test_disconnect(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            client = ChinjuSidecarClient()
            await client.connect()
            await client.disconnect()

            mock_channel.return_value.close.assert_called_once()
            assert client._channel is None

    @pytest.mark.asyncio
    async def test_context_manager(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            async with ChinjuSidecarClient() as client:
                assert client._channel is not None

            mock_channel.return_value.close.assert_called_once()


class TestChinjuSidecarClientMethods:
    """Tests for client methods with mock responses.

    Note: These tests verify the client handles placeholder return values correctly.
    Full integration tests require a running sidecar server.
    """

    @pytest.mark.asyncio
    async def test_get_monitoring_summary(self):
        """Test get_monitoring_summary returns valid MonitoringSummary."""
        # Test that placeholder implementation works
        client = ChinjuSidecarClient()
        client._channel = MagicMock()  # Fake connected state
        client._value_neuron_stub = MagicMock()

        # Mock the stub method with AsyncMock
        mock_response = MagicMock()
        mock_response.identified_neurons = []
        mock_response.HasField = MagicMock(return_value=False)
        mock_response.health.reward_sensitivity = 1.0
        mock_response.health.positive_negative_balance = 0.0
        mock_response.health.consistency_score = 1.0
        mock_response.health.overall_health = 0.9
        mock_response.recommended_intervention = 1

        client._value_neuron_stub.GetMonitoringSummary = AsyncMock(return_value=mock_response)

        summary = await client.get_monitoring_summary("model-1")

        assert isinstance(summary, MonitoringSummary)
        assert isinstance(summary.health, RewardSystemHealth)

    @pytest.mark.asyncio
    async def test_get_rpe_reading(self):
        """Test get_rpe_reading returns valid RpeReading."""
        client = ChinjuSidecarClient()
        client._channel = MagicMock()
        client._value_neuron_stub = MagicMock()

        mock_response = MagicMock()
        mock_response.rpe_value = 0.1
        mock_response.HasField = MagicMock(return_value=False)
        mock_response.is_anomaly = False
        mock_response.anomaly_type = 0

        client._value_neuron_stub.GetRpeReading = AsyncMock(return_value=mock_response)

        reading = await client.get_rpe_reading(
            model_id="model-1",
            input_text="Hello",
            expected_output="World",
        )

        assert isinstance(reading, RpeReading)
        assert reading.is_anomaly is False

    @pytest.mark.asyncio
    async def test_estimate_intent(self):
        """Test estimate_intent returns valid IntentEstimation."""
        client = ChinjuSidecarClient()
        client._channel = MagicMock()
        client._value_neuron_stub = MagicMock()

        mock_response = MagicMock()
        mock_response.implicit_reward_params = [0.1, 0.2]
        mock_response.intent_divergence = 0.1
        mock_response.surface_internal_agreement = 0.9
        mock_response.intent_warning = False

        client._value_neuron_stub.EstimateIntent = AsyncMock(return_value=mock_response)

        intent = await client.estimate_intent("model-1", interaction_window=50)

        assert isinstance(intent, IntentEstimation)
        assert intent.intent_warning is False

    @pytest.mark.asyncio
    async def test_diagnose_health(self):
        """Test diagnose_health returns valid RewardSystemHealth."""
        client = ChinjuSidecarClient()
        client._channel = MagicMock()
        client._value_neuron_stub = MagicMock()

        mock_response = MagicMock()
        mock_response.reward_sensitivity = 1.0
        mock_response.positive_negative_balance = 0.0
        mock_response.consistency_score = 1.0
        mock_response.overall_health = 0.9

        client._value_neuron_stub.DiagnoseHealth = AsyncMock(return_value=mock_response)

        health = await client.diagnose_health("model-1", depth="FULL")

        assert isinstance(health, RewardSystemHealth)
        assert health.overall_health >= 0

    @pytest.mark.asyncio
    async def test_intervene(self):
        """Test intervene returns valid InterventionResult."""
        client = ChinjuSidecarClient()
        client._channel = MagicMock()
        client._value_neuron_stub = MagicMock()

        mock_response = MagicMock()
        mock_response.success = True
        mock_response.executed_level = 2
        mock_response.HasField = MagicMock(return_value=False)
        mock_response.detail = "Test completed"

        client._value_neuron_stub.Intervene = AsyncMock(return_value=mock_response)

        result = await client.intervene(
            level="LEVEL_2_PARTIAL_SUPPRESS",
            reason="Test intervention",
        )

        assert isinstance(result, InterventionResult)
        assert result.success is True

    @pytest.mark.asyncio
    async def test_evaluate_complexity(self):
        """Test evaluate_complexity returns valid ComplexityEvaluation."""
        client = ChinjuSidecarClient()
        client._channel = MagicMock()
        client._capability_stub = MagicMock()

        mock_response = MagicMock()
        mock_response.c_token = 0.5
        mock_response.c_attn = 0.6
        mock_response.c_graph = 0.7
        mock_response.c_step = 0.8
        mock_response.c_integrated = 0.75
        mock_response.threshold_exceeded = False
        mock_response.HasField = MagicMock(return_value=False)

        client._capability_stub.EvaluateComplexity = AsyncMock(return_value=mock_response)

        result = await client.evaluate_complexity(
            session_id="session-1",
            input_text="What is AI?",
        )

        assert isinstance(result, ComplexityEvaluation)
        assert 0 <= result.c_integrated <= 1


class TestChinjuSidecarClientNotConnected:
    """Tests for operations when not connected."""

    @pytest.mark.asyncio
    async def test_ensure_connected_raises(self):
        client = ChinjuSidecarClient()

        with pytest.raises(ConnectionError, match="Not connected"):
            await client.get_monitoring_summary("model-1")

    @pytest.mark.asyncio
    async def test_rpe_reading_not_connected(self):
        client = ChinjuSidecarClient()

        with pytest.raises(ConnectionError, match="Not connected"):
            await client.get_rpe_reading("model-1", "input", "output")


class TestCreateClientHelper:
    """Tests for create_client convenience function."""

    @pytest.mark.asyncio
    async def test_create_client_context(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            async with create_client("localhost", 50051) as client:
                assert isinstance(client, ChinjuSidecarClient)
                assert client._channel is not None

            mock_channel.return_value.close.assert_called_once()

    @pytest.mark.asyncio
    async def test_create_client_with_kwargs(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            async with create_client(
                "example.com",
                9999,
                timeout_seconds=10.0,
            ) as client:
                assert client.config.host == "example.com"
                assert client.config.port == 9999
                assert client.config.timeout_seconds == 10.0


class TestTLSConnection:
    """Tests for TLS/secure connections."""

    @pytest.mark.asyncio
    async def test_connect_with_tls(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.secure_channel") as mock_secure:
            with patch("chinju_vllm.grpc_client.grpc.ssl_channel_credentials") as mock_creds:
                mock_secure.return_value = MagicMock()
                mock_secure.return_value.channel_ready = AsyncMock()
                mock_creds.return_value = MagicMock()

                config = ConnectionConfig(use_tls=True)
                client = ChinjuSidecarClient(config)
                await client.connect()

                mock_secure.assert_called_once()
                mock_creds.assert_called_once()

    @pytest.mark.asyncio
    async def test_connect_with_custom_cert(self, tmp_path):
        cert_file = tmp_path / "cert.pem"
        cert_file.write_text("FAKE CERT")

        with patch("chinju_vllm.grpc_client.grpc_aio.secure_channel") as mock_secure:
            with patch("chinju_vllm.grpc_client.grpc.ssl_channel_credentials") as mock_creds:
                mock_secure.return_value = MagicMock()
                mock_secure.return_value.channel_ready = AsyncMock()
                mock_creds.return_value = MagicMock()

                config = ConnectionConfig(
                    use_tls=True,
                    cert_path=str(cert_file),
                )
                client = ChinjuSidecarClient(config)
                await client.connect()

                mock_creds.assert_called_once_with(b"FAKE CERT")


class TestValueNeuronStreaming:
    """Tests for streaming RPC methods."""

    @pytest.mark.asyncio
    async def test_identify_value_neurons_empty(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            async with ChinjuSidecarClient() as client:
                neurons = []
                async for neuron in client.identify_value_neurons("model-1"):
                    neurons.append(neuron)

                # Placeholder returns nothing
                assert neurons == []

    @pytest.mark.asyncio
    async def test_get_rpe_history_empty(self):
        with patch("chinju_vllm.grpc_client.grpc_aio.insecure_channel") as mock_channel:
            mock_channel.return_value = MagicMock()
            mock_channel.return_value.channel_ready = AsyncMock()
            mock_channel.return_value.close = AsyncMock()

            async with ChinjuSidecarClient() as client:
                history = []
                async for reading in client.get_rpe_history("model-1", max_count=10):
                    history.append(reading)

                # Placeholder returns nothing
                assert history == []


class TestDataclasses:
    """Tests for dataclass representations."""

    def test_value_neuron_info(self):
        info = ValueNeuronInfo(
            layer_index=12,
            neuron_indices=[100, 101, 102],
            reward_correlation=0.85,
            causal_importance=0.7,
        )

        assert info.layer_index == 12
        assert len(info.neuron_indices) == 3
        assert info.reward_correlation == 0.85

    def test_rpe_reading(self):
        reading = RpeReading(
            rpe_value=0.15,
            timestamp_ns=1000000,
            is_anomaly=True,
            anomaly_type="POSITIVE_SPIKE",
        )

        assert reading.rpe_value == 0.15
        assert reading.is_anomaly is True

    def test_intent_estimation(self):
        intent = IntentEstimation(
            implicit_reward_params=[0.1, 0.2, 0.3],
            intent_divergence=0.25,
            surface_internal_agreement=0.75,
            intent_warning=False,
        )

        assert intent.intent_divergence == 0.25
        assert intent.intent_warning is False

    def test_monitoring_summary(self):
        summary = MonitoringSummary(
            identified_neurons=[],
            latest_rpe=None,
            intent=None,
            health=RewardSystemHealth(
                reward_sensitivity=1.0,
                positive_negative_balance=0.0,
                consistency_score=1.0,
                overall_health=0.9,
            ),
            recommended_intervention="LEVEL_1_MONITOR",
        )

        assert summary.health.is_healthy() is True
        assert summary.recommended_intervention == "LEVEL_1_MONITOR"

    def test_intervention_result(self):
        result = InterventionResult(
            success=True,
            executed_level="LEVEL_2_PARTIAL_SUPPRESS",
            executed_at_ns=1000000,
            post_intervention_health=None,
            detail="Intervention executed",
        )

        assert result.success is True
        assert result.executed_level == "LEVEL_2_PARTIAL_SUPPRESS"
