# Status

Crate scope: `crates/surface-engine` (package `gandr-surface-engine`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the reboot's complete CST-to-core surface engine: total lowering, origin tracking, structured diagnostics and goals, prelude/host/attribute tables, edit-action reconstruction, incremental checkpoints, linking, and stateful REPL sessions.
* Ported at rung F3 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-pipeline` crate.
  The recut renames the package to `gandr-surface-engine` while preserving its public module shape and behavior.
* Modules — 27,081 source lines across 23 source files:
  + `synnode` adapts the form-name-free melder CST to the lowerer's named-node view.
  + `lower` and its `data`, `codata`, `recursive`, `types`, and `node_kinds` submodules implement total source-to-core lowering.
  + `origin`, `goals`, `diag`, `attributes`, and `render` project core results back to structured, source-ranged front-end data.
  + `prelude`, `host`, `ffi`, `link`, and `session` connect the lowered core to typed native bindings, host effects, whole-file linking, and evaluation.
  + `edit`, `footprint`, and `checkpoint` provide edit reconstruction and dependency-validated incremental re-typing.
  + `boundary` owns the crate's typed scalar and string boundary wrappers.
* Dependency re-point:
  + `gandr-core` → `gandr-core-checker` for syntax, typing, diagnostics, marks, and native primitive definitions.
  + Evaluation is deliberately split to the reboot's L machine in `gandr-core-sequent`; `Session` and whole-file linker tests call `run_comp_with_prelude` / `run_comp` there, while the shared `Eval` outcome remains owned by `gandr-core-checker`.
  + `gandr-desc` → `gandr-theory-levitation`, `gandr-grammar` → `gandr-surface-grammar`, `gandr-nominal` → `gandr-theory-nominal-automata`, `gandr-order-maintenance` → `gandr-theory-orders`, `gandr-parser` → `gandr-surface-parser`, `gandr-recursion` → `gandr-theory-recursion`, and `gandr-syntax` → `gandr-surface-syntax`.
  + Every normal dependency has a direct source-level use.
    The predecessor's dev-only `gandr-grammar-contract-fixtures` edge was dropped: the engine tests now own the exact fixtures they exercise under `tests/fixtures`.
* Reboot API adaptations are localized at the relevant boundaries:
  + block statements recognize the reboot grammar's `val PAT = E;` and `run PAT <- E;` spellings while retaining the same core `Let` / `Bind` lowering;
  + stage-0 datatype elaboration targets `gandr-theory-levitation`'s current `Attr`, `CellFace`, and declaration-table API;
  + checked operator/list/record annotations remain a typing concern and are intentionally absent from the L-machine value payloads asserted by session tests;
  + `gandr-runtime-host` re-exports this crate's canonical `host` signatures, so the lowering table remains the single authority without creating a dependency cycle.
* O3 scrub: direct wyrd tracker identifiers and numeric predecessor ADR citations were replaced by current domain terms; predecessor package names survive only where the status/changelog records port provenance.
* Tests — the crate uses one `autotests = false` integration-test aggregator (`tests/engine.rs`) over the ported module suites and engine-local fixtures.
  All 299 tests pass under nextest with all features; none are parked or ignored.
* Feature posture: `default = []`; `codecs` enables the serde/JSON report surface; `regex` enables the matching primitive in both reboot core engines; `full = ["codecs", "regex"]`.

## designed direction

* Rung B3.1 owns structural deepening after the faithful port.
  F3 deliberately preserves the predecessor's module/API topology so behavior and parity remain reviewable before extracting deeper module boundaries.
* Rung F4 lands `surface-corpus`.
  It must consolidate or promote the engine-local surface/corpus fixtures into the executable corpus where they represent user-visible language behavior, then register them in the corpus coverage map as required by `docs/workflow/corpus.md`.

## open decision

* The F3 crate itself has no unresolved authority split: the checker owns typing, the sequent core owns evaluation, the surface engine owns source-facing host signatures, and runtime-host re-exports those signatures.
  The remaining fixture-promotion work belongs to F4 rather than this port rung.
