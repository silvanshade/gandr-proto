# gandr spec-corpus survey (gandr-fcw.1)

A feature-area → authoritative-source map for the gandr design corpus as it lives in `wyrd@failed-refactor:docs/gandr/`, with per-area reference pointers, supersession/addendum tracking, an implemented-vs-intended read, and an explicit list of every conflict and staleness signal found.
Scope: the 39 files under `wyrd@failed-refactor:docs/gandr/spec/` plus `dictionary.md`, `dictionary.yml`, `status.yml`, `VISION.md`, `README.md` under `wyrd@failed-refactor:docs/gandr/`.
`MANIFEST.yml` was read as the registry index although not in the enumerated list.

**Citation convention.** All source paths resolve under the alias base `wyrd@failed-refactor:docs/gandr/` — i.e. a bare `README.md` means `wyrd@failed-refactor:docs/gandr/README.md`, and `spec/core-ir-contract.md` means `wyrd@failed-refactor:docs/gandr/spec/core-ir-contract.md`.
Paths outside that base are written with an explicit prefix (e.g. `wyrd@failed-refactor:docs/adr/`).
Section (§) and `:line` anchors are given where they pin a claim.
No absolute or machine-local paths appear in this file.

---

## 1. Executive summary

* **Authority is layered and self-declared, and the layering is coherent.** The corpus itself is authoritative over the rest of the repo (`README.md` line 9).
  Within the corpus the _implementation-truth_ arbiter for the ported core IR is `spec/core-ir-contract.md` §0; `status.yml` is an explicit _consolidated view, not an arbiter_ (`status.yml` lines 8-17); `MANIFEST.yml` is the complete registry with typed provenance edges and gate-enforced BLAKE3 hashes; `dictionary.yml` (front-door `dictionary.md`) is the one-name-per-construct naming authority.
  Where any consumer disagrees with an arbiter, the arbiter wins.
* **MANIFEST.yml is current (verified).** BLAKE3 spot-checks of `core-ir-contract.md`, `status.yml`, and `proposal-parser-interaction-core.md` match MANIFEST exactly, so the hash gate is live and MANIFEST (last touched 2026-07-16) reflects the tree.
  Trust MANIFEST's edge graph.
* **`status.yml` is the single biggest staleness surface.** It was committed 2026-07-13 23:05, _before_ ADR-80/81/82 landed (2026-07-14 03:37–11:14).
  Its core rows therefore omit the frozen-core declared-data former (Data/Ctor/DataCase, ADR-80), the levitation stage-1 dependent core (`Type` universe + `Σ` pair, ADR-81), and the ADR-82 split-motive — all of which `core-ir-contract.md` §0 lists as **Live**.
  Arbiter (§0) wins; `status.yml` has drifted.
  See §7.1.
* **`proposal-diagnostics-architecture.md` has no `status.yml` row at all** — it is absent from the registry despite being an _Accepted implementation synthesis_ (Report v3, a new `gandr-diagnostics` crate) bound into the corpus.
  Coverage gap.
  See §7.3.
* **Later-doc-wins is mostly moot** because the corpus resolves its own conflicts in-band via supersession banners rather than leaving contradictory prose.
  The one governing pivot — **ADR-77 (2026-07-12) making the _minimal certified kernel_ the governing focus, superseding ADR-35's shell-usage focus** — is applied consistently across `README.md`, `roadmap.md`, `proposal-shell-usage-surface.md`, and `proposal-self-hosting.md`.
* **The parser lane authority is split by design:** `proposal-parser-interaction-core.md` (Adopted; P1–P3 as-built) _revises the W2–W4 execution arc of_ `proposal-graph-core.md` §8; and the **PBG + melder push parser is now the normative parser, retiring tree-sitter's authority** (`proposal-surface-syntax.md` as-built amendment; ADR-70/73/75). tree-sitter is retained as parity/editor tooling.
* **The four `*-addendum-vdc.md` files are additive, not superseding.** All four are VDC-reflection deltas realizing ADR-68/69; they touch _later_ stages (L2, stage-1, the negative surface) and the data-patterns one is explicitly "confirmations…the MVP cut is untouched."
  None changes what is built today.
  See §5.
* **Implemented-vs-intended is well-instrumented.** `core-ir-contract.md` §0 carries a per-shape Live/frozen-unbuilt table for the core; `status.yml` carries a five-valued stance (built / partial / adopted-unbuilt / design-pass / dormant) per area.
  The built surface is a CBPV checker + derived typing machine + A5.1 CK evaluator + PBG parser lane + value-model rungs 1–4 + rung-1 identity + declared-data 1-cell MVP + levitation stage-1 core + sequent L0 + kernel level oracle + corpus.
  Nearly everything theoretical (PROP fabric, polygraph data, erasible evidence, temporal univalence, sessions, worlds, modules, metaprogramming, backend, packages, pretty-printing) is _adopted-unbuilt_ or _design-pass_.
* **An adjacent structure sits just outside scope:** `wyrd@failed-refactor:docs/manual/` (Typst reference manual) is present and populated; many specs declare their content "absorbed 2026-07-13" into a `docs/manual/chapters/<x>.yml` chapter while asserting "this file remains the authoritative design record."
  The manual _renders_ `status.yml` directly.
  The worktree name ("failed-refactor") plus this dual spec/manual structure is worth the coordinator's attention (§9).
* **One explicit tracker/doc drift is self-declared:** `proposal-parser-interaction-core.md` line 304 states "some already-written bead texts carry superseded stances… the tracker and this proposal disagree in places."
  The beads tracker is out of this survey's scope but the coordinator should treat parser-lane bead text as suspect.

---

## 2. The authority model (how to resolve any conflict)

Precedence, highest first.
This is reconstructed from the docs' own statements, not imposed.

| Rank | Source                                                    | Role                                                                                      | Self-statement                                                                                                                    |
| ---- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 0    | `README.md`                                               | Corpus-level authority over the whole repo                                                | "where it conflicts with other documents in this repository, the corpus is correct" (`README.md:9`)                               |
| 1    | `spec/core-ir-contract.md` §0                             | **Implementation truth** for the ported core IR (which shapes are Live vs frozen-unbuilt) | "the single source of implementation truth" (`core-ir-contract.md:35`); §0 registry "stays the live build-status arbiter" (`:10`) |
| 1    | Each spec's own **status / as-built banner**              | Rule-level / area-level truth, amended at each landing                                    | e.g. `effects-control-shell.md:3`, `proposal-sequent-kernel.md:3-6`                                                               |
| 2    | Crate reality (`crates/gandr-*` + tests + `gandr-corpus`) | Ground truth of what runs                                                                 | named per area in `status.yml` `carriers`/`evidence`                                                                              |
| 3    | `MANIFEST.yml`                                            | Complete registry: roles + typed provenance edges + BLAKE3 gate                           | "the complete registry… gate-enforced" (`README.md:55`)                                                                           |
| 3    | `dictionary.yml` (+ `dictionary.md`)                      | Naming authority; `status: core` = **contract membership, not implementation**            | `dictionary.md:61`                                                                                                                |
| 4    | `status.yml`                                              | **Consolidated view, explicitly NOT an arbiter**; must agree with ranks 1-2               | "This file is a CONSOLIDATED VIEW, not a new arbiter" (`status.yml:8-17`)                                                         |

Two cross-cutting rules the corpus states about itself:

* **"One source per fact"** (`core-ir-contract.md:26`): where the contract and a home spec (`type-system.md`, `effects-control-shell.md`) state the same shape, the home spec owns the _rule_ and the contract owns the _frozen-data view_; drift between them is a tracked bug.
* **Frozen ≠ unilateral** (`core-ir-contract.md:20`): a §§1-6 shape change goes through an ADR + bead + dictionary resync, never a one-track edit.

---

## 3. Chronology (basis for later-doc-wins)

Last-commit dates (author time) for every surveyed file, oldest → newest.
Because supersession is handled in-band, dates mainly matter for spotting a stale doc that predates a decision it should reflect (the ADR-80/81/82 line is the one that bites — see §7.1).

| Date (2026)     | File                                                                                                                                                                                 | Note                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| 07-09 17:08     | `spec/proposal-metatheory-relaunch.md`                                                                                                                                               | oldest                                               |
| 07-12 22:43     | `spec/proposal-data-patterns-addendum-vdc.md`                                                                                                                                        | (banner authored-date 07-09)                         |
| 07-13 09:44     | `VISION.md`                                                                                                                                                                          |                                                      |
| 07-13 14:56     | `spec/kernel-boundary.md`                                                                                                                                                            |                                                      |
| 07-13 15:02     | `spec/proposal-sequent-kernel(-addendum-vdc).md`                                                                                                                                     |                                                      |
| 07-13 15:11     | `spec/proposal-codata-corecursion(-addendum-vdc).md`, `spec/proposal-levitation(-addendum-vdc).md`                                                                                   |                                                      |
| 07-13 15:19     | `spec/proposal-graph-core.md`                                                                                                                                                        |                                                      |
| 07-13 22:30     | `spec/proposal-metaprogramming.md`                                                                                                                                                   |                                                      |
| 07-13 22:35     | `spec/proposal-solver-interface.md`                                                                                                                                                  |                                                      |
| 07-13 22:47     | `spec/proposal-{compilation-backend,packages,self-hosting,shell-usage-surface,wasm}.md`                                                                                              |                                                      |
| 07-13 22:56     | `spec/{effects-control-shell,incremental-pipeline}.md`, `spec/proposal-{attributes,data-patterns,ffi,operators,recursion-iteration,surface-syntax,term-face,value-semantics-mvp}.md` |                                                      |
| **07-13 23:05** | **`status.yml`**                                                                                                                                                                     | **predates ADR-80/81/82 (see §7.1)**                 |
| 07-14 02:47     | `spec/proposal-pretty-printing.md`                                                                                                                                                   |                                                      |
| 07-14 04:42     | `spec/proposal-{diagnostics-architecture,inspection-protocol}.md`, `README.md`, `spec/roadmap.md`                                                                                    |                                                      |
| 07-14 06:48     | `spec/proposal-vdc-reflection.md`                                                                                                                                                    |                                                      |
| **07-14 11:14** | **`spec/core-ir-contract.md`**, `dictionary.md`, `dictionary.yml`, `spec/type-system.md`, `spec/typing-machine.md`                                                                   | same commit as ADR-82 landing; reflects ADR-80/81/82 |
| 07-14 11:16     | `spec/modules.md`                                                                                                                                                                    |                                                      |
| 07-14 22:58     | `spec/proposal-identity-univalence.md`                                                                                                                                               |                                                      |
| **07-16 23:31** | **`MANIFEST.yml`**, `spec/proposal-parser-interaction-core.md`                                                                                                                       | newest; MANIFEST hashes verified current             |

Relevant ADR landing commits: ADR-80 declared-data `cd1945b9` (07-14 03:37), ADR-81 levitation stage-1 `6ee8e343` (07-14 05:18), ADR-82 split-motive `457836b7` (07-14 11:14).
HEAD as surveyed is `df3ef8fe` (2026-07-18); the 07-15…07-18 commits are Rust crate lint-gate restorations, not doc changes, so the corpus is stable as of 07-16.

---

## 4. Feature-area → authoritative-source map

The backbone is `status.yml`'s `areas:` registry (one row per area, with `stance`, `sources`, `adrs`, `carriers`, `evidence`), cross-checked against each spec's own banner and `core-ir-contract.md` §0.
**Authoritative source** = the doc that owns the design of record for that area (the rule-level / decision-level home).
Stances are `status.yml`'s, corrected against the §0 arbiter where they drift (drift flagged → §7).

### 4.1 The ported / built core (arbiter: `core-ir-contract.md` §0)

| Area (`status.yml` id)                                            | Stance                                                                             | Authoritative source(s)                                                          | Also see / ADRs                  |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------- |
| `core-cbpv` — CBPV core + bidirectional checker                   | built                                                                              | `spec/type-system.md` (rules) + `spec/core-ir-contract.md` §§1-6 (frozen shapes) | ADR-1/28/31/32/38/39             |
| `grades` — semiring-graded thunks 0/1/ω                           | built                                                                              | `spec/type-system.md` §2 + `core-ir-contract.md` §4                              | ADR-4/28                         |
| `value-model` — strings/numerics/lists/records (rungs 1-4)        | built                                                                              | `spec/proposal-shell-usage-surface.md` §2 + `core-ir-contract.md` §0 rows        | ADR-38/39/40/42/45; roadmap §1.2 |
| `effects-handlers` — algebraic effects on `F`                     | built (typing + A5.1 runtime)                                                      | `spec/effects-control-shell.md` §1-2 + `core-ir-contract.md` §5                  | ADR-14/29/33/34                  |
| `control-stacks` — first-class stacks, reset/shift                | built (typing + A5.1)                                                              | `spec/effects-control-shell.md` §2 + `core-ir-contract.md` §6.3                  | ADR-25/29                        |
| `shell-dsl` — POSIX shell DSL + headless host runtime             | built                                                                              | `spec/effects-control-shell.md` (Part C)                                         | ADR-35/42                        |
| `identity` — Path/here/walk, no K                                 | **partial** (rung U1 Live)                                                         | `spec/proposal-identity-univalence.md` §2-4 + `core-ir-contract.md` §0           | ADR-76/79                        |
| **declared-data** (Data/Ctor/DataCase, 1-cell MVP)                | **Live** per §0 — but **no `status.yml` row** (→ §7.1)                             | `spec/core-ir-contract.md` §0/§2 + `spec/proposal-data-patterns.md` §3.4         | ADR-80, ADR-54 §3.4              |
| **levitation stage-1 core** (`Type` universe, `Σ` pair, `decode`) | **Live** per §0 — `status.yml` `levitation` row still says "stage 0 only" (→ §7.1) | `spec/proposal-levitation.md` §6 + `core-ir-contract.md` §0/§2                   | ADR-81; split-motive ADR-82      |

Note the `core-cbpv` `status.yml` row asserts only "§0 rows 1-3" are Live; the §0 table now has **13 rows**, with declared-data, `Type`/`Σ`, and split-motive as the newest Live rows.
That truncation is the drift in §7.1.

### 4.2 Parser lane + surface

| Area                                                   | Stance                               | Authoritative source                                                                                                                                               | Also see / ADRs                                                    |
| ------------------------------------------------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `surface-syntax` — ML-style Candidate-C surface        | built                                | `spec/proposal-surface-syntax.md` (ML surface) — **tree-sitter authority retired** by its own as-built amendment                                                   | ADR-44/73/75                                                       |
| `operators` — fixity, notation islands                 | **partial**                          | `spec/proposal-operators.md` (ADR-43 architecture)                                                                                                                 | ADR-43/75; machine-side `ResolveOperatorSeq` retired for PBG forms |
| `parser-core` — PBG/melder/mold/obligations            | **partial** (P1-P3 built)            | `spec/proposal-parser-interaction-core.md` (as-built, normative) — **revises W2-W4 of** `spec/proposal-graph-core.md` §8                                           | ADR-70/73/75                                                       |
| (graph substrate)                                      | design pass                          | `spec/proposal-graph-core.md` (the `gandr-graph` substrate, `PrecDag`, walk index) — substrate owner; execution-arc guidance superseded by parser-interaction-core | ADR-70                                                             |
| `interaction-surfaces` — driver/REPL/TUI/LSP           | built                                | `spec/proposal-parser-interaction-core.md` + `spec/proposal-inspection-protocol.md`                                                                                | ADR-62/64/74                                                       |
| `typing-machine` — defunctionalized step machine       | built                                | `spec/typing-machine.md`                                                                                                                                           | ADR-9/27 (ADR-27 supersedes pre-A1 core block); ADR-82 KSplitBody  |
| `incremental-pipeline` — checkpoints, resumable typing | **partial** (NO incremental parsing) | `spec/incremental-pipeline.md`                                                                                                                                     | ADR-10/17                                                          |

### 4.3 Type-system extensions beyond the ported core (all adopted-unbuilt)

| Area                                                               | Stance          | Authoritative source                                                                                     |
| ------------------------------------------------------------------ | --------------- | -------------------------------------------------------------------------------------------------------- |
| `type-system-extensions` — ∪/∩, polymorphism, kinding, worlds      | adopted-unbuilt | `spec/type-system.md` §§4-5, §9                                                                          |
| `sessions` — binary/shared/multiparty                              | adopted-unbuilt | `spec/type-system.md` §§6-8                                                                              |
| `modules` — 1ML-style                                              | adopted-unbuilt | `spec/modules.md`                                                                                        |
| `metaprogramming` — phases, hygienic macros, elaborator reflection | adopted-unbuilt | `spec/proposal-metaprogramming.md` (ADR-19/20)                                                           |
| `solver-interface` — pluggable domains, SMT refinements            | adopted-unbuilt | `spec/proposal-solver-interface.md` (ADR-16); dependency-rejection §superseded-in-direction by ADR-32/46 |

### 4.4 Usability surface (Wave 0/1)

| Area                                                          | Stance                                      | Authoritative source                                                                              | ADRs / notes                                                                     |
| ------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `attributes` — entity attributes                              | built (Wave-1 MVP, §14)                     | `spec/proposal-attributes.md`                                                                     | ADR-55                                                                           |
| `ffi` — extern blocks + interpreter FFI                       | built (Wave-1 MVP, §15)                     | `spec/proposal-ffi.md`                                                                            | ADR-58; pliron path awaits backend                                               |
| `data-patterns` — data decls + pattern matching (**surface**) | **partial** (grammar-only lane, §17)        | `spec/proposal-data-patterns.md` + `spec/proposal-data-patterns-addendum-vdc.md`                  | ADR-54/46/80. Distinct from the _core_ declared-data former (§4.1) which IS Live |
| `recursion-iteration` — fix, def rec, loops                   | design-pass                                 | `spec/proposal-recursion-iteration.md`                                                            | ADR-57                                                                           |
| `value-semantics` — MVP mutation/update stance                | **partial** (functional update landed, §13) | `spec/proposal-value-semantics-mvp.md`                                                            | ADR-53/25/29                                                                     |
| `shell-usage-surface` — REPL-first pillars                    | **partial**                                 | `spec/proposal-shell-usage-surface.md` — **no longer governing focus** (ADR-77 superseded ADR-35) | ADR-35/77; §5 nushell-gate retired                                               |

### 4.5 Sequent-machines program (ADR-65..69)

| Area                                                 | Stance                                                  | Authoritative source                                                                   | ADRs      |
| ---------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------- | --------- |
| `sequent-kernel` — polarized L command IL            | **partial** (L0 built, L1 partial)                      | `spec/proposal-sequent-kernel.md` + `-addendum-vdc.md`                                 | ADR-65    |
| `levitation` — datatypes as descriptions             | **partial** (stage 0 built; stage-1 core Live per §7.1) | `spec/proposal-levitation.md` + `-addendum-vdc.md`                                     | ADR-67/81 |
| `codata` — copatterns, corecursion                   | **partial** (MVP on desc side)                          | `spec/proposal-codata-corecursion.md` + `-addendum-vdc.md`                             | ADR-66    |
| `vdc-reflection` — CFVDC / FVDblTT internal language | adopted-unbuilt                                         | `spec/proposal-vdc-reflection.md` (shared theory; the four addenda carry local deltas) | ADR-68/69 |

### 4.6 Kernel program (ADR-77/78) — the governing focus

| Area                                         | Stance                                          | Authoritative source            | ADRs      |
| -------------------------------------------- | ----------------------------------------------- | ------------------------------- | --------- |
| `kernel-levels` — universe-level oracle      | built (slices 1-2)                              | `spec/kernel-boundary.md` §4/§8 | ADR-77/78 |
| `kernel-boundary` — minimal certified kernel | **partial** (design adopted; slices 1-2 landed) | `spec/kernel-boundary.md`       | ADR-77/78 |

### 4.7 Decided directions (ADR-46 capstone + companions) — all adopted-unbuilt

| Area                                                               | Authoritative source                                                                       | ADRs        |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ----------- |
| `prop-nominal-model` — wheeled polarity-sorted graded nominal PROP | `spec/proposal-term-face.md`; `VISION.md` §8                                               | ADR-41/46-A |
| `term-face` — Σ-zone multi-output terms + feedback wheel           | `spec/proposal-term-face.md`                                                               | ADR-49      |
| `polygraph-data` — laws as computed inspectable IR                 | `spec/proposal-term-face.md`; `core-ir-contract.md` §8                                     | ADR-32/46-B |
| `erasible-evidence` — phase-distinct evidence → non-dependent core | `spec/proposal-vdc-reflection.md`; `type-system.md` §12; `core-ir-contract.md` §8          | ADR-46-C    |
| `temporal-univalence` — staged certificate-carrying                | `spec/proposal-identity-univalence.md` + `spec/proposal-vdc-reflection.md`; `VISION.md` §8 | ADR-76      |

### 4.8 Toolchain facets (ADR-46 D + companions)

| Area                                                          | Stance                                                         | Authoritative source                                                                          | ADRs / notes                                             |
| ------------------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `inspection-protocol` — render bus / wire protocol            | built (Wave-1 MVP, §14; §14.7 diagnostics-transport amendment) | `spec/proposal-inspection-protocol.md`                                                        | ADR-59                                                   |
| **diagnostics architecture** (Report v3, `gandr-diagnostics`) | **Accepted synthesis, unbuilt — NO `status.yml` row** (→ §7.3) | `spec/proposal-diagnostics-architecture.md`                                                   | ADR-9/35/59; wyrd-fztr/atq4                              |
| `compilation-backend` — pliron IR + Cranelift                 | adopted-unbuilt                                                | `spec/proposal-compilation-backend.md`                                                        | ADR-46-D/51                                              |
| `packages` — package/build manager                            | adopted-unbuilt                                                | `spec/proposal-packages.md`                                                                   | ADR-46-D/56                                              |
| `pretty-printing` — layout VM + formatter + core printer      | adopted-unbuilt                                                | `spec/proposal-pretty-printing.md` (**three crates**: `gandr-doc`/`gandr-fmt`/`gandr-pretty`) | ADR-46-D; status.yml row names only `gandr-fmt` (→ §7.4) |
| `wasm` — playground-first emission                            | design-pass                                                    | `spec/proposal-wasm.md`                                                                       | ADR-61                                                   |
| `self-hosting` — gandr-in-gandr trajectory                    | design-pass                                                    | `spec/proposal-self-hosting.md` — reframed by ADR-77 (kernel = trust anchor)                  | ADR-60/77                                                |

### 4.9 Metatheory + corpus + naming

| Area                                                         | Stance                                                  | Authoritative source                                      | ADRs   |
| ------------------------------------------------------------ | ------------------------------------------------------- | --------------------------------------------------------- | ------ |
| `metatheory` — Agda relaunch (internal-univalence substrate) | design-pass (R0-R2 + R3 slice on unmerged branch)       | `spec/proposal-metatheory-relaunch.md`                    | ADR-30 |
| `corpus` — executable example corpus                         | built (82 cases: model 42, pathological 24, surface 16) | (ADR-52; no spec source — lives in `crates/gandr-corpus`) | ADR-52 |
| naming / dictionary                                          | (cross-cutting authority)                               | `dictionary.yml` + front-door `dictionary.md`             | ADR-30 |

---

## 5. The four `*-addendum-vdc.md` files (supersession status: ADDITIVE)

All four are single-purpose VDC (virtual double category) reflection-pass deltas, realizing **ADR-68/69** and _derived from_ `proposal-vdc-reflection.md` (`MANIFEST.yml` edges lines 282-305).
Each carries banner authored-date **2026-07-09** (committed 07-12/07-13 — a minor provenance offset, not a conflict).
**None supersedes its parent; each is additive to a _later_ stage and does not change today's build.**

| Addendum                                      | Parent                           | What it adds                                                                                                                                                                                                                                                                                                      | Touches built code? |
| --------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `proposal-sequent-kernel-addendum-vdc.md`     | `proposal-sequent-kernel.md`     | `Cell` gains `CellMeta` (variance/linearity + `invertible`); certificate identity = replay-equivalence; two composition ops (`compose_invertible` free, `compose_directed` acyclicity-gated). "Nothing here alters K1-K8 or the L0-L3 phase gates; every delta is additive to **L2**."                            | No (L2 is unbuilt)  |
| `proposal-levitation-addendum-vdc.md`         | `proposal-levitation.md`         | stage-0 `CellFace` gains `CellVarMeta` (constant `Producer` until consumer positions exist); dictionary-test (F0) as a second stage-0 consumer; typed cell face = protype of the reflected judgment layer; **the §6 stage-1 bill is explicitly UNCHANGED**, now double-purposed (unlocks the LLV Ł1-Ł4 fragment). | Stage-0 field only  |
| `proposal-data-patterns-addendum-vdc.md`      | `proposal-data-patterns.md`      | **"all recorded stances here are _confirmations with a sharper pedigree_, not changes; the MVP cut is untouched"** (`:6`): `rule` productions → loose-arrow generators; respect obligations exportable as certificates; without-K = the LLV J-restriction one level up.                                           | No                  |
| `proposal-codata-corecursion-addendum-vdc.md` | `proposal-codata-corecursion.md` | bisimulation engine = loose-arrow/`ProtypeIso` supplier; stream-fusion cells are the first `Mixed` variance (gated composition); the `−ᵒᵖ` duality caveat — the reflection face will **not** supply session duality before the LLV fragment.                                                                      | No                  |

`status.yml` correctly lists all four as `sources` on their respective areas (`sequent-kernel`, `levitation`, `codata`, `data-patterns`).
The `vdc-reflection` area (`status.yml`, stance adopted-unbuilt) records that the deltas are "partially reflected in gandr-desc cell/wellformedness structure, but the F5 architecture… is not built."

---

## 6. Supersessions, retirements, and reframings (consolidated)

Every one of these is handled _in-band_ by the corpus (a banner or a labeled row), so they are supersessions the docs already track, not silent conflicts.
Listed so the coordinator can see the moving edges at a glance.

1. **ADR-77 kernel-focus supersedes ADR-35 shell-focus (2026-07-12).** The governing focus is now the minimal certified kernel.
   Applied in `README.md:7`, `roadmap.md:55/104`, `proposal-shell-usage-surface.md:4`, `proposal-self-hosting.md:6`.
   The gate-script "bootstrap" milestone is **retired**; "bootstrapping" now names the kernel posture (`dictionary.md:51` collisions).
2. **PBG + melder push parser supersedes tree-sitter as the normative parser** (ADR-70/73/75).
   `proposal-surface-syntax.md:5` as-built amendment; tree-sitter kept as parity/reference; ADR-75 retires machine-side `ResolveOperatorSeq` for PBG forms.
   `proposal-surface-syntax.md:29` also supersedes the syntax heuristics in `packages/tree-sitter-gandr/AGENTS.md`.
3. **`proposal-parser-interaction-core.md` revises the W2-W4 execution arc of `proposal-graph-core.md` §8** (`:3`); the old `Mold { sort, tips }` identity is **retired** (`:24/:122`); the "decline to exploit incrementality" framing is retired (`:40/:66`). graph-core's G5 already retired the "fall back to resilient-RD" framing.
4. **ADR-46 Decision D three-component layout split supersedes the single `gandr-fmt` framing.** `proposal-pretty-printing.md:27` supersedes the Wadler default of `proposal-shell-usage-surface.md` §6 for this lane.
5. **Dependency-rejection in `proposal-solver-interface.md` is superseded in direction** by ADR-32/ADR-46-C (in-gandr erasible-evidence); the SMT-refinement carve-out "stands and composes" (`:114-115`).
6. **ADR-27 supersedes the pre-A1 core block** in `typing-machine.md:130`; since ADR-82 `KSplitBody` carries the precomputed split answer (`:142`).
7. **`roadmap.md` Appendix A (feature-complete ordering) is superseded as the *active plan*** by the A-track (`roadmap.md:11/327`) but **retained intact** as the reference ordering and the source of per-stage acceptance criteria.
   The active plan is `roadmap.md` §1 (A-track + value-model ladder + MVP shell track + ADR-46 decided-direction milestones + usability-MVP program).
8. **`data-patterns-addendum-vdc.md:32`** records a partial supersession of the "Levitation `Desc`-universe bead" by `proposal-levitation.md` V3/V4.
9. **ADR-80 resolves ADR-54's declared-data representation** (commit `f0613a2c`); `proposal-data-patterns.md` §3.4 is the surface home, `core-ir-contract.md` §0/§2 the frozen-core home.

---

## 7. Conflicts and staleness — explicit flags

Flagged rather than harmonized, per the task.
Ordered by impact.

### 7.1 `status.yml` core rows predate and omit ADR-80/81/82 (HIGH)

`status.yml` (committed 2026-07-13 23:05) is older than the ADR-80/81/82 landings (2026-07-14 03:37–11:14).
Concrete drift vs the §0 arbiter (which is verified current by BLAKE3):

* `core-cbpv` row (`status.yml:55-67`): `adrs: [1, 28, 31, 32, 38, 39]` and `as_built` "The full frozen-v0 fragment is Live (§0 rows 1-3)".
  But `core-ir-contract.md` §0 now lists **declared-data `D(ā)`/`Ctor`/`DataCase` (ADR-80)**, **`Type` universe + `Σ` pair (ADR-81)**, and the **ADR-82 split-motive** as Live rows (`core-ir-contract.md:49-50`, §2 lines 89-105).
  ADR-80/81/82 are absent from the row's `adrs`.
* `value-model` row (`:80-91`): `adrs: [38,39,40,42,45]`; `as_built` stops at "closed records… (§0 rows 3-5)".
  Declared-data and the levitation stage-1 formers are the §0 rows it does not mention.
* `levitation` row (`:338-349`): `as_built` "Stage 0 built… Stages 1+… are open."
  But ADR-81 landed the **stage-1 dependent core** (`Type` + `Σ` + `decode`) as Live in the frozen core (`core-ir-contract.md:50`, `dictionary.md:72`, `proposal-levitation.md` §6).
  The row is stale; the `decode` large-elimination lives in `gandr-desc` and the two frozen-core formers in `gandr-core`.
* `data-patterns` row (`:279-287`): "data declarations and pattern semantics are not built" — this is correct **for the surface** (grammar-only lane), but there is **no `status.yml` area** capturing the ADR-80 _core_ declared-data landing, so a reader scanning `status.yml` alone would conclude no data support exists, contradicting §0.

**Resolution rule:** `core-ir-contract.md` §0 wins (it is the named arbiter, and its BLAKE3 matches MANIFEST).
`status.yml` should gain/repair rows for the ADR-80/81/82 formers; MANIFEST re-hashed in the same change.
The corpus's own §0 doctrine ("Where this file and an arbiter disagree, the arbiter is correct and this file has drifted", `status.yml:16`) anticipates exactly this.

### 7.2 `proposal-parser-interaction-core.md` self-declares tracker/doc drift (MEDIUM)

`:304`: "Absorbing the dissertation late means some already-written bead texts carry superseded stances; until the §6.4 advisories are applied, the tracker and this proposal disagree in places."
The beads tracker is out of scope here, but the coordinator should treat parser-lane (`wyrd-4uhp` family) bead text as potentially stale against the proposal.
Also `:174` names the **in-code `Adaptation` records** (`gandr-grammar/src/surface/*.rs`) — not the proposal prose — as the authoritative registry for as-built syntax adaptations.

### 7.3 The diagnostics architecture has no `status.yml` registry presence (MEDIUM)

`proposal-diagnostics-architecture.md` is an _Accepted implementation synthesis_ (`:3`, wyrd-fztr 2026-07-14; reconciled to the atq4 corrective synthesis "pending its coordinator-assigned landing commit") introducing a new `gandr-diagnostics` crate and a **Report v3** schema — **none built yet**.
It is referenced by `README.md:51` and by `roadmap.md:90` (usability area 3, "Readable errors"), but it appears **nowhere in `status.yml`** — not as an area row and not in any `sources` list (verified by grep).
The `inspection-protocol` row covers the render bus and the §14.7 transport amendment, which is a _subset_.
So the implemented-vs-intended registry is silent on the whole diagnostics lane.
Coverage gap to close.

### 7.4 `pretty-printing` status row lags the three-crate split (LOW)

`status.yml:480-487` `as_built`: "gandr-fmt is named but unbuilt."
The authoritative `proposal-pretty-printing.md:3-5` (and ADR-46 Decision D) specifies **three** crates — `gandr-doc` (layout VM), `gandr-fmt` (CST formatter), `gandr-pretty` (core printer).
The row understates the scope; all three are unbuilt, so the _stance_ is right but the as-built text is stale.

### 7.5 `proposal-surface-syntax.md` body describes retired tree-sitter authority (LOW)

The banner amendment (`:5`) correctly declares the PBG parser normative and tree-sitter parity-only, but the **body** of the document is still written as a tree-sitter grammar design record (G3 gates framed as tree-sitter conflicts, `:514` still poses "tree-sitter-first acceptable?"
as an open question).
The banner resolves the authority; a reader who skips it could be misled.
Flagged as banner-vs-body tension, not a decision conflict.

### 7.6 `incremental-pipeline.md` title/intent vs as-built (LOW, self-flagged)

The document is titled/scoped for incremental parsing + typing, but `:3` records "As built 2026-07-11: there is **no incremental parsing**… reuse happens at the typing layer."
The `status.yml` `incremental-pipeline` row (partial) matches this.
Not a conflict — the doc flags its own gap — but the coordinator should not assume incremental _parsing_ exists.

### 7.7 VDC addenda authored-date vs commit-date (INFORMATIONAL)

All four addenda banners say "2026-07-09"; git commit author-dates are 2026-07-12/13.
This is content-authored vs repo-committed, not a conflict; noted for provenance only.

---

## 8. Implemented vs intended — the built surface at a glance

**Arbiters:** `core-ir-contract.md` §0 (core IR, verified current) and each spec's as-built banner; `status.yml` stances corrected per §7.1.

**Built and running today** (`README.md:64`, `core-ir-contract.md` §0, spec banners):

* CBPV checker + derived defunctionalized typing machine (property-tested against the recursive checker); trail-based worklist solver.
* A5.1 sequential CK evaluator (`gandr-core/src/eval.rs`) — the frozen driver and differential oracle; runs the pure spine + algebraic effects (`perform`/deep `handle`)
  + delimited control (`reset`/`shift`/`resume`).
    **A5.2 process-soup runtime is deferred.**
* Value-model rungs 1-4: string/int/numeric literals, covariant `List` (check-only literals + eliminator), closed records (width/depth `<:` + projection).
* Grades: sealed `0/1/ω` carrier with `fin/leq/plus/times` + `dup`/`drop`.
* Gradual layer: `Unknown` (subtyping reflexive-but-not-transitive once `Unknown` participates), holes.
* **Declared-data 1-cell MVP (ADR-80)**, **levitation stage-1 dependent core — `Type` universe + `Σ` pair + `decode` (ADR-81)**, **split-motive (ADR-82)** — Live per §0, _missing from `status.yml`_ (§7.1).
* Rung-1 identity: `Path`/`here`/`walk`, definitional walk-β, **no K, no η** (ADR-76/79), with a without-K negative witness in the test suite.
* PBG parser lane end-to-end: melder push machine (P1-P3), obligations/expected as first-class queries, merkle origin map; REPL (reedline), TUI, sans-io LSP — all tree-sitter-free.
* Sequent **L0** command face + static focusing (L1 partial: pure-spine checkpoint merged, zero-disagreement differential vs CK oracle).
* Levitation **stage 0** descriptions + codata MVP (on the description side).
* Kernel level oracle `gandr-kernel-levels` slices 1-2 (level algebra + Bezem-Coquand Horn-clause loop-checking).
* Wave-1 usability MVPs: attributes, FFI (interpreter path), functional record/list update, inspection-protocol wire crate.
* Executable corpus: 82 cases (model 42 / pathological 24 / surface 16).

**Adopted-unbuilt (decided design, no code):** type-system extensions (∪/∩, polymorphism, worlds), sessions, modules, metaprogramming, solver interface, `vdc-reflection` F5 layer, `temporal-univalence` (beyond U1), the entire ADR-46 fabric (`prop-nominal-model`, `term-face`, `polygraph-data`, `erasible-evidence`), compilation-backend, packages, pretty-printing, diagnostics Report v3.

**Design-pass (exploration on record, not fully decided):** `recursion-iteration`, `wasm`, `self-hosting`, `metatheory` relaunch.

**Partial (a slice runs, the rest open):** `identity` (U1), `operators`, `parser-core`, `incremental-pipeline`, `data-patterns` (surface, grammar-only), `value-semantics`, `shell-usage-surface`, `sequent-kernel` (L0/L1), `levitation`, `codata`, `kernel-boundary`.

The direction (`VISION.md` §8, `roadmap.md` §1.4): all features are facets of **one fabric** — a wheeled, polarity-sorted, graded nominal PROP — with polygraphical data and a temporal-univalent dependent future; ADR-46 makes these _decided directions_ with construction as the open research obligation.
`VISION.md:192` gives the honest register: shifts 4/5, the evidence layer, and univalent reflection "have essentially no runnable language code today."

---

## 9. Adjacent discoveries (flagged briefly, not chased)

* **The reference manual (`docs/manual/`) is present and populated** — Typst sources + rendered `main.pdf` + per-area `chapters/*.yml`+`*.typ`.
  Many specs declare their content "absorbed 2026-07-13" into a manual chapter while asserting the spec "remains the authoritative design record" (e.g. `core-ir-contract.md:10`, `type-system.md:7`, `effects-control-shell.md:7`, `proposal-data-patterns.md:9`).
  The manual **renders `status.yml` directly** (`status.yml:19-22`).
  This dual spec-of-record / manual-of- presentation structure, plus the **worktree name "failed-refactor"**, is worth the coordinator's attention — the manual migration is the most recent large refactor and may be the "failed" one; nothing in the surveyed scope proves it failed, but the two faces (spec `.md` and manual `.yml`) are a drift surface to watch.
  Out of survey scope (docs/gandr/spec + 5 root docs only).
* **`docs/adr/`** is the decision log (one file per ADR; `0001`…`0082`+), migrated 2026-07-12 from a monolithic `spec/ADR.md` (`README.md:31`).
  Every proposal's authority ultimately grounds in an ADR; MANIFEST edges encode `realizes`/`grounds` to `../adr/README.md#adr-N`.
  The ADRs themselves were not read (out of scope) but are the root of the `realizes`/`grounds` provenance for every area.
* **`MANIFEST.yml` provenance graph** is a ready-made dependency map (roles: index / vision / spec / proposal / contract / addendum / dictionary / status-data / plan) with `derives`/`grounds`/`realizes`/`reviews` edges and external refs (`external:internal-univalence`, `external:tylr`, `external:arxiv-2310.01530`, `external:miette-7.6`).
  Useful for the coordinator's plan synthesis; verified current.
* **`crates/gandr-corpus`** (ADR-52) is the executable ground truth the status registry points at; `examples/` was removed 2026-07-13 and its content moved to the manual's Examples chapter + the corpus crate (`README.md:56`).
* **Naming collisions** are centralized in `dictionary.md:44-52` / `dictionary.yml` `collisions:` — six glyphs/words (`Σ`, phase, `≤`, step/cost, `package`, bootstrap, corpus) each have an owning entry.
  Any plan touching these must respect the owner.
* **`type-system.md` and `effects-control-shell.md` carry no `Status:` staleness** — they are the stable rule-level homes; `effects-control-shell.md:3` explicitly relabels its old "Proposal" framing as an artifact of recovery order (ADR-29), now "Adopted — core design."

---

## 10. One-line pointer index (for fast lookup)

* Corpus authority statement → `README.md:9`
* Core implementation truth → `core-ir-contract.md` §0 (`:29-55`), per-shape table `:37-50`
* Status registry (consolidated view) → `status.yml` `areas:` (`:53`+), authority note `:8-17`
* Complete registry + provenance edges + BLAKE3 gate → `MANIFEST.yml`
* Naming authority + collisions → `dictionary.yml`; front-door `dictionary.md`
* Active build plan → `roadmap.md` §1 (`:13-107`); superseded reference ordering → Appendix A
* Where the design is going (one fabric) → `VISION.md` §8 (`:160-194`)
* Governing focus (kernel) → `README.md:7`; `kernel-boundary.md`; ADR-77
* VDC deltas → the four `spec/proposal-*-addendum-vdc.md`; shared theory `proposal-vdc-reflection.md`
