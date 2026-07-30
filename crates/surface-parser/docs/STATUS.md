# Status

Crate scope: `crates/surface-parser` (package `gandr-surface-parser`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the front-end's resumable push-machine melder plus its obligation taxonomy — the W4′ parser lane.
  It turns source text into a committed CST (`gandr-surface-syntax`) over the checked PBG (`gandr-surface-grammar`), carrying every parse's obligations beside the tree so the front-end is total: any `SourceSlice` yields a well-formed `ParseResult`, never a panic.
* Ported at rung F2 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-parser` crate.
  The recut renames the package to `gandr-surface-parser` and re-points the workspace dependencies to their landed reboot homes.
* Modules — 9436 source lines across six modules:
  + `oblig` (the obligation taxonomy): the closed `Oblig` severity ladder, the `ObligationInstance` (class plus span), and the `Delta` per-class count array with lexicographic net-then-gross comparison from highest severity down.
  + `label` (the labeler): a hand-rolled total DFA over source bytes into lexical `Token`/`Lexeme` classes; no `logos`/`phf`/proc-macro, mirroring the tree-sitter lexical surface, with a byte the grammar has no tile for flowing the `UnmoldedTok` path rather than a lexer error.
  + `mold` (the molder): resolves a token's candidate molds to the one the melder should push (`Molder`, `candidate_labels`), the deterministic disambiguation layer between the labeler and the melder.
  + `meld` (the melder): the resumable, first-order push machine (`MeldState`) — `push` (Shift / Reduce / Degrout) is primary and total, `commit` derives the batch, and `checkpoint`/`resume`/`finalize` complete the streaming surface.
  + `parse` (the batch entry): `parse` folds `push` over the labeled + molded stream and commits the batch `ParseResult`, recording trivia for losslessness.
  + `lib`: the public re-export surface.
* Dependency re-point: `gandr-grammar` → `gandr-surface-grammar` and `gandr-syntax` → `gandr-surface-syntax`.
  The wyrd manifest's third edge, `gandr-graph`, was **dropped**, not re-pointed: the parser reaches every graph type it uses (`Prec`, `PrecDag`, `PrecSpec`, `Assoc`, `Bound`, `Dir`) through the grammar's public re-exports and has zero direct `gandr-theory-graphs` references, so the direct edge was vestigial in wyrd and is not carried (an authorized deviation from the staging table's dep list — F3's pricing should not re-add it blindly).
* The grammar's `parity` feature does not exist in the reboot (deferred to F6); the parser never references it, so no adaptation was needed on that axis.
* O3 / ADR scrub — 74 wyrd bead-ID occurrences and 7 wyrd `ADR-NN` citations (ADR-43/48/73/76/80) were dropped or inlined in current terms; zero survive into rustdoc, and the crate is clean under `cargo:doc-check`.
  Per the F1 boundary, the wyrd design-wave labels (W3′/W4′/W4b/W4d/W4e), academic figure references (`paper Fig. N`), and the tree-sitter `grammar.js` cross-references were preserved as behaviour-bearing / provenance content; the proposal / `graph-core` section cross-references (`§N`) were likewise preserved except where they shared a parenthetical with a scrubbed bead-ID.
* Tests — the parser-driven goldens funnelled through the `autotests = false` aggregator `tests/lib.rs` (the `parser` test target): the self-contained `contracts` module (paper-trace fixtures, module-declaration and recovery goldens, and the totality / determinism / checkpoint-resume proptests) plus the `acceptance` submodule (`tests/acceptance.rs`) of end-to-end goldens over inline gandr text. 66 pass under nextest with 7 parked (see below); `src/parse.rs` also carries inline losslessness + hostile-input proptests.

## designed direction

* **The corpus gate returns at F4.** The parser's zero-obligation corpus gate parse-gates the future `surface/` witness tree (`docs/workflow/corpus.md`), but the corpus it reads (`crates/surface-corpus/examples`) lands at rung F4 with `surface-corpus`.
  The gate and its count lock are ported faithfully — the count-lock constants stay pinned in the test body — and the corpus-reading tests are parked with `#[ignore]`, returning when F4 removes the attribute.
  The locked counts the gate re-establishes against the reboot corpus: the model + pathological trees mold **87 / 87 clean**, the `surface/` tree is **non-empty and every fixture molds clean**, and the whole corpus carries **zero total obligations** (`corpus_molds_to_zero_obligations`).
  Parked to F4 (five corpus tests + one inline): `acceptance::corpus_molds_to_zero_obligations` (THE gate), `acceptance::corpus_parses_totally`, `acceptance::corpus_files_cold_parse_within_p99_latency_budget`, `acceptance::expected_agrees_with_committed_finalize`, `acceptance::minimization_prefers_clean_readings`, and the inline `parse::tests::corpus_parses_totally`.
  A parked test's in-body prose count (e.g. the `67 / 67` intermediate) is a wyrd-era artifact carried verbatim; F4 reconciles the prose to the asserted count when it regenerates the reboot corpus.
* **The incomplete-input recovery fixtures return at F6.** `acceptance::incomplete_input_flags_statement_local_obligations` reads `gandr-grammar-contract-fixtures`, whose fold is deferred to F6 (staging call O4); it is parked with `#[ignore]` and its fixture path is a forward reference F6 resolves.
* **The corpus-driven benches return at F4.** The wyrd `push` and `parse` criterion benches build their streams from the committed corpus, so they cannot run before F4; they are deferred to co-land with `surface-corpus`, and the `criterion` dev-dependency was dropped with them (it re-enters at F4).
* The restored grammar contracts suite: F2 re-adds `gandr-surface-parser` as a **dev-only** dependency of `surface-grammar` (the deliberate cycle-break direction — normal dep parser→grammar, dev dep grammar→parser) and restores the six parser-coupled `contracts` tests parked at F1; see `crates/surface-grammar/docs/STATUS.md`.

## open decision

* The parked corpus/fixture tests and the deferred benches are the F2 residuals; each returns at its named rung (F4 corpus, F6 contract-fixtures) by removing the `#[ignore]` / re-adding the bench targets once its input exists.
