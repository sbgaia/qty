//! Compares qty with other unit crates on a few basic operations:
//!
//! 1. `construction_from_km`: build a length from kilometres.
//! 2. `convert_km_to_m`: read a length in metres.
//! 3. `add_two_lengths`: add two lengths in the same unit.
//! 4. `chain`: build one length from km and one from m, add them, and read
//!    the sum in metres.
//! 5. `sum_10k_lengths`: sum 10,000 lengths, with bare `f64` as baseline.
//! 6. `scale_10k_lengths`: scale 10,000 lengths in place, to see whether the
//!    wrapper gets in the way of vectorisation.
//!
//! Run with `cargo bench` in `benches/`. The HTML report ends up in
//! `benches/target/criterion/report/index.html`.
//!
//! qty, uom, simple-si-units and measurements store metres, so they multiply
//! when building from km and just read when asked for metres. ferrunitas and
//! measures-rs store the unit they were given and multiply when converting.
//! In `chain` everyone does one multiplication and one addition.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// qty stores metres: `from_kilometers` multiplies by 1000 and `as_meters`
// reads the field.

#[inline(always)]
fn qty_from_km(km: f64) -> qty::Length {
    qty::Length::from_kilometers(km)
}

#[inline(always)]
fn qty_to_m(l: qty::Length) -> f64 {
    l.as_meters()
}

#[inline(always)]
fn qty_add(a: qty::Length, b: qty::Length) -> qty::Length {
    a + b
}

#[inline(always)]
fn qty_chain(km: f64, m: f64) -> f64 {
    (qty::Length::from_kilometers(km) + qty::Length::from_meters(m)).as_meters()
}

// uom checks dimensions with typenum parameters and also stores metres.

use uom::si::f64::Length as UomLength;
use uom::si::length::{kilometer as uom_km, meter as uom_m};

#[inline(always)]
fn uom_from_km(km: f64) -> UomLength {
    UomLength::new::<uom_km>(km)
}

#[inline(always)]
fn uom_to_m(l: UomLength) -> f64 {
    l.get::<uom_m>()
}

#[inline(always)]
fn uom_add(a: UomLength, b: UomLength) -> UomLength {
    a + b
}

#[inline(always)]
fn uom_chain(km: f64, m: f64) -> f64 {
    (UomLength::new::<uom_km>(km) + UomLength::new::<uom_m>(m)).get::<uom_m>()
}

// simple-si-units: `Distance<V>` stores metres in its `m` field.

use simple_si_units::base::Distance;

#[inline(always)]
fn simple_from_km(km: f64) -> Distance<f64> {
    Distance::<f64>::from_km(km)
}

#[inline(always)]
fn simple_to_m(d: &Distance<f64>) -> f64 {
    d.to_m()
}

#[inline(always)]
fn simple_add(a: Distance<f64>, b: Distance<f64>) -> Distance<f64> {
    a + b
}

#[inline(always)]
fn simple_chain(km: f64, m: f64) -> f64 {
    (Distance::<f64>::from_km(km) + Distance::<f64>::from_m(m)).to_m()
}

// measurements stores metres in an f64; `as_meters` takes `&self`.

use measurements::Length as MeasLength;

#[inline(always)]
fn meas_from_km(km: f64) -> MeasLength {
    MeasLength::from_kilometers(km)
}

#[inline(always)]
fn meas_to_m(l: &MeasLength) -> f64 {
    l.as_meters()
}

#[inline(always)]
fn meas_add(a: MeasLength, b: MeasLength) -> MeasLength {
    a + b
}

#[inline(always)]
fn meas_chain(km: f64, m: f64) -> f64 {
    (MeasLength::from_kilometers(km) + MeasLength::from_meters(m)).as_meters()
}

// ferrunitas stores the value in the unit it was given and converts on
// `.convert::<Metre>()`. Adding two units converts the right side to the left
// side's unit.

use ferrunitas::system::{Kilometre as FerrKm, Metre as FerrM};
use ferrunitas::Unit as _;

#[inline(always)]
fn ferr_from_km(km: f64) -> ferrunitas::Measure<FerrKm> {
    FerrKm::new(km)
}

#[inline(always)]
fn ferr_to_m(l: &ferrunitas::Measure<FerrKm>) -> f64 {
    l.convert::<FerrM>().value()
}

#[inline(always)]
fn ferr_add(
    a: ferrunitas::Measure<FerrKm>,
    b: ferrunitas::Measure<FerrKm>,
) -> ferrunitas::Measure<FerrKm> {
    a + b
}

#[inline(always)]
fn ferr_chain(km: f64, m: f64) -> f64 {
    // The sum is in kilometres, the left side's unit.
    (FerrKm::new(km) + FerrM::new(m)).convert::<FerrM>().value()
}

// measures-rs generates its types with `define_measure_types!`. Like
// ferrunitas it stores the given unit, and `+` needs both sides in the same
// unit.

mod msr {
    // `exact`: plain f64, no uncertainty.
    measures::define_measure_types! {
        exact,
        scalar_properties [
            Length [
                Metre {
                    suffix: " m",
                }
                Kilometre {
                    suffix: " km",
                    ratio: 1000.,
                }
            ]
        ]
    }
}

#[inline(always)]
fn msr_from_km(km: f64) -> msr::Measure<msr::Kilometre> {
    msr::Measure::<msr::Kilometre>::new(km)
}

#[inline(always)]
fn msr_to_m(l: msr::Measure<msr::Kilometre>) -> f64 {
    l.convert::<msr::Metre>().value
}

#[inline(always)]
fn msr_add(
    a: msr::Measure<msr::Kilometre>,
    b: msr::Measure<msr::Kilometre>,
) -> msr::Measure<msr::Kilometre> {
    a + b
}

#[inline(always)]
fn msr_chain(km: f64, m: f64) -> f64 {
    // `+` needs the same unit on both sides, so convert to metres first.
    let a: msr::Measure<msr::Metre> =
        msr::Measure::<msr::Kilometre>::new(km).convert::<msr::Metre>();
    let b = msr::Measure::<msr::Metre>::new(m);
    (a + b).value
}

/// Builds a length from kilometres. Crates that store metres multiply here;
/// ferrunitas and measures-rs only wrap the value.
fn bench_construction(c: &mut Criterion) {
    let km = black_box(1.5_f64);
    let mut g = c.benchmark_group("construction_from_km");
    g.bench_function("qty", |b| b.iter(|| qty_from_km(km)));
    g.bench_function("uom", |b| b.iter(|| uom_from_km(km)));
    g.bench_function("simple-si-units", |b| b.iter(|| simple_from_km(km)));
    g.bench_function("measurements", |b| b.iter(|| meas_from_km(km)));
    g.bench_function("ferrunitas", |b| b.iter(|| ferr_from_km(km)));
    g.bench_function("measures-rs", |b| b.iter(|| msr_from_km(km)));
    g.finish();
}

/// Reads a length built from kilometres in metres. This is where ferrunitas
/// and measures-rs multiply.
fn bench_conversion(c: &mut Criterion) {
    let km = 1.5_f64;
    let mut g = c.benchmark_group("convert_km_to_m");
    let u = qty_from_km(km);
    let q = uom_from_km(km);
    let s = simple_from_km(km);
    let me = meas_from_km(km);
    let fe = ferr_from_km(km);
    let mr = msr_from_km(km);

    g.bench_function("qty", |b| b.iter(|| qty_to_m(black_box(u))));
    g.bench_function("uom", |b| b.iter(|| uom_to_m(black_box(q))));
    g.bench_function("simple-si-units", |b| b.iter(|| simple_to_m(black_box(&s))));
    g.bench_function("measurements", |b| b.iter(|| meas_to_m(black_box(&me))));
    g.bench_function("ferrunitas", |b| b.iter(|| ferr_to_m(black_box(&fe))));
    g.bench_function("measures-rs", |b| b.iter(|| msr_to_m(black_box(mr))));
    g.finish();
}

/// Adds two lengths in the same unit. The operands are built outside the
/// timed loop.
fn bench_add(c: &mut Criterion) {
    let (km1, km2) = (1.5_f64, 2.3_f64);
    let mut g = c.benchmark_group("add_two_lengths");

    let u_a = qty_from_km(km1);
    let u_b = qty_from_km(km2);
    let q_a = uom_from_km(km1);
    let q_b = uom_from_km(km2);
    let s_a = simple_from_km(km1);
    let s_b = simple_from_km(km2);
    let m_a = meas_from_km(km1);
    let m_b = meas_from_km(km2);
    let f_a = ferr_from_km(km1);
    let f_b = ferr_from_km(km2);
    let r_a = msr_from_km(km1);
    let r_b = msr_from_km(km2);

    g.bench_function("qty", |b| {
        b.iter(|| qty_add(black_box(u_a), black_box(u_b)))
    });
    g.bench_function("uom", |b| {
        b.iter(|| uom_add(black_box(q_a), black_box(q_b)))
    });
    g.bench_function("simple-si-units", |b| {
        b.iter(|| simple_add(black_box(s_a), black_box(s_b)))
    });
    g.bench_function("measurements", |b| {
        b.iter(|| meas_add(black_box(m_a), black_box(m_b)))
    });
    g.bench_function("ferrunitas", |b| {
        b.iter(|| ferr_add(black_box(f_a), black_box(f_b)))
    });
    g.bench_function("measures-rs", |b| {
        b.iter(|| msr_add(black_box(r_a), black_box(r_b)))
    });
    g.finish();
}

/// Builds a length from km and one from m, adds them and reads metres. Every
/// crate does one multiplication and one addition here.
fn bench_chain(c: &mut Criterion) {
    let (km, m) = (black_box(1.5_f64), black_box(300.0_f64));
    let mut g = c.benchmark_group("chain_construct_add_convert");
    g.bench_function("qty", |b| b.iter(|| qty_chain(km, m)));
    g.bench_function("uom", |b| b.iter(|| uom_chain(km, m)));
    g.bench_function("simple-si-units", |b| b.iter(|| simple_chain(km, m)));
    g.bench_function("measurements", |b| b.iter(|| meas_chain(km, m)));
    g.bench_function("ferrunitas", |b| b.iter(|| ferr_chain(km, m)));
    g.bench_function("measures-rs", |b| b.iter(|| msr_chain(km, m)));
    g.finish();
}

/// Number of values in the bulk benchmarks.
const N: usize = 10_000;

/// Builds `N` lengths in metres with each library from the same raw values.
fn raw_values() -> Vec<f64> {
    (0..N).map(|i| 0.5 + i as f64 * 1e-3).collect()
}

/// Sums `N` lengths in order. A float fold can't be vectorised, so this
/// shows whether `+` stays a single add inside a loop.
fn bench_sum(c: &mut Criterion) {
    let raw = raw_values();
    let u: Vec<qty::Length> = raw.iter().map(|&m| qty::Length::from_meters(m)).collect();
    let q: Vec<UomLength> = raw.iter().map(|&m| UomLength::new::<uom_m>(m)).collect();
    let s: Vec<Distance<f64>> = raw.iter().map(|&m| Distance::<f64>::from_m(m)).collect();
    let me: Vec<MeasLength> = raw.iter().map(|&m| MeasLength::from_meters(m)).collect();

    let mut g = c.benchmark_group("sum_10k_lengths");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| black_box(&raw).iter().fold(0.0, |acc, &x| acc + x))
    });
    g.bench_function("qty", |b| {
        b.iter(|| {
            black_box(&u)
                .iter()
                .fold(qty::Length::default(), |acc, &x| acc + x)
        })
    });
    g.bench_function("uom", |b| {
        b.iter(|| {
            black_box(&q)
                .iter()
                .fold(UomLength::new::<uom_m>(0.0), |acc, &x| acc + x)
        })
    });
    g.bench_function("simple-si-units", |b| {
        b.iter(|| {
            black_box(&s)
                .iter()
                .fold(Distance::<f64>::from_m(0.0), |acc, &x| acc + x)
        })
    });
    g.bench_function("measurements", |b| {
        b.iter(|| {
            black_box(&me)
                .iter()
                .fold(MeasLength::from_meters(0.0), |acc, &x| acc + x)
        })
    });
    g.finish();
}

/// Scales `N` lengths in place. Each element is independent, so a wrapper
/// the optimiser can see through should give the same SIMD loop as `f64`.
/// Written as `*x = *x * k` because not every crate has `*=`.
#[allow(clippy::assign_op_pattern)]
fn bench_scale(c: &mut Criterion) {
    let raw = raw_values();
    let mut r = raw.clone();
    let mut u: Vec<qty::Length> = raw.iter().map(|&m| qty::Length::from_meters(m)).collect();
    let mut q: Vec<UomLength> = raw.iter().map(|&m| UomLength::new::<uom_m>(m)).collect();
    let mut s: Vec<Distance<f64>> = raw.iter().map(|&m| Distance::<f64>::from_m(m)).collect();
    let mut me: Vec<MeasLength> = raw.iter().map(|&m| MeasLength::from_meters(m)).collect();
    // `k` is 1.0 behind `black_box`, so the values stay the same between
    // iterations but the work can't be skipped.
    let k = black_box(1.0_f64);

    let mut g = c.benchmark_group("scale_10k_lengths");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| black_box(&mut r).iter_mut().for_each(|x| *x = *x * k))
    });
    g.bench_function("qty", |b| {
        b.iter(|| black_box(&mut u).iter_mut().for_each(|x| *x = *x * k))
    });
    g.bench_function("uom", |b| {
        b.iter(|| black_box(&mut q).iter_mut().for_each(|x| *x = *x * k))
    });
    g.bench_function("simple-si-units", |b| {
        b.iter(|| black_box(&mut s).iter_mut().for_each(|x| *x = *x * k))
    });
    g.bench_function("measurements", |b| {
        b.iter(|| black_box(&mut me).iter_mut().for_each(|x| *x = *x * k))
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_construction,
    bench_conversion,
    bench_add,
    bench_chain,
    bench_sum,
    bench_scale
);
criterion_main!(benches);
