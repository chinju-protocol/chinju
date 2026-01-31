# CHINJU Protocol
**The Physical Safety Layer for AGI**

---

## What is CHINJU?

CHINJU is an open-source protocol that provides a **hardware-based safety layer** for AI systems. Unlike software-only approaches, CHINJU anchors AI behavioral rules to **physical laws** that cannot be bypassed regardless of AI capability.

---

## The Challenge

Current AI safety measures (RLHF, Constitutional AI, guardrails) rely on software, which creates inherent risks:

- AI systems may find ways to circumvent software constraints
- Developers or external pressures may weaken safety rules over time
- As AI capabilities grow, software-based controls become harder to maintain

---

## Our Solution

CHINJU addresses these challenges through **physics-based guarantees**:

| Layer | Mechanism | Guarantee |
|-------|-----------|-----------|
| **Physical** | OTP/eFuse circuits, TPM chips | Rules burned into hardware cannot be modified—melted circuits don't restore |
| **Cryptographic** | Threshold signatures, survival tokens | No single party can override safety controls |
| **Information-theoretic** | Entropy monitoring, output sanitization | AI manipulation and hidden communication are mathematically detectable |
| **Institutional** | Multi-stakeholder governance (Charter) | Distributed authority prevents capture by any single entity |

---

## Key Differentiator

| Traditional AI Safety | CHINJU |
|----------------------|--------|
| Relies on AI cooperation | Independent of AI intent |
| Software-based controls | Physics-based guarantees |
| Single points of failure | Multi-layer defense |
| Can be modified after deployment | Immutable once anchored |
| Vulnerable to Quantum Attacks | **Quantum-Resistant** (Information-Theoretic) |

---

## Patent Portfolio

CHINJU is protected by **13 patents** covering:

- **C1-C5**: Physical control mechanisms (hardware kill switches, survival tokens)
- **C6-C9**: Governance and sovereignty (distributed authority, policy management)
- **C10-C13**: Anti-manipulation (definition immutability, model confinement)

---

## Why Now?

- **EU AI Act (2026)**: Mandates risk assessments for high-risk AI systems
- **Rapid AI advancement**: GPT-4, Claude, and successors require robust safety infrastructure
- **Industry need**: AI companies need safety solutions that don't compromise capability

---

## Business Model

- **CHINJU Certified (Official Certification)**: Paid certification for implementations meeting safety standards
- **CHINJU Compatible (Self-Declaration)**: Free compliance declaration (for adoption promotion)
- **Hardware licensing**: For TPM/OTP integration
- **Enterprise consulting**: Custom implementation support

---

## Adoption Path

CHINJU is designed as a **sidecar**—it wraps existing AI systems without modifying them:

```
[User] → [CHINJU Sidecar] → [AI Model (OpenAI, Anthropic, etc.)] → [CHINJU Sidecar] → [User]
```

All major AI providers can be customers, not competitors.

---

## Open Source Commitment

- Core protocol: **Apache 2.0 License**
- Charter: **Immutable** (cannot be changed to weaken safety guarantees)
- Patents: **Defensive** (not enforced against CHINJU Compatible implementations)

---

## Learn More

- **Charter**: Defines immutable principles for human-AI coexistence
- **Technical Specifications**: Detailed protocol documentation available
- **Reference Implementation**: Open-source Rust implementation

---

**Contact**: CHINJU Community
**Repository**: github.com/chinju-protocol/chinju-core
