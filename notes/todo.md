# Todo

This file uses [Prose form](../AGENTS.md#prose-form). It
contains near term tasks with a short description and
uses links or reference links for more details.

## In Progress

_No cycle currently in progress._

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

1. feat: registry + buffer-cycling recorder — one handoff
   protocol (per-slot atomic buffer states) serving both the
   benchmark drive (inline collect at defined points) and the
   real-time drive (async collector)
   [design](design.md#registry-and-buffer-cycling-handoff)
2. feat: sample types — `mark()` point events, span
   checkpoints (const-generic K), sample-type descriptor in
   the wire format
   [design](design.md#sample-types-and-the-compression-ladder)
3. chore: recorder cost matrix — hot-path per-event cost vs
   system cost, tprobe measuring itself
   [design](design.md#first-experiment-recorder-cost)

## Done

Completed tasks are moved from `## Todo` to here, `## Done`, as they are completed
and older `## Done` sections are moved to [done.md](done.md) to keep this file small.

- feat: no_std tprobe core [[1]]

# References

[1]: chores/chores-01.md#feat-no_std-tprobe-core
