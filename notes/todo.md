# Todo

This file uses [Prose form](../AGENTS.md#prose-form). It
contains near term tasks with a short description and
uses links or reference links for more details.

## In Progress

**feat: no_std tprobe core**

[design.md](design.md) settles tprobe's founding rationale and
the collection / analysis / presentation phase split (pluggable
Recorder, histogram-as-compression), but leaves open questions
that gate the crate's shape: which `no_std` histogram, the
recorder trait shape, the snapshot wire format, whether
iiac-perf's fork is deliberate, and where pinning /
`perf_event_open` live. Resolve the gating ones — recording
each decision in a chores design subsection — then seed the
`no_std` core from the existing copies under the new
constraints (no-alloc histogram, `&'static str` names,
cycle-counter trait, recorders, `std`-gated reporting).

- 0.1.0-0 prep: open cycle + chores design subsections for the
  three questions (done)
- 0.1.0-1 docs: no_std core phase split + unwrap lints (done)
- 0.1.0-2 feat: seed no_std core + tp_pc parity example (done)
  - ticks + ArrayRecorder + TProbe; `examples/tp_pc` is a
    parity port of iiac-perf's `tp-pc` (same loop, channels,
    band-table output)
  - also ran the tp_pc comparison vs iiac-perf's `tp-pc`
    (ABBA 300 s runs); cost/benefit findings recorded in a
    chores design subsection
- 0.1.0-3 feat: 3900x run-length sweep + remote protocol (done)
  - 2×2 sweep on the 3900x (10 pairs per cell): run length
    {6 s, 60 s} × core pair {same-CCX, cross-CCX}; findings in
    the round-3 chores block — core-pair topology is
    first-order, and raw-delta recording cost grows with run
    length (B's buffer leaves L3) while histogram recording is
    run-length invariant
  - verdict: ArrayRecorder is a short-capture tool (fastest
    and lossless while cache-resident), not the long-run
    default — continuous collection wants the fixed-footprint
    histogram (Q1)
  - ssh-driving the box (vs the rounds-1–2 on-box session)
    measurably quieted it: identical A binary ran −650 ns
    trim mean, +8.8% throughput, −37% extreme tail; confirmed
    by an ABBA 300 s ssh-driven replication, which also
    retired round 2's "<1 ns repeatability" (luck, n=2)
  - protocol extended for ssh-driven runs: `--no-inhibit`
    symmetry + `kde-inhibit`, straggler guard, config header
    in output files, host-aware `--cap`; `scripts/ssh-3900x.sh`
    added (works from a normal shell and the bot sandbox)
- 0.1.0-4 decide `no_std` histogram — existing crate vs
  hand-rolled fixed-bucket table [[1]]
- 0.1.0-5 confirm iiac-perf fork intent — deliberate (two-repo)
  vs incidental (three-repo consolidation) [[2]]
- 0.1.0-6 place pinning / `perf_event_open` — this crate's `std`
  feature vs a separate runner crate [[3]]
- 0.1.0 close-out and validation

## Todo

 Entries are in **strict priority rank** — #1 highest,
 descending. Reprioritize by moving an entry, then
 `vc-x1 fix-todo --no-dry-run notes/todo.md` to renumber.
 The numbers are positional rank, not stable IDs — to refer
 to a Todo, name it by its **title** (a greppable mention;
 a numbered list item has no anchor to link to), not its
 number. Long-tail entries
 live in [todo-backlog.md](todo-backlog.md). Use the
 [Prose Form in AGENTS.md](../AGENTS.md#prose-form); deeper
 detail goes in `notes/chores/chores-NN.md` design
 subsections (link via `[N]` ref).

## Done

Completed tasks are moved from `## Todo` to here, `## Done`, as they are completed
and older `## Done` sections are moved to [done.md](done.md) to keep this file small.

# References

[1]: chores/chores-01.md#q1-no_std-histogram-crate-vs-hand-rolled
[2]: chores/chores-01.md#q2-iiac-perf-fork-deliberate
[3]: chores/chores-01.md#q3-pinning-and-perf_event_open-placement
