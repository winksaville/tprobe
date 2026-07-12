//! Recorders — where the collection phase puts each delta.
//!
//! The recorder is the pluggable half of the probe (see the
//! "Phases" section of `notes/design.md`): the hot path does a
//! tick read, a delta, and one [`Recorder::record`] call;
//! analysis and presentation are deferred off the measured path.
//!
//! - [`ArrayRecorder`] — preallocated raw-delta array, for
//!   benchmarks (bounded runs with a natural "after").
//! - `RingRecorder` / `HistogramRecorder` land later in the
//!   ladder.

/// Sink for measured tick deltas — the pluggable half of
/// [`crate::TProbe`]. Static dispatch for now; the trait-shape
/// question (generic vs enum) is an open question in
/// `notes/design.md`.
pub trait Recorder {
    /// Record one tick delta.
    ///
    /// Must be cheap and infallible by construction — overflow
    /// policy is the implementor's (drop-and-count, overwrite,
    /// saturate), never a panic and never a `Result` on the hot
    /// path.
    fn record(&mut self, delta: u64);
}

/// Preallocated raw-delta recorder: one sequential store per
/// event, drop-and-count when full.
///
/// - `B` is any `[u64]`-backed storage — `&mut [u64]`, an owned
///   array, or (with an allocator) a `Vec<u64>` / boxed slice —
///   so an owning recorder can move across threads with its
///   storage (e.g. returned via a `JoinHandle`).
/// - No work beyond the store happens at record time; analyze
///   after the run via [`Self::deltas`].
/// - Full buffer: the delta is dropped and counted in
///   [`Self::dropped`], never overwritten — a benchmark wants to
///   know its buffer was undersized, not silently lose the head.
pub struct ArrayRecorder<B> {
    buf: B,
    len: usize,
    dropped: u64,
}

impl<B: AsRef<[u64]> + AsMut<[u64]>> ArrayRecorder<B> {
    /// Wrap preallocated storage; capacity is `buf`'s length.
    pub fn new(buf: B) -> Self {
        Self {
            buf,
            len: 0,
            dropped: 0,
        }
    }

    /// The recorded deltas, in record order.
    pub fn deltas(&self) -> &[u64] {
        &self.buf.as_ref()[..self.len]
    }

    /// Storage capacity, in deltas.
    pub fn capacity(&self) -> usize {
        self.buf.as_ref().len()
    }

    /// Number of deltas dropped because the buffer was full.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

impl<B: AsRef<[u64]> + AsMut<[u64]>> Recorder for ArrayRecorder<B> {
    #[inline(always)]
    fn record(&mut self, delta: u64) {
        let buf = self.buf.as_mut();
        if self.len < buf.len() {
            buf[self.len] = delta;
            self.len += 1;
        } else {
            self.dropped += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_in_order_up_to_capacity() {
        let mut r = ArrayRecorder::new([0u64; 3]);
        r.record(10);
        r.record(20);
        assert_eq!(r.deltas(), &[10, 20]);
        assert_eq!(r.capacity(), 3);
        assert_eq!(r.dropped(), 0);
    }

    #[test]
    fn full_buffer_drops_and_counts() {
        let mut r = ArrayRecorder::new([0u64; 2]);
        r.record(1);
        r.record(2);
        r.record(3);
        r.record(4);
        assert_eq!(r.deltas(), &[1, 2]);
        assert_eq!(r.dropped(), 2);
    }

    #[test]
    fn borrowed_slice_storage_works() {
        let mut buf = [0u64; 4];
        let mut r = ArrayRecorder::new(&mut buf[..]);
        r.record(7);
        assert_eq!(r.deltas(), &[7]);
    }

    #[cfg(feature = "std")]
    #[test]
    fn vec_storage_works() {
        let mut r = ArrayRecorder::new(vec![0u64; 4]);
        r.record(5);
        r.record(6);
        assert_eq!(r.deltas(), &[5, 6]);
    }
}
