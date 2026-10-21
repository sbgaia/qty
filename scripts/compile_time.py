# /// script
# requires-python = ">=3.9"
# ///
"""Compare the compile times of qty and uom.

Two kinds of measurement:

1. Library: a cold build of the crate itself, from `cargo clean`, for
   `cargo check`, a debug build and a release build. Both libraries are
   built with the same features: every quantity, `f64` storage and `std`,
   then the same with serde. qty with its core quantities only comes last.
2. Downstream: rebuilding a crate that uses the library, with the library
   already built (every quantity, `f64`, `std`). The crate has `--blocks`
   copies of a short block of physics: build a length, a duration and a
   mass, divide to a speed and, in the `kinetic` variant, compute 0.5·m·v²
   as an energy (with `formula!` in qty, plain operators in uom). qty's
   `operators` variant computes it with plain operators too. The `speed`
   variant leaves out the energy. Incremental compilation is off,
   so every rebuild compiles the whole crate. `scripts/scaling.py` does
   the same for more patterns and sizes, and times the programs too.

Each measurement runs in its own throwaway Cargo project under
`target/compile-time/`, against this checkout of qty and uom from crates.io.
Times are wall clock; the report gives the median and the fastest of
`--runs` runs. Run it on an otherwise idle machine:

    python3 scripts/compile_time.py
    python3 scripts/compile_time.py --runs 5 --blocks 400 --json results.json
    python3 scripts/compile_time.py --only downstream --blocks 50 --runs 1

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

QTY_PRELUDE = "use qty::*;\n"
QTY_BLOCK = {
    "speed": (
        "{{ let l = Length::from_meters(a + {i}.0); "
        "let t = Duration::from_seconds(a * {i}.0 + 1.0); "
        "let v: Speed = l / t; "
        "acc += v.as_meters_per_second(); }}\n"
    ),
    "kinetic": (
        "{{ let l = Length::from_meters(a + {i}.0); "
        "let t = Duration::from_seconds(a * {i}.0 + 1.0); "
        "let m = Mass::from_kilograms(a); "
        "let v: Speed = l / t; "
        "let e: Energy = formula!(0.5 * m * v * v); "
        "acc += e.as_joules() + v.as_meters_per_second(); }}\n"
    ),
    # The kinetic energy with plain operators, as in uom's block.
    "operators": (
        "{{ let l = Length::from_meters(a + {i}.0); "
        "let t = Duration::from_seconds(a * {i}.0 + 1.0); "
        "let m = Mass::from_kilograms(a); "
        "let v: Speed = l / t; "
        "let e: Energy = 0.5 * m * v * v; "
        "acc += e.as_joules() + v.as_meters_per_second(); }}\n"
    ),
}

UOM_PRELUDE = (
    "use uom::si::f64::*;\n"
    "use uom::si::{energy::joule, length::meter, mass::kilogram, "
    "time::second, velocity::meter_per_second};\n"
)
UOM_BLOCK = {
    "speed": (
        "{{ let l = Length::new::<meter>(a + {i}.0); "
        "let t = Time::new::<second>(a * {i}.0 + 1.0); "
        "let v: Velocity = l / t; "
        "acc += v.get::<meter_per_second>(); }}\n"
    ),
    "kinetic": (
        "{{ let l = Length::new::<meter>(a + {i}.0); "
        "let t = Time::new::<second>(a * {i}.0 + 1.0); "
        "let m = Mass::new::<kilogram>(a); "
        "let v: Velocity = l / t; "
        "let e: Energy = 0.5 * m * v * v; "
        "acc += e.get::<joule>() + v.get::<meter_per_second>(); }}\n"
    ),
}

# Downstream builds: (name, arguments).
DOWNSTREAM_COMMANDS = [
    ("debug", ["build"]),
    ("release", ["build", "--release"]),
]


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.split("\n\n")[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--runs", type=int, default=3, help="runs per measurement (default 3)")
    parser.add_argument(
        "--blocks", type=int, default=400, help="physics blocks in the downstream crate (default 400)"
    )
    parser.add_argument("--uom-version", default="0.38.0", help="uom version to compare with (default 0.38.0)")
    parser.add_argument(
        "--only", choices=["library", "downstream"], help="run one kind of measurement only"
    )
    parser.add_argument("--json", type=Path, help="also write the results to this file")
    args = parser.parse_args()
    if args.runs < 1 or args.blocks < 1:
        parser.error("--runs and --blocks must be at least 1")

    machine = describe_machine()
    print(f"# Compile times, qty against uom {args.uom_version}\n")
    for key, value in machine.items():
        print(f"- {key}: {value}")
    print(f"- runs per measurement: {args.runs}\n")

    results: dict = {"machine": machine, "runs": args.runs, "uom": args.uom_version}
    if args.only in (None, "library"):
        results["library"] = measure_library(args)
    if args.only in (None, "downstream"):
        results["downstream"] = measure_downstream(args)

    if args.json:
        args.json.write_text(json.dumps(results, indent=2) + "\n")
        print(f"\nResults written to {args.json}")
    return 0


def measure_library(args: argparse.Namespace) -> list[dict]:
    """Cold builds of each library configuration."""
    print("## Library, cold build\n")
    print("| Configuration | " + " | ".join(name for name, _ in LIBRARY_COMMANDS) + " |")
    print("| --- |" + " --- |" * len(LIBRARY_COMMANDS))
    rows = []
    for index, (name, _crate, dependency) in enumerate(LIBRARY_CONFIGS):
        project = make_project(f"library-{index}", dependency.format(qty=REPO_ROOT, uom=args.uom_version))
        write_main(project, "fn main() {}\n")
        cargo(project, ["fetch"])
        row = {"configuration": name, "dependency": dependency, "times": {}}
        for command_name, command in LIBRARY_COMMANDS:
            times = []
            for _ in range(args.runs):
                cargo(project, ["clean"])
                times.append(timed(project, command))
            row["times"][command_name] = times
        rows.append(row)
        print(f"| {name} | " + " | ".join(fmt(row["times"][c]) for c, _ in LIBRARY_COMMANDS) + " |")
    print()
    return rows


def measure_downstream(args: argparse.Namespace) -> list[dict]:
    """Rebuilds of a crate using each library, with the library already built."""
    print(f"## Downstream crate, {args.blocks} blocks, rebuild without incremental compilation\n")
    print("| Library | Variant | " + " | ".join(name for name, _ in DOWNSTREAM_COMMANDS) + " |")
    print("| --- | --- |" + " --- |" * len(DOWNSTREAM_COMMANDS))
    libraries = [
        (
            "qty",
            f'qty = {{ path = "{REPO_ROOT}", default-features = false, features = ["std", "full"] }}',
            QTY_PRELUDE,
            QTY_BLOCK,
        ),
        (
            "uom",
            f'uom = {{ version = "{args.uom_version}", default-features = false, features = ["si", "f64", "std"] }}',
            UOM_PRELUDE,
            UOM_BLOCK,
        ),
    ]
    rows = []
    for library, dependency, prelude, blocks in libraries:
        for variant, block in blocks.items():
            project = make_project(f"downstream-{library}-{variant}", dependency)
            source = downstream_source(prelude, block, args.blocks)
            row = {"library": library, "variant": variant, "blocks": args.blocks, "times": {}}
            for command_name, command in DOWNSTREAM_COMMANDS:
                # Build the dependencies (and check that the code compiles).
                write_main(project, source)
                cargo(project, command)
                times = []
                for run in range(args.runs):
                    # A changed file makes Cargo rebuild the crate, and only it.
                    write_main(project, source + f"// run {run} {time.time_ns()}\n")
                    times.append(timed(project, command))
                row["times"][command_name] = times
            rows.append(row)
            print(
                f"| {library} | {variant} | "
                + " | ".join(fmt(row["times"][c]) for c, _ in DOWNSTREAM_COMMANDS)
                + " |"
            )
    print()
    return rows


def downstream_source(prelude: str, block: str, count: int) -> str:
    body = "".join(block.format(i=i) for i in range(count))
    return (
        "#![allow(unused)]\n"
        + prelude
        + "\n#[inline(never)]\nfn work(a: f64) -> f64 {\n    let mut acc = 0.0;\n"
        + body
        + "    acc\n}\n\n"
        + 'fn main() {\n    println!("{}", work(std::env::args().count() as f64));\n}\n'
    )


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


def env() -> dict:
    environment = dict(os.environ)
    environment["CARGO_INCREMENTAL"] = "0"
    environment.pop("RUSTFLAGS", None)
    return environment


def cargo(project: Path, arguments: list[str]) -> None:
    result = subprocess.run(
        ["cargo", *arguments, "--quiet"],
        cwd=project,
        env=env(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"`cargo {' '.join(arguments)}` failed in {project}:\n{result.stderr}")


def timed(project: Path, arguments: list[str]) -> float:
    start = time.perf_counter()
    cargo(project, arguments)
    return time.perf_counter() - start


def fmt(times: list[float]) -> str:
    median = statistics.median(times)
    if len(times) == 1:
        return f"{median:.1f} s"
    return f"{median:.1f} s (min {min(times):.1f})"


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
