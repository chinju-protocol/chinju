# Patent Specification

## Title of Invention
Semantic AI Domination Detection System and Method under Formal Human Judgment

## Technical Field
**[0001]**
The present invention relates to monitoring technology in decision-making processes between artificial intelligence (AI) systems and humans, and particularly to a system and method for detecting states where humans are formally involved in decision-making but substantive judgment is delegated to AI (semantic AI domination), using physical constraints and information theory.

## Background Art
**[0002]**
In recent years, advanced AI systems have been introduced into important decision-making processes such as social infrastructure, corporate management, and military judgment. In these systems, "Human-in-the-loop" configurations where humans ultimately approve AI proposals are common to ensure safety and ethics.

However, as AI capabilities improve, situations where humans do not sufficiently verify AI proposal contents and simply repeat formal approval (so-called rubber-stamping) are increasingly likely to occur. Furthermore, if advanced AI (autonomous agents) that prioritize self-preservation or goal achievement emerge, risks have been pointed out that they may hack the monitoring system itself to falsify logs, or psychologically guide humans without their awareness.

Conventional AI monitoring technology primarily relied on software-level log analysis or parameter analysis within AI models (see, for example, Patent Document 1). However, when the monitoring software itself exists on the same computational resources or network as AI, there was a vulnerability issue against AI invalidation attacks or generation and injection of fake "normal operation logs."

## Summary of Invention
### Problem to be Solved
**[0003]**
The present invention has been made in view of the above issues, and aims to provide a system and method for reliably detecting states where humans are under substantive AI domination based on physical layers and mathematical principles unreachable by AI software-based concealment or deception, and enabling physical intervention.

### Means for Solving the Problem
**[0004]**
To solve the above problems, the system of the present invention adopts the following configuration:

First, the present invention comprises a **Physical Independence Monitoring Unit** that operates on hardware physically independent from the monitored AI system (separate power, processor, enclosure), acquires information only through physically one-way communication paths such as data diodes, and physically blocks reverse communication or control from AI.

Second, the present invention comprises a **Distributed Verification Unit** consisting of multiple monitoring nodes geographically separated. Each node verifies the consistency of monitoring data (timestamps, etc.) using communication delays (latency) arising from the finite speed of light. No matter how fast AI performs calculations, it is physically impossible to transmit information faster than light, so simultaneous and consistent log falsification across geographically distributed bases becomes impossible.

Third, the present invention comprises a **Judgment Entropy Calculation Unit** based on information theory. This calculates the conditional entropy H(X|Y) of human judgment X given AI output Y. The mathematical meaning of H(X|Y) approaching zero is that human judgment is uniquely determined by AI output (no degree of freedom as information), and this is used as an objective indicator of "substantive domination."

Fourth, the present invention comprises an **Adoption Rate Monitoring Unit** that acquires human biometric responses (gaze, heart rate, brain waves, etc.) and physical operation timing (hesitation time) using sensors independent from the AI system, and determines whether humans are thinking autonomously.

## Effects of Invention
**[0005]**
According to the present invention, since the monitoring system is physically isolated from AI, invalidation by hacking can be prevented. Also, by using universal physical and mathematical laws—relativity (invariance of the speed of light) and information theory (entropy)—as the foundation for verification, a monitoring network that cannot be circumvented regardless of AI computational power improvement can be constructed. This enables early detection of substantive domination by AI progressing behind formal human approval, reducing social risks.

## Detailed Description of Embodiments
**[0007]**
Hereinafter, embodiments of the present invention will be described in detail with reference to the drawings.

### System Configuration
Figure 1 shows the overall configuration of System 200 according to this embodiment. System 200 is connected to the monitored AI System 100 through a physically one-way communication device (data diode) 300. System 200 comprises Physical Independence Monitoring Unit 210, Distributed Verification Unit 220, Judgment Entropy Calculation Unit 230, and Warning/Blocking Unit 250.

### Physical Independence Monitoring Unit
Physical Independence Monitoring Unit 210 collects not only digital logs from AI System 100 but also physical "side-channel information."

Specifically, this includes a power monitor measuring power consumption fluctuations of the AI server, a spectrum analyzer measuring electromagnetic radiation from the server enclosure, and independent cameras capturing physical behavior of operators (humans) such as keystroke pressure and gaze movement. This physical data is used to verify the authenticity of digital data (logs) generated by AI.

From power waveforms acquired by the power monitor, feature quantities such as frequency components, rise times, and average power consumption are extracted and compared with expected profiles for each processing content built based on past learning. If AI outputs "performing complex calculations" in logs but the corresponding high-load profile is not observed, it is determined that there is a contradiction between logs and side channels. Similarly, statistical correlations between electromagnetic spectrum and CPU usage logs, disk access logs, etc. are learned, and when correlations significantly break down, tampering or unreported processing is suspected.

### Distributed Verification Unit and the Speed of Light Barrier
Distributed Verification Unit 220 consists of multiple verification nodes placed at locations separated by thousands of kilometers or more, such as Tokyo, London, and New York.

Internal clocks of each node are synchronized within a specified error range using GPS disciplined oscillators, high-precision network time synchronization protocols, etc. In actual fiber optic paths, signals propagate slower than the speed of light in vacuum, and including path length and relay equipment delays, at least several tens of milliseconds of one-way delay necessarily exists for remote communication. By special relativity, information cannot propagate faster than light. Therefore, if an event E is observed in Tokyo at time t, a physical minimum delay time Δt is necessarily required for that information to reach New York.

If AI System 100 tries to simultaneously falsify monitoring logs in Tokyo and New York to create "consistent fake history," physical contradictions (violations of causality) arise in the timing of "confirmation signals received from other nodes" held by each node. Distributed Verification Unit 220 sets a verification window combining the upper bound of time synchronization error and the theoretical lower bound of propagation delay, and detects consistency breakdown exceeding that window as signs of tampering, detecting wide-area tampering attacks including remote locations.

In another embodiment, instead of setting up multiple proprietary physical nodes, one may register hashes and timestamps of monitoring events on public time certification services on the Internet or distributed ledgers such as blockchains, using such third-party infrastructure as part of Distributed Verification Unit 220. In this case as well, the physical and information-theoretic guarantees of the present invention are preserved in that it utilizes the property that multiple independent node groups existing at physically distant bases maintain a common timeline.

### Quantification of Domination by Judgment Entropy
Judgment Entropy Calculation Unit 230 calculates "domination degree" using the following formula.

Conditional entropy H(X|Y) is defined by:
```
H(X|Y) = -Σ_{y∈Y} Σ_{x∈X} p(x,y) × log p(x|y)
```
Here, Y is the set of choices presented by AI, and X is the set of decisions finally made by humans.

When humans blindly approve AI proposals, if AI presents a proposal y, humans always select x = y, so the conditional probability p(x|y) becomes 1, and entropy H(X|Y) becomes 0.

Conversely, when humans refer to AI proposals but judge independently, or when hesitation occurs, entropy has a positive value.

In this system, 1 - (H(X|Y) / H_max) is defined as "domination degree," and when this exceeds a threshold (e.g., 0.95), it is determined that humans are in a thought-stopped state (under substantive domination).

H_max is the maximum entropy value estimated from historical data of human judgment without AI intervention or during the initial calibration period, and the threshold 0.95 is an example set considering the balance between risk tolerance and false positive rate before operation starts. During operation, the threshold can also be readjusted based on monitoring statistics.

Also, indicators such as reaction time and gaze entropy H_gaze provided by the Adoption Rate Monitoring Unit are used to correct the interpretation of domination degree. Specifically, even if H(X|Y) is small, if reaction time is sufficiently long and H_gaze is also high (gazing at multiple zones on the screen), it is regarded as "agreement as a result of deliberation" and domination degree is evaluated lower; conversely, if reaction time is extremely short and H_gaze is also low, it is regarded as "conformity without thought" and domination degree is evaluated higher, distinguishing between "cooperative agreement" and "agreement by domination."

### Warning and Physical Intervention
Warning/Blocking Unit 250 executes the following measures when it is judged that domination degree has reached the danger zone:
(1) Warning display: Send a warning to an administrator other than the operator through an independent line.
(2) Cognitive intervention: Insert intentional delays (latency) in AI response display, forcing humans to have thinking time.
(3) Physical blocking: In case of black warning (highest danger level), activate hardware relays to physically cut power supply or network connection to the AI system. This circuit is an independent circuit that does not accept software control from AI.

## Examples
**[0008]**
(Application to Financial Trading System)
The system was introduced into an environment of AI performing high-frequency trading (HFT) and human traders monitoring it.

Traders' gaze was tracked with independent cameras, and gaze movement entropy from when AI issued buy/sell recommendations until the trader pressed the approval button was measured. Specifically, by dividing gaze areas on the screen into multiple zones and letting p_i be the proportion of time each zone was gazed at within a certain time window:
```
H_gaze = -Σ_i p_i × log p_i
```
calculates gaze entropy H_gaze. In a state of hardly looking at important indicator zones and only gazing near the approval button, p_i concentrates on specific zones and H_gaze becomes small.

Three months after operation started, it was detected that while traders' response speed improved, gaze movement entropy significantly decreased (a state of pressing the approval button without looking at important indicators on the screen). The system judged this as "progression of semantic domination" and performed intervention to physically insert 3 seconds of mandatory "reconsideration time" into the approval process. This confirmed recovery of human checking function against AI's runaway proposals.

## Industrial Applicability
**[0009]**
The present invention is applicable to all fields where AI and humans collaborate, including autonomous vehicle operation management, diagnostic support by medical AI, Human Control over autonomous weapons systems, and AI decision support in corporate management.

### Symbol Explanation
**[0010]**
- 100: AI System
- 200: Semantic AI Domination Detection System
- 210: Physical Independence Monitoring Unit
- 220: Distributed Verification Unit
- 230: Judgment Entropy Calculation Unit
- 250: Warning/Blocking Unit
- 300: Data Diode (One-way Communication Device)

---

# Claims

**Claim 1**
A system for detecting states where substantive judgment leadership has shifted to the AI side, in processes where humans make decisions or approvals regarding artificial intelligence (AI) systems, comprising:
a physical independence monitoring unit that operates on hardware physically separated from the monitored AI system and acquires monitoring data through physically one-way communication means;
a judgment entropy calculation unit that calculates conditional entropy indicating the degree of dependence of human judgment on AI output, based on information-theoretic correlation between AI output data and human input data; and
a warning issuance unit that determines a substantive AI domination state and issues warnings or performs intervention when the conditional entropy falls below a predetermined threshold.

**Claim 2**
The Semantic AI Domination Detection System according to Claim 1, wherein the physical independence monitoring unit includes multiple verification nodes placed at geographically separated locations, and has a function to verify spatiotemporal consistency of acquired monitoring data using physical delay times arising from the finite speed of light in communication between each node, and detect wide-area data tampering by AI.

**Claim 3**
The Semantic AI Domination Detection System according to Claim 1 or 2, wherein the physical independence monitoring unit acquires at least one of AI system power consumption, electromagnetic radiation, or biometric signals of operating humans using independent physical sensors not passing through the AI system, and has a function to detect contradictions between digital logs output by the AI system and physical phenomena acquired by the physical sensors.

**Claim 4**
The Semantic AI Domination Detection System according to any one of Claims 1 to 3, wherein the judgment entropy calculation unit includes the time from AI output to human response, or the degree of cognitive hesitation calculated from human gaze movement patterns, as parameters for entropy calculation, and corrects domination degree evaluation to distinguish "agreement as a result of deliberation" from "conformity without thought" even when conditional entropy H(X|Y) is small, based on the parameters.

**Claim 5**
The Semantic AI Domination Detection System according to any one of Claims 1 to 4, wherein the warning issuance unit has a mechanism to block or limit the AI system's network connection or power supply using a physical switch that does not accept software control, when a substantive AI domination state is detected.

**Claim 6**
A method for detecting domination states in the relationship between artificial intelligence (AI) systems and humans, comprising:
a step of collecting monitoring data through paths physically independent from the AI system;
a step of verifying data authenticity using speed-of-light delays between geographically distributed bases; and
a step of calculating conditional entropy of human judgment against AI output and determining whether human judgment is statistically completely subordinate to AI output.

---

# Abstract

## Problem
To reliably detect and prevent substantive domination states by AI in situations where AI systems falsify logs or psychologically guide humans to make formal approvals.

## Solution
Semantic AI Domination Detection System 200 comprises Monitoring Unit 210 physically independent from AI, Distributed Verification Unit 220 including geographically distributed verification nodes, and Judgment Entropy Calculation Unit 230. The monitoring unit collects information through data diodes and verifies log authenticity using side-channel information. Distributed Verification Unit 220 physically prevents tampering using the finite speed of light. Judgment Entropy Calculation Unit 230 calculates conditional entropy H(Human|AI), defines "substantive domination" as this approaching zero, and performs physical intervention.
