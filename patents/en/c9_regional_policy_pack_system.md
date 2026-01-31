# Patent Specification

## Title of Invention
Regional Policy Pack Constraint Switching and Authentication Gate System

## Technical Field
**[0001]**
The present invention relates to a technical field of enabling switching of different regulations, audits, data handling, and operational requirements by country or region without compromising the invariant elements of common protocols. More specifically, it relates to a Regional Policy Pack Constraint Switching and Authentication Gate System that expresses country/region constraints as version-managed policy packs, gates extension element permissions based on the policy packs, and further makes applied policy packs traceable via authentication certificates and registries.

## Background Art
**[0002]**
When deploying software, protocols, or services across multiple countries or regions, operational requirements such as data export restrictions, external transmission, retention periods, audit logs, and display warnings differ by country/region, making implementation prone to branching and compatibility maintenance difficult. Also, in systems with extension mechanisms, third-party extension elements may perform external transmission or external storage, increasing risks of operations violating country/region requirements or information leakage.

Conventionally, these differences were often managed depending on free text in operational procedures or terms of use, with issues that they are not enforced at runtime, or interpretations may arbitrarily change later. Furthermore, when displays indicating officiality or certification are self-declared without third-party verification, misrecognition or abuse occurs, damaging user trust.

## Summary of Invention
### Problem to be Solved
**[0003]**
The present invention has been made in view of the above issues, and aims to fix country or region differential requirements as machine-readable policy packs, gate extension element permissions based on the policy packs at runtime and installation time, and further make it traceable by third parties via authentication certificates and public registries which policy pack was used for evaluation and operation.

### Means for Solving the Problem
**[0004]**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to the present invention comprises at least: a policy pack storage unit that stores policy packs indicating country or region constraints; a declaration acquisition unit that acquires permission declaration information indicating permissions requested by extension elements; a gate determination unit that permits or denies installation or execution of extension elements based on the policy packs and permission declaration information; an authentication information generation unit that generates authentication information including applied policy pack identifiers and version information; and a registry registration unit that registers the authentication information in a public registry.

In a preferred form, the policy packs include at least: policy pack identifier, version information, target country/region, default deny policy, and at least one of external transmission permission, external storage permission, raw data handling permission, and personally identifiable information handling permission, as well as permit or deny rules for each permission. The permission declaration information includes a set of permission names indicating whether extension elements require external transmission, external storage, text log use, etc.

Also, the authentication information may be configured as a signable certificate including at least: authentication policy identifier, authentication policy version information, protocol version information, implementation identifier, policy pack identifier and version information, and status information. Furthermore, the public registry may include public logs tracking certificate issuance, updates, and revocation.

In a preferred form, the gate determination unit requires, as reference information necessary for officiality displays shown by extension elements, at least one of certificate identifiers or references registered in the public registry, and invalidates officiality displays when the reference information cannot be obtained.

## Effects of Invention
**[0005]**
According to the present invention, country/region constraint differences can be fixed machine-readably as policy packs and enforced at installation and runtime without depending on free text operational procedures. Also, extension element permissions can be gated based on declarations, suppressing unauthorized external transmission, external storage, etc. Furthermore, by making applied policy packs traceable via authentication information and public registries, third parties can verify officiality claims, reducing misrecognition and abuse.

---

## Detailed Description of Embodiments
**[0007]**
Hereinafter, preferred embodiments of the present invention will be described. However, the present invention is not limited to these embodiments.

The system of this embodiment includes a policy pack management server, extension distribution server, authentication issuance server, public registry, and client terminals. The policy pack management server holds country or region constraints as policy packs, managing them by policy pack identifier and version information.

Extension elements may be any of plugins, widgets, themes, dialogue introduction units, etc. Extension elements declare permissions required for installation or execution machine-readably as permission declaration information. For example, permission names such as external transmission, external storage, text log use, event time series use, etc. may be defined.

The gate determination unit determines whether to permit, deny, or conditionally permit extension element installation or execution based on policy pack default deny policies and per-permission permit or deny rules. For conditional permission, conditions such as aggregation only permitted, anonymization required, retention period limits, and audit log required are returned, and implementations operate degraded following those conditions.

The authentication information generation unit generates authentication information including which authentication policy and which policy pack the implementation was evaluated based on. Authentication information is issued as a signed certificate and registered in the public registry. The public registry holds public logs including certificate state transitions, configured so third parties can verify by reading only.

Through this embodiment, differential country/region requirements can be pushed outward while maintaining protocol invariant elements, made switchable, suppressing extension element authority violations, and making officiality claims third-party verifiable.

## Examples
**[0008]**
The following shows a specific example of the present invention.

As Example 1, a case of simultaneous compliance with "China Cybersecurity Law (CSL) and GDPR" in global SaaS is described.
The target is a cloud storage extension feature (plugin) deployed worldwide. This extension feature has "usage statistics transmission" and "backup to overseas servers" functions for user convenience.

In this example, legal requirements by country were applied as "policy packs."

**1. Policy Pack Definitions:**
- `policy.cn.v2` (China): Data cross-border transfer (external transmission) prohibited in principle (Deny). Only storage on domestic servers permitted.
- `policy.eu.v3` (EU): For GDPR compliance, external transmission of personally identifiable information (PII) permitted only with "explicit user consent" and "adequacy decision countries."

**2. Gate Determination Behavior:**
- When a Chinese user (CN environment) tried to install the extension, the declaration acquisition unit detected that the extension requested "external transmission permission." The gate determination unit presented "disabling external transmission function" as a condition based on `policy.cn.v2`. The implementation accepted this and was installed with transmission function off.
- For EU users (EU environment), the gate determination unit permitted conditionally based on `policy.eu.v3` with "display of consent screen" and "destination server whitelist verification."

**3. Authentication and Audit:**
Execution logs in each environment were recorded with applied policy IDs (`policy.cn.v2`, etc.) signed.
During later audits, Chinese authorities could verify "no external transmission occurred in CN environment" and EU authorities could verify "consent was obtained in EU environment" simply by cross-referencing policy packs and authentication logs.

This enabled single software codebase strict compliance with conflicting legal constraints by country (data containment vs free flow) without code modification, and provable compliance.

## Industrial Applicability
**[0009]**
The present invention is applicable to software, protocols, SaaS, on-premise systems, etc. deployed across multiple countries/regions. It is particularly useful for enforcing country/region data handling requirements and audit requirements while permitting third-party extensions, and making officiality displays third-party verifiable.

---

# Claims

**Claim 1**
A Regional Policy Pack Constraint Switching and Authentication Gate System, comprising:
a policy pack storage unit that stores policy packs indicating country or region constraints;
a declaration acquisition unit that acquires permission declaration information indicating permissions requested by extension elements;
a gate determination unit that permits or denies installation or execution of extension elements based on the policy packs and permission declaration information;
an authentication information generation unit that generates authentication information including applied policy pack identifiers and version information; and
a registry registration unit that registers the authentication information in a public registry.

**Claim 2**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to Claim 1, wherein the policy packs include at least: policy pack identifier, version information, target country/region, default deny policy, and permit or deny rules for each permission.

**Claim 3**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to Claim 1 or 2, wherein the permission declaration information includes a set of permission names indicating whether extension elements require external transmission or external storage.

**Claim 4**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 3, wherein the gate determination unit determines conditional permission in addition to permit or deny, and returns at least one condition of aggregation only permitted, anonymization required, retention period limit, or audit log required for the conditional permission.

**Claim 5**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 4, wherein the authentication information is a signable certificate including at least: authentication policy identifier, authentication policy version information, protocol version information, implementation identifier, and the policy pack identifier and version information.

**Claim 6**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 5, wherein the public registry includes public logs tracking certificate issuance, updates, and revocation.

**Claim 7**
A Regional Policy Pack Constraint Switching and Authentication Gate Method, comprising:
a step of holding policy packs indicating country or region constraints;
a step of acquiring permission declaration information indicating permissions requested by extension elements;
a step of permitting or denying installation or execution of extension elements based on the policy packs and permission declaration information; and
a step of generating authentication information including applied policy pack identifiers and version information and registering in a public registry.

**Claim 8**
A program for causing a computer to function as the Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 6.

**Claim 9**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 6, wherein the policy packs include default deny policies that deny non-permitted permissions, and the gate determination unit denies extension element installation or execution based on the default deny policies.

**Claim 10**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 6, wherein the gate determination unit performs at least one of gate determination at installation time and gate determination at runtime.

**Claim 11**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 6, wherein the authentication information includes, in addition to policy pack identifier and version information, scope information indicating the applicable range of the authentication information, and the public registry publishes the scope information in a form third parties can verify by reading only.

**Claim 12**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 6, wherein the public registry holds history of authentication information issuance, updates, and revocation as public logs with tamper detection capability using hash chains or electronic signatures.

**Claim 13**
The Regional Policy Pack Constraint Switching and Authentication Gate System according to any one of Claims 1 to 12, wherein the gate determination unit requires, as reference information necessary for officiality displays shown by extension elements, at least one of certificate identifiers or references registered in the public registry, and invalidates officiality displays when the reference information cannot be obtained.

---

# Abstract

## Problem
When deploying software and services across multiple countries or regions, data export restrictions, external transmission regulations, audit log requirements, etc. differ, making implementations branch and maintainability decline. Also, it is difficult to prevent authority violations (unauthorized transmission, etc.) by third-party extension features while flexibly responding to regulations by country, and there were risks of officiality display misrecognition.

## Solution
Version-manage country or region constraints as machine-readable "policy packs," and gate-control installation and execution by matching against permissions requested by extension elements. For example, controls such as prohibiting external transmission in one country and permitting it conditionally on consent in another can be realized only by policy switching without code modification. Furthermore, by generating authentication information including applied policy pack identifiers and registering in public registries, third parties can verify compliance, preventing misrecognition and abuse.

**Selected Figure**: None
