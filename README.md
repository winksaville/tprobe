# tprobe

A small, `no_std`-first measurement probe: read a hardware tick
counter, record deltas into a named histogram, and render a
percentile band-table. It consolidates four drifted copies of a
tick-probe (actor-x1, zc-ring-x1, iiac-perf, and prospectively
zc-msg-x1) into one crate — a no-alloc core with an opt-in `std`
reporting/pinning layer. Founding rationale and design:
[notes/design.md](notes/design.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
