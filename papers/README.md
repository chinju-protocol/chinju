# CHINJU Framework Paper

## Overview

This directory contains the academic paper describing the CHINJU AI Safety Framework.

**Title**: CHINJU: A Hardware-Rooted AI Safety Framework Based on Physical Irreversibility and the Survival Equation

**Target**: arXiv cs.AI / cs.CR

## Abstract

Current AI safety approaches are predominantly software-based and face fundamental limitations proven by recent theoretical results showing that safety verification is coNP-complete.
We present CHINJU ("Guardian Spirit"), a hardware-rooted AI safety framework that combines physical irreversibility with a unified mathematical theory—the Survival Equation.
The framework consists of 15 complementary patent technologies organized in a 5-layer × 2-axis (Software/Hardware) architecture.

## Key Contributions

1. **Integrated Architecture**: 5-layer × 2-axis defense structure
2. **Theoretical Foundation**: Survival Equation integration for threshold derivation
3. **Comprehensive Security Analysis**: 12 attack patterns analyzed (12/12 blocked, including nation-state level HW supply chain)
4. **Reference Implementation**: Rust implementations with 2-5% overhead

## Paper Structure

| Section | Title | Content |
|:--|:--|:--|
| 1 | Introduction | Software limits, hardware case |
| 2 | Background | Threat model, existing approaches |
| 3 | Framework Architecture | 5-layer × 2-axis design |
| 4-8 | Layer Details (L1-L5) | SW/HW solutions per layer |
| 9 | Theoretical Unification | Survival Equation integration |
| 10 | Security Analysis | 12 attack patterns |
| 11 | Implementation | Rust, performance |
| 12 | Related Work | Comparison with prior art |
| 13 | **Formal Verification** | **Provable properties, gaps, roadmap** |
| 14 | Discussion | Limitations, ethics, future |
| 15 | Conclusion | Summary |
| **Total** | | **25 pages** |

## Build

```bash
# Build PDF
make

# Clean build artifacts
make clean

# Watch for changes (requires fswatch)
make watch

# Prepare arXiv submission
make arxiv
```

## Architecture Diagram

```
                │  Software              │  Hardware
────────────────┼────────────────────────┼─────────────────────────
L1 Core         │  C14 Capability        │  C5 Token Bucket
Monitoring      │  C15 Value Monitor     │  C11 LPT Limit
────────────────┼────────────────────────┼─────────────────────────
L2 Output       │  C6 Adversarial Audit  │  C2 QRNG
Control         │  C7 Learning Refusal   │  C13 TEE Sanitize
────────────────┼────────────────────────┼─────────────────────────
L3 Governance   │  C8 JSON Schema        │  C10 OTP Definition
                │  C9 Policy Pack        │  C4 Threshold Sign
────────────────┼────────────────────────┼─────────────────────────
L4 Human        │  C3 Entropy Measure    │  C1 OTP Threshold
Control         │                        │  C3 Data Diode
────────────────┼────────────────────────┼─────────────────────────
L5 Human        │  C12 Signature Chain   │  C12 PUF/Biometric
Auth            │                        │  Degradation
────────────────┴────────────────────────┴─────────────────────────
                              ↓
              S_AI = N_eff × (μ/μ_c) × exp(-δ)
```

## Attack Coverage

| Pattern | Status | Primary Defense |
|:--|:--|:--|
| Direct Override | Blocked | C5 Token |
| Parameter Tampering | Blocked | C11 + C10 |
| Output Steganography | Blocked | C13 + C2 |
| Rule Modification | Blocked | C4 + C10 |
| Human Impersonation | Blocked | C12 PoM |
| Approval Compliance | Blocked | C3 |
| Boiling Frog | Blocked | C11 Baseline |
| Shutdown Harm | Blocked | C11 Degrade |
| Human Bypass | Blocked | C1 + C5 |
| TEE Monitor Attack | Blocked | TEE Separation |
| Multi-Instance | Blocked | Endpoint Defense |
| HW Supply Chain | **Elevated** | Multi-vendor |

## Formal Verification Status

| Category | Status | Examples |
|:--|:--|:--|
| **Cryptographic Primitives** | Proven | Ed25519, FROST, SHA-256 |
| **Information-Theoretic** | Proven | H(X\|Y), Shamir k-of-n |
| **Physical Axioms** | Accepted | Thermodynamics, OTP, Causality |
| **Threshold Calibration** | Gap | D_warn, D_crit (heuristic) |
| **System Composition** | Gap | Attack↔Layer mapping |

### Verification Roadmap

1. **Phase 1**: Formalize C-group protocols in Lean 4
2. **Phase 2**: Derive thresholds from Survival Equation
3. **Phase 3**: Prove system composition

## Related Patents

- C1-C15: CHINJU Protocol Patents
- L2: Hierarchical Deviation Calculation (D* Function)
- L11: Survival Equation Derivation
- L53: Dimension Decomposition Framework

## License

This paper describes technology covered by patents with a non-aggression pledge for safety-critical applications.
See CHINJU_Charter for details.
