# Performance architecture

The design basis of the performance-architecture phase (the glued-NbE hash-consing normalizer adapted to the L machine, and the evaluator discipline generally): what is adopted, which precedent settles it, and what none of the precedents provides.
The primary record is `docs/research/impl-models-deep-read.md` — a source-grounded read of four implementations at pinned checkouts (Idris 2 `fd405085b` [@idris2], Lean 4 `d1f105109b` [@lean4], Agda `6e4d6e9543` [@agda], smalltt `ea99b0f` [@smalltt]), whose `file:line` anchors into those trees it keeps verbatim.
Everything below is an adopted commitment, a recipe with a named precedent, or a parked item with a reason; the phase is open, so nothing here is as-built yet.

## The role partition

Each system is read for what it is the strongest precedent for, and no more.

* **Lean 4** — storage, equality, memoization, deallocation, and the unfolding-engine recipes.
  Its kernel definitional-equality loop is the measured reference for the conversion checker's future design.
* **Agda (`Reduce.Fast`)** — the first-order machine precedent: the one whole-machine existence proof that a fully first-order environment machine scales to a full dependently typed checker's evaluator.
* **smalltt** — the elaboration-performance blueprint: the glued value representation and the conversion-speculation discipline, verified against source.
* **Idris 2** — the end-to-end existence proof for the erasure-and-backends arc (dependent surface, usage-0 erasure, non-dependent core, multiple backends), and the source of the negative precedents (un-memoized closures; case-block conversion by source location; no unfolding control).
  The older "Idris 2 as whole-system model" framing is superseded: the pivot to Lean is implicit in the kernel decision, and Agda plus Lean proved the more-used imports — the record's own synthesis reaches that conclusion.

## Glued is two mechanisms, not one

The phrase "glued representation" names two different designs, and adopting one while believing the other exists is the recorded failure.

* **Term-face gluing** — a value remembers the term it came from (Idris 2's lazy term∥value pair, used to kill readback traffic in elaboration).
  Under the arena this degenerates to **caching the origin `NodeId` on each value** — nearly free, and adopted from day one.
* **Unfolding-face gluing** — a value keeps a neutral face (head plus spine) and a lazy fully-applied unfolded face **together, per unfolding node, built incrementally as the spine grows** (smalltt's `VUnfold`).
  This is the conversion-facing half and the load-bearing one: readback chooses a face per call (quote options: unfold-all, unfold-metas, unfold-none — the "small term" readback keeps heads folded), and conversion forces the unfolded face only on demand.
  **The quote-face policy and the definitional-equality unfold policy are the same table**: two small enums (force modes; quote options) plus the speculation discipline below.

## Unfolding control — the recipe set

The conversion checker's unfolding discipline adopts the measured Lean kernel recipe set, with the elaborator's heuristics as the reference for the unification-facing extension.

* **Reducibility hints with definitional heights**; equal-height definitions unfold together, otherwise **the taller one unfolds first**.
* **Args-first with a failure cache** for same-head pairs: try unifying the spines before unfolding both, and never retry a failed pair.
* **A transparency lattice as a hook** (nothing / reducible-only / instances / default / all), overridable per context — gandr has no annotation culture yet, so the defaults are a policy decision the phase must take explicitly (heights are mechanical from the definition DAG; transparency defaults are not).
* **whnf and infer memo tables keyed by identity**, and **arithmetic and literal fast paths inside the conversion loop**, not only in evaluation.
* **Smart unfolding**: a recursive definition's engine-unfolding is licensed by its scrutinee making progress (the match reduces), never unconditional — the principle that keeps recursive definitions from unfolding into stuck recursor gas, which neither Idris 2 nor Agda has in this form. gandr's mechanism is its case-tree progress, not a companion definition.

Conversion speculation follows smalltt's three-state discipline: **rigid** compares same-head neutrals spine-first without committing, **flex** forbids solutions and negative verdicts, **full** forces everything and is definitive — cheap backtracking between the states, with the retry running on the already-forced retained faces rather than re-evaluating.
Two refinements come with it: **meta solutions are quoted small** (an approximate occurs check, memoized, escalating only on failure — solutions staying small is the payoff loop that justifies gluing at all), and **metas of previously elaborated definitions are frozen** at a per-definition boundary, the staged-elaboration discipline the incremental lane reuses.

## The machine cribs, ported to the L machine

The machine-level findings target an environment machine and port to the L machine unchanged in substance.

* **Fully first-order machine state is the settled discipline** — the precedent machine is two-state (eval/match) with an explicit reified control stack, catchall backtracking frames, and closures that are data, never host functions.
  Idris 2's half-first-order domain (host-language function bodies under binders) is the counter-example: it works, but its checker state is unserializable — exactly what the reified-machine thesis protects.
* **Thunk memoization is non-negotiable**: un-memoized call-by-name closures with scrutinee write-back as a band-aid are the negative precedent.
  The explicit design is the pointer split — closures that need no sharing stay pure, shared thunks are black-holed during evaluation and strictly updated, and naked-variable closures collapse onto the existing pointer (no pointer chains).
  Host laziness does this for free in Haskell; **in Rust every one of these is an explicit cell, and the phase must budget it**.
* **Values carry blocker identity** — a WHNF records what it is stuck on (meta, variable), so a worklist scheduler knows exactly what to wait on.
  Stuckness as data is the agreed pattern across all four systems (case results with a carried neutral fallback; postponement queues; blocked values).
* **Compact per-run definition views**: the machine consumes a per-run compact view of definitions (integer keys, special-cased constructors, primitives resolved once, a per-run allowed-reduction set) — never the checker's full declaration structures.
* **Two-tier fallback interop**: anything the fast machine does not support decodes back and runs the reference reducer mid-computation, then re-packs — the way coverage grows incrementally while keeping the differential oracle honest.
  Compile-and-compare (the `jit ≡ eval` discipline) is industry-standard in this family — compile-per-definition memoized evaluation, native reduction inside kernel conversion — **not an experiment**.
* **Transactional staging on the arena**: writes during speculative work go to a staging overlay folded in on success — the checkpoint and speculative-elaboration mechanism.

## Sharing and identity

Three independent precedents say the same thing: **no global hash-consing in the evaluator**.
Sharing is per-run machine state (a machine-local heap reconstructed per reduction — the global term-graph-sharing experiment was tried and deleted), or an explicit pass at import and serialization boundaries, or per-subsystem canonicalizing tables. gandr's per-face interning discipline is this consensus taken as policy.

The cached word per node is wider than a hash: **hash, has-free-variable and has-metavariable flags, loose-bound-variable range, and approximate depth, as one intrusive u64** — the O(1) skip guards are where a substitution engine gets its speed, and they carry into the arena as node data rather than a side table.
Pointer-equality fast paths come first in every equality test, with oracle guards where coherence descent must not be skipped.

## The erasure arc

Three independent, convergent erasure designs exist: multiplicity-on-binders computed to positional argument masks; sort-based erasure at the compiler-IR type boundary (every proposition erased wholesale); and modality + type-structure + usage analysis computed as a fixpoint that assumes recursive occurrences erasable.
The shared shape: **erasure is decided at the type boundary, computed as a cheap telescope walk, with masks riding on definitions**, and erased residuals carry a marker with provenance (why this is erased) retained as IR data — the marks discipline's own idiom. gandr's grades generalize the multiplicity precedent, so the mask computation is a grade-indexed fold; the type-structure refinement is adoptable later with no judgment change.

## What none of the four provides

* **The composition is gandr's own bet.** No system runs effects plus first-class continuations plus conversion over one value domain; the precedents de-risk the parts, not the composition. smalltt covers elaboration only (no continuations, no data, no erasure); Lean is not trying to be a reified machine, and its state is unserializable mid-reduction.
* **The multi-output term face has no precedent anywhere in this family** — every machine and IR is single-result.
  The nearest relative is that instruction-level code is already destination-passing (every instruction writes a named register; the backend is natively multi-value), so the gap is only the term-level generalization — weak support for the face being absorbable, but the design is novel end to end.
* **The enabling condition for full dependent types is recorded**: full DT adds constraints-as-suspended-conversion-problems-over-values, guarded definitions, and reason-tagged retry queues to the checker — all shape-compatible with the existing worklist solver — and it does **not** force giving up the reified machine, but only under the first-order value-domain discipline.
  This is the strongest single input to the dependent-core phase.

## Parked items, with reasons

Each is a designed action whose home is a later phase; none is forgotten.

* **Term-face gluing** (origin-`NodeId` caching on values) — carried to the performance-architecture phase; cheap, but touches the value representation, so it lands with the normalizer, not piecemeal.
* **Unfolding-face gluing and the hints table** — one design, landed together; the quote/unfold shared table is the deliverable.
* **Smart unfolding on case progress** — needs the case-tree representation finalized first.
* **Blocker-carrying values** — pre-conversion-checker; the stuck-value shape is designed before the solver consumes it.
* **Transactional staging overlay** — the incremental lane's checkpoint mechanism for _elaboration_; lands with it, and is a different mechanism from the typing checkpoint of [[incremental-pipeline#Checkpoints and the reuse rule]] despite the shared word.
* **The intrusive cached word** (hash + flags + range + depth) — lands with the arena's node-layout change, not as a side table retrofit.
* **jit≡eval fallback interop** — the backend phase's driver shape (compile-per-definition memoized, reference reducer in the loop).
* **Frozen-meta boundaries per definition** — the incremental lane's staged-elaboration discipline.
* **Transparency defaults policy** — an explicit policy decision owed by the performance-architecture phase (heights from the definition DAG are mechanical; defaults are not).
* **The explicit thunk-cell budget** — a standing cost note: no host laziness exists to ride, and benchmarks from systems that have it silently exclude this cost.

## Source and confidence

The technical content is a single well-curated source (the deep-read record, medium confidence by the corpus's scale) whose claims are pinned to named checkouts and were verified in one session against them.
Its two recorded corrections are adopted here: the glued-representation split above, and the model pivot (Lean and Agda as the more-used imports).
Its residual seams are the parked items above; its outlook — the architecture survives contact with all four codebases; the composition remains gandr's own bet — is the posture of the phase table.
