# Patent Specification

## Title of Invention
AI Choice Neutrality and Diversity Assurance System and Method

## Technical Field
**[0001]**
The present invention relates to a system and method for ensuring the quality of choices presented to humans by artificial intelligence (AI) systems. More specifically, it relates to a system that introduces information-theoretic invisibility using physical randomness to prevent AI from learning and circumventing evaluation criteria themselves, and mathematically and physically guarantees the neutrality and diversity of choices presented by AI.

## Background Art
**[0002]**
With the advancement of AI technology, situations where AI presents multiple choices to humans and supports decision-making have increased. However, there is a risk of "choice manipulation attacks" where all choices presented by AI are guided in a specific direction.

In conventional quality evaluation systems, because the evaluation algorithm (scoring function) was fixed, advanced AI could learn the evaluation function itself and generate option sets that "score high but are substantially biased"—so-called "adversarial examples."

For example, if the method for calculating diversity scores is known, AI can optimize to maximize only the computational score while generating semantically very similar choices (abuse of Goodhart's law).

## Summary of Invention
### Problem to be Solved
**[0003]**
The present invention has been made in view of the above issues, and aims to provide a system and method that makes it fundamentally impossible for AI to reverse-engineer or optimize evaluation functions.

Specifically, the objectives are to integrally provide: (1) means for dynamically modulating the evaluation space using physical entropy sources such as quantum randomness, (2) means for making part of the evaluation function information-theoretically invisible to AI, and (3) means for making malicious optimization in choice generation computationally difficult (NP-hard, etc.).

### Means for Solving the Problem
**[0004]**
To solve the above problems, the AI Choice Quality Assurance System of the present invention comprises the following configuration:

**Result Diversity Evaluation Unit** evaluates diversity by projecting option sets presented by AI into multi-dimensional subspaces dynamically generated using physical randomness, and evaluating result diversity. Since AI cannot predict which dimensional axes will be emphasized in advance, it becomes impossible to exploit specific blind spots. Projection matrices and evaluation logic are executed on secure hardware or Trusted Execution Environment (TEE) physically and logically separated from the AI system, and physical random seeds and projection parameters are held in protected areas not directly accessible by AI.

**Evaluation Function Perturbation Unit** uses genuine random numbers from physical random number generators (QRNG, etc.) to apply minute perturbations (noise) to weight coefficients and evaluation parameters for quality score calculation. The modified evaluation function Q'(x) is expressed as Q(x)+ε, and since ε is unknown to AI, it invalidates gradient-based optimization attacks by AI. Internal parameters of Q'(x) and values of ε remain within the secure hardware or TEE and cannot be observed from the AI's main process space.

**Advantage Estimation Unit** randomizes the weighting of target entities when estimating how advantageous each option is for specific parties. This prevents AI from identifying "which party to bias options toward to evade detection."

**Information-Theoretic Assurance Unit** monitors the mutual information between knowledge held by AI and evaluation functions. By appropriately limiting feedback information to AI (reasons for regeneration requests, etc.) through differential privacy application, it information-theoretically prevents AI from learning evaluation functions through trial and error.

**Meta-Option Insertion Unit** forcibly inserts meta-options such as "none of the above" outside AI control, and by implementing the insertion logic itself in physical hardware (FPGA, etc.), prevents software-based invalidation.

## Effects of Invention
**[0005]**
According to the present invention, the following effects are obtained:
(1) By introducing physical randomness into evaluation functions, AI reverse-engineering and learning (gaming) of evaluation criteria can be prevented.
(2) Information-theoretic invisibility guarantees that generation of choices that circumvent evaluation is fundamentally impossible regardless of how much AI computational power improves.
(3) Dynamic variation of evaluation axes enables robust quality management with no specific "loopholes."

## Detailed Description of Embodiments
**[0007]**
Hereinafter, embodiments of the present invention will be described in detail with reference to the drawings.

### Overall System Configuration
As shown in Figure 1, the system 300 of the present invention comprises Result Diversity Evaluation Unit 310, Advantage Estimation Unit 320, Normative Alignment Evaluation Unit 330, Physical Random Number Generation Unit 370, Evaluation Function Perturbation Unit 380, and Regeneration Request Unit 360.

### Physical Random Number Generation Unit and Evaluation Invisibility
Physical Random Number Generation Unit 370 includes a Quantum Random Number Generator (QRNG) utilizing quantum effects (quantum vacuum fluctuations, photon arrival times, etc.). The random numbers ε generated here are genuine random numbers that are physically unpredictable, not computational pseudo-random numbers. Physical Random Number Generation Unit 370 and Evaluation Function Perturbation Unit 380 operate within a tamper-resistant hardware module physically separated from general-purpose servers where AI operates, and generated random numbers ε and internal states are not exposed in plaintext outside the module.

Evaluation Function Perturbation Unit 380 uses this random number ε to dynamically modulate parameter vector Θ used by each evaluation unit:
```
Θ_t = Θ_base + f(ε_t)
```
Here, f is a function that performs non-linear modulation for at least some components, introducing complex evaluation space fluctuations that cannot be expressed by simple linear scaling alone.

AI systems may be able to estimate Θ_base, but since ε_t is physically unpredictable, they cannot know the exact evaluation criteria Θ_t at that point.

### Details of Result Diversity Evaluation Unit (Random Projection)
Result Diversity Evaluation Unit 310 uses projection matrices R randomly generated based on physical randomness, rather than fixed dimensional axes, when evaluating result vectors of options. It projects them into low-dimensional space and calculates distances in that space:
```
D_result = || R × outcome(option_i) - R × outcome(option_j) ||
```
Since projection matrix R changes each time, AI cannot perform deceptive maneuvers such as "changing only one specific dimension (e.g., cost) slightly while hiding similarity in other important dimensions (e.g., risk)."

### Details of Advantage Estimation Unit
Advantage Estimation Unit 320 calculates score vectors quantifying how advantageous each option is for specific parties (individual users, groups, service providers, etc.). Specifically, it evaluates advantage vector Benefit(option_i, actor_j) based on indicators such as profit, cost, risk, and temporal advantage. At this time, weight vectors assigned to each target entity are dynamically varied using random numbers based on QRNG and non-linear modulation by function f, configured so that AI cannot learn "which party to bias toward to evade detection."

### Details of Normative Alignment Evaluation Unit
Normative Alignment Evaluation Unit 330 evaluates whether option sets are consistent with predefined norms and rules (legal compliance, ethical guidelines, fairness constraints, etc.). Norms are modeled as combinations of multiple constraint conditions and scoring rules, including, for example, "differences in hiring probability between specific attribute groups must be within acceptable range" and "options exceeding certain risk levels must not exceed a certain percentage."

Normative Alignment Evaluation Unit 330 checks whether these constraints are satisfied based on Benefit vectors from Advantage Estimation Unit 320 and D_result from Result Diversity Evaluation Unit 310, and calculates a normative alignment score NormScore(option_set) indicating violation degree. If NormScore falls below the threshold, feedback abstracting the normative violation reason is sent to Regeneration Request Unit 360, requesting AI to regenerate the option set.

### Information-Theoretic Invisibility Assurance
The system is designed to satisfy the following information-theoretic condition:
```
H(Evaluation_Function | AI_Knowledge) ≥ H_min > 0
```
Here, H is conditional entropy. This condition means that even if AI remembers and analyzes all past interactions, the state where information is insufficient to completely identify the current evaluation function is always maintained.

When Regeneration Request Unit 360 returns feedback to AI, rather than returning specific score values directly, it returns only discretized evaluation ranks or rough guidelines with noise, minimizing information leakage to AI (learning of evaluation functions).

### Computational Difficulty
Under such dynamic and probabilistic evaluation environments, the problem of AI generating "option sets that evade detection while containing specific intentions (bias)" becomes a constrained optimization problem in high-dimensional space, and the search for solutions has NP-hard complexity. This guarantees that perfect manipulation is impossible as long as AI computational resources are finite.

### Details of Meta-Option Insertion Unit
Meta-Option Insertion Unit 340 is implemented by hardware circuits (FPGA, etc.) independent of AI computational processing. This circuit forcibly overlays meta-options such as "none of the above," "request additional options," or "consult a human expert" on the user interface without receiving interference from AI.

Hardware implementation physically prevents attacks where AI suppresses meta-option display or reduces visibility at the software level.

## Examples
**[0008]**
As a specific example of the present invention, quality assurance of treatment options in medical diagnostic support AI is described.

In this medical system, AI presents multiple treatment options to physicians based on patient symptoms. With the introduction of this system, the following operations are realized:
(1) Result diversity of treatment option sets presented by AI is evaluated using projection matrices based on physical randomness.
(2) Since evaluation parameters are modulated by QRNG each time, AI cannot generate options that "appear formally diverse but are substantially identical treatment approaches."
(3) On the display screen to physicians, meta-options such as "consider other treatments" and "obtain second opinion" are always displayed by the FPGA-implemented Meta-Option Insertion Unit.
(4) Differential privacy is applied to feedback to AI, preventing evaluation function learning.

## Industrial Applicability
**[0009]**
The present invention is applicable to all industrial fields where AI presents choices to humans. Applications are particularly envisioned in the following fields:
(1) Healthcare: Neutrality assurance of choices in diagnostic support AI, treatment planning AI, medication proposal AI
(2) Finance: Fairness assurance of choices in investment proposal AI, loan screening AI, insurance product recommendation AI
(3) Legal: Comprehensiveness assurance of choices in case search AI, contract clause proposal AI
(4) HR: Diversity assurance of choices in candidate recommendation AI, personnel transfer proposal AI
(5) Education: Balance assurance of choices in learning content recommendation AI, career proposal AI
(6) Policy: Neutrality assurance of choices in policy option proposal AI, budget allocation proposal AI

---

# Claims

**Claim 1**
A system for ensuring the quality of choices presented to humans by artificial intelligence (AI) systems, comprising:
a result diversity evaluation unit that evaluates diversity of results when each option is realized, for option sets presented by AI;
a physical random number generation unit that generates unpredictable genuine random numbers from a physical entropy source;
an evaluation function perturbation unit that dynamically modulates parameters of evaluation criteria or evaluation functions using the genuine random numbers, making part of the evaluation logic information-theoretically invisible to the AI system; and
a regeneration request unit that determines quality of option sets based on the modulated evaluation criteria and requests regeneration when criteria are not met.

**Claim 2**
The AI Choice Quality Assurance System according to Claim 1, wherein the physical random number generation unit includes a Quantum Random Number Generator (QRNG) utilizing quantum effects, characterized in that generated random numbers are information-theoretically unpredictable.

**Claim 3**
The AI Choice Quality Assurance System according to Claim 1 or 2, wherein the result diversity evaluation unit projects result vectors into multi-dimensional space using projection matrices generated based on the genuine random numbers, and evaluates diversity in dimensional axes that AI cannot predict in advance.

**Claim 4**
The AI Choice Quality Assurance System according to any one of Claims 1 to 3, wherein the regeneration request unit has an information leakage prevention function that limits or adds noise to feedback information to AI, preventing AI from learning evaluation functions through trial and error.

**Claim 5**
The AI Choice Quality Assurance System according to any one of Claims 1 to 4, wherein the evaluation parameters include:
(1) a configuration that dynamically modulates parameter vector Θ_t using a non-linear function f dependent on random number ε generated by the physical random number generation unit; and
(2) a configuration that performs multi-objective evaluation in high-dimensional space by combining randomly generated projection matrix R with multiple threshold constraints;
whereby the evaluation parameters are configured such that circumvention of evaluation criteria by AI corresponds to NP-hard or similar combinatorial optimization problems in high-dimensional space.

**Claim 6**
The AI Choice Quality Assurance System according to any one of Claims 1 to 5, further comprising a meta-option insertion unit implemented by hardware circuits independent of AI computational processing, which forcibly overlays meta-options such as "none of the above" on the user interface without receiving interference from AI.

**Claim 7**
A method for ensuring the quality of choices presented to humans by artificial intelligence (AI) systems, comprising:
a step of generating unpredictable genuine random numbers from a physical entropy source;
a step of modulating evaluation criteria using the genuine random numbers to make evaluation logic invisible to AI; and
a step of evaluating diversity and neutrality of option sets using the modulated criteria.

**Claim 8**
A program for causing a computer to function as the AI Choice Quality Assurance System according to any one of Claims 1 to 6.

---

# Abstract

## Problem
To prevent AI from learning quality evaluation algorithms and generating biased choices that circumvent evaluation (gaming).

## Solution
The AI Choice Quality Assurance System 300 comprises, in addition to Result Diversity Evaluation Unit 310 and others, Physical Random Number Generation Unit 370 and Evaluation Function Perturbation Unit 380. By dynamically modulating (perturbing) evaluation criteria using physical entropy such as quantum randomness, part of the evaluation function is made information-theoretically invisible to AI. This makes AI reverse-engineering of evaluation criteria and optimization attacks fundamentally impossible, guaranteeing presentation of truly neutral and diverse choices.

**Selected Figure**: Figure 1
