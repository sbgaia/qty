# /// script
# requires-python = ">=3.9"
# ///
"""Compare qty with uom on generated programs of increasing size.

For each usage pattern and each size, the script writes the same program
three times, with qty, with uom and with plain `f64` numbers, and measures:

1. Compile time: `cargo check`, a debug build and a release build of the
   program, with its dependencies already built. Incremental compilation is
   off, so every build compiles the whole program.
2. Run time: the release build calls every function of the program in a
   loop and reports the median time per call. The `f64` program is the
   floor: a units library should match it.

The program has `size` functions, a hundred per module, each with one block
of the pattern. The patterns:

- `arith`: chains of operators between named quantities (speed, force,
  energy, power).
- `formula`: formulas whose intermediate steps have no named type, with
  `formula!` in qty and plain operators in uom.
- `operators`: the same formulas with plain operators in qty too.
- `literal`: quantities written with a unit, `qty!(12.5 km)` in qty and
  `Length::new::<kilometer>(12.5)` in uom.
- `convert`: conversions between non-SI units (miles to feet, bar to kPa,
  °F to °C, ...).
- `types`: six of forty quantity types per function, so that the program
  uses many types.
- `generic`: a generic function per block, called with three quantity
  types.
- `serde`: a struct of quantities per block, with `Serialize` and
  `Deserialize` derived, written to JSON and read back.
- `mixed`: `arith`, `formula`, `literal` and `convert` in each function.
- `everyday`: `arith`, `operators`, `literal` and `convert` in each
  function: the code most programs write, with plain operators.

Both libraries are built with the same features: every quantity, `f64`
storage, `std`, and serde for the `serde` pattern only. That is qty with
`default-features = false, features = ["std", "full"]` (all 128 quantities)
and uom with `default-features = false, features = ["si", "f64", "std"]`
(all 116 SI quantities, no `f32`, no `autoconvert`, which qty has no
counterpart for). The programs check that the three versions compute the
same numbers.

Each library gets its own target directory under `target/scaling/`, shared
by its patterns, so its dependencies build once. Times are wall clock; the
report gives the median of `--runs` runs. Run it on an otherwise idle
machine:

    python3 scripts/scaling.py
    python3 scripts/scaling.py --sizes 10,100,1000,5000 --runs 3 --json scaling.json
    python3 scripts/scaling.py --patterns arith,formula --sizes 100 --no-run

A full run with the default sizes takes about an hour on four cores, most
of it in uom's release builds. It needs network access the first time, to
fetch uom.
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
from string import Template

REPO_ROOT = Path(__file__).resolve().parent.parent
WORK_DIR = REPO_ROOT / "target" / "scaling"
FUNCTIONS_PER_MODULE = 100

# Cargo invocations: (name, arguments).
COMMANDS = [
    ("check", ["check"]),
    ("debug", ["build"]),
    ("release", ["build", "--release"]),
]

LIBRARIES = ["qty", "uom", "f64"]

# Templates use `$i` for the function's index and `$c` for a constant that
# differs between functions, so that no two functions are the same. The
# function's arguments are `a` and `b`, both `f64`. A body ends with an
# `f64` expression, the function's result.

ARITH = {
    "qty": """\
let d = Length::from_meters(a + $c);
let t = Duration::from_seconds(b + 1.0);
let v: Speed = d / t;
let acc: Acceleration = v / t;
let m = Mass::from_kilograms(a * 0.5 + $c);
let f: Force = m * acc;
let e: Energy = f * d;
let p: Power = e / t;
let total: Energy = p * t + e;
total.as_joules() + v.as_meters_per_second() + p.as_watts()""",
    "uom": """\
let d = Length::new::<length::meter>(a + $c);
let t = Time::new::<time::second>(b + 1.0);
let v: Velocity = d / t;
let acc: Acceleration = v / t;
let m = Mass::new::<mass::kilogram>(a * 0.5 + $c);
let f: Force = m * acc;
let e: Energy = f * d;
let p: Power = e / t;
let total: Energy = p * t + e;
total.get::<energy::joule>() + v.get::<velocity::meter_per_second>() + p.get::<power::watt>()""",
    "f64": """\
let d = a + $c;
let t = b + 1.0;
let v = d / t;
let acc = v / t;
let m = a * 0.5 + $c;
let f = m * acc;
let e = f * d;
let p = e / t;
let total = p * t + e;
total + v + p""",
}

FORMULA = {
    "qty": """\
let m = Mass::from_kilograms(a + $c);
let v = Speed::from_meters_per_second(b);
let r = Length::from_meters(a * 2.0 + 1.0);
let t = Duration::from_seconds(b + $c);
let e: Energy = formula!(0.5 * m * v * v);
let f: Force = formula!(G * m * m / (r * r));
let p: Power = formula!(m * v * v / t);
let x: Length = formula!(v * t + 0.5 * (v / t) * t * t);
e.as_joules() + f.as_newtons() + p.as_watts() + x.as_meters()""",
    "uom": """\
let m = Mass::new::<mass::kilogram>(a + $c);
let v = Velocity::new::<velocity::meter_per_second>(b);
let r = Length::new::<length::meter>(a * 2.0 + 1.0);
let t = Time::new::<time::second>(b + $c);
let g = Force::new::<force::newton>(6.674_30e-11) * Area::new::<area::square_meter>(1.0)
    / (Mass::new::<mass::kilogram>(1.0) * Mass::new::<mass::kilogram>(1.0));
let e: Energy = 0.5 * m * v * v;
let f: Force = g * m * m / (r * r);
let p: Power = m * v * v / t;
let x: Length = v * t + 0.5 * (v / t) * t * t;
e.get::<energy::joule>() + f.get::<force::newton>() + p.get::<power::watt>() + x.get::<length::meter>()""",
    "f64": """\
let m = a + $c;
let v = b;
let r = a * 2.0 + 1.0;
let t = b + $c;
let e = 0.5 * m * v * v;
let f = 6.674_30e-11 * m * m / (r * r);
let p = m * v * v / t;
let x = v * t + 0.5 * (v / t) * t * t;
e + f + p + x""",
}

# The formulas of `FORMULA` with plain operators: the same code as uom's.
OPERATORS = {
    "qty": """\
let m = Mass::from_kilograms(a + $c);
let v = Speed::from_meters_per_second(b);
let r = Length::from_meters(a * 2.0 + 1.0);
let t = Duration::from_seconds(b + $c);
let e: Energy = 0.5 * m * v * v;
let f: Force = G * m * m / (r * r);
let p: Power = m * v * v / t;
let x: Length = v * t + 0.5 * (v / t) * t * t;
e.as_joules() + f.as_newtons() + p.as_watts() + x.as_meters()""",
    "uom": FORMULA["uom"],
    "f64": FORMULA["f64"],
}

LITERAL = {
    "qty": """\
let g: Acceleration = qty!(9.81 m/s^2);
let d: Length = qty!(12.5 km);
let d = d * a + qty!(250 m);
let t: Duration = qty!(1.5 h);
let t = t * b + qty!(30 s);
let m: Mass = qty!(70 kg);
let m = m * a;
let p: Pressure = qty!(2.4 bar);
let dp: Pressure = qty!(15 kPa);
let p = p + dp * b;
let v: Speed = d / t;
let f: Force = m * g;
v.as_meters_per_second() + f.as_newtons() + p.as_pascals() + $c""",
    "uom": """\
let g = Acceleration::new::<acceleration::meter_per_second_squared>(9.81);
let d = Length::new::<length::kilometer>(12.5) * a + Length::new::<length::meter>(250.0);
let t = Time::new::<time::hour>(1.5) * b + Time::new::<time::second>(30.0);
let m = Mass::new::<mass::kilogram>(70.0) * a;
let p = Pressure::new::<pressure::bar>(2.4) + Pressure::new::<pressure::kilopascal>(15.0) * b;
let v: Velocity = d / t;
let f: Force = m * g;
v.get::<velocity::meter_per_second>() + f.get::<force::newton>() + p.get::<pressure::pascal>() + $c""",
    "f64": """\
let g = 9.81;
let d = 12_500.0 * a + 250.0;
let t = 5400.0 * b + 30.0;
let m = 70.0 * a;
let p = 240_000.0 + 15_000.0 * b;
let v = d / t;
let f = m * g;
v + f + p + $c""",
}

CONVERT = {
    "qty": """\
let x = a + $c;
Length::from_miles(x).as_feet()
    + Length::from_inches(b).as_centimeters()
    + Mass::from_pounds(x).as_grams()
    + Duration::from_hours(b).as_minutes()
    + Speed::from_kilometers_per_hour(x).as_miles_per_hour()
    + Pressure::from_bars(x).as_kilopascals()
    + Energy::from_kilowatt_hours(b).as_megajoules()
    + Power::from_kilowatts(x).as_watts()
    + Temperature::from_degrees_fahrenheit(x).as_degrees_celsius()
    + Volume::from_liters(b).as_cubic_centimeters()""",
    "uom": """\
let x = a + $c;
Length::new::<length::mile>(x).get::<length::foot>()
    + Length::new::<length::inch>(b).get::<length::centimeter>()
    + Mass::new::<mass::pound>(x).get::<mass::gram>()
    + Time::new::<time::hour>(b).get::<time::minute>()
    + Velocity::new::<velocity::kilometer_per_hour>(x).get::<velocity::mile_per_hour>()
    + Pressure::new::<pressure::bar>(x).get::<pressure::kilopascal>()
    + Energy::new::<energy::kilowatt_hour>(b).get::<energy::megajoule>()
    + Power::new::<power::kilowatt>(x).get::<power::watt>()
    + ThermodynamicTemperature::new::<thermodynamic_temperature::degree_fahrenheit>(x)
        .get::<thermodynamic_temperature::degree_celsius>()
    + Volume::new::<volume::liter>(b).get::<volume::cubic_centimeter>()""",
    "f64": """\
let x = a + $c;
x * 1609.344 / 0.3048
    + b * 0.0254 / 0.01
    + x * 0.45359237 / 0.001
    + b * 3600.0 / 60.0
    + x * (1000.0 / 3600.0) / 0.44704
    + x * 100_000.0 / 1000.0
    + b * 3.6e6 / 1e6
    + x * 1000.0
    + ((x + 459.67) * 5.0 / 9.0 - 273.15)
    + b * 0.001 / 1e-6""",
}

# (qty type, qty unit method suffix, uom type, uom module::unit), all in the
# base unit except qty's mass flow, whose base unit is the gram per second.
TYPES = [
    ("Length", "meters", "Length", "length::meter"),
    ("Mass", "kilograms", "Mass", "mass::kilogram"),
    ("Duration", "seconds", "Time", "time::second"),
    ("Speed", "meters_per_second", "Velocity", "velocity::meter_per_second"),
    ("Acceleration", "meters_per_second_squared", "Acceleration", "acceleration::meter_per_second_squared"),
    ("Force", "newtons", "Force", "force::newton"),
    ("Energy", "joules", "Energy", "energy::joule"),
    ("Power", "watts", "Power", "power::watt"),
    ("Pressure", "pascals", "Pressure", "pressure::pascal"),
    ("Area", "square_meters", "Area", "area::square_meter"),
    ("Volume", "cubic_meters", "Volume", "volume::cubic_meter"),
    ("Density", "kilograms_per_cubic_meter", "MassDensity", "mass_density::kilogram_per_cubic_meter"),
    ("Frequency", "hertz", "Frequency", "frequency::hertz"),
    ("ElectricCurrent", "amperes", "ElectricCurrent", "electric_current::ampere"),
    ("ElectricPotential", "volts", "ElectricPotential", "electric_potential::volt"),
    ("ElectricCharge", "coulombs", "ElectricCharge", "electric_charge::coulomb"),
    ("ElectricResistance", "ohms", "ElectricalResistance", "electrical_resistance::ohm"),
    ("ElectricCapacitance", "farads", "Capacitance", "capacitance::farad"),
    ("ElectricInductance", "henries", "Inductance", "inductance::henry"),
    ("ElectricConductance", "siemens", "ElectricalConductance", "electrical_conductance::siemens"),
    ("ElectricField", "volts_per_meter", "ElectricField", "electric_field::volt_per_meter"),
    ("MagneticField", "teslas", "MagneticFluxDensity", "magnetic_flux_density::tesla"),
    ("MagneticFlux", "webers", "MagneticFlux", "magnetic_flux::weber"),
    ("Torque", "newton_meters", "Torque", "torque::newton_meter"),
    ("RotationalSpeed", "radians_per_second", "AngularVelocity", "angular_velocity::radian_per_second"),
    ("Angle", "radians", "Angle", "angle::radian"),
    ("SolidAngle", "steradians", "SolidAngle", "solid_angle::steradian"),
    ("VolumeFlow", "cubic_meters_per_second", "VolumeRate", "volume_rate::cubic_meter_per_second"),
    ("MassFlow", "kilograms_per_second", "MassRate", "mass_rate::kilogram_per_second"),
    ("Impulse", "newton_seconds", "Momentum", "momentum::kilogram_meter_per_second"),
    ("Jerk", "meters_per_second_cubed", "Jerk", "jerk::meter_per_second_cubed"),
    ("DynamicViscosity", "newton_seconds_per_meter_squared", "DynamicViscosity", "dynamic_viscosity::pascal_second"),
    ("KinematicViscosity", "square_meters_per_second", "KinematicViscosity", "kinematic_viscosity::square_meter_per_second"),
    ("HeatFlux", "watts_per_square_meter", "HeatFluxDensity", "heat_flux_density::watt_per_square_meter"),
    ("ThermalConductivity", "watts_per_meter_kelvin", "ThermalConductivity", "thermal_conductivity::watt_per_meter_kelvin"),
    ("TemperatureDelta", "kelvins", "TemperatureInterval", "temperature_interval::kelvin"),
    ("AmountOfSubstance", "moles", "AmountOfSubstance", "amount_of_substance::mole"),
    ("MolarMass", "kilograms_per_mole", "MolarMass", "molar_mass::kilogram_per_mole"),
    ("Illuminance", "lux", "Illuminance", "illuminance::lux"),
    ("LuminousIntensity", "candela", "LuminousIntensity", "luminous_intensity::candela"),
]
TYPES_PER_FUNCTION = 6

GENERIC_ITEMS = {
    "qty": """\
fn smooth$i<Q>(xs: [Q; 4], w: f64) -> Q
where
    Q: Copy + Add<Output = Q> + Sub<Output = Q> + Mul<f64, Output = Q>,
{
    let mut acc = xs[0];
    for &x in &xs[1..] {
        acc = acc + (x - acc) * w;
    }
    acc
}
""",
}
GENERIC_ITEMS["uom"] = GENERIC_ITEMS["qty"]
GENERIC_ITEMS["f64"] = GENERIC_ITEMS["qty"]

GENERIC = {
    "qty": """\
let l = smooth$i([Length::from_meters(a), Length::from_meters(b), Length::from_meters($c), Length::from_meters(a * b)], 0.25);
let t = smooth$i([Duration::from_seconds(b), Duration::from_seconds(a + 1.0), Duration::from_seconds($c), Duration::from_seconds(2.0)], 0.5);
let e = smooth$i([Energy::from_joules(a), Energy::from_joules($c), Energy::from_joules(b), Energy::from_joules(a + b)], 0.75);
let v: Speed = l / t;
v.as_meters_per_second() + e.as_joules()""",
    "uom": """\
let l = smooth$i([Length::new::<length::meter>(a), Length::new::<length::meter>(b), Length::new::<length::meter>($c), Length::new::<length::meter>(a * b)], 0.25);
let t = smooth$i([Time::new::<time::second>(b), Time::new::<time::second>(a + 1.0), Time::new::<time::second>($c), Time::new::<time::second>(2.0)], 0.5);
let e = smooth$i([Energy::new::<energy::joule>(a), Energy::new::<energy::joule>($c), Energy::new::<energy::joule>(b), Energy::new::<energy::joule>(a + b)], 0.75);
let v: Velocity = l / t;
v.get::<velocity::meter_per_second>() + e.get::<energy::joule>()""",
    "f64": """\
let l = smooth$i([a, b, $c, a * b], 0.25);
let t = smooth$i([b, a + 1.0, $c, 2.0], 0.5);
let e = smooth$i([a, $c, b, a + b], 0.75);
let v = l / t;
v + e""",
}

SERDE_ITEMS = {
    "qty": """\
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Record$i {
    pub distance: Length,
    pub mass: Mass,
    pub elapsed: Duration,
    pub speed: Speed,
    pub energy: Energy,
}
""",
    "uom": """\
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Record$i {
    pub distance: Length,
    pub mass: Mass,
    pub elapsed: Time,
    pub speed: Velocity,
    pub energy: Energy,
}
""",
    "f64": """\
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Record$i {
    pub distance: f64,
    pub mass: f64,
    pub elapsed: f64,
    pub speed: f64,
    pub energy: f64,
}
""",
}

SERDE = {
    "qty": """\
let record = Record$i {
    distance: Length::from_meters(a + $c),
    mass: Mass::from_kilograms(b),
    elapsed: Duration::from_seconds(a + b),
    speed: Speed::from_meters_per_second(a * b),
    energy: Energy::from_joules($c),
};
let json = serde_json::to_string(&record).unwrap();
let back: Record$i = serde_json::from_str(&json).unwrap();
back.distance.as_meters() + back.energy.as_joules() + back.speed.as_meters_per_second()""",
    "uom": """\
let record = Record$i {
    distance: Length::new::<length::meter>(a + $c),
    mass: Mass::new::<mass::kilogram>(b),
    elapsed: Time::new::<time::second>(a + b),
    speed: Velocity::new::<velocity::meter_per_second>(a * b),
    energy: Energy::new::<energy::joule>($c),
};
let json = serde_json::to_string(&record).unwrap();
let back: Record$i = serde_json::from_str(&json).unwrap();
back.distance.get::<length::meter>() + back.energy.get::<energy::joule>() + back.speed.get::<velocity::meter_per_second>()""",
    "f64": """\
let record = Record$i { distance: a + $c, mass: b, elapsed: a + b, speed: a * b, energy: $c };
let json = serde_json::to_string(&record).unwrap();
let back: Record$i = serde_json::from_str(&json).unwrap();
back.distance + back.energy + back.speed""",
}

PRELUDE = {
    "qty": "use qty::*;\nuse qty::constants::GRAVITATIONAL_CONSTANT as G;\n",
    "uom": "use uom::si::f64::*;\nuse uom::si::*;\n",
    "f64": "",
}
SERDE_PRELUDE = "use serde::{Deserialize, Serialize};\n"
OPS_PRELUDE = "use std::ops::{Add, Mul, Sub};\n"


def types_body(library: str, index: int) -> str:
    """Six quantity types, starting at a different one in each function."""
    terms = []
    for k in range(TYPES_PER_FUNCTION):
        qty_type, method, uom_type, uom_unit = TYPES[(index * TYPES_PER_FUNCTION + k) % len(TYPES)]
        if library == "qty":
            new, get = f"{qty_type}::from_{method}", f".as_{method}()"
            ratio = "x / y"
        elif library == "uom":
            new, get = f"{uom_type}::new::<{uom_unit}>", f".get::<{uom_unit}>()"
            ratio = "(x / y).value"
        else:
            new, get, ratio = "", "", "x / y"
        terms.append(
            f"{{ let x = {new}(a + {k}.0); let y = {new}(b + $c); "
            f"(x + y * 2.0 - x / 4.0){get} + {ratio} }}"
        )
    # In parentheses: a statement can't start with a block followed by `+`.
    return "(" + "\n    + ".join(terms) + ")"


# Pattern name -> (function body per library, items per library or None,
# whether it needs serde).
PATTERNS = {
    "arith": (ARITH, None, False),
    "formula": (FORMULA, None, False),
    "operators": (OPERATORS, None, False),
    "literal": (LITERAL, None, False),
    "convert": (CONVERT, None, False),
    "types": (None, None, False),
    "generic": (GENERIC, GENERIC_ITEMS, False),
    "serde": (SERDE, SERDE_ITEMS, True),
    "mixed": (None, None, False),
    "everyday": (None, None, False),
}

# Patterns that put several others in each function.
COMBINED = {
    "mixed": [ARITH, FORMULA, LITERAL, CONVERT],
    "everyday": [ARITH, OPERATORS, LITERAL, CONVERT],
}


def body(pattern: str, library: str, index: int) -> str:
    if pattern == "types":
        template = types_body(library, index)
    elif pattern in COMBINED:
        parts = COMBINED[pattern]
        template = "(" + "\n    + ".join("{\n" + part[library] + "\n}" for part in parts) + ")"
    else:
        template = PATTERNS[pattern][0][library]
    return Template(template).substitute(i=index, c=f"{1.0 + index * 1e-3!r}")


def program(pattern: str, library: str, size: int) -> str:
    """The generated `main.rs`."""
    _, items, serde = PATTERNS[pattern]
    head = "#![allow(unused, clippy::all)]\n" + PRELUDE[library]
    if serde:
        head += SERDE_PRELUDE
    if pattern == "generic":
        head += OPS_PRELUDE
    modules = []
    for start in range(0, size, FUNCTIONS_PER_MODULE):
        functions = []
        for index in range(start, min(size, start + FUNCTIONS_PER_MODULE)):
            if items:
                functions.append(Template(items[library]).substitute(i=index))
            code = body(pattern, library, index).replace("\n", "\n    ")
            functions.append(f"pub fn f{index}(a: f64, b: f64) -> f64 {{\n    {code}\n}}\n")
        module = "\n".join(functions).replace("\n", "\n    ").rstrip()
        modules.append(f"mod m{start // FUNCTIONS_PER_MODULE} {{\n    use super::*;\n\n    {module}\n}}\n")
    table = ",\n    ".join(f"m{i // FUNCTIONS_PER_MODULE}::f{i}" for i in range(size))
    return (
        head
        + "\n"
        + "\n".join(modules)
        + f"\nstatic FUNCTIONS: [fn(f64, f64) -> f64; {size}] = [\n    {table},\n];\n"
        + MAIN
    )


# Prints a checksum, then with an argument times the calls: as many rounds
# over all the functions as fit in 0.2 s, seven times, and the median.
MAIN = """
fn inputs(k: usize) -> (f64, f64) {
    (1.0 + k as f64 * 1e-3, 2.0 + (k % 7) as f64)
}

fn main() {
    let checksum: f64 = FUNCTIONS.iter().enumerate().map(|(k, f)| { let (a, b) = inputs(k); f(a, b) }).sum();
    println!("checksum {checksum:e}");
    if std::env::args().count() < 2 {
        return;
    }
    let round = || {
        let mut acc = 0.0;
        for (k, f) in FUNCTIONS.iter().enumerate() {
            let (a, b) = inputs(k);
            acc += std::hint::black_box(f)(std::hint::black_box(a), std::hint::black_box(b));
        }
        std::hint::black_box(acc);
    };
    let mut rounds = 1;
    loop {
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            round();
        }
        if start.elapsed().as_secs_f64() > 0.05 {
            break;
        }
        rounds *= 2;
    }
    let rounds = rounds * 4;
    let mut samples: Vec<f64> = (0..7)
        .map(|_| {
            let start = std::time::Instant::now();
            for _ in 0..rounds {
                round();
            }
            start.elapsed().as_secs_f64() * 1e9 / (rounds * FUNCTIONS.len()) as f64
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    println!("ns_per_call {}", samples[3]);
}
"""


def dependencies(library: str, serde: bool, uom_version: str) -> str:
    lines = []
    if library == "qty":
        features = '"std", "full"' + (', "serde"' if serde else "")
        lines.append(f'qty = {{ path = "{REPO_ROOT}", default-features = false, features = [{features}] }}')
    elif library == "uom":
        features = '"si", "f64", "std"' + (', "use_serde"' if serde else "")
        lines.append(f'uom = {{ version = "{uom_version}", default-features = false, features = [{features}] }}')
    if serde:
        lines.append('serde = { version = "1", features = ["derive"] }')
        lines.append('serde_json = "1"')
    return "\n".join(lines)


def make_project(library: str, pattern: str, uom_version: str) -> Path:
    """A binary crate for one library and pattern, in its own workspace."""
    name = f"{library}-{pattern}"
    project = WORK_DIR / name
    shutil.rmtree(project, ignore_errors=True)
    (project / "src").mkdir(parents=True)
    # The empty `[workspace]` keeps Cargo from treating it as part of qty's
    # workspace, which it sits inside.
    (project / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n\n'
        f"[dependencies]\n{dependencies(library, PATTERNS[pattern][2], uom_version)}\n\n[workspace]\n"
    )
    return project


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.split("\n\n")[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--sizes", default="10,100,1000", help="functions per program, comma-separated (default 10,100,1000)"
    )
    parser.add_argument(
        "--patterns", default=",".join(PATTERNS), help=f"comma-separated, from {', '.join(PATTERNS)} (default all)"
    )
    parser.add_argument(
        "--libraries", default=",".join(LIBRARIES), help="comma-separated, from qty, uom, f64 (default all)"
    )
    parser.add_argument("--runs", type=int, default=1, help="runs per measurement (default 1)")
    parser.add_argument("--uom-version", default="0.38.0", help="uom version to compare with (default 0.38.0)")
    parser.add_argument("--no-run", action="store_true", help="measure compile times only")
    parser.add_argument(
        "--skip-after",
        type=float,
        default=600.0,
        help="skip larger sizes of a library and pattern once a build takes longer, in seconds (default 600)",
    )
    parser.add_argument("--json", type=Path, help="also write the results to this file")
    parser.add_argument("--keep", action="store_true", help="keep the generated projects in target/scaling/")
    args = parser.parse_args()

    sizes = sorted(int(s) for s in args.sizes.split(","))
    patterns = args.patterns.split(",")
    libraries = args.libraries.split(",")
    for pattern in patterns:
        if pattern not in PATTERNS:
            parser.error(f"unknown pattern {pattern!r}")
    for library in libraries:
        if library not in LIBRARIES:
            parser.error(f"unknown library {library!r}")
    if args.runs < 1 or min(sizes) < 1:
        parser.error("--runs and --sizes must be at least 1")

    machine = describe_machine()
    print(f"# Generated programs, qty against uom {args.uom_version}\n")
    for key, value in machine.items():
        print(f"- {key}: {value}")
    print(f"- runs per measurement: {args.runs}\n")

    results: dict = {
        "machine": machine,
        "runs": args.runs,
        "uom": args.uom_version,
        "dependencies": {
            library: dependencies(library, False, args.uom_version) for library in libraries
        },
        "patterns": {},
    }
    for pattern in patterns:
        results["patterns"][pattern] = measure_pattern(pattern, libraries, sizes, args)
        if args.json:
            args.json.write_text(json.dumps(results, indent=2) + "\n")
    print_summary(results["patterns"], libraries, not args.no_run)
    if args.json:
        print(f"Results written to {args.json}")
    if not args.keep:
        for library in libraries:
            for pattern in patterns:
                shutil.rmtree(WORK_DIR / f"{library}-{pattern}", ignore_errors=True)
    return 0


def measure_pattern(pattern: str, libraries: list[str], sizes: list[int], args: argparse.Namespace) -> dict:
    """Every size of one pattern, for every library."""
    print(f"## {pattern}\n")
    rows: dict = {library: {} for library in libraries}
    projects = {library: make_project(library, pattern, args.uom_version) for library in libraries}
    skipped: set = set()
    for size in sizes:
        for library in libraries:
            if library in skipped:
                continue
            project = projects[library]
            source = program(pattern, library, size)
            row: dict = {"times": {}}
            for command_name, command in COMMANDS:
                # Build the dependencies (and check that the code compiles).
                write_main(project, source)
                cargo(project, library, command)
                times = []
                for run in range(args.runs):
                    # A changed file makes Cargo rebuild the program, and only it.
                    write_main(project, source + f"// run {run} {time.time_ns()}\n")
                    times.append(timed(project, library, command))
                row["times"][command_name] = times
            binary = target_dir(library) / "release" / project.name
            row["checksum"] = float(run_binary(binary, []).split()[1])
            if not args.no_run:
                row["ns_per_call"] = float(run_binary(binary, ["time"]).split()[-1])
            rows[library][size] = row
            if max(max(t) for t in row["times"].values()) > args.skip_after:
                skipped.add(library)
        print_progress(pattern, size, rows)
    print()
    print_tables(pattern, libraries, sizes, rows, not args.no_run)
    check_checksums(pattern, libraries, sizes, rows)
    return rows


def print_progress(pattern: str, size: int, rows: dict) -> None:
    parts = []
    for library, by_size in rows.items():
        if size in by_size:
            times = by_size[size]["times"]
            parts.append(library + " " + "/".join(f"{statistics.median(times[c]):.1f}" for c, _ in COMMANDS))
    print(f"- {size} functions: " + ", ".join(parts) + " s (check/debug/release)", flush=True)


def print_tables(pattern: str, libraries: list[str], sizes: list[int], rows: dict, runtime: bool) -> None:
    columns = [name for name, _ in COMMANDS] + (["ns/call"] if runtime else [])
    print("| Functions | Library | " + " | ".join(columns) + " |")
    print("| ---: | --- |" + " ---: |" * len(columns))
    for size in sizes:
        for library in libraries:
            row = rows[library].get(size)
            if row is None:
                cells = ["skipped"] * len(columns)
            else:
                cells = [fmt(row["times"][name]) for name, _ in COMMANDS]
                if runtime:
                    cells.append(f"{row['ns_per_call']:.2f}")
            print(f"| {size} | {library} | " + " | ".join(cells) + " |")
    print()


def print_summary(results: dict, libraries: list[str], runtime: bool) -> None:
    """The largest size of every pattern that all libraries reached."""
    print("## Summary, largest size\n")
    columns = [f"{name} {library}" for name, _ in COMMANDS for library in libraries]
    if runtime:
        columns += [f"ns/call {library}" for library in libraries]
    print("| Pattern | Functions | " + " | ".join(columns) + " |")
    print("| --- | ---: |" + " ---: |" * len(columns))
    for pattern, rows in results.items():
        common = set.intersection(*(set(rows[library]) for library in libraries))
        if not common:
            continue
        size = max(common)
        cells = [fmt(rows[library][size]["times"][name]) for name, _ in COMMANDS for library in libraries]
        if runtime:
            cells += [f"{rows[library][size]['ns_per_call']:.2f}" for library in libraries]
        print(f"| {pattern} | {size} | " + " | ".join(cells) + " |")
    print()


def check_checksums(pattern: str, libraries: list[str], sizes: list[int], rows: dict) -> None:
    """The three versions of a program compute the same numbers, up to the
    last digits of conversion factors: uom's pound is 0.4535924 kg rather
    than 0.45359237 kg, for example."""
    for size in sizes:
        sums = {library: rows[library][size]["checksum"] for library in libraries if size in rows[library]}
        values = list(sums.values())
        if values and max(values) - min(values) > 1e-6 * max(abs(v) for v in values):
            print(f"Warning: {pattern} with {size} functions computes different results: {sums}\n")


def write_main(project: Path, source: str) -> None:
    (project / "src" / "main.rs").write_text(source)


def target_dir(library: str) -> Path:
    return WORK_DIR / f"target-{library}"


def env(library: str) -> dict:
    environment = dict(os.environ)
    environment["CARGO_INCREMENTAL"] = "0"
    environment["CARGO_TARGET_DIR"] = str(target_dir(library))
    environment.pop("RUSTFLAGS", None)
    return environment


def cargo(project: Path, library: str, arguments: list[str]) -> None:
    result = subprocess.run(
        ["cargo", *arguments, "--quiet"],
        cwd=project,
        env=env(library),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"`cargo {' '.join(arguments)}` failed in {project}:\n{result.stderr[-5000:]}")


def timed(project: Path, library: str, arguments: list[str]) -> float:
    start = time.perf_counter()
    cargo(project, library, arguments)
    return time.perf_counter() - start


def run_binary(binary: Path, arguments: list[str]) -> str:
    result = subprocess.run([str(binary), *arguments], capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"{binary} failed:\n{result.stderr}")
    return result.stdout.strip().splitlines()[-1]


def fmt(times: list[float]) -> str:
    median = statistics.median(times)
    return f"{median:.2f}" if median < 10 else f"{median:.1f}"


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
