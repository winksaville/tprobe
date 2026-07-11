# tprobe — founding rationale & consolidation plan

This crate exists to end a four-way copy-paste. A tick-counter
measurement probe was vendored independently into four repos
(actor-x1, zc-ring-x1, iiac-perf, and prospectively zc-msg-x1)
and drifted in each. This note is the founding record: why we
are consolidating those copies into one crate, no_std-first, and
what that crate should be. It is the writeup of the design
discussion that created this repo.

## The problem: four divergent copies, none no_std

Four independent copies of the tick-probe exist, drifted apart —
not just vendored duplicates but three different scope decisions:

- **actor-x1** (`crates/tprobe`, v0.1.5) — the richest:
  `tprobe.rs` (~312 lines) folds `overhead.rs`
  (apparatus-overhead calibration) and `pin.rs` (CPU pinning)
  *into* the probe crate. Its manifest says "vendored from
  iiac-perf". This repo is where the probe was first created.
- **zc-ring-x1** (`tprobe` v0.1.0 + sibling `tp_runner`) — split
  the other way: a bare `tprobe` crate (probe + `ticks` +
  `band_table` + `tprobe_span`) and a separate `tp_runner` crate
  holding the drive loop, pinning, and Linux `perf_event_open`
  counters. `tprobe.rs` is ~110 lines.
- **iiac-perf** (`src/tprobe.rs` + `src/tprobe2.rs` +
  `src/band_table.rs`, ~442 lines total, in the 0.20.0 binary) —
  a minimal probe core with the full calibrated harness built
  around it in the same crate.
- **zc-msg-x1** — would have been a fifth copy if its seeding
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
nanosecond-scale, cross-core, tail-latency *distribution* of a
lock-free hot path, measurable on the target:

- **criterion / divan** — host-std statistical microbench
  harnesses. Good for A/B + throughput at the *mean*; they do not
  surface deep tail bands (the n5..n10 / p.999_999_999 rows these
  measurements are about — batch-timing discards the
  distribution). Both are `std`, so neither runs on an embedded
  target. divan is additionally stale (last release April 2025).
- **iai / gungraun (callgrind)** — instruction-count / cache
  *simulation*. Noise-free and great for CI regression guards,
  but they model a single simulated run and cannot reproduce the
  real cross-core cache-line-transfer economics that is the whole
  point (the 26–40% SPSC-vs-MPSC gap zc-ring measured is a
  hardware effect) [[2]].
- **hyperfine** — whole-process wall-clock timing; wrong
  granularity for ns round-trips. **bencher** — a CI tracking
  layer, orthogonal to how the numbers are produced.

So for *host-side* CI regression gating an off-the-shelf harness
would work, but iiac-perf already covers host-side, making a
second one largely redundant; and for *on-target* embedded
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
- **Spans** — the span-style API exists in some copies
  (`tprobe_span.rs` in zc-ring, `tprobe2.rs` in iiac-perf) and
  not in actor-x1; fold in as an optional part of the core, same
  `no_std` discipline.

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
- **iiac-perf** — active; today it *forks* rather than depends
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
- Is iiac-perf's fork deliberate? Decides whether consolidation
  is three-repo (actor-x1 + zc-msg + iiac-perf depend) or two.
- Where do pinning and `perf_event_open` live — in the `std`
  feature of this crate, or kept in a separate runner/harness
  crate as zc-ring split them?

# References

[1]: https://nnethercote.github.io/perf-book/benchmarking.html
[2]: https://github.com/winksaville/zc-ring-x1
