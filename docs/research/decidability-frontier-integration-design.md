# Integration design — the decidability-frontier / canonicity-staging program (J-42 R1–R4)

> **Status: DESIGN.** Companion to [decidability-frontier-and-canonicity-staging.md](decidability-frontier-and-canonicity-staging.md) (the study).
> The owner ratified _pursuit_ of the study's program (2026-07-20, after independent prior investigations); this document fixes _how_ each recommendation lands against the machinery that actually exists.
> Grounded in a four-surface integration analysis (2026-07-20) over the live reboot tree, the iu library, and the register; each section names the artifacts it was read from.
> Calls the owner should confirm are marked **RATIFY**; everything else is executable as written.
> **Superseded machinery (2026-07-31).** The machine-validated component corpus, its block schema, and its derived citation register were removed with the grammatical-framework pipeline: §1.1's schema reading and §6's re-derivation mechanics are historical.
> The landing homes are now the Markdown corpus under `docs/gandr/spec/` and its bibliography `docs/gandr/spec/bibliography.yml`; a citation-bearing registration is a `bibliography-v2.md` row plus a matching bibliography entry, with no generator step.
> Every other correction and recommendation here is unaffected.

## 1. Corrections to the study, against the live machinery

The study's recommendations survive; four elements of its _picture of the machinery_ do not, and builders following the study verbatim would mis-locate work.
These corrections are load-bearing for §§2–5.

1. **There is no `rationale` element.** The fcw.8 block taxonomy (`crates/workflow-docs/src/model.rs`, the normative schema: `Section, Prose, Judgements, Grammar, Rule, Definition, Diagram, Code, Example, References`) defines no `Rationale` block, and `validate.rs` has no reservation mechanism (terms are define-on-first-use).
   "Rationale element" is the project's phrase for the _ex-ADR prose home_ inside a component (per the gandr-hvw W12/W17/W18 pinning), not a schema affordance.
   R1 is realized as an ordinary `<section title="Rationale">` of `<prose>`/`<definition>` blocks in the future certificate component.
2. **`cells_equal` is gandr-side Rust, and its fast path does not exist yet.** The study locates the decidable normal-form fast path iu-side and "recorded as available"; in fact `cells_equal` lives at `crates/theory-virtual-doctrines/src/vdc.rs` (trait method + `CellStoreVdc` impl, backed by `elaborations_replay_equivalent`), both bodies do replay-equivalence only, and the fast path is the **pending** intake item W10 — a planned change, not an artifact.
   Consequence: R2's consumer is a _cross-repo_ dependency (iu statement → gandr code), and the fast path is **TCB-adjacent** (it is the equality the VDC law tests decide) — see §3.
3. **The enumerator largely exists; the gap is alphabet-locality.** `crates/theory-computads` already ships the J-42-shaped capability, gate-tested: `overlap::enumerate_overlaps` (critical pairs, confluence + composition kinds), `completion::complete` (budgeted Knuth–Bendix/Squier with decline-with-report), `rewrite::normalize`, tracelet replay, and the `fused ≡ two-step` property differential — all **monomorphic over the sequent-kernel `CmdPat` alphabet**.
   The study's "the merged project has no counterpart" is wrong at the mechanism level; what is missing is the tool _for the iu alphabets_, and the binding constraint is how an alphabet gets into the tool (§4).
4. **The directed Ł-ladder is live reboot code, not pre-reboot source.** `crates/theory-virtual-doctrines/src/directed/{context,hom,coend,boundary}.rs` carry Ł1–Ł4 (with the `CutOutcome { Coherent, Directed, Declined }` two-mode routing running as code).
   The study cites only the `wyrd@failed-refactor` paths.
   Its architectural conclusions hold verbatim against the live crate — but future citations should use the live paths. (Those modules also cite pre-reboot doc anchors, e.g. `proposal-vdc-reflection.md`, that do not exist in the reboot tree — recorded for the docs-lane absorption program.)

## 2. R1 — tractability classification in the B9 certificate component

**Landing.** Nothing is authored now: the certificate component (`docs/spec/<certificates>.xml`) does not exist until B9 mints it, and per §1.1 nothing can or need be reserved.
At B9, R1 is a rationale `<section>` containing two `<term-def>`s — `convergent-fragment` ("a canonicity theorem covers the boundary; decision by normal form") and `certificate-carried` ("general band; per-instance witness") — plus prose stating each mode's _reason for availability_.
Well under an hour of authoring inside the component B9 writes anyway.

**The axis-separation rule (the one substantive design constraint).** R1's tractability axis is **not** the invertible/directed composition-mode axis of `directed-univalence-design.md` §8.1 item 2 (`CutOutcome::Coherent` vs `Directed`).
They coincide in today's two-band design; the study's own R4 anticipates a _convergent directed fragment_ (a focusing-staged `⇒²⇝`), at which point convergent ≠ invertible.
The B9 author must classify by **tractability reason**, keeping the classification separable from the mode tag, or R4 later has to unpick a baked-in accident.
This rule is recorded on gandr-wvd.9 so it survives until B9.

**Term-ownership note.** `convergent-fragment` / `certificate-carried` are corpus-wide define-once terms (`validate.rs`); the B9 component owns their `<term-def>`s.

## 3. R2 — `VTractableAt` in the T5/`VSquier` register, and the `cells_equal` attachment

**Register discipline (read from `iu:src/Internal/Doctrine/Complex.agda` 240–328).** Parameterized statement-grade `Set₀` records under `--safe --without-K`, zero postulates, each predicate either supplied-by-construction (`VCoherenceAt` via `freeCoherence`) or named-as-obligation (`VAcyclicAt`, the T5 crux), bundled by `VSquier`.
`Certified.agda`'s `UnitLayer` confirms this is house style.

**Landing sketch** (statement-grade, no proof obligation, stays `--safe`):

```agda
record VTractableAt (Φ : VSphere) : Set₀ where
  constructor tractableAt
  field
    conf : ∀ {x y : VPos Φ} {f g : VCell Φ x y}
         → VCoh f → VCoh g → VCell (Φ ▸ᵛ x ⇴ y) f g
    term : ∀ {x y : VPos Φ} (f : VCell Φ x y) → VCoh f → VAcc Φ f
```

with an optional `tractable : VTractableAt Φ` premise field in `VSquier`.
**RATIFY (modeling):** the sketch states tractability as convergence (termination + confluence, matching the realization layer's `Convergent`); the purest J-42 reading instead states FP as ≤ 1 diagonal filler (a uniqueness shape).
These are different predicates; the owner picks the fence's shape before the iu track lands it.

**Q3 is answered by the register's own style: the general parameterized record.** The realization layer already generalizes convergence as one relation-agnostic record (`Rewriting.agda` `module Presentation`, `record Convergent` = `Terminating` + `LocallyConfluent`, "so every instance reuses them") instantiated per fragment.
Per-fragment ad-hoc records would drift from the discipline and re-derive the fence at each fragment.
**RATIFY** as the Q3 call.

**The `cells_equal` attachment is three-part, not documentation.** The W10 fast path changes what is computed, and it is TCB-adjacent, so: (a) a guard in `cells_equal` that takes the fast normal-form branch only where a convergent-fragment witness applies (rule-word comparison per W9/W10), falling through to replay-equivalence otherwise; (b) the iu `VTractableAt` inhabitant as the _soundness certificate_ for the guard's fragment — a code-only guard without the register witness is unsound; (c) optionally a mode-bearing widening of `VdcCellEquality` (which mode answered: fast-NF vs replay) — minimal now is guard + internal mode enum.
The study's "off-TCB" framing applies to R3's enumerator only; conflating it with the W10 fast path is the one dangerous mistake this program could make.

**Ownership.** R2 is iu-track work (the statement) consumed by gandr (the guard).
The durable cross-link is prose in both ledgers — gandr bead cites `iu-ij6`/W10 by name; W10 gains a back-reference — because the two trackers sync out-of-band and cannot depend on each other mechanically.

## 4. R3 — the enumerator: `workflow-search` v1

**Q2 is entangled with the first-user choice**, and the study's "natural first user" (the interchange-independence bounded search) conflates two different groupoid alphabets: the trivial `FreeGroupoid` (one cancellation rule) and the real one — `iu:UaBase/Rules.agda`'s `_⇒²_`, ~28 dependently-typed constructor families, churning (an erratum cell was ratified 2026-07-17, three days before the study).
The independence question lives over `_⇒²_`.
A Rust tool pays an alphabet-transcription tax against a moving target; an Agda-side search tracks it for free but is not the reusable substrate.

**The call — RATIFY: path (C), Rust with a frozen minimal fixture.**

- Crate: `crates/workflow-search` (the `workflow-` tooling charter fits the study's own "semantics-free, no-TCB, bug-finder" framing; a deliberate commitlint scope row when the crate lands, `tooling` scope for interim commits).
- v1 scope: **one user** — the interchange-independence instance.
  The iu track pins the exact target cell (candidate: the double-distribution interchange `R-coh`) and the minimal `_⇒²_` sub-fragment it touches, then freezes that sub-alphabet as a checked-in fixture with a provenance pin to `UaBase/Rules.agda@<commit>` — the core-sequent pre-lowered-fixtures discipline, reader included.
  This bounds transcription to a page and makes staleness explicit: an "underivable within N" answer is version-scoped to the pinned alphabet.
- Interface: `bounded_search(alphabet, target_cell, bound) → DerivedBy(path) | NotFoundWithin(bound)`, a standalone bounded BFS with decline-with-report (mirroring `CompletionBudget`) — **not** a generalization of theory-computads over an alphabet trait (a real cross-module refactor; deferred until ≥ 2 Rust-native alphabets justify it).
- Epistemic status: `NotFoundWithin` is _evidence_, never a theorem — same posture as J-42's own uncertified tool.
- Costs to budget: the workspace lint wall (`indexing_slicing`, `arithmetic_side_effects`, `panic`/`unwrap` denies) makes a search tool meaningfully more expensive than a throwaway; and alphabet churn requires a re-sync discipline on the fixture.
- Recorded alternative: if the owner prioritizes the reusable substrate over the iu answer, v1 becomes path (A) — bounded-enumeration/evidence entry points on the existing `CmdPat` engine, iu alphabets deferred.
  Do not attempt both in one v1.

## 5. R4 — the directed band: confirmed pure re-entry work

Verified against the live crate: the attachment surface does not exist (`hom.rs` is refl-generated — no one-way `ℰ⇝` classes; `directed.rs` declares directed univalence out of scope pending the B10 universe object), and the only "cheaper-if-now" lever — a variance/polarity-marked certificate plane — is _already_ reserved via the B2 variance-slot ask (directed-univalence-design §6.3/Q1, cross-filed on gandr-wvd.2).
R4 adds nothing to buy now.
Two pointers recorded for the re-entry evaluation: (i) the focusing pass should inherit the polarized certificates from that B2 slot; (ii) `theory-virtual-doctrines/src/query.rs` (`normalize` → `RewritePath`, `reduces_to`) is the undirected normal-form precedent whose shape the directed `⇒²⇝` pass would parallel.

## 6. The register package

| edit                             | status                      | mechanics                                                                                                                                                                                                                                                           |
| -------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| (a) J-42 Feature-cell correction | **landed with this design** | single cell in `bibliography-v2.md`; the Feature column does not flow into `refs.yml` (verified against `scripts/refs-yml`), so no re-derivation; row stays `+hold` — descriptive correction, not a citation-bearing upgrade                                        |
| (c) named-gaps bullet            | **landed with this design** | one Appendix-3 hydration-debt-style bullet naming the Johnstone FP source, the Dawson–Paré–Pronk free-adjoint/Π₂ line, and the transformation-monoid/finite-set presentations row (already demanded by directed §10 Q8); no locators invented, no `refs.yml` impact |
| (b) Došen–Petrić row             | **owner-gated**             | a new citation-bearing registration (Q4 discipline); mechanically a two-file atomic edit — register row **plus** a matching `scripts/refs-yml/data.nu` curated entry (the generator hard-errors otherwise) — then re-derive `refs.yml`                              |

**Q4 note.** No item above requires the J-42 anchor-verification pass; what triggers it is citation-bearing _use_ — promoting Thm 3.23/3.27 into the B9 rationale or the T5 register.
An adversarial three-surface anchor-verification run was prepared this session and stood down in favor of this integration spike; it remains the ready-made discharge path for Q4 when citation-bearing use approaches (first plausible trigger: B9 authoring).

## 7. Ownership and sequencing

| item                             | executes           | when                                                        | blocked by                 |
| -------------------------------- | ------------------ | ----------------------------------------------------------- | -------------------------- |
| R1 rationale prose               | gandr B9 worker    | B9 mints the certificate component                          | wvd.9 (itself after wvd.8) |
| R2 `VTractableAt` statement      | iu track           | any time; before W10's fast path lands                      | owner modeling RATIFY (§3) |
| W10 fast-path guard              | gandr (B10-era)    | after R2's witness exists                                   | R2                         |
| R3 `workflow-search` v1          | gandr tooling lane | after owner RATIFY of path (C) and the iu-track fixture pin | Q2 ratification            |
| R4 focusing-staged NF evaluation | fcw.12 re-entry    | at Temporal-Univalence re-entry                             | fcw.12                     |
| register (b)                     | gandr docs lane    | on owner call                                               | Q4-adjacent owner decision |

Nothing blocks the backbone; the study's §5 claim survives the spike intact.

## 8. Owner ratification queue

1. **Q2 / R3 path**: (C) Rust + frozen minimal `_⇒²_` fixture, first user = interchange-independence (recommended) — vs (A) `CmdPat` dogfood substrate.
2. **Q3 / R2 shape**: general parameterized `VTractableAt Φ` (recommended, house-style evidence) — vs per-fragment records.
3. **R2 modeling**: convergence-shaped fence (termination + confluence, as sketched) vs FP-literal (≤ 1 filler).
4. **Register (b)**: mint the Došen–Petrić v5-only row now or at first iu-side use.
5. **Q4 timing**: run the prepared anchor-verification pass at B9-approach (recommended) or sooner.
