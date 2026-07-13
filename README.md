# tprobe

A small, `no_std`-first measurement probe: read a hardware tick
counter, record deltas into a named histogram, and render a
percentile band-table. It consolidates four drifted copies of a
tick-probe (actor-x1, zc-ring-x1, iiac-perf, and prospectively
zc-msg-x1) into one crate — a no-alloc core with an opt-in `std`
reporting/pinning layer. Founding rationale and design:
[notes/design.md](notes/design.md).

## Build & test

Standard cargo:

- `cargo build` — `no_std` core plus the default `std` feature
- `cargo test` — unit tests
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt` —
  the lint + format gate every commit runs

## Run

The crate is a library; the runnable artifact is the `tp_pc`
example — a producer/consumer bench, parity port of
iiac-perf's `tp-pc` (module docs:
[examples/tp_pc/main.rs](examples/tp_pc/main.rs)). Flags spell
the same as `tp-pc` where practical, and every run prints a
`tp_pc <version> — …` banner line identifying the build:

    cargo run --release --example tp_pc -- -d 6 --pin 4,5

- `-d/--duration F` — measured run duration (default 5.0)
- `--pin A,B` — pin producer / consumer to logical CPUs
  (default unpinned; core-pair topology is first-order — see
  the round-3 findings in
  [notes/chores/chores-01.md](notes/chores/chores-01.md))
- `--cap N` — per-thread delta-buffer capacity in samples
  (default 8 Mi)
- `--decimals N` — decimal digits on the report's time
  columns, 0–3 (default 1)
- `-t/--ticks` — report raw hardware ticks instead of ns
- `-V/--version` — print the banner and exit

## Install

To install `tp_pc` as a standalone binary (useful on a
measurement host — a stable path, and the banner's version
identifies the build):

    cargo install --path . --example tp_pc --locked

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
