//! Hardware tick counter abstraction: thin wrapper over the
//! target architecture's fixed-rate monotonic counter.
//!
//! The per-arch impl lives in a child module gated by
//! `#[cfg(target_arch = ...)]`; `x86_64` (`rdtsc`) and
//! `aarch64` (`CNTVCT_EL0`) are implemented today. The design's
//! cycle-counter trait (portable source for CAS-less /
//! counter-less targets) replaces this cfg dispatch later.
//!
//! - [`read_ticks`] — current counter value (`no_std` core).
//! - [`ticks_per_ns`] — calibrated conversion ratio (`std`;
//!   presentation-phase only, never on the hot path).
//! - [`check`] — verify the counter is usable, as a `Result` —
//!   the library never exits the process; a binary decides what
//!   to do with an unusable counter (`std`).

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
use x86_64 as imp;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
use aarch64 as imp;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!(
    "tprobe currently only supports target_arch = \"x86_64\" and \
     \"aarch64\". The planned cycle-counter trait (notes/design.md) \
     will let other targets supply their own tick source."
);

/// Read the current tick counter. Monotonic and fixed-rate.
#[inline(always)]
pub fn read_ticks() -> u64 {
    imp::read_ticks()
}

/// Conversion ratio: counter ticks per nanosecond. Calibrated
/// (x86_64) or read from hardware (aarch64, `CNTFRQ_EL0`).
/// Cached — the first call does the work, so call it eagerly
/// before measuring if the first report shouldn't pay for it.
#[cfg(feature = "std")]
pub fn ticks_per_ns() -> f64 {
    imp::ticks_per_ns()
}

/// Verify the tick counter is usable for probe measurements.
/// Returns `Err` with a diagnostic if not; the caller decides
/// whether to abort. The checks performed depend on the target
/// architecture.
#[cfg(feature = "std")]
pub fn check() -> Result<(), &'static str> {
    imp::check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_ticks_is_nondecreasing() {
        let mut prev = read_ticks();
        for _ in 0..1000 {
            let now = read_ticks();
            assert!(now >= prev);
            prev = now;
        }
    }

    #[test]
    fn ticks_per_ns_is_positive() {
        assert!(ticks_per_ns() > 0.0);
    }
}
