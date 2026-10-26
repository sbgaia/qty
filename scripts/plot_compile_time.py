# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9"]
# ///
"""Draw the results of `scripts/compile_time.py` as a bar chart.

One group of bars per measurement of the program (rebuild, edit one
function, from scratch), one bar per library, with the mean and an error
bar of one standard deviation. Plain `f64` is the floor.

Usage:
    python3 scripts/compile_time.py --runs 10 --json results.json
    uv run scripts/plot_compile_time.py results.json
    uv run scripts/plot_compile_time.py results.json --command check --output compile-times-check.svg
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.figure import Figure

# Measurements: (key in the JSON, title).
MEASUREMENTS = [
    ("rebuild", "Rebuild after a change"),
    ("edit", "Edit one function\n(incremental)"),
    ("scratch", "From scratch,\nlibrary included"),
]

# The bars of each group, top to bottom: (library in the JSON, label, colour).
# The floor, plain f64, comes first, dark as in plot_benchmarks.py; qty stands
# out and uom is muted.
LIBRARIES = [
    ("f64", "f64 (no units)", "#334155"),  # slate-700
    ("qty", "qty", "#2563eb"),  # blue-600
    ("uom", "uom", "#94a3b8"),  # slate-400
]

TEXT_COLOR = "#334155"
MUTED_COLOR = "#64748b"
GRID_COLOR = "#e2e8f0"


def build_figure(results: dict, command: str) -> Figure:
    groups = [(key, title) for key, title in MEASUREMENTS if key in results]
    if not groups:
        sys.exit("No program measurements in the results: run compile_time.py without `--only library`.")

    bar_height = 0.26
    fig, ax = plt.subplots(figsize=(9, 1.6 + 1.25 * len(groups)))
    longest = 0.0
    for group_index, (key, _) in enumerate(groups):
        rows = {row["library"]: row for row in results[key]}
        means = {
            library: statistics.mean(row["times"][command]) for library, row in rows.items() if command in row["times"]
        }
        for bar_index, (library, label, color) in enumerate(LIBRARIES):
            row = rows.get(library)
            if row is None or command not in row["times"]:
                continue
            times = row["times"][command]
            mean = means[library]
            spread = statistics.stdev(times) if len(times) > 1 else 0.0
            longest = max(longest, mean + spread)
            # The 0.04 leaves a gap of background between neighbouring bars.
            y = group_index + (bar_index - 1) * (bar_height + 0.04)
            ax.barh(
                y,
                mean,
                height=bar_height,
                color=color,
                xerr=spread if spread else None,
                error_kw={"ecolor": MUTED_COLOR, "elinewidth": 1, "capsize": 3},
                label=label if group_index == 0 else None,
            )
            text = f"  {mean:.1f} s"
            if library == "qty" and "uom" in means:
                text += f"   {compared_with_uom(means['uom'] / mean)}"
            ax.text(
                mean + spread,
                y,
                text,
                va="center",
                ha="left",
                fontsize=9,
                color=TEXT_COLOR,
                fontweight="bold" if library == "qty" else "normal",
            )

    command_text = {"check": "`cargo check`", "debug": "debug build"}[command]
    machine = results["machine"]
    ax.set_title(
        f"Compile time of {results['size']:,} functions of unit code, {command_text}",
        fontsize=12,
        fontweight="bold",
        color=TEXT_COLOR,
        loc="left",
        pad=30,
    )
    ax.set_yticks(range(len(groups)), [title for _, title in groups], fontsize=10, color=TEXT_COLOR)
    ax.invert_yaxis()
    # Room on the right for the labels.
    ax.set_xlim(0, longest * 1.25)
    ax.set_xlabel(
        f"Seconds, mean of {results['runs']} runs ± one standard deviation (lower is better)\n"
        f"{machine['cores']} cores, {machine['rustc']}, uom {results['uom']}",
        fontsize=8,
        color=MUTED_COLOR,
    )
    ax.tick_params(axis="x", labelsize=8, colors=MUTED_COLOR)
    ax.tick_params(axis="y", length=0)
    ax.grid(axis="x", color=GRID_COLOR, linewidth=0.8)
    ax.set_axisbelow(True)
    for side in ("top", "right", "left"):
        ax.spines[side].set_visible(False)
    ax.spines["bottom"].set_color(GRID_COLOR)
    # Above the bars, under the title, where no bar can reach it.
    ax.legend(
        loc="lower left",
        bbox_to_anchor=(0, 1.0),
        ncol=len(LIBRARIES),
        frameon=False,
        fontsize=9,
        labelcolor=TEXT_COLOR,
        handlelength=1.2,
        borderaxespad=0.2,
    )
    fig.tight_layout()
    return fig


def compared_with_uom(ratio: float) -> str:
    """How qty's time compares with uom's, given uom's time over qty's."""
    if ratio >= 1.05:
        return f"{ratio:.1f}× faster than uom"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× slower than uom"
    return "as fast as uom"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("results", type=Path, help="the JSON file written by compile_time.py --json")
    parser.add_argument(
        "--command",
        choices=["check", "debug"],
        default="debug",
        help="which build to show; every measurement has both (default debug)",
    )
    parser.add_argument("--output", type=Path, default=Path("compile-times.svg"), help="default compile-times.svg")
    args = parser.parse_args()

    results = json.loads(args.results.read_text())
    fig = build_figure(results, args.command)
    fig.savefig(str(args.output), bbox_inches="tight", facecolor="white")
    plt.close(fig)
    print(f"Saved the chart to {args.output}")


if __name__ == "__main__":
    main()
