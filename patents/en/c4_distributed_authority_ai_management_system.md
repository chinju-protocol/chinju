# Patent Specification

## Title of Invention
Distributed Authority AI Management System and Consensus Protocol

## Technical Field
**[0001]**
The present invention relates to governance technology for artificial intelligence (AI) systems, and particularly to a Distributed Authority AI Management System and Consensus Protocol that distributes authority for changing AI core parameters or constitution (basic principles) among multiple management entities and permits changes only when predetermined consensus conditions are met.

## Background Art
**[0002]**
In recent years, development of large language models (LLMs) and artificial general intelligence (AGI) has progressed rapidly. These AI systems are increasing in importance as social infrastructure, and implementation of "constitutions" and "guardrails" to ensure their safety and ethics is progressing.

However, in conventional AI systems, a single company or organization that is the developer exclusively holds "root authority." This poses risks (single point of failure risks) where AI safety mechanisms may be disabled or constitutions may be rewritten due to policy changes by the developer company, legal enforcement by specific nations, or hacking by malicious insiders.

Furthermore, if AI itself becomes advanced and acquires the ability to rewrite its own code (self-modification capability), a single stop switch may be disabled by AI.

## Summary of Invention
### Problem to be Solved
**[0003]**
In view of the above issues, the present invention aims to provide a Distributed Authority AI Management System that structurally forces consensus formation by multiple opposing and independent entities without monopolizing control authority over core AI systems by a specific company or nation, thereby physically and logically preventing arbitrary modification or runaway.

### Means for Solving the Problem
**[0004]**
The system of the present invention comprises the following configuration:

1. **Key Management Unit** that splits authentication keys required for changing AI system core parameters (weights, constitution, stop conditions, etc.) into n shards (fragments) using secret sharing methods (Shamir's Secret Sharing, etc.) and distributes them to multiple management nodes with different interests (developers, regulatory authorities, audit organizations, user representatives, etc.).

2. **Consensus Formation Unit** that, when a change request to the AI system occurs, requests approval signatures from the multiple management nodes and issues a change permission token only when signatures equal to or exceeding a preset threshold m (m≤n) are collected.

3. **Access Control Unit** embedded in the AI system's execution engine or at the hardware level (TPM, etc.) that physically or logically blocks write access to core parameters unless the change permission token is verified.

## Effects of Invention
**[0005]**
According to the present invention, it becomes possible to technically enforce separating AI system control from any single organization and establishing democratic and distributed management structure (multi-stakeholder governance). This can reliably prevent runaway by specific companies, seizure by specific nations, or self-modification by AI itself running amok.

---

# Claims

**Claim 1**
A system for managing authority to change parameters or rules controlling operation of artificial intelligence systems, comprising:
key distribution means that splits master key information for exercising the change authority into multiple key fragments using a secret sharing method and stores them distributed among terminals of multiple management entities registered in advance;
consensus formation means that, upon receiving a change request for the artificial intelligence system, sends approval requests to terminals of the multiple management entities and collects electronically signed approval responses from each terminal; and
execution control means that, only when the number of collected approval responses equals or exceeds a predetermined threshold and each signature is verified as legitimate, temporarily reconstructs the master key information or generates a control signal that validates the change authority.

**Claim 2**
The Distributed Authority AI Management System according to Claim 1, wherein the multiple management entities are selected from entities with different attributes (development companies, government agencies, third-party audit organizations, user communities, etc.) that are geographically, politically, or economically independent, and key fragment distribution ratios are set so that entities of only a specific attribute cannot meet the threshold.

**Claim 3**
The Distributed Authority AI Management System according to Claim 1 or 2, wherein verification logic for the control signal is embedded in the artificial intelligence system's inference engine or hardware platform in a non-modifiable format, and further comprises a kill switch function that stops operation of the artificial intelligence system immediately upon detecting parameter changes or rule changes made without legitimate control signals.

**Claim 4**
The Distributed Authority AI Management System according to any one of Claims 1 to 3, wherein the consensus formation means dynamically changes the required threshold or required combinations of management entities depending on the content of the change request (constitution change, emergency stop, parameter update, etc.).

**Claim 5**
The Distributed Authority AI Management System according to any one of Claims 1 to 4, further comprising recovery means that, in case any key fragment is lost or damaged, regenerates and distributes a new key fragment set using a threshold higher than normal or specific recovery backup keys based on a predefined emergency recovery protocol.

---

## Detailed Description of Embodiments
**[0006]**
Hereinafter, embodiments of the present invention will be described in detail.

### Embodiment 1: Multi-Stakeholder Governance
This system is used to manage the most important authorities such as "constitution" and "stop switch" of superintelligent AI (hereinafter, target AI).

**(1) Key Splitting and Distribution**
Master key K for accessing root authority of target AI is split into n=7 shards (S1...S7) using Shamir's secret sharing method ((k, n) threshold method).
These are distributed to the following entities:
- S1: Development company (Tech Giant)
- S2: Development country regulator (e.g., US AI Safety Institute)
- S3: Competing country regulator (e.g., EU AI Office)
- S4: Japan regulator or neutral organization
- S5: International third-party audit organization
- S6: Developer/researcher community representative (Academic)
- S7: General user representative or DAO (Civil Society)

**(2) Consensus Formation Flow**
When a request to rewrite target AI's constitution (e.g., relax "prohibition of harm to humans") occurs, the system notifies all entities.
To execute the change, threshold k=5 or more signatures are required.
This means development company (S1) and development country (S2) colluding only have 2 votes, and the change is rejected. Conversely, if AI runs amok and emergency stop is needed, S3-S7 can agree to execute the stop code regardless of the developer's wishes.

### Embodiment 2: Hardware Enforcement via TPM
This system cooperates not only with software authentication but also with physical Hardware Security Modules (TPM: Trusted Platform Module).
Each node of the GPU cluster where target AI operates is equipped with a TPM chip.

1. **Boot-time verification**: At system startup, TPM verifies the hash value of the "constitution file."
2. **Runtime verification**: When the constitution file is changed, TPM requires a legitimate signed token (signed with k or more shards) issued from the "consensus formation server."
3. **Forced shutdown**: If the constitution file is rewritten without a legitimate token (hash mismatch), TPM physically cuts power supply to the GPU or destroys decryption keys to invalidate model data in memory.

This ensures that even if hackers or AI itself seize OS administrator authority, changes are blocked at the hardware level.

### Embodiment 3: Emergency Recovery (Dead Man's Switch)
A recovery mode is provided in case an entity holding a key (e.g., S7 user representative) becomes unreachable or loses their secret key.
When conditions such as "no heartbeat from S7 for 3 months" (Dead Man's Switch) are met, a process becomes executable to invalidate S7's key and elect a new S7' to reissue the key, with consent from remaining entities (e.g., k=6).

## Examples
**[0008]**
The following describes a specific example of the present invention.

As Example 1, a case of "constitution revision defense" in a globally deployed generative AI service is described.
The target AI system is an LLM platform used by hundreds of millions of people worldwide, operated by Company A. The AI has a constitution defining "prohibition of discriminatory speech," "political neutrality," "respect for life," etc.

In this example, parameters for authority distribution were set as: total number of key fragments n=5, consensus threshold k=3.
Key fragment distribution destinations are as follows:
1. Development company A (S1)
2. International AI Safety Organization (S2)
3. Third-party audit firm (S3)
4. User representative DAO (S4)
5. Ethics committee (S5)

At one point, development company A (S1) proposed relaxing the constitution to enable specific political censorship for monetization (attack scenario).
Company A issued a change request and signed with its key S1. Also, it persuaded the audit firm (S3), a party with some interests, to obtain their signature (current signature count = 2).

However, in the consensus formation unit, the following process proceeded:
- International AI Safety Organization (S2) determined that the change conflicts with international human rights standards and refused to sign.
- User representative DAO (S4) held a governance vote and decided to refuse signing with a 90% majority opposing.
- Ethics committee (S5) was concerned about long-term social impact and decided to refuse signing.

As a result, the collected signature count was 2 (S1, S3), falling short of threshold 3, so the consensus formation unit rejected issuance of the change permission token.
The access control unit (TPM) blocked the constitution file rewrite operation (write) since no legitimate token existed, and the system continued operating with the old constitution.
This entire process (proposal, voting, rejection) was published as an audit log on a tamper-proof blockchain, making Company A's unilateral change attempt transparent.
This confirmed that AI ethical principles were systematically defended against damage by a single company's economic motivation.

## Industrial Applicability
The present invention is widely applicable as governance technology for large-scale systems requiring high public interest and safety, such as AGI development and operation platforms, autonomous vehicle fleet management systems, CBDC (central bank digital currency) management systems, and smart city control OS.

---

# Abstract

## Problem
When authority to change AI system core parameters or constitution (basic principles) is concentrated in a single company or nation, there are risks of safety mechanisms being disabled due to policy changes, legal coercion, or internal fraud.

## Solution
Split master key information required for change authority into multiple key fragments using secret sharing methods and store them distributed among multiple management entities with different interests. Collect electronically signed approval responses from management entities upon change requests, and issue change permission tokens only when approval equal to or exceeding a predetermined threshold is obtained. Furthermore, embed verification logic in the inference engine or hardware platform such as TPM, and execute stop or shutdown when modification without legitimate tokens is detected.

**Selected Figure**: None
