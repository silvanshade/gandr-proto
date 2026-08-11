# Workflow: the executable example corpus (surface-corpus)

> Read when: landing a language feature, or adding or editing corpus examples.
> The landing rule below supersedes the earlier separate timing and two-tree-cardinality rules while preserving the model/pathological separation.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

`crates/surface-corpus` is the **executable example corpus** — real gandr programs exercising the implemented language surface end-to-end (parse → lower → check → eval over the `gandr-surface-engine` session seam, plus the mode-specific inspection harnesses: `sequent` for L0 command faces, `desc` for stage-0 descriptions) and the integration tie-point across the crates.

- **Landing rule.** A new surfaced language feature lands with its full corpus treatment in the **same change**: runnable model example(s) with literate documentation, runnable pathological coverage, harness assertions, and coverage-map registration.
  Passing implementing-crate tests first is necessary but not sufficient; examples are landing evidence, never residual work.
  Design-stage work is exempt: proposals carry a _Corpus examples plan_ section instead.
  A syntax-first change gets a parse-gated `surface/` witness.
  The semantics-graduation change promotes that witness to runnable `model/`, adds runnable pathological coverage, harness assertions, and coverage-map registration in that same change.
- **Internal-before-surface rule.** An engine feature with no user syntax lands with named runnable crate fixtures exercised by named crate tests or harness assertions and mirroring the intended gandr programs.
  Its consolidated closeout residuals bead records those fixtures, the future corpus programs, and the exact blocker that enables promotion; the manual states that the feature is not user-writable yet ([tracker.md](tracker.md) §“Feature landing and residual closeout”).
- **Three trees, never mixed.** `examples/model/` = pedagogy — fully-commented programs explaining what they compute **and why they are written that way**.
  `examples/pathological/` = testing — semantic edge cases, failure goldens, stress cases.
  `examples/surface/` = syntax reservations — surfaced-but-not-yet-implemented concrete syntax, parse-gated by the parser's zero-obligation corpus gate, excluded from lowering/evaluation harnesses, each naming the bead that graduates it to `model/`.
  Grammar-level pathology stays authoritative in `gandr-grammar-contract-fixtures`; the corpus cross-references it, never duplicates it.
- **Authority.** Where an example disagrees with the design corpus (`docs/gandr/`), the design corpus wins and the example is a bug (`docs/WORKFLOW.md` §“Source of truth”).
  The example corpus is executable witness, not a second source of truth.
- **The `gandr-pro` skill has not crossed the reboot** — no live skill file exists; its carried home when ported is `crates/surface-corpus/skills/gandr-pro/SKILL.md` (`docs/research/toolchain-retention.md`), and it resumes its maintenance train (mental model, surface tables, semantic traps, feature → model-example map) then.
- **Focused gate.** `cargo test -p gandr-surface-corpus` after any corpus change; the parser's zero-obligation count lock (`gandr-surface-parser` `acceptance::corpus_molds_to_zero_obligations`) guards the example trees — the tree-sitter highlight-parity suite is parked to rung F6 (`docs/research/front-end-port-staging.md` §9) and guards nothing yet.
