# workflow-gates

## Intent

`gandr-workflow-gates` owns typed project checks, campaign planning, and the contained mutation-test boundary used by the repository's `mise` tasks.

## Current provision

The crate exposes the `mutants` facade for snapshot, changed-range, scheduled, and sweep campaigns.
`mutants::record` defines deterministic, replayable mutation records with exact source edits, bounded base identities, and distinct killed, compile-error, and survivor verdicts.
Compile errors are never survivors; replay applies exact one-hunk edits, rejects base mismatches and ambiguous sites, and preserves multi-line patches as provenance.

## Planned but absent

The crate does not replace cargo-mutants' guest execution engine or provide a remote campaign service.
Full-campaign scheduling remains an explicit task surface rather than a merge gate.

## Usage

Use the named tasks (`mise run mutants:push`, `mutants:merge`, `mutants:scheduled`, or `mutants:sweep`).
Library consumers can serialize a `MutationRecord` with `to_json`, decode it with `from_json`, and apply it with `reapply` after validating the repository base.

## Named ideas and references

The mutation adequacy contract follows the reach/infect/propagate/reveal model from _Introduction to Software Testing_, Paul Ammann and Jeff Offutt, 2nd ed., Cambridge University Press, ISBN 978-1107172012, and the adequacy ladder uses _Mutation Analysis_, Yue Jia and Mark Harman, IEEE Transactions on Software Engineering 37(3), DOI: 10.1109/TSE.2010.62.
