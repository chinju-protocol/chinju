# Patent Specification

## Title of Invention
Metric Definition Registry, Machine Verification, and Dialogue Generation Support System

## Technical Field
**[0001]**
The present invention relates to a technical field of fixing metric definitions in machine-readable formats and consistently executing measurement, display, and operation based on those definitions across diverse domains including organizations, individuals, healthcare, education, economics, and AI control. More specifically, it relates to a Metric Definition Registry, Machine Verification, and Dialogue Generation Support System that registers metric definitions as version-managed definition data, mechanically verifies the definition data, and further generates and finalizes the definition data through dialogue.

## Background Art
**[0002]**
Conventionally, metrics for evaluating organizational or service states tend to be scattered across personnel's textual definitions, spreadsheet formulas, or implicit specifications in implementation code. Even with the same metric name, measurement targets, preconditions, handling of missing values, domains, and exception conditions can vary by environment. Consequently, metric comparability is lost, and when meanings change retroactively, reliability and auditability of decision-making are compromised.

Also, when non-developers try to create or customize metrics, measurement failures or misuse easily occur due to definition deficiencies, missing calculation procedures, ambiguous handling of missing data, and absence of falsification conditions. Furthermore, when quality assurance of metric definitions depends on manual review, operational costs increase with scale expansion, making distribution and sharing difficult.

## Summary of Invention
### Problem to be Solved
**[0003]**
The present invention has been made in view of the above issues, and aims to provide metric operations that are measurable regardless of domain, difficult to change meaning retroactively, and robust against missing data, by fixing metric definitions not as free text but as machine-readable definition data, mechanically verifying minimum requirements that the definition data must satisfy, and further generating and finalizing the definition data based on dialogue with users.

### Means for Solving the Problem
**[0004]**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to the present invention comprises at least: a definition registry that stores metric definitions; a definition verification unit that verifies whether the metric definitions satisfy predetermined schema requirements; a dialogue generation unit that generates metric definition candidates based on dialogue input with users; a definition finalization unit that registers and finalizes in the definition registry only candidates that pass verification by the definition verification unit; and an output generation unit that generates metric outputs based on the finalized metric definitions.

In a preferred form, the metric definitions include at least: metric identifier, version information, applicable domain and target type, output domain and meaning, set of proxy observation items required for measurement, calculation procedure, confidence calculation rules, and falsification conditions. The set of proxy observation items includes at least one required item and includes policies such as stop, substitute, or degrade for handling missing values, and confidence degradation rules.

Also, the dialogue generation unit presents to users a questionnaire that sequentially prompts: determination of minimum proxy observation items, degradation policy when missing, output domain and units, and description of falsification conditions, and generates metric definition data including calculation procedures based on responses. Furthermore, the definition verification unit verifies at least one of: schema conformance, output domain consistency, existence of required items, existence of calculation procedure, existence of confidence rules, and existence of falsification conditions.

In a preferred form, the output generation unit outputs at least one of: estimated metric values, confidence, and candidate data that would improve accuracy if added, and associates the output with the version information of the metric definition referenced by that output.

## Effects of Invention
**[0005]**
According to the present invention, by fixing metric definitions as machine-readable definition data and guaranteeing minimum quality through machine verification, metric meaning confusion and retroactive changes can be suppressed. Also, through dialogue generation, non-developers can create and finalize definition data, reducing measurement failures due to definition deficiencies. Furthermore, since handling of missing values and confidence rules are included in definitions, processing can continue degrading as reduced confidence rather than stopping when required data is incomplete.

---

## Detailed Description of Embodiments
**[0007]**
Hereinafter, preferred embodiments of the present invention will be described. However, the present invention is not limited to these embodiments.

The system of this embodiment includes a definition management server operating on a server device and client terminals connected to the server. The definition management server comprises a definition registry, schema storage unit, definition verification unit, dialogue generation unit, definition finalization unit, etc.

The definition registry uniquely manages metric definition data by metric identifier and version information. Metric definition data includes at least: (1) applicable domain and target type, (2) output domain and meaning, (3) set of proxy observation items, (4) calculation procedure, (5) confidence calculation rules, and (6) falsification conditions. Proxy observation items may each include item identifier, description, required flag, recommended observation window, candidate data sources, and missing handling policy.

The definition verification unit verifies whether metric definition data conforms to predetermined schemas held in the schema storage unit, rejecting registration if non-conformant. For example, it verifies: output domain is consistent as numerical values, at least one proxy observation item with required flag exists, calculation procedure exists, confidence rules exist, and at least one falsification condition exists.

The dialogue generation unit receives natural language input from users, organizes metric purpose, applicable domain, targets, candidate available data, etc., and presents a questionnaire. Based on questionnaire responses, the dialogue generation unit generates metric definition data candidates including minimum set of proxy observation items, missing handling policy, calculation procedure, confidence rules, and falsification conditions. Generated candidates are verified by the definition verification unit and registered to the definition registry by the definition finalization unit only if they pass.

In this embodiment, metric definition data may be distributed as extension elements such as widgets or plugins in combination with other distribution metadata. In this case, by including the version information referenced by metric definition data in display output, users can track which definition that output is based on.

## Examples
**[0008]**
The following shows a specific example of the present invention.

As Example 1, a case of definition fixing for "infection risk metrics" and panic prevention during a pandemic is described.
Background: During unknown infectious disease outbreaks, media and municipalities announced "infection rates" and "risk levels" with their own definitions, causing citizen confusion (excessive self-restraint or hoarding).

In this example, a public health agency used this system to define and register "Infection Risk Metric v1.0."

**1. Definition Fixing:**
- Identifier: `public_health.infection_risk.v1`
- Output domain: 0-100 (integer), meaning: "healthcare strain level based on estimated new infections per 100,000 people"
- Proxy observation items: "PCR positive count (required)," "fever consultation count (recommended)," "sewage virus concentration (recommended)"
- Missing handling policy: "If PCR positive count is missing, measurement is impossible and stops. If only fever consultations available, output 50% value with confidence 'low' (degraded operation)"
- Falsification condition: "If proportion of asymptomatic carriers exceeds 50%, this metric may underestimate risk (false negative)"

**2. Machine Verification and Distribution:**
The definition verification unit verified calculation procedure closure and domain consistency, and registered to the registry.
News apps and municipal dashboards loaded this "v1.0" definition data and displayed risk levels.

**3. Operation and Effect:**
In one region, "PCR positive count" reporting was delayed due to test kit shortages.
Conventionally this would be misreported as "infection count dropping sharply," but this system correctly displayed "measurement impossible (data insufficient)" or "fever consultation-based estimate (confidence: low)" following the missing handling policy.
This prevented false safety declarations due to data deficiencies and maintained appropriate citizen behavioral changes. Also, when later updated to definition v1.1, continuity with past values was guaranteed through version management.

## Industrial Applicability
**[0009]**
The present invention is applicable to all domains requiring metric definition and operation, including organizational management, healthcare, education, economics, and AI control. It is particularly useful in fields requiring metric distribution, sharing, audit, and reproducibility, for fixing metric definitions machine-readably, supporting creation through dialogue generation, and guaranteeing quality through machine verification.

---

# Claims

**Claim 1**
A Metric Definition Registry, Machine Verification, and Dialogue Generation Support System, comprising:
a definition registry that stores metric definitions;
a definition verification unit that verifies whether the metric definitions satisfy predetermined schema requirements;
a dialogue generation unit that generates metric definition candidates based on dialogue input with users;
a definition finalization unit that registers and finalizes in the definition registry only candidates that pass verification by the definition verification unit; and
an output generation unit that generates metric outputs based on the finalized metric definitions.

**Claim 2**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to Claim 1, wherein the metric definitions include at least: metric identifier, version information, applicable domain and target type, output domain and meaning, set of proxy observation items required for measurement, calculation procedure, confidence calculation rules, and falsification conditions.

**Claim 3**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to Claim 1 or 2, wherein the set of proxy observation items includes at least one proxy observation item with a required flag, and includes at least one policy of stop, substitute, or degrade for handling missing values.

**Claim 4**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 3, wherein the definition verification unit verifies at least one of: output domain consistency, existence of required flag proxy observation items, existence of calculation procedure, existence of confidence calculation rules, and existence of falsification conditions.

**Claim 5**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 4, wherein the dialogue generation unit presents a questionnaire sequentially prompting determination of minimum proxy observation items, degradation policy when missing, output domain and units, and description of falsification conditions, and generates metric definition candidates based on responses.

**Claim 6**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 5, wherein when metric definition candidates fail verification by the definition verification unit, the definition finalization unit rejects registration of the candidates, and the dialogue generation unit presents additional questions or modification candidates according to failure reasons.

**Claim 7**
A Metric Definition Registry, Machine Verification, and Dialogue Generation Support Method, comprising:
a step of holding metric definitions in a definition registry;
a step of generating metric definition candidates based on dialogue input with users;
a step of verifying whether the candidates satisfy predetermined schema requirements; and
a step of registering and finalizing the candidates in the definition registry only if verification passes.

**Claim 8**
A program for causing a computer to function as the Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 6.

**Claim 9**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 6, wherein the metric definitions include domain invariants and falsification conditions machine-readably, and the definition verification unit verifies existence of at least one of the invariants or falsification conditions.

**Claim 10**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 6, wherein the dialogue generation unit generates metric definition candidates using a machine learning model or large language model, and the definition finalization unit finalizes only candidates that pass verification by the definition verification unit.

**Claim 11**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 6, wherein the metric definitions include rules for degrading output confidence according to missing proxy observation items, and the dialogue generation unit generates candidate data that would improve accuracy if added when missing and associates them with the metric definitions.

**Claim 12**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 6, wherein the definition registry holds metric definitions as immutable by version information, and permits changes to metric definitions only as new registrations accompanied by new version information.

**Claim 13**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 12, wherein the output generation unit outputs confidence in addition to metric estimated values, and associates the output with the version information of the metric definition referenced by that output.

**Claim 14**
The Metric Definition Registry, Machine Verification, and Dialogue Generation Support System according to any one of Claims 1 to 13, wherein the output generation unit generates candidate data that would improve accuracy if added according to missing proxy observation items, and includes the candidate data in output.

---

# Abstract

## Problem
Metrics used in organizations, healthcare, economics, and other fields have definitions scattered across personnel's free text or implementation code, and even with the same name, formulas and preconditions may differ. This loses metric comparability, and risks of retroactive meaning changes and measurement failures due to ambiguous missing data handling arise. Also, creating rigorous metric definitions is difficult for non-developers, and quality assurance costs increase.

## Solution
Hold metric definitions in a registry as machine-readable definition data including calculation procedures, domains, missing handling policies, and falsification conditions. Mechanically verify whether definition data satisfies predetermined schema requirements (closure, consistency, etc.), registering and finalizing only if it passes. Furthermore, provide a dialogue generation unit that generates appropriate metric definition candidates from purpose and available data based on dialogue input with users. This fixes metric meanings regardless of domain and realizes verifiable and robust measurement operations.

**Selected Figure**: None
