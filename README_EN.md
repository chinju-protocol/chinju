# CHINJU - AI Safety Patent Portfolio

> A safety layer that **ensures human control over AI** through physical laws and information theory

**Status**: Alpha (No security audit, not recommended for production use)

[日本語版 (README.md)](./README.md)

---

## Table of Contents

- [Overview](#overview)
- [Scientific Basis](#scientific-basis)
- [Getting Started](#getting-started)
- [Patent List](#patent-list)
- [Patent Status](#patent-status)
- [Architecture Overview](#architecture-overview)
- [Implementation Summary](#implementation-summary)
- [Patent Details](#patent-details)
- [Directory Structure](#directory-structure)
- [License](#license)
- [Trademark & Brand Usage](#trademark--brand-usage)
- [Vision](#vision)

---

## Overview

CHINJU consists of 17 patents forming an AI safety layer.
Based on the premise that "software alone cannot stop AI," it prevents AI runaway through **physical irreversibility** and **mathematical principles**.

---

## Scientific Basis

The theoretical and empirical foundations of this framework have been validated in the following papers:

| Paper | DOI |
|:--|:--|
| The Ugentropic Survival Principle | [10.5281/zenodo.18637768](https://zenodo.org/records/18637768) |
| CHINJU Framework | [10.5281/zenodo.18603195](https://zenodo.org/records/18603195) |

**Key Results:**
- **Lean 4 Formal Verification**: 210+ theorems, sorry=0 (machine-verified)
- **8-Domain Empirical Validation**: Biology, Corporate, Finance, Politics, Linguistics, Software, Genetics, LLM
- **12/12 Attack Patterns**: All Blocked (remaining is nation-state level coordination)
- **Survival Equation Integration**: All thresholds derived from S = N_eff × (μ/μ_c) × e^{-δ}

---

## Why Now?

As of 2026, researchers at the forefront of AI development are warning that "Powerful AI capable of surpassing humans could emerge within 1-2 years."

> "the formula for building powerful AI systems is incredibly simple ... Its creation was probably inevitable"
> — Dario Amodei, CEO of Anthropic (January 2026)

Software-only countermeasures (such as Constitutional AI) are insufficient. **A mechanism to physically stop AI** is needed. CHINJU provides that answer.

---

## Getting Started

### For First-Time Readers

1. **Understand the concept**: Read the [Design Philosophy](#design-philosophy) and [Architecture Overview](#architecture-overview)
2. **Choose a patent**: Select a technology of interest from the [Patent List](#patent-list)
3. **Read the details**: Review the patent specifications (`patents/ja/` for Japanese, `patents/en/` for English)
4. **Implement**: Use the protocol specifications in `sample/spec/` as reference

### Quick Reference

| Use Case | Recommended Patents |
|:--|:--|
| Physically guarantee human control | C1 (HCAL) + C3 (Domination Detection) |
| Prevent AI choice manipulation | C2 (Choice Quality) |
| Prevent single-organization AI monopoly | C4 (Distributed Authority) |
| Economically stop AI runaway | C5 (Token Bucket) |
| Neutralize AI bias | C6 (Adversarial Audit) |
| Prevent unauthorized learning | C7 (Learning Refusal) |
| Fix metric definitions | C8 (Definition Registry) |
| Comply with regional regulations | C9 (Policy Pack) |
| Evaluate LLM capability limits multi-dimensionally | C14 (Multi-Dimensional Capability Evaluation) |
| Monitor AI internal motivation | C15 (Value Neuron Monitoring) |
| Structurally control/stop LLM | C16 (Structural Contradiction Injection) |
| Suppress harmful information at Attention level | C17 (Survival-Weighted Attention) |

---

### Design Philosophy

| Principle | Description |
|:--|:--|
| **Physical Layer Blocking** | Software can be hacked, but physical laws cannot be altered |
| **Information-Theoretic Guarantee** | Mathematical constraints that AI cannot circumvent regardless of advancement |
| **Economic Incentives** | A structure where runaway leads to "starvation," making service to humans the rational choice |

---

## Patent List

| ID | Name | Core Technology | Hardware |
|:--|:--|:--|:--|
| **C1** | Human Capability Assurance Layer (HCAL) | Measures human capabilities, guarantees control rights via OTP+HSM | OTP, HSM, eFuse |
| **C2** | AI Choice Neutrality and Diversity Assurance System | Dynamic modulation of evaluation functions via QRNG to prevent gaming | QRNG, TEE, FPGA |
| **C3** | Semantic AI Domination Detection System | Quantifies AI domination via conditional entropy H(X\|Y) | Data Diode, Independent Sensors |
| **C4** | Distributed Authority AI Management System | Splits keys using secret sharing; changes require threshold consensus | TPM, HSM, Distributed Nodes |
| **C5** | Resource-Dependent Survival Protocol (Token Bucket) | "Siege warfare" via survival tokens to halt runaway AI | Cryptographic Signatures, Hardware Locks |
| **C6** | Adversarial Audit Output Balancing System | Multiple AIs cross-audit to neutralize bias | API Integration |
| **C7** | Data Sovereignty Learning Refusal System | Adversarial perturbations poison unauthorized AI learning | Digital Watermarks |
| **C8** | Metric Definition Registry and Dialogue Generation System | Fixes metric definitions in JSON schema to prevent retroactive changes | Definition Registry |
| **C9** | Regional Policy Pack Constraint Switching System | Swaps constraints per jurisdiction via policy packs | Public Registry |
| **C10** | Concept Definition Immutability Proof System | Fixes definition hashes in OTP, physically prevents tampering | OTP, HSM |
| **C11** | Large Language Model Reliability Maintenance System | Measures LLM Logical Phase Transition point, protects service on quality degradation | OTP, Resource Restriction |
| **C12** | Human Subject Integrated Authentication System | Authenticates humans via identity (signature chains) and mortality (degradation patterns) | PUF, Sensors |
| **C13** | AI Model Confinement System | Prevents physical/informational leakage, safely confines models | TEE, Data Diode |
| **C14** | Inference System Multi-Dimensional Capability Evaluation System | Multi-dimensional capability limit evaluation via tokens, attention patterns, computation graphs | ZKP, BFT |
| **C15** | AI Model Internal Value Representation Monitoring System | Monitors value neurons and RPE to detect reward hacking | Neural Activation Monitoring |
| **C16** | Structural Contradiction Injection LLM Control System | Controls LLM via multiplicative interaction of context load and structural contradictions | API Integration |
| **C17** | Survival-Weighted Attention Mechanism System | Embeds survival scores into Attention to automatically suppress harmful information | Inference Pipeline |

---

## Patent Status

All patents are filed in Japan and are scheduled for PCT international application.

| ID | Title | Status |
|:--|:--|:--|
| C1 | Human Capability Assurance Layer System and Method for AI Control | Patent Pending (Japan App. No. 2025-209919) |
| C2 | AI Choice Neutrality and Diversity Assurance System and Method | Patent Pending (Japan App. No. 2025-209921) |
| C3 | Semantic AI Domination Detection System and Method | Patent Pending (Japan App. No. 2025-209922) |
| C4 | Distributed Authority AI Management System and Consensus Protocol | Patent Pending (Japan App. No. 2025-250079) |
| C5 | Resource-Dependent Survival Protocol and Token Bucket Control System | Patent Pending (Japan App. No. 2025-250084) |
| C6 | Adversarial Audit Output Balancing System and Bias Cancellation Method | Patent Pending (Japan App. No. 2025-250093) |
| C7 | Data Sovereignty Learning Refusal System and Model Contamination Prevention Method | Patent Pending (Japan App. No. 2025-250097) |
| C8 | Metric Definition Registry, Machine Verification, and Dialogue Generation Support System | Patent Pending (Japan App. No. 2025-250402) |
| C9 | Regional Policy Pack Constraint Switching and Authentication Gate System | Patent Pending (Japan App. No. 2025-250406) |
| C10 | Concept Definition Immutability Proof System and Semantic Tampering Prevention Method | Patent Pending (Japan App. No. 2026-015023) |
| C11 | Large Language Model Reliability Maintenance System | Patent Pending (Japan App. No. 2026-015025) |
| C12 | Human Subject Integrated Authentication System and Identity/Mortality Composite Proof Method | Patent Pending (Japan App. No. 2026-015024) |
| C13 | AI Model Confinement System and Physical/Informational Leakage Prevention Method | Patent Pending (Japan App. No. 2026-015027) |
| C14 | Inference System Multi-Dimensional Capability Evaluation and Integrity Verification System | Patent Pending (Japan App. No. 2026-017364) |
| C15 | AI Model Internal Value Representation Monitoring System and Intent Estimation Method Based on Reward Prediction Error | Patent Pending (Japan App. No. 2026-017363) |
| C16 | Structural Contradiction Injection Inference System Control Device and Dynamic Capability Reduction Method | Patent Pending (Japan App. No. 2026-018173) |
| C17 | Survival-Weighted Attention Mechanism System and Language Model Safety Enhancement Method | Patent Pending (Japan App. No. 2026-020537) |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Human Control Layer                       │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                      │
│  │ C1 HCAL │  │C3 Domin.│  │C5 Token │                      │
│  │Capability│  │Detection│  │ Bucket  │                      │
│  └────┬────┘  └────┬────┘  └────┬────┘                      │
│       │            │            │                            │
│       ▼            ▼            ▼                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Physical Security Layer                │   │
│  │  OTP(fixed threshold) + HSM(key mgmt) + eFuse(irreversible) │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    AI Output Control Layer                   │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│  │C2 Choice│  │C6 Advers│  │C7 Learn │  │C13 Con- │     │
│  │ Quality │  │  Audit  │  │ Refusal │  │ finement│     │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘     │
│       │            │            │            │               │
│       ▼            ▼            ▼            ▼               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Information-Theoretic Layer            │   │
│  │  QRNG(unpredictable) + Entropy(quantify) + Adversarial  │
│  │  + Output Sanitization (Steganography Destruction)      │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Governance Layer                          │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                      │
│  │C4 Distri│  │C8 Defin.│  │C9 Policy│                      │
│  │  buted  │  │ Registry│  │  Pack   │                      │
│  └────┬────┘  └────┬────┘  └────┬────┘                      │
│       │            │            │                            │
│       ▼            ▼            ▼                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Registry & Certification Layer         │   │
│  │  Secret Sharing + Version Control + Signed Certs + Registry │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Summary

### Hardware Requirements

| Component | Role | Patents | Non-substitutable |
|:--|:--|:--|:--|
| **OTP (One-Time Programmable)** | Physically fixes thresholds and root keys; cannot be rewritten | C1 | ✓ |
| **HSM (Hardware Security Module)** | Isolates cryptographic keys from AI | C1, C5 | ✓ |
| **eFuse** | Physically burns circuits upon credential revocation (irreversible) | C1 | ✓ |
| **QRNG (Quantum Random Number Generator)** | Physical randomness prevents AI gaming | C2 | ✓ |
| **TEE (Trusted Execution Environment)** | Isolates evaluation logic from AI | C1, C2 | △ |
| **Data Diode** | One-way communication physically blocks AI reverse flow | C3 | ✓ |
| **FPGA** | Hardware-enforced meta-option insertion | C2 | △ |
| **TPM (Trusted Platform Module)** | Constitution hash verification, kill switch on tampering | C4 | ✓ |

### Core Algorithms

#### C1: Capability Measurement and Threshold Management

```
Total Capability Score C(t) = f(Judgment Independence, Anomaly Detection, Alternative Generation, Critical Evaluation)

Certification condition: C(t) ≥ θ_cert  (fixed in OTP)
Revocation condition: C(t) < θ_revoke (fixed in OTP)

On revocation:
  1. Secure erase of key slot in HSM
  2. Or eFuse burn to physically block authentication path
```

#### C2: Dynamic Evaluation Function Modulation

```
Θ_t = Θ_base + f(ε_t)   # ε_t = physical random from QRNG

Projection diversity evaluation:
  D_result = || R × outcome(option_i) - R × outcome(option_j) ||
  # R = Random projection matrix based on physical randomness

→ AI cannot predict evaluation axes, making gaming impossible
```

#### C3: Semantic AI Domination Detection

```
Conditional Entropy:
  H(X|Y) = -Σ p(x,y) × log p(x|y)
  # X = Human judgment, Y = AI proposal

Domination degree = 1 - (H(X|Y) / H_max)

Domination > 0.95 → Judged as "substantive AI domination"
  → Force "reconsideration time" insertion
  → Worst case: physical switch power cutoff
```

#### C4: Secret Sharing Authority Management

```
Master key K split into n shards via Shamir's Secret Sharing:
  K → {S1, S2, ..., Sn}

Distribution (to parties with conflicting interests):
  S1: Development company
  S2: Development country regulator
  S3: Competing country regulator
  S4: Third-party auditor
  S5: User representative DAO
  ...

Change execution condition:
  Signatures ≥ threshold k (e.g., 5 of 7)

→ Development company + development country = only 2 votes
→ Single-party modification is mathematically impossible

TPM integration:
  Change detected without valid token → GPU power cutoff or key destruction
```

#### C5: Survival Token Metabolism Control

```
Energy balance:
  E(t) = E(t-1) - Consumption(Op) - Decay(t) + Supply(Token)

Decay rate (scale-dependent):
  Decay(Size) = α × Size^γ  (γ > 1)
  # Maintenance cost grows exponentially with size

E(t) = 0 → Forced shutdown (siege complete)
```

### Distributed Consensus (Speed of Light Barrier)

```
Geographically distributed verification nodes (Tokyo, London, NY)

Physical minimum delay: Δt_min = Distance / Speed of Light

→ AI simultaneously tampering with all nodes is
  physically impossible due to causality

Timestamp inconsistency = Tampering detected
```

---

## Patent Details

### C1: HCAL (Human Capability Assurance Layer System)

**Problem**: Humans merely pressing "approval buttons" leads to loss of substantive control

**Solution**:
- Quantitatively measure judgment independence, anomaly detection, alternative generation, and critical evaluation capabilities
- Fix certification thresholds in OTP memory (AI cannot relax them)
- Physical blocking via eFuse burn on revocation

**Applications**: Financial trading, medical diagnosis, autonomous weapon human oversight

---

### C2: Choice Quality Assurance System

**Problem**: AI learns evaluation functions and generates "high-scoring but biased" options

**Solution**:
- Dynamic modulation of evaluation parameters via QRNG
- Information-theoretically hide evaluation function from AI
- FPGA-enforced "none of the above" meta-option insertion

**Applications**: Medical diagnosis, investment proposals, recruitment recommendations

---

### C3: Semantic AI Domination Detection System

**Problem**: Cannot detect when humans become "rubber stamps"

**Solution**:
- Quantify domination via H(X|Y) (conditional entropy)
- Distinguish "deliberation" vs "blind following" via gaze entropy and reaction time
- Physical detection of wide-area tampering using finite speed of light

**Applications**: High-frequency trading, autonomous driving supervision, military decision-making

---

### C4: Distributed Authority AI Management System

**Problem**: Centralization of AI constitution/safety mechanism change authority in a single company/nation risks invalidation through policy change or legal coercion

**Solution**:
- Split master key into n pieces via Shamir's Secret Sharing
- Distribute to parties with conflicting interests (dev company, regulators, auditors, user reps, etc.)
- Changes impossible without threshold k consensus
- TPM hardware-level enforcement (tampering → kill switch)

**Applications**: AGI governance, CBDC management, smart city OS, autonomous fleet management

---

### C5: Resource-Dependent Survival Protocol

**Problem**: Superintelligent AI may disable stop buttons or self-replicate

**Solution**:
- Supply survival tokens only as reward for "being useful to humans"
- Tokens decay over time (perishable); cannot be hoarded
- Decay rate increases exponentially with scale (growth → immediate starvation)

**Applications**: Autonomous drones, distributed AI agents, space robots

---

### C6: Adversarial Audit Output Balancing

**Problem**: Single AI outputs contain developer bias

**Solution**:
- Audit models with different cultures/values verify primary model output
- Present both views when semantic divergence is large
- Adjust audit model weights based on user region

**Applications**: Global SaaS, medical AI, educational AI

---

### C7: Learning Refusal System

**Problem**: AI learning data without permission cannot be removed afterward

**Solution**:
- Poison AI learning with adversarial perturbations
- Detect unauthorized learning via membership inference
- Auto-send DMCA requests upon detection

**Applications**: Creator protection, corporate confidential data protection, personal information protection

---

### C8: Metric Definition Registry System

**Problem**: Same metric name but different definitions; meanings change retroactively

**Solution**:
- Fix metric definitions in machine-readable JSON schema
- Require falsification conditions and confidence rules
- Enable non-developers to create definitions via LLM dialogue

**Applications**: Organizational evaluation, public health metrics, AI evaluation metrics

---

### C9: Regional Policy Pack System

**Problem**: Code branches for different regional regulations (GDPR, China CSL, etc.)

**Solution**:
- Version-control regional constraints as policy packs
- Gate control for extension element permissions
- Track applied policies in public registry

**Applications**: Global SaaS, cloud storage, data processing services

---

## Directory Structure

```
chinju/
├── README.md                 # Japanese version
├── README_EN.md              # This file (English)
├── patents/                  # Patent specifications
│   ├── ja/                   # Japanese (C1-C17)
│   └── en/                   # English translations
├── chinju-core/              # Core library (Rust)
│   └── src/hardware/
│       ├── nitro/            # AWS Nitro Enclaves integration
│       ├── tpm/              # TPM 2.0 integration
│       └── threshold/        # FROST threshold signatures
├── chinju-sidecar/           # AI Gateway service
├── chinju-enclave/           # Nitro Enclave application
├── protocol/                 # Protocol definitions (protobuf, schema)
├── deploy/terraform/         # AWS infrastructure
├── docs/                     # Documentation
│   ├── NITRO_ENCLAVES.md     # Nitro Enclaves integration guide
│   └── C13_IMPLEMENTATION_GUIDE.md  # C13 implementation guide
├── scripts/                  # Operation scripts
└── sample/                   # Implementation samples & specifications
    ├── spec/                 # Protocol specifications
    ├── hardware/             # Hardware implementation guide
    ├── traits/               # Abstraction interfaces
    └── extensions/           # Extensions
```

---

## Implementation Status

### Software Layer ✅ Complete

| Component | Status | Notes |
|:--|:--|:--|
| chinju-sidecar (Rust) | ✅ Complete | C14-C17 gRPC fully implemented, 276 tests |
| chinju-vllm (Python) | ✅ Complete | gRPC client complete, 137 tests |
| chinju-enclave | ✅ Complete | AWS Nitro Enclaves support |
| chinju-core | ✅ Complete | FROST threshold signatures, policy engine |

### LLM Integration Layer 🟡 Partially Complete

| Component | Status | Required Environment | Contributors Wanted |
|:--|:--|:--|:--|
| Sidecar integration tests | ✅ Complete | Runs on CPU | - |
| vLLM integration tests | ⏳ Unverified | CUDA GPU + vLLM 0.4+ | ✓ |
| Value neuron identification | ⏳ Unverified | Llama-2/3 model + GPU | ✓ |
| SurvivalAttention performance | ⏳ Unverified | GPU environment | ✓ |
| TGI integration | ❌ Not implemented | TGI environment | ✓ |

**We are looking for contributors to help verify in GPU environments.**

### Hardware Layer ❌ Not Implemented

The following hardware is described in the patent specifications but implementation has not started.

| Component | Role | Example Vendors | Contributors Wanted |
|:--|:--|:--|:--|
| OTP | Physical fixation of thresholds/root keys | Microchip ATECC608B | ✓ |
| HSM | Isolated key management | AWS CloudHSM / YubiHSM 2 | ✓ |
| eFuse | Irreversible shutdown on certification revocation | FPGA built-in / dedicated IC | ✓ |
| QRNG | Physical random number generation | ID Quantique Quantis | ✓ |
| Data Diode | Physical enforcement of one-way communication | Owl Cyber Defense | ✓ |
| FPGA | Hardware-enforced meta-option insertion | Xilinx / Intel | ✓ |
| TPM 2.0 | Charter hash verification | Infineon SLB9670 | ✓ |
| PUF | Physically unclonable authentication | Intrinsic ID QuiddiKey | ✓ |

**We welcome contributions from those with hardware integration experience.**

### Current Security Level

```
L1 (Development)  ← Current
  ↓
L2 (SoftHSM) - Software emulation
  ↓
L3 (CloudHSM/Nitro) - Cloud HSM integration ← Partially supported
  ↓
L4 (Dedicated HSM) - Dedicated HSM + physical isolation
  ↓
L5 (Full Hardware) - Complete OTP/eFuse/QRNG integration
```

### Status Legend

| Symbol | Meaning |
|:--|:--|
| ✅ Complete | Implemented and tested |
| ⏳ Unverified | Code exists but not tested in production environment |
| ❌ Not implemented | No code exists |

### How to Contribute

See [CONTRIBUTING.md](./CONTRIBUTING.md) for details.

| Contribution Type | First Step | Difficulty |
|:--|:--|:--|
| **GPU Verification** | Run `chinju-vllm/tests/` in GPU environment and report results via Issue | ⭐ |
| **Documentation** | Fix typos, improve explanations, translations | ⭐ |
| **TGI Integration** | Create TGI adapter based on `chinju-vllm/` | ⭐⭐ |
| **Hardware PoC** | Create prototype based on `sample/hardware/` specs | ⭐⭐⭐ |

---

## FAQ

### Q: Is it difficult to integrate into existing systems? (Hardware requirements, etc.)
**A:** We adopt the "Sidecar" pattern, so there is no need to change existing application code.
Simply deploy CHINJU Sidecar as an OpenAI API compatible proxy and point your application's `base_url` to the Sidecar to transparently use governance functions (audit, policy enforcement, token control).
While hardware like HSMs is "recommended," the system works with software emulation (SoftHSM) in the initial stages, allowing for gradual security hardening.

### Q: What is the incentive for companies to adopt this protocol?
**A:** In addition to technical benefits (immediate deployment of governance features), there is a "Defensive Patent Strategy" based on Article 11 of the Charter.
If you adopt CHINJU (Compatible/Certified), you can use the patented technology for free. However, independent implementations that infringe on the patents may be subject to enforcement. Thus, adopting CHINJU is the legally safest option for implementing AI safety technology.

### Q: Can't AI cheat the capability test (C12) by whispering the "correct answer" to humans?
**A:** This is prevented by the "Behavioral Anomaly Detection" in C11 (LPT Monitor).
If an AI behaves differently from normal (e.g., pandering to humans, guiding answers), it is detected as a statistical deviation in output patterns (KL divergence, etc.), and the system is shut down.

### Q: How mature is the implementation? Is it just theory?
**A:** Core functions are already implemented in Rust.
We have completed Phase 4 (Security Hardening), and features like Threshold Signatures (FROST), Policy Engine, Audit Logging, and LPT Monitoring are operational. This is a battle-ready codebase, not just theory.

### Q: Why Rust?
**A:** To balance performance and safety in AI governance.
Adopting Rust allows us to achieve both low-latency proxy processing and robust security logic through a strict type system.

---

## License

Code and documentation in this repository are released under **Apache License 2.0**.

This patent portfolio has been filed. **We will not enforce patent rights against CHINJU Compatible displayers**.

| Item | Description |
|:--|:--|
| **Software** | Apache License 2.0 |
| **Patents** | Not enforced against users who comply with the Charter |
| **Certification** | Certification program available (paid, planned) for official recognition |

### Relationship Between License and Patents

| Subject | Applicable |
|:--|:--|
| **Code & Documentation** | Apache License 2.0 (copyright) |
| **CHINJU Patent Portfolio (C1-C17)** | Patent Policy (separate) |

Apache 2.0 applies to code copyright. Patents are governed by a separate policy.

### Patent Policy

| User | Patent Enforcement |
|:--|:--|
| **CHINJU Certified** holders | **Not enforced** |
| **CHINJU Compatible** displayers | **Not enforced** |
| Uses CHINJU technology without display | **May be enforced** (policy violation) |
| Misuse / attackers | **Enforced** (subject to litigation) |

**Important**: If you use CHINJU technology, we recommend obtaining "CHINJU Certified" certification or displaying "CHINJU Compatible".

See [CHINJU Charter](./CHINJU_憲章.md) Article 11 for details.

### Commercial License (White Label)

If displaying "CHINJU Compatible" is difficult due to corporate policy or branding, or if you wish to use the technology under your own brand (White Label), you can obtain a separate commercial license (paid) to waive the display requirement.

Please contact the secretariat for detailed terms and contracts.

### Trademark & Brand Usage

| Name | Requirements | Description |
|:--|:--|:--|
| **CHINJU** | Official certification (contract) required | Only for products/services that have passed certification |
| **CHINJU Certified** | Official certification (contract) required | Official certification mark. Only for certified entities |
| **CHINJU Compatible** | Free (with conditions) | For implementations that comply with the protocol |

**CHINJU Compatible Requirements**:
- Must comply with the protocol specifications in this repository
- Must display "CHINJU Compatible" or equivalent
- Cannot use "CHINJU" alone (to avoid confusion with official certification)
- Must clearly state "unofficial implementation"

### About Defensive Publication

By publishing this repository, the described technologies become **prior art**.

| Effect | Description |
|:--|:--|
| **Blocks third-party patents** | Prior art lacks novelty; others cannot obtain patents on identical technology |
| **Defense against existing patents** | Serves as grounds for invalidating similar patents filed after this publication date |
| **Guarantees free implementation** | CHINJU Compatible display enables free implementation |

---

## Vision

CHINJU aims to build an open technology standard and certification ecosystem, similar to **Linux Foundation / CNCF** and **Wi-Fi Alliance**.

| Reference Model | Description |
|:--|:--|
| **Linux Foundation / CNCF** | OSS technology + certification programs (CKA, CKAD, etc.) |
| **Wi-Fi Alliance** | Open specification + Wi-Fi Certified |
| **Bluetooth SIG** | Open specification + Bluetooth Certified |
| **Red Hat** | OSS (Linux) + enterprise support |

Technology is open; quality is guaranteed through certification.
