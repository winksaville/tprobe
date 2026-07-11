# Todo

This file uses [Prose form](../AGENTS.md#prose-form). It
contains near term tasks with a short description and
uses links or reference links for more details.

## In Progress

**feat: no_std tprobe core**

[design.md](design.md) settles tprobe's founding rationale but
leaves three open questions that gate the crate's shape: which
`no_std` histogram, whether iiac-perf's fork is deliberate, and
where pinning / `perf_event_open` live. Resolve the three —
recording each decision in a chores design subsection — then
stand up the `no_std` core (no-alloc histogram, `&'static str`
names, cycle-counter trait, `std`-gated reporting).

- 0.1.0-0 prep: open cycle + chores design subsections for the
  three questions (current)
- 0.1.0-1 decide `no_std` histogram — existing crate vs
  hand-rolled fixed-bucket table [[1]]
- 0.1.0-2 confirm iiac-perf fork intent — deliberate (two-repo)
  vs incidental (three-repo consolidation) [[2]]
- 0.1.0-3 place pinning / `perf_event_open` — this crate's `std`
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
