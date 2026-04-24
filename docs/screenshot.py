#!/usr/bin/env python3
"""Generate an SVG screenshot of the zellai plugin interface.

Runs `cargo run --bin screenshot` to capture terminal text output, then uses
the `rich` library to render it as a styled terminal SVG.

Usage:
    python3 docs/screenshot.py

Output:
    docs/screenshot.svg
"""

import subprocess
import sys
from pathlib import Path

from rich.console import Console
from rich.text import Text

# Resolve paths relative to repo root (parent of docs/)
REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT_PATH = REPO_ROOT / "docs" / "screenshot.svg"


def capture_screenshot_text() -> str:
    """Run the screenshot binary and return its stdout."""
    result = subprocess.run(
        ["cargo", "run", "--bin", "screenshot"],
        capture_output=True,
        text=True,
        cwd=str(REPO_ROOT),
    )
    if result.returncode != 0:
        print(f"Error running screenshot binary:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)
    return result.stdout


# Color scheme — a dark terminal palette
BG = "#1a1b26"  # dark navy (Tokyo Night inspired)
FG = "#a9b1d6"  # soft lavender

# Status icon colors
COLOR_THINKING = "#7aa2f7"  # blue
COLOR_WAITING = "#e0af68"  # amber/yellow
COLOR_IDLE = "#565f89"  # dim gray
COLOR_ERROR = "#f7768e"  # red
COLOR_BORDER = "#3b4261"  # muted border
COLOR_TITLE = "#7dcfff"  # cyan
COLOR_BRANCH = "#9ece6a"  # green
COLOR_DIM = "#565f89"  # dim text
COLOR_STATUS_BAR = "#bb9af7"  # purple


def style_line(line: str) -> Text:
    """Apply rich styling to a single line of screenshot output."""
    text = Text()

    # Header box lines
    if line.startswith("┌") or line.startswith("└"):
        text.append(line, style=f"bold {COLOR_TITLE}")
        return text
    if line.startswith("│  zellai"):
        text.append("│  ", style=COLOR_BORDER)
        text.append("zellai", style=f"bold {COLOR_TITLE}")
        text.append(" — AI agent workspace for Zellij  ", style=FG)
        text.append("│", style=COLOR_BORDER)
        return text

    # Sidebar borders
    if line.startswith("╭") or line.startswith("╰"):
        text.append(line, style=COLOR_BORDER)
        return text

    # Status bar line
    if line.startswith("Status bar:"):
        label = "Status bar: "
        rest = line[len(label):]
        text.append(label, style=f"bold {COLOR_DIM}")
        text.append(rest, style=f"bold {COLOR_STATUS_BAR}")
        return text

    # Sidebar content lines starting with │
    if line.startswith("│"):
        text.append("│", style=COLOR_BORDER)
        inner = line[1:]
        if inner.endswith("│"):
            inner = inner[:-1]
            end_border = True
        else:
            end_border = False

        # Detect card type by status icon
        stripped = inner.lstrip()

        if stripped.startswith("◉"):
            # Thinking agent line
            text.append(inner.split("◉")[0], style=FG)
            text.append("◉", style=f"bold {COLOR_THINKING}")
            rest = inner.split("◉", 1)[1]
            # Split name — status
            if "—" in rest:
                parts = rest.split("—", 1)
                text.append(parts[0] + "—", style=FG)
                text.append(parts[1], style=COLOR_THINKING)
            else:
                text.append(rest, style=FG)
        elif stripped.startswith("⚠"):
            # Waiting/attention agent line
            text.append(inner.split("⚠")[0], style=FG)
            text.append("⚠", style=f"bold {COLOR_WAITING}")
            rest = inner.split("⚠", 1)[1]
            if "—" in rest:
                parts = rest.split("—", 1)
                text.append(parts[0] + "—", style=FG)
                text.append(parts[1], style=COLOR_WAITING)
            else:
                text.append(rest, style=FG)
        elif stripped.startswith("○"):
            # Idle agent line
            text.append(inner.split("○")[0], style=FG)
            text.append("○", style=COLOR_IDLE)
            rest = inner.split("○", 1)[1]
            if "—" in rest:
                parts = rest.split("—", 1)
                text.append(parts[0] + "—", style=COLOR_DIM)
                text.append(parts[1], style=COLOR_DIM)
            else:
                text.append(rest, style=COLOR_DIM)
        elif stripped.startswith("✗"):
            # Error agent line
            text.append(inner.split("✗")[0], style=FG)
            text.append("✗", style=f"bold {COLOR_ERROR}")
            rest = inner.split("✗", 1)[1]
            if "—" in rest:
                parts = rest.split("—", 1)
                text.append(parts[0] + "—", style=FG)
                text.append(parts[1], style=COLOR_ERROR)
            else:
                text.append(rest, style=FG)
        elif "●" in stripped:
            # Branch / dir detail line
            text.append(inner.split("●")[0], style=COLOR_BRANCH)
            text.append("●", style=COLOR_DIM)
            text.append(inner.split("●", 1)[1], style=COLOR_DIM)
        elif stripped.strip():
            # Message detail line (3rd line of detailed card)
            text.append(inner, style=COLOR_DIM)
        else:
            # Empty line inside sidebar
            text.append(inner, style=FG)

        if end_border:
            text.append("│", style=COLOR_BORDER)

        return text

    # Default: plain text
    text.append(line, style=FG)
    return text


def main():
    raw = capture_screenshot_text()
    lines = raw.rstrip("\n").split("\n")

    # Create a console that records output for SVG export
    console = Console(record=True, width=52, force_terminal=True)

    for line in lines:
        styled = style_line(line)
        console.print(styled, highlight=False)

    svg = console.export_svg(title="zellai", theme=None)

    # Patch the SVG background to our dark theme
    svg = svg.replace("#292929", BG)
    svg = svg.replace("#0c0c0c", BG)

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(svg, encoding="utf-8")
    print(f"✓ Screenshot saved to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
