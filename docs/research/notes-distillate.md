# Notes distillate for the gandr reboot (gandr-fcw.5)

> Research deliverable for the gandr wayfinder epic (gandr-fcw). This is a self-contained
> synthesis of the durable knowledge held **only** in ephemeral session notes across three
> repos, distilled for a coordinator who will build a project plan. Every claim cites its
> primary source in `alias:relative/path` form. No machine-local paths appear here.
>
> Alias key used in citations:
> `wyrd-notes:` = the wyrd contributor-notes repo · `iu-notes:` = the internal-univalence
> contributor-notes repo · `iu:notes/` = the (gitignored) in-repo working-notes directory of
> the internal-univalence repo · `wyrd@failed-refactor:` = the canonical wyrd source tree
> (specs/ADRs/crates) · `iu:` = the internal-univalence source tree (Agda + `docs/spec`).

---

## 1. Executive summary

1. **The gandr repo this branch lives in is a *fresh reboot*: it has no `docs/`, no ADRs, no
   specs — only tooling config, `AGENTS.md`, `CLAUDE.md`.** So "durable knowledge no spec/ADR
   records" is, for gandr, effectively *all* substantive design content in these notes. The
   prior incarnation, **wyrd** (`wyrd@failed-refactor`), carries the full record: 85 ADRs, a
   `docs/gandr/spec/` corpus, and built crates (`gandr-core`, `gandr-sequent`, `gandr-desc`,
   `gandr-grammar`/`gandr-parser` planned). The reboot's job is to re-lay that foundation; this
   distillate is the map of what must be re-decided, re-recorded, or carried forward.

2. **The single biggest carry-into-PLAN artifact is the polarized System-L kernel synthesis**
   (`wyrd-notes:sequent-machines-plan.md` + `wyrd-notes:digest/sequent-machines-dossier.md`):
   nine decisions (D1–D9) that unify the machine IL, fusion-by-completion, codata, sessions,
   levitation, closures, and implicits under one symmetry (data/codata = producer/consumer =
   μ/μ̃ = cut/communication). Its thesis — "gandr's CEK machine already *is* a sequent machine
   in a costume; `meet` is the cut, `Cont::Bind` is `μ̃`" — is backed by a file:line as-built
   dossier and is the highest-leverage design in the corpus.

3. **The universe-stratification decision (adopted as wyrd ADR-78) is settled and carry-worthy**
   (`wyrd-notes:universe-stratification-synthesis-2026-07-13.md`): a judgmental level kernel
   `{0,+1,max}` with a Bezem–Coquand polytime oracle, elaborator-side complete inference, prenex
   interfaces, displacement UX, declared landmark posets; Prop rejected impredicatively, SProp
   bracketed; the unsoundness triad (Type:Type / positivity / non-termination) each has a plan.

4. **The parser-replacement analysis is a complete, decision-gated engineering plan**
   (`wyrd-notes:parser-replacement-analysis-2026-07-09.md`): replace tree-sitter with a Rust
   port of tylr's `meldr` operator-precedence calculus over a merkle-hashed flat-arena CST.
   Three measured findings invert the original framing: gandr does no incremental parsing today,
   it would buy nothing at gandr's scale, and the mixfix restriction is a deliberate invariant,
   not a tree-sitter artifact. Totality is a *theorem*, not a recovery discipline.

5. **The owner's headline pattern — engineering feeding back into mathematics — is real,
   recurring, and now has a durable home:** `iu-notes:WYRD-INTAKE.md`, an explicit outbound
   feedback ledger (W1–W18) from the internal-univalence (Agda) side to wyrd/gandr. The cleanest
   instance is *verified against source*: gandr's `wyrd@failed-refactor:crates/gandr-desc/src/arity.rs`
   (multi-out = the bridge arity `A ←s— J —π→ I —t→ B = Σ_t∘Π_π∘Δ_s`) *determined the shape of
   the mathematics*: the iu DΣ / Σ-former mirror is **additive, not a refactor**, because the
   engineering artifact was already single-out-factored. §3 catalogs five such instances.

6. **A concrete hazard: the in-repo `iu:notes/` directory is real, is gitignored (so not
   committed), and duplicates `iu-notes:secured/iu-c2h1/` byte-for-byte** for all shared files,
   plus holds 6 unique session-close files. It is contributor-concern working material for the
   iu η-coherence proof campaign (bead `iu-c2h.1`) — adjacent to gandr, valuable only via
   WYRD-INTAKE. Full inventory + classification in §11.

7. **A source contradiction to record:** `iu-notes:secured/iu-c2h1/RESUME.md` cites
   `wyrd docs/gandr/spec/proposal-ua-base.md §3.3` as naming the U3 gate, but **no
   `proposal-ua-base.md` exists in `wyrd@failed-refactor`** — the ua-base/U3 material lives in
   `wyrd@failed-refactor:docs/gandr/spec/proposal-vdc-reflection.md` and
   `proposal-identity-univalence.md`. Likely a push-freeze / vendored-pin lag (see §3.7). Flagged, not harmonized.

8. **Recurring process learnings worth institutionalizing** (not currently in any ADR):
   the house **differential-testing discipline** (`L ≡ run`, `checker ≡ machine`, `eval ≡ run`,
   fused ≡ two-step, `jit ≡ eval`) is the load-bearing quality mechanism; **worktree isolation +
   persist-failing-artifacts**; **features must land with corpus examples**; **frozen-core §0
   changes compete for one narrow post-arena window**; the **K-free witness discipline** on the
   iu side. §10 collects these.

---

## 2. Corpus orientation — what these repos are and how they relate

Three coupled projects, discovered from the notes (primary sources cited per claim):

| Project | What it is | Tracker prefix | Source of record |
| --- | --- | --- | --- |
| **wyrd / gandr** | A dependently-typed language + proof system in Rust; "gandr" is the built language, "wyrd" the enclosing project/vision. This repo is its **reboot**. | `wyrd-*` | `wyrd@failed-refactor` (85 ADRs, `docs/gandr/spec/`, crates) |
| **internal-univalence (iu)** | A companion **Agda formalization** — the "mathematics" side: CFVDC/doctrinal-carrier metatheory, univalence, coherence proofs. | `iu-*` | `iu:` (Agda `src/`, `docs/spec/DESIGN-*`) |
| **feedback channel** | The deliberate iu→wyrd conduit. iu-side intake bead `iu-ij6`; ledger `iu-notes:WYRD-INTAKE.md`. | — | `iu-notes:WYRD-INTAKE.md` |

Key structural facts (each a carry-into-notes reference point):

- **gandr = the reboot; wyrd = the prior full build.** This gandr worktree has no `docs/`.
  wyrd's `docs/adr/` holds 0001..0081+ (85 files verified in `wyrd@failed-refactor:docs/adr/`),
  and `gandr-sequent`/`gandr-desc` crates are built. The reboot must reconstitute the ADR/spec
  layer — treat wyrd's as the salvage corpus, not as authority the reboot already owns.
- **"temporal univalence" is the committed name** for gandr's univalence direction (owner steer,
  `wyrd-notes:identity-reflection-arc-handoff-2026-07-12.md`; iu ADR-12 D3; deliberately avoids
  the nLab "internal univalence" collision; the "temporal, not spatial" slogan is *banned in the
  manual* but "under reconsideration" for provocative use — raise at next doc touch, don't assume).
- **Bootstrapping reframe** (`wyrd-notes:triage-plan-2026-07-12.md` §"Owner steers", bead
  `wyrd-tpgh`): the objective is *architecting for a bootstrapping future* — a minimal certified
  kernel (Lean-4-shaped: elaborator over a small trusted kernel, **not** Idris) plausibly able to
  bootstrap gandr-in-gandr — **not** a shell/gate-porting exercise. Kernel-focus is a design
  invariant; crates keep being built as usability scaffolding.

---

## 3. The engineering ↔ mathematics feedback pattern (owner's headline interest)

The owner's stated example — "gandr's VDC reflection and the tracelet implementation yielding new
insights" — is one facet of a **bidirectional** loop that these notes make explicit. The durable,
not-yet-ADR'd content is the loop itself and its concrete instances. **All of this is
carry-into-PLAN + carry-into-notes** (it is both a design driver and a provenance record).

**The loop has a name and a home.** `iu-notes:WYRD-INTAKE.md` is "the single durable home for
observations, corrections, and design feedback that internal-univalence sessions produce *for*
wyrd/gandr" (iu-notes:WYRD-INTAKE.md §Purpose). Protocol: iu sessions append dated/anchored items;
a wyrd agent works the `PENDING` set and reports back absorbed/declined. **Standing goal (owner,
`iu:notes/2026-07-17-session-close-funext-push.md`): every iu session that produces
wyrd/gandr-relevant observations APPENDS there — never scatter into session notes.** This ledger
should be adopted as durable coordination infrastructure in the reboot.

### 3.1 Instance A (verified against source) — `arity.rs` shaped the iu Σ-former

The strongest and most concrete case, and it runs *engineering → mathematics*:

- gandr's `wyrd@failed-refactor:crates/gandr-desc/src/arity.rs` implements multi-out arities as
  the **bridge diagram** `A ←s— J —π→ I —t→ B` (`BridgeArity`, `single_output`, destination
  aggregation requiring a commutative monoid). *Verified directly*: the module doc opens
  "Multi-out **arities** — the bridge diagram `A ←s— J —π→ I —t→ B`".
- When the iu maintainer asked whether to generalize the mathematics on the multi-out and
  directed axes *now*, the coordinator grounded the answer **in the Rust source**: because
  gandr's multi-out is *already* `Σ_t∘Π_π∘Δ_s` and single-out-factored, "the iu mirror is
  ADDITIVE (a DΣ local-coproducts former + ADR-49-style Σ-zone aggregation discipline), NOT a
  cell-boundary refactor" (`iu:notes/2026-07-16-session-close-s2-spike-landed.md`
  §"Multi-out / directed-LLV question"). The iu DΣ design was then "verified at source (arity.rs,
  ADR-49 direct reads)" (`iu:notes/2026-07-17-session-close-overnight-eta.md`, iu-4pq).
- Recorded outbound as **WYRD-INTAKE W13** (bridge-factorization metatheory face): the Σ-zone
  commutative-monoid discipline "appears as the SAME gate on both sides of the seam — `arity.rs`
  gates the combine at a port; the iu statement carries the destination monoid as a premise-form
  alphabet item at ∼-grade, so no modulo-AC rewriting enters the polygraph on the metatheory side
  either" (iu-notes:WYRD-INTAKE.md W13).

This is the pattern in its purest form: **an implemented Rust data structure fixed a design choice
in the Agda metatheory**, saving a refactor and confirming the "wheel firewall" (fan-in needs a
commutative monoid) from the engineering side.

### 3.2 Instance B — VDC reflection: the concrete carrier drives the math, the math certifies the engineering

- iu undertook the doctrinal-carrier determination (`∞VDGraph`/`∞VDC`/`𝔇ω`) **specifically so the
  wyrd maintainer could revisit gandr's F2 design against a concrete carrier**: "resolving §10's
  deferred question NOW, before F2, so the maintainer can revisit F2 and improve the overall
  design against a concrete carrier" (`iu-notes:HANDOFF-doctrinal-carrier.md` §1). The math is
  scouted to feed an engineering redesign.
- The reverse: iu's `UnitLayer` (`unit`/`reflᵛ`/`⟦Jᵛ⟧`) became "ua-base's certification home" —
  gandr's `ua-base : CodeIso ≅ (x ⤳ y)` reads as "the D1 fold supplies a `UnitLayer` whose `⟦Jᵛ⟧`
  is discharged by path normal forms" (WYRD-INTAKE W6; `iu:notes/2026-07-15-next-session-and-gandr-feedback.md` §2.1).
- The **funext/first-univalence-layer thread was reprioritized to LEAD** with the explicit
  rationale: "unlocking wyrd/gandr's concrete funext/ua demonstrations is the highest-value
  near-term outcome; **the engineering artifact feeds back**" (`iu:notes/2026-07-16-session-close-s2-spike-landed.md`
  §"QUEUE REORDERED"). The mathematics agenda is steered by what the engineering needs to
  demonstrate.

### 3.3 Instance C — tracelet / certificate identity: the math hands the engineering its exact obligation

- The sequent plan's fusion engine emits "**tracelet-shaped 3-cell certificates**" (Behr
  compositional-rewriting shape), differential-tested against the two-step composite
  (`wyrd-notes:sequent-machines-plan.md` D3, §3.2). "Tracelets as certificate shape. Implement
  none of the fibrational apparatus" (ibid §1 source map).
- iu then supplied the *identity* semantics gandr's tracelet/cell store needs: "rule-composites
  are the completion's alphabet, so decidable normal-form identity on convergent fragments
  compares rule-words, not raw derivations" (WYRD-INTAKE W9).
- And the **faithfulness obligation gandr inherits is now a crisp iu lemma**: `NFC : ⟦u⟧ ≐ σ →
  u ≈ᶜ perm-het(σ)` — "the crisp statement of 'Coxeter presentation = kernel of realization on
  reduced words' for the F2/Track-B cell-store schema ... the exact obligation wyrd's instance
  inherits when it mechanizes DESIGN-ua-base-vocabulary §5 step 4" (WYRD-INTAKE W14;
  `iu:notes/2026-07-17-session-close-overnight-eta.md` §HANDOFF). Mathematics tells the engineering
  precisely what its certificate-equality must prove.

### 3.4 Instance D — the differential machine caught a real bug (engineering rigor loop)

- "**SEQUENT-001 fixed** ... the L1 property differential caught a real machine bug — first-order
  natives whose results carry argument thunks declined instead of returning"
  (`wyrd-notes:identity-reflection-arc-handoff-2026-07-12.md`). The `L ≡ run` differential (a
  second machine face checked against the CEK oracle) surfaced a defect no unit test had. This is
  the engineering-discipline analogue of the math loop and argues for keeping the differential
  faces as first-class (§10).

### 3.5 Instance E — the math is being pushed *past published literature* to serve the engineering

- iu's η/rig-coherence proof (bead `iu-c2h.1`), undertaken to complete gandr's computable ua-base,
  **exceeds the only existing formalization**: Choudhury–Karwowski–Sabry (POPL 2022) "admitted
  their" multiplicative 2-cells (`--allow-unsolved-metas`, `eval₂-aux = TODO-`, 118 admit sites);
  iu's completion "is the only machine-checked multiplicative/rig coherence anywhere ... the claim
  DECAYS if someone formalizes it first" (`iu-notes:secured/iu-c2h1/RESUME.md` §"Why resume";
  WYRD-INTAKE W15). The engineering demand (certify gandr's negation/permutation univalence)
  produced a genuine mathematical novelty.

### 3.6 Instance F — literature evaluation is explicitly steered by "what wyrd must implement"

- The iu maintainer's directive on the polarization/focusing literature sweep: "the PRIORITY
  evaluation axis at synthesis is whether polarization/focusing SIMPLIFIES THE MACHINERY WYRD MUST
  IMPLEMENT (certificate composition / VDC reflection seam)" (`iu:notes/2026-07-17-session2-state.md`
  §"superseded heading"). Outcome (WYRD-INTAKE W17/W18): the **produoidal normal form** is
  recommended as "wyrd's two-mode home" — two interacting composition modes (sequential ◁ +
  parallel ⊗) with lax interchange, off-the-shelf normalization as a free idempotent monad, and a
  **polarized collapse** (on pure produced/consumed objects, sequential = parallel — interchange
  is free). Mathematics literature triaged by engineering payoff, then fed back as a design target.

### 3.7 The one contradiction / stale anchor in the feedback layer

`iu-notes:secured/iu-c2h1/RESUME.md` §"Why resume" points at `wyrd docs/gandr/spec/proposal-ua-base.md
§3.3` for the U3 gate that names the acyclicity-floor-is-operationally-sufficient claim. That file
**does not exist** in `wyrd@failed-refactor` (verified: the spec dir has `proposal-vdc-reflection.md`,
`proposal-identity-univalence.md`, and the four `*-addendum-vdc.md`, but no `proposal-ua-base.md`).
The WYRD-INTAKE ledger itself warns of exactly this: "wyrd's vendored pin ... may LAG the referenced
state — the push freeze is still on" (iu-notes:WYRD-INTAKE.md §Protocol). **Record, do not
harmonize:** the ua-base/U3 content is real but its wyrd home is `proposal-vdc-reflection.md`, and
the RESUME anchor is either aspirational or points at unpushed state.

---

## 4. Sequent / System-L kernel — the central design synthesis (carry into PLAN, top priority)

Primary sources: `wyrd-notes:sequent-machines-plan.md` (the D1–D9 decisions),
`wyrd-notes:digest/sequent-machines-dossier.md` (the file:line as-built machine dossier).
Both are *plans/analyses*, not spec; the spec home in wyrd is
`wyrd@failed-refactor:docs/gandr/spec/proposal-sequent-kernel.md` (+`-addendum-vdc`), but the
notes carry the load-bearing synthesis and the CEK→L correspondence that no spec fully records.

**The one-sentence thesis (carry verbatim):** gandr's machine already *is* a sequent machine
wearing a CEK costume — `eval.rs::meet` is the cut, `Cont::Bind(x,u,ρ)` is `μ̃x.⟨u‖e⟩`, `Arg` is
the application coterm, the `Vec<Cont>` continuation is a consumer coterm — so the L move is a
controlled *reification* (make commands first-class IL data), not a rewrite
(`wyrd-notes:sequent-machines-plan.md` §0; dossier §4 gives the full frame→consumer table).

The nine decisions (each carry-into-PLAN; the plan file is the authority):

| # | Decision (condensed) |
| --- | --- |
| D1 | Adopt polarized System-L (λμμ̃) as the machine-and-optimization **IL** — a second face beside the CEK first, promoted to canonical runtime IL with the arena migration. Frozen CBPV core stays the source/typing calculus; L is a conservative *presentation* of CBPV (Curien–Fiore–Munch), not a rival semantics. |
| D2 | **Evaluation strategy = polarity orientation on the μ/μ̃ cell**: positive cuts fire μ-first (CBV), negative fire μ̃-first (CBN). Strategies become orientation choices on polygraph cells — "the project's stated desideratum, realized literally." |
| D3 | **Fusion = Squier completion on cut seams.** User `rule` 2-cells elaborate to command rewrites `⟨p‖c⟩ ⇝ ⟨p'‖c'⟩`; rule overlaps at a cut are critical pairs; a completion engine synthesizes fused rules with tracelet 3-cell certificates, differential-tested. No pragmas. |
| D4 | **Codata MVP with zero frozen-core spend**: `codata` blocks, `cocase` copatterns, elaborated through a `Cosplit` case-tree node to a record-of-thunks over the existing record former + `U_r`. |
| D5 | **Corecursion = `fix` over a `Cosplit`** (ADR-57 reused). Productivity ladder: step budget (now) → syntactic guardedness (next) → sized types (reserved). |
| D6 | **Sessions = linear, two-sided, typestate-indexed codata**, gated behind codata MVP. Under L: session duality = logical duality, communication = cut — theorem-shaped. |
| D7 | **Levitation as a staged ladder** (see §7). Stage 0 costs zero dependent types; stage 1 needs exactly {one universe bump, Σ value types, large elimination}. |
| D8 | **Closure conversion as in-IL 2-cells** (Sullivan abstract closures at the `U`/`force` boundary only), feeding a CC-normal-form Cranelift pre-lowering where `jit ≡ eval` is a theorem. |
| D9 | **Implicit-argument elaboration via contextual metavariables** (Miller-pattern solver, without-K variant) — kernel-agnostic, schedulable independently. |

**Sequencing / phasing (carry into PLAN):** L0 (command face + focusing `𝓕`, no frozen-core
touch) → L1 (the L machine, `L-run∘𝓕 ≡ run` gate) → L2 (2-cells on command seams = fusion) → L3
(promotion decision — *is L the frozen-core canonical machine?* — explicitly gated and reversible;
the GHC "Sequent Core stays an IL" position is the default). Build order and the 13 advisory
follow-up beads are in `wyrd-notes:sequent-machines-plan.md` §§9,11.

**As-built status (dossier + `wyrd-notes:NEXT-STEPS-2026-07-14-kernel-landing.md`,
`wyrd-notes:project-coherence-sweep-2026-07-12-rerun.md`):** L0 is built; **L1 pure spine is
merged** (corpus differential green, zero disagreements); effects/control (perform/handle/
shift/reset/resume) landed; the remaining L1 gap was **un-focusing readback** (`wyrd-vcgm`), which
then feeds the **L3 promotion ADR** (`wyrd-c1a6`). Answer-of-record until L3: **CEK remains primary
+ oracle; L is the parity-gated kernel IL.** `gandr-sequent` crate exists (verified).

**Honest limits to carry (dossier §Risks, plan §3.4/§10):**
- **Natives are opaque to fusion.** `prim.rs` reduces in Rust with no producer/consumer seam;
  cut-elimination / critical-pairs cannot see through it. Never claim "fusion for the whole
  language" until hot natives are re-expressed as L cells or the boundary is accepted.
- **Two step budgets** (`eval.rs:597` mirrored in `gandr-shell/src/driver.rs:42`) desync silently
  if an L rewrite changes step accounting — keep a single shared constant.
- **η-law hygiene is a soundness constraint, easy to miss:** codata-η is valid only under CBN,
  data-η only under CBV; the completion engine must consult cut polarity before any η step (plan
  §3.2, risk register "η misuse ... high if missed"). Pin with a pathological corpus example.
- **Frozen-core §0 window contention:** ADR-54's queued nominal-tag touch, any L3 promotion, and
  the stage-1 dependent-types bill all compete for one narrow post-arena window.

---

## 5. The CEK machine as-built + the ADR-50 arena (carry into notes; reconstruct in PLAN)

`wyrd-notes:digest/sequent-machines-dossier.md` is a file:line inventory of gandr-core as it
stood. The reboot cannot rely on those line numbers (the tree has since moved and this is a
reboot), but the **architecture facts** are durable reference:

- **CEK configuration** `State{focus,env,cont,contenv,gensym,steps,prelude}`; 5 frame kinds
  (`Arg`/`Bind`/`Prj` structural + reifiable; `Reset`/`Handle` runtime-only); `step`→`meet`/`drive`;
  `eval_comp` = pure-spine recursive *reference/oracle* (declines control/effects via
  `UnsupportedByReference`); `run`/`step` = full iterative CEK; **CBN memo via `MemoCell`/`ThunkMemo`
  black hole**; step budget 1e6; effects via `Reset`/`Handle` frames + α-renamed `ContEnv`;
  eliminators reduce in place by env-extend.
- **ADR-50 status:** arena substrate (`NodeArena`/`FlatArena`, **append-only, no dedup**),
  first-order `(Env, NodeId)` closures, CEK env machine, CBN memo = **built**. Term
  content-addressing/hashconsing (`wyrd-yg03`), glued NbE values (`wyrd-zg5r`), a dedicated NbE
  driver, and frozen-core-on-arena = **not built**. Interning that exists is confined to the
  `mark.rs` marking layer, not the hot path.
- **`mark.rs` is total semantic error *marking* (Zhao et al. POPL 2024, "marks not aborts"), NOT
  rewriting** — its "certificates" are error localizations, not Squier 3-cells. Important
  correction to carry so no one conflates it with the polygraph layer.
- **Recommendation (dossier §Adoption):** build L as **option (a) first** — a second machine face
  beside CEK with an `L ≡ run` differential — reusing `conformance.rs`; do NOT abandon the reified
  `K` for a pure reducer (named dead-end, ADR-50 C); do L over the Rc-tree first, migrate to arena
  second (content-addressing, needed for polygraph node identity, is unbuilt).

Classification: **carry into new-repo notes** as the machine-architecture reference; the *design
decisions* (L-as-second-face, reuse reified K, arena sequencing) are **carry into PLAN**.

---

## 6. Fusion-by-completion / polygraph / codata / sessions (carry into PLAN)

From `wyrd-notes:sequent-machines-plan.md` Parts II–III (the not-yet-spec detail):

- **The fusion engine** (`gandr-polygraph`, new crate): cell store (oriented command rewrites as
  inspectable IR, keyed on arena content-addressing `wyrd-yg03`), multi-sum overlap enumerator
  (Behr Def 2.1 — the cut position makes overlaps *shallow*, which is why L makes this tractable
  where tree rewriting needs full subterm traversal), Knuth–Bendix/Squier completion loop, and
  differential correctness (fused ≡ two-step). Worked example: the `add`-on-`Nat` rules become
  command cells with a *definable* return-side constructor frame `Succ⁻(α) := μ̃x.⟨Succ(x)|α⟩`
  (the dialogue's ad-hoc `KSucc`, now precise).
- **Codata MVP (route a, zero core spend):** one lhs-problem elaborator generalizing Maranget
  (Cockx–Abel §5) — the data fragment degenerates to Maranget, so it *extends* the patterns lane,
  not a second engine; one new `Cosplit` case-tree node; codata value → record-of-thunks over the
  record former; **no codata η** (undecidable + breaks the elaborator scope invariant). Reserved
  route (b): a CBPV-faithful labeled n-ary negative product — the representation under which
  cocase/destructor cuts become native fusion seams.
- **Sessions/async/generators/iterators** unify as three buckets: plain codata (stream/iterator;
  `for`-over-codata = observation loop), handler→codata (generator/async), genuine session
  (linear grade-1 typestate codata + a **duality involution** — the one genuinely new type-level
  structure; channel creation duality-gated). The coinductive relation engine (bisimulation =
  equivalence, simulation = subtyping, decidable) doubles as the codata-equivalence engine.

Honest limits (plan §3.4): non-linear overlaps produce rule *families* not single fused rules
(only the Σ-zone linear-wire case is unique); completion may not terminate (convergent-slice
restriction + budget + decline-with-report). **Do not claim "Haskell-free deforestation
everywhere"; the honest claim is "principled fusion on the seam-visible convergent fragment."**

---

## 7. Levitation / Desc, staged (carry into PLAN)

`wyrd-notes:sequent-machines-plan.md` Part IV (D7). The dependent-types bill turned out **small and
precise**, which is the load-bearing finding:

| Stage | Content | Prerequisites | Payoff |
| --- | --- | --- | --- |
| **0** | Decl table in `tagDesc` shape + Rust generic ops (derive, wire serialization, uniform polygraph ops) | none — gandr has enums/records | the owner's three payoffs **now, zero core dependent types** |
| **1** | Closed typed `Desc` universe + trusted decoder, **first-order fragment only** | exactly {one predicative universe bump, Σ value types, **large elimination**} — *not* full Π-over-SET | typed generic eq/serialization |
| **2** | User-facing generic programming; generic induction/catamorphism; free monad `D*` | stage 1 + the `μ⁺` value rung | generics written once in gandr |
| **3** | Full levitation (`Desc = μ DescD`) | universe stratification | quote/unquote collapse |

**gandr's five extensions to `Desc`** (resolving the spec's flagged gap): 2-cell faces = pairs of
free-monad `D*` elements; multi-out arities = the **bridge diagram** (this is exactly `arity.rs` —
see §3.1); grades (erased by the value decoder); binder/atom-abstraction fields; attributes. The
positivity story: datatypes enter the kernel as **description codes drawn from strictly-positive
functors — positivity by construction, unrepresentable rather than rejected**, deleting the
bug-rich syntactic positivity checker (`wyrd-notes:universe-stratification-synthesis-2026-07-13.md`
triad item 2). This is also the strongest coupling between levitation and the soundness plan (§8).

---

## 8. Universe stratification, Prop, the soundness triad (carry into PLAN; adopted as wyrd ADR-78)

Primary: `wyrd-notes:universe-stratification-synthesis-2026-07-13.md`. This synthesis was **adopted
as wyrd ADR-78** (`wyrd-notes:triage-plan-2026-07-12.md` §"Session 3 final addendum"), so its
decisions have a spec home in wyrd — but the *rationale, the rejected-alternatives table, and the
triad threat model* are richer in the note than in a typical ADR body and are worth carrying whole.

**The kernel/elaborator split, applied to universes:**
- **Kernel:** levels as *meta-level data* (judgments, not a type), algebra `{0,+1(⁺),max(∨)}` — a
  join-semilattice whose word problem is **proven polytime with a sorted normal form** (Bezem–Coquand
  TCS 2022); strict `U_l : U_m iff l < m`; **no cumulativity** (explicit lifts), **no imax** (no
  impredicative Prop), **no level constraints in the kernel**; per-declaration prenex level
  polymorphism.
- **Elaborator (untrusted):** all inference — Bezem–Coquand loop-checking as the *complete* solver
  (never stuck on max=max, eliminating Lean's annotation pain; Sozeau–Bezem Rocq branch is the
  precedent), prenex generalization at declaration close, McBride displacement as the zero-solving
  UX default.
- Hits all three design dimensions: local-and-complete inference per definition, no global
  constraint graph anywhere (cures Rocq's 25%-compile-time disease), library composition by prenex
  instantiation against stable interfaces.

**The hybrid verdict (half survives):** DIES — atoms-as-displacement-base and ONS/ordered-atom
algorithms (twice-refuted: max-as-composition breaks left-invariance; order-homogeneity is the
opposite of named landmarks). SURVIVES — **named level landmarks as a declared poset** (e.g.
`kernel < surface < tooling`), which the POPL'23 framework supports natively as distinguished level
constants; adds no global-analysis fragility because it is *declared interface, not inferred state*.
Open research: landmark *joins* (`γ = α∨β`).

**Prop:** not now, and never impredicatively (Hurkens/Geuvers hazards; drags imax back). gandr's
erasure needs are covered by the grade/phase axis (ADR-32); **erasure ≠ irrelevance** (different
axes). If definitional proof irrelevance is ever wanted: a *predicative* SProp (POPL'19,
without-K-compatible) + sort polymorphism — bracketed re-entry, keep identity out of any such sort.

**Unsoundness triad — threat model (carry whole):** (1) *Type:Type* — mitigated by strict
stratification over the decidable algebra; kernel-resident level constraints **banned**. (2)
*Curry/positivity* — structural: levitated Desc codes strictly positive by construction (§7),
deleting the syntactic positivity checker. (3) *Non-termination* — three layers: CBPV effect
quarantine (divergence is an effect; evidence phase admits only thunkable/total terms), step budgets
(engineering guard, not a logical guarantee), totality obligations (sized types long-term,
guardedness interim — type-based, avoiding the syntactic guard checker that is empirically the #1
kernel bug source).

**The Bezem–Coquand oracle was built** as `gandr-kernel-levels` slice 1+2 (no_std, zero-dep,
evidence-returning, differentially tested against an independent free-AST semantic reference)
(`wyrd-notes:triage-plan-2026-07-12.md` sessions 3–4). Surface universe keyword = **`Type`, not
`Set`** (owner steer). Kernel-boundary design record: `wyrd@failed-refactor:docs/gandr/spec/kernel-boundary.md`
(verified present), behind wyrd ADR-77/78.

---

## 9. Parser replacement — tree-sitter → meldr/PBG (carry into PLAN; complete gated plan)

Primary: `wyrd-notes:parser-replacement-analysis-2026-07-09.md` (1,543 lines, provenance-tagged
MEASURED/CITED/ESTIMATED). Status: analysis, "not yet an ADR." This is a **ready-to-execute
engineering epic** and the most self-contained carry-into-PLAN document in the corpus.

**Recommendation (E′):** replace tree-sitter with a Rust port of tylr's `meldr` calculus —
tile-based operator-precedence parsing with material obligations — over a **flat-arena CST with
stable `NodeId`s and per-node merkle hashes**. Do **not** build an incremental parser; inherit
incremental locality from OP parsing's bounded-context property and *decline to exploit it*. Keep
tree-sitter as a generated, parity-gated tooling artifact whose `grammar.json` is **projected from
the same PBG** that drives the melder.

**Three measured findings that invert the original framing (carry as decided facts):**
1. gandr does **no** incremental parsing today — every `Parser::parse` passes `None`.
2. Incrementality buys nothing at scale — largest `.gandr` file 1,902 B; whole corpus 45,180 B;
   cold-parses in <1 ms; every measured incremental scheme makes the *cold* parse slower.
3. The mixfix restriction is a **deliberate declaration-independence invariant** (required for sound
   reuse), not a tree-sitter artifact — it survives the rewrite unchanged.

**Why the harder base wins:** totality is **Theorem 3.4** (parse is sound and total by
construction), not a recovery discipline; the **8-class obligation taxonomy** lowers directly onto
gandr's `HoleNote`s (gandr adds `AmbiguousPrec` at max severity — parse totally, classify, let
lowering decide what is an error, satisfying ADR-43 D3's no-warn-and-guess rule); **tiles are data**,
so open mixfix becomes a *priced option* rather than impossible (reaches `ADR.md:1196`'s reopen
trigger); merkle identity is `O(1)`, sound, complete, **whitespace-invariant** (dissolves the
OM-over-CST resync bead `wyrd-c5h3`) — the requirement tree-sitter *and* rowan both fail.

**The load-bearing divergence to price:** tylr's precedence is an integer level; ADR-43 D3 mandates
a **named DAG with incomparability**. Contained (`Prec.re` is 40 lines behind `lt`/`gt`/`eq`), but
the metatheory (Theorems 3.1/3.4) was proven over a *total order* and is **not** checked against a
partial order — "the largest unverified theoretical claim in this document" (§13).

**Two kill-gates, both before `lower.rs` is touched (fallback = option B, hand-written resilient RD
over the same arena — a strict subset):** P2 asks whether gandr's grammar *is* a PBG (Operator Form
+ Unique Tiles); P3 is a **perf spike** (largest file < 500 µs, corpus p99 < 1 ms) — tylr's authors
explicitly did not optimize and "make no strong claims," so this is the plan's central risk.

**Single-source-of-truth win:** under E′ the melder/molder are *generic over the PBG*, and
`grammar.json` is a *projection* of the same `Pbg`, so acceptance drift reduces to projection
fidelity (one ~200-line function). 15-phase migration plan (Phase 0 = de-tree-sitter `edit.rs`, a
free unconditional win) and the P0–P15 bead epic are fully specified in §§10–11 of the note.

**Unconditional corrections to make regardless of outcome (carry into PLAN as bugs):** the node-id
"stable across **unchanged** subtrees" claim is *unsound* — tree-sitter guarantees only
`reused ⇒ same id`, and the stored `cst_node_id` is the address of a freed subtree slot (a loaded
gun, ABA-unsound the moment a diff is implemented). Fix the `origin.rs`/`incremental-pipeline.md`
wording and delete-or-replace `cst_node_id`.

**Adjacent finding referenced (drop for PLAN, note only):** the syntax fold-in inventory
`wyrd-notes:syntax-inventory-2026-07-11.md` (bead `wyrd-ku0f`) already specifies **which concrete
surface constructs enter the PBG grammar now** (data/codata/`def rec`/copatterns/loops/reserved
op/rule/GADT/grade slots) with per-construct grammar-engineering sketches and a 9-item risk section
(the `ident:Type` collision family R1, `co` vs `codata` longest-match R2, reserving `op`/`rule`/
`with`/`in`/`rec` breaks identifiers R6). This is the PBG's *content* to the parser's *engine* — a
direct carry-into-PLAN input for whoever builds `gandr-grammar`.

---

## 10. Process, methodology, and hazards not recorded in any ADR (carry into notes; some into PLAN)

These are the durable *how-we-work* learnings scattered across the handoffs. None is in an ADR; the
reboot should institutionalize the load-bearing ones.

**Differential-testing discipline (the house quality mechanism — carry into PLAN as an invariant).**
Every new machine/face lands with a differential oracle row: `checker ≡ machine`, `eval ≡ run`,
`L-run∘𝓕 ≡ run`, `fused ≡ two-step`, `jit ≡ eval`. Realization ratchets (`corpus_differential.rs`
floors) only ever rise. This caught SEQUENT-001 (§3.4) and is the reason the L migration is
low-risk (`wyrd-notes:sequent-machines-plan.md` §2.1; `wyrd-notes:identity-reflection-arc-handoff-2026-07-12.md`).

**Features land WITH corpus examples** (owner pain, bead `wyrd-4xxf`): "features landed without
examples demonstrating usability" — evaluate a corpus-coverage gate
(`wyrd-notes:NEXT-STEPS-2026-07-14-kernel-landing.md`).

**Worktree isolation + persist-failing-artifacts** (iu standing rule
`workers-persist-failing-artifacts`): every worker brief instructs committing failing/abandoned
attempts as labeled `wip:` commits; checkpoint before any significant course change; greenness
applies to the integrated deliverable only (`iu-notes:HANDOFF-doctrinal-carrier.md` §8). Preserved
attempt branches are named and kept (e.g. `iu-48k-paste-deceq-attempt`, `iu-c2h.1-staging`).

**The K-free witness discipline** (iu, standing rule wired into every indexed-family brief):
B-μ graph-of-μ witnesses, B-diag derive-the-diagonal, "no green slime"
(`iu:notes/2026-07-16-session-close-s2-spike-landed.md` §"Sequence ADOPTED"). Relevant to gandr's
elaborator when it mechanizes the ua-base instance.

**BlanketBase false-floor discipline** (iu): when *stating* a higher universal property, fix the
alphabet/quantifier range in the statement — the `BlanketBase` module is the permanent compile-only
witness that over the blanket all-cells alphabet, base-sphere acyclicity is *underivable*
(`BlanketBase → UIP`), so an under-restricted statement is a false floor
(`iu:notes/2026-07-16-iu-sz9-scout-report.md` §7). The `eq²` Coh-witnessed-alphabet restatement is
"normative when stating higher UPs" (maintainer, `iu:notes/2026-07-16-session-close-s2-spike-landed.md`).

**Doc/manual doctrine (owner, binding — carry into notes for when the reboot writes docs):** the
manual is "not a historical document" — current state + why + trajectory only; no
migration/candidate-survey/decision-log narratives; no internal-sentiment voice; **proposals retire
THROUGH implementation and exhaustive absorption into the manual**; Lean-reference-manual skeleton +
fp-lean voice; the identity-univalence honesty gate binds manual claims
(`wyrd-notes:triage-plan-2026-07-12.md` sessions 6–7; `wyrd-notes:manual-restructure-plan-2026-07-13.md`).

**Publishable-history / provenance:** machine-local paths are rejected by a publishable-history hook;
sibling repos are named without paths; contributor-concern (session forensics, SHAs, harness
mechanics) stays out of tracked history (`wyrd-notes:triage-plan-2026-07-12.md` §"Session 6").

**Recurring harness hazards (carry into notes; mostly drop for PLAN):**
- **1Password can lose signing mid-interactive-session** — retry once, else land unsigned + queue a
  re-sign pass; the AFK unsigned-fallback pattern is maintainer-approved
  (`iu-notes:NEXT-STEPS.md`; multiple iu session-closes).
- **`bd update --notes` REPLACES the notes field** (clobbered `iu-wsg` once) — always re-paste prior
  content (`iu:notes/2026-07-17-session-close-funext-push.md`).
- **MANIFEST hash ordering matters**: edit → treefmt → b3sum → MANIFEST (treefmt rewrites markdown;
  a pre-treefmt hash is stale); no `manifest:update` task exists
  (`iu:notes/2026-07-16-session-close-s2-spike-landed.md`).
- **Worktrunk hook bugs** (fixed upstream: max-sixty/worktrunk#3453): WorktreeCreate omitted
  `--base=@` (worktrees born at MAIN not session HEAD) and died outside a git repo; `wt merge`
  *rebases* (drops merge commits) and re-creates commits unsigned
  (`wyrd-notes:triage-plan-2026-07-12.md` §"Harness bugs", sessions 2/4).
- **Honest-pipes (H20):** never pipe gate/mutation commands through output filters; a PreToolUse
  hook denies mutation-pipes; bound output with the blessed `gate.nu` wrapper
  (`wyrd-notes:identity-reflection-arc-handoff-2026-07-12.md`).
- **iu commit hooks are UNINSTALLED** and the act-CI workflow is broken — apply commit conventions
  manually, never `--no-verify` (`iu:notes/2026-07-16-next-session-procontext-and-s2.md`).

---

## 11. The in-repo `iu:notes/` directory — full inventory + classification (mandated sweep)

**Finding:** the in-repo `iu:notes/` directory exists, holds **54 entries**, and is **gitignored**
(`iu:.gitignore:41` `notes/`, under a "Contributor-local working notes ... untracked" comment). So
it is *working-tree contamination, not a committed leak* — the content will not enter iu git history.
It **byte-duplicates `iu-notes:secured/iu-c2h1/`** for every shared file (verified: zero content
diffs across all common files), plus holds **6 unique session-close/next-session files** not in
secured, and *lacks* secured's `RESUME.md`. It is contributor-concern working material for the iu
η-coherence proof campaign (bead `iu-c2h.1`) and the S2/funext threads.

**Overall classification: the whole directory is DROP for the gandr PLAN** (it is iu-side proof
mechanics, not gandr design), **and its durable content is already carried into notes elsewhere**
(WYRD-INTAKE W13–W18, the iu DESIGN specs, and the 8 session-close files). The one gandr-relevant
distillate content (produoidal/polarization) is fully captured in WYRD-INTAKE W17/W18. Recommend the
coordinator flag to the owner that this directory be removed or relocated (it clutters the iu
working tree; secured already backs it up).

Complete inventory by category (all in `iu:notes/`; all read or title-inventoried):

| Group | Files | Content | Classification |
| --- | --- | --- | --- |
| **Session-close / next-session** (8; 6 unique + 2 shared w/ secured) | `2026-07-15-next-session-and-gandr-feedback.md`, `2026-07-16-iu-sz9-scout-report.md`, `2026-07-16-next-session-procontext-and-s2.md`, `2026-07-16-session-close-s2-spike-landed.md`, `2026-07-17-session-close-{eta-wave2,funext-push,overnight-eta}.md`, `2026-07-17-session2-state.md` | iu S2/funext/η session state + **the gandr-feedback file** (§3) | durable content → **notes** (via WYRD-INTAKE, already done); session mechanics → **drop** |
| **Per-wall η handoffs** (14) | `iu-c2h1-{core,core2,dp,dp2,du,swap2,swap3,wtr,wtr2}-handoff.md`, `iu-c2h1-letterc-w{3,4,6,6-out,6-swap}-handoff.md` | iu proof-state handoffs for the 8 coherence "walls" (⊕swap, ⊗assoc, ⊗ʳ, dist…) | **drop for PLAN**; carry into **iu-notes** only if `iu-c2h.1` resumes |
| **Agda proof scratch** (18) | `iu-c2h1-*-scratch.agda.txt`, `iu-c2h1-*-probe-*.agda.txt`, `iu-c2h1-dp2-infra.agda.txt`, `swap-braid-rr-dev.agda` | verbatim Agda proof fragments (lemmas, probes) | **drop** — proof mechanics; iu object-store branches hold the canonical form |
| **Literature distillates** (10) | `iu-c2h1-lit-{cfl,collages,devices,distributive-cluster,graded,mellies,munch,polar-shuffles,produoidal,thesis}-distillate.md` | 7+3-source polarization/two-mode-algebra sweep verdicts | durable verdicts → **notes** (already in WYRD-INTAKE W17/W18); produoidal + thesis are the wyrd-relevant keepers, rest are ruling-out records → **drop** |
| **Archive** (1) | `iu-c2h1-notes-archive-2026-07-17.md` (41 KB) | archived chronological log of the `iu-c2h.1` bead statement of record | **drop** — superseded by the bead comment stream |

**Distillate one-liners (for the coordinator, source: the 10 title lines + WYRD-INTAKE W17/W18):**
produoidal (Earnshaw–Hefford–Román, CSL 2024) = **CONFIRMED, wyrd's two-mode home**; Román thesis
(Monoidal Context Theory) = **primary citation + implementation source** (C3 resolved, C1 hardened,
C2 VDC-bridge stays wyrd-bespoke); Melliès chiralities = REFUTED (negation polarity); Munch duploids
= REFUTED (divergence-driven; keepers: distribute-first, stoup move); CFL of string diagrams = no
φ-classifier (keeper: Thm 6.6 stoup move); graded-monoidal = wrong (flipped) axis; devices =
negative (block vs mediate); polar shuffles = REFUTED (reversibility collapse); collages = hard
negative (VDC/equipment = 0 hits); distributive-cluster = only rig-adjacent find, inapplicable but
two conceptual mirrors kept. Meta-finding: three distinct collapse mechanisms (divergence /
interchange / reversibility) killed the effectful frameworks; convergent pointer = Cockett–Seely
linearly-distributive categories if ⊕-with-distributor polarization is ever needed.

---

## 12. Consolidated classification of every distinct knowledge item

Legend: **PLAN** = carry into the project plan (design/architecture/sequencing that shapes what
gets built) · **NOTES** = carry into new-repo notes (durable reference, provenance, hazards,
coordination) · **DROP** = do not carry (with reason).

| # | Knowledge item | Primary source | Class | Note / reason |
| --- | --- | --- | --- | --- |
| 1 | gandr = reboot with empty docs; wyrd = prior full build (85 ADRs) is the salvage corpus | this repo state; `wyrd@failed-refactor:docs/adr/` | PLAN+NOTES | frames the whole reboot |
| 2 | Polarized System-L kernel D1–D9 + L0→L3 phasing | `wyrd-notes:sequent-machines-plan.md` | PLAN | central design; top priority |
| 3 | CEK↔L frame→consumer correspondence (`meet`=cut, `Bind`=μ̃) | `wyrd-notes:digest/sequent-machines-dossier.md` §4 | PLAN | the reification thesis |
| 4 | CEK/ADR-50 as-built architecture (arena, memo, mark.rs≠rewriting) | dossier §§1–3 | NOTES | reference; line numbers stale |
| 5 | Fusion-by-completion engine (`gandr-polygraph`, overlaps, tracelet certs) | `wyrd-notes:sequent-machines-plan.md` II | PLAN | needs content-addressing first |
| 6 | Codata/copatterns/corecursion MVP (Cosplit, record-of-thunks, no η) | plan III | PLAN | zero-core-spend route |
| 7 | Sessions = linear typestate codata + duality involution | plan III D6 | PLAN | gated on codata MVP |
| 8 | Levitation ladder stage 0–3; dependent bill = {Σ, 1 universe, large-elim} | plan IV | PLAN | small precise bill is the finding |
| 9 | Desc's 5 gandr extensions (2-cell=D*, multi-out=bridge, grades, binders, attrs) | plan IV §5.2 | PLAN | bridge = arity.rs (§3.1) |
| 10 | Universe stratification (BC oracle kernel, prenex, landmarks) = ADR-78 | `wyrd-notes:universe-stratification-synthesis-2026-07-13.md` | PLAN | adopted; rationale richer than ADR |
| 11 | Prop stance (no impredicative; SProp bracketed; erasure≠irrelevance) | ibid | PLAN | design invariant |
| 12 | Unsoundness triad threat model (Type:Type / positivity / non-term) | ibid | PLAN | positivity-by-Desc-construction |
| 13 | Parser: meldr/PBG over merkle flat-arena; totality theorem; obligation taxonomy | `wyrd-notes:parser-replacement-analysis-2026-07-09.md` | PLAN | gated epic, P0–P15 |
| 14 | Parser: 3 measured findings inverting incrementality framing | ibid §2 | PLAN+NOTES | decided facts |
| 15 | Parser: node-id unsoundness + freed-slot cst_node_id | ibid §2.3 | PLAN | unconditional bug fix |
| 16 | Prec-DAG generalization is the largest unverified metatheory claim | ibid §13 | NOTES | risk to track |
| 17 | Syntax fold-in inventory (which constructs enter PBG now) + 9 risks | `wyrd-notes:syntax-inventory-2026-07-11.md` | PLAN | PBG content input |
| 18 | **arity.rs → iu DΣ additive-not-refactor** (engineering→math) | `iu:notes/2026-07-16-session-close-s2-spike-landed.md`; verified `wyrd@failed-refactor:crates/gandr-desc/src/arity.rs` | PLAN+NOTES | owner's headline pattern |
| 19 | VDC reflection: concrete carrier drives F2; UnitLayer certifies ua-base | `iu-notes:HANDOFF-doctrinal-carrier.md`; `iu:notes/2026-07-15-next-session-and-gandr-feedback.md` | PLAN+NOTES | bidirectional loop |
| 20 | Tracelet/NFC: math hands engineering its certificate-equality obligation | `iu-notes:WYRD-INTAKE.md` W9/W14 | PLAN+NOTES | pattern instance |
| 21 | iu η-coherence exceeds published (CKS) literature | `iu-notes:secured/iu-c2h1/RESUME.md`; WYRD-INTAKE W15 | NOTES | novelty provenance |
| 22 | Produoidal normal form = wyrd's two-mode home; polarize certificate boundaries | WYRD-INTAKE W17/W18 | PLAN+NOTES | design target for fusion IL |
| 23 | WYRD-INTAKE ledger as standing iu→wyrd coordination infrastructure | `iu-notes:WYRD-INTAKE.md` | NOTES | adopt the protocol |
| 24 | Differential-testing discipline (all `X ≡ run` faces; ratchets) | multiple wyrd notes | PLAN | quality invariant |
| 25 | Features-land-with-examples; corpus-coverage gate | `wyrd-notes:NEXT-STEPS-2026-07-14-kernel-landing.md` | PLAN | owner-mandated |
| 26 | Worktree-isolation + persist-failing-artifacts; K-free discipline; BlanketBase false-floor | iu handoffs | NOTES | working-method reference |
| 27 | Manual/doc doctrine (current-state-only; absorb proposals; Lean shape) | `wyrd-notes:manual-restructure-plan-2026-07-13.md` | NOTES | for when docs get written |
| 28 | Bootstrapping reframe (minimal certified kernel, Lean model) = wyrd-tpgh | `wyrd-notes:triage-plan-2026-07-12.md` | PLAN | design invariant |
| 29 | Naming scheme (`here`/`then`/`back`/`walk`/`Path`/`Flow`) — iu-side | `iu:notes/2026-07-16-next-session-procontext-and-s2.md` | NOTES | iu vocabulary; may inform gandr |
| 30 | Harness hazards (1Password signing, bd --notes clobber, MANIFEST ordering, wt hooks, honest-pipes) | multiple | NOTES | recurring; institutionalize the fixes |
| 31 | in-repo `iu:notes/` = gitignored duplicate + 6 unique (full inventory §11) | this analysis | NOTES | flag for removal; content already carried |
| 32 | Contradiction: RESUME cites nonexistent `proposal-ua-base.md` | `iu-notes:secured/iu-c2h1/RESUME.md` vs `wyrd@failed-refactor` | NOTES | pin lag; recorded, not harmonized |
| 33 | Stale SHAs / tracker counts / session bookkeeping / bead purge cadence | triage/coherence-sweep notes | DROP | ephemeral; superseded by DoltHub tracker |
| 34 | iu per-wall η proof handoffs + Agda scratch (walls, lemmas) | `iu:notes/iu-c2h1-*` | DROP | iu proof mechanics; not gandr design |
| 35 | Literature ruling-out records (Melliès/Munch/devices/shuffles/graded/collages/CFL) | `iu:notes/iu-c2h1-lit-*` | DROP (keep verdict lines) | negative results; verdicts already in W17/W18 |
| 36 | Project-coherence audit ledgers (B-AUD/GDOC-GEN/PARSER/UDC IDs) | `wyrd-notes:project-coherence-sweep-2026-07-12-rerun.md` | DROP | audit bookkeeping tied to wyrd's old tree state |

---

## 13. Hazards and surprises for the coordinator

1. **The reboot starts from zero docs.** Nothing in `docs/`, no ADRs. The entire wyrd ADR/spec
   corpus (85 ADRs) lives only in `wyrd@failed-refactor` and must be deliberately re-adopted, not
   assumed. Several decisions in these notes were *already adopted as wyrd ADRs* (ADR-77/78 universe;
   ADR-50 arena; ADR-54 data/patterns; ADR-57 recursion; ADR-66 codata) — the reboot must decide
   which to re-mint and which to leave in wyrd.

2. **`wyrd@failed-refactor` is literally a "failed-refactor" branch** (HEAD `df3ef8fe`, branch
   `failed-refactor`). Its spec/crate state is a *salvage source*, and at least one cited path
   (`proposal-ua-base.md`) is absent — treat file:line anchors in the notes as approximate against
   this tree, and verify before relying (the notes' own house policy: "verify the FILE / bead, not
   the hash").

3. **The parser plan's central risk is unmeasured performance** (tylr authors "make no strong
   claims"), and its metatheory (Prec-DAG vs total-order) is unverified. Both are front-loaded kill
   gates (P2/P3) — the coordinator should schedule them *before* any `lower.rs` retargeting so the
   fallback (option B) strands only `gandr-grammar`.

4. **Frozen-core §0 is a scarce shared resource.** Three future commitments — ADR-54's nominal-tag
   touch, any L3 promotion of commands into the core, and the stage-1 dependent-types bill — all
   compete for one narrow post-arena window. Content-addressing (`wyrd-yg03`) is the unbuilt
   precondition for the polygraph cell store *and* the arena migration; it gates the fusion story.

5. **The engineering↔math loop needs a policy, not just a ledger.** WYRD-INTAKE has 18 pending items,
   several `ACTIONABLE` (W17/W18 design-level). The push-freeze means wyrd's vendored iu pin lags —
   the coordinator should decide how the reboot consumes W1–W18 and how it reports absorbed/declined
   back (the protocol exists but has never round-tripped a full pass).

6. **The `iu:notes/` contamination is benign but real** — gitignored, so no history risk, but it
   duplicates secured working notes in the iu *working tree* and holds the only copies of 6
   session-close files (which I have distilled here). Recommend the owner relocate/remove it; nothing
   gandr-durable is lost by doing so.

7. **Two independent step budgets** (`eval.rs` and `gandr-shell/src/driver.rs`) will silently desync
   under any L step-accounting change — a latent correctness hazard flagged in both the dossier and
   the plan; the reboot should collapse them to one shared constant early.

8. **Adjacent lateral track not in scope but discovered:** an exact-reals + synthetic-topology (ASD)
   epic (`wyrd-buf9`) and a Nat/Int bigint rename (`Integer→Int`, add `Nat`) were adopted as a
   firewalled side-track (`wyrd-notes:NEXT-STEPS-2026-07-14-kernel-landing.md`). Flagged briefly; not
   pursued here.
