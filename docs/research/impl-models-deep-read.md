# Implementation-model deep read: Idris 2, Lean 4, Agda, and smalltt internals for the performance program

> Absorbed research record (gandr-wgq, 2026-07-20) — the **performance-program primary record**.
> This is a single-session, source-grounded read of four implementations (Idris 2, Lean 4, Agda, smalltt), executed in the wyrd program on 2026-07-06 under bead `wyrd-o4s7` (Idris 2 as implementation model, ADR-50 Decision A; joint with `wyrd-3esy` full-DT feasibility) and extended by owner steer to a three-plus-one-system read + synthesis (smalltt added as the elaboration-performance blueprint).
> It originates in the wyrd program and is absorbed into gandr unchanged in technical substance; the adaptations below are framing-only.
> All claims are grounded in the pinned checkouts (below); no fanout.

## Source checkout of record

The read was pinned to these commits; machine-local checkout paths are elided (contributor-concern) — only `(system, branch, commit)` is project-concern.
Upstream `file:line` anchors throughout the body are repo-relative to the corresponding system's tree at these commits and are kept verbatim as primary evidence.

| System  | Branch | Commit                    |
| ------- | ------ | ------------------------- |
| Idris 2 | main   | `fd405085b` (2026-06-22)  |
| Lean 4  | master | `d1f105109b` (2026-07-06) |
| Agda    | master | `6e4d6e9543` (2026-07-01) |
| smalltt | main   | `ea99b0f` (2026-02-02)    |

## Adaptation notes (this absorption)

The digest's technical body (§§1–5) is carried faithfully and unabridged; the only edits are framing, listed here so the record stays honest:

- **Machine-local paths stripped.** The four per-system checkout paths (in the source's checkout table and the §4 smalltt header) are reduced to `(system, branch, commit)`; the `no-machine-local-paths` prek hook also enforces this.
  Repo-relative `file:line` anchors are untouched.
- **The "Idris 2 primary" / "whole-system model" framing is historical.** The title and §1 lead with Idris 2 as the primary implementation model; read this as the 2026-07-06 framing, superseded per Owner caveat (a) below (the model pivot to Lean).
  The digest's own §5 synthesis already reaches that conclusion.
- **"CEK machine" reads as the L-machine.** The body's machine-level findings target a CEK machine (ADR-50 C). gandr's machine is now the sequent/L-machine; per Owner caveat (c), the machine cribs port to it.
  Inline "CEK" mentions are left verbatim for fidelity — apply caveat (c) globally rather than expecting per-site edits.
- **wyrd-program references retained as provenance.** `wyrd-*` bead ids and `ADR-50/51`/`ADR-77` numbers are wyrd-program historical references (the doc-inventory precedent already cites wyrd bead ids).
  The gandr-ward re-keying of consumers is in the "gandr consumers" section; the body keeps the original wyrd-* consumer keys.

## Owner caveats at absorption (2026-07-19)

Carried verbatim from the doc-inventory row of record; these remain the caveats of record for this document.

- **(a)** The "Idris 2 primary" framing is historical — the model pivot to Lean is implicit in the ADR-77 kernel decision ("let's do a kernel like Lean"; epic `wyrd-tpgh`), and in practice Agda (`Reduce.Fast` machine) + Lean (def-eq/engine recipes) proved the more-used imports, as the digest's own §5 synthesis already concludes.
- **(b)** ADR-50's text still nominally names Idris as interpreter model — when gandr absorbs ADR-50, update that phrasing to match. (This absorbed document _notes_ this; it does not edit any ADR.)
- **(c)** _Adaptation:_ the digest's machine-level findings target a CEK machine; gandr's machine is the L-machine (`fcw.9`: CEK retires at L3 once the L differential is green), so the cribs (first-order closures, blocker-as-data, thunk memoization, compact def views, fallback interop) must be ported to the sequent/L machine — judged doable.

## gandr consumers

Re-keyed gandr-ward from the digest's original wyrd consumer partition (which the body retains as historical references):

- **`gandr-3ln` (massive-term program)** — §§2.1–2.2 and §3.2 are its prior-art spine: the Lean cached-word / pointer-eq / `ShareCommon` / worklist-deallocator recipes and the Agda `--sharing`-deletion lesson.
- **`fcw.9` (the L-machine)** — the §3.1 / §5.3 machine cribs, under Owner caveat (c) (the CEK → sequent/L port).
- **Future S2 conversion design** — the §§2.2–2.4 def-eq recipe set plus §5.2's glued-representation split and engine-unfolding recipe.
- **The B2.4 conversion record** — the conversion program's landing artifact draws on the same §2 / §5.2 recipe set.

The digest's original consumers line (below, historical): `wyrd-zg5r` (glued NbE + engine unfolding), `wyrd-8pu3` (CEK/arena), `wyrd-3esy` (full-DT feasibility), `wyrd-h1z4` (multi-output term face), `wyrd-rd49.2` (pliron proposal), and the ADR-50 role partition.
In the body, §5.2 keys to `wyrd-zg5r`, §5.3 to `wyrd-8pu3`, §5.4 to `wyrd-3esy`, §5.5 to `wyrd-h1z4`/`wyrd-rd49.2` — read them through the gandr re-keying above.

---

**Consumers (original, historical):** `wyrd-zg5r` (glued NbE + engine unfolding), `wyrd-8pu3` (CEK/arena), `wyrd-3esy` (full-DT feasibility), `wyrd-h1z4` (multi-output term face — backend seam notes), `wyrd-rd49.2` (pliron proposal), ADR-50 role partition (binding: Idris 2 = whole-system model; Lean 4 = storage/equality/memo/dealloc + unfolding engine recipes; Agda = vehicle + two impl lessons).

---

## 1. Idris 2 — the whole-system model

### 1.1 Value domain (`Core.Value`) — what "glued" actually means here

- `NF vars` is a spine-form value domain: `NBind | NApp (NHead, spine) | NDCon | NTCon | NAs | NDelayed/NDelay/NForce | NPrimVal | NErased | NType`.
  Spines are `List (FC, Closure)`; heads are `NLocal | NRef | NMeta`.
- `Closure` is **first-order data** (`MkClosure opts localEnv env term` | `MkNFClosure opts env nf`) — _except_ `NBind`'s body, which is a **host-language function** `Defs -> Closure vars -> Core (NF vars)`.
  So Idris 2 is only _half_ first-order. gandr's ADR-50 C ("first-order `(Env, NodeId)` pairs, never host closures in machine state") is **stricter than its own model**; the fully-first-order precedents are Agda's `Reduce.Fast` (§3) and smalltt (§4 — `Closure Env Tm`, `VLam NameIcit Closure`), not Idris 2.
  Design consequence: gandr's binder-body-under-NbE must be `(Env, NodeId)` re-entry, which Idris 2 shows is _workable to avoid only if you accept non-serializable values_ — it accepted, gandr must not (reified-machine thesis).
- **`Glued` (in `Core.Normalise.Eval`) = `MkGlue fromTerm (Core (Term vars)) (Ref Ctxt Defs -> Core (NF vars))`** — a lazy **(syntax ∥ semantics)** pair: the _term_ and a _computation of its NF_, whichever a consumer needs forced on demand (used pervasively for elaborator types: `gnf`, `glueBack`).
  **Correction for ADR-50 D / `wyrd-zg5r`:** this is NOT smalltt's glued representation. smalltt glues **two value-forms** (top-level-unfolded vs local/neutral-preserving) inside the value domain so readback can choose the small form; Idris 2 glues **term ∥ value** so elaboration can avoid quoting (types re-used as terms without readback).
  ADR-50 D's phrase "the smalltt / Idris 2 `Core.Value` shape" conflates these two distinct designs.
  Both are cheap to support over one domain, but they answer different problems:
  - term∥value gluing (Idris) kills _quote_ traffic (elaborator-facing);
  - unfolded∥local gluing (smalltt) kills _size blowup in readback_ under top-level unfolding (conversion-facing).
  Recommendation recorded in §4: gandr's "glued from day one" should name **both** faces explicitly; the conversion-facing one is the load-bearing one for the evidence layer, the term-facing one is nearly free given the arena (a `NodeId` _is_ the term-face pointer — gluing degenerates to caching the origin `NodeId` on the value).
- No memoization on `Closure` forcing: `evalClosure` re-runs `eval` each time (pure call-by-name).
  Scrutinee results are written back into the local env (`updateLocal`, as `MkNFClosure`) — a _point_ fix, not a general one.
  Idris 2's typechecking slowness is folklore-attributed substantially to this; Agda chose call-by-need (§3).
  For gandr: the CEK env should hold **update-able thunks** (or the arena equivalent) — do not copy Idris 2 here.

### 1.2 Evaluator (`Core.Normalise.Eval`) — spine machine, not CEK

- `eval env locs term stack` — a Krivine-style environment machine: `App` pushes a closure on the `Stack` (argument spine), `Lam` pops into the local env, heads that get stuck rebuild `NApp head spine`.
  Strategy is a runtime flag (`EvalOrder = CBV | CBN` in `EvalOpts`).
- **There is no continuation K** — control (case) is handled by recursive `evalTree` returning `CaseResult = Result | NoMatch | GotStuck`, with a `Lazy NF` _neutral fallback_ passed down (`def`): if reduction gets stuck anywhere, the pre-built neutral is returned.
  This "carry the neutral you'd return if stuck" pattern is cheap and elegant — worth copying into gandr's NbE driver (the CEK driver keeps K per ADR-50 C; Idris 2 simply has no effects/control to need it — it confirms the two-driver partition rather than contradicting it).
- Under-application: `argsFromStack` fails → return neutral.
  Over-application: leftover stack re-applied after tree evaluation (`evalWithOpts ... stk'`).
- **Unfolding control is crude** (the negative precedent motivating ADR-50 E): per-call-site `EvalOpts` records — `holesOnly`/`argHolesOnly` (unification-driven partial eval), `tcInline` (totality), `fuel : Maybe Nat`, `reduceLimit : List (Name, Nat)` (per-name budgets for partial evaluation), `removeAs`, `evalAll` (private-name visibility override) — plus per-def gates: visibility (`reducibleInAny`), `alwaysReduce`, `TCInline`, `PartialEval`.
  No transparency lattice, no height heuristic, no def-eq memoization.
  Conversion (§1.3) just evaluates both sides.

### 1.3 Conversion (`Core.Normalise.Convert`) + quote (`Core.Normalise.Quote`)

- `convGen`: NbE conversion — eval both sides to `NF`, compare structurally; spine args forced via `allConv` (all closures evaluated) with one optimization: `quickConv` head-mismatch fast-fail before recursing into arguments.
  **No lazy-δ, no heights, no caching.** Eta is handled by explicit λ-wrap + re-eval.
  `NErased` converts with anything; `Dotted` is transparent.
- **Case-block conversion is a confessed hack**: two elaborator-generated case functions convert if their case trees match structurally with corresponding-variable conversion (`getMatchingVars`), else if they were **defined at the same source location** with convertible scrutinees ("relies on the location being stored accurately — a quick way to find out!").
  Lesson for gandr: keeping `case` first-class in the core IR (as gandr does) dodges an entire class of conversion pain that case-lifting-to-top-level creates under full DT.
- Quote: fresh-name readback (`MN "qv" i` + `Bounds` re-indexing to de Bruijn by unique int), `QuoteOpts { topLevel, patterns, sizeLimit }` — a **fuel-limited quote** (`sizeLimit` throws on exceed) and a patterns-mode partial quote; `quoteWithPi` reads back _only_ the Pi spine with an emptied context (`clearDefs` — quoting against empty defs = readback with zero unfolding).
  The `clearDefs` idiom is the poor-man's unfolding control on the quote side; gandr's NbE driver should expose the same knob as a principled parameter instead.
- Recursive, not iterative — no ADR-47-style discipline; nothing to borrow on that axis.

### 1.4 Context (`Core.Context`) — the arena that already exists

- Global context: `Name` interned to `Resolved Int` → **`IOArray ContextEntry`** — an arena with int handles, exactly the ADR-50 B shape at declaration granularity ("we can only have one context, because name references point at locations in here" — the single-table caveat gandr's per-face interners deliberately avoid).
- Entries are **lazily decoded from binary** (`Coded ns Binary | Decoded GlobalDef`) — module-file (TTC) loading defers deserialization per entry until first lookup.
  Relevant to `wyrd-0iba` checkpoints: flat serialized arena + decode-on-demand is a proven pattern.
- **Speculative elaboration = staging discipline**: `branchDepth : Nat` + `staging : IntMap ContextEntry`; writes at depth > 0 go to staging, `commit` folds them into the array on branch success.
  This is a transactional overlay on an arena — directly reusable for gandr checkpoint/rollback over the `NodeId` arena.
- `GlobalDef` co-locates **all faces of a definition**: type, compile-time case tree + **separate runtime case tree** (`PMDef treeCT treeRT`), erasure metadata (`eraseArgs`, `safeErase : NatSet`), inlining/codegen flags, cached compiled forms (`compexpr : Maybe CDef`, `namedcompexpr`, `schemeExpr`).
  The two-tree + per-face-cache pattern is the in-one-system precedent for gandr's differential faces (checker vs machine) — but note Idris 2 shares one table across them; ADR-48/50's per-face separation is a deliberate strengthening.

### 1.5 QTT erasure → codegen (the 3esy spine)

Pipeline: `Term` (elab, QTT multiplicities on binders) → **`findErased`** (`TTImp.Elab.Utils`: walk the checked type's Pi telescope; `Rig0` positions → `eraseArgs`, collapsibility analysis → `safeErase`) stored on `GlobalDef` at type-processing time → **`Compiler.CompileExpr.toCExp`**: per application of a def/constructor, `numArgs` classifies into `NewTypeBy arity pos` (newtype: collapse to identity on the sole relevant arg — with a `%World` caveat forcing a let-bind to preserve effect ordering) / `EraseArgs arity epos` (positional drop via `dropPos`; case alts drop bound erased args via `mkDropSubst` + `shrinkCExp` thinning) / `Arity` (eta-expand to saturation).
Erased _terms_ residualize to `CErased` (unit-like), `Rig0` lets are shrunk away when unused, `WorldVal` matches erased.

Backend ladder after erasure (all IRs in `Core.CompileExpr` / `Compiler.*`):

```text
Term --(treeRT + eraseArgs)--> CExp (scoped de Bruijn, higher-order, saturated cons, explicit Force/Delay, CErased, CCrash)
  --> opts: Inline (on NamedCExp: "explicit names, faster but less safe"), CSE, ConstantFold, Identity, CaseOpts, ToplevelConstants
  --> Lifted (lambda-lifted, still scoped) --> ANF (Int locals; every argument a variable; AUnderApp explicit)
  --> VMCode (registers RVal/Loc/Discard; MKCON/MKCLOSURE/APPLY/CALL(tailpos)/CASE/PROJECT)
```

- VMCode's header comment is the backend contract in one line: "as long as you have a representation of closures, and an 'apply' function which adds an argument and evaluates if it's fully applied, you can translate this directly to a target language."
  That is the minimum runtime gandr's Cranelift JIT needs for the _open-term-free_ fragment plus closures.
- **Everything is single-result** (`RVal`); the ladder never grows a multi-value seam.
  ADR-49 D5 is a genuine departure with no Idris 2 precedent — the `h1z4` design pass gets no help here (Lean's IR is the nearer cousin but also single-result; see §2).
- The inliner deliberately abandons intrinsic scoping (NamedCExp: "explicit names, which are faster (but less safe) to manipulate in the inliner") — a data point on intrinsically-scoped IR ergonomics under heavy transformation; gandr's arena + u32 ids sidestep the same pain differently.
- **Fast normalization by compilation exists in-tree**: `Core.SchemeEval` — compile definitions once to Chez closures (memoized per def in `schemeExpr`), evaluate CBV (no under-λ eval), read back `SObj → SNF → Term`, **fall back to the slow evaluator when Scheme isn't available** (`snormalise`).
  This is Idris 2's `vm_compute`, and it is the exact architecture of gandr's planned Cranelift-JIT normalization with `jit ≡ eval` differential (ADR-51): same-values check comes for free by comparing against the tree-walking driver.

### 1.6 Unification / solver architecture (what full DT adds to a worklist checker)

- `UState`: `constraints : IntMap Constraint` where a constraint is a **suspended conversion problem** `MkConstraint fc withLazy env (x : NF) (y : NF)`; holes/guesses as `IntMap (FC, Name)` keyed by `Resolved` ints; `Guess` definitions carry constraint-ID lists and become real definitions when their constraints resolve.
- **Delayed elaborators** (`delayedElab : List (DelayReason, Int, hints, Core ClosedTerm)`) with an explicit priority order (CaseBlock < Ambiguity < LazyDelay < RecordUpdate < Rewrite) chosen for error quality — retry scheduling is _reason-tagged_, not FIFO.
- `noSolve` set (checking an LHS must not solve its own type's metas), `polyConstraints` (LHS polymorphism guards), `dotConstraints`.
- Net for `wyrd-3esy`/gandr: full DT turns the checker's worklist items into (a) conversion problems over the _value domain_ (hence ADR-50 D's "conversion checker as thin extension" is exactly where Idris 2 lives), (b) guarded definitions, (c) reason-tagged retry queues. gandr's existing worklist solver + frames architecture is shape-compatible; the new object is the constraint-as-NF-pair, which demands values be **storable in checker state** — the first-order value-domain requirement again (Idris 2 stores `NF` with host-closure binders in `UState` and pays with non-serializability of checker state).

### 1.7 Elaborator architecture (brief)

Bidirectional elaboration over `TTImp` with: metavariables as context entries (`Hole`, `Guess`), **branching + staging** for ambiguity (§1.4), delayed elaborators (§1.6), auto-implicit proof search (`BySearch`), `%World`-token IO threading, LHS checked as terms then converted to patterns (`As` nodes carry use-side for linearity).
Case blocks elaborate to top-level functions (with the conversion cost noted in §1.3).

### 1.8 Metatheory-presentation borrowings (flagged only; ADR-30 unaffected)

- QTT (Atkey/McBride) as _presentation_ of usage-0 erasure: multiplicities live on binders, erasure is decided at the type (Pi telescope), `findErased` is ~40 lines — the whole "dependent surface → erased runtime" story needs no new judgment forms.
  Candidate borrowing: state gandr's erasure pass as a telescope-walk producing positional masks (ADR-21 grading coupling: gandr grades generalize Rig; the mask computation stays a fold over binder grades).
- `WhyErased = Placeholder | Impossible | Dotted` — erasure _with provenance_ (why is this hole erased) retained through the core; matches gandr's marks discipline (evidence-with-provenance as IR data).

---

## 2. Lean 4 — the engine recipes (storage / equality / memo / dealloc / unfolding control)

### 2.1 Storage & equality: cached-word-per-node, pointer-eq fast path, sharing as a pass

- Every `Expr` node carries one packed `uint64` (`Expr.Data`, `src/Lean/Expr.lean:132-159`, mirrored in C++ `kernel/expr.h:126-158` with cross-checked bit layout): **hash (low 32)** | approxDepth (8) | hasFVar (40) | hasExprMVar (41) | hasLevelMVar (42) | hasLevelParam (43) | **looseBVarRange (44+, 20 bits)**.
  `hash e` and all the "does it contain X" guards are O(1) field reads.
  This is exactly the `wyrd-yg03` cached-hash side table, except **intrusive** — for the arena end-state gandr can put the word in the node (u64 alongside children ids), not in a side table.
- `is_eqp` = raw pointer equality; used everywhere as the first def-eq test (`quick_is_def_eq`).
  No global hash-consing: two structurally equal exprs are usually _not_ pointer-equal; **maximal sharing is an explicit pass** — `ShareCommon` (`src/Lean/Util/ShareCommon.lean`: a `StateFactory` of hash-map/hash-set, `shareCommon : α → α`) run at serialization/import boundaries and in specific subsystems (`MetavarContext`, elab pre-definitions).
  New in-tree: `Meta/Sym/AlphaShareCommon` + `Meta/Canonicalizer` — the `grind`/symbolic engine is growing an **alpha-aware canonicalizing table** on the side, which is precisely gandr's "canonical-key prerequisites before interning" (ADR-50 B) appearing independently: interning tables live per-subsystem, keyed canonically, never globally in the evaluator.
- **Worklist deallocator** (`src/runtime/object.cpp:432-458`): `lean_dec_ref_cold` frees with an explicit intrusive `todo` stack (`lean_del_core` decs children onto `todo`; loop pops until empty) — no recursion, no deep-`Drop` stack overflow.
  Bonus recipe ADR-50 didn't record: **`LEAN_LAZY_RC`** (`object.cpp:350-362`) — deletion can instead push onto a thread-local `g_to_free` list, and `lean_alloc_object` pops _one pending object per allocation_ — lazy, allocation-amortized freeing with bounded pause.
  Relevant only on the ADR-50 reversal path (arena makes both moot), but the citation is now precise.

### 2.2 Kernel def-eq: lazy-δ with heights + caches (`src/kernel/type_checker.cpp`)

- `reducibility_hints` = `Opaque | Abbreviation | Regular (height : UInt32)`; height = definitional depth of the body.
  `compare` (`kernel/declaration.cpp:24-45`): equal kinds+heights → unfold both; regular vs regular → **unfold the taller one**; opaque loses to everything; abbreviation unfolds first.
- `lazy_delta_reduction_step` (`type_checker.cpp:884-941`): only-one-side-δ → unfold it, _unless_ the other side is a projection application (`try_unfold_proj_app` — the `expensive_term =?= instFoo.1 a` perf fix: prefer reducing the projection).
  Both-δ, same head, regular hints → **try args-only unification first**, guarded by a **failure cache** (`m_failure : set<expr_pair>` normalized by hash order; `cache_failure`) so the args-first optimization never retries a failed pair; on failure unfold both.
  After every step, `quick_is_def_eq`.
- The driving loop (`lazy_delta_reduction`, 973-999) interleaves fast paths _before_ each δ-step: Nat-offset peeling (`succ^k` both sides), closed-`Nat` GMP arithmetic (`reduce_nat`), **`reduce_native`**, string-literal expansion.
  Fuel for gandr: the oracle-path arithmetic fast paths (ADR-39-adjacent) belong _inside_ the conversion loop, not only in eval.
- Memo tables in `type_checker::state` (`type_checker.h:27-33`): `m_infer_type[2]` (two caches), `m_whnf_core`, `m_whnf`, `m_failure` — all pointer/hash-keyed `expr_map`s.
  Def-eq _success_ caching lives at the Meta layer (`Meta/Basic.lean:389-394`: persistent + `defEqTrans` transient cache, reset when mvar assignments change).
- Kernel whnf has `cheap_rec`/`cheap_proj` variants (whnf_core at recursor major premise / projected arg instead of full whnf) — depth-limited unfolding knobs baked into the API.

### 2.3 Elaborator def-eq: the full 11-rule unfolding policy (`src/Lean/Meta/ExprDefEq.lean:1662-1694`)

Verbatim policy (doc comment at `isDefEqDelta`): (1/2) only one side δ-candidate → unfold it; (3) same head → args-first, then unfold both; (4/5) **projection-application vs not → unfold the non-projection side** (motivations: `id ?m =?= (a,b).1`, `List.length (a::as) =?= length as + 1`; class projections additionally unwrapped via `packedInstanceOf?` chasing, issue #1419); (6/7) **`@[reducible]` vs not → unfold the reducible side**; (8) mvar-free → kernel height strategy (`unfoldDefEq`, 1582-1593); (9/10) **unfold whichever side's unfolding matches the other's head symbol** (`unfoldComparingHeadsDefEq`, `sameHeadSymbol`); (11) unfold both.
Rules 4-7 and 9-10 have no kernel counterpart — they are unification-quality heuristics; rule 8 shows the kernel strategy is deliberately _gated on mvar-freeness_.

### 2.4 Transparency + smart unfolding (`src/Lean/Meta/GetUnfoldableConst.lean:16-38`, `Meta/WHNF.lean`)

- Transparency lattice (`canUnfoldDefault`): `none` (nothing) < `reducible` (`@[reducible]` only) < `instances` (+`instanceReducible`) < `implicit` (+`implicitReducible`) < `default` (all but `@[irreducible]`) < `all`.
  Overridable per-context hook (`ctx.canUnfold?` — e.g. `canUnfoldAtMatcher` special-cases for `simp` at reducible transparency).
- **Smart unfolding** (`Meta/WHNF.lean:866-956`): for each structurally-recursive `f`, a companion `f._sunfold` holds the _match-annotated_ equations; `unfoldDefinition?` unfolds via the companion and succeeds **only if the annotated match actually reduces** (recursive argument reaches a constructor, checked via `getStructuralRecArgPos?` + `isConstructorApp`).
  Prevents the classic disaster of unfolding a recursive def into stuck `brecOn` gas.
  This is the load-bearing piece of "recursive definitions behave under def-eq" that neither Idris 2 nor Agda has in this form; gandr's unroll-freeze (L2 lane) + future conversion checker should adopt the _principle_: a recursive definition's engine-unfolding is licensed by its scrutinee making progress, not by unconditional δ.
- For `wyrd-zg5r` Decision E execution, the minimal Lean-recipe set is: reducibility hints with heights + the taller-unfolds rule; args-first-with-failure-cache; transparency lattice as a `canUnfold` hook on the NbE driver's `evalRef` analogue; whnf/infer caches keyed by identity; smart-unfolding principle for recursive defs (can piggyback on gandr's case-tree progress).

### 2.5 Codegen/erasure sliver (3esy input)

LCNF (code-gen IR): `toLCNFType` (`Compiler/LCNF/Types.lean:127-232`) maps **any Prop to `lcErased`** and non-representable dependency to `lcAny` — erasure decided _at the type-translation boundary into the compiler IR_ (proofs gone wholesale; no per-binder usage analysis like QTT — Lean erases by _sort_, Idris 2 by _multiplicity_).
Then base→mono phases, boxed→unboxed, down to the old IR → C (plus the newer native `reduce_native` hook the kernel can call back into — compiled evaluation reused _inside_ conversion, the same jit≡eval seam as Idris 2's SchemeEval and gandr's planned Cranelift JIT).

One structural note: Lean's kernel/elaborator reducer is **not an environment machine** — it rewrites `Expr`s by instantiation, and compensates with the cached-word guards (skip traversals when `looseBVarRange == 0` / no mvars), the whnf/infer caches, pointer-eq fast paths, and the literal/native fast lanes.
Lean is the existence proof that a _cache-and-share_ substitution engine can be fast; but it is also not trying to be a reified machine, and its state is unserializable mid-reduction. gandr's CEK choice tracks Idris/Agda, and takes Lean's caches on top — the ADR-50 partition, confirmed from source.

---

## 3. Agda — the first-order machine precedent + the sharing lesson

### 3.1 `Reduce.Fast` (`src/full/Agda/TypeChecking/Reduce/Fast.hs`) — the machine 8pu3 should crib from

- **Fully first-order, two-state CEK**: `Eval !(Closure s) !(ControlStack s)` and `Match QName FastCompiledClauses (Spine s) !(MatchStack s) !(ControlStack s)`.
  `Closure = Closure IsValue Term (Env s) (Spine s) | BlackHole`; `Env = [Pointer s]` (de Bruijn); `Spine = [SElim s]` (apply/proj/cubical-IApply).
  **No host closures anywhere** — the one whole-machine precedent for ADR-50 C's first-order discipline.
- **Explicit reified control stack**: `CaseK` (match continuation with left/right spine segments + match stack), `ArgK` (NF-mode readback continuation holding a **spine zipper** — normalization of arguments is itself continuation-driven, i.e. iterative readback in the ADR-47 sense), `NormaliseK`, `ForceK`/`EraseK` (primitives that inspect evaluation), plus update frames.
  `MatchStack = CatchallFrames :> Closure` — catch-all backtracking frames + the stuck-return closure carried on the stack (case trees are not fully expanded; inner partial matches fall through to outer catchalls).
- **Call-by-need via ST heap**: `Pointer = Pure Closure | Pointer (STRef (Thunk s))` — `Pure` for closures that don't need sharing (no update cost), real pointers black-holed during evaluation (`blackHole`, loop debugging), **strict `storePointer`** (leak prevention), and `createThunk` collapses naked-variable closures onto the existing env pointer (**no pointer chains**).
  The heap is machine-local — see 3.2.
- **Values carry their blocker**: `IsValue = Value Blocked_ | Unevaled` — a WHNF closure records _why_ it can't reduce further (which meta/var it is stuck on).
  This is the load-bearing detail for a typechecker machine: stuckness propagates as data, so the constraint scheduler upstream knows what to wait on. gandr's future conversion/solver should adopt exactly this (values ∋ blocker id), replacing nothing in v0 dynamics (the machine's `UnsupportedByReference`-style outcomes are the current analogue).
- **Per-run compact definition views**: memoized `getConstInfo` → `CompactDef`/`FastCompiledClauses` preprocessing (drop arities the machine doesn't need, `NameId` int keys instead of `QName`, a special `suc` branch, primops as pure `[Literal] -> Term` functions, builtins resolved once into `BuiltinEnv`).
  The machine never sees the checker's full `Definition`.
  For gandr: the CEK machine should consume a compact per-run view of definitions, not checker-face structures — this is per-face separation (ADR-48/50) appearing as a pure performance move.
- **Graceful two-tier fallback**: anything the machine doesn't support (`COther`, `Level`/`Sort`/`Dummy` terms, non-standard equality, unconfirmed defs) → `fallbackEval` decodes the focus back to a `Term`, runs the _slow_ substitution-based reducer (`slowReduceTerm`/`slowNormaliseArgs`), and re-packs the result as a value closure — **mid-computation interop between fast machine and reference evaluator**.
  Directly reusable: gandr's CEK can grow coverage incrementally with the current interpreter as in-loop fallback, which also keeps the ADR-48 oracle honest.
- Unfolding control: a per-run `AllowedReductions` SmallSet (function/projection/recursive/copattern/inline/...) + per-def `shouldReduceDef` — coarser than Lean, finer than Idris; applied once at `CompactDef` build time, not per step.

### 3.2 The `--sharing` lesson, from the release notes (the ADR-50 citation made precise)

- 2.5.1: "New options: `--sharing` and `--no-sharing` ... enable/disable sharing and call-by-need evaluation.
  The default is `--no-sharing`" — the term-graph sharing experiment.
- 2.5.4 (`doc/release-notes/2.5.4.md:18-21`): "Compile-time weak-head evaluation is now call-by-need, but **each weak-head reduction has a local heap, so sharing is not maintained between different reductions**" — the rewrite as the `Reduce.Fast` machine.
- 2.6.0: "Deprecated options `--sharing` and `--no-sharing` now raise an error." — the experiment deleted.
  Net: Agda tried global term-graph sharing, abandoned it, and landed on **sharing as machine-local state, reconstructed per run**.
  Together with Lean's boundary-pass `ShareCommon` and Kovács's space argument, all precedents agree with ADR-50 B: no canonicalizing table under β; interning is for static data and lives per-subsystem/per-face.

### 3.3 Erasure + backends sliver (3esy input)

Treeless pipeline (`Agda.Compiler.Treeless.*`, consumed by MAlonzo/GHC and JS backends): case trees → `TTerm` (de Bruijn ints, `TErased` residual) → **`Erase.hs`**: per-type `TypeInfo = Empty | Erasable | NotErasable` (type-*structure* analysis: empty types, propositional/singleton content) computed with a fixpoint that **assumes recursive occurrences erasable** (`memoRec ... Erasable`), combined with **modality** (`usableModality` — Quantity-0/irrelevance) and **usage analysis** (`usedArguments`); erased constructor args recorded (`setErasedConArgs`), functions get per-argument erasure masks (`getFunInfo`), erasure applied as substitution of `TErased` + case-on-erased collapsing.
So Agda erases by _modality + type-structure + usage_, Idris 2 by _multiplicity (+detag)_, Lean by _sort_.
Three independent designs converge on: (a) per-position masks on defs/constructors, (b) a residual erased-marker term, (c) erasure decided before the backend IR proper — the gandr erasibility story (grades → masks) sits squarely in this design space, with Agda's type-structure analysis the best candidate _addition_ on top of grade-driven masks.

---

## 4. smalltt — the elaboration-performance blueprint, verified against source

(smalltt @ `ea99b0f`; added to the read by owner steer 2026-07-06.
ADR-50 A names smalltt the elaboration-performance blueprint; this section verifies what the blueprint actually is.)

### 4.1 The value domain and what "glued" means here (`src/CoreTypes.hs`)

- `Val = VLocalVar Lvl Spine | VFlex MetaVar Spine | VUnfold UnfoldHead Spine ~Val | VLam NameIcit Closure | VPi NameIcit VTy Closure | VU | VIrrelevant` with `Closure = Closure Env Tm`, `Env = ENil | EDef Env ~Val` — **fully first-order closures**, de Bruijn _indices_ in terms, de Bruijn _levels_ in values.
- **Gluing is per-unfolding-node, not a global pair**: `VUnfold head spine ~unfolded` keeps the _neutral_ form (head + spine, where head = top-level var or solved meta, bit-packed `UnfoldHead# Int ~Val`) **and** a lazy fully-applied unfolded value, built incrementally as the spine grows (`app` extends both: `VUnfold h (SApp sp u i) (app v u i)`).
  Additionally `G {g1, g2 :: ~Val}` pairs two lazy values for elaborator types (both start as the same value via `gjoin`; one side gets forced, the other stays small).
- Terms cache values too: `TopVar Lvl ~(DontPrint Val)` — the top-level _term node itself_ carries the definition's value lazily; `MetaEntry = Solved (cache ref) LvlSet Tm ~Val` stores term and value both.
  Metas and top-defs live in **int-indexed dynamic arrays** (`MetaCxt`, `TopVals`) — the arena discipline again.
- Call-by-need comes **from host laziness** (`~Val` fields, GHC thunks).
  Rust gets no such freebie — the explicit version is Agda's pointer/STRef pattern (§3.1); this is a real representation-design consequence for `wyrd-8pu3`, not a footnote.

### 4.2 Forcing and quotation: unfolding control as _two small enums_ (`src/Evaluation.hs`, `src/Common.hs:184-222`)

- Three force modes: `force` (only convert solved `VFlex` heads to `VUnfold`), `forceMetas` (chase meta-unfoldings only), `forceAll` (eliminate all unfoldings from the head).
  Values never lose the neutral face — forcing chooses which face to _look at_.
- `QuoteOption = UnfoldAll | UnfoldMetas | UnfoldNone` — readback picks the face per call; `UnfoldNone` quote is the "small term" readback (top defs and solved metas stay as heads), `zonk` = the term-level pass that inlines solved metas while otherwise minimizing output.
  This is the concrete mechanism behind §5.2's "the quote-face policy and the def-eq-unfold policy are the same table."

### 4.3 Conversion/unification: the speculation discipline (`src/Unification.hs`)

- `ConvState = Rigid | Flex | Full`.
  In `Rigid`, two same-head `VUnfold`s are compared **spine-first in `Flex` state, with exception-based backtracking to `Full`** (`unifySp Flex sp sp' \`catch\` \_ -> unify Full (G topt t) (G topt' t')` — and the `Full` retry uses the _already-forced_ side of the `G` pair, no re-evaluation). `Flex` state forbids committing: no meta solutions (`guardCS` throws `FlexSolution`), no negative verdicts on unfolding mismatch. `Full` forces everything and is definitive.
  This is Lean's args-first heuristic (§2.2) as an explicit three-state discipline — with the failure-cache replaced by cheap exceptions + glued retry.
- **Meta solutions are quoted small**: `rigidQuote` (solution readback under a partial renaming) keeps `VUnfold`s as heads with a per-spine validity bit, running an **approximate occurs-check memoized per solved meta** (`Solved (RF.Ref MetaVar) …` caches the last successfully-checked occurring meta) and escalating to `fullCheckRhs` only when the approximation fails.
  Solutions stay small ⇒ the meta context never explodes ⇒ later unfoldings stay cheap — this is the payoff loop that justifies gluing at all.
- **Frozen metas**: metas belonging to previously-elaborated top defs are frozen (`frz` boundary; `TopEntry` records each def's meta bound) — solving is scoped to the active definition.
  A staged-elaboration discipline gandr's A2 incremental checking can reuse directly.
- `flexFlex` solved by try-left-catch-try-right; partial renaming over an `AFM.Array Lvl` (flat int array, `-1` = undefined) — everything int-indexed, no maps on hot paths (symbol interning happens once at the parser: `SymTable` with a custom hash).

### 4.4 What the blueprint amounts to for gandr

smalltt is the _composition proof_ for zg5r's Decision D: one first-order value domain where (a) unfolding is delayed per-node with both faces retained, (b) force/quote select faces via two small enums, (c) conversion speculates cheaply and retries on the retained faces, (d) meta solutions are read back small.
None of it needs host closures, global sharing, or a second value domain.
What smalltt does **not** model: effects/continuations (no `K` — pure λΠ), case trees/data, erasure, or serialization concerns — those come from Idris 2/Agda (§§1,3).
The gandr-specific deltas remain: `K` stays reified (ADR-50 C), thunk memoization must be explicit (no host laziness), and values live in the arena.

---

## 5. Synthesis — keyed to consumers

### 5.1 Convergences across the systems (strong signals)

1. **No global hash-consing in the evaluator; sharing is per-run machine state or a boundary pass.** Idris 2: none (and it hurts).
   Agda: tried it (`--sharing`), deleted it, per-reduction local heap.
   Lean: `ShareCommon` at import/serialization + per-subsystem canonicalizers (`Meta/Sym`).
   ADR-50 B (content-address as third discipline, per-face interners, no canonicalization under β) is confirmed three ways.
2. **Case trees before the machine, compact per-run def views, int-keyed lookups.** Idris `Resolved Int`+`IOArray`; Agda `CompactDef`+`NameId` keys; Lean's environment maps with pointer-hashed exprs. gandr's arena + per-face compact views is this pattern taken to its limit.
3. **Two-tier evaluation with a semantic reference in the loop.** Agda fast-machine ↔ `slowReduceTerm` mid-computation interop; Idris SchemeEval (compile-to-Chez, memoized per def, quote back, fallback to slow); Lean `reduce_nat`/`reduce_native` inside kernel def-eq. gandr's `jit ≡ eval` differential (ADR-51) is this industry pattern _plus verification_ — and Agda's fallback shape is the right way to grow CEK coverage incrementally.
4. **Stuckness as data.** Idris: `CaseResult{GotStuck}` + lazy neutral fallback; Agda: `IsValue = Value Blocked_` (values carry their blocker); Lean: `l_undef` + postponement queues.
   For the future conversion checker/solver: gandr values should carry blocker identity (meta/hole id) — the cheap version of Agda's design — so worklist scheduling has exact wake-up conditions (matches the existing solver's frame discipline).
5. **Single-result machine/IR everywhere.** No multi-value seam exists in any of the four (see 5.5 for what h1z4 can still take).

### 5.2 For `wyrd-zg5r` (glued NbE + engine unfolding) — the sharpest corrections

- **"Glued" must be split into two named faces** (ADR-50 D conflates them): (a) _term-face gluing_ (Idris `Glued` = lazy term ∥ lazy NF) — elaborator-facing, kills quote traffic; under the arena this degenerates to **caching the origin `NodeId` on each value** (a value remembers the term it came from — nearly free, do it from day one); (b) _unfolding-face gluing_ (smalltt, now source-verified §4.1: **per-unfolding-node** `VUnfold head spine ~unfolded` — neutral face and unfolded face retained together, built incrementally as the spine grows) — conversion/readback-facing, kills size blowup; this is the one that interacts with Decision E's unfolding control (readback chooses the face via `QuoteOption`; conversion forces the unfolded face on demand via the `force`/`forceAll` split, §4.2).
  Recommendation: implement (a) now (trivial), design (b) together with the unfolding hints so the "which face does quote choose" policy and the "which side does def-eq unfold" policy are the same table — smalltt shows the whole table is two small enums plus a three-state conversion discipline (`ConvState`, §4.3).
- **Engine unfolding: adopt the Lean recipe set with citations** — heights on defs (`ReducibilityHints`), taller-unfolds-first + args-first-with-**failure-cache** for same-head (kernel `type_checker.cpp:884-941`), transparency as a `canUnfold` hook (`GetUnfoldableConst.lean:16-38`), whnf/infer memo tables, arithmetic/literal fast paths _inside_ the conversion loop, and the elaborator's 11-rule policy (`ExprDefEq.lean:1662-1694`) as the reference for the unification-facing extension (projection-preference, reducible-preference, head-symbol matching — rules with no kernel counterpart).
  The **smart-unfolding principle** (recursive def unfolds only if its recursive scrutinee makes progress; `Meta/WHNF.lean:866-956`) is the missing piece neither Idris nor Agda has — it is what makes recursive definitions def-eq-stable, and gandr can implement it directly on case-tree progress (no `_sunfold` companion defs needed since gandr keeps case first-class).
- Quote: keep it iterative via Agda's `ArgK`/spine-zipper readback shape (a worked example of continuation-driven normalization = ADR-47-compliant readback); adopt Idris's `sizeLimit` fuel and `clearDefs`-style zero-unfold readback as explicit `QuoteOpts`.

### 5.3 For `wyrd-8pu3` (CEK/arena) — machine details worth cribbing

- Agda `Reduce.Fast` is the implementation blueprint at machine granularity: two-state Eval/Match, `MatchStack` catchall frames, `Pure` vs shared-pointer thunks, black-holing, strict thunk update, naked-var pointer-chain collapsing, compact def views with int keys and a special `suc`/literal branch. gandr differences: K already reified (ADR-9/34), arena `NodeId` instead of `Term`, and the heap should be the env/arena rather than `STRef`s.
- **Memoized closures are non-negotiable**: Idris 2's un-memoized CBN closures are the negative precedent (its `updateLocal` scrutinee-writeback is a band-aid).
  Call-by-need or explicit update slots from day one.
- Idris's `Lazy NF` neutral-fallback threading (build the stuck answer once, return it from any stuck path) is a clean micro-pattern for the machine's stuck-outcome paths.
- Idris context staging (`branchDepth` + `staging : IntMap`, commit-on-success) is a worked transactional-overlay pattern for the arena under speculative work — relevant to checkpoints (`wyrd-0iba`) and any future backtracking elaboration.
- For `wyrd-yg03` (interim, already licensed): Lean's cached word argues the side table should carry **hash + hasMeta/hasFVar bits + loose-bvar-range + approx-depth**, not hash alone — the O(1) "can I skip this traversal/instantiation" guards are where Lean's substitution engine gets its speed, and they carry into the arena as an intrusive u64.

### 5.4 For `wyrd-3esy` (full-DT feasibility) — inputs, not a verdict

- **Compile-and-run-efficiently: positive, well-trodden.** The erasure story has three independent, convergent designs (masks + residual marker + pre-backend erasure; §1.5/§2.5/§3.3); the runtime contract is small (VMCode's "closures + apply" one-liner); Idris 2 is the end-to-end existence proof for exactly gandr's decided arc (dependent surface → usage-0 erasure → non-dependent core → multiple backends).
  Erasure itself is _cheap_ (`findErased` is a telescope walk; masks ride on defs).
- **ADR-21 grading coupling: precedented.** QTT multiplicities-on-binders → positional masks is the worked instance of "grades drive erasure"; gandr grades subsume Rig0, so the mask computation is a grade-indexed fold.
  Agda adds the useful refinement: erasability by _type structure_ (empty/singleton) and _usage_, computed as a fixpoint that assumes recursive occurrences erasable — adoptable later without judgment changes.
- **Reified-machine survival: the differentiator.** Idris 2 stores host-closure-bearing `NF`s in solver state (`UState` constraints) and its checker state is therefore unserializable — it _works_, but it forfeits exactly what gandr's ADR-9 thesis protects.
  Agda's machine shows first-order closures + blockers-as-data scale to a full dependently-typed checker's evaluator.
  Full DT does **not** force giving up the reified machine — but only under the first-order value-domain discipline ADR-50 C already mandates.
  This is the strongest single input to the 3esy gate from this read.
- **What full DT will actually add to the checker** (from Idris's `UState`): constraints as suspended conversion problems over values, guarded definitions (`Guess`) keyed by constraint ids, reason-tagged retry queues, an LHS `noSolve` discipline. gandr's worklist solver is shape-compatible; the new requirement is values-in-solver-state (→ first-order, again).
- **Costs observed in the wild**: conversion-driven performance cliffs (Idris without memoization/caches; Lean's failure cache + smart unfolding exist because the cliffs are real).
  The Lean recipe set is the known mitigation; gandr is already adopting it (zg5r).

### 5.5 For `wyrd-h1z4` (multi-output term face) + `wyrd-rd49.2` (pliron)

- No multi-value seam exists in Idris/Lean/Agda IRs — h1z4 gets no direct precedent, confirming it must be designed, not borrowed.
  The nearest relative: **VMCode is destination-passing at instruction granularity** (every instruction's first `Reg` is its destination; `PROJECT`/`MKCON` write into named registers) — i.e., the machine level is _already_ destination-style everywhere; what's missing is only _multiple_ destinations per operation.
  This weakly supports ADR-49's claim that the multi-output face is a term-level generalization the backend can absorb (Cranelift is natively multi-value; the gap is purely in the middle IRs).
- For rd49.2: Idris's ladder (CExp → Lifted → ANF → VMCode) is a reference decomposition for what gandr's checked-core → pliron lowering must cover (saturation/eta at entry; lambda-lift; ANF; registers), and `Compiler.Common` shows per-def compiled-face caching on the context.
  Lean LCNF's base→mono phase split (polymorphic → monomorphic/boxed) is the analogous decomposition with types kept longer.

### 5.6 Residual seams (to file as beads)

1. zg5r: implement term-face gluing as origin-`NodeId` caching; design unfolding-face gluing jointly with the hints table (the ADR-50 D conflation fix), per-node `VUnfold`-style (§4.1).
2. zg5r: smart-unfolding-on-case-progress for recursive defs (Lean principle, gandr mechanism).
3. 8pu3: values carry blocker identity (Agda `IsValue`) — spec the stuck-value shape before the conversion checker exists.
4. 8pu3/0iba: transactional staging overlay on the arena (Idris `branchDepth`/`staging` pattern) for checkpoints + speculative elaboration.
5. yg03 refinement: cached word = hash + flag bits + loose-var-range + approx-depth (not hash alone).
6. 3esy: record the reified-machine-survival finding (first-order discipline is the enabling condition) in the study.
7. Backend arc: jit≡eval fallback interop shape (Agda `fallbackEval` / Idris SchemeEval memoized-compile pattern) for the Cranelift JIT driver.
8. zg5r/conversion: smalltt's `ConvState` Rigid/Flex/Full speculation with glued retry + memoized approximate occurs-check — the conversion-checker design seed (evidence-layer era, but the value-domain hooks are laid now).
9. A2/incremental: smalltt's frozen-meta boundary per top definition (`frz`) as the staged-elaboration discipline for gandr's checkpoint/streaming checker.
10. 8pu3: explicit thunk-memoization cells required — smalltt's call-by-need rides GHC laziness (`~Val`), unavailable in Rust; Agda's `Pure`-vs-`Pointer` split is the explicit design (which closures need sharing at all).

### Outlook

**FOR (what this read de-risks):**

- ADR-50's every major decision has at least one worked in-production precedent (arena/int-handles: Idris context + smalltt's int-indexed meta/top arrays; first-order machine: Agda + smalltt; caches/unfolding: Lean; glued: smalltt per-node `VUnfold`, source-verified §4.1).
- The full-DT arc (3esy) has an end-to-end existence proof whose known weaknesses (closure memoization, conversion caching, serializability) are exactly the points where ADR-50 already chose the stronger design.
- The jit≡eval pattern is standard practice in this family (SchemeEval, reduce_native), not an experiment.
- The unfolding-face gluing now has a precise, verified design (smalltt §4): two enums + a three-state conversion discipline — small enough to adopt wholesale in zg5r.

**AGAINST / open:**

- "Glued from day one" as written in ADR-50 D names two different mechanisms (term-face vs unfolding-face); the 5.2 split must be adopted explicitly in zg5r's design or the elaborator-facing half gets built while the conversion-facing half is believed to exist.
- None of the four systems runs gandr's combination (effects + first-class K + conversion over one value domain); the two-driver-one-domain architecture remains gandr's own bet — the precedents de-risk the parts, not the composition. smalltt in particular has no `K`, no data/case, no erasure — its blueprint covers elaboration only.
- Multi-value (h1z4) is confirmed precedent-free in this family; the design pass carries full novelty risk.
- Lean's unfolding-control quality rests on _heights computed at definition time_ and on `@[reducible]`-style curated annotations; gandr has no annotation culture yet — the engine layer will need sensible defaults (height from definition DAG depth is mechanical; transparency defaults need a policy decision).
- smalltt's call-by-need is free host laziness; gandr must budget explicit thunk cells (5.6 #10) — a cost smalltt's benchmarks silently exclude.

**Net:** the ADR-50/51 architecture survives contact with all four codebases; the one substantive correction is the glued-representation split (5.2, now with a verified design for the hard half), and the highest-value imports are Agda's machine details (5.3), Lean's def-eq recipe set with the smart-unfolding principle (5.2), and smalltt's force/quote/ConvState table (§4).
