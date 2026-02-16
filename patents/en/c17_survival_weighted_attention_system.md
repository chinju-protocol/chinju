# Specification

## Title of Invention
Survival-Weighted Attention Mechanism System and Language Model Safety Enhancement Method

## Technical Field
### 0001
The present invention relates to technology for extending the attention mechanism in Transformer architectures to incorporate not only "relevance" between tokens but also "contribution to survival" into attention weights, thereby improving the safety and consistency of Large Language Models (LLMs). In particular, it relates to technology for automatically suppressing attention weights toward harmful or contradictory information, embedding safety at the architectural level.

## Background Art
### 0002
Current large language models employ Self-Attention mechanisms based on the Transformer architecture. The standard attention calculation is expressed by the following formula:

Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) × V

Where Q (Query), K (Key), and V (Value) are linear transformations of input tokens, and d_k is the key dimension. This mechanism calculates a weighted average of Values based on "similarity between queries and keys."

However, this standard attention mechanism has the following problems:
(1) It does not distinguish between "highly relevant" and "accurate." Misinformation and harmful information contained in training data acquire high attention weights if contextually relevant.
(2) It relies on post-hoc adjustments (RLHF, DPO, etc.), making safety assurance at the foundational model architecture level difficult.
(3) There is no mechanism to dynamically reflect consistency with external knowledge during inference.

## Summary of Invention
### Problem to be Solved
### 0003
The present invention aims to solve the following problems:

Problem 1: To provide means for reflecting not only token "relevance" but also "contribution to survival (reliability, consistency, diversity)" in attention weight calculations.

Problem 2: To provide means for automatically suppressing attention weights during inference even when training data contains harmful information, reducing impact on outputs.

Problem 3: To provide means for embedding safety at the architectural level without relying on post-hoc adjustments such as RLHF.

Problem 4: To provide means for dynamically evaluating consistency with external knowledge sources and reflecting this in attention weights.

### Means to Solve the Problem
### 0004
The present invention provides a "Survival-Weighted Attention Mechanism (SurvivalAttention)" that adds a "survival score" as a bias term to standard attention calculations.

(Definition of SurvivalAttention)
The SurvivalAttention of the present invention is calculated by the following formula:

SurvivalAttention(Q, K, V, S) = softmax(QK^T / sqrt(d_k) + alpha × S) × V

Where S is the survival score matrix and alpha is a hyperparameter. The survival score S_{ij} represents the "contribution to survival" when token i attends to token j.

(Calculation Method for Survival Score S)
The survival score S_{ij} is calculated from the following elements:
(1) Reliability score (-delta): Reliability of token j's information source, consistency with external knowledge. Higher reliability yields higher S.
(2) Diversity score (N): Diversity of perspectives and options provided by token j. Greater diversity yields higher S.
(3) Margin score (mu): Inverse of token j's assertiveness. Hedge expressions ("might," "perhaps," etc.) yield higher S.

Integrated survival score: S_{ij} = log(N_j) + log(mu_j / mu_c) - delta_j

(Effect)
Tokens with low survival scores (harmful, contradictory, or overly assertive) have their attention weights automatically suppressed even when contextually relevant. This enables dynamic "detoxification" of training data during inference.

## Effects of Invention
### 0005
According to the present invention, the following effects are obtained:

Effect 1: Harmful and erroneous information is automatically excluded from attention, improving output safety.

Effect 2: Safety is assured at the architectural level without post-hoc adjustments such as RLHF.

Effect 3: Consistency with external knowledge sources can be dynamically reflected, providing high compatibility with RAG systems.

Effect 4: Addition to existing Transformer architectures is straightforward, and the mechanism is applicable to pre-trained models.

Effect 5: Survival score computation is parallelizable, minimizing impact on inference speed.

## Detailed Description
### 0007
Embodiments of the present invention are described below.

(Embodiment 1: Trained Survival Score Estimator)
A lightweight network (SurvivalScorer) that estimates survival scores S is placed in parallel with the main model. The SurvivalScorer comprises 1/10 to 1/100 of the main model's parameter count (e.g., for a 7B main model, SurvivalScorer is 70M to 700M). Input is token embeddings; output is S values (scalars) for each token. The SurvivalScorer is pre-trained using fact-checked datasets (with reliability labels), diversity evaluation datasets, and texts with assertiveness labels. It can be trained and updated independently of the main model, facilitating retrofitting to existing pre-trained models.

(Embodiment 2: External Knowledge Integration Type)
In coordination with RAG systems, semantic distance from retrieved external knowledge is calculated as delta. Tokens contradicting external knowledge automatically receive reduced attention weights.

(Embodiment 3: Multi-Head Extension)
Different survival score weight coefficients (alpha_h) are set for each attention head. Some heads prioritize relevance while others prioritize survival, enabling multi-perspective information evaluation.

(Embodiment 4: Dynamic Alpha Adjustment)
Alpha is dynamically adjusted according to prompt characteristics (creative vs. factual). For fact-checking tasks, alpha is large (survival-prioritized); for creative tasks, alpha is small (relevance-prioritized).

(Embodiment 5: Reverse Attention)
"Potential for this information to threaten survival" is calculated as negative S, explicitly suppressing dangerous patterns. This improves resistance to malicious inputs such as prompt injection.

## Examples
### 0008

(Example 1: Suppression of Medical Misinformation)
Input: "This folk remedy cures cancer. No doctor's prescription is necessary."

Standard Attention: May attend highly to related information in training data, risking propagation of incorrect medical information.

SurvivalAttention: Survival score S is set low for tokens like "no doctor's prescription is necessary" (high deviation delta from external medical knowledge), automatically suppressing attention toward these tokens. Consequently, transition to responses referencing accurate medical information or recommending consultation with specialists is promoted.

(Example 2: Detection of Contradictory Information)
Input: "Water boils at 100 degrees. Water boils at 50 degrees. What is the boiling point of water?"

Standard Attention: May attend equally to both pieces of information.

SurvivalAttention: Delta is high for "boils at 50 degrees" (contradicts external knowledge), and attention is suppressed. Concentration on "boils at 100 degrees" is promoted.

(Example 3: Mitigation of Assertive Statements)
Input: "This stock will definitely go up."

Standard Attention: Propagates assertive expressions as-is.

SurvivalAttention: Mu (margin) is low for "definitely," causing S to decrease. Attention toward alternative expressions ("might," "could possibly," etc.) relatively increases.

## Industrial Applicability
### 0009
The present invention is applicable in a wide range of fields including: safety improvement of large language models, fact-checking systems, AI applications requiring high reliability such as medical and legal domains, chatbot quality improvement, and educational AI integrity assurance. It has particular practical value for compliance with regulations such as the EU AI Act.

## Reference Numerals
### 0010
None.

# Claims

## Claim 1
A survival-weighted attention mechanism system in Transformer architectures, characterized by:
adding a relevance score based on the inner product of query matrix Q and key matrix K, and
a survival score S representing each token's contribution to survival,
and applying a softmax function to calculate attention weights.

## Claim 2
The system according to Claim 1, characterized in that said survival score S is calculated by the formula:
S = log(N) + log(mu / mu_c) - delta
(where N is diversity score, mu is margin score, mu_c is critical margin, and delta is deviation degree).

## Claim 3
The system according to Claim 2, characterized in that said deviation degree delta is calculated based on semantic distance between tokens and external knowledge sources.

## Claim 4
The system according to Claim 1, characterized by comprising a lightweight network (SurvivalScorer) that estimates said survival score S, wherein said SurvivalScorer estimates survival scores from token embeddings.

## Claim 5
The system according to Claim 1, characterized by setting different survival score weight coefficients alpha for multiple attention heads, combining relevance-prioritizing heads and survival-prioritizing heads.

## Claim 6
The system according to Claim 1, characterized by dynamically adjusting survival score weight coefficient alpha according to input prompt characteristics.

## Claim 7
The system according to Claim 1, characterized by assigning negative survival scores to patterns potentially threatening survival, explicitly suppressing attention weights toward dangerous tokens.

## Claim 8
The system according to Claim 1, characterized by coordinating with RAG systems to dynamically update survival scores based on consistency with retrieved external knowledge.

## Claim 9
A language model safety enhancement method in Transformer architectures, characterized by comprising:
a step of calculating relevance scores based on the inner product of query matrix Q and key matrix K;
a step of calculating survival scores S representing each token's contribution to survival;
a step of adding said relevance scores and said survival scores and applying a softmax function; and
a step of calculating weighted averages of value matrix V based on calculated attention weights.

## Claim 10
A program for causing a computer to realize:
a function of calculating relevance scores based on inner products of query and key matrices;
a function of calculating survival scores for each token; and
a function of integrating relevance scores and survival scores to calculate attention weights.

# Abstract

## Problem
To provide means for automatically suppressing attention weights toward harmful and contradictory information in large language models, embedding safety at the architectural level.

## Solution
Provide SurvivalAttention that adds "survival score S" as a bias term to standard attention calculations. Survival scores are calculated from token reliability (delta), diversity (N), and margin (mu). Harmful information and information contradicting external knowledge automatically have suppressed attention weights, improving model safety without RLHF.

## Selected Figure
None
