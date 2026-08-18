# gandr-workflow-gates

`gandr-workflow-gates` owns typed project checks, campaign planning, and the contained mutation-test boundary used by the repository's `mise` tasks.

## Current provision

The crate exposes the `mutants` facade for snapshot, changed-range, scheduled, single-package, and sweep campaigns.
Package campaigns validate one exact name on the host, archive current `HEAD`, run the existing guest with `--package`, publish before reporting survivors, and clean temporary state.
`mutants::record` defines deterministic, replayable mutation records with exact source edits, bounded base identities, and distinct killed, compile-error, and survivor verdicts.
Compile errors are never survivors; replay applies exact one-hunk edits, rejects base mismatches and ambiguous sites, and preserves multi-line patches as provenance.

## Planned but absent

The crate does not replace cargo-mutants' guest execution engine or provide a remote campaign service.
Full-campaign scheduling remains an explicit task surface rather than a merge gate.

## Using it

Use the named tasks (`mise run mutants:push`, `mutants:merge`, `mutants:scheduled`, `mutants:package <name>`, or `mutants:sweep`).
Library consumers can serialize a `MutationRecord` with `to_json`, decode it with `from_json`, and apply it with `reapply` after validating the repository base.

## Theoretical ideas relied on

Mutation adequacy, and its reach, infect, propagate and reveal model; the adequacy ladder; the distinction between a compile error and a survivor, which is what keeps a mutation score from counting unbuildable mutants as evidence.

## Primary references

- Paul Ammann and Jeff Offutt, _Introduction to Software Testing_, 2nd edition, Cambridge University Press, 2016, ISBN 978-1107172012 — the reach, infect, propagate and reveal model the adequacy contract follows.
- Yue Jia and Mark Harman, _Mutation Analysis_, IEEE Transactions on Software Engineering 37(3), `doi:10.1109/TSE.2010.62` — the adequacy ladder.
