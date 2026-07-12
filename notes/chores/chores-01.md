# Chores-01

Chores-XX files use [Prose form](../../AGENTS.md#prose-form). They
contain discussions and notes on various chores in github compatible
markdown. There is also a [todo.md](../todo.md) file that tracks
tasks and in general there should be a chore section for each task
with the why and how this task will be completed.

## feat: no_std tprobe core

Commits:

[design.md](../design.md) is the founding record; it leaves
three open questions [[1]] that set the crate's shape. This
cycle resolves them — recording each decision in a `###`
subsection below — then stands up the `no_std` core.

- Scope is the **probe** only — read a tick counter, record
  deltas into a named histogram, render a percentile band-table.
  The harness layer (loop sizing, overhead calibration, bench
  registry) stays in iiac-perf.
- Each open question below is a ladder step; the step lands the
  decision and this subsection records the outcome and its
  reasoning.

### Q1 no_std histogram crate vs hand-rolled

The core needs a `no_std`, no-alloc histogram to replace
`hdrhistogram` (std, and heavy for embedded) — the real design
nut. We think a small fixed log-linear bucket table, sized like
the existing band scheme, is enough for the tail bands we report
(n5..n10 / p.999_999_999). Open: adopt an existing `no_std`
histogram crate, or hand-roll the fixed-bucket table. Evaluate
existing crates before hand-rolling; the choice drives the
core's shape. Data point from the tp_pc comparison (below):
hdrhistogram-at-collection costs ~2% round-trip throughput vs
a raw array store, stable across two A-B-B-A rounds (~80 ns
per record by full mean, less by trimmed mean) — whatever wins
Q1 should beat that by a wide margin.

### tp_pc parity comparison: ArrayRecorder vs hdrhistogram

`examples/tp_pc` (tprobe `TProbe` + `ArrayRecorder`, raw-delta
store) vs iiac-perf's `tp-pc` (hdrhistogram record at
collection), identical loop / channels / band-table, on cores
2,3, A = iiac-perf, B = tprobe. Driver:
[scripts/tp-pc-cmp.sh](../../scripts/tp-pc-cmp.sh) — run from
the repo root with an iiac-perf sibling checkout; output tees
to `./tmp/` (ignored), and a run worth keeping is promoted into
`notes/` as a lab record.

**Round 1 (discarded — methodology lesson only).** Four 300 s
runs A-B-B-A with no warmup, `--cap 50M`. Raw output kept as
evidence:
[tp-pc-abba-300s-2026-07-12.txt](../tp-pc-abba-300s-2026-07-12.txt).
Discarded because within-variant drift (A1→A2 = 188 ns on the
trimmed mean, plus a grossly anomalous first-run tail: p99-max
mean 16.8 µs, max 3.3 ms) was as large as the A−B effect. Two
lessons kept: **warm up before measuring** (the first run of a
session is not usable) and **interleave + pair-difference**
(single uninterleaved runs would have been inconclusive). One
mechanism note that stands regardless: in a ping-pong loop each
thread's measured delta includes waiting for its peer, so we
think the peer's recording cost lands *inside* the measured
window — recording cost moves means, not just throughput.

**Round 2** — after iiac-perf dropped `minstant` (calibration
now `std::time::Instant`, matching tprobe) and with a 60 s
discarded warmup ahead of A1. Raw output:
[tp-pc-abba-300s-r2-2026-07-12.txt](../tp-pc-abba-300s-r2-2026-07-12.txt).
Producer-loop summary:

| run | count | mean | mean min-p99 | p99-max mean |
|-----|-------|------|--------------|--------------|
| A1  | 34,653,806 | 8,573 ns | 8,491 ns | 16,917 ns |
| B1  | 35,336,797 | 8,477 ns | 8,425 ns | 13,873 ns |
| B2  | 35,300,513 | 8,486 ns | 8,454 ns | 11,831 ns |
| A2  | 34,679,472 | 8,568 ns | 8,491 ns | 16,419 ns |

- **Metric choice matters.** Full `mean` imports the extreme
  tail we attribute to system noise (A's heavy p99-max bands
  inflate its mean). Cleanest metric is **count/throughput**
  (directly measured, no trimming): B +1.97% / +1.79%.
  Second: `mean min-p99` (trimmed): A−B = 66 / 37 ns. Full
  mean (96 / 82 ns) overstates the gap.
- **Warmup killed the drift.** Within-variant repeatability
  on the trimmed mean went from ~170–190 ns (round 1) to
  **0 ns** (A: 8,491/8,491; consumer 8,560/8,559) and 29 ns
  (B). Warmup + interleave is the protocol going forward.
- **A repeats to <1 ns; B varies by 29 ns.** With ~31 M
  trimmed samples the standard error is ~0.06 ns, so B's
  29 ns run-to-run shift is systematic state, not sampling
  noise — and the platform demonstrably supports <1 ns
  repeatability (A). We think B's fresh 400 MB buffer per run
  (address, THP assembly, page-table layout differing run to
  run) is the leading suspect, vs A's compact ~656 KB
  histogram working set — the "large array" effect showing up
  as run-to-run variance rather than a mean shift. The
  recorder-cost experiment should sweep array size, and cap
  should be sized to need (~36 M), not padded (50 M).
- **Gap confirmed, minstant ruled out.** Equalizing the
  calibration time source did not move the gap — expected,
  since tp-pc's measured loop reads `ticks::read_ticks()`
  directly and never touched minstant. Per-record saving
  ~80 ns by full mean, less by trimmed mean; the controlled
  experiment owns the precise number.
- **Why the percentage is small:** the mpsc round-trip is
  ~8.5 µs, so ~80–90 ns/record is ~1% of the iteration. The
  recorder-cost experiment (tight loop, no channel) will show
  the same absolute cost at tens of percent, and separates the
  open "large array streaming vs small cache-hot buffer"
  question.
- Unexplained tail wrinkle, recorded not attributed: both
  round-2 A runs show heavy p99-max bands (~16.5 µs mean)
  where round-1 A2 was clean (11.8 µs) and B1's max hit
  4.7 ms. We think the extreme tail is dominated by rare
  system events, not the variants.

### Q2 iiac-perf fork deliberate

iiac-perf today *forks* the probe rather than depending on it —
it uses `minstant` and its own band code. Open: is that fork
deliberate (independent evolution) or incidental. Deliberate →
consolidation is two-repo (actor-x1 + zc-msg depend); incidental
→ three-repo (iiac-perf depends too). Confirm before assuming
which shape we are building for.

### Q3 pinning and perf_event_open placement

CPU pinning (`pin.rs`) and Linux `perf_event_open` counters sit
in different places across the copies — folded into the probe in
actor-x1, split into a separate `tp_runner` crate in zc-ring.
Open: put them behind this crate's `std` feature, or keep them
in a separate runner/harness crate as zc-ring split them. We
think the core stays `no_std` either way; this only decides
where the host-only machinery attaches.

# References

[1]: /notes/design.md#open-questions
