# The incremental pipeline

The incremental pipeline is the loop that runs **between a keystroke and a rendered derivation**: reparse, diff, find a reusable checkpoint, re-type what changed, revalidate what did not, and merge the result into a derivation a renderer can draw.

It exists because the checker's state is data ([[typing-machine]]), and a checker whose state is data can be stopped, saved, and resumed.
Everything here is the consequence of that: **reuse is the point, and soundness of reuse is the whole difficulty.**

The one claim to carry away is that reuse here is **validated, never assumed**.
A cached result for an unchanged region of the program is not automatically still correct, because typing threads shared state — a substitution, a context, a solver trail — that re-typing a changed region can move underneath it.
So every reuse is a check, and the check is cheap enough that reuse still pays.

## What is built

The pipeline is the most-built unspecified subsystem in the tree, and the partition is not what a reader would guess from the design alone.

| component                           | status                                                                                                                                    |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| cold reparse                        | **built** — `gandr-surface-parser`; there is no incremental parser and that is a measured decision                                        |
| structural diff                     | **built** — `gandr-surface-syntax`'s `diff` module, over merkle-identified arena nodes                                                    |
| changed-region seam                 | **built** — `gandr-core-incrementality`'s `region` module, deliberately parser-agnostic                                                   |
| dependency footprint                | **built at item granularity** — that crate's `footprint` module                                                                           |
| validated checkpoints and resume    | **built at item granularity** — that crate's `checkpoint` module                                                                          |
| the incremental-equals-scratch gate | **built** — `tests/incremental.rs`, a differential against a from-scratch re-type, run both parser-free and through the surface front end |
| order maintenance                   | **built** — `gandr-theory-orders`, and consumed by the engine's resume to keep item identity across an edit                               |
| localized edit actions              | **built** — `gandr-surface-engine`'s `edit` module, twelve actions over the lowered core                                                  |
| hole-tolerant lowering and goals    | **built** — the engine's `lower` and `goals` modules, with holes surfaced through the report envelope                                     |
| solver-side footprints              | **not built** — no constraint solver exists to have a trail; see [[typing-machine#Serialization and checkpointing]]                       |
| the derivation tree and its merger  | **not built** — the wire node exists in `gandr-surface-render-remote`, nothing constructs a tree                                          |
| per-node marks                      | **not built** — marks are carried in the report envelope, not attached to core nodes                                                      |
| debouncing, render budget, eviction | **not built** — no editor client exists to debounce for                                                                                   |

**The built granularity is the top-level item, and the design's is the syntax node.** That is the single largest divergence in this document and it is stated in full at [[#Granularity: the item and the node]].

## The loop

```text
edit
  │
  ▼
cold reparse ......... total; a tree always results, obligations become holes
  │
  ▼
structural diff ...... unchanged subtrees identified by content hash
  │
  ▼
checkpoint resolver .. the nearest checkpoint covering the unchanged region,
  │                    whose dependency footprint is still valid
  ▼
typing machine ....... resume from that checkpoint; re-type the changed region;
  │                    re-validate — never blindly reuse — what follows it
  ▼
derivation merger .... splice adopted nodes and re-typed nodes into one tree,
  │                    preserving identity where the program did not change
  ▼
render
```

**Each stage's output is the next stage's only input**, which is what makes the loop specifiable stage by stage and testable against a from-scratch run of the whole thing.

## Cold reparse

**Parsing is total and it is not incremental**, and both halves are deliberate.

Total means a tree always results.
Ill-formed input does not abort the parse: it produces **obligations** in the parser's severity-ordered taxonomy, and the lowering step turns the obligations that stand for missing terms into **holes** ([[#Holes]]).
The consequence is the one that matters for this loop — **the checker runs on every keystroke**, because there is no state of the buffer in which it declines to run.

Not incremental is a measurement rather than a gap.
Cold reparse of the largest corpus file sits well inside the latency budget, so the bookkeeping an incremental parser needs costs more than it saves at this scale; the parsing calculus's bounded-context locality is inherited but deliberately unexploited, and statement-level resync by content identity is available as an optimization if the measurement ever changes.
[[../surface-language/grammar#Performance and reuse discipline]] owns that finding with its measured budgets, and [[../surface-language/operators#What the substrate substitution retired]] records that the incremental-parsing claim an earlier statement of the checkpoint footprint leaned on was **withdrawn as untrue of this pipeline**.

**Unchanged subtrees are recognized by content, never by address.** Node identity is a merkle hash over the node's significant content, computed by `gandr-surface-syntax`, and identity is therefore reproducible across runs and across processes rather than being a property of one parse's allocation.
The alternative — a parser-carried node address — is unsound for this use twice over: an address is only a witness of _reuse_ rather than of _unchangedness_, and an address into a tree the pipeline does not retain is a pointer into freed storage.

The parse result the rest of the loop consumes carries exactly two things:

```text
ParseResult = {
  root        : Cst,                  -- always present
  obligations : Obligation list,      -- severity-ordered, statement-local
}
```

**The obligations do not gate typing.** They are diagnostics about the text, delivered alongside a tree that types regardless.

## The structural diff

The diff's job is to partition the new tree against the old one into what may be reused and what must be re-derived.

The design states that partition as three regions:

```text
diffAst(old, new) = {
  commonPrefix   : Node list,   -- unchanged leading subtrees
  changedSubtrees: Node list,
  commonSuffix   : Node list,   -- unchanged trailing subtrees, reuse candidates
}
```

Top-down comparison finds the first divergence, then the convergence, and calls everything between them changed.
Matching by content hash is what keeps this near-linear and reproducible, and it is what makes the matches golden-testable rather than dependent on a parse's internal state.

**The built diff computes a different partition, and the difference is a genuine improvement rather than a shortfall.** `gandr-surface-syntax`'s `diff` emits **matched subtree-root pairs plus unmatched roots on each side**, not a prefix-changed-suffix triple.
Its traversal is iterative and top-down; an equal `(kind, payload, hash)` triple at a root emits one match and **prunes the whole subtree** rather than descending into it; a differing interior node aligns its non-space children by longest common subsequence over the same triple and recurses only into aligned pairs; and ties in that alignment advance the new side first, so the result is deterministic.
Whitespace is not significant to the match, and malformed or unreadable nodes are reported conservatively as unmatched rather than silently aligned.

Two consequences follow, and both matter downstream.
A prefix-and-suffix partition cannot express an unchanged region in the _middle_ of an edited parent, so it under-reports reuse on exactly the edit shape an editor produces most often.
And a match set with explicit unmatched roots on both sides is what an **edit script** can be derived from, which a triple of regions cannot support.

**That edit script is built**, in `gandr-surface-engine`'s `edit` module, as twelve localized actions over the lowered core: three that edit the top-level item list (insert, delete, and change of ascription), three coarse ones that install a whole subtree at a path (replace, fill a hole, erase to a hole), and six fine ones that change a leaf in place (an integer, a variable, a grade, a side, a binding, an annotation).
The three coarse actions are applied identically and kept distinct only because the distinction — a hole _filled_ against a term _erased_ to a hole against a constructor _replaced_ — is precisely the signal an agent-facing consumer wants.

## Checkpoints and the reuse rule

A checkpoint is a machine state tagged with the program position it covers **and with the dependencies its result rests on**:

```text
Checkpoint = {
  nodeId     : node_id,
  state      : MachineState,
  deps       : {
    tyvars     : tyvar list;     -- unification variables this result mentions
    gradeVars  : gradevar list;
    trailDepth : nat;            -- solver choice-point depth when taken
    stepId     : step_id;
  },
  derivation : DerivationNode,
}
```

### The soundness condition

**A checkpoint is valid for reuse exactly when three conditions hold**, and the second is the one an implementation is likeliest to omit:

1. its program position lies in the region the diff reported unchanged;
2. no variable in its footprint — type or grade — has been re-assigned since the step at which it was taken;
3. the solver trail has not been popped below its recorded depth.

**Condition two is what a naive design lacks, and its absence is a soundness bug rather than a performance one.** The substitution, the constraint worklist, and the grade budgets are _global_ to a check.
So re-typing a changed subtree can resolve a variable that a cached result elsewhere silently mentions, and that cached result is then stale while looking untouched.
Blind reuse of an unchanged prefix or suffix is unsound; validated reuse is not.

**Condition three is what makes reuse compatible with backtracking.** A checkpoint taken inside a speculative search region — the region a union or intersection subtyping decision opens — has no meaning once that region is abandoned, so it dies with it.
The test is one integer comparison, against the **watermark** the choice point records: the step at which its speculative region opened.

Conditions two and three are the machine's own, and [[typing-machine#Serialization and checkpointing]] states them in the machine's register with the footprint record beside them; condition one is this document's, because it is the diff that establishes an unchanged region and the machine has no notion of one.

### Strategy

* **Save on frame push and pop boundaries**, indexed by program position, so lookup is a single map hit.
* **Record the footprint as the check proceeds** — the solver already knows which variables each constraint touched, so the footprint costs bookkeeping rather than analysis.
* **Invalidate by watermark on backtrack**, which costs one integer comparison per live checkpoint rather than a traversal.

### This is not the arena's staging overlay

Two mechanisms in this tree are called checkpointing, they are named alike, and they are not the same thing.

**The one specified here guards a speculative solver region by trail depth.** The other is the **transactional staging overlay on the normalizer arena** ([[performance-architecture#The machine cribs, ported to the L machine]]): writes taken during speculative elaboration go to an overlay that is folded in on success and discarded on failure.
One decides whether a cached typing result may be believed; the other decides whether an arena write survives.
The hazard is specific and worth naming: a reader who takes one for the other concludes that reuse is already specified, and neither document says otherwise on its own.

### Granularity: the item and the node

**The design checkpoints syntax nodes; the tree checkpoints top-level items.** Neither is a refinement of the other, and a reader who assumes the built form is simply a coarser version of the designed one will get the footprint wrong.

| aspect                | as designed                                                      | as built                                                                 |
| --------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------ |
| unit                  | a syntax node                                                    | a top-level item, lowered                                                |
| identity              | the node's identity in the diff                                  | the item's name and ascription, with its lowered term as the content key |
| unchanged-region test | membership in the diff's unchanged region                        | structural equality of the lowered term against the edited item          |
| footprint contents    | type variables, grade variables, a trail depth, a step id        | the free names the term read from the ambient context, plus two flags    |
| invalidation trigger  | a footprint variable re-assigned, or the trail popped past depth | a name in the footprint whose binding the edit changed                   |
| conservative fallback | not stated                                                       | an **opaque** footprint reads everything and is never adopted            |

The two flags are worth naming individually because each buys a specific safety property.
**Opaque** is set when the footprint scan meets a core form it cannot express as a read set — a reified stack, an identity form, a declared-data or native form — and an opaque footprint is treated as reading everything, so its item is never adopted.
**Holey** is set when the term carries a hole, and the incremental layer declines to cache a typing for such an item.

**The built condition two is the design's condition two read one stratum up**, and the reading is exact: the shared, edit-mutable state threaded across items is the name-to-type context rather than a substitution, so "a variable this result mentions was re-assigned" becomes "a name this item read had its binding change".
The payoff is the same one the design argues for — an edit that changes a definition's _body_ but not its _type_ leaves every dependent's footprint untouched, so the dependents are adopted rather than re-typed.

**Item granularity is what the current architecture exposes, not a decision against node granularity.** Items lower independently and are typed against an accumulating context threaded item to item, which is exactly the structure a per-item checkpoint needs; a per-node checkpoint additionally needs the machine's stack to be resumable part-way through an item, which [[typing-machine]] specifies and nothing yet drives.

### The changed-region seam

The detector and the checkpoint engine read **lowered items only** — an optional name, an optional ascription, and a lowered core term — and never a concrete front end.
`gandr-core-incrementality`'s `region` module names that boundary, and the consequence is that the unchanged-region test is parser-agnostic by construction: it is structural equality over data that carries no surface syntax, no byte ranges, and no parser identity.

`gandr-surface-engine` supplies the real producer, in its `item_source` module; the incrementality crate ships a test double so the reuse machinery is exercised without one.

## The edit loop

```text
on_edit(text):
  1. parse   = reparse(text)                    -- total; a tree always results
  2. ast     = lower(parse.root)                -- obligations become holes
  3. diff    = structural_diff(old_ast, ast)
  4. if diff is empty: return

  5. cp      = nearest_valid_checkpoint(diff)   -- the three conditions above
     state   = if cp then resume(cp.state) else init(ast)

  6. for each subtree in diff.changed:
       state = type_subtree(state, subtree)     -- may move the substitution,
                                                -- the grades, and the trail

  7. for each node in diff.unchanged_after:     -- revalidation, not blind reuse
       cached = checkpoints[node]
       state  = if cached and deps_untouched(cached.deps, state)
                then adopt(state, cached)       -- splice the cached result
                else type_subtree(state, node)  -- re-check; usually hits inner
                                                -- caches that ARE untouched
  8. derivation = merge(state)
  9. render(derivation); old_ast = ast
```

**Step seven is the whole design in one line: the region after the edit is _revalidated_, not adopted on faith.** The common case — an edit inside one definition, with no cross-definition inference — touches no shared variable, so every candidate adopts and the step costs the size of the diff.
The pathological case — an edit that re-solves a variable the whole file mentions — degrades to a full re-check.

**That degradation is the correctness property, not the performance failure it looks like.** The alternative to re-checking is answering quickly and wrongly, and a loop that runs on every keystroke would then be wrong on every keystroke until something else forced a full check.

**Re-checking a node whose outer footprint was invalidated is cheaper than it sounds**, because the invalidation is rarely uniform: the outer result depended on a variable that moved, while the inner results usually did not, so the re-check hits caches that are still valid on the way down.

### The differential gate

Reuse is a claim about equality with a computation that was not performed, so it gets a differential rather than a test suite.

**The theorem is that for every edit, a validated resume yields exactly the per-item typings a from-scratch re-type of the edited program yields.** Adoption skips work; the gate proves the skips never change the answer.
It is realized twice as `tests/incremental.rs` — parser-free in `gandr-core-incrementality`, and over real source through the item seam in `gandr-surface-engine` — over four edit classes: adoption (a body-only edit adopting a type-stable dependent, an insertion adopting its untouched neighbours, a no-op adopting everything), invalidation (a type-changing edit re-typing every downstream reader, and a downstream type error surfacing exactly as it does from scratch), structural item-list edits, and property-generated edits.

This is the standing gate the incrementality lane carries ([[roadmap#Phase residuals worth pinning]]) and the second half of the ninth stage's acceptance criterion ([[feature-staging#stage-09]]).

## Derivation merging and identity stability

```text
merge(state) -> DerivationTree
  -- adopted nodes keep their original identities
  -- re-typed nodes take fresh identities
  -- orphaned checkpoints are dropped
```

**Identity stability is a user-visible property, not bookkeeping.** A derivation panel has expanded and collapsed nodes, a selection, and a scroll position, and all of those are keyed by node identity.
So an unchanged part of the program must come back with the identity it had, or the interface resets under the user on every keystroke.

The rule is therefore three-way: unchanged keeps its identity, new gets a fresh one, deleted is removed along with the checkpoints that referenced it.

**Half of this is built, in an unexpected place.** The engine's resume maintains the item order on the order-maintenance structure: base checkpoints seed one order element per item keyed by the item's stable identity, and an edit is applied by **splicing** — deleting removed elements and inserting new ones after their predecessors — so a matched item **keeps its original order element even as insertion and deletion shift every index around it**.
That is identity stability across an edit, realized for items.
The dirty-frontier pass then runs in that order, and because dependencies flow forward through the item list, one ordered pass propagates every binding change.

The derivation tree itself is not built: [[typing-machine#Derivation tree construction]] specifies the node, `gandr-surface-render-remote` carries its wire projection, and nothing yet constructs the tree those two describe.

## Holes

**A hole is a term with a typing rule, not a parse failure with a placeholder.**

```text
Γ ⊢ ?hole ⇑ α        -- fresh α; no constraint emitted
Γ ⊢ ?hole ⇓ A        -- succeeds for any A
```

Because a hole emits no constraint, it **never pollutes the substitution**, and that is what makes the rule safe to apply on every keystroke: a half-typed lambda body yields a partial derivation with a hole at the leaf rather than a cascade of errors invented by the checker's own guesses.
The editor shows the syntactic diagnostic; the derivation shows the typed surroundings with the hole marked.

This is what makes "do not type-check malformed code" unnecessary as a rule and incoherent as a design: there is no malformed input, only input with holes in it.

**As built, the rule is present with a different unknown.** `gandr-core-checker`'s recursive checker gives a value hole and a computation hole each a case in both directions, and it returns a **distinguished unknown type** rather than a fresh unification variable.
The two coincide while nothing solves constraints; they diverge the moment something does, because a fresh variable can be _solved_ by a later constraint and a distinguished unknown cannot.
Recorded here as a divergence to reconcile when the solver lands, not as a defect.

**The incremental layer declines to cache a typing for an item that carries a hole.** That is a policy about reuse rather than about typing — a holey item is one the user is mid-edit in, so its typing is the least likely of any to survive the next keystroke — and it is not the rule above narrowed.

### Type errors

A type error is a **position in a derivation**, not a message: the machine yields the error paired with the frame stack at failure and the contexts at that point, which is to say with the partial derivation, and a renderer draws it with the failing frame marked ([[typing-machine#Error handling]]).
Editing resumes from the last valid checkpoint _before_ the error, so an error does not cost the work that preceded it.

### Graceful degradation

If the incremental path fails structurally — a checkpoint store that cannot be read, or a diff that covers the whole tree — the loop performs a full re-type behind a brief indicator, and falls back to non-stepping evaluation if stepping cannot keep up.
**Degradation is to a slower correct answer, never to a stale one.**

## Performance targets

The budget the loop is designed against, edit to render:

| operation                        | target              | note                                         |
| -------------------------------- | ------------------- | -------------------------------------------- |
| cold reparse                     | under 1 ms          | measured well inside budget at corpus scale  |
| structural diff                  | under 1 ms          | linear in tree depth with hash pruning       |
| checkpoint lookup and validation | O(1) plus footprint | a map hit; footprints are small              |
| one typing step                  | under 1 ms          | first-order pattern match                    |
| revalidation after the edit      | O(diff) typical     | O(file) worst case, on shared-variable edits |
| derivation render                | under 16 ms         | virtualized; must fit a 60 hertz frame       |
| whole loop                       | under 50 ms         | the acceptance criterion                     |

**Debouncing:** editor input at 300 milliseconds, animation at frame rate, checkpoint saves batched per completed frame.

**Memory:** the checkpoint cache is linear in program size with least-recently-used eviction, derivation nodes are dropped when unreferenced, and a full reparse reclaims the previous revision's storage.

None of this half is built, because nothing consumes the loop at frame rate yet.
[[feature-staging#stage-09]] carries the fifty-millisecond figure as the stage's acceptance criterion.

## Resumable and shareable state

```text
state = machine.checkpoint()   -- first-order data, serializable
-- persist, or transmit
machine.resume(state)
```

**The whole machine state is first-order data** — frames, finite-map contexts, a trail list, and the dependency footprint — so a checkpoint is a value that can be written to a file, put on a clipboard, or sent over a link, and resumed elsewhere.
Cross-session sharing of an in-progress check follows from that and needs no additional mechanism.
[[typing-machine#Execution modes]] carries the mode vocabulary this uses, of which `resume` and `checkpoint` are the two this loop drives.

## The read-evaluate loop

The interactive loop is the same pipeline with a prompt in front of it:

```text
1. read      -- multi-line; holes make partial input typeable
2. reparse, lower, diff
3. resume from the last validated checkpoint
4. if the typing is complete  -> report the type; optionally evaluate
   if the typing is partial   -> show the partial derivation; prompt to continue
5. persist the session state
```

**The linear zone is what makes the loop's state rule non-obvious.** The ordinary and shared contexts, and module bindings, carry over from one entry to the next.
The **linear zone does not**: an endpoint left unconsumed when the prompt returns is an error, exactly as it is at the end of any other scope, because the alternative would be a prompt at which linearity quietly means nothing.

[[proposed/interactive-surface]] owns the surface this loop presents and the renderer firewall it sits behind; [[feature-staging#stage-09]] carries the same rule as the stage's fourth component.

## The literature this design is answerable to

Eight bodies of work bear directly on this pipeline, and this project has taken a position on each.
They are recorded as decisions rather than as citations because each one **changes what gets built**: two are adopted outright, one is adopted in part behind a stated gate, one is adopted only as an analogy, one is declined with a reversal condition, one is harvested for two named pieces, one is held pending a comparison, and one is the acknowledged ancestor of the whole scheme.

### pipeline-decision-01

**The marking discipline is adopted.** An ill-typed program still has a typed reading, and an error is a **mark on the offending node** rather than an abort of the derivation [@zhao-maroof-dukkipati-blinn-pan-omar-2024-total-type-error].

The reason it belongs here rather than only in the diagnostics story is that marks compose with checkpoints and aborts do not: **an error that localizes leaves the rest of the derivation reusable, while an error that truncates destroys every checkpoint after it.**

The line this comes from is the closest living relative of the whole application and is treated as work to mine rather than to cite: the structure-editor calculus in which every editor state has a well-defined typing [@omar-voysey-hilton-aldrich-hammer-2017-hazelnut], its dynamic semantics for incomplete programs [@omar-voysey-chugh-hammer-2019-live-typed-holes], and the marked calculus above.

**Owed, and stated as an obligation rather than as a gap:** per-node marks do not exist in this tree.
Marks are carried in the report envelope that [[inspection-protocol#The lightweight channel]] describes, and attaching them to core nodes is a prerequisite for the reuse property this decision is adopted for.

### pipeline-decision-02

**Incremental bidirectional typing over an order-maintenance structure is adopted as an additive layer, with a gate.** The work re-types _marked_ programs under fine-grained edits using order-maintenance structures and binding pointers, propagating updates as a small-step dynamics over the marked and annotated program; it proves its system **equivalent to naive re-analysis** in Agda alongside further metatheory, and reports a 275.96-fold speedup over from-scratch re-analysis on a stress test [@porter-kirisame-wei-panchekha-omar-2025-incremental-bidirectional].
Those three facts are verified at the paper's abstract; the design record's own naming of the mechanized properties as validity, convergence, and termination is **not** confirmed there and is carried as the record's.
Its order structure is borrowed from incremental page layout [@bender-cole-demaine-farachcolton-zito-2002-maintaining-order; @kirisame-wang-panchekha-2025-spineless-traversal].

**What is taken:** the order-maintenance intervals, which give constant-time term containment and logarithmic lowest-binder lookup and are the order structure this pipeline otherwise lacks; the per-node layout of a dual type, a boolean mark, and a dirty bit; **binding pointers**, which replace the coarse path from a binder through the substitution to its downstream readers with a direct binder-to-occurrence edge, so a structural rebinding becomes atomic while type propagation stays coupled to the solver; and the unchanged-type optimization.

**What is kept:** the validated checkpoints and the solver-side footprint, both unchanged.

**The correction this decision carries, because it is the part most easily got backwards.** An earlier framing had this work subsuming the context-threading half of invalidation.
It does not, and the reason is structural: **the ambient context is not in the footprint at all.** The footprint is solver-side, and contexts are threaded through frames.
What is subsumed is the structural checkpoint-coverage logic and the coarse binder-to-substitution-to-reader invalidation path; constraint solving is excluded from that work **by construction**.

**The gate, which is a soundness condition and not a scheduling note — and which is _this project's_ inference rather than a caveat the paper states.** The system that work proves its results about is a **bidirectional type system with no constraint store**, so its metatheory is established in that setting and says nothing either way about a solver-coupled one.
The conclusion drawn here is the conservative one: the dirty-frontier propagation engine is adopted as sound only over a characterized solver-free bidirectional-local fragment, and hands off to footprint invalidation at any solver-coupled step.
**Attributing that restriction to the paper would be the error to avoid**, since it is a fact about where the results were proved rather than a limit the authors record.
Characterizing the fragment is the seam this decision depends on, and it is unbuilt.

**The order-maintenance structure is deliberately its own crate rather than a member of the shared graph substrate**, because it is a specialized ancestry index rather than a relation over dense identifiers, and that boundary is recorded on the substrate's side too ([[graph-substrate#The boundary: what the substrate is not]]).
This pipeline's own dependency graph — the footprints and the dirty frontier — is the substrate's business and is named there as a future client, not a current one.

**What is built of this decision:** the order-maintenance structure entire, as `gandr-theory-orders` — a payload-carrying total order over generation-checked handles with constant-time comparison, insertion amortized at the square of the logarithm, and a density cap that makes relabeling always succeed so capacity exhaustion is a typed error rather than a panic — plus the interval-containment query over it, and the engine's use of it to keep item identity stable across an edit.
The two-level refinement to constant amortized insertion is deliberately deferred.

### pipeline-decision-03

**Co-contextual typing is not adopted, and the reason is not that it is worse.** Instead of threading a context downward — which makes every cached result depend on its ambient prefix — each subtree synthesizes **context requirements** that merge upward, so subtree reuse is sound by construction and the memoization key is just the subtree [@erdweg-bracevac-kuci-krebs-mezini-2015-cocontextual].

**It is the more principled fix.** It is declined here because it **replaces** the machine-checkpoint architecture rather than repairing it, and the stack-shaped derivation interface this whole track is built to serve is driven by that architecture.

**Reversal condition, stated so this is a decision rather than a preference:** revisit if footprint invalidation proves too coarse in practice — that is, if the pathological full-re-check case turns out to be the common case rather than the rare one.

### pipeline-decision-04

**The type-diff structure editor is harvested, not adopted.** It keeps a program well typed **by construction** through edits by propagating type diffs over one-hole contexts, and so never reparses and never re-checks [@prinz-blanchette-lampropoulos-2024-pantograph].

**It is a competing architecture rather than a component**, and it belongs in the same category as the previous decision: its premise is to avoid the parse-and-recheck front end this pipeline is built on, and its only edit inputs are structured one-hole-context operations, with text diffs explicitly excluded as an action source.

**Two things are harvested.** The typed error boundary, as a well-typed carrier for marks over the call-by-push-value types this tree uses, which never truncates the reusable derivation.
And the type diff as a composable typed delta — a first-class object rather than a procedure.

Its results are hand-proved rather than mechanized, which is recorded because the previous decision's are mechanized and the two are otherwise easy to weigh alike.

### pipeline-decision-05

**Total tile-based parsing with material obligations is already this project's parsing calculus**, so this decision records a lineage rather than an import: malformed input is completed with explicit obligations naming what is missing, rather than with error nodes [@moon-blinn-porter-omar-2025-tylr].

[[../surface-language/grammar#The obligation taxonomy]] owns the taxonomy; what this document takes from it is the guarantee the loop's first stage depends on — **a tree always results** — and the fact that the obligations are severity-ordered data rather than a failure signal.

### pipeline-decision-06

**The collaborative structure-editor calculus is held, on two axes, and the hold has a comparison attached.** Edits form a commutative replicated data type over a labeled multigraph, and conflicts — including relocation conflicts — are **explicit, typed editor states** with totality of marking [@adams-griffis-porter-satish-zhao-omar-2025-grove].

The two axes are a structure-editor interface for this application, and a merge-protocol treatment of multi-user interactive state.
**Neither is scheduled**, and the standing instruction attached to the hold is that this work is compared before any structure-editor direction is taken, precisely because the previous two decisions also propose structure editors and the three differ in what they make explicit.

### pipeline-decision-07

**A demand-driven incremental driver paired with a streaming server is the closest system-level reference for this pipeline, and its spine is adopted as an analogy.** The system in question is a proof assistant under active development whose build layer is a demand-driven incremental driver over a content-addressed cache and whose language server streams diagnostics **and subgoal displays** to its client.
It is the only reference here that converges on the _whole_ pipeline — driver plus streaming server — rather than on a marking or structure-editor calculus, and it is independent corroboration that a streaming checker is the centerpiece rather than an optimization.

**Locator-pending, and marked here rather than silently normalized.** This is a live project rather than a publication: it is identified by its author, Jon Sterling, and by its two components' behaviour, and its primary page is access-blocked, so the claims below are consistent paraphrases from reachable secondary material rather than verbatim primary source.
No bibliography entry is minted for it, because the corpus mints entries only for works with a verified locator.

**The adoptable spine, four items:**

* the demand-driven query graph with content-addressed keys, **as an analogy to the validated-checkpoint footprint and not as an adoption** — this tree's globally mutable solver state has no pure-functional early-cutoff equivalent, so what transfers is the commitment to **keep positions out of the checkpoint key**, exactly as `rust-analyzer` keys its queries on position-independent item indices because offsets move after every edit [@rust-analyzer];
* relative positioning in the syntax overlay, which becomes the resync described at [[#pipeline-question-01]];
* a thin streaming server, re-targeted from an editor protocol to an agent-facing stream, whose first version's seeds are the goal report and the diagnostic report that already exist;
* a reactive-first posture with batch behaviour derived from it, rather than the reverse.

**The provenance correction, which is load-bearing because getting it wrong would credit the wrong layer.** Relative _widths_ on the green half of a red-green syntax tree are standard practice in the lineage of that design [@roslyn; @rowan] and are not that project's contribution; stock red trees in both implementations carry _absolute_ positions.
The documented divergence is making the **red** overlay relative to the root, so that an edit to leading whitespace does not invalidate the whole suffix.
This tree imports neither red scheme: its order structure is the previous decision's, and its only new contribution here is the resync below.

**Three drops, each with its reason:** the particular content-addressed build engine it uses [@llbuild2fx], because the model is what is wanted and the demand-driven query lineage behind the language servers is the better-tested validation target [@salsa; @rust-analyzer; @roslyn]; the file-prefix granularity ceiling, because this tree's global substitution and grade budgets are an independent granularity hazard, which makes finer-than-file granularity a probe rather than a given; and the object theory, which is a different language.

### pipeline-decision-08

**The checkpoint-and-footprint scheme is a re-derivation of incremental attribute evaluation, and the ancestry is acknowledged rather than discovered** [@reps-teitelbaum-demers-1983-incremental-context-dependent].
In this document's vocabulary, a footprint is the dynamic dependency edges an analysis consulted, and a validated checkpoint is a cached attribute reused exactly when those dependencies are untouched.

**Marked at the claim:** this work is cited bibliographically and characterized in this project's own words; **the primary source has not been opened for this document**, so nothing here quotes or paraphrases it, and the correspondence above is a claim this project makes rather than one the source states.

## Open items

### pipeline-question-01

**Should the order-maintenance points be bound to the content-addressed syntax tree's node identity, and what does that buy?**

The resync would rebase the origin mapping onto order-maintenance keys so that a whitespace-only reparse leaves positions invariant.
Two distinct queries are involved and conflating them is the trap: the interval's constant-time operation is **node-to-node ancestry**, which is what a dirty frontier needs, while locating a node from a byte offset is a **stabbing query**, sharpenable to the depth of the tree by descent but not to constant time by the same structure.

**Status: designed, unbuilt, and deliberately waiting.** Whether the adapter pays for itself depends on the dirty-frontier consumer that would use it, and that consumer is gated by [[#pipeline-decision-02]]'s solver-free fragment.

### pipeline-question-02

**Is footprint invalidation too coarse in practice?**

This is [[#pipeline-decision-03]]'s reversal trigger stated as a measurement rather than an intuition.
The instrument exists: the differential gate already reports, per resume, how many items were adopted rather than re-typed, so the adoption rate over a realistic edit corpus is the number that would settle it.

### pipeline-question-03

**What does the checkpoint key become when granularity moves below the item?**

The built form keys on an item's name, ascription, and lowered term, which is available because items lower independently.
A sub-item checkpoint has no equivalent content key — a subtree's typing depends on its ambient context — so moving below the item either adopts the binding-pointer machinery of [[#pipeline-decision-02]] or adopts the upward-merging requirements of [[#pipeline-decision-03]].
**The granularity question and the architecture question are therefore the same question**, which is the reason to answer them together rather than in sequence.

### pipeline-question-04

**What replaces the derivation merger's identity rule when the derivation tree is finally built?**

The item order already gives stable identity across an edit for items, and the machine's monotone step counter gives node identity within one check.
Neither is yet the merger's three-way rule, and the merger is where a mismatch between them would surface.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot incremental-pipeline design record** in full — its dataflow, the parser integration, the diff, the checkpoint record and its three validity conditions, the strategy, the edit algorithm, the derivation merger, the hole rules with their error and degradation handling, the performance table with its debounce and memory policies, the resumable-state claim, the read-evaluate loop, and every one of its literature dispositions.
2. **The tree**, for every as-built claim: the `diff` module of `gandr-surface-syntax`; the `edit`, `lower`, `goals`, `item_source`, and `boundary` modules of `gandr-surface-engine`; the `checkpoint`, `footprint`, and `region` modules of `gandr-core-incrementality`; the `checker` module of `gandr-core-checker`; `gandr-theory-orders` entire; and the `tests/incremental.rs` differential in both crates that carry one.
3. **[[typing-machine]]**, of which the checkpoint record and the state this loop resumes are the machine's own; every condition stated in both places is stated once here and linked.
4. The **corpus documents the coverage sweep credited with this record** — the implementation track's surface-pipeline account and the performance architecture — re-measured against it, with the result that neither carries it and the sections that do are linked from the claims they genuinely carry.

**Confidence, by class.**

* **High** — the loop's stages, the three validity conditions, the hole rules, the performance targets, and the literature dispositions, all transcribed from the design record rather than re-derived; and every as-built claim, each read from a definition rather than from a doc comment.
* **Medium** — the two-directional divergence table, which is a comparison this document draws rather than one either side states.
* **Marked at the claim** — the streaming-driver reference of [[#pipeline-decision-07]], whose primary source is access-blocked, and the ancestry of [[#pipeline-decision-08]], whose primary source has not been opened.
* **Checked at the abstract only** — [[#pipeline-decision-02]]'s source, whose title, authorship, mechanization, speedup figure, and setting were read from its abstract on 2026-08-02, and whose body was not opened; the gate this document draws from that setting is marked in place as this project's inference rather than the paper's.
