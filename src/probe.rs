//! The probe — collection-phase front end over a pluggable
//! [`Recorder`].
//!
//! A [`TProbe`] pairs a name with a recorder and owns the
//! start-tick state, so a measured scope is
//! `probe.start()` … `probe.record()` with no tick locals or
//! delta math at the call site:
//!
//! - `start` reads the tick counter and stores it in the probe.
//! - `record` reads it again and hands
//!   `end.wrapping_sub(start)` to the recorder.
//! - One outstanding scope per probe — `start` overwrites; for
//!   interleaved scopes see the span API planned in
//!   `notes/design.md`.
//!
//! Not `Sync`; cross-thread *sharing* is out of scope. Moving a
//! probe between threads (e.g. returned via a `JoinHandle`) is
//! fine when its recorder is `Send`.

use crate::recorder::Recorder;
use crate::ticks;

/// A named probe: start-tick state plus the [`Recorder`] the
/// measured deltas go to. `&'static str` name — the `no_std`
/// core has no allocator (see `notes/design.md`).
pub struct TProbe<R> {
    name: &'static str,
    start_tick: u64,
    recorder: R,
}

impl<R: Recorder> TProbe<R> {
    /// Pair a name with a recorder. No tick-source validation
    /// happens here — a host binary should call
    /// [`crate::ticks::check`] (and eagerly
    /// [`crate::ticks::ticks_per_ns`]) once at startup.
    pub fn new(name: &'static str, recorder: R) -> Self {
        Self {
            name,
            start_tick: 0,
            recorder,
        }
    }

    /// Begin a measured scope: read the tick counter and store
    /// it in the probe. Overwrites any prior unfinished scope.
    #[inline(always)]
    pub fn start(&mut self) {
        self.start_tick = ticks::read_ticks();
    }

    /// End the scope begun by [`Self::start`]: read the tick
    /// counter and record `end.wrapping_sub(start)`.
    #[inline(always)]
    pub fn record(&mut self) {
        let end = ticks::read_ticks();
        self.recorder.record(end.wrapping_sub(self.start_tick));
    }

    /// The probe's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Borrow the recorder (e.g. to read results in place).
    pub fn recorder(&self) -> &R {
        &self.recorder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::ArrayRecorder;

    #[test]
    fn start_record_appends_one_delta() {
        let mut p = TProbe::new("t", ArrayRecorder::new([0u64; 4]));
        p.start();
        p.record();
        assert_eq!(p.recorder().deltas().len(), 1);
    }

    #[test]
    fn repeated_scopes_accumulate() {
        let mut p = TProbe::new("t", ArrayRecorder::new([0u64; 4]));
        p.start();
        p.record();
        p.start();
        p.record();
        assert_eq!(p.recorder().deltas().len(), 2);
        assert_eq!(p.recorder().dropped(), 0);
        assert_eq!(p.name(), "t");
    }
}
