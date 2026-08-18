# surface-corpus

`gandr-surface-corpus` is the executable example corpus: real gandr programs that exercise the implemented language surface end to end, plus the harness that runs each one and checks it against expectations written in the program itself.
It is also the integration tie-point across the front-end crates — a change that passes every implementing crate's own tests and breaks a corpus example has broken the language, not a unit.

## What it currently provides

- **Three trees, never mixed.** `examples/model/` holds fully commented programs that explain what they compute and why they are written that way.
  `examples/pathological/` holds semantic edge cases and failure goldens.
  `examples/surface/` holds surfaced-but-not-yet-implemented syntax, parse-gated and firewalled from execution — the walker never lowers or evaluates it.
- **Directives.** Each runnable example declares how to run itself and what to expect through `//@ key: value` lines, which are ordinary comments to the grammar and so never perturb the program.
- **Directives for the description route.** Beside the rule-face and cell counts, an example can ask how many η cells its declarations licensed and why one licensed none — the latter reported rather than diagnosed, because licensing no η law is the ordinary case.
- **Six run modes.** The session engine, the runtime host for shell programs, the foreign-call path, lowering alone, the phase-L0 sequent inspector, and the stage-0 description elaborator.
- A frozen root cardinality.
  The number of programs directly under each tree is pinned by a test whose comment records what moved it, so a feature landing that forgets its corpus treatment fails rather than passing quietly.
  Downstream sequent gates derive provenance from each source's repository-relative path and exact `.gandr` bytes; no pre-lowered anchor file is required.

## Planned but not implemented

- The feature-to-example map, whose home is a skill file that has not crossed the reboot; until it lands, the frozen-cardinality test's comment is where a landing's corpus treatment is registered.
- The tree-sitter highlight-parity suite, parked with the tree-sitter reference.

## Using it

`cargo test -p gandr-surface-corpus` runs every model and pathological example and reports the expectations that failed.
`check_case` runs one example if you want to drive a single program from another test.

## Theoretical ideas it relies on

Literate programming as the model tree's discipline, and golden-file testing for the failure tree.

## Primary resources

The corpus exercises the language rather than implementing a theory; the theoretical resources belong to the crates it exercises — `surface-grammar`, `surface-parser`, and `surface-engine` each carry their own.
