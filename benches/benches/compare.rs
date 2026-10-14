//! Benchmark comparing `unitsnet-rs` against popular Rust unit-conversion
//! libraries across six representative operations:
//!
//! 1. **`construction_from_km`** — build a `Length` value from a kilometres
//!    input (reveals whether the library eagerly converts to SI base at
//!    construction or stores the raw value in the given unit).
//! 2. **`convert_km_to_m`** — extract the numeric value expressed in metres
//!    (the complementary cost to construction).
//! 3. **`add_two_lengths`** — add two pre-built `Length` values of the same
//!    unit (pure f64 arithmetic overhead; operands are hoisted outside the
//!    timed loop so only the `+` itself is measured).
//! 4. **`chain`** — realistic round-trip: construct from km, construct from m,
//!    add them, then read the result as metres.
//! 5. **`sum_10k_lengths`** — fold-sum 10 000 lengths, against bare `f64`.
//! 6. **`scale_10k_lengths`** — scale 10 000 lengths in place, against bare
//!    `f64` (checks that the wrapper does not block SIMD vectorisation).
//!
//! # Running
//!
//! ```bash
//! cargo bench -p unitsnet-benches
//! # HTML report: target/criterion/report/index.html
//! ```
//!
//! # Architectural notes
//!
//! | Library        | Storage strategy | Construction cost | Conversion cost |
//! |----------------|-----------------|-------------------|-----------------|
//! | unitsnet-rs    | SI base (metres) | 1 multiply        | read (free)     |
//! | uom            | SI base          | 1 multiply        | read (free)     |
//! | simple-si-units| SI base          | 1 multiply        | read (free)     |
//! | measurements   | SI base          | 1 multiply        | read (free)     |
//! | ferrunitas     | caller's unit    | free (store only) | 1 multiply      |
//! | measures-rs    | caller's unit    | free (store only) | 1 multiply      |
//!
//! The `chain` benchmark equalises these costs: every library ultimately
//! performs exactly one multiplication plus one addition.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ─── unitsnet-rs ──────────────────────────────────────────────────────────────
// Stores every value in the SI base unit (metres).  `from_kilometers` multiplies
// by 1 000 at construction; `as_meters` is a plain field read.

#[inline(always)]
fn unitsnet_from_km(km: f64) -> unitsnet_rs::Length {
    unitsnet_rs::Length::from_kilometers(km)
}

#[inline(always)]
fn unitsnet_to_m(l: unitsnet_rs::Length) -> f64 {
    l.as_meters()
}

#[inline(always)]
fn unitsnet_add(a: unitsnet_rs::Length, b: unitsnet_rs::Length) -> unitsnet_rs::Length {
    a + b
}

#[inline(always)]
fn unitsnet_chain(km: f64, m: f64) -> f64 {
    (unitsnet_rs::Length::from_kilometers(km) + unitsnet_rs::Length::from_meters(m)).as_meters()
}

// ─── uom ─────────────────────────────────────────────────────────────────────
// Type-parameterised dimensional analysis via typenum.  `uom::si::f64::Length`
// is an alias for `Quantity<ISQ<P1,Z0,Z0,…>, SI<f64>, f64>`.  Internally stores
// in SI base (metres).

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

// ─── simple-si-units ──────────────────────────────────────────────────────────
// `Distance<V>` is generic over the numeric type.  The `m` field stores the
// SI base value; `from_km` multiplies at construction and `to_m` reads directly.

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

// ─── measurements ─────────────────────────────────────────────────────────────
// Struct-based physical quantities backed by a single f64 (SI base).
// `as_meters` takes `&self`; the return value is the raw stored f64.

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

// ─── ferrunitas ───────────────────────────────────────────────────────────────
// Zero-sized unit marker structs.  `Measure<U>` stores the raw value in the
// caller's unit — no conversion at construction.  The ×1000 km→m factor is
// applied lazily on `.convert::<Metre>()` or implicitly during cross-unit `+`.
//
// Addition of different-unit measures of the same dimension is supported:
//   `Measure<U1> + Measure<U2>` → `Measure<U1>`  (converts rhs to lhs unit)
// The value is accessed via `.value()` (the field is `pub(crate)`).

use ferrunitas::system::{Kilometre as FerrKm, Metre as FerrM};
// `Unit::new` is the constructor on all ferrunitas unit marker structs.
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
    // ferrunitas supports cross-unit addition (rhs is automatically converted
    // to the lhs unit before adding).  The result is Measure<Kilometre>.
    // We then call convert::<Metre>() to get the value in metres.
    (FerrKm::new(km) + FerrM::new(m)).convert::<FerrM>().value()
}

// ─── measures-rs ─────────────────────────────────────────────────────────────
// Macro-generated unit system.  A single `define_measure_types!` call emits:
//   - A `Measure<U, Number=f64>` struct with a `pub value: Number` field
//   - Trait impls (MeasurementUnit) for every declared unit
//   - Arithmetic operators for `Measure<U>` (same-unit only — convert first)
//
// Like ferrunitas, values are stored in the caller's unit; `convert::<T>()`
// multiplies by `(source_ratio / dest_ratio)` to switch units.
//
// Note: `add_two_lengths` uses same-unit operands here (both Kilometre), so
// the addition is a plain f64 add after LLVM optimises away the trivial ×/÷ 1
// that appears in the through-Quantity path.

mod msr {
    // `exact` generates non-uncertain (plain f64) Measure types.
    // All unit definitions live inside this one macro call.
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
    // `convert` multiplies by `Kilometre::RATIO / Metre::RATIO` = 1000.
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
    // measures-rs requires same-unit operands for `+`.  Convert km→m first,
    // then add the metre value, then read the public `.value` field.
    let a: msr::Measure<msr::Metre> =
        msr::Measure::<msr::Kilometre>::new(km).convert::<msr::Metre>();
    let b = msr::Measure::<msr::Metre>::new(m);
    (a + b).value
}

// ─── Benchmark groups ─────────────────────────────────────────────────────────

/// Benchmark 1 — construct a `Length` from a kilometres value.
///
/// Libraries that convert to SI base at construction (`unitsnet-rs`, `uom`,
/// `simple-si-units`, `measurements`) perform one f64 multiplication here.
/// Libraries that defer conversion (`ferrunitas`, `measures-rs`) perform only
/// a struct wrap — expect them to be faster in this group.
fn bench_construction(c: &mut Criterion) {
    let km = black_box(1.5_f64);
    let mut g = c.benchmark_group("construction_from_km");
    g.bench_function("unitsnet-rs",    |b| b.iter(|| unitsnet_from_km(km)));
    g.bench_function("uom",            |b| b.iter(|| uom_from_km(km)));
    g.bench_function("simple-si-units",|b| b.iter(|| simple_from_km(km)));
    g.bench_function("measurements",   |b| b.iter(|| meas_from_km(km)));
    g.bench_function("ferrunitas",     |b| b.iter(|| ferr_from_km(km)));
    g.bench_function("measures-rs",    |b| b.iter(|| msr_from_km(km)));
    g.finish();
}

/// Benchmark 2 — extract the metres value from a pre-built `Length` (in km).
///
/// For SI-base libraries this is a plain read (potentially free after inlining).
/// For lazy-convert libraries (`ferrunitas`, `measures-rs`) this does the ×1000
/// multiplication — the complementary cost to the cheap construction above.
fn bench_conversion(c: &mut Criterion) {
    let km = 1.5_f64;
    let mut g = c.benchmark_group("convert_km_to_m");
    // Pre-build outside the timed loop; pass by reference where the API requires it.
    let u = unitsnet_from_km(km);
    let q = uom_from_km(km);
    let s = simple_from_km(km);
    let me = meas_from_km(km);
    let fe = ferr_from_km(km);
    let mr = msr_from_km(km);

    g.bench_function("unitsnet-rs",    |b| b.iter(|| unitsnet_to_m(black_box(u))));
    g.bench_function("uom",            |b| b.iter(|| uom_to_m(black_box(q))));
    g.bench_function("simple-si-units",|b| b.iter(|| simple_to_m(black_box(&s))));
    g.bench_function("measurements",   |b| b.iter(|| meas_to_m(black_box(&me))));
    g.bench_function("ferrunitas",     |b| b.iter(|| ferr_to_m(black_box(&fe))));
    g.bench_function("measures-rs",    |b| b.iter(|| msr_to_m(black_box(mr))));
    g.finish();
}

/// Benchmark 3 — add two `Length` values of the same unit.
///
/// Operands are constructed once outside the timed loop; only the `+` is
/// measured.  Expected to compile down to a single `fadd` for all libraries.
fn bench_add(c: &mut Criterion) {
    let (km1, km2) = (1.5_f64, 2.3_f64);
    let mut g = c.benchmark_group("add_two_lengths");

    let u_a = unitsnet_from_km(km1); let u_b = unitsnet_from_km(km2);
    let q_a = uom_from_km(km1);     let q_b = uom_from_km(km2);
    let s_a = simple_from_km(km1);  let s_b = simple_from_km(km2);
    let m_a = meas_from_km(km1);    let m_b = meas_from_km(km2);
    let f_a = ferr_from_km(km1);    let f_b = ferr_from_km(km2);
    let r_a = msr_from_km(km1);     let r_b = msr_from_km(km2);

    g.bench_function("unitsnet-rs",    |b| b.iter(|| unitsnet_add(black_box(u_a), black_box(u_b))));
    g.bench_function("uom",            |b| b.iter(|| uom_add(black_box(q_a), black_box(q_b))));
    g.bench_function("simple-si-units",|b| b.iter(|| simple_add(black_box(s_a), black_box(s_b))));
    g.bench_function("measurements",   |b| b.iter(|| meas_add(black_box(m_a), black_box(m_b))));
    g.bench_function("ferrunitas",     |b| b.iter(|| ferr_add(black_box(f_a), black_box(f_b))));
    g.bench_function("measures-rs",    |b| b.iter(|| msr_add(black_box(r_a), black_box(r_b))));
    g.finish();
}

/// Benchmark 4 — realistic end-to-end: construct(km) + construct(m) → add →
/// extract as metres.
///
/// Every library performs exactly one f64 multiplication plus one addition
/// along this path (though *when* each operation occurs differs).  Timings
/// should be very close across all libraries.
fn bench_chain(c: &mut Criterion) {
    let (km, m) = (black_box(1.5_f64), black_box(300.0_f64));
    let mut g = c.benchmark_group("chain_construct_add_convert");
    g.bench_function("unitsnet-rs",    |b| b.iter(|| unitsnet_chain(km, m)));
    g.bench_function("uom",            |b| b.iter(|| uom_chain(km, m)));
    g.bench_function("simple-si-units",|b| b.iter(|| simple_chain(km, m)));
    g.bench_function("measurements",   |b| b.iter(|| meas_chain(km, m)));
    g.bench_function("ferrunitas",     |b| b.iter(|| ferr_chain(km, m)));
    g.bench_function("measures-rs",    |b| b.iter(|| msr_chain(km, m)));
    g.finish();
}

// ─── Bulk data ────────────────────────────────────────────────────────────────

/// Number of values in the bulk benchmarks.
const N: usize = 10_000;

/// Builds `N` lengths in metres with each library from the same raw values.
fn raw_values() -> Vec<f64> {
    (0..N).map(|i| 0.5 + i as f64 * 1e-3).collect()
}

/// Benchmark 5 — sum `N` lengths with a left fold of `+`.
///
/// A strict-order float fold cannot be vectorised, so this measures whether a
/// library's `+` stays a bare `addsd` inside a loop. The `f64` entry is the
/// unit-free baseline.
fn bench_sum(c: &mut Criterion) {
    let raw = raw_values();
    let u: Vec<unitsnet_rs::Length> = raw.iter().map(|&m| unitsnet_rs::Length::from_meters(m)).collect();
    let q: Vec<UomLength> = raw.iter().map(|&m| UomLength::new::<uom_m>(m)).collect();
    let s: Vec<Distance<f64>> = raw.iter().map(|&m| Distance::<f64>::from_m(m)).collect();
    let me: Vec<MeasLength> = raw.iter().map(|&m| MeasLength::from_meters(m)).collect();

    let mut g = c.benchmark_group("sum_10k_lengths");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| black_box(&raw).iter().fold(0.0, |acc, &x| acc + x))
    });
    g.bench_function("unitsnet-rs", |b| {
        b.iter(|| black_box(&u).iter().fold(unitsnet_rs::Length::default(), |acc, &x| acc + x))
    });
    g.bench_function("uom", |b| {
        b.iter(|| black_box(&q).iter().fold(UomLength::new::<uom_m>(0.0), |acc, &x| acc + x))
    });
    g.bench_function("simple-si-units", |b| {
        b.iter(|| black_box(&s).iter().fold(Distance::<f64>::from_m(0.0), |acc, &x| acc + x))
    });
    g.bench_function("measurements", |b| {
        b.iter(|| black_box(&me).iter().fold(MeasLength::from_meters(0.0), |acc, &x| acc + x))
    });
    g.finish();
}

/// Benchmark 6 — scale `N` lengths in place by a plain number.
///
/// Independent per-element work: any library whose wrapper is transparent to
/// the optimiser gets the same SIMD loop as bare `f64`.
fn bench_scale(c: &mut Criterion) {
    let raw = raw_values();
    let mut r = raw.clone();
    let mut u: Vec<unitsnet_rs::Length> = raw.iter().map(|&m| unitsnet_rs::Length::from_meters(m)).collect();
    let mut q: Vec<UomLength> = raw.iter().map(|&m| UomLength::new::<uom_m>(m)).collect();
    let mut s: Vec<Distance<f64>> = raw.iter().map(|&m| Distance::<f64>::from_m(m)).collect();
    let mut me: Vec<MeasLength> = raw.iter().map(|&m| MeasLength::from_meters(m)).collect();
    // Multiplying by 1.0 keeps the values stable across iterations without
    // letting the optimiser skip the work (`k` is opaque).
    let k = black_box(1.0_f64);

    let mut g = c.benchmark_group("scale_10k_lengths");
    g.bench_function("f64 (no units)", |b| {
        b.iter(|| black_box(&mut r).iter_mut().for_each(|x| *x = *x * k))
    });
    g.bench_function("unitsnet-rs", |b| {
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
