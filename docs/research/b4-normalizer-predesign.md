# B4 normalizer pre-design study — glued-NbE hash-consing over the L machine, module forms as named holes (gandr-wvd.4)

> **Status: PRE-DESIGN STUDY for owner review — not a decision record.** Design study for backbone phase B4 (`gandr-wvd.4`), produced overnight (2026-07-21, `gandr-9pv`) against the ratified B3-before-B4 ordering (`bd show gandr-wvd`, PLAN-review amendment 2026-07-20) and the fcw.11 backbone resolution (`bd comments gandr-fcw.11`).
> Nothing here is adopted until the owner says so; recommendations are marked as such, and §9 separates ratification-queue candidates from settled direction.
> **Consumers:** the B4 backbone phase itself (`gandr-wvd.4`), the future S2 conversion design, and the B3 landing (`gandr-wvd.3`), whose §7 forms this study duals into normalizer holes.
>
> **The B3-before-B4 mechanism.** B3 precedes B4 so the normalizer is designed against a term language that already has functor application and module projection (`bd show gandr-wvd`, constraint 4; b3-module-system-design.md §3.1).
> This study honors that ordering by **parameterizing the normalizer over the B3 module forms as named holes** (§5): B4's skeleton is built with every hole a neutral, and each hole is filled as its B3 rung lands — so B4 proper can start immediately after B3 without waiting for the whole module system.
>
> **Citation conventions.** Repo paths use the corpus alias style of the sibling research docs: `wyrd@failed-refactor:` = the canonical wyrd source tree; `ADR-NN` = `wyrd@failed-refactor:docs/adr/00NN-*.md`.
> Literature citations use register keys `[X-N]` from `docs/research/bibliography-v2.md`, with locators quoted verbatim from that register (§10).
> Two absorbed research records are cited by section: `impl-models` = `docs/research/impl-models-deep-read.md` (the performance-program primary record, `gandr-wgq`); `massive-term` = `docs/research/massive-term-design.md` (D1/D2/D3/RQ-n locators live there); `b3` = `docs/research/b3-module-system-design.md` (Q1-Q8/R1-R4 locators live in its §13/§6.3, and the N1-N6 forms in its §7).
> Stage records are cited by bead and timestamp: the **B2.1 stage record** = the `gandr-wvd.2` stage comment of 2026-07-20 17:10 (S1 conversion-algorithm design); the **B2 staging call** = the `gandr-wvd.2` comment of 2026-07-20 15:58 (the C1-C5 staging + K1-K5/E1-E6 kernel-boundary references); the **L-machine landing** = the `gandr-wvd.1` stage records (the core-sequent port).
> No machine-local paths and no retired-tracker bead IDs appear in this file.
>
> **Label locators (first-use hints).** `D1(C)`/`D2`/`D3`/`RQ-n` = massive-term §3/§4/§12.
> `C1-C5` staging calls + `K1-K5`/`E1-E6` = the B2 staging call + `spec/kernel-boundary.md` (ADR-77/78).
> `S0-S3` = the kernel subset ladder (`spec/kernel-boundary.md`; S1 = the pure polarized fragment, S2 = the term-indexed extensions).
> `Q1-Q8`/`R1-R4` = b3 §13/§6.3.
> `N1-N6` = b3 §7.
> New labels — the six module-form **holes** (`HOLE-PROJ`/`-APP`/`-SEAL`/`-VIEW`/`-PACK`/`-COERCE`), the **half-built-glue trap**, and the **generative-aliasing hazard** — are coined in this study and anchored at first use.
>
> **Method and caveat.** This is a paper study over the ADR-50 record, the two absorbed research digests, the B2/B3 stage records, and the L-machine landing; no code was written or run.
> The standing consult-before-design rule (`docs/workflow/rust.md`; `gandr-wvd.4` STANDING note) is honored: every concrete decision below cites the impl-models section it rests on.
> The digests read four external implementations at pinned commits (impl-models "Source checkout of record"); their `file:line` anchors are theirs, quoted only through the digest's own section numbers here.

---

## 1. Executive summary — the recommended design

1. **Ground truth: B4 is a growth of an existing seed, not a greenfield build** (§2).
   B4 inherits a nearly-complete substrate — the **L machine** (`machine.rs` over the ADR-50 arena; the sequent/focusing machine that supersedes the CEK once the L differential is green, `fcw.9`), the **shared first-order value domain** (ADR-50 Decisions C/D), the **D1(C)** per-environment arena with typed node ids and an **id-equality fast path** (massive-term §3.1, owner-veto pending RQ-1), and the **B2.1 conversion seed**: S1 conversion is TYPE-ONLY structural equality, and the quarantined term alpha-equality (`convertible_values`/`convertible_computations`) is implemented but **not invoked** — "the seed the term-indexed extensions will grow beta onto" (B2.1 stage record).
   B4 is exactly that growth.
2. **The load-bearing move is the two-face glued split** (§4.1; impl-models §5.2).
   "Glued" is **two** distinct mechanisms, and ADR-50 Decision D's phrasing conflates them: **term-face** gluing (a value caches the arena `NodeId` of the source term it came from — nearly free under the arena, adopt now) and **unfolding-face** gluing (smalltt's per-node `VUnfold head spine ~unfolded` — the neutral and unfolded forms retained together, built incrementally — the conversion/readback-facing half that interacts with Decision E).
   Building one half while believing the other exists is the named failure mode; this study coins it the **half-built-glue trap** and makes naming both faces in the value-domain type a landing invariant.
3. **The def-eq pipeline puts id-equality first** (§4.5).
   Order: id-equality (arena id-eq / `Rc::ptr_eq`) → cached-word guards → iterative structural comparison → lazy-δ with heights (taller-unfolds-first, args-first with a failure cache) → smart-unfolding gated on case-progress → `ConvState` (Rigid/Flex/Full) speculation with glued retry.
   This is the Lean recipe set (impl-models §2.2-2.4) plus smalltt's three-state discipline (§4.3), every face **iterative over a heap worklist** (the recursion dylint gate; B2.1 stage record).
4. **Hash-consing stays at the arena/decode boundary** (§4.3).
   Sharing is **preserved, never created** under β (massive-term §3.2, the C3 discipline); per-face interners exist only for static β-free data; the unfolding-face's incrementally-built unfolded values are **machine-local scratch past the admission watermark**, truncated after the verdict, never interned into the persistent arena.
   This is ADR-50 Decision B realized as D1(C)'s id-equality — no canonicalizing table under β, no table in the TCB.
5. **The normalizer is parameterized over six named module-form holes** (§5), the duals of b3 §7's N1-N6: `HOLE-PROJ`, `HOLE-APP`, `HOLE-SEAL`, `HOLE-VIEW`, `HOLE-PACK`, `HOLE-COERCE`.
   Each carries an interface contract and a **neutral-until-plugged** default; the B4 skeleton builds with all six as neutrals and fills each as its B3 rung (B3.1-B3.4) lands.
6. **The two headline holes are the stated reason B3 precedes B4.** `HOLE-APP` (functor-application unfolding; N2) is atom-minting, **stateful**, and must **never memoize or hash-cons across instantiations** — the content-address must include the minted atoms in value identity or two instantiations silently alias (the **generative-aliasing hazard**, coined here; the single sharpest constraint B3 exports, b3 §7 N2).
   `HOLE-PROJ` (module-projection unfolding; N1) demands **spine-local whnf only** — normalize the head to structure form, never the sibling components.
7. **Sealing is the first language-level unfolding barrier the normalizer meets** (§5, `HOLE-SEAL`; b3 §7 N3).
   Opacity is "this atom has no δ-rule," read on Decision E's **engine-layer** `canUnfold` hook; the unfolding-face never builds past the seal.
   Opacity here is **total, not scoped** — the GSACB scoped-unfolding line [L-6] stays the reserved **theory** layer for the evidence sublanguage (ADR-50 Decision E), and B3's sealing is designed not to preempt it.
8. **Kernel/S2 posture: the kernel normalizer sees only instantiated residue** (§6).
   The elaborator instantiates functor applications before export (b3 §6.2 posture (b)), so `HOLE-APP` fires **elaborator-side only**; the kernel (replay/B9) meets `AbstractType` atoms (`HOLE-SEAL`) and `Package` (`HOLE-PACK`) as new S2 kernel vocabulary, never a live functor.
   This mirrors b3 Q1 to the normalizer and is put to the owner as B4-RC6.
9. **The L-machine-specific cost is un-focusing readback** (§4.2; L-machine landing SEAM 1).
   Exact higher-order readback needs **inverting the focus translation** `𝓕` — the largest open seam in the core-sequent port.
   The term-face origin-`NodeId` cache short-circuits the **inert** case; a **reduced** value still needs `𝓕⁻¹`.
   Whether B4 lands full `𝓕⁻¹` or defers it behind the origin-`NodeId` cache plus the existing KIND-granularity fallback is B4-RC9.
10. **Nine ratification-queue candidates** (§9), numbered locally B4-RC1..B4-RC9 (global RQ-n assigned at ratification).
    The A2.5 streaming demo and incremental coupling are scoped to the A2 lane, not here (`gandr-wvd.4` description); the benchmark baseline and the standing perf discipline begin at B4 landing (the `gandr-wvd.4` exit).

---

## 2. Ground truth — the substrate B4 inherits

### 2.1 What ADR-50 fixed, and what B4 executes

ADR-50 is an implementation-architecture record; it binds the Rust interpreter, not the spec (ADR-50 "What this does NOT do").
Its five decisions are the B4 substrate:

| Decision                                                                  | Content                                                                                                                                                        | What it means for B4                                                                                |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **B** — arena-of-`NodeId`, content-address as a third identity discipline | children as `u32` indices, per-face interners, **no canonicalization under β**, ptr-eq/id-eq first                                                             | the hash-consing boundary (§4.3); id-equality is the first def-eq test (§4.5)                       |
| **C** — CEK machine with first-order closures                             | `(Env, NodeId)` closures, **never host closures**, `K` reified                                                                                                 | binder-body re-entry is `(Env, NodeId)`; the machine is serializable/checkpointable (§4.2)          |
| **D** — glued NbE lands now                                               | one shared value domain, two drivers (dynamics owns `K`; a normalize/quote pair drives pure normalization), values glued from day one, quote iterative         | the normalizer IS the NbE half; glued must be the **two faces** of §4.1                             |
| **E** — two-layer unfolding control                                       | **engine** layer (Lean heights + transparency, no theory footprint) + **surface/theory** layer (GSACB [L-6] scoped unfolding, reserved for the evidence layer) | the unfolding discipline (§4.4); sealing lands in the engine layer, scoped unfolding stays reserved |
| **F** — binding seams                                                     | ADR-49 D5 multi-value; marks/identities survive interning; checkpoint conversion slot; per-face oracle tables                                                  | conversion identities must survive interning (§4.3); no single-result bake-in                       |

The machine caveat governs the whole study: ADR-50 and the impl-models body name a **CEK** machine, but Owner caveat (c) (impl-models) and `fcw.9` retarget the cribs to the **L machine** — the CEK retires at L3 once the L differential is green.
Read every "CEK" below as the L machine; the dynamics driver is `machine.rs`, and the pure normalize/quote pair is the NbE half over the same shared value domain.

### 2.2 The L machine, as landed

The L-machine landing (core-sequent port) supplies the concrete substrate the normalizer plugs into:

* **`il.rs`** — three node families over the ADR-50 arena; the focused intermediate language, effect surface already present.
* **`focus.rs`** — the `𝓕` translation (source `Comp`/`Value` → focused IL).
* **`machine.rs`** — the L machine proper (the reduction engine).
* **`store.rs`** — the L1 heap-everything store (the arena's runtime heap).
* **`differential.rs`** — the agree/canonical comparison against the CEK oracle (`eval.rs`, ported for the differential, deleted last).

The load-bearing L-machine fact for readback: **exact higher-order comparison needs inverting `𝓕`.** The focused IL "no longer retains the thunk's source body," so `dispatch_native` declines higher-order combinators (`native_needs_unfocus`) and the differential compares returned thunks/functions/lazy-pairs at **KIND granularity only** — because exact structural readback needs `𝓕⁻¹` (L-machine landing SEAM 1, the largest seam).
This is precisely where the term-face glue (§4.1) earns its keep.

### 2.3 The B2.1 conversion seed — what B4 grows onto

At S1 the kernel's conversion is **type-only** and vacuously C5-quarantined (B2.1 stage record; C5 = the kernel-boundary "conversion never evaluates effectful computations" quarantine, B2 staging call):

* No S1 value-type or computation-type former is indexed by a value term (product is non-dependent; the arrow codomain cannot mention its argument; universe/lift embed only a `Level`).
* So conversion **coincides exactly with structural equality**, with canonical-`Level` equality at `Universe`/`Lift` nodes (the strata `Level`'s derived `Eq` is the ADR-78 oracle).
* **No beta law fires and no computation is evaluated** — the C5 quarantine holds vacuously because conversion never descends into a term.
* The value/computation term alpha-equality (`convertible_values`/`convertible_computations`) is **implemented, quarantined, and not invoked** — "the seed the term-indexed extensions (El/description codes, S2+) will grow beta onto."
* **Every conversion face is iterative over a heap worklist** — it computes the same relation as the derived `PartialEq` but the derived version recurses on depth and would overflow; the worklist keeps it total at any depth.

**The B4 charter, restated against the seed:** B4 grows the quarantined alpha-equality into a **conversion-with-β** by adding the whnf/definitional-unfolding steps (§4.5) — but the iterative-worklist mandate is non-negotiable: the recursion dylint (`cargo:dylint:recursion`, a `gate:merge` task) gates merges, and the recursive-reference checker (H-A in the B2.1 record) is a depth-budgeted stopgap owing a defunctionalized successor.
Every normalizer/conversion face B4 adds is iterative from birth.

### 2.4 The representation — D1(C) arena

D1(C) (massive-term §3.1, owner-veto RQ-1) is the representation the normalizer's values live in: a per-environment append-only arena, four typed id families (`ValueId`/`ComputationId`/`ValueTypeId`/`CompTypeId`) preserving the polarity discipline statically, and an **id-equality fast path** (same id in the same arena ⇒ same node ⇒ structural equality — the trivially-sound Lean `is_eqp`-first posture, no table in the TCB).
Two facts drive the normalizer design:

* **Id-equality is ADR-50 Decision B's "ptr-eq first" made concrete** — it is the first def-eq test (§4.5), and it is free.
* **A `NodeId` is the term-face pointer** — so term-face gluing (§4.1) degenerates to caching the origin id on each value, and the arena's structural sharing IS the "hash-consing" the title names, preserved by decode rather than created under β (§4.3).
* The admission-watermark discipline (D1(C)) gives the normalizer its scratch story: machine-local intermediates (including the unfolding-face's built-up unfolded forms) allocate past the watermark and are truncated after the verdict, so the persistent arena holds only admitted content.

---

## 3. Binding constraints and design lineage

### 3.1 Owner-decided / ratified constraints (not relitigated here)

1. **B3 precedes B4** so the normalizer is designed against functor application + module projection (`bd show gandr-wvd`, constraint 4; b3 §3.1).
   This study's response is the hole parameterization (§5).
2. **ADR-50 binds the interpreter, not the spec** — B4 changes no typing/evaluation rule; it executes Decisions C/D/E over the frozen core.
3. **No global hash-consing under β; ptr-eq/id-eq first; machine-local and boundary sharing only** (ADR-50 Decision B; the `gandr-wvd.4` charter constraint).
4. **The iterative-worklist mandate** (B2.1 stage record; `docs/workflow/rust.md`; the recursion dylint gate).
5. **B4's exit** (`gandr-wvd.4`): the normalizer is the checker's conversion engine; a benchmark baseline is recorded; the standing perf discipline starts.
   The A2.5 streaming demo is targeted alongside; incremental-coupled residuals live in the A2 lane, not here.

### 3.2 Lineage the design builds on

* **impl-models §5.2** — the glued-representation split correction (the load-bearing consumer input for `gandr-wvd.4`, flagged LOAD-BEARING in the bead's STANDING note): term-face vs unfolding-face; the half-built-glue failure mode.
* **impl-models §2.1-2.4** — the Lean engine recipes: cached-word guards, `ptrEq` first, `ShareCommon` as a boundary pass, lazy-δ with heights, the 11-rule elaborator policy, transparency lattice, smart unfolding.
* **impl-models §4 (smalltt)** — the force/quote/`ConvState` blueprint: two small force enums, `QuoteOption`, the Rigid/Flex/Full three-state conversion with glued retry.
* **impl-models §3.1 (Agda `Reduce.Fast`)** — the fully first-order machine precedent; values-carry-their-blocker; compact per-run def views; two-tier fallback.
* **impl-models §1 (Idris 2)** — the whole-system model and its negative precedents (un-memoized closures; crude unfolding control).
* **ADR-50 D/E** — the two-driver-one-domain architecture and the two-layer unfolding control; the theory layer cites [L-6].
* **b3 §7** (N1-N6), **§6.2/§6.3** (kernel posture; R1-R4 reservations), **§8** (the B6 telescope door) — the module forms and the kernel handshake this study duals into holes.
* **Literature:** Levy [A-1a], [A-2] for the CBPV polarity substrate; Abel & Sattler [A-37] for NbE over CBPV / the polarized calculus (the normalizer's metatheory reference); Gratzer–Sterling–Angiuli–Coquand–Birkedal [L-6] for the reserved controlled-unfolding theory layer.

---

## 4. The design core

### 4.1 Glued values — the two-face split, made concrete

**The single most important decision in this study.** impl-models §5.2 corrects ADR-50 Decision D: "glued from day one" names two different mechanisms, and building one while believing the other exists is a named failure mode.
This study coins that mode the **half-built-glue trap** and makes the corrective a landing invariant.

**Face (a) — term-face gluing (adopt now, nearly free).** A value caches the arena `NodeId` of the source term it came from — "a value remembers the term it came from" (impl-models §5.2(a)).
Under the D1(C) arena a `NodeId` IS the term-face pointer, so this degenerates to a single extra id field on the value.
On the L machine this is the **inert-case short-circuit for readback**: because the focused IL discards the source body (§2.2, SEAM 1), un-focusing readback normally needs `𝓕⁻¹`; but a value that has **not** been reduced still equals its origin term, so readback returns the cached origin `NodeId` directly and never inverts the focus translation.
This is the elaborator-facing half in the digest's framing (it kills quote traffic); on the L machine it additionally buys the cheapest slice of the SEAM-1 mitigation.

**Face (b) — unfolding-face gluing (the load-bearing half; design jointly with the hints table).** smalltt's per-unfolding-node representation (impl-models §4.1, source-verified): `VUnfold head spine ~unfolded` keeps the **neutral** form (head + spine) **and** a lazy fully-applied unfolded value, built incrementally as the spine grows.
This is the conversion/readback-facing half — it kills size blowup under top-level unfolding, and it is the one that interacts with Decision E's unfolding control: **readback chooses the face** (via a `QuoteOption`-style enum, §4.4) and **conversion forces the unfolded face on demand** (via the force/forceAll split).

**The invariant (B4-RC1).** The value domain names **both** faces in its type before either is built, and the "which face does quote choose" policy and the "which side does def-eq unfold" policy are **the same table** (impl-models §5.2 recommendation; smalltt shows the whole table is two small enums plus a three-state conversion discipline).
Face (a) is trivial and lands first; face (b) is designed together with the engine unfolding hints (§4.4) so the trap cannot open.

### 4.2 Environments vs closures on the L machine

* **Closures are first-order `(Env, NodeId)` pairs** (ADR-50 Decision C) — never host-language closures.
  The binder-body-under-NbE is `(Env, NodeId)` re-entry; the machine stays serializable, checkpointable, and inspectable (ADR-9's reified-machine thesis). smalltt and Agda `Reduce.Fast` are the fully-first-order precedents; Idris 2 is only half first-order (its `NBind` body is a host function) and gandr's discipline is deliberately **stricter than its own stated model** (impl-models §1.1).
* **Thunk cells must be explicitly memoized** (impl-models §5.3, §5.6 #10; §4.4 "no memoization on `Closure` forcing" is the Idris 2 negative precedent). smalltt's call-by-need rides GHC laziness (`~Val`), unavailable in Rust; Agda's `Pure`-vs-`Pointer` split is the explicit design — decide, per closure, which need sharing at all.
  On the L machine the heap is `store.rs`; the normalizer needs update-able thunk cells (or the arena equivalent) from day one, not Idris 2's re-`eval`-each-time.
* **Normalization = drive to (weak-head) focused form, then un-focus.** The NbE normalize half uses the L machine's reduction to reach whnf over the shared value domain; the quote half is the un-focusing readback (`𝓕⁻¹`, §2.2).
  Quote is **iterative** — Agda's `ArgK`/spine-zipper readback is the worked continuation-driven (ADR-47-compliant) example (impl-models §5.2 "Quote"), and Idris 2's `sizeLimit` fuel + `clearDefs`-style zero-unfold readback become explicit `QuoteOpts` knobs.
* **Stuckness as data (B4-RC7).** Agda's `IsValue = Value Blocked_` records **why** a WHNF value cannot reduce (which meta/var it is stuck on); the L machine already has a `StuckReason` vocabulary (`UnsupportedByReference`, `ForcedNonThunk`, `PerformNoHandler`).
  Values should carry blocker identity so the future conversion checker/solver's worklist has exact wake-up conditions (impl-models §5.1 #4, §5.6 #3), matching the existing frame discipline.

### 4.3 The hash-consing boundary — sharing preserved, not created

The title's "hash-consing" must be read through ADR-50 Decision B: **no global hash-consing under β.** The reconciliation is a boundary, not a table in the evaluator:

* **Under β, sharing is PRESERVED, never CREATED** (massive-term §3.2, the C3 discipline).
  The normalizer/kernel only _retains_ the sharing the decoder hands it; any sharing-*creation* pass is elaborator-side (`core-checker`'s `intern.rs`), never the normalizer.
* **Interning is for static, β-free data, per-face** (ADR-50 Decision B; impl-models §5.1 #1, confirmed three ways — Idris 2 has none and it hurts; Agda deleted `--sharing` for a per-reduction local heap; Lean interns only at import/serialization boundaries and per-subsystem).
  The differential faces (checker vs machine) never share a canonicalizing table; cross-face comparison stays structural, cached-hash-accelerated (per-face oracle tables, ADR-50 Decisions B/F).
* **The unfolding-face's built-up unfolded values are machine-local scratch.** When `VUnfold` grows its unfolded face under β (§4.1(b)), those new nodes allocate **past the D1(C) admission watermark** and are truncated after the verdict — they are never interned into the persistent arena, because interning them would be exactly the canonicalization-under-β that Decision B forbids.
* **Id-equality is the sound fast path with no table in the TCB** (massive-term §3.1/§3.2): same id in the same arena ⇒ structural equality, by immutability; it is an _added_ early-out, not the derived `PartialEq` (which stays structural).
  This is the B4 realization of "ptr-eq first."
* **Identities that conversion produces must survive interning** (ADR-50 Decision F; the `wyrd-h8og` seam): marks and any future certificates reference identities that outlive the intern boundary — relevant once sealing mints atoms (§5, `HOLE-SEAL`) that the export re-mints deterministically (b3 §6.3 R4).

### 4.4 The unfolding discipline — whnf, spine-local, per-scope δ

* **whnf is spine-local** (b3 §7 N1; [A-37] for NbE over the polarized core).
  A projection or application drives its **head** to a value form and no further — never the siblings.
  The glued representation's laziness is load-bearing here: the unfolded face is built only along the spine actually forced.
* **Force/quote are two small enums** (impl-models §4.2; smalltt): three force modes (force solved-meta heads only / chase meta-unfoldings / eliminate all unfoldings from the head) and a `QuoteOption` (UnfoldAll / UnfoldMetas / UnfoldNone).
  Values never lose the neutral face; forcing chooses which face to _look at_.
  This is the concrete mechanism behind "the quote-face policy and the def-eq-unfold policy are the same table" (§4.1).
* **The engine unfolding layer is Lean-style** (ADR-50 Decision E engine layer; impl-models §2.2-2.4): reducibility hints `Opaque | Abbreviation | Regular(height)` with height = definitional depth; a **transparency lattice** exposed as a `canUnfold` hook on the driver's `evalRef` analogue; whnf/infer memo caches keyed by identity; arithmetic/literal fast paths _inside_ the conversion loop. gandr has no annotation culture yet (impl-models Outlook, AGAINST): height-from-definition-DAG-depth is mechanical, but the **default transparency policy needs an owner decision** (B4-RC8).
* **The δ-environment is per-scope** (b3 §7 N4, `HOLE-VIEW`).
  Transparent ascription contributes definitional equations (δ-rules); the same atom may be **manifest** in one scope (inside the sealed module) and **opaque** outside, so the definitional environment must be per-scope, not a single global table.
* **The theory layer is reserved** (ADR-50 Decision E surface/theory layer; [L-6]).
  Declared, scoped unfolding (GSACB, cooltt, Agda `opaque`/`unfolding`) elaborating to extension types with a normalization theorem is the evidence-sublanguage's territory and lands with the evidence layer — B3's sealing (total opacity) is designed not to preempt it (b3 §7 N3).

### 4.5 The def-eq pipeline — id-equality first, growing β onto the S1 seed

The conversion engine is the S1 seed (§2.3) extended with whnf/unfolding, iterative throughout.
The pipeline, in order (each step falls through to the next only on a non-answer):

1. **id-equality** — arena id-eq / `Rc::ptr_eq`; O(1), sound by immutability (ADR-50 Decision B; massive-term §3.2; Lean `is_eqp` first, impl-models §2.1).
2. **cached-word guards** — Lean's intrusive `u64` (hash + `hasMeta`/`hasFVar` bits + loose-var-range + approx-depth); O(1) "can I skip this traversal" (impl-models §2.1, §5.3 #yg03; D1(C) gives a natural per-node slot, massive-term §3.1).
3. **iterative structural comparison** — the B2.1 seed relation (`convertible_values`/`convertible_computations`) over a heap worklist; `quickConv`-style head-mismatch fast-fail before recursing into arguments (impl-models §1.3).
4. **lazy-δ with heights** — same head, `Regular` hints ⇒ try **args-first with a failure cache**; else unfold the **taller** side; only-one-side-δ ⇒ unfold it (impl-models §2.2); the elaborator-facing extension adds the projection-preference / reducible-preference / head-symbol-matching rules with no kernel counterpart (impl-models §2.3, the 11-rule policy) when the future solver arrives.
5. **smart-unfolding gated on case-progress (B4-RC5)** — a recursive definition unfolds **only if its recursive scrutinee makes progress** (impl-models §2.4, §5.2).
   This is the missing piece neither Idris 2 nor Agda has in this form; gandr implements it directly on **case-tree progress** (case is first-class in the core, so no `_sunfold` companion is needed), and it couples to the L2 unroll-freeze lane.
6. **`ConvState` speculation** — Rigid / Flex / Full (impl-models §4.3; smalltt): same-head `VUnfold`s compare spine-first in **Flex** (no commitments), backtracking to **Full** on the **already-forced** glued face — no re-evaluation.
   This is Lean's args-first heuristic as an explicit three-state discipline, the failure cache replaced by cheap exceptions plus glued retry.

**Growth from the seed.** At S1 only steps 1-3 fire (type-only, structural, no β).
Steps 4-6 are the **term-indexed extension** (S2+): they add β to the quarantined alpha-equality.
The B3 module forms (§5) enter this pipeline as the holes' redexes; until a hole is plugged, its head is neutral and steps 4-6 simply do not fire on it.

---

## 5. B3-hole parameterization — the six module-form holes

B3 §7 names five redex/discipline classes (N1-N6) B3 exports to B4.
This study duals each into a **normalizer hole**: a named interface the B4 normalizer is parameterized over, with (i) the **interface contract** — what the normalizer requires from B3, (ii) the **neutral-until-plugged** default — how the normalizer behaves before the hole is filled, and (iii) the **B3 rung that plugs it**.
The mechanism honors B3-before-B4: B4's skeleton is built with all six holes as neutrals (the normalizer already handles neutral heads by construction, §4.4 whnf), and each hole is filled additively as its rung lands, so **B4 proper starts immediately after B3.0** rather than after the whole module system.

| Hole            | Carries (b3 §7)                       | Interface contract (what B4 requires from B3)                                                                                                                                     | Neutral-until-plugged                                                       | Plugged by                               |
| --------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------------- |
| **HOLE-PROJ**   | N1 structure projection               | `project(struct_value, ℓ) → component`; a projection whose head is a neutral **path** stays neutral; **spine-local** whnf only (head→structure, never siblings)                   | projection head stays neutral                                               | B3.1 (structures/paths)                  |
| **HOLE-APP**    | N2 functor β under generativity       | `apply(force F, M) → (body[M/X], minted_atoms)`; **stateful**, keyed by minted atoms; **never memoize/hash-cons across instantiations**; value identity **includes minted atoms** | application head stays neutral                                              | B3.3 (generative functors)               |
| **HOLE-SEAL**   | N3 sealing as unfolding barrier       | `is_opaque(atom) → bool` on the engine `canUnfold` hook; a sealed component has **no δ-rule**; the unfolded face **stops at the seal**                                            | sealed atom is a neutral with no δ-rule (already the whnf default)          | B3.2 (sealing + abstract atoms)          |
| **HOLE-VIEW**   | N4 transparent ascription strengthens | `type ℓ = A` contributes a δ-rule to a **per-scope** δ-environment; strengthening re-adds equations to a sealed-then-transparently-viewed module                                  | manifest type is opaque outside its scope                                   | B3.1 (ascription) + B3.2 (strengthening) |
| **HOLE-PACK**   | N5 `unpack∘pack`                      | reduces **generatively** (fresh atoms per unpack — the `HOLE-APP` discipline); `Package` values otherwise **inert** for conversion; conversion **never** runs init effects (C5)   | `Package` value is inert; `unpack` head stays neutral                       | B3.4 (`Package` + `pack`/`unpack`)       |
| **HOLE-COERCE** | N6 coercions are terms                | a matching coercion **normalizes like any other structure expression**; the normalizer **never** compares signatures, never needs permutation/width equations                     | (negative hole — nothing special; a coercion is an ordinary structure expr) | B3.1 (coercive matching)                 |

The two headline holes — the stated reason B3 precedes B4 (`bd show gandr-wvd`, constraint 4) — warrant detail.

### 5.1 HOLE-APP — functor-application unfolding (the sharpest constraint)

Generative functor application is **not a confluent pure rewrite** (b3 §7 N2; b3 §4.5): `(force F)(M) ▷ body[M/X]` **plus atom minting** for the sealed result, and two applications of the same functor to the same argument are **not convertible**.
The normalizer must therefore:

* treat instantiation as a **stateful** step keyed by the minted atoms, and
* **never memoize or hash-cons across instantiations** — the content-addressed sharing of ADR-50 Decision B must **include the minted atoms in identity**, or two distinct instantiations would silently alias.

This study coins that failure mode the **generative-aliasing hazard**.
It is the single point where the hash-consing boundary (§4.3) and generativity collide, and it is the sharpest constraint B3 exports (b3 §7 N2 "should appear in B4's charter checklist verbatim").
The mitigation is **B4-RC3**: bake "minted atoms are part of value identity" into the content-address discipline, and make freshness a _checkable_ property against the R4 minted-atom table (b3 §6.3 R4 — the export re-mints deterministically, so aliasing is caught by replay, not trusted from the elaborator).

Interface note: because the elaborator instantiates functor applications before export (§6), `HOLE-APP` fires in the **elaborator-side / S2 conversion** normalizer, **not** the kernel normalizer — the kernel meets only the already-instantiated residue (the minted atoms + member `Def`s).
So `HOLE-APP` is neutral in the kernel normalizer permanently at B3, and stateful only in the elaborator-side normalizer.

### 5.2 HOLE-PROJ — module-projection unfolding

`struct { …, ℓ = v, … }.ℓ ▷ v` (b3 §7 N1); a projection whose head is a **neutral path** stays neutral.
The contract is **spine-local whnf**: normalize the head to structure form, never the sibling components — the glued representation's laziness (§4.1(b)) is what makes this cheap, since the unfolded face is built only for the projected label.
Paths give the normalizer neutral heads and a spine-local whnf story (b3 §4.2, §7 N1); value-component projection from a path is pure and static.

### 5.3 The remaining four holes

* **HOLE-SEAL** is the **first genuine language-level unfolding barrier** the glued-NbE normalizer meets (b3 §7 N3).
  Opacity is "this atom has no δ-rule," a fact the kernel **re-derives rather than trusts** (b3 §4.4, the K2 discipline); it reads on the engine-layer `canUnfold` hook (§4.4), and the unfolded face never builds past it.
  Total, not scoped (the [L-6] theory layer stays reserved, §4.4).
* **HOLE-VIEW** forces the δ-environment to be **per-scope** (§4.4): the same atom is manifest inside a sealed module and opaque outside.
  This is the one hole that constrains the _shape_ of the normalizer's definitional environment rather than adding a redex.
* **HOLE-PACK** reduces generatively (the `HOLE-APP` discipline for fresh atoms per `unpack`); `Package` is otherwise inert for conversion, and conversion never runs initialization effects (C5 stands unmodified, b3 §7 N5).
  `Package σ` is a frozen-core positive value former added at B3.4 through the `core-ir-contract.md` §0 discipline (b3 §4.6).
* **HOLE-COERCE** is a **negative** hole: because matching is coercive (b3 §4.3), the normalizer never compares signatures and never needs permutation/width equations in conversion.
  This is what keeps the def-eq pipeline (§4.5) signature-free, and it is also the B6-telescope-door invariant I3 (b3 §8) — so the normalizer must **not** grow signature comparison even opportunistically.

---

## 6. Kernel / S2 interplay posture

The kernel handshake (b3 §6.2/§6.3) determines _which_ normalizer meets _which_ hole.

* **The elaborator flattens; the kernel checks the residue** (b3 §6.2 posture (b), recommended).
  Structures export as member `Def`s with structured (path-segment) names (R2) plus signature metadata; **functor applications are instantiated by the elaborator before export** — each generative instantiation exports its minted atoms + member `Def`s.
  The kernel's new obligations are only an `AbstractType` declaration form (a minted atom with its arity, **no δ-rule**) and, at B3.4, the `Package` former with `pack`/`unpack` typing.
* **Consequence for the normalizer (B4-RC6): the kernel normalizer sees only instantiated residue.** Functor _bodies_ are not independently kernel-normalized at B3 — only their instantiations (b3 §6.2 honest trade; b3 Q1).
  So in the **kernel** (replay/B9) normalizer, `HOLE-APP` is permanently neutral; the kernel meets `AbstractType` atoms (`HOLE-SEAL`) and `Package` (`HOLE-PACK`) as new S2 kernel vocabulary, never a live functor application.
  The **elaborator-side / S2** normalizer is where `HOLE-APP` and `HOLE-PROJ` fire.
* **This is the S1→S2 growth of the kernel subset ladder** (S0-S3; `spec/kernel-boundary.md`).
  At S1 conversion is type-only (§2.3); the module forms are term-indexed and enter kernel conversion at **S2** (the El/description-code stage the B2.1 seed anticipates).
  `AbstractType`/`Package` are kernel-vocabulary growth under the standing per-phase subset-growth obligation (fcw.11), and the R1 reserved declaration-kind tags (b3 §6.3, ratified 2026-07-20) keep the format additive.
* **The kernel takes no interning table into the TCB** (§4.3; massive-term §3.2 C3).
  Any sharing the kernel normalizer sees is decode-preserved; the id-equality fast path is the only "sharing-aware" step, and it is trivially sound.
* **What the kernel normalizer must NOT do** (b3 §8 anti-commitments, dualized): never compare signatures (keep `HOLE-COERCE` negative); never memoize across functor instantiations (the generative-aliasing hazard); never make `Package` eliminable by anything but `unpack`; never bake width/permutation equations into conversion.

---

## 7. Hazards

* **H1 — the half-built-glue trap** (§4.1; impl-models §5.2, Outlook AGAINST).
  Building the term-face while believing the unfolding-face exists (or vice versa).
  Mitigation: name both faces in the value-domain type before either is built (B4-RC1); design face (b) jointly with the engine hints table.
* **H2 — the generative-aliasing hazard** (§5.1; b3 §7 N2).
  Content-addressed sharing that does not include minted atoms silently aliases two functor instantiations.
  Mitigation: minted-atoms-in-identity (B4-RC3), checkable against the R4 minted-atom table.
  This is the sharpest and most soundness-relevant hazard.
* **H3 — un-focusing readback is the largest seam** (§2.2; L-machine landing SEAM 1).
  Exact higher-order comparison needs `𝓕⁻¹`; the term-face cache only covers the inert case.
  Mitigation/decision: B4-RC9 (land origin-`NodeId` term-face now; scope full `𝓕⁻¹` as a B4 sub-rung or defer behind the KIND-granularity fallback).
* **H4 — explicit thunk memoization** (§4.2; impl-models §5.6 #10).
  No host laziness in Rust; un-memoized closures are the Idris 2 performance cliff.
  Mitigation: explicit update-able thunk cells (Agda `Pure`-vs-`Pointer` split) from day one; a cost smalltt's benchmarks silently exclude.
* **H5 — iterative-or-die** (§2.3; the recursion dylint gate).
  Any recursive conversion/normalization/readback face fails the merge gate and can overflow on adversarial depth.
  Mitigation: worklist/defunctionalized from birth; quote via the Agda `ArgK` spine-zipper shape; the depth-budgeted recursive checker (H-A, B2.1 record) is a stopgap owing a defunctionalized successor.
* **H6 — no annotation culture for transparency defaults** (§4.4; impl-models Outlook).
  Height-from-DAG-depth is mechanical, but "what is reducible by default" is a policy gap.
  Mitigation: B4-RC8 (owner policy call); sensible defaults + the `canUnfold` hook.
* **H7 — the two-driver-one-domain composition is gandr's own bet** (impl-models Outlook AGAINST).
  No surveyed system runs effects + first-class `K` + conversion over one value domain; the precedents de-risk the _parts_, not the composition (smalltt has no `K`, no data/case, no erasure).
  Mitigation: the Agda-style in-loop fallback to the reference evaluator (impl-models §3.1, §5.1 #3) grows coverage incrementally and keeps the ADR-48 oracle honest; the `jit ≡ eval` differential (ADR-51) is the verification net.
* **H8 — per-scope δ-environment complexity** (§4.4; `HOLE-VIEW`).
  A global definitional table is wrong once transparent ascription and sealing coexist.
  Mitigation: scope the δ-environment from the start, even before `HOLE-VIEW` is plugged (the empty per-scope environment degenerates to the S1 seed).

---

## 8. Decision map

| #       | Decision                           | Recommendation                                                                                             | Alternative recorded                                                                        | Owner posture          |
| ------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ---------------------- |
| D-glue  | Glued value representation         | Two named faces; term-face (origin-`NodeId`) now, unfolding-face (`VUnfold`) designed with the hints table | single conflated "glued" (the ADR-50 D phrasing) — rejected, it is the half-built-glue trap | B4-RC1                 |
| D-pipe  | def-eq pipeline order              | id-eq → cached-word → iterative structural → lazy-δ/heights → smart-unfolding → `ConvState`                | Idris-style eval-both-sides, no caches — rejected (the performance cliff)                   | B4-RC2                 |
| D-ident | Content-address under generativity | minted atoms are part of value identity; freshness checkable vs R4 table                                   | atoms outside identity — rejected (the generative-aliasing hazard)                          | B4-RC3 (soundness)     |
| D-holes | Module-form parameterization       | build the skeleton with six neutral holes; fill per B3 rung                                                | wait for the whole B3 module system before B4 — rejected (defeats B3-before-B4)             | B4-RC4                 |
| D-recur | Recursive-def unfolding            | smart-unfolding on case-tree progress (gandr mechanism)                                                    | unconditional δ — rejected (stuck-`brecOn` disaster)                                        | B4-RC5                 |
| D-kern  | Kernel normalizer scope            | instantiated residue only; `HOLE-APP` elaborator-side                                                      | kernel-check functor bodies at B3 — deferred (TCB + replay cost)                            | B4-RC6 (mirrors b3 Q1) |
| D-stuck | Stuck-value shape                  | values carry blocker identity                                                                              | stuck-as-outcome only (current)                                                             | B4-RC7                 |
| D-trans | Transparency defaults              | (open)                                                                                                     | height-only, all-reducible-by-default                                                       | B4-RC8 (open)          |
| D-read  | Un-focusing readback timing        | origin-`NodeId` now; scope `𝓕⁻¹` as a sub-rung                                                             | full `𝓕⁻¹` at B4 landing                                                                    | B4-RC9 (open)          |

---

## 9. Ratification-queue candidates

Numbered locally **B4-RC1..B4-RC9** to avoid collision with the shared RQ-n namespace (RQ-n belongs to massive-term §12, and a sibling lane is extending it).
**Global RQ numbers are assigned at ratification.** Each: the decision, the recommendation, and the alternative.

* **B4-RC1 — the two-face glued split (adopt).** Name term-face and unfolding-face in the value-domain type; implement term-face (origin-`NodeId`) immediately, design unfolding-face (`VUnfold`) jointly with the engine hints table so quote-face and def-eq-unfold policies are one table.
  Alternative: the single conflated "glued" of ADR-50 D — rejected as the half-built-glue trap (impl-models §5.2).
* **B4-RC2 — the def-eq pipeline, id-equality first (adopt the Lean recipe set with citations).** id-eq → cached-word guards → iterative structural → lazy-δ/heights (taller-unfolds, args-first + failure cache) → smart-unfolding → `ConvState` speculation (impl-models §2.2-2.4, §4.3).
  Alternative: eval-both-sides with no caches (Idris 2) — rejected.
* **B4-RC3 — minted-atoms-in-identity for the content-address discipline (adopt; soundness).** Functor/`unpack` minted atoms are part of value identity; freshness is a checkable property against the R4 minted-atom table.
  Alternative: atoms outside identity — rejected as the generative-aliasing hazard (b3 §7 N2).
* **B4-RC4 — the six-hole parameterization (adopt; the B3-before-B4 mechanism).** Build the B4 skeleton with `HOLE-PROJ/-APP/-SEAL/-VIEW/-PACK/-COERCE` as neutrals; fill each as its B3 rung (B3.1-B3.4) lands.
  Alternative: block B4 on the full module system — rejected.
* **B4-RC5 — smart-unfolding on case-tree progress (adopt).** A recursive definition unfolds only if its scrutinee makes progress, implemented on gandr's first-class case (no `_sunfold` companion); couples to the L2 unroll-freeze lane.
  Alternative: unconditional δ — rejected (impl-models §2.4).
* **B4-RC6 — kernel normalizer sees instantiated residue only (owner call; mirrors b3 Q1).** `HOLE-APP` fires elaborator-side only; the kernel meets `AbstractType`/`Package` as S2 vocabulary, never a live functor.
  Alternative: kernel-certify functor bodies at B3 (b3 §6.2 (a)) — the TCB-size/replay-cost trade R1's reserved tags keep open.
* **B4-RC7 — stuck values carry blocker identity (adopt).** Spec the stuck-value shape (Agda `IsValue`) before the conversion checker; reuse the L machine's `StuckReason` vocabulary (impl-models §5.6 #3).
  Alternative: stuck-as-outcome only — deferrable but cheaper to lay now.
* **B4-RC8 — default transparency policy (OPEN owner call).** Height-from-DAG-depth is mechanical; the default reducibility set is a policy gap (no annotation culture, impl-models Outlook).
  Options span all-reducible-by-default to an `@[irreducible]`-style opt-out; needs an owner decision before the engine layer lands.
* **B4-RC9 — un-focusing readback timing (OPEN owner call).** Land origin-`NodeId` term-face now (covers the inert case) and scope full `𝓕⁻¹` (L-machine landing SEAM 1) as a B4 sub-rung, or land full `𝓕⁻¹` at B4 landing.
  Recommendation leans to the staged option — the seam is the largest in the port (~800-1400 LOC estimate) and the term-face cache defers the pressure — but it is a genuine scoping fork for the owner.

---

## 10. References (register keys, locators verbatim from `docs/research/bibliography-v2.md`)

| Key  | Citation                                                                                                         | Locator                                                                         |
| ---- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| A-1a | Levy — _Call-By-Push-Value: A Subsuming Paradigm_ (TLCA 1999)                                                    | doi:10.1007/3-540-48959-2_17                                                    |
| A-2  | Levy — _Call-By-Push-Value_ (PhD thesis, Queen Mary, University of London, 2001)                                 | <https://pblevy.github.io/papers/thesisqmwphd.pdf> (QMRO handle 123456789/4742) |
| A-37 | Abel & Sattler (2019, PPDP) — _Normalization by Evaluation for Call-By-Push-Value and Polarized Lambda Calculus_ | doi:10.1145/3354166.3354168                                                     |
| L-6  | Gratzer, Sterling, Angiuli, Coquand & Birkedal — _Controlling Unfolding in Type Theory_                          | arXiv:2210.05420                                                                |

Absorbed research records and stage records cited by section (not register keys): `impl-models` = `docs/research/impl-models-deep-read.md`; `massive-term` = `docs/research/massive-term-design.md`; `b3` = `docs/research/b3-module-system-design.md`; the B2.1 stage record and B2 staging call = the `gandr-wvd.2` stage comments (2026-07-20 17:10 and 15:58); the L-machine landing = the `gandr-wvd.1` stage records; ADR-50 = `wyrd@failed-refactor:docs/adr/0050-the-interpreter-architecture-arena-of-nodeid-ir-with-content.md`.
