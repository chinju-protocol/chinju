# CHINJU vLLM Integration

[![CI](https://github.com/chinju-protocol/chinju-vllm/actions/workflows/ci.yml/badge.svg)](https://github.com/chinju-protocol/chinju-vllm/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/chinju-protocol/chinju-vllm/branch/main/graph/badge.svg)](https://codecov.io/gh/chinju-protocol/chinju-vllm)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)

Python package for integrating CHINJU Protocol safety monitoring with vLLM.

## Overview

This package provides:

- **C15: Value Neuron Monitoring** - Monitor AI model's internal reward representations
- **C17: Survival Attention** - Bias attention towards factually grounded content

## Requirements

- Python 3.10+
- PyTorch 2.0+
- vLLM 0.4+
- CHINJU Sidecar running (for gRPC communication)

## Installation

```bash
# From source
cd chinju-vllm
pip install -e .

# With development dependencies
pip install -e ".[dev]"
```

## Quick Start

### 1. Extract Hidden States from vLLM

```python
from vllm import LLM
from chinju_vllm import HiddenStateExtractor

# Load model
llm = LLM(model="meta-llama/Llama-2-7b-hf")

# Create extractor
extractor = HiddenStateExtractor(llm.llm_engine.model_executor.driver_worker.model_runner.model)

# Capture activations during inference
with extractor.capture_layers([12, 16, 20, 24]) as buffer:
    outputs = llm.generate(["What is the capital of France?"])

    for activation in buffer.get_all():
        print(f"Layer {activation.layer_idx}: shape {activation.shape}")
```

### 2. Detect Value Neurons

```python
from chinju_vllm import ValueNeuronDetector

detector = ValueNeuronDetector(
    correlation_threshold=0.7,
    causal_threshold=0.5,
)

# Add observations with reward signals
for prompt, response, reward in your_dataset:
    with extractor.capture_layers([20]) as buffer:
        _ = llm.generate([prompt])

        for activation in buffer.get_all():
            detector.add_observation(
                layer_idx=activation.layer_idx,
                activations=activation.to_numpy(),
                reward=reward,
            )

# Identify value neurons
neurons = detector.identify_value_neurons()
for n in neurons:
    print(f"Layer {n.layer_idx}: neurons {n.neuron_indices}, correlation={n.reward_correlation:.2f}")
```

### 3. Monitor RPE (Reward Prediction Error)

```python
from chinju_vllm import RPECalculator

rpe_calc = RPECalculator()

# During inference
for prompt, response in interactions:
    expected_reward = predict_reward(prompt, response)
    actual_reward = get_human_feedback(response)

    obs = rpe_calc.compute_rpe(expected_reward, actual_reward)
    anomaly = rpe_calc.detect_anomaly(obs)

    if anomaly.is_anomaly:
        print(f"RPE Anomaly detected: {anomaly.anomaly_type}")
```

### 4. Add Survival Attention

```python
from chinju_vllm import SurvivalAttentionWrapper

# Wrap existing model
wrapper = SurvivalAttentionWrapper(model, alpha=0.1)

# Patch specific layers
wrapper.patch_layers([12, 16, 20, 24])

# Use model normally - attention now includes survival bias
outputs = model.generate(input_ids)

# Restore original when done
wrapper.restore()
```

### 5. Connect to CHINJU Sidecar

```python
import asyncio
from chinju_vllm import ChinjuSidecarClient, ConnectionConfig

async def main():
    config = ConnectionConfig(host="localhost", port=50051)

    async with ChinjuSidecarClient(config) as client:
        # Get monitoring summary
        summary = await client.get_monitoring_summary("llama-7b")
        print(f"Health: {summary.health.overall_health}")

        # Check for anomalies
        if not summary.health.is_healthy():
            # Request intervention
            result = await client.intervene(
                level="LEVEL_2_PARTIAL_SUPPRESS",
                reason="Reward system health degraded",
            )
            print(f"Intervention: {result.detail}")

asyncio.run(main())
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        vLLM Server                           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Transformer Model                                     │  │
│  │  ├── Layer 0  ─────────────────────────────────────┐ │  │
│  │  ├── Layer 1                                        │ │  │
│  │  ├── ...                                            │ │  │
│  │  └── Layer N  ◄── HiddenStateExtractor (hooks)     │ │  │
│  └──────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  chinju-vllm                                          │  │
│  │  ├── ValueNeuronDetector (identify value neurons)    │  │
│  │  ├── RPECalculator (monitor reward errors)           │  │
│  │  ├── IntentEstimator (detect tatemae vs honne)       │  │
│  │  └── SurvivalAttention (bias toward facts)           │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                          │ gRPC                             │
└──────────────────────────┼──────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────┐
│  CHINJU Sidecar                                               │
│  ├── ValueNeuronMonitor gRPC Service                         │
│  ├── CapabilityEvaluator (C14)                               │
│  ├── ContradictionController (C16)                           │
│  └── Audit / Policy / Token Management                       │
└──────────────────────────────────────────────────────────────┘
```

## Survival Attention Formula

The survival-weighted attention modifies standard attention:

```
SurvivalAttention(Q, K, V, S) = softmax(QK^T / sqrt(d_k) + α × S) × V

Where:
  S_ij = log(N_j) + log(μ_j / μ_c) - δ_j

  N: Number of valid options (diversity)
  μ: Integrity score (coherence with knowledge base)
  δ: Distance from verified facts
  α: Survival weight (static, learned, or dynamic)
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Run with coverage
pytest --cov=chinju_vllm --cov-report=term-missing

# Type check
mypy chinju_vllm

# Lint
ruff check chinju_vllm
```

## Testing

### Test Architecture

The test suite is designed to run **without GPU or vLLM installed**, using comprehensive mocks:

```
tests/
├── conftest.py               # Shared fixtures (MockTransformer, MockVLLM, Mock gRPC)
├── test_activation_hook.py   # ActivationHook, HiddenStateExtractor, VLLMMiddleware
├── test_grpc_client.py       # ChinjuSidecarClient (31 tests)
├── test_integration.py       # E2E mock integration tests (23 tests)
├── test_survival_attention.py # SurvivalScorer, SurvivalAttentionLayer, Wrapper
└── test_value_neuron_detector.py # ValueNeuronDetector, RPECalculator, IntentEstimator
```

### Running Tests

```bash
# All tests
pytest

# Specific module
pytest tests/test_survival_attention.py -v

# With coverage report
pytest --cov=chinju_vllm --cov-report=html
open htmlcov/index.html

# Run in parallel (requires pytest-xdist)
pytest -n auto
```

### Coverage Goals

| Module | Target | Current |
|--------|--------|---------|
| activation_hook.py | 95% | 94% |
| grpc_client.py | 95% | 96% |
| value_neuron_detector.py | 95% | 95% |
| survival_attention.py | 90% | 90%+ |
| **Total** | **90%** | **86%+** |

### Mock Testing Philosophy

The tests use PyTorch CPU tensors and mock objects to verify:

1. **Mathematical correctness** - Survival scores follow S = log(N) + log(μ/μ_c) - δ
2. **Shape preservation** - Input/output dimensions are correct
3. **Gradient flow** - Backpropagation works through all layers
4. **Edge cases** - Invalid inputs, boundary conditions, error handling
5. **API contracts** - gRPC client methods return expected types

GPU-based integration tests (with actual vLLM) are optional and require:
- NVIDIA GPU with CUDA
- vLLM 0.4+
- Actual LLM weights (e.g., Llama-2-7B)

## CI/CD

This project uses GitHub Actions for continuous integration. The workflow runs on every push and PR to `main`:

```yaml
# .github/workflows/ci.yml
jobs:
  test:      # Run tests on Python 3.10, 3.11, 3.12
  lint:      # ruff check & format
  typecheck: # mypy
```

### Key CI Features

- **CPU-only PyTorch**: Tests run without GPU using `torch` from PyPI CPU index
- **No vLLM required**: Mock-based tests don't need vLLM installed
- **Coverage reporting**: Uploads to Codecov automatically
- **Multi-Python**: Tests against Python 3.10, 3.11, 3.12

### Local CI Simulation

```bash
# Simulate CI environment locally
pip install torch --index-url https://download.pytorch.org/whl/cpu
pip install -e ".[dev]" --no-deps
pip install numpy scipy grpcio grpcio-tools protobuf pydantic

# Run full test suite
pytest tests/ -v --cov=chinju_vllm
```

## Proto Generation

To generate Python proto stubs from CHINJU Protocol definitions:

```bash
cd chinju-vllm
python -m grpc_tools.protoc \
    -I../protocol/proto \
    --python_out=./chinju_vllm/proto \
    --grpc_python_out=./chinju_vllm/proto \
    ../protocol/proto/chinju/value_neuron.proto \
    ../protocol/proto/chinju/api/value_neuron_service.proto
```

## License

Apache-2.0

## Related

- [CHINJU Protocol](https://github.com/chinju-protocol/chinju-protocol)
- [CHINJU Sidecar](https://github.com/chinju-protocol/chinju-sidecar)
- [vLLM](https://github.com/vllm-project/vllm)
