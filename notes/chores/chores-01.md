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
core's shape. Data point from the tp_pc comparison (below,
round 3): hdrhistogram-at-collection cost is topology- and
regime-dependent (up to +13% round-trip throughput vs a
cache-resident raw store on a same-CCX pair, nil-to-inverted
cross-CCX or once the raw buffer streams to DRAM), and the
round-3 verdict is that continuous collection wants a
fixed-footprint histogram anyway. The absolute per-record
cost bar for Q1 comes from the still-queued recorder-cost
experiment, not the ping-pong comparison.

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

**Round 2 (superseded — noisy-host baseline only).** Round 3
found the host was measurably contended in rounds 1–2 (the
driving session ran *on* the 3900x) and B's binary was rebuilt
afterward — don't quote this round's A−B numbers ("gap
confirmed", "~80 ns per record"); round 3's controlled 2×2
replaces them. Kept because its A runs are the noisy-host
baseline the round-3 quieting comparison reads against.
Conditions: after iiac-perf dropped `minstant` (calibration
now `std::time::Instant`, matching tprobe), with a 60 s
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

**Round 3 (3900x run-length + core-pair sweep, ssh-driven).**
`pairs` mode, 10 A-B pairs per cell, a 2×2 over run length
{6 s, 60 s} × core pair {4,5 same-CCX; 2,3 cross-CCX}. Same
box as rounds 1–2 — those ran from a local session *on* the
3900x; round 3 drives it from fwlaptop over ssh
([scripts/ssh-3900x.sh](../../scripts/ssh-3900x.sh)) with the
box otherwise idle. Host: 3900x — Zen 2, four 3-core CCXs
(16 MB L3 each; cpu2's L3 group is 0-2,12-14, cpu3's is
3-5,15-17, so the rounds-1–2 pair 2,3 was cross-CCX all
along), SMT siblings n/n+12,
`amd-pstate-epp` + `powersave` + EPP `balance_performance`,
boost on, irqbalance inactive, idle KDE desktop, load ~0.1.
Lab records: `notes/tp-pc-pairs-{6,60}s-c{45,23}-3900x-*.txt`.
Producer trim-mean pair difference A−B (positive = B faster)
and B-vs-A throughput:

| cell            | 6 s                    | 60 s                 |
|-----------------|------------------------|----------------------|
| 4,5 (same CCX)  | +702 ± 269 ns, +13.4%  | +260 ± 52 ns, +4.5%  |
| 2,3 (cross CCX) | +5 ± 165 ns (nil), +1.1% | −166 ± 82 ns, −1.1% |

- **Core-pair topology is first-order.** Same-CCX iterates at
  ~5.3–6.1 µs, cross-CCX at ~7.8–8.0 µs, and the A−B effect
  swings from +13.4% to nil-or-negative across the pairs. We
  think the fabric round-trip both dominates the iteration and
  overlaps (hides) recorder cost that the fast same-CCX loop
  exposes. Record the pair's L3 grouping in every run; never
  compare across pairs.
- **Run length changes the measurand for B, not just the
  noise.** A's trim mean is run-length stable (+38 / +15 ns
  going 6 s→60 s); B slows by +480 / +186 ns. B's raw-delta
  buffer scales with duration — ~10 MB at 6 s (L3-resident),
  ~105 MB at 60 s (DRAM streaming) — while A's histogram stays
  ~656 KB. This is round 2's "large array streaming vs small
  cache-hot buffer" question answered by the sweep: raw-delta
  recording cost grows with run length; histogram-at-collection
  is run-length invariant. Direct input to Q1: per-record cost
  must be compared *within a stated buffer regime*.
- **Repeatability vs run length.** Within-variant sd (trim
  mean, 10 runs): 6 s ≈ 104–203 ns; 60 s ≈ 24–69 ns. So
  6 s × 10 pairs resolves only ≥ ~250 ns effects; 60 s × 10
  resolves ~50 ns. A is ~2× tighter than B at 60 s same-CCX
  (24 vs 46 ns) — consistent with round 2's fresh-big-buffer
  variance suspect. Pair-diff sd ≈ within-variant sd, so
  interleaving is doing its job (no drift term to cancel).
- **Driving the box over ssh measurably quieted it.** A's
  binary is bit-identical across rounds 2 and 3 (built 15:55,
  never rebuilt), same box, same cores 2,3 — so the rounds
  compare directly: trim mean 8,491 → 7,841 ns (−650 ns),
  throughput 115.5 → 125.8 K/s (+8.8%), p99-max mean
  ~16.7 → ~10.5 µs (−37%). Run length can't explain it (A
  moves only +15 ns from 6 s → 60 s within round 3). We think
  the on-box session (bot + desktop activity) was real
  contention in rounds 1–2, and this also explains round 2's
  "unexplained tail wrinkle" above. B is not cross-comparable
  the same way (tp_pc rebuilt at 18:32, after round 2), so
  round 2's cross-CCX A−B numbers shouldn't be read against
  round 3's — the intra-round-3 2×2 is the controlled set.
- **ABBA 300 s ssh-driven replication** (exact round-2
  protocol, cores 2,3; lab record
  [tp-pc-abba-300s-c23-3900x-2026-07-12.txt](../tp-pc-abba-300s-c23-3900x-2026-07-12.txt);
  cap 66 M vs round 2's hand-padded 50 M):
  A 7,802/7,560, B 7,900/7,829 (trim, ns).
  - Confirms the quieting on the same protocol shape: A mean
    8,491 → 7,681 (−810 ns), tails ~16.7 → ~10.3 µs.
  - Retires round 2's "<1 ns repeatability": this A1−A2
    spread is 242 ns (n=2), so the round-2 exact agreement
    was luck, not a property of 300 s runs. The 60 s × 10
    sd (24–69 ns) is the characterized figure.
  - Cross-CCX verdict replicates at 300 s: A−B −184 ns /
    −1.41% (vs −166 ns / −1.13% at 60 s) — B's DRAM-streaming
    penalty saturates once the buffer is far past L3
    (528 MB ≈ 105 MB behavior).
  - Both variants ran ~150 ns faster than the 60 s sweep an
    hour earlier — common-mode session drift. Pair
    differences cancel it (why interleaving matters); raw
    numbers across sweeps carry a ~150 ns offset. Its advantage is conditional on the
  buffer staying cache-resident, its cost grows with run
  length, its run-to-run variance is ~2× the histogram's,
  and `--cap` pre-sizing is an operational footgun (silent
  truncation on an undersized cap). Its role is the short,
  bounded window — where it is both fastest *and* lossless
  (raw deltas support any post-hoc analysis a histogram
  can't recover). Continuous / long-running collection wants
  the fixed-footprint histogram, which strengthens the Q1
  direction and sets its cost bar: beat hdrhistogram's
  ~80 ns/record with O(1) memory. Caveat: measured inside
  the ping-pong loop, where we think peer-wait hides or
  exposes recorder cost by topology; the controlled
  recorder-cost experiment (tight loop, array-size sweep)
  still owns the absolute per-record number.
- **ssh-driven measurement mechanics** (now encoded in
  [scripts/tp-pc-cmp.sh](../../scripts/tp-pc-cmp.sh)):
  - polkit denies `systemd-inhibit` block mode in an ssh
    session, which also kills iiac-perf's default
    self-inhibit re-exec — so A runs `--no-inhibit`
    (symmetric with B, which never inhibited) and sleep
    blocking is the operator's job (`kde-inhibit --power`
    over the session bus works user-level over ssh).
  - a locally killed ssh command leaves the remote sweep
    running; the script now refuses to start while a
    benchmark process is alive (`pgrep` guard).
  - every output file opens with a config header (host, mode,
    secs, cores, cap) — the 6 s files initially couldn't be
    told apart by cores.
  - `--cap` sizing is pair-dependent: 3900x same-CCX hits
    ~198 K samples/s vs cross-CCX ~115–126 K; an undersized
    cap truncates B (count clamps at cap, throughput metric
    lost). Rate estimate now 220 K/s.

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
