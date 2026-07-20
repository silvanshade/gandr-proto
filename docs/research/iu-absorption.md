# Absorbing internal-univalence (iu) as gandr's metatheory — research findings

Ticket: **gandr-fcw.4** (epic gandr-fcw, wayfinder map).
Research-only; no plan is committed here.
Every claim cites its primary source as `alias:relative/path` (anchors `§`/`Ln` where useful).
Source aliases: `iu` = the internal-univalence Agda library; `iu-notes` = its untracked notes; `wyrd@failed-refactor` = the wyrd worktree carrying `metatheory/`; `papers` = the research-paper corpus.

Snapshot state observed (for reproducibility / staleness judgement):

* `iu` canonical HEAD `022ec64` (2026-07-18); 67 `.agda` modules under `iu:src/Internal/` (the question's "66" undercounts by one — likely excluding `Internal.Everything` the gate root).
* `iu` beads: 40 issues read via `bd list --json` from the iu repo root (local Dolt clone; **not pulled** — see Hazard H8).
* `wyrd@failed-refactor:metatheory/upstream/internal-univalence` submodule pinned at IU `40b9352` (2026-07-12).

---

## Executive summary

1. **Namespace.** `Internal.*` is the sole module root and is declared **permanent** by the iu owner (`iu:docs/spec/ADR.md` ADR-4; ADR-12 D3, `iu-9r3`): it names the _method_ (internal to MLTT), is consumer-load-bearing (wyrd imports `Internal.Graph` etc. verbatim), and is explicitly out of scope for even the "Temporal Univalence" repo rename.
   "Absorb verbatim" therefore means **keep `Internal.*`**; re-rooting to `Gandr.*` is a mechanical-but-pervasive sweep (ADR-4 calls a pre-1.0 root rename "a mechanical sweep") that would fork from the wyrd consumption contract.
   Recommend keep.

2. **Build.** Agda **2.8.0** pinned (`iu:mise.toml`).
   Kernel OPTIONS: `--safe --without-K --hidden-argument-puns`, with `--guardedness` added per-module only where coinduction is used.
   One **strict gate root** `Internal.Everything` (imports every hole-free module); one **holey leaf** `Internal.Polygraph.Coherence` gated separately with tolerated `UnsolvedInteractionMetas`; a separate **meta** root `Meta.Everything`.
   Gate = `mise run agda:check` via `aifix`.
   **Two library lines** (ADR-9): kernel `internal-univalence.agda-lib` is dependency-free; `internal-univalence-meta.agda-lib` (reflection tactics) depends on kernel + stdlib.

3. **Coherator — present home.** It now lives in **two** parallel realizations, mirroring the groupoid/doctrinal split.
   **Groupoidal (original, landed):** the `coh` cell + `Coh`/`CohCell`/`coh-total` in `iu:src/Internal/Polygraph/Complex.agda` (the `𝔉` free complex), its `⟦coh⟧` interpretation in the deliberately-holey crux leaf `iu:src/Internal/Polygraph/Coherence.agda`, over the Squier engine in `iu:src/Internal/Rewriting.agda`(+`Confluence`).
   **VDC-shifted (newer, statement grade):** `iu:src/Internal/Doctrine/Complex.agda` (`𝔇ω`) with `VCoh`/`coh`/ **`VSquier`**, where Squier completion is **named per level as an engine dependency, not interpreted inline**; certification tower in `iu:src/Internal/Doctrine/Certified.agda` (`∞VDC`, `UnitLayer`).
   In _both_, the coherator is the live open frontier, not a finished artifact.

4. **wyrd seam.** `wyrd@failed-refactor:metatheory/` is a `--safe --without-K` **signature tree** (`Gandr.*`, zero postulates, uninhabited proof-target records) consuming IU as a **pinned submodule** — only three engine modules (`Internal.Graph`, `Internal.Rewriting`, `Internal.Rewriting.Confluence`) + `Internal.Prelude`.
   A **port-drift gate** (`iu:check` → `scripts/check-iu-pin.nu`) fails if the submodule is dirty or off-pin.
   The pin **lags badly** (predates all Doctrine/UaBase/temporal work).

5. **stdlib direction change.** Current iu keeps the **kernel stdlib-free** (ADR-2, ADR-9); stdlib is admitted _only_ for the meta/reflection layer.
   The owner's new direction for gandr — **generally adopt stdlib, renamed locally to house-style as already done for Agda builtins** — **contradicts** the current iu ADRs and dissolves the two-library-lines split.
   It needs a gandr ADR.
   The pattern already exists (iu's `Internal.Prelude` re-exports builtins under house names; wyrd's `Gandr.Prelude` re-exports stdlib).

6. **Rust tooling + manual.** `iu:crates/agda-html-tokens` (Rust, `html5ever`) extracts source-offset token metadata from `agda --html` output to drive the **Typst** manual (`iu:docs/manual/`, 8 chapters, the "authority for meaning").
   Both are absorbable; the owner already wants the doc-gen toolkit _split from iu content_ (`iu-e4c`) so it can graduate to shared tooling.

7. **Two specced-but-unwritten seam docs.** `INTERFACE-wyrd.md` (`iu-b89`) and `CONSUMING.md` (`iu-070`) are **missing on disk** — they exist only as bead specifications.
   A coordinator should treat them as design inputs to author, not sources to copy.

---

## 1. Module namespace fate — keep `Internal.*`

* **The rule.** All library modules live under the single root `Internal.*`, with `Internal.Everything` the strict gate root (`iu:docs/spec/ADR.md` ADR-4, "Module namespace `Internal.*`").
  Rationale: Agda has no package namespacing, so one distinctive root is the only clash-avoidance mechanism; `Internal` "names the thesis — the mathematics is internal to MLTT".
  ADR-4 records that a pre-1.0 root rename would be "a mechanical sweep" and that the `.agda-lib` name `internal-univalence` (not the module root) disambiguates the library.
* **Permanence is decided.** ADR-12 D3 (`iu:docs/spec/ADR.md` Ln 429) and bead `iu-9r3` both state: `Internal.*` is **permanent** — "it names the method (internal to MLTT), which stays true under the new name, and it is consumer-load-bearing."
  Even the repo/library rename to "Temporal Univalence" (ADR-12 D3, `iu-9r3`) explicitly leaves `Internal.*` untouched: `iu-9r3` — "Internal.* namespace is PERMANENT — never in scope."
* **Consumer coupling.** wyrd imports the engine as `Internal.Graph` / `Internal.Rewriting` / `Internal.Rewriting.Confluence` / `Internal.Prelude` verbatim (`wyrd@failed-refactor:metatheory/README.md` "Upstream integration").
* **Implication for gandr.** "Absorb verbatim" ⇒ keep `Internal.*`.
  A `Gandr.Metatheory.Internal.*` or `Gandr.*` re-root is mechanically possible but touches every module header/import in 67 modules **and** breaks the wyrd contract and the owner's permanence decision.
  If gandr wants a single house root, the low-cost option is to leave `Internal.*` as-is and let `Gandr.*` (the wyrd signature tree, §4) sit _above_ it — exactly today's wyrd layering.
  Recommend: **keep `Internal.*`; do not re-root.**
  (Owner authority: this is an iu-owner decision already; a gandr re-root would need explicit new authority.)

## 2. Build configuration

### 2.1 Toolchain + flags

* **Agda 2.8.0**, pinned `"github:agda/agda" = "2.8.0"` (`iu:mise.toml` Ln 286).
* **Kernel OPTIONS policy:** `--safe --without-K --hidden-argument-puns`, with `--guardedness` added **per-module only where needed** (`iu:README.md` "Checking"; verified across all 67 headers — e.g. `Internal.Prelude`, `Internal.Doctrine`, `Internal.Step`, `Internal.UaBase.Code` omit `--guardedness`; coinductive modules include it).
  Every module carries its own file-level pragma (ADR-2, "no library-wide flags; per-file pragmas keep deviations visible and greppable").
  + **Doc drift (minor, H1):** ADR-2 "Flags and gate" (`iu:docs/spec/ADR.md` Ln 53) states the blanket pragma as `--safe --guardedness --without-K` and **omits `--hidden-argument-puns`**, and states `--guardedness` as universal.
    The README and the actual source are the accurate statement (per-module guardedness; puns present).
    ADR-2's two historical "exemption classes" (with-K confluence isolate; a `TERMINATING` fold) are also **both retired** now (`iu:README.md` "Checking": "no exemption class remains"; the holey leaf's `TERMINATING` fold is gone).
    Record, do not harmonize.
* `Set₀`/`lzero` pin on the code layers (HANDOFF §8; visible in `𝔉`/`𝔇ω` module params).

### 2.2 The gate (`iu:mise.toml` `[tasks."agda:check"]`, Ln 132-137)

Three invocations:

1. `aifix batch agda --fail-on-diagnostics -- -v0 -i src src/Internal/Everything.agda` — the **kernel strict root** (stdlib-free, `-i src` only).
2. `aifix batch agda --fail-on-diagnostics -- -v0 -i src -i meta -i vendor/agda-stdlib/src --warning=noUnsupportedIndexedMatch meta/Meta/Everything.agda` — the **meta strict root** (admits vendored stdlib).
3. `aifix batch agda --fail-on-diagnostics --expected-code UnsolvedInteractionMetas -- -v0 -i src src/Internal/Polygraph/Coherence.agda` — the **holey leaf**, tolerating deliberate interaction metas (the coherator crux, §3).

`Internal.Everything` (`iu:src/Internal/Everything.agda`) imports every hole-free module — a stray meta anywhere fails the gate; declared holey **leaves** are gated separately (ADR-2 "Strict-root/ holey-leaf rule").
A bare toolchain-only check is `agda --no-libraries -i src src/Internal/Everything.agda` (`iu:README.md`).

### 2.3 Two library lines (ADR-9)

* `iu:internal-univalence.agda-lib` — `name: internal-univalence`, `include: src`, **no `depend:`** (dependency-free kernel).
* `iu:meta/internal-univalence-meta.agda-lib` — `name: internal-univalence-meta`, `include: .`, `depend: internal-univalence standard-library-2.4`.
* ADR-9 "Two library lines" (`iu:docs/spec/ADR.md` Ln 304-314): kernel stays stdlib-free; the meta layer may import `Reflection.AST.*`/`Reflection.TCM.*`/meta-level `Data.*` **at elaboration time only** — emitted proof terms reference iu's own combinators exclusively.
  Meta root is `Meta.*` (a top-level `Tactic.*` is unavailable — stdlib owns it).
  The meta tree is 6 modules (`iu:meta/Meta/`: `Wiring`, `Tactic/Cong`(+`Test`), `Tactic/Paste`(+`Test`)).

## 3. The coherator's present home (the headline question)

**Owner's framing:** the coherator was the `coh` cell in the IR complex + the `[[coh]]` interpretation (with Squier completion), originally in the groupoidal Complex; the organization has since shifted for the VDC.
**Finding: it now exists in two parallel homes**, and in both it is the project's live open frontier rather than a closed construction.

### 3.1 Groupoidal realization (original; landed with a located hole)

* **The `coh` cell** — `iu:src/Internal/Polygraph/Complex.agda`, the `𝔉` free cellular-extension former (fraktur layer letter, ADR-7 addendum 2026-07-05).
  Ln 86-89: `coh : ∀ {Φ}{x y} {f g : Cell Φ x y} → (ϕ : Coh f)(ψ : Coh g) → Cell (Φ ▸ˢ x ⇴ y) f g` — the Squier-completion filler adjoining, one dimension up, the cell between two parallel **pure** coherences (comment Ln 45-46).
* **`Coh`** (Ln 96-121) — the pure-coherence predicate: `c` built only from structural/law constructors, **never** a generator (`atom` excluded, Ln 95).
  It is an inductive family so a coherence derivation is a first-class recursable object (the validator's carrier).
* **`CohCell`/`cohCell`** (Ln 134-138) — a cell bundled with its `Coh` derivation: the letter alphabet the Coherence leaf's word route presents; load-bearing at the base sphere.
* **`coh-total`** (Ln 148-161) — conservativity of the `Coh` restriction above the base sphere.
* **The `[[coh]]` interpretation** — `iu:src/Internal/Polygraph/Coherence.agda`, the **declared holey leaf** (ADR-2 strict-root/holey-leaf rule; **nothing imports it**; gated separately with `UnsolvedInteractionMetas`, §2.2).
  The fold's `coh` clause (header Ln 34): `⟦ coh ϕ ψ ⟧ ≔ cohInterp Φ' a' (acyclicSphere Φ' a') f g ϕ ψ`, and `cohInterp` (Ln 211, 334-338) routes through `R.coherence-from-acyclicity` — **Squier's theorem internalized** (Squier Prop 3.2.4 / Guiraud; manual §5).
  The whole leaf is one forward-declared SCC; the fold is mutual with loop-triviality.
  **Located open obligation (4)** (header Ln 63-68): the closing recursion — "the frontier — do not fill; the point is that it is open and exactly located" (marked HS-15; `iu:docs/spec/ROADMAP.md` "The mathematical frontier").
* **The Squier engine** — `iu:src/Internal/Rewriting.agda` (+ `Rewriting/Confluence.agda`): presentation, critical branchings, Squier completion, contraction, acyclicity, `coherence-from-acyclicity` (Ln 454-496).
  This is the layer **wyrd already consumes** (§4).

### 3.2 VDC-shifted realization (the reorganization; statement grade)

`iu:src/Internal/Doctrine/Complex.agda` — `𝔇ω`, the ω-dimensional **free doctrine complex**, the doctrinal analogue of `𝔉` (module header Ln 3-9).
Landed at **statement grade** (bead `iu-cnb`, "tranche d"): codes validated K-free, proofs only where refl-grade.
Its multi-globular telescope `VSphere` bottoms out at a VDC cell boundary and iterates only the **tight** direction (the settled VDC lesson; header Ln 18-27).

How the coherator moves (the **law-set audit** table, header Ln 47-84):

| `𝔉` class                                     | lands in `𝔇ω` as                                                                                                                                                                                                 |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `coh` (Squier filler over `Coh`-marked pairs) | `coh` over **`VCoh`**-marked pairs — but `VCoh` **includes `rule`** where `Coh` **excludes `atom`** (the marking is "in the image of the former-rule sublanguage"; completion runs one dimension up, over rules) |
| `atom` (dim-1 generators)                     | one dimension up: `rule` (dim-3 former-rule generators on `Paste`-trees)                                                                                                                                         |
| `mon-λ/ρ/α`                                   | **not generators** — lemmas about `graft`; **zero mon-class generators at any level**                                                                                                                            |
| `gpd-κ/ι`, `inv↔`                             | overlay-confined (`gpd-κ↑`/`gpd-ι↑`/`back↑`), premised on the invertibility threshold `p ≤ Vdim Φ`                                                                                                               |

* **`VCoh`** (Ln 225-242): `here↑`/`then↑`/`rule`/`coh`.
  The decisive difference from `Coh` is the `rule` clause (Ln 234-239).
* **The Squier entry point is now _named_, not interpreted inline** (header Ln 86-94; code Ln 272-312).
  `VCoherenceAt`/`VAcyclicAt` are the coherence-supply / loop-triviality statements; **coherence-supply holds by construction** — `freeCoherence = coherenceAt coh` (Ln 306-307), i.e. the `coh` constructor _is_ the filler; **acyclicity (`VAcyclicAt`) is the genuine crux**, supplied by the **tree-ARS kernel** (accessibility / confluence / Newman over `Paste`-trees), the design's **T5 engine dependency** — "named here, never built." `VSquier Φ` (Ln 309-312) is the spec register bundling `acyclicSphereᵛ` (the crux) and `coherence-from-acyclicity`.
* **Check (c)** (Ln 249-254): `VPos (⋆ f g s t) ≡ Paste f g s t` by `≡.idn` — the depth-1 fragment of `𝔇ω` projects **definitionally** to `Internal.Doctrine`.
  `D1Exemplar` (Ln 330+) is the one fully-worked rule (`walk(here; θ) ⇒ θ`).

The **contrast** is the answer to "how did the organization shift": on the groupoid side the coherator's _interpretation_ (`⟦coh⟧`) is computed inside a holey crux leaf against the concrete Squier engine; on the VDC side the coherator's _completion_ is **factored into a per-level spec register (`VSquier`) supplied by the engine**, coherence-supply is free (the `coh` constructor), and the whole complex is statement-grade codes rather than a computing fold.

### 3.3 Certification tower + design of record

* `iu:src/Internal/Doctrine/Certified.agda` — `∞VDC`, `descendᵛ`/`strᵛ`, and `UnitLayer` (`unit`/`reflᵛ`/`⟦Jᵛ⟧`), the per-level path-protype datum that is ua-base's certification home (WYRD-INTAKE W6; `iu-notes:WYRD-INTAKE.md` §W6).
* Design of record: `iu:docs/spec/DESIGN-doctrine-polygraph.md`, `iu:docs/spec/DESIGN-doctrinal-carrier.md` (the two-sorted `∞VDGraph` carrier; the HANDOFF `iu-notes:HANDOFF-doctrinal-carrier.md` is its brief).
* Doctrine tower module set: `iu:src/Internal/Doctrine.agda` (T0 core, `Paste`/`Forest₁`), `Doctrine/Carrier.agda`, `Doctrine/Stratum.agda`, `Doctrine/Certified.agda`, `Doctrine/Complex.agda`, `Doctrine/Instances.agda`.

**Caveat (H2):** the coherator is _not_ a finished component in either home.
Groupoid side: located open hole (4) in `Coherence.agda`.
Doctrine side: `VSquier` is named-not-built and depends on an unbuilt tree-ARS kernel (T5) and the unbuilt n-ary `graft` (the T0 residual; `Doctrine.Complex` Ln 314-328 OBLIGATIONS).
Absorbing "the coherator" absorbs a **live research frontier**, not a closed proof.

## 4. Relationship to `wyrd@failed-refactor:metatheory/`

`wyrd@failed-refactor:metatheory/` is gandr's Agda metatheory face today (per ADR-30, Agda is the sole proof vehicle; `wyrd@failed-refactor:metatheory/README.md`).
Structure:

* **`Gandr.*` signature tree** (`.../src/Gandr/`, 16 modules).
  Every declaration is a **field of a record signature** or an **uninhabited proof-target record**; the R1 triage (`wyrd-7jxp`) replaced every `postulate` with a signature field, so the tree is `--safe --without-K` with **zero postulates** while asserting nothing (README §Status).
  Layout: `Gandr.Core.CBPV`/`Phase`, `Gandr.Algebra.Cost`, `Gandr.Effect.Algebraic`/`Step`/`Parallel`, `Gandr.Control.Stacks`, `Gandr.Metatheory` (the `wyrd-c0cp` proof targets: subject reduction, progress, confluence, canonicity), `Gandr.Prop.*` (free-PROP R3 layer), `Gandr.Everything` (gate entry point).
  Terminology follows the **calf/decalf** line (README module table).
* **IU consumed as a pinned git submodule** at `metatheory/upstream/internal-univalence`, built once upstream, **not re-ported** (README "Upstream integration").
  `Gandr.Everything` imports exactly three upstream modules — `Internal.Graph`, `Internal.Rewriting`, `Internal.Rewriting.Confluence` — which transitively type-check the consumed kernel; `Gandr.Prop.*` additionally imports `Internal.Prelude`.
  **IU's holey `Internal.Polygraph.Coherence` is deliberately NOT imported** and stays off wyrd's gate.
  So wyrd consumes only the **Squier/rewriting engine**, not the coherator interpretation or the Doctrine/UaBase towers.
* **Two preludes coexist:** `Gandr.Prelude` re-exports the **vendored agda-stdlib** for the signature tree; upstream `Internal.Prelude` is the **stdlib-free** base of the engine and of `Gandr.Prop.*`. (This is directly relevant to the stdlib-direction question, §5 — gandr already runs a stdlib-backed house prelude alongside the stdlib-free kernel.)
* **The pin.** `.agda-lib`: `depend: standard-library-2.4`.
  Submodule URL `https://github.com/silvanshade/internal-univalence`, gitlink recorded at IU **`40b9352`** (README Ln 52; verified submodule HEAD = `40b9352b1d2fcc5a7ffdd1badcd14f97c730bece`, 2026-07-12).
* **The port-drift gate.** `iu:check` (`nu scripts/check-iu-pin.nu`, wired into `prek.toml`) **fails the build if the submodule is uninitialized, dirty, or off its recorded pin** (README Ln 67).
  Pin-bump is deliberate (mirrors the `.agents/core` submodule / `mise run core:update`): check out the intended IU commit, `git add` the mount to move the gitlink, commit the new pin in its own step.
* **The pin is intentionally-and-badly stale (H3).** README Ln 54: "This pin was bumped ahead of the wyrd↔IU re-synchronization: `agda:check` is **not expected to pass** against it until that re-sync lands (the CI `agda-check` job is disabled, and no active gate runs the metatheory)."
  The pin `40b9352` (2026-07-12) **predates the entire body of recent iu work**: the Doctrine/VDC complex, the doctrinal carrier, all of `Internal.UaBase.*`, the temporal-univalence program, and ~40 commits up to canonical HEAD `022ec64` (2026-07-18).
  `WYRD-INTAKE.md` confirms: "wyrd's vendored pin … may LAG the referenced state — the push freeze is still on" (`iu-notes:WYRD-INTAKE.md` Protocol Ln 9).
* **Pin-reference discrepancy (record, do not harmonize):** `iu-ij6` (note dated 2026-07-15) says "their pin **de03731** predates Internal.Profunctor entirely," while the wyrd README + live submodule say **`40b9352`**.
  README states `40b9352` was a forward bump from an earlier pin; the `iu-ij6` note is likely itself stale relative to that bump.
  The coordinator should reconcile from the live gitlink (`40b9352`), not the bead text.
* **Provenance the coordinator should know:** R2 (wyrd commit `62adeda`) first landed the IU engine as a **translate-and-fix port** into `Gandr.Ambient.*`/`Gandr.Rewriting*` — **zero fixes needed**, every ported body type-checked unchanged under the shared Agda 2.8.0 toolchain — then **superseded it the same day** with the submodule (owner decision `wyrd-s3tl`: integrate upstream rather than carry a copy) (README Ln 68-69).
  This is direct evidence that verbatim absorption of the engine layer is low-risk.

**Absorption implication.** gandr already vendors IU.
"Absorbing iu verbatim as gandr's metatheory" is a **decision to invert the vendoring** — move IU's source _into_ gandr rather than pin it — which retires the submodule + port-drift gate and folds the `Internal.*` tree beside `Gandr.*`.
The wyrd metatheory tree shows the target shape: `Gandr.*` signatures over an `Internal.*` engine, two preludes.

## 5. agda-stdlib implications

* **Current iu posture (kernel = zero stdlib).** `iu:internal-univalence.agda-lib` has no `depend:`.
  The kernel imports **only** `Agda.Builtin.*` and `Agda.Primitive` (verified: the only non-`Internal.*` imports across `iu:src/Internal/` are `Agda.Builtin.{Equality,Nat,String,Unit,List,Bool,Reflection}` and `Agda.Primitive`).
  ADR-2 "Dependencies": "Self-contained: no agda-stdlib … a stdlib dependency would be the library's first external trust and compatibility surface, for no gain."
  Stdlib is admitted **only** for the meta layer (ADR-9 "Two library lines"; `iu:meta/…` imports `Reflection.AST.*`, `Reflection.TCM.*`, `Data.{Bool,List,Maybe,Nat,Unit,Product}.Base`, `Function.Base`, `Relation.Nullary.Decidable.Core`).
  This matches the owner's "originally reflection-tactics-only."
* **The house-builtin-renaming pattern (the owner's analogy).** `iu:src/Internal/Prelude.agda` wraps Agda builtins under house names: `Agda.Builtin.Equality` `renaming (refl to idn)` (Ln 40-41); `Agda.Builtin.Nat` `renaming (Nat to ℕ; suc to succ)` (Ln 366-369; note **`succ` not `suc`**); `Agda.Builtin.Sigma` `renaming (Σ to ∐)` (Ln 192).
  This is exactly the "rename locally to house-style as done for builtins" the owner references for stdlib.
* **Owner's NEW direction for gandr (contradicts current iu ADRs — Hazard H4).** Per this task: "now generally adopt [agda-stdlib] but rename locally to house-style as done for builtins."
  This **reverses ADR-2's "no stdlib" and dissolves ADR-9's two-library-lines quarantine**.
  It also weakens ADR-11's "zero transitive trust surface" property that made the kernel submodule-consumable — though since gandr is _absorbing_ rather than _being consumed_, that property matters less.
  The mechanism already exists on the wyrd side: `Gandr.Prelude` **re-exports the vendored agda-stdlib** for the signature tree (`wyrd@failed-refactor:metatheory/README.md` "Two preludes coexist").
  The natural gandr realization is a **house prelude that re-exports stdlib under house-style names** (the builtin-rename pattern generalized), then a sweep of kernel modules from `Agda.Builtin.*` to the house prelude.
  **This is a design decision requiring a gandr ADR that explicitly supersedes iu ADR-2/ADR-9;** record it as owner direction, not as current state.
* **stdlib version + provisioning.** agda-stdlib **v2.4**, verified under Agda 2.8.0 (`wyrd@failed-refactor:metatheory/README.md` "Build"; `iu:meta/internal-univalence-meta.agda-lib` `depend: … standard-library-2.4`).
  **Provisioning divergence (H5):** iu still vendors stdlib as a **git submodule** (`iu:.gitmodules` `[submodule "vendor/agda-stdlib"]`), whereas wyrd provisions it by **gitignored local clone** via `agda:deps`.
  Bead `iu-49f` (open) is the plan to move iu to the wyrd model, and its rationale is a direct absorption hazard: "a vendored-submodule stdlib is a recursive-checkout hazard for every consumer of IU. wyrd hit it 2026-07-13 — its worktree bootstrap full-cloned agda-stdlib history (~600 MB) through the nested submodule until it bumped its IU pin … and stopped materializing IU's nested checkouts entirely." **gandr must provision stdlib locally, not via a nested submodule**, and if it generally adopts stdlib (H4) this becomes load-bearing rather than meta-only.

## 6. IU's Rust tooling and Typst manual

* **`iu:crates/agda-html-tokens`** — a Rust workspace crate (edition 2024, rust ≥1.96; `iu:Cargo.toml`).
  Streaming extractor over `agda --html --html-highlight=code` output using the Servo `html5ever` tokenizer; emits **deterministic per-module JSON** of anchor tokens + source-offset metadata (`iu:crates/agda-html-tokens/src/lib.rs` header; `#![forbid(unsafe_code)]`; deps `html5ever`, `serde`, `serde_json`, `thiserror`).
  Driven by `mise` tasks `agda:html` → `agda:tokens` (`iu:mise.toml` Ln 146-178): output `gen/agda-html/tokens/**/*.json`.
  The crate carries an extremely strict clippy/rustc lint wall (`iu:Cargo.toml` `[workspace.lints]`).
* **Typst manual** — `iu:docs/manual/` (8 chapters `01-charter` … `08-landscape`; `main.typ`, `refs.yml`, `STYLE.md`; reusable `lib/{template,agda-html,rules,diagrams}.typ`; `main.pdf` checked in).
  The token JSON feeds `lib/agda-html.typ` so the manual can splice highlighted, source-anchored Agda.
  The manual is declared **"the authority for meaning"** (`iu:README.md` Layout; ADR-1).
* **Owner direction already on file:** `iu-e4c` (wyrd-fb-3, open) — **split the reusable doc-gen toolkit (`lib/*.typ` + the extractor pipeline) from iu manual content** (`chapters/` + `STYLE.md`), so a consumer adopts the pipeline without importing iu's charter/chapters; this split is the prerequisite to graduating the toolchain upstream (to agentic-dev core / a shared fragment, consumed back by pin).
  ADR-11 already classifies "the `agda --html` token → Typst manual toolchain" as consumable via the reference-project mechanism.
  **Absorption implication:** the toolkit (`crate` + `lib/*.typ` + mise tasks) is cleanly liftable; the _chapter content_ is iu-specific meaning that gandr would re-frame under the temporal program (see the manual-overhaul beads, §7).

## 7. Knowledge extraction from the 40 iu beads

Read via `bd list/show --json` from the iu repo root.
Status counts: 3 in_progress (`iu-c2h.1`, `iu-48k`, `iu-up7`), the rest open; priorities P1–P4.
Full title index available on request; the load-bearing set for absorption:

### 7.1 The named tickets

* **`iu-c2h` (EPIC, P1) — temporal realignment.** Plan of record `iu:docs/spec/PLAN-temporal-realignment.md`; **RATIFIED 2026-07-12** (ADR-12).
  Realigns iu as "the metatheory backend of wyrd's identity/univalence program": seam artifacts first (consumer contract, correspondence/demand tables, base-stratum ua note), manual overhaul second, results re-steer third.
  Children: `iu-b89 iu-1i2 iu-at1 iu-9r3 iu-fbc iu-0by iu-ywa iu-26l iu-48k iu-gcq iu-ij6`.
  Decision face ADR-12.
  **This epic IS the "gandr is the consumer" story** — its whole frame is wyrd/gandr as first downstream.
* **`iu-9r3` (P1) — rename execution: "Temporal Univalence".** ADR-12 D3, one coordinated change: GitHub repo rename, manual title/subtitle, README, **`.agda-lib` name**, MANIFEST paths, notes sweep; **coordinate wyrd's `.gitmodules` URL + library-name bump in the same window** (their `iu:check` pin gate). "`Internal.*` namespace is PERMANENT — never in scope."
  Blocked on the push-freeze lift (`iu-wy1`).
  **Absorption note:** if gandr absorbs the source, the _library/package_ name is in play but the _module namespace_ is not; coordinate with wyrd's pin gate.
* **`iu-b89` (P1) — INTERFACE-wyrd correspondence + demand tables.** ADR-12 D1.
  Two tables destined for `iu:docs/spec/INTERFACE-wyrd.md` (**currently MISSING on disk — H6**): (i) gandr-vdc constructor ↔ iu module ↔ Nasu anchor, incl. the split-by-representation vs split-by-construction row; (ii) wyrd rung (U0–U4) ↔ iu artifact ↔ state, using Track B's three-gate split.
  Notes carry a wyrd **F0 verdict (2026-07-12): GO** — laws 1–4 PASS, law 5 (units) PARTIAL; split-by-representation confirmed empirically; located F2 production gaps (H1 no FreeTerm substitution/matching, H2 single-signature wf only, H3 no production SigMorphism).
  **This bead is the constructor dictionary a coordinator needs — but must be authored, not copied.**
* **`iu-070` (P1) — consumer import contract.** ADR-12 D1, ratified ASAP.
  Deliverable `iu:docs/CONSUMING.md` (**currently MISSING on disk — H6**) stating: (a) stable public surface = `internal-univalence.agda-lib` (kernel, dependency-free, uniformly `--safe --without-K`); the meta `.agda-lib` is consume-at-own-risk; (b) `Internal.*` is API + pre-1.0 no-stability caveat; (c) pin/version story (submodule, gitlink pin, deliberate bumps); (d) the **aube** consumption path — clarify whether aube is repo-tooling only and submodule is THE dep mechanism.
  Notes: ".agda-lib NAME changes with the rename — coordinate."
* **`iu-ij6` (P2) — feedback dispatch to wyrd.** Flags: their ADR-46 Decision E describes iu with "sized types" (**excluded under `--safe`**, ADR-2) and the old "(n, r)" dial letter (**renamed to `p`** at bootstrap — the invertibility threshold; ADR-2 Vocabulary "the (n, p) dial"); pin-advance readiness once the freeze lifts. 2026-07-15 additions: the doctrine fragment gives gandr's `cells_equal` a cheaper mode (decidable normal-form identity on convergent fragments); their η discipline should be checked against the doctrine ladder's η orientations.
  **Absorption note:** carries corrections gandr's own ADR-46 will need.

### 7.2 Other absorption-relevant beads

* **`iu-49f` (P2)** — replace the vendor/agda-stdlib **submodule** with gitignored local provisioning (see §5, H5; the recursive-checkout hazard).
* **`iu-e4c` (P2)** — split the doc-gen toolkit from manual content (see §6).
* **`iu-070`/`iu-b89`** seam docs missing (H6).
* **`iu-7bt` (EPIC, P1)** — presentation-side reshape (UIP honesty, Π/ν split, named-atom solver, syntactic cell complex): the epic that produced the current `𝔉`/`Coherence` shape (§3.1).
* **`iu-48k` (in_progress, P2)** — doctrine fragment: free-CFVDC pasting normal forms ahead of M-crux; the T0 core behind `Internal.Doctrine` (`Paste`/`Forest₁`, n-ary `graft` residual — the coherator's doctrine-side dependency, §3.2).
* **`iu-cnb`** (referenced by the HANDOFF, not in the 40-list snapshot as a distinct open row — likely closed/renamed; **H7**) — the doctrinal-carrier investigation that produced `Internal.Doctrine.Complex` (`𝔇ω`) and `Doctrine/Certified.agda`.
* **`iu-c2h.1` (in_progress, P1)** — ua-base η/faithfulness: the edit-calculus rule layer (R-comm/R-coxeter/R-coh + convergence); the staged η half of `Internal.UaBase.*` (WYRD-INTAKE W4/W14).
* **`iu-law` (EPIC, P1)**, `iu-p0t`, `iu-akb`, `iu-gta`, `iu-4k6` — the notation/reflection/solver line (the Squier/polygraph consumer solvers; ADR-9 "the engine is the kernel").
* **`iu-wy1` (P1, bug)** — CI cannot clone the private agentic-dev submodule; push-freeze context.
* **`iu-jwd` (P4)** — the atom frontier: complex generators beyond dimension ≤1 (the coherator's declared higher-dimension extension).
* Manual-overhaul chapter beads: `iu-at1` (ch.6 environments/CwF), `iu-fbc` (ch.7 program/predictions), `iu-5ps` (ch.1 Brouwerian lineage + ch.8), `iu-0by` (appendix A implementer's discipline), `iu-jje`, `iu-e4c`.
  These define how the manual is being re-framed under the temporal program (§6).

## 8. iu-notes intake ledgers

### 8.1 `iu-notes:WYRD-INTAKE.md` — the outbound feedback ledger (iu → wyrd/gandr)

The single durable home for iu→wyrd design feedback; iu-side channel bead `iu-ij6`. 18 **PENDING** items (W1–W18), one **Absorbed** (Track B revisions).
The ledger is a coordinator asset — it is the pre-digested set of "things wyrd/gandr should absorb."
Highlights bearing on the metatheory absorption:

* **W2/W3/W4/W14** — `Internal.UaBase.*` (nine modules) proves ua-base O1+O2 end-to-end on a toy leaf-free universe; the η half is staged (`iu-c2h.1`).
  The saturation invariant is _the store being a profunctor_ (`actˡ`/`actʳ` + laws in `iu:src/Internal/Profunctor.agda`).
  NFC faithfulness reduces to one typed lemma.
* **W6** — `UnitLayer` (`iu:src/Internal/Doctrine/Certified.agda`) is ua-base's certification home (§3.3).
* **W9/W10/W12/W13** — F2 cell-store implications: no mon-class law cell ever needs storing (graft-lemmas); `cells_equal` gains a decidable normal-form fast path on convergent fragments; ADR-69 composition modes get their metatheory face (`seq-dinat` in `Profunctor/Dinatural.agda`); the DΣ bridge mirrors `arity.rs`/ADR-49.
* **W17/W18 (ACTIONABLE)** — the produoidal-normal-form sweep for wyrd's two-mode certificate composition; primary source is Román's _Monoidal Context Theory_ thesis (`papers`/theses).
  Design-level, for wyrd's certificate machinery — **not** metatheory-absorption per se, but flagged ACTIONABLE.
* Anchors of the form `docs/spec/…` are **iu-repo paths**, and the ledger warns the vendored pin may LAG the anchor — treat the maintainer's shuttled copies as authoritative when the pin predates an anchor (`iu-notes:WYRD-INTAKE.md` Protocol Ln 9; corroborates H3).

### 8.2 `iu-notes:HANDOFF-doctrinal-carrier.md` — the doctrinal carrier determination

Prepared 2026-07-15 (tracking `iu-cnb`).
The mission: determine the ω-carrier for the doctrinal side (the two-sorted `∞VDGraph`) and assemble "the analogous tower" to the groupoid side.
**This is the document that produced the VDC-shifted coherator organization of §3.2.** Its table (§1) is the exact groupoid↔doctrine correspondence:

| groupoid side (landed)                                                                                       | doctrinal side                                                             |
| ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| `𝔉`: `Sphere`/`Pos`/`Cell`/`Coh` + `coh` contraction — `Internal.Polygraph.Complex`                          | `𝔇ω` (`Internal.Doctrine.Complex`)                                         |
| Squier: word route, acyclicity=coherence-supply fullness, `coherence-from-acyclicity` — `Internal.Rewriting` | **per-level rule layers on trees; completion named per level** (`VSquier`) |
| `∞Groupoid` + `descend` + `str` — `Internal.Groupoid.Univalent`, `Internal.Polygraph`                        | `∞VDC` + `descendᵛ` + `strᵛ`                                               |

Success bar (owner's words, HANDOFF §1): "we don't need full proofs of everything, but enough to show that there is some sort of analogous tower" — data structures validated K-free; tower statements typecheck; proofs only where cheap.
**This is why the VDC coherator is statement-grade** (H2).
The HANDOFF also records the OPTIONS/gate discipline (§8): `--safe --without-K --hidden-argument-puns`, `Set₀` pin, new modules leaf-only and added to `Internal.Everything` when hole-free — the absorption's module-addition contract.

---

## 9. Hazards, surprises, and contradictions for the coordinator

* **H1 (doc drift, minor).** `iu:docs/spec/ADR.md` ADR-2 states the kernel pragma as `--safe --guardedness --without-K` (universal guardedness, no `--hidden-argument-puns`); the README and the actual 67 module headers are the accurate statement (per-module guardedness; puns present; both historical exemption classes retired).
  Trust source + README over ADR-2's flag sentence.
* **H2 (the coherator is a live frontier, not a finished proof).** Groupoid side: located open hole (4) in `Internal.Polygraph.Coherence` (the crux leaf, deliberately not imported).
  Doctrine side: `VSquier` is named-not-built and depends on an unbuilt tree-ARS kernel (T5) and the unbuilt n-ary `graft` (T0 residual).
  "Absorb the coherator verbatim" absorbs open obligations.
  This is by design (ADR-1 posture: "construction + precisely-located open problem").
* **H3 (the wyrd pin is deliberately, badly stale).** `40b9352` (2026-07-12) predates ALL of Doctrine/VDC, UaBase, and the temporal program; `agda:check` is "not expected to pass" against it and the CI job is disabled.
  Absorbing the _canonical_ iu (`022ec64`) is a large jump past the last verified-consumed state (the engine-only `Graph`/`Rewriting`/`Confluence` at `40b9352`).
  Re-verification of the fuller tree under the gandr toolchain has not been done.
* **H4 (owner's stdlib direction contradicts current iu ADRs).** "Generally adopt stdlib, house-renamed" reverses ADR-2 "no stdlib" and ADR-9's two-library-lines quarantine.
  Needs a new gandr ADR that explicitly supersedes them.
  The wyrd `Gandr.Prelude` (stdlib re-export) is the existing mechanism.
* **H5 (nested-submodule stdlib is a checkout hazard).** iu still vendors stdlib as a git submodule (`iu:.gitmodules`); `iu-49f` documents wyrd hitting a ~600 MB recursive-clone blowup. gandr must provision stdlib by gitignored local clone (the wyrd `agda:deps` model), especially if it adopts stdlib generally (H4).
* **H6 (two key seam docs do not exist yet).** `iu:docs/spec/INTERFACE-wyrd.md` (`iu-b89`, the constructor dictionary) and `iu:docs/CONSUMING.md` (`iu-070`, the import contract) are **specced in beads but absent on disk**.
  A coordinator expecting to read the correspondence tables will not find them — they are design inputs to write, not sources to cite.
* **H7 (`iu-cnb` not in the open-bead snapshot).** The HANDOFF and `Doctrine.Complex` cite `iu-cnb` as the live doctrinal-carrier investigation, but it does not appear as a distinct row in the 40-bead list (likely closed or subsumed).
  Reconcile against the live tracker before relying on its status.
* **H8 (bead reads are from an un-pulled local Dolt clone).** Per user global policy, trusting `bd show` requires `bd dolt pull` first; this research treated all source trees (incl.
  `.beads`) as strictly read-only and did **not** pull.
  Bead statuses/notes may be locally stale relative to DoltHub.
  The pin-reference discrepancy (`de03731` in `iu-ij6` vs live `40b9352`, §4) is one visible symptom.
* **Surprise (positive).** The R2 translate-and-fix port of the IU engine into wyrd needed **zero fixes** — every ported body type-checked unchanged under the shared Agda 2.8.0 toolchain (`wyrd@failed-refactor:metatheory/README.md` Ln 68).
  Strong evidence the engine layer absorbs cleanly; the risk concentrates in the _newer, statement-grade_ Doctrine/UaBase towers, not the engine.
* **Adjacent discovery (flag only).** The manual re-framing under the "Temporal Univalence" program (`iu:docs/spec/DESIGN-temporal-univalence.md`; the Brouwerian reading) is a substantial narrative/naming layer riding on top of the code.
  It is success-conditional and manual-gated (the slogan is _banned from the manual_ until a prediction lands), so it should not drive metatheory-absorption structure — but a gandr that absorbs the manual inherits this program's honesty gates.
