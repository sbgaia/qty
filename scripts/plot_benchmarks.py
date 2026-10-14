# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9"]
# ///
"""Generate a grid of bar charts comparing benchmark results across libraries.

Reads Criterion JSON output from target/criterion/ and produces benchmarks.png.

Usage:
    uv run scripts/plot_benchmarks.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.figure import Figure

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

CRITERION_DIR = Path("target/criterion")
OUTPUT_PATH = Path("benchmarks.png")

# Benchmark groups: (directory name, human-readable title)
GROUPS: list[tuple[str, str]] = [
    ("construction_from_km", "Construction (from km)"),
    ("convert_km_to_m", "Conversion (km to m)"),
    ("add_two_lengths", "Addition (a + b)"),
    ("chain_construct_add_convert", "Chain (construct + add + convert)"),
    ("sum_10k_lengths", "Sum of 10k lengths"),
    ("scale_10k_lengths", "Scale 10k lengths in place"),
]

# Colour for unitsnet-rs, the unit-free baseline, and everything else
HIGHLIGHT_COLOR = "#2563eb"  # blue-600
BASELINE_NAME = "f64 (no units)"
BASELINE_COLOR = "#334155"  # slate-700
DEFAULT_COLOR = "#94a3b8"  # slate-400


# ---------------------------------------------------------------------------
# Data helpers
# ---------------------------------------------------------------------------


def read_mean_ns(estimates_path: Path) -> float:
    """Return the mean point estimate (ns) from a Criterion estimates.json."""
    with estimates_path.open() as f:
        data = json.load(f)
    return float(data["mean"]["point_estimate"])


def format_time(ns: float) -> str:
    """Format a nanosecond value into a human-readable string with auto-scaled units."""
    if ns < 1.0:
        return f"{ns * 1000:.1f} ps"
    if ns < 1000.0:
        return f"{ns:.2f} ns"
    if ns < 1_000_000.0:
        return f"{ns / 1000:.2f} us"
    return f"{ns / 1_000_000:.2f} ms"


def collect_results() -> dict[str, dict[str, float]]:
    """Walk criterion output and return {group: {library: mean_ns}}."""
    results: dict[str, dict[str, float]] = {}
    for group_dir, _ in GROUPS:
        group_path = CRITERION_DIR / group_dir
        if not group_path.is_dir():
            print(f"Warning: missing group directory {group_path}", file=sys.stderr)
            continue
        libs: dict[str, float] = {}
        for lib_path in sorted(group_path.iterdir()):
            est = lib_path / "new" / "estimates.json"
            if est.is_file():
                libs[lib_path.name] = read_mean_ns(est)
        results[group_dir] = libs
    return results


# ---------------------------------------------------------------------------
# Plotting
# ---------------------------------------------------------------------------


def build_figure(results: dict[str, dict[str, float]]) -> Figure:
    """Create a two-column grid of horizontal bar charts, one per group."""
    rows = (len(GROUPS) + 1) // 2
    fig, axes = plt.subplots(rows, 2, figsize=(14, 4 * rows))
    fig.suptitle(
        "Rust Units Library Benchmark Comparison",
        fontsize=16,
        fontweight="bold",
        y=0.98,
    )

    for ax, (group_dir, title) in zip(axes.flat, GROUPS):
        libs = results.get(group_dir, {})
        if not libs:
            ax.set_visible(False)
            continue

        # Sort by time descending so the fastest library ends up at the bottom
        # (visually prominent position for horizontal bars).
        sorted_items = sorted(libs.items(), key=lambda kv: kv[1], reverse=True)
        names = [name for name, _ in sorted_items]
        values = [val for _, val in sorted_items]
        colors = [
            HIGHLIGHT_COLOR
            if n == "unitsnet-rs"
            else BASELINE_COLOR
            if n == BASELINE_NAME
            else DEFAULT_COLOR
            for n in names
        ]

        bars = ax.barh(names, values, color=colors, height=0.6, edgecolor="white")

        # Annotate each bar with its exact value
        max_val = max(values)
        for bar, val in zip(bars, values):
            label = format_time(val)
            # Place label inside the bar if it's wide enough, otherwise outside
            if val > max_val * 0.35:
                ax.text(
                    bar.get_width() - max_val * 0.02,
                    bar.get_y() + bar.get_height() / 2,
                    label,
                    ha="right",
                    va="center",
                    fontsize=9,
                    fontweight="bold",
                    color="white",
                )
            else:
                ax.text(
                    bar.get_width() + max_val * 0.02,
                    bar.get_y() + bar.get_height() / 2,
                    label,
                    ha="left",
                    va="center",
                    fontsize=9,
                    fontweight="bold",
                    color="#334155",
                )

        ax.set_title(title, fontsize=12, fontweight="bold", pad=8)
        ax.set_xlabel("Mean time (lower is better)", fontsize=9, color="#64748b")
        ax.tick_params(axis="y", labelsize=10)
        ax.tick_params(axis="x", labelsize=8, colors="#94a3b8")
        ax.spines["top"].set_visible(False)
        ax.spines["right"].set_visible(False)
        ax.spines["bottom"].set_color("#e2e8f0")
        ax.spines["left"].set_color("#e2e8f0")

    fig.tight_layout(rect=[0, 0, 1, 0.94])
    return fig


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    if not CRITERION_DIR.is_dir():
        print(
            f"Error: {CRITERION_DIR} not found. Run `cargo bench -p unitsnet-benches` first.",
            file=sys.stderr,
        )
        sys.exit(1)

    results = collect_results()
    if not results:
        print("Error: no benchmark results found.", file=sys.stderr)
        sys.exit(1)

    fig = build_figure(results)
    fig.savefig(str(OUTPUT_PATH), dpi=200, bbox_inches="tight", facecolor="white")
    plt.close(fig)
    print(f"Saved benchmark plot to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
