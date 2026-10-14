# Fuzzing

Coverage-guided fuzz targets, run with [`cargo fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
on a nightly toolchain:

```sh
cargo install cargo-fuzz
cd fuzz
cargo +nightly fuzz run parse -O    # text parsing: no panics, display/parse round trips
cargo +nightly fuzz run fixed -O    # integer quantities vs exact i128 arithmetic
```

Add `-- -max_total_time=60` to stop after a minute. CI runs each target for a
minute on every push. The same properties run on stable as `tests/properties.rs`
(proptest), as part of `cargo test`.
