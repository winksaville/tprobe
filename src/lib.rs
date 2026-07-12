//! tprobe — a `no_std`-first hardware tick-counter measurement
//! probe.
//!
//! Reads a hardware tick counter and records deltas through a
//! pluggable [`Recorder`] (collection phase); analysis and
//! presentation happen off the measured path — see the "Phases"
//! section of `notes/design.md`:
//!
//! - [`ticks`] — per-arch tick counter (`rdtsc`,
//!   `CNTVCT_EL0`); reads are `no_std`, calibration and
//!   validation are `std`-gated
//! - [`Recorder`] / [`ArrayRecorder`] — where deltas go;
//!   no-alloc, embeddable on-target
//! - [`TProbe`] — the collection front end
//!   (`start()` … `record()`)
//!
//! Founding rationale and design live in `notes/design.md`. The
//! core is built out over the `0.1.0` ladder; `examples/tp_pc`
//! is the parity port of iiac-perf's `tp-pc` bench.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod probe;
pub mod recorder;
pub mod ticks;

pub use probe::TProbe;
pub use recorder::{ArrayRecorder, Recorder};
