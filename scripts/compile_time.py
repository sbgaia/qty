# /// script
# requires-python = ">=3.9"
# ///
"""Compare the compile times of qty, uom and plain `f64` code.

The program measured is the same in all three: `--size` functions (1,000 by
default) in modules of a hundred, each with the arithmetic, unit literals
and unit conversions that everyday code has: speeds, forces, energies and
powers from named quantities, kinetic energy and orbital formulas with
plain operators, values in km, h, bar or kPa, and conversions such as
miles to feet or °F to °C. It is the `everyday` pattern of
`scripts/scaling.py`; with `f64` it is the same arithmetic on plain numbers,
which is what the compiler would do without any units library, the floor.

Four measurements:

1. Rebuild: the program changed, its dependencies are already built, and
   incremental compilation is off, so the whole program compiles again:
   `cargo check`, a debug build and a release build.
2. Edit one function: the developer's loop. The program was built with
   incremental compilation, one function changes, and it builds again:
   `cargo check` (what an editor runs) and a debug build.
3. From scratch: a fresh target directory, so the library compiles too, as
   on a first build or in CI: `cargo check` and a debug build. qty with its
   core quantities only, all the program needs, is measured as well.
4. Library: a cold build of the library alone, from `cargo clean`. qty and
   uom are built with the same features: every quantity, `f64` storage and
   `std`, then the same with serde. qty with its core quantities only comes
   last.

Unless said otherwise, qty is built with every quantity
(`default-features = false, features = ["std", "full"]`, 128 quantities)
and uom with `default-features = false, features = ["si", "f64", "std"]`
(116 SI quantities; uom can't leave any out).

Each measurement runs in its own throwaway Cargo project under
`target/compile-time/`, against this checkout of qty and uom from crates.io.
Times are wall clock; the report gives the mean and the standard
deviation of `--runs` runs, and `--json` keeps every time. Rebuilds and
edits start with one untimed run. `scripts/plot_compile_time.py` draws the
JSON as a bar chart. Run it on an otherwise idle machine:

    python3 scripts/compile_time.py
    python3 scripts/compile_time.py --runs 10 --json results.json
    python3 scripts/compile_time.py --only program --size 100 --runs 1

It needs network access for the first run, to fetch uom.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

from scaling import program

REPO_ROOT = Path(__file__).resolve().parent.parent
WORK_DIR = REPO_ROOT / "target" / "compile-time"

# Library configurations: (name, crate, dependency line for Cargo.toml).
# `{qty}` is replaced by the path of this checkout, `{uom}` by the uom version.
# They come in pairs with the same features: every quantity (qty's 128, uom's
# 116 SI ones), `f64` storage and `std`, then the same with serde. uom's
# `autoconvert` and `f32` are off, as qty has no counterpart for them. The
# last row, qty with its core quantities only, has no uom counterpart.
LIBRARY_CONFIGS = [
    (
        "qty, all quantities (std, full)",
        "qty",
        'qty = {{ path = "{qty}", default-features = false, features = ["std", "full"] }}',
    ),
    (
        "uom, all quantities (si, f64, std)",
        "uom",
        'uom = {{ version = "{uom}", default-features = false, features = ["si", "f64", "std"] }}',
    ),
    (
        "qty, all quantities and serde",
        "qty",
        'qty = {{ path = "{qty}", default-features = false, features = ["std", "full", "serde"] }}',
    ),
    (
        "uom, all quantities and serde",
        "uom",
        'uom = {{ version = "{uom}", default-features = false, features = ["si", "f64", "std", "use_serde"] }}',
    ),
    (
        "qty, core quantities (std only)",
        "qty",
        'qty = {{ path = "{qty}", default-features = false, features = ["std"] }}',
    ),
]

# Cargo invocations for the library builds: (name, arguments).
LIBRARY_COMMANDS = [
    ("check", ["check"]),
    ("debug", ["build"]),
    ("release", ["build", "--release"]),
]

# Libraries the program is written with: (name, library, dependency line).
# `library` picks the version of the program, qty's, uom's or plain `f64`.
PROGRAM_CONFIGS = [
    ("qty", "qty", 'qty = {{ path = "{qty}", default-features = false, features = ["std", "full"] }}'),
    ("uom", "uom", 'uom = {{ version = "{uom}", default-features = false, features = ["si", "f64", "std"] }}'),
    ("f64", "f64", ""),
]
# Also measured from scratch: the quantities the program uses are all core.
CORE_CONFIG = (
    "qty, core quantities",
    "qty",
    'qty = {{ path = "{qty}", default-features = false, features = ["std"] }}',
)
PATTERN = "everyday"

# The measurements of the program: (key, title, cargo invocations).
REBUILD_COMMANDS = [("check", ["check"]), ("debug", ["build"]), ("release", ["build", "--release"])]
EDIT_COMMANDS = [("check", ["check"]), ("debug", ["build"])]
SCRATCH_COMMANDS = [("check", ["check"]), ("debug", ["build"])]
# Untimed runs before the timed ones in the rebuild and edit measurements.
# The first incremental rebuild after a full build is several times slower
# than the ones after it, which are what someone editing the code waits for.
WARMUP_RUNS = 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.split("\n\n")[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--runs", type=int, default=3, help="runs per measurement (default 3)")
    parser.add_argument("--size", type=int, default=1000, help="functions in the program (default 1000)")
    parser.add_argument("--uom-version", default="0.38.0", help="uom version to compare with (default 0.38.0)")
    parser.add_argument("--only", choices=["program", "library"], help="run one kind of measurement only")
    parser.add_argument("--json", type=Path, help="also write the results to this file")
    args = parser.parse_args()
    if args.runs < 1 or args.size < 2:
        parser.error("--runs must be at least 1 and --size at least 2")

    machine = describe_machine()
    print(f"# Compile times, qty against uom {args.uom_version} and plain f64\n")
    for key, value in machine.items():
        print(f"- {key}: {value}")
    print(f"- runs per measurement: {args.runs}\n")

    results: dict = {"machine": machine, "runs": args.runs, "uom": args.uom_version, "size": args.size}
    if args.only in (None, "program"):
        results["rebuild"] = measure_rebuild(args)
        results["edit"] = measure_edit(args)
        results["scratch"] = measure_scratch(args)
    if args.only in (None, "library"):
        results["library"] = measure_library(args)

    if args.json:
        args.json.write_text(json.dumps(results, indent=2) + "\n")
        print(f"\nResults written to {args.json}")
    return 0


def measure_rebuild(args: argparse.Namespace) -> list[dict]:
    """Whole-program rebuilds, with the dependencies already built."""
    print(f"## Your code changed: rebuild of the program, {args.size} functions\n")
    print_header(REBUILD_COMMANDS)
    rows = []
    for name, library, dependency in PROGRAM_CONFIGS:
        project = make_project(f"rebuild-{name}", dependency.format(qty=REPO_ROOT, uom=args.uom_version))
        source = program(PATTERN, library, args.size)
        row = {"library": name, "times": {}}
        for command_name, command in REBUILD_COMMANDS:
            # Build the dependencies (and check that the program compiles).
            write_main(project, source)
            cargo(project, command, incremental=False)
            times = []
            for run in range(-WARMUP_RUNS, args.runs):
                # A changed file makes Cargo rebuild the program, and only it.
                write_main(project, source + f"// run {run} {time.time_ns()}\n")
                elapsed = timed(project, command, incremental=False)
                if run >= 0:
                    times.append(elapsed)
            row["times"][command_name] = times
        rows.append(row)
        print_row(name, row, REBUILD_COMMANDS)
    print()
    return rows


def measure_edit(args: argparse.Namespace) -> list[dict]:
    """Incremental builds after a change to one function's body."""
    print(f"## Edit one function: incremental build, {args.size} functions\n")
    print_header(EDIT_COMMANDS)
    rows = []
    for name, library, dependency in PROGRAM_CONFIGS:
        project = make_project(f"edit-{name}", dependency.format(qty=REPO_ROOT, uom=args.uom_version))
        source = program(PATTERN, library, args.size)
        row = {"library": name, "times": {}}
        for command_name, command in EDIT_COMMANDS:
            write_main(project, source)
            cargo(project, command, incremental=True)
            times = []
            for run in range(-WARMUP_RUNS, args.runs):
                write_main(project, edit_one_function(source, args.size // 2, run))
                elapsed = timed(project, command, incremental=True)
                if run >= 0:
                    times.append(elapsed)
            row["times"][command_name] = times
        rows.append(row)
        print_row(name, row, EDIT_COMMANDS)
    print()
    return rows


def measure_scratch(args: argparse.Namespace) -> list[dict]:
    """Builds of the program and its dependencies from an empty target directory."""
    print(f"## From scratch: library and program, {args.size} functions\n")
    print_header(SCRATCH_COMMANDS)
    rows = []
    configs = PROGRAM_CONFIGS[:1] + [CORE_CONFIG] + PROGRAM_CONFIGS[1:]
    for name, library, dependency in configs:
        project = make_project(f"scratch-{len(rows)}", dependency.format(qty=REPO_ROOT, uom=args.uom_version))
        write_main(project, program(PATTERN, library, args.size))
        cargo(project, ["fetch"], incremental=False)
        row = {"library": name, "dependency": dependency, "times": {}}
        for command_name, command in SCRATCH_COMMANDS:
            times = []
            for _ in range(args.runs):
                cargo(project, ["clean"], incremental=False)
                times.append(timed(project, command, incremental=False))
            row["times"][command_name] = times
        rows.append(row)
        print_row(name, row, SCRATCH_COMMANDS)
    print()
    return rows


def edit_one_function(source: str, index: int, run: int) -> str:
    """The program with a new statement in the body of function `index`."""
    signature = f"pub fn f{index}(a: f64, b: f64) -> f64 {{\n"
    edited = source.replace(signature, signature + f"        let a = a + {run + 2}.0e-9;\n", 1)
    assert edited != source, f"function f{index} not found"
    return edited


def print_header(commands: list) -> None:
    print("| Library | " + " | ".join(name for name, _ in commands) + " |")
    print("| --- |" + " ---: |" * len(commands))


def print_row(name: str, row: dict, commands: list) -> None:
    print(f"| {name} | " + " | ".join(fmt(row["times"][c]) for c, _ in commands) + " |", flush=True)


def measure_library(args: argparse.Namespace) -> list[dict]:
    """Cold builds of each library configuration."""
    print("## Library alone, cold build\n")
    print("| Configuration | " + " | ".join(name for name, _ in LIBRARY_COMMANDS) + " |")
    print("| --- |" + " --- |" * len(LIBRARY_COMMANDS))
    rows = []
    for index, (name, _crate, dependency) in enumerate(LIBRARY_CONFIGS):
        project = make_project(f"library-{index}", dependency.format(qty=REPO_ROOT, uom=args.uom_version))
        write_main(project, "fn main() {}\n")
        cargo(project, ["fetch"], incremental=False)
        row = {"configuration": name, "dependency": dependency, "times": {}}
        for command_name, command in LIBRARY_COMMANDS:
            times = []
            for _ in range(args.runs):
                cargo(project, ["clean"], incremental=False)
                times.append(timed(project, command, incremental=False))
            row["times"][command_name] = times
        rows.append(row)
        print(f"| {name} | " + " | ".join(fmt(row["times"][c]) for c, _ in LIBRARY_COMMANDS) + " |")
    print()
    return rows


def make_project(name: str, dependency: str) -> Path:
    """A fresh binary crate with one dependency, in its own workspace."""
    project = WORK_DIR / name
    shutil.rmtree(project, ignore_errors=True)
    (project / "src").mkdir(parents=True)
    # The empty `[workspace]` keeps Cargo from treating it as part of qty's
    # workspace, which it sits inside.
    (project / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n\n'
        f"[dependencies]\n{dependency}\n\n[workspace]\n"
    )
    return project


def write_main(project: Path, source: str) -> None:
    (project / "src" / "main.rs").write_text(source)


def env(incremental: bool) -> dict:
    environment = dict(os.environ)
    environment["CARGO_INCREMENTAL"] = "1" if incremental else "0"
    environment.pop("RUSTFLAGS", None)
    return environment


def cargo(project: Path, arguments: list[str], incremental: bool) -> None:
    result = subprocess.run(
        ["cargo", *arguments, "--quiet"],
        cwd=project,
        env=env(incremental),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"`cargo {' '.join(arguments)}` failed in {project}:\n{result.stderr}")


def timed(project: Path, arguments: list[str], incremental: bool) -> float:
    start = time.perf_counter()
    cargo(project, arguments, incremental)
    return time.perf_counter() - start


def fmt(times: list[float]) -> str:
    mean = statistics.mean(times)
    if len(times) == 1:
        return f"{mean:.1f} s"
    return f"{mean:.1f} ± {statistics.stdev(times):.1f} s"


def describe_machine() -> dict:
    def output(command: list[str]) -> str:
        try:
            return subprocess.run(command, capture_output=True, text=True, check=True).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return "unknown"

    cpu = platform.processor() or "unknown"
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    return {
        "rustc": output(["rustc", "--version"]),
        "cpu": cpu,
        "cores": os.cpu_count(),
        "os": platform.platform(),
        "qty commit": output(["git", "-C", str(REPO_ROOT), "rev-parse", "--short", "HEAD"]),
    }


if __name__ == "__main__":
    sys.exit(main())
