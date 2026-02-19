# Contributing to CHINJU (English)

Thank you for your interest in contributing to CHINJU.

[日本語版 (CONTRIBUTING.md)](./CONTRIBUTING.md)

## Language Policy

- Issues and PRs are welcome in either English or Japanese.
- Maintainers will reply in the same language when possible.
- If a source document is currently Japanese-only, open an English issue and we will help map it to the relevant section.

## Repository Policy (Monorepo)

CHINJU currently uses a monorepo to keep safety-spec changes and implementation changes synchronized.

We consider splitting repositories only when one or more of the following becomes true:
- Teams are fully separated and need independent release cadences.
- Stronger permission isolation is required (for example, enclave code managed separately).
- CI duration or review contention reaches an operational limit.

## Contribution Areas

### 1. GPU Validation (Difficulty: ⭐)

`chinju-vllm` is tested in CPU environments, but we still need validation on real GPU environments.

```bash
# Requirements
# - CUDA 11.8+ / 12.x
# - vLLM 0.4+
# - Llama-2-7B or Llama-3-8B models

cd chinju-vllm
pip install -e ".[dev]"

# Run tests
pytest tests/ -v
```

Please report:
- GPU model and CUDA version
- vLLM version
- Test results (pass/fail)
- Error logs if available

### 2. Documentation Improvements (Difficulty: ⭐)

- Fix typos
- Clarify explanations
- Improve English documentation
- Add usage examples

### 3. TGI Integration (Difficulty: ⭐⭐)

Create an adapter for Text Generation Inference (TGI).

Reference: `chinju-vllm/chinju_vllm/activation_hook.py`

### 4. Hardware PoC (Difficulty: ⭐⭐⭐)

Build prototypes for hardware integrations described in the patent specifications.

Reference: specs under `sample/hardware/`

Target hardware examples:
- OTP (e.g. Microchip ATECC608B)
- HSM (YubiHSM 2, AWS CloudHSM)
- QRNG (ID Quantique Quantis)
- TPM 2.0 (Infineon SLB9670)

## Development Setup

### Rust (chinju-sidecar, chinju-core)

```bash
# Rust 1.75+
rustup update stable

cd chinju-sidecar
cargo build
cargo test
```

### Python (chinju-vllm)

```bash
# Python 3.10+
cd chinju-vllm
pip install -e ".[dev]"

# Test
pytest tests/ -v

# Type check
mypy chinju_vllm/

# Lint
ruff check chinju_vllm/
```

## Coding Standards

### Rust

- Format with `cargo fmt`
- Keep `cargo clippy` warnings at zero

### Python

- Lint with `ruff`
- Type-check with `mypy --strict`
- Public APIs should include docstrings

## Pull Request Process

1. Open an issue first to avoid duplicate work.
2. Fork and create a branch.
3. Commit your changes in small logical chunks.
4. Run relevant tests and checks.
5. Open a PR using the template.

### Safety-Critical Change Requirements

For changes around policy, credential/auth, audit, streaming, token accounting, or enclave boundaries:

- Add failure-case tests before or with the implementation.
- Explain safety impact and regression risk in the PR description.
- Avoid introducing bypass paths between policy decision and execution.

### Commit Message

```text
<type>: <summary>

<body>
```

type: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

Example:

```text
feat: add TGI activation hook

Implements activation extraction for Text Generation Inference,
mirroring the existing vLLM implementation.
```

## License

By contributing, you agree that your code is licensed under Apache License 2.0.

## Questions

- GitHub Issues: bugs, feature requests
- GitHub Discussions: design questions and general Q&A
- Confidential Code of Conduct reports: `contact@chinju.org`

## Code of Conduct

Please follow [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
