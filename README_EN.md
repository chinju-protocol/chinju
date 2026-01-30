# CHINJU - AI Safety Patent Portfolio

> A safety layer that **ensures human control over AI** through physical laws and information theory

[日本語版 (README.md)](./README.md)

---

## Table of Contents

- [Overview](#overview)
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

CHINJU consists of 12 patents forming an AI safety layer.
Based on the premise that "software alone cannot stop AI," it prevents AI runaway through **physical irreversibility** and **mathematical principles**.

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
3. **Read the details**: Review the patent specifications (`c1_*.md` to `c9_*.md`, English versions in `en/`)
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
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                      │
│  │C2 Choice│  │C6 Advers│  │C7 Learn │                      │
│  │ Quality │  │  Audit  │  │ Refusal │                      │
│  └────┬────┘  └────┬────┘  └────┬────┘                      │
│       │            │            │                            │
│       ▼            ▼            ▼                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Information-Theoretic Layer            │   │
│  │  QRNG(unpredictable) + Entropy(quantify) + Adversarial  │
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
chinju-ip/
├── README.md                 # Japanese version
├── README_EN.md              # This file (English)
├── c1_*.md 〜 c9_*.md        # Patent specifications (9 files, Japanese)
├── en/                       # English translations
│   └── c1_*.md 〜 c9_*.md    # Patent specifications (English)
└── sample/                   # Implementation samples & specifications
    ├── spec/                 # Protocol specifications
    │   ├── 00_overview.md
    │   ├── 01_c1_hcal.md 〜 09_c9_policy_pack.md
    │   ├── CONFIG_SCHEMA.md
    │   ├── ERROR_CODES.md
    │   └── VERSIONING.md
    ├── hardware/             # Hardware implementation guide
    │   ├── REQUIREMENTS.md
    │   ├── SECURITY_LEVELS.md
    │   ├── INTEGRATION_GUIDE.md
    │   └── reference/        # OTP, HSM, TEE, QRNG details
    ├── traits/               # Abstraction interfaces
    └── extensions/           # Extensions
```

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
| **CHINJU Patent Portfolio (C1-C12)** | Patent Policy (separate) |

Apache 2.0 applies to code copyright. Patents are governed by a separate policy.

### Patent Policy

| User | Patent Enforcement |
|:--|:--|
| **CHINJU Certified** holders | **Not enforced** |
| **CHINJU Compatible** displayers | **Not enforced** |
| Uses CHINJU technology without display | **May be enforced** (policy violation) |
| Misuse / attackers | **Enforced** (subject to litigation) |

**Important**: If you use CHINJU technology, we recommend obtaining "CHINJU Certified" certification or displaying "CHINJU Compatible".

See [CHINJU Charter](./CHINJU_Charter_v1.0.md) Article 10 for details.

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
