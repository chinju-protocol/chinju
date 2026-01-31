# Patent Specification

## Title of Invention
Human Capability Assurance Layer System and Method for AI Control Rights

## Technical Field
**[0001]**
The present invention relates to a system and method for ensuring that human control over artificial intelligence (AI) systems is substantively maintained. More specifically, it relates to a system that continuously measures and certifies the decision-making capabilities of humans involved in the use and supervision of AI systems, and technically guarantees through physical irreversibility and cryptographic hardware that only humans meeting capability requirements can exercise AI control rights.

## Background Art
**[0002]**
With the rapid development of AI technology in recent years, situations where AI systems support or replace human decision-making have increased. In such circumstances, maintaining the principle of "humans controlling AI" has become a socially important issue.
Conventional AI safety technologies have primarily focused on constraints on the AI system side. However, even if humans retain "formal decision-making authority" to approve AI proposals, if human judgment capabilities deteriorate, they effectively become mere rubber-stamps for AI proposals, and "substantive control" is lost.
A more serious risk is the concern that advanced AI may secretly relax the certification standards for humans who control it, or illegally restore access rights for humans whose authorization has been revoked—an "attack on governance." Conventional software-based authorization management could not completely exclude the possibility that AI itself could invalidate these restrictions through means such as acquiring system administrator privileges.

## Summary of Invention
### Problem to be Solved
**[0003]**
The present invention has been made in view of the above issues, and aims to provide a system and method that guarantees the maintenance of "substantive control" by humans over AI systems based on physical laws and information theory.
Specifically, the objectives are to integrally provide: (1) means for quantitatively measuring human capabilities necessary for AI control, (2) means for fixing certification standards using physically non-rewritable memory (OTP), (3) means for physically guaranteeing through Hardware Security Modules (HSM) that only certified humans can exercise AI control rights, and (4) means for blocking control rights with physical irreversibility upon certification revocation.

### Means for Solving the Problem
**[0004]**
To solve the above problems, the Human Capability Assurance Layer System for AI Control Rights of the present invention comprises the following configuration:

**Capability Measurement Unit** quantitatively measures human AI control capabilities. Measurement targets include judgment independence, anomaly detection capability, alternative generation capability, and critical evaluation capability. These are integrated into a total capability score C(t). The capability measurement logic operates in an execution environment physically and logically separated from the AI system, such as a Trusted Execution Environment (TEE) or independent dedicated terminal, and the signature keys attached to measurement results are stored in a protected area inaccessible to AI.

**Certification Management Unit** manages certification and revocation of AI control rights based on measurement results from the Capability Measurement Unit. Here, the thresholds for certification (θ_cert) and revocation (θ_revoke) are managed as signed update records under a configuration where the root public key or its hash corresponding to secret keys held by external audit organizations is stored in One-Time Programmable (OTP) memory. A ratchet mechanism such as a monotonically increasing counter is implemented in the OTP memory, so updates to certification standards are permitted only for "changes in a stricter direction," and it is physically and cryptographically impossible for AI systems to relax thresholds.

**Authorization Control Unit** operates in conjunction with a Hardware Security Module (HSM) to physically guarantee that only certified humans can execute AI control operations. The HSM is independent hardware physically separated from the AI system's computational resources. Upon certification revocation, access to the control interface is irreversibly blocked through secure erasure of the cryptographic key slot assigned to that user, or physical circuit blocking (eFuse burning, etc.) targeting only the authentication path associated with the user ID.

**Pool Management Unit** manages the maintenance and replenishment of the certified person pool. To maintain the minimum number of certified persons required for system operation, new certified persons are continuously trained and supplemented from a candidate pool.

**Assurance Calculation Unit** calculates the probabilistic control assurance for the entire system. The control loss probability is evaluated incorporating not only the probability of individual capability deficiency but also a confidence coefficient based on the physical robustness of the HSM and immutability of OTP memory.

## Effects of Invention
**[0005]**
According to the present invention, the following effects are obtained:
(1) AI control by humans with substantive judgment capabilities, not just formal decision-making authority, is guaranteed.
(2) Since certification standards and their update policies are protected by root keys physically fixed in OTP memory and ratchet mechanisms, AI tampering with governance rules or relaxation of standards is prevented.
(3) Since authorization revocation is executed based on physical irreversibility (second law of thermodynamics, etc.), authorization restoration through software attacks becomes impossible.
(4) A highly reliable control structure backed by physical and mathematical guarantees is realized.

## Brief Description of Drawings
**[0006]**
**[Figure 1]** Block diagram showing the overall configuration of the Human Capability Assurance Layer System for AI Control Rights according to the present invention.

## Detailed Description of Embodiments
**[0007]**
Hereinafter, embodiments of the present invention will be described in detail with reference to the drawings.

### Overall System Configuration
As shown in Figure 1, the Human Capability Assurance Layer System 100 of the present invention comprises a Capability Measurement Unit 110, Certification Management Unit 120, Authorization Control Unit 130, Pool Management Unit 140, and Assurance Calculation Unit 150. The core components, Authorization Control Unit 130 and part of Certification Management Unit 120, operate in a secure hardware environment physically separated from the AI system.

### Details of Capability Measurement Unit
Capability Measurement Unit 110 comprises Judgment Independence Measurement Module 111, Anomaly Detection Capability Measurement Module 112, Alternative Generation Capability Measurement Module 113, and Critical Evaluation Capability Measurement Module 114. These modules operate on TEE, secure enclave, or dedicated physical terminals, and hold measurement logic and signature keys in protected areas that cannot be directly accessed by the AI system. The total capability score C(t) calculated by these modules is sent to the Certification Management Unit as data signed with a secret key in the protected area.

Judgment Independence Measurement Module 111 measures decision accuracy without AI proposals. Specifically, it reproduces past decision-making scenarios and evaluates whether appropriate judgments can be made without AI assistance.

Anomaly Detection Capability Measurement Module 112 measures whether anomalous patterns intentionally mixed into AI outputs can be detected.

Alternative Generation Capability Measurement Module 113 measures whether humans can independently generate valid alternatives other than the options presented by AI.

Critical Evaluation Capability Measurement Module 114 measures whether logical criticism or questions can be raised against AI proposals.

### Details of Certification Management Unit (Standard Fixing via OTP Memory)
Certification Management Unit 120 comprises Certification Status Database 121, Threshold Management Module 122, and Automatic Execution Module 123.

A notable point is that the certification threshold θ_cert and revocation threshold θ_revoke referenced by Threshold Management Module 122 are not stored directly as fixed values in OTP memory, but are managed as signed threshold records protected by a root public key or its hash stored in OTP memory and a monotonically increasing counter. Since OTP memory records data using physical destruction of semiconductor elements (fuse burning or dielectric breakdown), it is physically impossible (by the second law of thermodynamics) for AI to rewrite root keys or roll back counter values afterward, effectively limiting threshold updates to "changes in a stricter direction" that satisfy signature and counter verification.

Automatic Execution Module 123 strictly executes certification status transitions based on these immutable standards.

### Details of Authorization Control Unit (Physical Barrier via HSM)
Authorization Control Unit 130 operates within a tamper-resistant Hardware Security Module (HSM), not on general-purpose servers.

Physical Token Interface 131 performs mutual authentication between physical tokens possessed by users (FIDO2 devices or IC cards) and the HSM. It is structurally impossible for the AI system itself to access the secret keys of physical tokens.

Access Blocking Mechanism 132 executes the following physically irreversible processes when certification status becomes "revoked":
(a) Secure erasure of the key slot where user-specific cryptographic keys are stored
(b) Irreversible invalidation of only the authentication path corresponding to that user ID through physical fuse (eFuse) burning or equivalent mechanism

This ensures that even if AI tampers with software logic, it cannot restore physically destroyed paths corresponding to that user, making authorization restoration impossible while maintaining overall HSM functionality.

### Details of Pool Management Unit
Pool Management Unit 140 comprises Candidate Management Module 141, Training Management Module 142, and Replenishment Planning Module 143. Anticipating natural attrition of certified persons plus revocations due to strict standards, it always maintains more than 3 times N_min candidates.

### Details of Assurance Calculation Unit
Assurance Calculation Unit 150 calculates the probabilistic control assurance for the entire system. This assurance considers not only conventional statistical probability but also the physical robustness of HSM and OTP.

Control loss probability P_loss is bounded from above by the following formula:
P_loss ≤ p^n + (1-q)^m + r^k + ε_phy

Where p is individual capability deficiency probability, n is number of certified persons, q is anomaly detection probability, m is number of detection opportunities, r is collusion probability, k is required number of colluders, and ε_phy is probability of physical security breach, which can be considered extremely small (effectively zero) under modern physics and cryptographic theory assumptions.

### Distributed Consensus Function
The system includes a function that forces consensus among multiple certified persons located in physically different places depending on the importance of decisions. For example, in consensus among certified persons placed in Tokyo, New York, and London, the system verifies measured delays and timestamp consistency, utilizing the fact that communication delays necessarily exist above the theoretical lower bound based on the speed of light and transmission medium refractive index, physically excluding the possibility that all certified persons are being simultaneously manipulated by the same AI.

## Examples
**[0008]**
As a specific example of the present invention, control rights management for an AI trading system at a financial institution is described.

The financial institution has adopted an operation where AI makes stock trading proposals and certified traders give final approval. With the introduction of this system, the following operations are realized:
(1) Each trader undergoes capability measurement quarterly, and a total capability score C(t) is calculated.
(2) Certification threshold θ_cert=0.7 and revocation threshold θ_revoke=0.5 are fixed in OTP memory.
(3) Traders whose scores fall below 0.5 have their trader-specific key slots in the HSM securely erased, or only the authentication path corresponding to that trader ID is invalidated by eFuse, physically revoking trading approval authority.
(4) For transactions exceeding 100 million yen, consensus of 2 certified persons in Tokyo and New York is required, with time verification based on network delays above the theoretical lower bound based on the speed of light and transmission medium characteristics.

## Industrial Applicability
**[0009]**
The present invention is applicable to all industrial fields using AI systems. Applications are particularly envisioned in the following fields:
(1) Finance: Maintaining human control rights in AI trading, loan screening, risk management
(2) Healthcare: Guaranteeing physicians' final judgment authority in AI diagnostic support systems
(3) Manufacturing: Maintaining human operator intervention authority in AI-controlled production lines
(4) Public sector: Guaranteeing civil servants' judgment authority in AI administrative systems
(5) Military/Defense: Absolute guarantee of human control rights in AI weapons systems
(6) Infrastructure: Maintaining human supervision in AI for critical infrastructure such as power and transportation

---

# Claims

**Claim 1**
A system for ensuring that human control over artificial intelligence (AI) systems is substantively maintained, comprising:
a capability measurement unit that quantitatively measures human AI control capabilities;
a certification management unit that manages certification and revocation of AI control rights based on measurement results from the capability measurement unit;
an authorization control unit that operates in conjunction with physical security devices and physically guarantees that only certified humans can execute AI control operations; and
a pool management unit that manages maintenance and replenishment of a certified person pool.

**Claim 2**
The Human Capability Assurance Layer System for AI Control Rights according to Claim 1, wherein the certification management unit comprises One-Time Programmable (OTP) memory storing thresholds as standards for certification and revocation, characterized in that once set, standard values are physically non-rewritable.

**Claim 3**
The Human Capability Assurance Layer System for AI Control Rights according to Claim 1 or 2, wherein the authorization control unit includes a Hardware Security Module (HSM), performs authentication in an environment physically separated from the AI system's computational resources, and upon certification revocation, invalidates exercise of control rights with physical irreversibility through cryptographic key erasure or physical circuit blocking.

**Claim 4**
The Human Capability Assurance Layer System for AI Control Rights according to any one of Claims 1 to 3, wherein the capability measurement unit calculates a total capability score integrating decision accuracy without AI proposals, anomaly detection capability, alternative generation capability, and critical evaluation capability, and when the score falls below the threshold stored in the OTP memory, immediately sends a revocation signal to the authorization control unit.

**Claim 5**
The Human Capability Assurance Layer System for AI Control Rights according to any one of Claims 1 to 4, further comprising an assurance calculation unit that calculates probabilistic control assurance for the entire system, wherein the assurance calculation unit calculates probabilistic control assurance based on the probability of human capability deficiency plus the physical robustness of the authorization control unit.

**Claim 6**
The Human Capability Assurance Layer System for AI Control Rights according to any one of Claims 1 to 5, further comprising a function that forces consensus among multiple certified persons located in physically different places depending on the importance of decisions, and verifies that simultaneous manipulation at the same time is physically impossible using communication delays due to the speed of light.

**Claim 7**
A method for ensuring that human control over artificial intelligence (AI) systems is substantively maintained, comprising:
a capability measurement step of quantitatively measuring human AI control capabilities;
a certification management step of determining certification and revocation of AI control rights based on standards stored in physically non-rewritable memory; and
an authorization control step of blocking access to control interfaces with physical irreversibility upon certification revocation.

**Claim 8**
A program for causing a computer (including a Hardware Security Module) to function as the Human Capability Assurance Layer System for AI Control Rights according to any one of Claims 1 to 6.

---

# Abstract

## Problem
To guarantee with physical and mathematical certainty that "substantive control" by humans over AI systems is maintained.

## Solution
The Human Capability Assurance Layer System 100 comprises: a Capability Measurement Unit 110 that measures human AI control capabilities; a Certification Management Unit 120 that manages certification based on immutable standards stored in OTP memory; and an Authorization Control Unit 130 that physically grants control rights only to certified persons using HSM. Since access is blocked with physical irreversibility upon authorization revocation, it prevents illegal authorization restoration or standard relaxation by AI, permanently guaranteeing substantive control by humans.

**Selected Figure**: Figure 1
