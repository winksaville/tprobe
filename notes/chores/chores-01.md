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
core's shape.

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
