# tprobe — founding rationale & consolidation plan

This crate exists to end a four-way copy-paste. A tick-counter
measurement probe was vendored independently into four repos
(actor-x1, zc-ring-x1, iiac-perf, and prospectively zc-msg-x1)
and drifted in each. This note is the founding record: why we
are consolidating those copies into one crate, no_std-first, and
what that crate should be. It is the writeup of the design
discussion that created this repo.

## The problem: four divergent copies, none no_std

Three independent copies of the tick-probe exist, drifted apart —
not just vendored duplicates but three different scope decisions:

- **iiac-perf** (`src/tprobe.rs` + `src/tprobe2.rs` +
  `src/band_table.rs`, ~442 lines total, in the 0.20.0 binary) —
  a minimal probe core with the full calibrated harness built
  around it in the same crate and is the original version.
- **actor-x1** (`crates/tprobe`, v0.1.5) — the richest:
  `tprobe.rs` (~312 lines) folds `overhead.rs`
  (apparatus-overhead calibration) and `pin.rs` (CPU pinning)
  _into_ the probe crate. And is based on iiac-perf version.
- **zc-ring-x1** (`tprobe` v0.1.0 + sibling `tp_runner`) — split
  the other way: a bare `tprobe` crate (probe + `ticks` +
  `band_table` + `tprobe_span`) and a separate `tp_runner` crate
  holding the drive loop, pinning, and Linux `perf_event_open`
  counters. `tprobe.rs` is ~110 lines.
- **zc-msg-x1** — would have been a forth copy if its seeding
  plan (path-dep zc-ring-x1's `tprobe`/`tp_runner`) were followed
  as written. Consolidating here is what stops that.

Two facts fall out of comparing them:

- **The duplication is real and drifting.** Three live,
  independently-edited forks — one bundling pin+overhead, one
  splitting them out, one minimal — is the DRY trigger the
  earlier "extract when a second consumer materializes" note was
  waiting for. We think each fork happened because there was no
  shared crate to depend on at the time, so each repo vendored
  and then edited locally.
- **None of the copies is `no_std`.** Every one leans on
  `hdrhistogram` (std), `String` probe names, and std time /
  `minstant`. So today nothing here can measure on an embedded
  target — the copies are all host-only.

## Scope: a probe is not a harness

The existing code already draws a clean line, worth preserving:

- **Probe** (this crate) — read a hardware tick counter, record
  deltas into a named histogram, render a percentile band-table.
  Small, embeddable, ideally `no_std`.
- **Harness** (iiac-perf; zc-ring's `tp_runner`) — adaptive loop
  sizing, apparatus-overhead subtraction, thread pinning, a bench
  registry, `perf_event_open` counters. zc-ring's `tp_runner`
  doc says it plainly: "Deliberately not a benchmark harness …
  for that scale of machinery use iiac-perf."

The consolidation target is the **probe**. The harness layer
(sizing, calibration, registry) stays where it is — iiac-perf
owns host-side characterization and already produces the
published percentile tables.

## Why not an off-the-shelf benchmarking crate

The perf-book benchmarking page [[1]] lists criterion, divan,
hyperfine, iai/gungraun (callgrind), bencher, and rustc-perf.
None replaces the probe for the need that motivates it — a
nanosecond-scale, cross-core, tail-latency _distribution_ of a
lock-free hot path, measurable on the target:

- **criterion / divan** — host-std statistical microbench
  harnesses. Good for A/B + throughput at the _mean_; they do not
  surface deep tail bands (the n5..n10 / p.999_999_999 rows these
  measurements are about — batch-timing discards the
  distribution). Both are `std`, so neither runs on an embedded
  target. divan is additionally stale (last release April 2025).
- **iai / gungraun (callgrind)** — instruction-count / cache
  _simulation_. Noise-free and great for CI regression guards,
  but they model a single simulated run and cannot reproduce the
  real cross-core cache-line-transfer economics that is the whole
  point (the 26–40% SPSC-vs-MPSC gap zc-ring measured is a
  hardware effect) [[2]].
- **hyperfine** — whole-process wall-clock timing; wrong
  granularity for ns round-trips. **bencher** — a CI tracking
  layer, orthogonal to how the numbers are produced.

So for _host-side_ CI regression gating an off-the-shelf harness
would work, but iiac-perf already covers host-side, making a
second one largely redundant; and for _on-target_ embedded
measurement nothing off-the-shelf applies at all. This
home-grown probe is the only thing that can measure on the
target — which is why it is worth investing in rather than
replacing.

## The design: no_std core + std feature layer

This crate should not be a file-move of any one fork (that just
relocates std code and freezes one of three designs). It should
be a redesign toward a `no_std` core with an opt-in `std` layer:

- **`no_std`, no-alloc histogram** — a fixed log-linear bucket
  histogram replacing `hdrhistogram` (std, and heavy for
  embedded). This is the real design nut. We think a small
  fixed-bucket table sized like the existing band scheme is
  enough for the tail bands we report; evaluate whether any
  existing `no_std` histogram crate fits before hand-rolling.
- **`&'static str` names** instead of `String`, so the core
  needs no allocator.
- **Cycle-counter abstraction** — the per-arch `ticks/`
  (`x86_64` `rdtsc`, `aarch64` `CNTVCT_EL0`) is already close to
  `no_std`; put it behind a portable-atomic-style trait so
  CAS-less / counter-less targets can supply their own source.
- **`std` feature** — the rich band-table reporting
  (thousands-separator formatting, tick→ns calibration display),
  and optionally the pinning / `perf_event_open` counters, gated
  so a mainstream host build keeps everything and an embedded
  build gets just the core.
- **Spans** — dissolved, not folded in. The copies split
  TProbe/TProbe2 (`tprobe2.rs` in iiac-perf, `tprobe_span.rs`
  in zc-ring) because one histogrammed inline and one buffered
  records; the Recorder split removes that reason. The span
  API's `site_id` goes with it: TProbe2 kept one buffer and one
  histogram for many sites, so its `site_id` was dead code
  awaiting "per-site grouping" — which a probe per site (the
  `&'static str` name is the site) gives for free. What might
  return later, on the one probe type:
  - a caller-held start token (`start_span() -> Span`,
    `record_span(Span)`) for overlapping scopes on a single
    probe — rare once probes are per-site; add when a consumer
    needs it;
  - time-ordered cross-site tracing — a recorder sample-type
    question, not a probe question. The per-probe sample types
    are now designed (see
    [Sample types](#sample-types-and-the-compression-ladder));
    the cross-site `id` correlation remains future.

## Phases: collection, analysis, presentation

Two use classes shape the crate. In a **benchmark** the
measurement is the work (iiac-perf, the actor-x1 benches). In an
**instrumented application** the probe discovers critical paths
and monitors the app generally — we think this is the majority
use case if the crate succeeds, including embedded targets where
data is never presented on the device. The existing copies fold
analysis into collection (histogram update on the hot path,
rendering in-process); this design separates three phases and
keeps everything but raw collection off the measured path:

- **Collection** — the hot path does a tick read, a delta, and
  one recorder operation; nothing else. The recorder is a
  pluggable trait with three `no_std` impls:
  - `ArrayRecorder` — preallocated delta array, one sequential
    store per event. For benchmarks: bounded iteration counts
    and a natural "after" mean no concurrent thread is needed;
    analysis runs after the loop with zero overlap with any
    measurement.
  - `RingRecorder` — raw deltas into an SPSC ring drained by a
    collector thread. For instrumented apps with spare cores:
    at 10M events/s the raw stream is ~80 MB/s, trivial against
    within-machine memory bandwidth. Shipping buffers between
    threads is zc-ring's domain — a natural composition, though
    transport stays out of this crate.
  - `HistogramRecorder` — in-place log-linear bucket increment.
    For embedded / off-machine links where raw-sample bandwidth
    is infeasible; the device ships fixed-size snapshots
    instead.
- **Analysis** — histogram build and percentile extraction.
  Always off the measured path: after the loop (array), on the
  collector thread (ring), or off-device (histogram snapshots).
- **Presentation** — band-table rendering, formatting, tick→ns
  display. `std`-only, operates on snapshots, never on the
  measuring device.

The organizing insight: **histogramming is compression, not
analysis**. A log-linear histogram compresses the delta
distribution, lossy only in bucket resolution, and it is the
interchange format between phases wherever compression happens.
The recorder choice just moves the compression point — after the
run, on a collector core, or on the device — so analysis and
presentation downstream are identical in all three modes.

Consequences for the core:

- **Snapshot/drain is a first-class operation** — wait-free for
  the recorder (double-buffered counts or an epoch swap), so a
  collector can harvest without blocking the hot path.
- **A small versioned `no_std` wire format** for histogram
  snapshots (bucket-layout parameters + counts) is the boundary
  between this crate and the world.
- **Transport is out of scope** — the probe produces snapshots
  and raw rings; which thread, socket, or UART moves them is
  the application's business.
- Raw recorders retain **time ordering** that histograms
  discard (correlating spikes with events, periodic patterns) —
  a flight-recorder use distinct from distribution measurement,
  and a reason the raw modes are first-class rather than merely
  an optimization.

### Outer loop as cross-check, not measurement

For the benchmark harness (iiac-perf), summing the recorded
inner deltas gives the duration directly, which demotes the
outer-loop timing from "the measurement" to a cross-check worth
keeping:

- `sum(inner deltas)` — time actually spent in the measured op;
  the right numerator for op throughput.
- `outer − sum(inner)` — the per-iteration apparatus cost (tick
  reads, the record store, loop overhead), *measured* for that
  exact run. Today the copies estimate this with a separate
  calibration pass (actor-x1's `overhead.rs`); this makes it a
  free per-run self-check that calibration matches reality.

### First experiment: recorder cost

We think the raw array/ring store is cheaper per event than the
histogram increment (a store vs. leading-zeros + shift + counter
bump), but the histogram's table stays L1-resident while a raw
array streams through cache — for long runs the eviction
pressure could invert the ranking. Comparing recorder impls is
the first experiment the consolidated tprobe should run against
itself, folded into probe-overhead calibration.

The comparison is a two-axis cost matrix, not one ranking:

- **Hot-path cost per event** — what the probed code pays:
  store-to-RAM (array / cycled buffer) vs histogram increment.
  Measured by tprobe against itself — same measured op, swap
  the recorder (a one-line A/B under the trait) — with
  `outer − sum(inner)` self-measuring the apparatus.
- **System cost** — what the machine pays around the hot path:
  collector CPU, memory bandwidth, storage/egress writes.
  "Record to storage" lives here, never in `record()` —
  storage is where the *collector* drains READY buffers to,
  and its cost is a collector-side measurement (with its own
  probe).

A fair A/B needs the recorder dispatch itself to cost
nothing — weight for the generic/monomorphized side of the
trait-shape open question, at least in measurement builds.

## Sample types and the compression ladder

What the recorder keeps per event is its own axis, independent
of where compression happens. Three sample types cover the
uses that have surfaced:

- **Duration** — the landed `start()`…`record()` delta,
  8 B/event. Distribution measurement; discards position in
  time.
- **Timestamp** — a raw tick per point event, via a new probe
  verb `mark()` (one tick read, one store). Inter-arrival and
  ordering measurement; discards nothing. Feeding
  `mark()`-to-`mark()` deltas to a duration recorder instead
  records inter-arrivals — same rung as duration.
- **Timestamp + duration** — `(start_ts, delta)` span
  records, 16 B/event. The flight-recorder form, and the
  general one: it reconstructs both the start-timestamp series
  and the duration series; its byte cost is the only reason
  the cheaper types exist.

These rungs form a compression ladder, and the histogram of
the Phases section is its bottom rung — compression inside
`record()` itself, discarding everything but the distribution.
The two axes compose: any sample stream can be histogrammed,
so a deeply embedded target (no storage, no high-bandwidth
egress) keeps a jitter distribution (histogrammed `mark()`
inter-arrivals) and a processing-time distribution on-device
at fixed size — the original tprobe's
record-directly-to-histogram mode, preserved as
`HistogramRecorder`. Each probe picks its own (sample type ×
compression) cell per deployment.

The motivating flight-recorder case is a 100 Hz control loop,
where one span probe covers three asks with one record:

- **Jitter** — consecutive start timestamps against the 10 ms
  nominal grid; no separate timestamp probe needed.
- **Processing time** — the span duration, same record.
- **Phase breakdown** — intermediate checkpoints inside the
  span: `start()`, `checkpoint()` ×K, `record()` yields a
  record of `start + K checkpoint deltas + end`. `K` is
  const-generic so records stay fixed-size — variable-length
  records would poison the no-alloc buffer format. One atomic
  record per iteration also beats a probe per phase: no
  cross-buffer alignment to reconstruct an iteration, no torn
  iterations at buffer boundaries.
- **Loss visibility** — drop-and-count on a duration series
  says how many samples vanished; timestamps show *which*
  iterations and when — what jitter forensics wants.

At 100 Hz even the fat checkpointed record is ~5 KB/s,
trivial to push asynchronously to storage; the same full form
at 10M events/s is ~160 MB/s — why the ladder is chosen
per-probe, not per-crate. Wire-format consequence: the
snapshot/buffer payload needs a sample-type descriptor
(duration / timestamp / span / span+K), with `K` a layout
parameter just as the bucket layout is for histograms.

## Registry and buffer-cycling handoff

How collected data crosses from the hot path to analysis. The
Phases section left probe discovery implicit ("a collector
thread drains"); the decision is a **registry**: a probe
registers at construction and receives a set of buffers to
fill, and a collector harvests filled buffers through the
registry. One handoff protocol serves both use classes:

- **The protocol is buffer cycling.** A probe fills its
  current buffer with plain stores; when full it marks the
  buffer READY and takes a FREE one; the collector drains
  READY buffers and returns them FREE. Ownership is one atomic
  state per buffer slot (FILLING → READY → DRAINING → FREE).
  "Send the buffer" (queue view) and "set a bit the registry
  notices" (in-place view) are the same ownership transfer —
  in the `no_std` core both are just atomics; how the
  collector learns of READY (polling the bits, or an eventfd /
  condvar wakeup) is a `std`-layer add-on.
- **Benchmark drive** — collection points are well defined, so
  the "collector" is inline: the harness calls collect on the
  registry from its own thread after the loop. With one big
  buffer per probe this degenerates to exactly
  `ArrayRecorder` — the landed recorder is the 1-buffer
  special case.
- **Real-time drive** — collection is asynchronous: a
  collector thread polls the ready bits (or blocks on the std
  wakeup) and histograms off the measured path. Per-event cost
  is unchanged (store + increment); the swap path's few
  atomics amortize once per buffer.
- **Payloads generalize.** A buffer carries any
  [sample type](#sample-types-and-the-compression-ladder) *or*
  a histogram table: the same handoff gives `HistogramRecorder`
  the double-buffered snapshot the Phases section calls for,
  and it reframes `RingRecorder` as a ring of *buffers* rather
  than of deltas. We think the buffer ring is the better
  shape — the collector reads contiguous batches, and it
  matches zc-ring's buffer-shipping domain.
- **The registry is app-visible, not ambient.** The app
  constructs it (or declares it static) and passes it to probe
  construction — within-process brokering only; transport
  stays out of scope, as before.

Design questions to settle in the implementing cycle:

- **Fixed capacity** — no-alloc `no_std` means compile-time
  bounds (max probes, buffers per probe, buffer length): a
  const-generic registry owning all storage, probes claiming
  slots atomically at construction. A growable `std` variant
  can layer on later if ever needed.
- **Backpressure** — FREE exhausted (collector behind) must
  never block or overwrite: drop-and-count, the
  `ArrayRecorder::dropped` philosophy.
- **Partial buffers** — a low-rate probe may never fill a
  buffer, starving the collector of its data. Plan: an
  explicit probe-side flush for defined collection points,
  plus a per-slot flush-request flag the probe checks with one
  relaxed load. Collector-side *stealing* of a partial buffer
  would break the single-writer rule — rejected.
- **Identity and reclamation** — the registry slot index is
  the buffer's probe id (a legitimate rebirth of the `site_id`
  the Spans bullet dissolved — this time load-bearing, since a
  harvested buffer outlives its fill site); probe drop needs a
  slot-release story so benchmarks can construct probes per
  run.

## Sequencing: this crate before zc-msg-x1's messaging layer

Chosen path (decision "B" in the originating discussion):
consolidate `tprobe` here **before** seeding zc-msg-x1's
messaging layer. Seeding zc-msg on a std copy we intend to
replace is throwaway work, and measurement is first-class enough
across this repo family (embedded/on-target is a stated
requirement, not a nice-to-have) to justify fixing the
foundation first. The rejected alternative was to path-dep
zc-ring's existing std `tprobe` into zc-msg now and defer no_std
— faster to start, but it adds a fifth copy and discards work
later.

## Consumers and migration

This crate should let the live consumers converge; migration is
per-repo and not all at once:

- **actor-x1** — active consumer and the probe's birthplace; a
  prime candidate to drop its vendored copy for this crate.
- **iiac-perf** — active; today it _forks_ rather than depends
  (uses `minstant` and its own band code). Confirm whether that
  fork is deliberate (independent evolution) or incidental before
  assuming it will depend on this crate.
- **zc-ring-x1** — frozen lab notebook; we think we will **not**
  migrate it (leave its `tprobe`/`tp_runner` as the record).
- **zc-msg-x1** — the new consumer; starts on this crate instead
  of a vendored copy.

## Name

Kept **`tprobe`** — already the crate name in all four repos
(recognition carries over), short, and "t" reads as tick/time,
what it counts. The only more-descriptive alternative considered
was `tickprobe`; the added clarity did not seem worth losing the
continuity.

## Open questions

- Which `no_std` histogram: an existing crate or a hand-rolled
  fixed-bucket table? Drives the core's shape.
- Recorder trait shape: generic (static dispatch, monomorphized
  per recorder) vs. an enum? Affects hot-path codegen and the
  API every consumer sees.
- Snapshot wire format: encoding, versioning, and how the
  bucket-layout parameters travel with the counts.
- Is iiac-perf's fork deliberate? Decides whether consolidation
  is three-repo (actor-x1 + zc-msg + iiac-perf depend) or two.
- Where do pinning and `perf_event_open` live — in the `std`
  feature of this crate, or kept in a separate runner/harness
  crate as zc-ring split them?

# References

[1]: https://nnethercote.github.io/perf-book/benchmarking.html
[2]: https://github.com/winksaville/zc-ring-x1
