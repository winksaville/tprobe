//! tprobe — a `no_std`-first hardware tick-counter measurement
//! probe.
//!
//! Reads a hardware tick counter, records deltas into a named
//! histogram, and renders a percentile band-table:
//!
//! - the core is no-alloc and `no_std`, embeddable on-target
//! - the `std` feature (default) adds host-side reporting,
//!   tick→ns calibration display, and optionally pinning /
//!   `perf_event_open` counters
//!
//! Founding rationale and design live in `notes/design.md`. The
//! core is built out over the `0.1.0` ladder; this is the crate
//! skeleton.
#![cfg_attr(not(feature = "std"), no_std)]
