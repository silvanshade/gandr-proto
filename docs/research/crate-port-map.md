# Crate port map — `wyrd@failed-refactor:crates/` → gandr reboot (gandr-fcw.3)

> Research deliverable for the gandr language reboot (epic `gandr-fcw`).
> Read for a port plan: public API surface, workspace/wyrd coupling, feature-flag posture vs the proposed scope-prefixed scheme, and the five targeted questions (a)–(e).
> Every claim cites its primary source as `alias:relative/path[:line]`.
> The read source tree is `wyrd@failed-refactor` (canonical wyrd tree); the reboot worktree (the gandr repo) currently has **no `crates/` and no `docs/`** — only top-level config seeded from wyrd — so the entire port is greenfield-into-an-empty-repo.

---

## 0. Executive summary

* **Count correction.** The question says "20 gandr-\* crates plus wyrd-rust-gates and wyrd-dylint."
  The source tree actually has **24 `gandr-*` crates + 2 `wyrd-*` crates = 26** under `wyrd@failed-refactor:crates/` (`wyrd@failed-refactor:Cargo.toml`).
  This report covers all 26.
* **Rename is mostly done at the crate-name level, not in the prose.** 24 of 26 crates are already named `gandr-*`; only `wyrd-rust-gates` and `wyrd-dylint` keep the old name.
  But **126 distinct `wyrd-*` bead IDs** are cited across crate comments/docs (concentrated in wyrd-rust-gates, gandr-pipeline, gandr-core, gandr-grammar, gandr-parser), plus residual "Wyrd" branding in a few doc-comments and the `dylint_lib = "wyrd_dylint"` test attributes.
  These are stale references to the retired tracker — the port must decide whether to rewrite, remap to `gandr-*` beads, or leave as provenance.
* **Dependency spine is clean and layered** (§2).
  Leaves: `gandr-nominal`, `gandr-graph`, `gandr-syntax`, `gandr-recursion`, `gandr-order-maintenance`, `gandr-render-proto`.
  `gandr-core` sits on `gandr-nominal` only; `gandr-pipeline` is the hub (8 inbound-ish workspace deps).
  Driver `gandr` sits on top.
* **Two orphan crates with no inbound workspace edges** (question c): `gandr-data` and `gandr-kernel-levels`.
  Both are _intentional_ leaves awaiting a consumer that is not built yet (§7c). (Reboot update, B2: the kernel-levels consumer now exists — see the §7c update note.)
* **tree-sitter is demoted, not deleted** (question d): `gandr-tree-sitter` is workspace-`exclude`d and reachable only through `gandr-grammar`'s optional `parity` feature; the tree-sitter grammar package (`packages/tree-sitter-gandr/`) still exists as the differential-parity reference.
  The Rust default graph is tree-sitter-free, enforced by a `wyrd-rust-gates` gate (§7d).
* **Formatting is designed as three crates that do not exist yet** (question e): `proposal-pretty-printing.md` plans `gandr-doc` (layout VM), `gandr-fmt` (CST formatter), and **`gandr-pretty`** (core presentation printer).
  The premise that "gandr-pretty never existed" is true _as code_ but **contradicted by the spec**, which explicitly names it (§7e).
  Today pretty/render logic is inline and scattered across ≥6 modules.
* **The proposed scope-prefixed feature scheme (`gandr_core_*` / `gandr_feat_*` / `gandr_tool_*` / `gandr_test_*`) exists nowhere in source or docs** — it is purely a target.
  The current posture is ad-hoc `default`/`full` plus a few bespoke feature names, with **four concrete non-conformances** and **two outright bugs/typos** the migration must fix (§5).
* **L-machine parity gaps in `gandr-sequent`** (question a) are three named, tracked residual seams — the un-focusing readback residual, prelude free-name resolution in force position, and the whole-program ADR-76 identity-former decline — all reporting a _defined_ `UnsupportedByReference`, never a panic (§7a).
  A fourth, adjacent decline (ADR-80 declared-data `DataCase`) rides the same rule.
* **Biggest port-risk crates** are the ones with heavy external stacks and unsafe seams: `gandr-tui` (8 TUI crates), `gandr` (reedline), `gandr-ffi` (libffi/libloading/cc, unsafe), `gandr-tree-sitter` (C compile), and `wyrd-rust-gates` (37k LOC, deeply wyrd-workflow-coupled incl. an IU submodule pin) (§8).

---

## 1. Method & scope

* Source: `wyrd@failed-refactor:crates/` (26 crate dirs) + `wyrd@failed-refactor:docs/` (84 ADRs, spec, proposals) + `wyrd@failed-refactor:mise.toml` (test lanes) + `wyrd@failed-refactor:packages/tree-sitter-gandr/`.
* Read-only throughout.
  All manifests, `lib.rs` module surfaces, and the substantive modules for questions (a)–(e) were read directly.
* LOC figures below are `wc -l` over each crate's `src/**.rs` (includes inline `#[cfg(test)]` modules; excludes `tests/`), a rough size signal only.

---

## 2. Dependency topology

Workspace (path) edges only; external crates in §3.
`(opt)` = optional/feature-gated, `(dev)` = dev-dependency edge.

```text
gandr-nominal            ← gandr-core, gandr-pipeline
gandr-graph              ← gandr-grammar, gandr-parser, gandr-polygraph
gandr-syntax             ← gandr-grammar, gandr-parser, gandr-pipeline, gandr-tui, gandr-lsp(opt), gandr
gandr-render-proto       ← gandr-grammar, gandr-tree-sitter, gandr-tui, gandr-lsp(opt)
gandr-recursion          ← gandr-pipeline
gandr-order-maintenance  ← gandr-pipeline
gandr-desc               ← gandr-pipeline, gandr-polygraph, gandr-vdc, gandr-corpus
gandr-core               ← (nearly everything) — gandr-pipeline, gandr-sequent, gandr-shell,
                           gandr-data, gandr-desc, gandr-ffi(opt), gandr-tui, gandr-lsp(opt),
                           gandr-corpus, gandr
gandr-grammar            ← gandr-parser, gandr-pipeline, gandr-tui, gandr-lsp(opt), gandr, (dev of grammar: gandr-parser)
gandr-parser             ← gandr-pipeline, gandr-tui, gandr-lsp(opt), gandr, (dev of grammar)
gandr-pipeline           ← gandr-sequent(dev), gandr-shell, gandr-ffi(opt), gandr-tui, gandr-lsp(opt),
                           gandr-corpus, gandr
gandr-sequent            ← gandr-polygraph, gandr-corpus, gandr-vdc(dev)
gandr-polygraph          ← gandr-vdc
gandr-shell              ← gandr-corpus, gandr
gandr-tui                ← gandr
gandr-grammar-contract-fixtures ← gandr-pipeline(dev only)
gandr-tree-sitter        ← gandr-grammar(opt, parity)      [workspace-EXCLUDEd]
gandr-data               ← (NOBODY)                         [orphan — §7c]
gandr-kernel-levels      ← (NOBODY)                         [orphan — §7c]
gandr-vdc                ← (NOBODY inbound; top of the VDC lane)
gandr-corpus             ← (NOBODY inbound; test/harness leaf)
gandr-lsp                ← (NOBODY inbound; binary)
gandr-render-proto/present, wire ← leaf wire types
gandr                    ← (NOBODY inbound; the toolchain driver binary)
wyrd-rust-gates          ← (NOBODY inbound; gate crate, no gandr deps)
wyrd-dylint              ← (standalone [workspace]; not a member; loaded as a dylint at test time)
```

Sources: each crate's `wyrd@failed-refactor:crates/<crate>/Cargo.toml` `[dependencies]` / `[dev-dependencies]`.

**Layer reading (bottom→top):** substrate leaves (nominal, graph, syntax, recursion, order-maintenance, render-proto) → `gandr-core` (checker + typing machine) → `gandr-desc` / `gandr-grammar` / `gandr-parser` → `gandr-pipeline` (the lowering hub) → surfaces (`gandr-shell`, `gandr-ffi`, `gandr-sequent`, `gandr-lsp`, `gandr-tui`) → `gandr` driver.
Two parallel research lanes hang off the side: the **sequent/rewrite lane** (`gandr-sequent` → `gandr-polygraph` → `gandr-vdc`, all on `gandr-desc`/`gandr-graph`) and the **kernel lane** (`gandr-kernel-levels`, still detached).

---

## 3. Per-crate inventory

Columns: LOC (src, incl. inline tests); role; key public modules/types; external (non-workspace) deps; features.
Manifest source is `wyrd@failed-refactor:crates/<crate>/Cargo.toml`; module source is `…/src/lib.rs`.

| Crate                               |   LOC | Role                                                                                                         | Key public surface                                                                                                                                                                      | External deps                                                                                                                   | Features                                                               |
| ----------------------------------- | ----: | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| **gandr-core**                      | 38.2k | CBPV bidirectional checker + defunctionalized typing machine + CK evaluator (A1/A2; ADR-9)                   | `checker`, `machine`, `eval`, `syntax`, `types`, `subtype`, `grade`, `identity`, `effect`, `control`, `ctx`, `mark`, `prim`, `stack`, `nominal`, `intern`, `boundary`; opt `strategies` | `no-panic`, `thiserror`, `regex`(opt), `proptest`(opt)                                                                          | `default`, `full=[regex]`, `regex`, `proptest-strategies`              |
| **gandr-pipeline**                  | 27.1k | Incremental CST→core lowering hub with origin tracking + session engine (A2)                                 | `lower`, `checkpoint`, `desc_elab`, `diag`, `edit`, `goals`, `host`, `link`, `origin`, `prelude`, `render`, `session`, `synnode`, `ffi`, `attributes`, `footprint`                      | `serde`(opt), `serde_json`(opt), `thiserror`                                                                                    | `default`, `codecs`, `full=[codecs,regex]`, `regex`                    |
| **gandr-grammar**                   |  8.1k | Checked precedence-bounded grammar core over precedence DAGs; mold highlighter                               | `check`, `model`, `mold`, `walk`, `surface`, `highlight`, `parity`; `TREE_SITTER_NAMED_KINDS`                                                                                           | `criterion`(dev), `serde_json`(dev), `tree-sitter`(opt)                                                                         | `default`, `full=[]`, `parity` ⚠ (§5)                                  |
| **gandr-parser**                    |  9.4k | Resumable push-machine melder + obligation taxonomy over the checked PBG (ADR-73)                            | `label`, `mold`, push machine, obligation types                                                                                                                                         | `criterion`(dev), `proptest`(dev)                                                                                               | `default`, `full=[]`                                                   |
| **gandr-syntax**                    |  3.8k | Flat-arena CST + incremental syntax diffing                                                                  | `Cst`, `CstBuilder`, `MoldId`, `NodeKind`, `Diff`, `diff`, `GrammarFingerprint`, `SourceSlice`                                                                                          | —                                                                                                                               | `default`, `full=[]`                                                   |
| **gandr-graph**                     |  8.4k | Deterministic dense-u32 graph algorithms; petgraph adapter hidden                                            | `Reachability`, `StronglyConnectedComponents`, `ImmediateDominators`, `TransitiveReductionClosure`, `CycleWitness`, `all_simple_paths`, `condensation`, `prec` (DAG)                    | `fixedbitset`, `petgraph`, `proptest`(dev)                                                                                      | `default`, `full=[]`                                                   |
| **gandr-nominal**                   |  0.6k | Sort-tagged `Atom<S>` + monotone `Gensym<S>` name substrate (ADR-41)                                         | `Atom<S>`, `Gensym<S>`, `Sort`, `AtomId`, `Unifiability`, `GensymExhausted`                                                                                                             | `proptest`(dev)                                                                                                                 | `default`, `full=[]`                                                   |
| **gandr-recursion**                 |  0.3k | Safe first-order structural-recursion engine ("Wyrd-owned")                                                  | `Machine` trait, `Step`, `StepResult`, `run`                                                                                                                                            | —                                                                                                                               | **none** ⚠ (§5)                                                        |
| **gandr-order-maintenance**         |  2.0k | O(1)-compare total order for the incremental pipeline (A2)                                                   | `OrderMaintenance`, `Pos`, `Interval`, `HandleMembership`, `OrderError`                                                                                                                 | `thiserror`, `criterion`(dev), `proptest`(dev)                                                                                  | `default`, `full=[]`                                                   |
| **gandr-desc**                      |  4.3k | Levitation stage-0 datatype descriptions `{1,var,×,σ}` + generic programs + decoder (ADR-67; ADR-81)         | `desc`, `code`, `decode`, `generic`, `arity`, `cell`, `typed_cell`, `wellformed`, `builtin`, `intern`                                                                                   | `proptest`(dev)                                                                                                                 | `default`, `full=[]`                                                   |
| **gandr-sequent**                   |  7.6k | Polarized System-L command IL + static focusing `𝓕` (L0) + L1 machine (partial) (ADR-65)                     | `il`, `focus`, `machine` (`LMachine`,`LValue`), `check`, `store`, `pretty`, `inspect`, `differential`, `boundary`                                                                       | `gandr-pipeline`(dev), `proptest`(dev)                                                                                          | `default`, **`features=[]`** ⚠ typo (§5)                               |
| **gandr-polygraph**                 |  5.4k | Fusion by Squier completion over the command IL (L2 target; ADR-65/68/69)                                    | `cell`, `overlap`, `completion`, `rewrite`, `compose`, `subst`, `pattern`, `bridge`, `tracelet`, `elaborate`                                                                            | `proptest`(dev)                                                                                                                 | `default`, `full=[]`                                                   |
| **gandr-vdc**                       |  4.1k | VDC reflection face (FVDblTT-shaped) over the rewrite layer (ADR-68/69)                                      | `vdc`, `check`, `iso`, `query`, `syntax`, `directed/{hom,coend,context}`                                                                                                                | `gandr-sequent`(dev), `proptest`(dev)                                                                                           | `default`, `full=[]`                                                   |
| **gandr-kernel-levels**             |  5.9k | Certified kernel level oracle `{0,+1,max}` + Bezem–Coquand loop-checking (ADR-78)                            | `level` (`Level`,`lt`,`leq`), `order` (witnesses/validators), `poset` (`LandmarkPoset`), `entail`, `horn`                                                                               | `proptest`(dev). **`#![no_std]`**, TCB wall: no non-kernel deps                                                                 | `default`, `full=[]`                                                   |
| **gandr-shell**                     |  2.8k | Headless host-effect runtime (Exec/Fs/Proc/Env) over `run_with_host` (ADR-35)                                | `driver` (`run_program`,`run_source`,`run_source_file`), `handler` (`ShellHandler`), `sig`, `codec`, `error`                                                                            | `thiserror`                                                                                                                     | `default`, `full=[]`                                                   |
| **gandr-ffi**                       |  1.8k | Interpreter FFI path: libffi+libloading native handler at the host seam (ADR-35; proposal-ffi.md)            | `driver`, `handler` (`FfiHandler`), `registry`, `error`, `boundary`                                                                                                                     | `libffi`(opt), `libloading`(opt), `cc`(build,opt), `thiserror`(opt)                                                             | `default`, `ffi`, `full=[ffi]`                                         |
| **gandr-data**                      |  2.2k | JSON/TOML/YAML ↔ typed gandr value codecs over the public value API (ADR-35 D6; proposal-shell §2)           | `json_codec`, `toml_codec`, `yaml_codec`, `common`, `error`                                                                                                                             | `serde_json`(opt), `toml`(opt), `yaml-rust2`(opt), `thiserror`                                                                  | `default`, `codecs`, `full=[codecs]`                                   |
| **gandr-render-proto**              |  2.3k | Leaf wire types for the inspection render bus + graduated TUI present seam (proposal-inspection-protocol.md) | `wire`, `present`                                                                                                                                                                       | `serde`(opt)                                                                                                                    | `default`, `codecs`, `full=[codecs]`                                   |
| **gandr-lsp**                       |  3.6k | Hand-rolled sans-io LSP server over the pipeline (ADR-64; wyrd-3vxh)                                         | `analysis`, `framing`, `rpc`, `protocol`, `server`, `position`, `tokens`; bin `gandr-lsp`                                                                                               | `serde`(opt), `serde_json`(opt), `thiserror`(opt); 6 opt gandr deps                                                             | `default`, `codecs` (whole-crate gate), **`full=[]`** ⚠ (§5)           |
| **gandr-tui**                       |  7.2k | Full-screen ratatui surface over the pipeline (ADR-62; proposal-gandr-tui.md)                                | `app`, `worker`, `editor`, `event`, `keymap`, `pane`, `present`, `render`, `theme`, `view`, `highlight`, `run`                                                                          | `ratatui`, `crossterm`, `edtui`, `color-eyre`, `nucleo-matcher`, `portable-pty`, `tui-term`, `vt100`, `thiserror`, `insta`(dev) | `default`, `full=[]`                                                   |
| **gandr-corpus**                    |  2.2k | Executable example corpus + end-to-end harness (ADR-52)                                                      | corpus runner; `split_items`; model/pathological `.gandr` trees                                                                                                                         | `gandr-core`(strategies,dev)                                                                                                    | `default`, `ffi`, `full=[ffi,regex]`, `regex`                          |
| **gandr**                           |  2.5k | Toolchain driver: reedline shell-REPL + script runner + tui/lsp/mcp/fmt/build namespace (ADR-62/74)          | `engine`, `repl`, `script`, `shell`, `render`, `boundary`; bin `gandr`                                                                                                                  | `reedline`, `nu-ansi-term`, `color-eyre`, `thiserror`                                                                           | `default`, `full=[]`                                                   |
| **gandr-grammar-contract-fixtures** |  0.6k | Fixture locator: path constants + embedded manifest bytes (shared Rust/Node)                                 | `FIXTURE_ROOT_RELATIVE_PATH`, `MANIFEST_RELATIVE_PATH`, `MANIFEST_JSON`, `MANIFEST_SCHEMA_JSON`                                                                                         | `criterion`(dev). **`#![no_std]`**, EMPTY `[dependencies]`                                                                      | `default`, `full=[]`                                                   |
| **gandr-tree-sitter**               |  0.5k | tree-sitter reference parser+highlighter for the E1/E2 parity lane (reference-only)                          | highlight query driver; `HighlightError`                                                                                                                                                | `tree-sitter`, `streaming-iterator`, `thiserror`, `cc`(build), `criterion`(dev)                                                 | `default`, `full=[]`. **workspace-EXCLUDEd**; no workspace inheritance |
| **wyrd-rust-gates**                 | 37.2k | Rust-AST contract + graph-boundary gates for the workspace                                                   | `contracts`, `coverage`, `graph_boundary`, `mutants`, `docs`, `project`, `source_policy`, `workflow`, `maintenance`, `parser_facade`, `support`, opt `fuzzing`                          | `blake3`, `serde`, `serde_json`, `syn`, `toml`, `yaml-rust2`, `criterion`/`insta`/`proptest`(dev)                               | `default`, `full=[]`, **`fuzzing`** (only crate with it)               |
| **wyrd-dylint**                     |  1.3k | Project-local Dylint rules for wyrd Rust type boundaries                                                     | cdylib; loaded via `dylint_lib="wyrd_dylint"`                                                                                                                                           | `clippy_utils`(git pin `9fca3bc9`), `dylint_linting`, `dylint_testing`(dev). **rustc_private**, standalone `[workspace]`        | **none** ⚠ (§5)                                                        |

**Reboot update (F0, 2026-07-21):** the two front-end leaves in this inventory are ported.
`gandr-syntax` landed as `gandr-surface-syntax` (`crates/surface-syntax`); `gandr-render-proto` landed as `gandr-surface-render-remote` (`crates/surface-render-remote`) — the owner renamed it from the `surface-render-proto` recommendation to the _remote_ render face, dovetailing the planned `surface-render` printer (wyrd-era `gandr-pretty`).
Both are verbatim ports with the retired bead-ID and wyrd ADR citations dropped (staging call O3 plus the current-terms provenance rule) and a `docs/STATUS.md` + `docs/CHANGELOG.md` pair each.
See `front-end-port-staging.md` §4 (reconciliation) and §9 (rung F0).

---

## 4. Wyrd coupling inventory

**(1) Crate names.** 24/26 already `gandr-*`.
Remaining `wyrd-*`: `wyrd-rust-gates` (tooling), `wyrd-dylint` (lints).
Both are build/CI tooling with no calculus role; the rename decision is independent of the language crates.

**(2) Bead-ID citations.** 126 distinct `wyrd-*` bead IDs appear in comments/docs across the crates (raw `grep -oE 'wyrd-[a-z0-9]{3,5}'` over `src/**`), heaviest in (mentions): `wyrd-rust-gates` 205, `gandr-pipeline` 150, `gandr-core` 98, `gandr-grammar` 90, `gandr-parser` 75, `gandr-sequent` 41, `gandr-tui` 34, `gandr-lsp` 25, `gandr-vdc` 23.
Five crates cite **zero**: `gandr-recursion`, `gandr-nominal`, `gandr-kernel-levels`, `gandr-grammar-contract-fixtures`, `gandr-data` (these are the cleanest to port verbatim).
These IDs point at the retired wyrd tracker; the port must choose a policy (rewrite → gandr beads / drop / keep as provenance).
Contributor-vs-project-concern note: many of these are workflow forensics (bead numbers) that arguably should not carry into the reboot's tracked history.

**(3) "Wyrd" branding residue** (not bead IDs): `gandr-recursion/src/lib.rs:1` ("structurally recursive **Wyrd** algorithms") and its manifest description (" **Wyrd**-owned first-order recursion engine"); `dylint_lib = "wyrd_dylint"` test attributes in `gandr-sequent/tests/{focus_properties,differential}.rs` (5 sites).

**(4) Agentic-dev references.** No `agentic`/`swarm`/`.agents` strings in crate _source_ (`crates/**.rs`) — the agentic-target framing lives only in docs (`wyrd@failed-refactor:docs/adr/0026-agentic-target-and-swarm-strategy.md`).
ADR-26 frames gandr as a control plane/runtime for a distributed agentic computing environment, with the doc corpus as swarm substrate and property tests/mechanization /per-stage gates as the propose-dispose trust mechanism.
Relevant to the port only as governing intent, not code coupling.

**(5) wyrd-workflow coupling in the gate crate.** `wyrd-rust-gates/src/project.rs` hardcodes an IU (internal-univalence) submodule pin at `metatheory/upstream/internal-univalence` (`…project.rs`, `DEFAULT_IU_PATH` + `iu-pin-*` finding kinds) and a `FORBIDDEN_DEFAULT_GRAPH_PACKAGES` list (`…project.rs:63–70`: tree-sitter, gandr-tree-sitter, regex, regex-automata, regex-syntax, aho-corasick).
This crate encodes wyrd's _specific_ CI policy and is the most reboot-coupled artifact after the driver.

---

## 5. Feature-flag audit vs the proposed scope-prefixed scheme

**The proposed scheme is not present anywhere.** Grep for `gandr_core_*` / `gandr_feat_*` / `gandr_tool_*` / `gandr_test_*` / "scope-prefix" across `docs` + `crates` returns **no design reference** (the only `gandr_test_` hits are FFI test function names in `gandr-ffi/tests/ffi.rs`, unrelated).
Treat the scheme as a greenfield target to impose during the port, not a pattern to preserve.

**Current posture:** almost every crate has `default=[]` + `full=[]`; a minority carry real optional features that `full` aggregates.
The scheme's rule ("every crate has `default` + `full`; `full` = all features except `fuzzing`") requires these **non-conformances / bugs** to be fixed:

| Crate                 | Issue                                                                                                                                                                                                                                                                                      | Source                                                                                                 |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| **gandr-sequent**     | `[features]` has `default=[]` and **`features=[]`** — a feature literally named `features`, almost certainly a typo for `full`. No `full` exists.                                                                                                                                          | `wyrd@failed-refactor:crates/gandr-sequent/Cargo.toml:25–27`                                           |
| **gandr-grammar**     | `full=[]` but the inline comment says "`--features=full` activates [parity]"; the main nextest lane runs `--workspace --features=full` yet `full` does **not** include `parity`. Either `full` should be `["parity"]` or the comment is stale. **Direct contradiction — see hazard H1.**   | `wyrd@failed-refactor:crates/gandr-grammar/Cargo.toml:51–59`; `wyrd@failed-refactor:mise.toml:631–639` |
| **gandr-lsp**         | `full=[]` is empty, but `codecs` is a whole-crate gate (the `gandr-lsp` binary is `required-features=["codecs"]`). So `full` builds an empty library and never the server — violates "full = all features."                                                                                | `wyrd@failed-refactor:crates/gandr-lsp/Cargo.toml` `[features]`, `[[bin]]`                             |
| **gandr-recursion**   | No `[features]` section at all (no `default`, no `full`).                                                                                                                                                                                                                                  | `wyrd@failed-refactor:crates/gandr-recursion/Cargo.toml`                                               |
| **wyrd-dylint**       | No `[features]`; standalone `[workspace]`, `rustc_private`, not a workspace member — sits outside the scheme entirely.                                                                                                                                                                     | `wyrd@failed-refactor:crates/wyrd-dylint/Cargo.toml`                                                   |
| **gandr-tree-sitter** | Has `default`/`full` but is `exclude`d and does **not** inherit workspace `version`/`edition`/`license`/`lints` (hand-pinned). Outside the normal graph.                                                                                                                                   | `wyrd@failed-refactor:crates/gandr-tree-sitter/Cargo.toml`                                             |
| **wyrd-rust-gates**   | The **only** crate with a `fuzzing` feature — the exact case the scheme's "full excludes fuzzing" rule is written for. Currently `full=[]`, `fuzzing=[]` (already disjoint, so conformant in spirit; `full` just needs to aggregate the non-fuzzing set once features are scope-prefixed). | `wyrd@failed-refactor:crates/wyrd-rust-gates/Cargo.toml`                                               |

**Feature-name → scheme-bucket hints** (for the migration): `regex`, `codecs`, `ffi`, `parity` are capability toggles → `gandr_feat_*`; `proptest-strategies` (gandr-core) exposes test generators to other crates → `gandr_test_*`; `fuzzing` (wyrd-rust-gates) → `gandr_test_*` and excluded from `full`.
`codecs` doubling as a whole-crate gate in `gandr-lsp` (vs an additive toggle in gandr-data/pipeline/ render-proto) is an inconsistency to normalize.

---

## 6. External-dependency surface (port-risk signal)

Beyond the workspace, the crates pull: `no-panic` (gandr-core, cfg-gated), `petgraph`+`fixedbitset` (gandr-graph), `thiserror`/`serde`/`serde_json`/`toml`/ `yaml-rust2` (broadly), `regex` (opt, gandr-core), `criterion`/`proptest`/`insta` (dev).
The concentrated-risk stacks:

* **gandr-tui**: `ratatui`, `crossterm`, `edtui`, `color-eyre`, `nucleo-matcher` (MPL-2.0, flagged in the workspace manifest), `portable-pty`, `tui-term`, `vt100` — 8 pinned UI crates, each with load-bearing feature choices documented inline in `wyrd@failed-refactor:Cargo.toml` `[workspace.dependencies]`.
* **gandr** (driver): `reedline` (nushell line editor) + `nu-ansi-term`.
* **gandr-ffi**: `libffi` + `libloading` + `cc` — native, unsafe, builds a bundled C fixture; entirely behind the `ffi` feature.
* **gandr-tree-sitter**: `tree-sitter` + `cc` (compiles `parser.c`) — the reason it is `exclude`d from the default graph.
* **wyrd-dylint**: `clippy_utils` via a **git rev pin** (`9fca3bc9…`) + `rustc_private` — toolchain-coupled, will drift with the compiler.

---

## 7. The five targeted questions

### (a) L-machine parity gaps in `gandr-sequent`

Context: `gandr-sequent` is sequent-machine phase **L0** (command IL + focusing `𝓕`), with the **L1** iterative arena machine _partial_ — the two-region store and pure-spine checkpoint landed behind the `L-run ∘ 𝓕 ≡ run` differential, effects/ control/native "substantially advanced" but not full L1 promotion (`wyrd@failed-refactor:docs/gandr/spec/proposal-sequent-kernel.md:3–8`, tracked on `wyrd-4xtv` / `wyrd-5qdq.1`).
The CEK evaluator (`gandr_core::eval::run`) is the external oracle (`…/gandr-sequent/src/differential.rs:1–12`).

**Currency (B1 stage F):** the CEK evaluator above has since retired in the reboot port — the L machine (`crates/core-sequent`, `machine.rs`) is now the sole operational driver, anchored by checked-in corpus outcome snapshots and property-differential snapshots rather than a live CEK oracle.
The CEK-oracle references in this subsection — this one and the A3 cross-lane rule below — describe the `wyrd@failed-refactor` source tree as read at port-planning time, not current gandr reality.

Three named residual seams — each a _defined_ `StuckReason::UnsupportedByReference`, never a panic:

**A1 — the un-focusing readback residual.** The focused IL discards the source- syntax bodies `𝓕` consumed, so anything that is not a first-order value cannot be structurally read back; the inverse translation ("un-focusing", `𝓕⁻¹`) is unbuilt.
Concretely:

* Higher-order native combinators — `each`/`where`/`reduce`/`any`/`all`/ `update_where` — need their thunk argument's source body and therefore **decline** (`…/machine.rs:1207` in `dispatch_native`; predicate `native_needs_unfocus` at `…/machine.rs:1822–1834`; module header `…/machine.rs:40–51`).
  A first-order prim never inspects a thunk body, so it dispatches; thunks cross the host seam as indexed readback markers and are resolved back to the original machine thunk (identity + call-by-need cell survive; comment `…/machine.rs:1196–1206`).
* The differential compares thunks / bare functions / lazy pairs / reified stacks / partial natives at **KIND granularity**, not structurally — their exact readback "needs _un-focusing_ the IL back to source syntax … a listed residual seam, not a semantic divergence" (`…/differential.rs:23–38`; `CanonOutcome`/`CanonValue` variants `Function`, `LazyPair`, `Thunk(Grade)`, `Stk`, `Native`).
  A thunk's **grade** is compared exactly (carried by `wyrd-5v6i`); only the body is opaque.
* Terminal readback fills un-focused copattern/cocase arm bodies with holes/ placeholders (`…/machine.rs:1564–1600`).
* **Closing it:** build the `𝓕⁻¹` un-focusing / readback of IL → source syntax (the `wyrd-4xtv`-era L1 work).
  Once landed, the six higher-order natives dispatch and the differential upgrades those kinds from opaque to structural comparison.

**A2 — prelude free-name resolution in force position.** A free/neutral producer name forced against the prelude should resolve per ADR-42, but the prelude path is not wired at this checkpoint; a force miss on a non-thunk returns `StuckReason::ForcedNonThunk` (`meet_force`, `…/machine.rs:1318`, `1336–1339`; the neutral `LValue` is documented as "a prelude binding … consulted at a force miss", `…/machine.rs:129–130`; header `…/machine.rs:65–67` — "prelude resolution … still grow[s] in later checkpoints").
**Closing it:** wire prelude lookup at the free-name force site so a forced prelude binding resolves instead of decoding as `ForcedNonThunk`.

**A3 — ADR-76 identity formers declined by the focusing translation.** Before focusing, `focus_comp`/`focus_value` scan the _entire_ program for identity formers (`Value::Here` / `Comp::Walk`, ADR-76); if any is reachable, the whole program focuses to **one** `FocusOrigin::Unsupported` decline — a hole producer cut against `★`, which the L machine runs to `UnsupportedByReference` (`…/focus.rs:1644–1706`: `focus_comp` at `1662–1672`, `unsupported_program` builder at `1681–1693`, `unsupported_former_scan` worklist at `1694+`).
This is a deliberate **cross-lane rule**: this lane does not build L-machine Walk-β, and a partial per-node hole fallback "would silently disagree with the CEK oracle on realized items rather than declining" — so it declines whole and defers the identity fragment's realization to the parallel sequent lane (`…/focus.rs:1644–1653`; proposal `…/proposal-sequent-kernel.md:7`).
That quoted "CEK oracle" retired at B1 stage F (subsection note above), leaving the L machine the sole driver.
**Closing it:** build the L-machine Walk-β / identity-former realization so `𝓕` translates `Here`/`Walk` node-by-node instead of declining the program.

**Adjacent (worth flagging).** The **ADR-80 declared-data eliminator** `Comp::DataCase` and the declared-data constructor value ride the _same_ whole-program decline as the identity formers (`…/focus.rs:1701–1703`, `1770–1781`).
So closing A3's mechanism should be scoped to also cover declared-data, or they remain a second whole-program decline class.
The pathological corpus already pins the `𝓕`-only-entry invariant with `unfocused-stuck.gandr` (`…/proposal-sequent-kernel.md:364`).

### (b) `gandr-grammar-contract-fixtures` fold-in feasibility

What it is: a **fixture locator**, not logic.
`#![no_std]`, **empty `[dependencies]`**, exposing only four `pub const`s — a repo-relative fixture-root path, a repo-relative manifest path, and two `include_str!`-embedded blobs (`MANIFEST_JSON`, `MANIFEST_SCHEMA_JSON`) — plus a `fixture_manifest` criterion bench and manifest-invariant tests (`wyrd@failed-refactor:crates/gandr-grammar-contract-fixtures/src/lib.rs:1–36`).
It owns a `fixtures/` tree of `.gandr` sources + `manifest.{json,schema.json}` and its own `docs/` set.
Its **only** consumer is a **dev-dependency** edge from `gandr-pipeline` (`…/gandr-pipeline/Cargo.toml` `[dev-dependencies]`).

Feasibility: **mechanically easy, with one real coupling to redesign.**

* Pros: zero production deps, zero production dependents (single dev-edge), tiny.
  Folding it into `gandr-grammar` as a `fixtures` submodule (or a `gandr_test_*`- scoped feature) is low-risk; `gandr-pipeline`'s dev-dep would then target `gandr-grammar` (feature-gated).
* The coupling: the crate is a **cross-language contract**.
  The manifest is JSON "so Node consumers can read the same file by path convention" and the path constants are **repo-relative** strings (`crates/gandr-grammar-contract-fixtures/…`, `…/src/lib.rs:3–6`, `24–31`).
  It pairs with `packages/tree-sitter-gandr/`.
  Folding it in changes those repo-relative paths and forces `gandr-grammar` (a lean grammar core) to carry fixture bytes + a criterion bench.
* **Recommendation for the coordinator:** feasible either way; if kept separate, rename/rescope it as a test-fixtures crate under the `gandr_test_*` bucket.
  If folded in, treat the repo-relative path constants and the Rust/Node shared-manifest contract as the migration's real work, not the code move.

### (c) `gandr-kernel-levels` and `gandr-data` wiring intent (no inbound edges)

Both are **intentional orphans awaiting an unbuilt consumer** — not dead code.

* **gandr-kernel-levels** is the _first_ `gandr-kernel-*` subcrate (kernel-boundary slice 1+2, ADR-78), deliberately holding **levels only** with a hard TCB dependency wall — "no terms, no types, no universe rule (the rule `U_l : U_m` iff `l < m` is one call into `Level::lt`, and belongs to the **kernel-core crate**)" (`wyrd@failed-refactor:crates/gandr-kernel-levels/src/lib.rs:1–24`).
  The consumer is the future certified **kernel-core** crate, which is not built: the frozen interpreter core (`gandr-core`) carries only the `{0,+1}` _former_, "no first-class `Γ ⊢ A : U_l` type-formation judgment (that judgment is the kernel's S2 job)" (`…/docs/gandr/spec/core-ir-contract.md:102`; ADR-81 reconciliation `…/docs/adr/0081-…:11,19`; `…/docs/gandr/spec/kernel-boundary.md:6–8,45,139–140`).
  So the missing inbound edge is expected: it awaits kernel S2.
  **Reboot update (B2, 2026-07-21):** the awaited consumer now exists — `gandr-kernel-core` (`crates/kernel-core`, landed B2.1) consumes `gandr-kernel-strata`, the reboot home of the kernel-levels content (`Level`/`lt`/`LandmarkPoset`, ADR-78), as its only dependency per the kernel-boundary §2 TCB wall.
  The orphan status is discharged in the reboot tree; the `wyrd@failed-refactor` description above stands as source-material history.
* **gandr-data** is a pure codec crate (JSON/TOML/YAML ↔ typed gandr `Value` over the **public** value constructors only; ADR-35 D6 renderer/encoder firewall), touching no part of the calculus (`wyrd@failed-refactor:crates/gandr-data/src/lib.rs:1–34`).
  Its intended consumer is the **shell / self-hosting bootstrap** lane: porting `scripts/*.nu` to gandr is what exercises the codecs (`…/docs/gandr/spec/proposal-shell-usage-surface.md:30,35,89`; `…/docs/gandr/spec/proposal-self-hosting.md:50,87`).
  That lane's driver does not yet depend on it, so it currently dangles. (Note: `gandr-data` cites **zero** `wyrd-*` bead IDs — clean to port.)

### (d) tree-sitter excision state

**Demoted to reference-only, not deleted.**

* `gandr-tree-sitter` is `exclude`d from the workspace (both listed in `members` and `exclude`; the manifest comment explains it must be `exclude`d rather than merely dropped, because `gandr-grammar`'s optional path dep would otherwise make it an implicit member) and is reachable **only** through `gandr-grammar`'s non-default `parity` feature (`wyrd@failed-refactor:Cargo.toml` workspace comment; `…/crates/gandr-grammar/Cargo.toml:29–59`).
* The **tree-sitter grammar itself still exists**: `packages/tree-sitter-gandr/` (`grammar.js`, `src/node-types.json`, `queries/`, `test/`) is the reference the differential-parity lane checks the PBG labeler/mold highlighter against.
* **Parity lanes that still reference tree-sitter:**
  + `gandr-grammar` tests: `tests/token_stream_parity.rs`, `tests/highlight_parity.rs` (E1/E2 differential, `#[cfg(feature="parity")]`), plus `tests/node_types_gate.rs` and `tests/surface.rs`, which gate the committed `node-types.json` against `gandr_grammar::TREE_SITTER_NAMED_KINDS` (`…/tests/node_types_gate.rs:4–81`, `…/tests/surface.rs:35,108–120`).
  + `fuzz/fuzz_targets/parity.rs` (the AFL parity target).
* **Residue elsewhere:** `gandr-parser/src/{label.rs,mold.rs}` keep tree-sitter _documentation comments_ (the labeler mirrors gandr's tree-sitter lexemes) but no dependency; `gandr-corpus/src/lib.rs:607` documents the "retired tree-sitter parser" replaced by the melder push-machine.
* **Enforced default-graph exclusion:** `wyrd-rust-gates/src/project.rs:63–70` hardcodes `FORBIDDEN_DEFAULT_GRAPH_PACKAGES = [tree-sitter, gandr-tree-sitter, regex, regex-automata, regex-syntax, aho-corasick]`, and the retained `check-default-graph-tree-sitter-free.nu` policy (`…/project.rs:147`) fails the build if the default `-e normal,build` graph pulls any of them (bead `wyrd-66jm`).

### (e) where formatting should live

**Contradiction with the premise, recorded explicitly:** the question says "a `gandr-fmt` crate was planned; `gandr-pretty` never existed."
A dedicated proposal — `wyrd@failed-refactor:docs/gandr/spec/proposal-pretty-printing.md` (facet C, ADR-46 D) — plans a **three-crate split**, and it _does_ name `gandr-pretty`:

* `gandr-doc` — shared PrettyExpressive-style layout VM (arXiv:2310.01530) (`…/proposal-pretty-printing.md:5,18–19,100–107`).
* `gandr-fmt` — canonical **CST/source** formatter; depends on `gandr-doc`/`gandr-parser`/`gandr-grammar`/`gandr-syntax`, and explicitly **not** on `gandr-core`/checker/`gandr-pipeline`/`gandr-tree-sitter`/`gandr-pretty` (`…:100–103,567,700–702`; ADR-35 D6 renderer firewall; ADR-62 bead `wyrd-d71t`).
* `gandr-pretty` — core **presentation** printer for the REPL/diagnostics (values/types/computations/goals/marks/stacks); depends on `gandr-doc`+`gandr-core`
  + checked pipeline carriers; neither core nor pipeline depend back
  (`…:19,106–107,127`).

So the accurate statement is: **`gandr-pretty` is a _planned_ crate that has no code yet** — true only at the code level.
None of `gandr-doc`, `gandr-fmt`, `gandr-pretty` exist in `crates/` today.

**Where pretty/render actually lives now (inline and scattered):**

* `gandr-sequent/src/pretty.rs` — the command-IL pretty-printer (§2.1 notation, depth-bounded).
* `gandr-pipeline/src/render.rs`, `gandr-render-proto/src/present.rs`, `gandr-tui/src/{present.rs,render.rs}`, `gandr/src/render.rs`, `wyrd-rust-gates/src/coverage/render.rs`.

**Recommendation for the coordinator:** the target home is the three-crate lane (`gandr-doc`/`gandr-fmt`/`gandr-pretty`); the port should treat the existing `gandr-sequent/src/pretty.rs` and the various `render.rs` modules as **candidates to consolidate** behind `gandr-doc`/`gandr-pretty` (respecting the ADR-35 D6 firewall: formatters/printers parse/lower/type/mark nothing).
This is a _graduation_ of an inline facet into a component — flag for a scope decision, since no such crate exists to port into.

---

## 8. Port-complexity assessment

Tiers by dependency depth, external/unsafe surface, and coupling.
"Verbatim-clean" = zero `wyrd-*` bead IDs cited.

* **Tier 0 — trivial leaves (port first, verbatim).** `gandr-nominal`, `gandr-recursion`, `gandr-kernel-levels` (all verbatim-clean, tiny, `#![no_std]` or near), `gandr-render-proto`, `gandr-order-maintenance`, `gandr-syntax`, `gandr-graph` (petgraph adapter is hidden), `gandr-data` (verbatim-clean, orphan), `gandr-grammar-contract-fixtures` (verbatim-clean; see §7b for the fold-in choice).
* **Tier 1 — core, moderate.** `gandr-core` (38k LOC but self-contained on `gandr-nominal`; the checker↔machine step-agreement anchor is the porting risk if split), `gandr-desc`, `gandr-grammar` (+ the parity feature wart, H1), `gandr-parser`.
* **Tier 2 — the hub.** `gandr-pipeline` (27k LOC, 8 workspace deps, 150 bead-ID citations) — the integration center; port after all its deps.
* **Tier 3 — research lanes (staged).** `gandr-sequent` (three tracked residuals, §7a), `gandr-polygraph` (L2, not started), `gandr-vdc`.
  Portable as-is but carry known open work.
* **Tier 4 — surfaces with heavy external stacks.** `gandr-shell`, `gandr-ffi` (unsafe, libffi/cc, feature-isolated), `gandr-lsp` (codecs/full wart), `gandr-tui` (8 UI crates), `gandr` driver (reedline).
* **Tier 5 — tooling, decouple decision needed.** `wyrd-tree-sitter`/reference lane (C compile, `exclude`d), `wyrd-rust-gates` (37k LOC, encodes wyrd CI policy incl. the IU submodule pin — needs a re-home/rewrite decision), `wyrd-dylint` (rustc_private, git-pinned `clippy_utils`, standalone workspace — most toolchain-fragile).

---

## 9. Hazards & surprises for the coordinator

* **H1 (feature-graph contradiction).** `gandr-grammar` `full = []`, but its own comment and the workspace-root manifest comment assert `--features=full` activates the `parity` lane; the main gate runs `cargo nextest run --workspace --all-targets --features=full` (`wyrd@failed-refactor:mise.toml:631–639`).
  Since `full` does not include `parity`, the E1/E2 tree-sitter differential-parity tests are `#[cfg(feature="parity")]`-compiled-out under that command and **may never run in the main lane** — the differential could be silently inert.
  I did not execute cargo to confirm feature unification; recorded as a contradiction to verify, not harmonized.
  Fix is either `full = ["parity"]` or correcting the comment + adding an explicit parity lane.
* **H2 (typo'd feature).** `gandr-sequent` declares a feature literally named `features` (`Cargo.toml:27`), almost certainly a botched `full`.
  Any `--features full -p gandr-sequent` today errors.
  Trivial fix; flag because the scheme migration will trip over it.
* **H3 (`gandr-lsp` full is a no-op).** `full=[]` while `codecs` gates the entire server (bin is `required-features=["codecs"]`).
  Building `gandr-lsp --features full` yields an empty library and no server — the scheme's `full` must pull `codecs`.
* **H4 (count mismatch).** 24 `gandr-*` crates, not 20; total 26 with the two `wyrd-*` crates.
  If the epic's plan is sized for 22, it is under-scoped by 4 crates.
* **H5 (members ∩ exclude).** `gandr-tree-sitter` and `fuzz` appear in **both** `members` and `exclude` in the workspace manifest (intentional per the inline comment, to defeat implicit membership via an optional path dep).
  This is an unusual Cargo pattern the reboot's workspace file must replicate deliberately or it will re-drag tree-sitter/regex into the default graph and trip the `wyrd-rust-gates` gate.
* **H6 (`gandr-pretty` premise wrong).** The task states `gandr-pretty` "never existed"; the spec plans it (with `gandr-doc`) — see §7e.
  Planning a port should not silently drop the two-of-three unbuilt formatting crates.
* **H7 (wyrd tracker debt).** 126 `wyrd-*` bead IDs are woven through comments/docs of the crates being ported.
  Leaving them is stale-reference debt; rewriting them is a large mechanical pass.
  Needs an explicit policy (and note the contributor-concern angle: bead forensics arguably should not enter the reboot's tracked history).
* **H8 (`wyrd-rust-gates` / `wyrd-dylint` are wyrd-CI-shaped).** The gate crate hardcodes an IU submodule pin and a forbidden-package list; the dylint crate is rustc_private with a git-pinned `clippy_utils`.
  Both encode wyrd's exact toolchain/ workflow and are the least "just port it" artifacts — decide re-home vs rewrite vs drop before sequencing them.
* **H9 (adjacent decline class).** Beyond the three question-(a) gaps, `gandr-sequent` also declines the **ADR-80 declared-data eliminator** (`Comp::DataCase`) whole- program by the same cross-lane rule (§7a).
  Any L1 plan that closes A3 should scope declared-data in too, or it stays a second whole-program `UnsupportedByReference` class.

```text
```

## 10. Storage-tier absorptions (mach prolly-bao → gandr `storage-*`, gandr-5t3 RQ-9)

> Not `wyrd@failed-refactor` ports: these two crates are absorbed directly from the owner's unpublished `mach` `prolly-bao` work (Apache-2.0, same owner; source commit `fb78601`) into the ratified `storage-*` tier (`massive-term-design.md` §6.1).
> Skeleton landing — crates plus their contract suites, no export-path wiring, no `rkyv` crate, no new features.
> `prolly-bao-cli` is dropped (its dogfood value lives in the carried contract suites).

| Crate (gandr)                  | Source (`mach@fb78601`)     | Role                                                               | Key public surface                                                                                                       | External deps                            | Features                       |
| ------------------------------ | --------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------- | ------------------------------ |
| **gandr-storage-chunker**      | `crates/prolly-bao-chunker` | `no_std`, zero-runtime-dep record-safe chunker; 85-byte commitment | `ChunkerParams`, `ChunkLimits`, `chunk_record_slices`, `chunk_spans`, `PARAMETER_COMMITMENT_LEN`                         | none (empty `[dependencies]`)            | `default`                      |
| **gandr-storage-prolly-trees** | `crates/prolly-bao`         | `alloc` ordered-record Merkle tree, proofs, and block stores       | `ProllyTree`, `BlockStore`, `InMemoryBlockStore`, `PackedSegmentStore`, `NodeHash`, `TreeParams`; proofs (feat `proofs`) | `blake3`, `thiserror`, `storage-chunker` | `default = [proofs]`, `proofs` |

## 11. Storage-tier native crates (gandr-5t3 B2.3 outer-layer wiring)

> Not an absorption — `gandr-storage-artifact` is gandr-native code landed at B2.3 (`massive-term-design.md` §6): the outer-layer CAS wiring and manifest identity that consumes the ratified `storage-*` tree/chunker as a sorted-record consumer.
> It is untrusted plumbing by the kernel-boundary naming rule; hashing lives here, outside the kernel TCB.

| Crate (gandr)              | Origin              | Role                                                                                                       | Key public surface                                                                                      | External deps                                                                   | Features  |
| -------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | --------- |
| **gandr-storage-artifact** | gandr-native (B2.3) | Outer-layer CAS wiring: v1 export artifacts as sorted keyed declaration records + BLAKE3 manifest identity | `ArtifactRecord`, `ArtifactRecordSet`, `ArtifactManifest`, `ArtifactIdentity`, `BuiltArtifact`, `build` | `blake3`, `thiserror`, `kernel-core`, `storage-chunker`, `storage-prolly-trees` | `default` |

## 12. Naming authority — the wyrd → reboot rename table (owner-ratified 2026-07-21)

Adapted from the owner's original `PROMPT.md` rename table (repo root, the reboot commissioning prompt), reconciled against what is actually built, and generalized in `docs/workflow/rust.md` §conventions ("Crate naming follows the category schema").
**Consult this table before minting any crate**; a crate not listed derives its name from the schema (`<category>-<name>` directory, `gandr-<directory>` package) and adds a row here in the same change.
Divergences from the original suggestion are recorded, not smoothed.

| wyrd crate                        | reboot directory                 | status / note                                                                                                    |
| --------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `gandr-core`                      | `core-checker`                   | built (B1)                                                                                                       |
| `gandr-sequent`                   | `core-sequent`                   | built (B1); "should sequent stay a separate crate?" remains an open owner note from `PROMPT.md`                  |
| `gandr-kernel-levels`             | `kernel-strata`                  | built (B2); **diverges** from the suggested `theory-universes` — the level oracle is TCB substrate, kernel-side  |
| —                                 | `kernel-core`                    | reboot-native (B2), no wyrd source                                                                               |
| `gandr-nominal`                   | `theory-nominal-automata`        | built                                                                                                            |
| `gandr-graph`                     | `theory-graphs`                  | built                                                                                                            |
| `gandr-desc`                      | `theory-levitation`              | built                                                                                                            |
| `gandr-recursion`                 | `theory-recursion`               | built; schema-derived (not in the original table)                                                                |
| `gandr-order-maintenance`         | `theory-orders`                  | built                                                                                                            |
| `gandr-polygraph`                 | `theory-computads`               | built                                                                                                            |
| `gandr-vdc`                       | `theory-virtual-doctrines`       | built                                                                                                            |
| —                                 | `theory-stone-duality`           | planned (original table), no wyrd source                                                                         |
| `gandr-shell`                     | `runtime-host`                   | built (B1); **diverges** from the suggested `runtime-effects`; source-runner faces re-wire at the front-end port |
| `gandr-data`                      | `runtime-codecs`                 | unported (shell/self-hosting lane)                                                                               |
| `gandr-ffi`                       | `runtime-ffi`                    | unported, deferred                                                                                               |
| `gandr-syntax`                    | `surface-syntax`                 | porting (front-end F0); schema-derived                                                                           |
| `gandr-render-proto`              | `surface-render-remote`          | porting (front-end F0); the remote/wire face beside the future `surface-render`                                  |
| `gandr-pretty`                    | `surface-render`                 | future — spec-planned presentation printer, no wyrd code; whether formatting also lives here is an open question |
| `gandr-grammar`                   | `surface-grammar`                | front-end F1; absorbs `gandr-grammar-contract-fixtures`                                                          |
| `gandr-parser`                    | `surface-parser`                 | front-end F2; schema-derived                                                                                     |
| `gandr-pipeline`                  | `surface-engine`                 | front-end F3; "clearer intent" over `-pipeline`                                                                  |
| `gandr-corpus`                    | `surface-corpus`                 | front-end F4                                                                                                     |
| `gandr` (driver)                  | `surface-driver`                 | stub built; un-stub at front-end F5                                                                              |
| `gandr-lsp`                       | `surface-lsp`                    | deferred                                                                                                         |
| `gandr-tui`                       | `surface-tui`                    | deferred                                                                                                         |
| `gandr-tree-sitter`               | `surface-tree-sitter`            | deferred (parity reference, F6); schema-derived                                                                  |
| `gandr-grammar-contract-fixtures` | — (folds into `surface-grammar`) | front-end F1                                                                                                     |
| `wyrd-rust-gates`                 | `workflow-gates`                 | re-homed reboot-native (not a port)                                                                              |
| `wyrd-dylint`                     | `workflow-dylint`                | re-homed reboot-native                                                                                           |
