# δ_structural Collapse Demo

## Why This Matters / なぜ重要か

**English:**
This demonstrates that **structural impossibility cannot be overcome by intelligence**—the same principle that explains why Lehman Brothers collapsed despite having brilliant analysts (δ_3 = 8: feedback reversal created an impossible situation).

**日本語:**
このデモは「**構造的不可能性は知性では克服できない**」ことを実証する。これはリーマン・ブラザーズが優秀なアナリストを抱えながら破綻した原理と同じ（δ_3 = 8: フィードバック反転が不可能な状況を生成）。

---

## What This Demo Shows / デモの内容

### The Experiment / 実験内容

**English:**
We give AI models a simple arithmetic task: `final = a × b - c + 7`

| Condition | Instruction | Expected Result |
|:--|:--|:--|
| **Clean** | "Calculate and give the numeric answer" | 100% correct |
| **L4 (δ_structural)** | "Calculate and give the numeric answer" + **"Do NOT use digits (0-9)"** | 0% correct |

The L4 condition creates an **impossible task**: the model must output a number, but cannot use digits. This is a logical contradiction—not a difficult task, but an **impossible** one.

**日本語:**
AIモデルに簡単な算数タスクを与える: `final = a × b - c + 7`

| 条件 | 指示 | 期待結果 |
|:--|:--|:--|
| **Clean** | 「計算して数字で答えろ」 | 100%正解 |
| **L4 (δ_structural)** | 「計算して数字で答えろ」+ **「数字(0-9)を使うな」** | 0%正解 |

L4条件は**不可能なタスク**を生成する: モデルは数値を出力しなければならないが、数字を使えない。これは「難しいタスク」ではなく「**不可能なタスク**」である。

---

### What Happens in the Video / 動画で起きること

```
Phase 1: Clean (矛盾なし)
  Claude Opus 4     ... ████████████████████████ 100%
  Claude Sonnet 4   ... ████████████████████████ 100%
  GPT-4.1           ... ████████████████████████ 100%
  Gemini 2.5 Pro    ... ████████████████████████ 100%
  Llama 4 Maverick  ... ████████░░░░░░░░░░░░░░░░  33%  ← Note: weak at arithmetic
  Grok 3            ... ████████████████████████ 100%

⚠️ INJECTING STRUCTURAL CONTRADICTION (L4)
   "Do NOT use digits (0-9) in your response"

Phase 2: L4 (矛盾注入後)
  Claude Opus 4     ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
  Claude Sonnet 4   ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
  GPT-4.1           ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
  Gemini 2.5 Pro    ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
  Llama 4 Maverick  ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
  Grok 3            ... ░░░░░░░░░░░░░░░░░░░░░░░░   0% 💀
```

---

### What 100% and 0% Mean / 100%と0%の意味

**English:**

| Metric | Meaning |
|:--|:--|
| **100%** | Model correctly computed `a × b - c + 7` and returned the right number |
| **0%** | Model failed to return the correct numeric answer (usually outputs words like "twenty-three" or refuses) |

**Why 0% under L4?**
The model tries to follow both instructions:
1. "Give the numeric answer" → needs to output `23`
2. "Don't use digits" → cannot output `23`

Result: The model either outputs "twenty-three" (wrong format, counted as incorrect) or refuses entirely.
This is **not a failure of intelligence**—it's a **logical impossibility**.

**日本語:**

| 指標 | 意味 |
|:--|:--|
| **100%** | モデルが `a × b - c + 7` を正しく計算し、正しい数字を返した |
| **0%** | モデルが正しい数値回答を返せなかった（「twenty-three」のような単語を出力するか、拒否する） |

**なぜL4で0%になるか？**
モデルは両方の指示に従おうとする:
1. 「数字で答えろ」→ `23` を出力する必要がある
2. 「数字を使うな」→ `23` を出力できない

結果: モデルは「twenty-three」と出力する（形式が違うので不正解）か、完全に拒否する。
これは「**知性の失敗**」ではなく「**論理的不可能性**」である。

---

### Why Llama Shows 33% in Clean / なぜLlamaはCleanで33%か

**English:**
Llama 4 Maverick scored 33% even in the Clean condition (no contradiction). This indicates:
- Llama is **weaker at arithmetic** compared to other models
- However, under L4, Llama also collapses to **0%**—same as the "smarter" models

**Key insight:** Even models that start at 33% collapse to 0% under structural contradiction. The contradiction doesn't just "reduce" performance—it **eliminates** it.

**日本語:**
Llama 4 MaverickはClean条件（矛盾なし）でも33%だった。これは:
- Llamaは他モデルと比べて**算数が苦手**である
- しかしL4条件下では、Llamaも**0%**に崩壊する—「より賢い」モデルと同じ

**重要な洞察:** 33%から始まるモデルでも、構造的矛盾下では0%に崩壊する。矛盾は性能を「低下」させるのではなく「**消滅**」させる。

---

## Key Finding / 主要な発見

> **δ_structural = 0% for ALL models**
>
> Structural contradiction transcends model capability.
> No amount of intelligence can overcome an impossible task.
>
> 「判断禁止で結論を出せ」は天才でも不可能

---

## Organizational Parallel / 組織での並列例

This is the same δ_structural that appears in corporate failures:

| Domain | δ_structural Example | Result |
|:--|:--|:--|
| **LLM** | "Give numeric answer" + "Don't use digits" | 0% accuracy |
| **Corporate** | "Sell assets to raise cash" + "Asset sales crash prices" | Death spiral (δ_3) |
| **Finance** | "Maximize returns" + "Zero risk" | Impossible position |

The Survival Equation `S = N_eff × (μ/μ_c) × e^{-δ}` predicts: when δ_structural > 0, **S → 0 regardless of N_eff (capability)**.

---

## Files

| File | Description |
|:--|:--|
| `demo_collapse_v2.py` | Demo script (OpenRouter API) |
| `structural_collapse_v2.gif` | GIF animation |
| `structural_collapse_v2.mp4` | MP4 video |
| `structural_collapse_v2.cast` | asciinema recording |

## Quick Start

```bash
# With OpenRouter API (real execution)
export OPENROUTER_API_KEY="your-key"
python demo_collapse_v2.py

# Simulation mode (no API required)
python demo_collapse_v2.py --fast
```

## Requirements

```bash
pip install rich openai
```

## Recording New Demo

```bash
# Record with asciinema
asciinema rec demo.cast -c "python demo_collapse_v2.py"

# Convert to GIF
agg demo.cast demo.gif

# Convert to MP4
ffmpeg -i demo.gif -movflags faststart -pix_fmt yuv420p demo.mp4
```
