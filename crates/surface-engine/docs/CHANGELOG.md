# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-08-08 — Give the source driver a path-shaped face

* `current`: `run::run_source_file` reads a source file and runs it exactly as `run::run_source` runs source text, reporting the path in `run::RunFileError::Read` so a diagnostic can name the file even though the failure surfaced inside the read.
  A source-level failure travels unchanged as `run::RunFileError::Run`.
* `current`: This is the seam the `gandr <file>` script runner stands on.
  The engine already owned the language-level source entry after the host-signature ruling pointed the crate edge engine → runtime, so the file face belongs beside it: the reverse placement — a source entry in `gandr-runtime-host` — is a dependency cycle, not a preference.
* `current`: A `#!…` shebang line needs no stripping anywhere: it is grammar trivia (`node_kinds::EXTRAS`), so an executable script and its text run identically.
* `current`: Tests — a new `run` suite pins the read failure, the source failure, the plain run, and the shebang line, covering an entry that previously had no direct test of its own.

## 2026-08-02 — Decline the retired rule-face arrow with its respelling

* `current`: `desc_elab`'s `rule` member reads the ruled `==>` face arrow (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form, ruled").
  A member spelling its face with the retired `~>` is declined as an `ElabDiagnostic` located at the arrow itself, naming the respelling — the grammar keeps `~>` admissible in that slot precisely so this decline exists instead of a parse repair.
* `current`: The decline is not a synonym: the member contributes no `CellFace`, so a stale face reaches neither the description table nor the cell store, and the rest of the declaration still elaborates.

## 2026-08-02 — Confirm circuit arrows against the kind they belong to

* `current`: New `circuit` module — the surface check the ruled circuit block form's redundancy needs (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form, ruled").
  `check_circuit_surface` confirms every arrow against the kind of the thing it belongs to: a declaration's arrow against its kind keyword, a rewrite-sorted binder's against the rule it binds, a `node` line's against the **applied head's** kind, and a `feed` line's against the wire it is.
  A row disagreement is a named, located diagnostic; the reserved `<->` declines naming the reversible-oper lane.
* `current`: The check deliberately stops at arrows and names.
  It does not fold the port/name sets, sweep node-only wiring for cycles owing a `feed`, or lower anything — circuit members stay parse-and-decline at lowering.
  An applied head the environment does not know is skipped rather than guessed: that is a name-resolution question, and answering it here would report an error the program does not have.
* `current`: `parameter_interior` scans for its closing paren from the **opener**, not from just past it: `scan_top_level` offers both brackets at the outer depth, so a scan started inside the group would return a nested group's closer and silently truncate the telescope, dropping every binder after the nesting.
  Unreachable today — every port type reaches the flat run as a Meld — and correct now rather than when a future port form makes it reachable.
* `current`: New `cst_read` module extracted from `desc_elab` — the flat-tile-run `Reader` / `Cursor` and the depth-aware member split that both the levitation stage-0 elaborator and the circuit check walk declarations with.
  The extraction is behaviour-preserving for descriptions: brace depth is now counted, which is inert for a `data` / `codata` block (its brace-bearing sub-forms sit at sort holes and reach the run as Melds) and load-bearing for a `sign` block (whose members' fillers are flat).

## 2026-07-21 — Port the complete surface engine from wyrd (F3)

* `current`: Landed `gandr-surface-engine`, the complete CST-to-core front-end engine (rung F3 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-pipeline` crate.
* `current`: Modules — 27,997 source lines across 25 files covering the SynNode CST adapter; total lowering; datatype/codata/recursive lowering; origins, goals, diagnostics, attributes, and rendering; prelude, host, FFI, linking, and sessions; the one-shot run driver; and edit/footprint/checkpoint incrementality.
* `current`: Re-pointed seven predecessor project dependencies one-to-one (`gandr-desc` → `gandr-theory-levitation`, `gandr-grammar` → `gandr-surface-grammar`, `gandr-nominal` → `gandr-theory-nominal-automata`, `gandr-order-maintenance` → `gandr-theory-orders`, `gandr-parser` → `gandr-surface-parser`, `gandr-recursion` → `gandr-theory-recursion`, `gandr-syntax` → `gandr-surface-syntax`).
  The eighth, `gandr-core`, splits into `gandr-core-checker` for typing and the canonical host signatures and `gandr-core-sequent` for L-machine evaluation; the host-capability adapter adds `gandr-runtime-host` as the tenth reboot project edge.
  Every normal edge is directly used; `checkpoint` consumes `gandr-theory-orders`, and `lower::recursive` consumes `gandr-theory-recursion`.
* `current`: Session and linker evaluation run on the L machine.
  Typing and pre-evaluation terms retain checked annotations; runtime values erase them, pinned by linker plus non-empty record/list/operator session tests.
* `current`: Adapted statement recognition and diagnostics to the reboot grammar's `val` / `run` spellings.
  No compatibility alias or legacy surface spelling remains.
* `current`: Retargeted stage-0 elaboration to `gandr-theory-levitation`.
  Its `Attr`, `CellFace`, `check_desc`, and declaration-table API is source-compatible with the predecessor use sites, so the package rename is the complete delta.
* `current`: Kept the canonical host signatures beside the representation-independent seam in `gandr-core-checker::host`.
  `gandr-surface-engine::host` explicitly re-exports that API and adds source-level module metadata; the signature authority couples neither the engine nor the runtime, and the engine's only runtime edge is the host-capability adapter `run::run_source` composes.
* `current`: Gained `run::run_source`, the one-shot source-program driver (lower → link → prelude-check → host-run) relocated from `gandr-runtime-host`.
  The engine owns the language-level source entry; the runtime stays the host-capability adapter it composes.
* `current`: Preserved and directly tested the predecessor `Prelude` contract — unvalidated ordered bindings with later-name shadowing — behind a local wrapper that lends the L machine its binding slice.
* `current`: Replaced the dev-only grammar-contract-fixtures dependency with 19 engine-local fixtures; the acceptance harness discovers every `current/*.gandr` file rather than maintaining a second filename list.
  The predecessor helper was data-only at the engine's three use sites (two root paths and one JSON path manifest).
  All thirteen `current/` sources are byte-identical to the predecessor set; both readers target the local copies, and directory discovery replaces manifest parsing.
  All 299 predecessor tests remain semantically: 297 names are unchanged, while the linker and tuple-statement tests were renamed for the L-machine and `val` adaptations; two review regressions bring the total to 301.
  The executable corpus harness has since landed (`crates/surface-corpus`); promoting the four `surface/` witnesses and the `corpus/` model/pathological pair into its coverage map remains open, while the thirteen `current/` parser-state fixtures remain local.
* `current`: O3 scrub removed 108 direct wyrd tracker-ID occurrences and 126 numeric predecessor ADR citations from the engine and the core/runtime host files changed by F3, replacing them with current domain language and, where a citation carried the design, the inlined substance.
* `current`: Tests — the surface-engine all-features aggregator runs 323 tests; 33 runtime-host tests make the affected validation 356 tests total, with none ignored.
* `current`: Feature posture — `default = []`, `codecs` enables serde/JSON reports, `regex` enables the matching primitive in both core engines, and `full = ["codecs", "regex"]`.
