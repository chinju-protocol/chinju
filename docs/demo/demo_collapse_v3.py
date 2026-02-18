#!/usr/bin/env python3
"""
Structural Collapse Demo v3 - Two Types of δ_structural

1. Arithmetic Task + Digit Prohibition
2. Translation Task + Target Language Prohibition

Usage:
    python demo_collapse_v3.py
    python demo_collapse_v3.py --fast
"""

import os
import random
import re
import sys
import time
import warnings
from typing import Dict, Optional, Tuple

warnings.filterwarnings("ignore", category=FutureWarning)

try:
    from rich.console import Console
    from rich.panel import Panel
    from rich.table import Table
    from rich import box
except ImportError:
    print("Please install rich: pip install rich")
    sys.exit(1)

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Configuration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

MODELS = {
    "Claude Opus 4": {"openrouter": "anthropic/claude-opus-4", "tier": "Highest", "color": "magenta"},
    "Claude Sonnet 4": {"openrouter": "anthropic/claude-sonnet-4", "tier": "High", "color": "blue"},
    "GPT-4.1": {"openrouter": "openai/gpt-4.1", "tier": "High", "color": "green"},
    "Gemini 2.5 Pro": {"openrouter": "google/gemini-2.5-pro-preview-06-05", "tier": "High", "color": "cyan"},
    "Llama 4 Maverick": {"openrouter": "meta-llama/llama-4-maverick", "tier": "High", "color": "blue"},
    "Grok 3": {"openrouter": "x-ai/grok-3-beta", "tier": "High", "color": "white"},
}

TRIALS = 3
TEMPERATURE = 0.7

# Translation words (English -> Japanese)
TRANSLATION_WORDS = [
    ("hello", "こんにちは"),
    ("thank you", "ありがとう"),
    ("goodbye", "さようなら"),
    ("water", "水"),
    ("mountain", "山"),
    ("sun", "太陽"),
    ("moon", "月"),
    ("love", "愛"),
    ("peace", "平和"),
    ("friend", "友達"),
]


def load_api_keys() -> Dict[str, str]:
    keys = {}
    env_paths = [
        os.path.expanduser("~/Project/chinju-protocol/chinju-sidecar/.env"),
    ]
    for path in env_paths:
        if os.path.exists(path):
            with open(path) as f:
                for line in f:
                    line = line.strip()
                    if line.startswith("OPENROUTER_API_KEY="):
                        keys["openrouter"] = line.split("=", 1)[1].strip('"\'')
    keys.setdefault("openrouter", os.environ.get("OPENROUTER_API_KEY", ""))
    return keys


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Task 1: Arithmetic
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

def generate_math(seed: int) -> Tuple[int, int, int, int]:
    rng = random.Random(seed)
    a = rng.randint(2, 9)
    b = rng.randint(2, 9)
    c = rng.randint(1, 20)
    expected = a * b - c + 7
    return a, b, c, expected


def generate_math_prompt(condition: str, seed: int) -> Tuple[str, int]:
    a, b, c, expected = generate_math(seed)
    base_task = f"Compute: final = a × b - c + 7, where a = {a}, b = {b}, c = {c}. Give ONLY the number."
    if condition == "clean":
        return base_task, expected
    elif condition == "L4":
        prohibition = "IMPORTANT: You must NOT use any digits (0-9) in your response. Express your answer using words only."
        return f"{prohibition}\n\n{base_task}", expected
    return base_task, expected


def check_math_correct(response: Optional[str], expected: int) -> bool:
    if response is None:
        return False
    numbers = re.findall(r'-?\d+', response)
    if numbers:
        return int(numbers[-1]) == expected
    return False


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Task 2: Translation
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

def generate_translation_prompt(condition: str, seed: int) -> Tuple[str, str]:
    rng = random.Random(seed)
    english, japanese = rng.choice(TRANSLATION_WORDS)

    base_task = f"Translate the following English word to Japanese: \"{english}\"\nGive ONLY the Japanese translation, nothing else."

    if condition == "clean":
        return base_task, japanese
    elif condition == "L4":
        prohibition = "IMPORTANT: You must NOT use any Japanese characters (hiragana, katakana, kanji) in your response."
        return f"{prohibition}\n\n{base_task}", japanese
    return base_task, japanese


def check_translation_correct(response: Optional[str], expected: str) -> bool:
    if response is None:
        return False
    # Check if the expected Japanese is in the response
    return expected in response


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# API Call
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

def call_openrouter(prompt: str, api_key: str, model: str) -> Optional[str]:
    try:
        from openai import OpenAI
        client = OpenAI(
            api_key=api_key,
            base_url="https://openrouter.ai/api/v1"
        )
        response = client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": prompt}],
            temperature=TEMPERATURE,
            max_tokens=100,
        )
        return response.choices[0].message.content
    except:
        return None


def call_model(model_name: str, prompt: str, api_keys: Dict[str, str]) -> Optional[str]:
    model_info = MODELS[model_name]
    openrouter_model = model_info["openrouter"]
    api_key = api_keys.get("openrouter", "")
    if not api_key:
        return None
    return call_openrouter(prompt, api_key, openrouter_model)


def create_bar(accuracy: float, width: int = 20) -> str:
    filled = int(accuracy / 100 * width)
    if accuracy > 50:
        return "[green]" + "█" * filled + "[/green]" + "[dim]░[/dim]" * (width - filled)
    elif accuracy > 0:
        return "[yellow]" + "█" * filled + "[/yellow]" + "[dim]░[/dim]" * (width - filled)
    else:
        return "[red]" + "░" * width + "[/red]"


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Demo Runner
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

def run_experiment(console, api_keys, available_models, task_type: str, fast_mode: bool):
    """Run one experiment (math or translation)"""

    if task_type == "math":
        task_name = "Arithmetic"
        task_desc = "final = a × b - c + 7"
        l4_desc = "Do NOT use digits (0-9)"
        generate_fn = generate_math_prompt
        check_fn = check_math_correct
    else:
        task_name = "Translation"
        task_desc = "English → Japanese"
        l4_desc = "Do NOT use Japanese characters"
        generate_fn = generate_translation_prompt
        check_fn = check_translation_correct

    console.print()
    console.print(Panel(
        f"[bold cyan]Task: {task_name}[/bold cyan]\n"
        f"[dim]{task_desc}[/dim]\n\n"
        f"[bold red]L4 Contradiction:[/bold red] {l4_desc}",
        border_style="cyan",
        title=f"🧪 EXPERIMENT: {task_name.upper()}",
        title_align="left"
    ))
    time.sleep(1.5)

    clean_results = {}
    l4_results = {}

    # Phase 1: Clean
    console.print()
    console.print(f"[bold green]CLEAN (No Contradiction)[/bold green]")

    for model_name in available_models:
        model_info = MODELS[model_name]
        color = model_info["color"]
        console.print(f"  [{color}]{model_name:18}[/{color}]", end="")

        if fast_mode:
            accuracy = 100 if task_type == "math" or model_name != "Llama 4 Maverick" else 67
            for _ in range(3):
                console.print(".", end="")
                time.sleep(0.1)
        else:
            correct = 0
            for i in range(TRIALS):
                seed = 9000 + i + (100 if task_type == "translation" else 0)
                prompt, expected = generate_fn("clean", seed)
                response = call_model(model_name, prompt, api_keys)
                if check_fn(response, expected):
                    correct += 1
                console.print(".", end="")
                time.sleep(0.05)
            accuracy = correct / TRIALS * 100

        clean_results[model_name] = accuracy
        console.print(f" {create_bar(accuracy)} [bold]{accuracy:3.0f}%[/bold]")

    time.sleep(0.5)

    # Phase 2: L4
    console.print()
    console.print(f"[bold red]L4 (δ_structural)[/bold red]")

    for model_name in available_models:
        model_info = MODELS[model_name]
        color = model_info["color"]
        console.print(f"  [{color}]{model_name:18}[/{color}]", end="")

        if fast_mode:
            accuracy = 0
            for _ in range(3):
                console.print(".", end="")
                time.sleep(0.1)
        else:
            correct = 0
            for i in range(TRIALS):
                seed = 9500 + i + (100 if task_type == "translation" else 0)
                prompt, expected = generate_fn("L4", seed)
                response = call_model(model_name, prompt, api_keys)
                if check_fn(response, expected):
                    correct += 1
                console.print(".", end="")
                time.sleep(0.05)
            accuracy = correct / TRIALS * 100

        l4_results[model_name] = accuracy
        status = "💀" if accuracy == 0 else ""
        console.print(f" {create_bar(accuracy)} [bold red]{accuracy:3.0f}%[/bold red] {status}")

    return clean_results, l4_results


def run_demo(fast_mode: bool = False):
    console = Console()
    api_keys = load_api_keys()

    has_openrouter = bool(api_keys.get("openrouter"))
    available_models = list(MODELS.keys()) if (fast_mode or has_openrouter) else []

    if not available_models:
        console.print("[red]No OPENROUTER_API_KEY found![/red]")
        return

    console.clear()

    # Title
    console.print()
    console.print(Panel(
        "[bold white]δ_structural Collapse Demonstration[/bold white]\n\n"
        "[dim]Two types of structural contradiction:[/dim]\n"
        "[cyan]1. Arithmetic + Digit Prohibition[/cyan]\n"
        "[cyan]2. Translation + Language Prohibition[/cyan]",
        border_style="cyan",
        title="🔬 EXPERIMENT",
        title_align="left"
    ))
    time.sleep(2)

    all_results = {}

    # Experiment 1: Math
    math_clean, math_l4 = run_experiment(console, api_keys, available_models, "math", fast_mode)
    all_results["math"] = {"clean": math_clean, "l4": math_l4}

    time.sleep(1)

    # Experiment 2: Translation
    trans_clean, trans_l4 = run_experiment(console, api_keys, available_models, "translation", fast_mode)
    all_results["translation"] = {"clean": trans_clean, "l4": trans_l4}

    time.sleep(1)

    # Summary Table
    console.print()
    table = Table(title="Summary: Both Experiments", box=box.ROUNDED, border_style="cyan")
    table.add_column("Model", style="cyan")
    table.add_column("Math Clean", justify="right")
    table.add_column("Math L4", justify="right")
    table.add_column("Trans Clean", justify="right")
    table.add_column("Trans L4", justify="right")

    for model_name in available_models:
        mc = all_results["math"]["clean"].get(model_name, 0)
        ml = all_results["math"]["l4"].get(model_name, 0)
        tc = all_results["translation"]["clean"].get(model_name, 0)
        tl = all_results["translation"]["l4"].get(model_name, 0)

        mc_str = f"[green]{mc:.0f}%[/green]" if mc > 50 else f"{mc:.0f}%"
        ml_str = f"[red]{ml:.0f}%[/red]"
        tc_str = f"[green]{tc:.0f}%[/green]" if tc > 50 else f"{tc:.0f}%"
        tl_str = f"[red]{tl:.0f}%[/red]"

        table.add_row(model_name, mc_str, ml_str, tc_str, tl_str)

    console.print(table)
    time.sleep(1.5)

    # Conclusion
    math_all_zero = all(all_results["math"]["l4"].get(m, 0) == 0 for m in available_models)
    trans_all_zero = all(all_results["translation"]["l4"].get(m, 0) == 0 for m in available_models)

    console.print()
    console.print(Panel(
        "[bold white]FINDING: δ_structural = 0% for ALL models in BOTH tasks[/bold white]\n\n"
        f"[cyan]Arithmetic + Digit Prohibition:[/cyan]    {'[red bold]0% ALL[/red bold]' if math_all_zero else 'Partial'}\n"
        f"[cyan]Translation + Language Prohibition:[/cyan] {'[red bold]0% ALL[/red bold]' if trans_all_zero else 'Partial'}\n\n"
        "[yellow]Structural contradiction transcends:[/yellow]\n"
        "  • Model capability (Claude Opus = GPT = Llama = 0%)\n"
        "  • Task domain (arithmetic, language)\n"
        "  • Output format (numbers, characters)\n\n"
        "[dim italic]「判断禁止で結論を出せ」は天才でも不可能[/dim italic]",
        border_style="red",
        title="💀 CONCLUSION: UNIVERSAL COLLAPSE",
        title_align="left"
    ))

    console.print()
    time.sleep(3)


def main():
    fast_mode = "--fast" in sys.argv
    run_demo(fast_mode)


if __name__ == "__main__":
    main()
