//! Compares qty with uom and with bare `f64` on the usage patterns of
//! `scripts/scaling.py`, over 10,000 values each:
//!
//! 1. `arith_10k`: a chain of operators between named quantities, from a
//!    length, a duration and a mass to a speed, a force, an energy and a
//!    power.
//! 2. `formula_10k`: formulas whose intermediate steps have no named type
//!    (kinetic energy, gravitation), with `formula!` in qty and plain
//!    operators in uom.
//! 3. `convert_10k`: miles per hour to kilometres per hour, and degrees
//!    Fahrenheit to degrees Celsius.
//! 4. `generic_10k`: exponential smoothing written once as a generic
//!    function, run over lengths.
//!
//! A units library should take as long as `f64` on all of them. Run with
//! `cargo bench -p qty-benches --bench patterns`.

use std::ops::{Add, Mul, Sub};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qty::constants::GRAVITATIONAL_CONSTANT as G;
use qty::{
    formula, Acceleration, Duration, Energy, Force, Length, Mass, Power, Speed, Temperature,
};
use uom::si::f64 as u;
use uom::si::{
    area, energy, force, length, mass, power, thermodynamic_temperature, time, velocity,
};

/// Number of values per benchmark.
const N: usize = 10_000;

/// Raw inputs: distances in metres, durations in seconds, masses in
/// kilograms, all positive.
struct Inputs {
    distance: Vec<f64>,
    duration: Vec<f64>,
    mass: Vec<f64>,
}

fn inputs() -> Inputs {
    Inputs {
        distance: (0..N).map(|i| 1.0 + i as f64 * 1e-3).collect(),
        duration: (0..N).map(|i| 2.0 + (i % 7) as f64).collect(),
        mass: (0..N).map(|i| 0.5 + (i % 13) as f64).collect(),
    }
}

/// The values of `raw` as quantities.
fn wrap<Q>(raw: &[f64], quantity: impl Fn(f64) -> Q) -> Vec<Q> {
    raw.iter().map(|&x| quantity(x)).collect()
}

// 1. Operators between named quantities.

#[inline(always)]
fn arith_f64(d: f64, t: f64, m: f64) -> f64 {
    let v = d / t;
    let acc = v / t;
    let f = m * acc;
    let e = f * d;
    let p = e / t;
    p * t + e + v + p
}

#[inline(always)]
fn arith_qty(d: Length, t: Duration, m: Mass) -> f64 {
    let v: Speed = d / t;
    let acc: Acceleration = v / t;
    let f: Force = m * acc;
    let e: Energy = f * d;
    let p: Power = e / t;
    (p * t + e).as_joules() + v.as_meters_per_second() + p.as_watts()
}

#[inline(always)]
fn arith_uom(d: u::Length, t: u::Time, m: u::Mass) -> f64 {
    let v: u::Velocity = d / t;
    let acc: u::Acceleration = v / t;
    let f: u::Force = m * acc;
    let e: u::Energy = f * d;
    let p: u::Power = e / t;
    (p * t + e).get::<energy::joule>()
        + v.get::<velocity::meter_per_second>()
        + p.get::<power::watt>()
}

// 2. Formulas with unnamed intermediate dimensions. The speed is the
// distance in metres read as metres per second.

#[inline(always)]
fn formula_f64(r: f64, m: f64) -> f64 {
    let v = r;
    0.5 * m * v * v + 6.674_30e-11 * m * m / (r * r)
}

#[inline(always)]
fn formula_qty(r: Length, m: Mass) -> f64 {
    let v = Speed::from_meters_per_second(r.as_meters());
    let e: Energy = formula!(0.5 * m * v * v);
    let f: Force = formula!(G * m * m / (r * r));
    e.as_joules() + f.as_newtons()
}

#[inline(always)]
fn formula_uom(r: u::Length, m: u::Mass) -> f64 {
    let v = u::Velocity::new::<velocity::meter_per_second>(r.get::<length::meter>());
    let g = u::Force::new::<force::newton>(6.674_30e-11) * u::Area::new::<area::square_meter>(1.0)
        / (u::Mass::new::<mass::kilogram>(1.0) * u::Mass::new::<mass::kilogram>(1.0));
    let e: u::Energy = 0.5 * m * v * v;
    let f: u::Force = g * m * m / (r * r);
    e.get::<energy::joule>() + f.get::<force::newton>()
}

// 3. Conversions between non-SI units. The inputs are read as miles per
// hour and degrees Fahrenheit.

#[inline(always)]
fn convert_f64(x: f64) -> f64 {
    x * 0.44704 * 3.6 + ((x + 459.67) * 5.0 / 9.0 - 273.15)
}

#[inline(always)]
fn convert_qty(x: f64) -> f64 {
    Speed::from_miles_per_hour(x).as_kilometers_per_hour()
        + Temperature::from_degrees_fahrenheit(x).as_degrees_celsius()
}

#[inline(always)]
fn convert_uom(x: f64) -> f64 {
    u::Velocity::new::<velocity::mile_per_hour>(x).get::<velocity::kilometer_per_hour>()
        + u::ThermodynamicTemperature::new::<thermodynamic_temperature::degree_fahrenheit>(x)
            .get::<thermodynamic_temperature::degree_celsius>()
}

// 4. One generic function for every library.

#[inline(always)]
fn smooth<Q>(values: &[Q], w: f64) -> Q
where
    Q: Copy + Add<Output = Q> + Sub<Output = Q> + Mul<f64, Output = Q>,
{
    let mut acc = values[0];
    for &x in &values[1..] {
        acc = acc + (x - acc) * w;
    }
    acc
}

fn bench_arith(c: &mut Criterion) {
    let raw = inputs();
    let d = wrap(&raw.distance, Length::from_meters);
    let t = wrap(&raw.duration, Duration::from_seconds);
    let m = wrap(&raw.mass, Mass::from_kilograms);
    let ud = wrap(&raw.distance, u::Length::new::<length::meter>);
    let ut = wrap(&raw.duration, u::Time::new::<time::second>);
    let um = wrap(&raw.mass, u::Mass::new::<mass::kilogram>);

    let mut g = c.benchmark_group("arith_10k");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| {
            let (d, t, m) = black_box((&raw.distance, &raw.duration, &raw.mass));
            (0..N).map(|i| arith_f64(d[i], t[i], m[i])).sum::<f64>()
        })
    });
    g.bench_function("qty", |b| {
        b.iter(|| {
            let (d, t, m) = black_box((&d, &t, &m));
            (0..N).map(|i| arith_qty(d[i], t[i], m[i])).sum::<f64>()
        })
    });
    g.bench_function("uom", |b| {
        b.iter(|| {
            let (d, t, m) = black_box((&ud, &ut, &um));
            (0..N).map(|i| arith_uom(d[i], t[i], m[i])).sum::<f64>()
        })
    });
    g.finish();
}

fn bench_formula(c: &mut Criterion) {
    let raw = inputs();
    let r = wrap(&raw.distance, Length::from_meters);
    let m = wrap(&raw.mass, Mass::from_kilograms);
    let ur = wrap(&raw.distance, u::Length::new::<length::meter>);
    let um = wrap(&raw.mass, u::Mass::new::<mass::kilogram>);

    let mut g = c.benchmark_group("formula_10k");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| {
            let (r, m) = black_box((&raw.distance, &raw.mass));
            (0..N).map(|i| formula_f64(r[i], m[i])).sum::<f64>()
        })
    });
    g.bench_function("qty", |b| {
        b.iter(|| {
            let (r, m) = black_box((&r, &m));
            (0..N).map(|i| formula_qty(r[i], m[i])).sum::<f64>()
        })
    });
    g.bench_function("uom", |b| {
        b.iter(|| {
            let (r, m) = black_box((&ur, &um));
            (0..N).map(|i| formula_uom(r[i], m[i])).sum::<f64>()
        })
    });
    g.finish();
}

fn bench_convert(c: &mut Criterion) {
    let raw = inputs();
    let mut g = c.benchmark_group("convert_10k");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| {
            black_box(&raw.distance)
                .iter()
                .map(|&x| convert_f64(x))
                .sum::<f64>()
        })
    });
    g.bench_function("qty", |b| {
        b.iter(|| {
            black_box(&raw.distance)
                .iter()
                .map(|&x| convert_qty(x))
                .sum::<f64>()
        })
    });
    g.bench_function("uom", |b| {
        b.iter(|| {
            black_box(&raw.distance)
                .iter()
                .map(|&x| convert_uom(x))
                .sum::<f64>()
        })
    });
    g.finish();
}

fn bench_generic(c: &mut Criterion) {
    let raw = inputs();
    let d = wrap(&raw.distance, Length::from_meters);
    let ud = wrap(&raw.distance, u::Length::new::<length::meter>);
    let w = black_box(0.25);

    let mut g = c.benchmark_group("generic_10k");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| smooth(black_box(&raw.distance), w))
    });
    g.bench_function("qty", |b| b.iter(|| smooth(black_box(&d), w).as_meters()));
    g.bench_function("uom", |b| {
        b.iter(|| smooth(black_box(&ud), w).get::<length::meter>())
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_arith,
    bench_formula,
    bench_convert,
    bench_generic
);
criterion_main!(benches);
