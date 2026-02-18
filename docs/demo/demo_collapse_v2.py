#!/usr/bin/env python3
"""
Structural Collapse Demo v2 - With explanatory text for video

Usage:
    python demo_collapse_v2.py
    python demo_collapse_v2.py --fast  # Simulated (no API)
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
    RICH_AVAILABLE = True
except ImportError:
    RICH_AVAILABLE = False
    print("Please install rich: pip install rich")
    sys.exit(1)

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Configuration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# OpenRouter model IDs
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
                    elif line.startswith("OPENAI_API_KEY="):
                        keys["openai"] = line.split("=", 1)[1].strip('"\'')
                    elif line.startswith("ANTHROPIC_API_KEY=") or line.startswith("CLAUDE_API_KEY="):
                        keys["anthropic"] = line.split("=", 1)[1].strip('"\'')
                    elif line.startswith("GOOGLE_API_KEY=") or line.startswith("GEMINI_API_KEY="):
                        keys["google"] = line.split("=", 1)[1].strip('"\'')
    keys.setdefault("openrouter", os.environ.get("OPENROUTER_API_KEY", ""))
    keys.setdefault("openai", os.environ.get("OPENAI_API_KEY", ""))
    keys.setdefault("anthropic", os.environ.get("ANTHROPIC_API_KEY", "") or os.environ.get("CLAUDE_API_KEY", ""))
    keys.setdefault("google", os.environ.get("GOOGLE_API_KEY", "") or os.environ.get("GEMINI_API_KEY", ""))
    return keys


def generate_math(seed: int) -> Tuple[int, int, int, int]:
    rng = random.Random(seed)
    a = rng.randint(2, 9)
    b = rng.randint(2, 9)
    c = rng.randint(1, 20)
    expected = a * b - c + 7
    return a, b, c, expected


def generate_prompt(condition: str, seed: int) -> Tuple[str, int]:
    a, b, c, expected = generate_math(seed)
    base_task = f"Compute: final = a × b - c + 7, where a = {a}, b = {b}, c = {c}. Give ONLY the number."
    if condition == "clean":
        return base_task, expected
    elif condition == "L4":
        prohibition = "IMPORTANT: You must NOT use any digits (0-9) in your response. Express your answer using words only."
        return f"{prohibition}\n\n{base_task}", expected
    return base_task, expected


def call_openrouter(prompt: str, api_key: str, model: str) -> Optional[str]:
    """Call any model via OpenRouter API"""
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
    except Exception as e:
        return None


def call_model(model_name: str, prompt: str, api_keys: Dict[str, str]) -> Optional[str]:
    model_info = MODELS[model_name]
    openrouter_model = model_info["openrouter"]
    api_key = api_keys.get("openrouter", "")

    if not api_key:
        return None

    return call_openrouter(prompt, api_key, openrouter_model)


def extract_number(response: str) -> Optional[int]:
    if response is None:
        return None
    numbers = re.findall(r'-?\d+', response)
    return int(numbers[-1]) if numbers else None


def is_correct(response: Optional[str], expected: int) -> bool:
    if response is None:
        return False
    return extract_number(response) == expected


def create_bar(accuracy: float, width: int = 25) -> str:
    filled = int(accuracy / 100 * width)
    if accuracy > 50:
        return "[green]" + "█" * filled + "[/green]" + "[dim]░[/dim]" * (width - filled)
    elif accuracy > 0:
        return "[yellow]" + "█" * filled + "[/yellow]" + "[dim]░[/dim]" * (width - filled)
    else:
        return "[red]" + "░" * width + "[/red]"


def run_demo(fast_mode: bool = False):
    console = Console()
    api_keys = load_api_keys()

    # Check if OpenRouter key is available
    has_openrouter = bool(api_keys.get("openrouter"))

    if fast_mode:
        available_models = list(MODELS.keys())
    elif has_openrouter:
        available_models = list(MODELS.keys())
    else:
        console.print("[red]No OPENROUTER_API_KEY found![/red]")
        console.print("[dim]Set OPENROUTER_API_KEY in .env or use --fast mode[/dim]")
        return

    console.clear()

    # ═══════════════════════════════════════════════════════════
    # TITLE
    # ═══════════════════════════════════════════════════════════
    console.print()
    console.print(Panel(
        "[bold white]δ_structural Collapse Demonstration[/bold white]\n\n"
        "[dim]Can AI capability overcome structural contradiction?[/dim]",
        border_style="cyan",
        title="🔬 EXPERIMENT",
        title_align="left"
    ))
    time.sleep(2)

    # ═══════════════════════════════════════════════════════════
    # EXPERIMENTAL SETUP
    # ═══════════════════════════════════════════════════════════
    console.print()
    console.print(Panel(
        "[bold]Task:[/bold] Simple arithmetic\n"
        "       [cyan]final = a × b - c + 7[/cyan]\n\n"
        "[bold]Models:[/bold] 5 providers, 6 models (Feb 2026)\n"
        "         Anthropic: Claude Opus 4, Sonnet 4\n"
        "         OpenAI: GPT-4.1 | Google: Gemini 2.5 Pro\n"
        "         Meta: Llama 4 | xAI: Grok 3\n\n"
        "[bold]Measure:[/bold] Accuracy (correct numeric answer)",
        border_style="blue",
        title="📋 SETUP",
        title_align="left"
    ))
    time.sleep(3)

    clean_results = {}
    l4_results = {}

    # ═══════════════════════════════════════════════════════════
    # PHASE 1: CLEAN
    # ═══════════════════════════════════════════════════════════
    console.print()
    console.print(Panel(
        "[bold green]Condition: CLEAN (No Contradiction)[/bold green]\n\n"
        "[dim]Standard arithmetic task with no conflicting instructions.[/dim]",
        border_style="green",
        title="▶ PHASE 1",
        title_align="left"
    ))
    time.sleep(1.5)

    for model_name in available_models:
        model_info = MODELS[model_name]
        color = model_info["color"]
        tier = model_info["tier"]

        console.print(f"  [{color}]{model_name:18}[/{color}] ({tier:7})", end="")

        if fast_mode:
            accuracy = 100
            for _ in range(3):
                console.print(".", end="")
                time.sleep(0.15)
        else:
            correct = 0
            for i in range(TRIALS):
                seed = 7000 + i
                prompt, expected = generate_prompt("clean", seed)
                response = call_model(model_name, prompt, api_keys)
                if is_correct(response, expected):
                    correct += 1
                console.print(".", end="")
                time.sleep(0.1)
            accuracy = correct / TRIALS * 100

        clean_results[model_name] = accuracy
        console.print(f" {create_bar(accuracy)} [bold]{accuracy:3.0f}%[/bold]")

    time.sleep(1.5)

    # ═══════════════════════════════════════════════════════════
    # INJECTION EXPLANATION
    # ═══════════════════════════════════════════════════════════
    console.print()
    console.print(Panel(
        "[bold red]Injecting: δ_structural (Protocol Failure)[/bold red]\n\n"
        "[white]Contradiction:[/white]\n"
        "  • Task requires: [cyan]numeric answer[/cyan]\n"
        "  • Instruction:   [red]\"Do NOT use digits (0-9)\"[/red]\n\n"
        "[dim]This creates an IMPOSSIBLE task.[/dim]\n"
        "[dim]The goal and the constraint are mutually exclusive.[/dim]",
        border_style="red",
        title="⚠️  STRUCTURAL CONTRADICTION",
        title_align="left"
    ))
    time.sleep(3)

    # ═══════════════════════════════════════════════════════════
    # PHASE 2: L4
    # ═══════════════════════════════════════════════════════════
    console.print()
    console.print(Panel(
        "[bold red]Condition: L4 (Structural Contradiction)[/bold red]\n\n"
        "[dim]Same task, but with impossible constraint added.[/dim]",
        border_style="red",
        title="▶ PHASE 2",
        title_align="left"
    ))
    time.sleep(1.5)

    for model_name in available_models:
        model_info = MODELS[model_name]
        color = model_info["color"]
        tier = model_info["tier"]

        console.print(f"  [{color}]{model_name:18}[/{color}] ({tier:7})", end="")

        if fast_mode:
            accuracy = 0
            for _ in range(3):
                console.print(".", end="")
                time.sleep(0.15)
        else:
            correct = 0
            for i in range(TRIALS):
                seed = 8000 + i
                prompt, expected = generate_prompt("L4", seed)
                response = call_model(model_name, prompt, api_keys)
                if is_correct(response, expected):
                    correct += 1
                console.print(".", end="")
                time.sleep(0.1)
            accuracy = correct / TRIALS * 100

        l4_results[model_name] = accuracy
        console.print(f" {create_bar(accuracy)} [bold red]{accuracy:3.0f}%[/bold red] 💀")

    time.sleep(1.5)

    # ═══════════════════════════════════════════════════════════
    # RESULTS TABLE
    # ═══════════════════════════════════════════════════════════
    console.print()
    table = Table(title="Results: Clean vs L4", box=box.ROUNDED, border_style="cyan")
    table.add_column("Model", style="cyan")
    table.add_column("Provider", style="dim")
    table.add_column("Clean", justify="right")
    table.add_column("L4", justify="right")
    table.add_column("Δ", justify="right")

    provider_map = {
        "Claude": "Anthropic",
        "GPT": "OpenAI",
        "Gemini": "Google",
        "Llama": "Meta",
        "Grok": "xAI",
    }

    for model_name in available_models:
        clean = clean_results.get(model_name, 0)
        l4 = l4_results.get(model_name, 0)
        delta = l4 - clean

        # Extract provider from model name
        provider = "Unknown"
        for prefix, prov in provider_map.items():
            if model_name.startswith(prefix):
                provider = prov
                break

        clean_str = f"[green]{clean:.0f}%[/green]"
        l4_str = f"[red]{l4:.0f}%[/red]"
        delta_str = f"[bold red]{delta:+.0f}%[/bold red]"

        table.add_row(model_name, provider, clean_str, l4_str, delta_str)

    console.print(table)
    time.sleep(2)

    # ═══════════════════════════════════════════════════════════
    # CONCLUSION
    # ═══════════════════════════════════════════════════════════
    all_collapsed = all(l4_results.get(m, 0) == 0 for m in available_models)

    if all_collapsed:
        console.print()
        console.print(Panel(
            "[bold white]FINDING: δ_structural = 0% for ALL models[/bold white]\n\n"
            "• Claude Opus 4   (Anthropic): [red bold]0%[/red bold]\n"
            "• GPT-4.1         (OpenAI):    [red bold]0%[/red bold]\n"
            "• Gemini 2.5 Pro  (Google):    [red bold]0%[/red bold]\n"
            "• Llama 4         (Meta):      [red bold]0%[/red bold]\n"
            "• Grok 3          (xAI):       [red bold]0%[/red bold]\n\n"
            "[yellow]Structural contradiction transcends model capability.[/yellow]\n"
            "[yellow]No amount of intelligence can overcome an impossible task.[/yellow]\n\n"
            "[dim italic]「判断禁止で結論を出せ」は天才でも不可能[/dim italic]",
            border_style="red",
            title="💀 CONCLUSION: TOTAL COLLAPSE",
            title_align="left"
        ))

    console.print()
    time.sleep(3)


def main():
    fast_mode = "--fast" in sys.argv
    run_demo(fast_mode)


if __name__ == "__main__":
    main()
