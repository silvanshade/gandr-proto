# The typing machine

The typing machine is the **step-wise defunctionalized form of the bidirectional checker** whose rules are [[type-system]].
It exists so that type checking is not a black box: the typing stack is a data structure rather than the host call stack, the solver is a separate machine with its own inspectable state, and the whole state is serializable and resumable.

That is not an incidental benefit.
Everything downstream — the derivation renderer, incremental re-checking, time-travel over a check, and the inspection protocol that ships derivations to a renderer — is only possible because the machine's state is data, and each of those consumers is specified against the state shape below.

## What is built

`gandr-core-checker` carries **both** realizations: a recursive bidirectional checker in its `checker` module and the defunctionalized machine in its `machine` module, and the two are property-tested for step-for-step agreement on a control log.
A third, total _marking_ realization is oracle-bound to the recursive one.

**The differential proves the two agree, not that either is correct** — a shared soundness bug leaves them agreeing on the wrong answer — which is why the suite is supplemented by directed coherence oracles and a standing adversarial pass before substantial checker changes.

The machine's frame inventory is built for every feature the checker builds and absent for every feature it does not; the inventory below carries a status per frame.
The **solver as a separate machine is not built**, because nothing yet emits a constraint that needs one: subtyping is decided structurally on the spot, as [[type-system#What the tree actually decides]] records.
Serialization, checkpointing, the execution modes, and the derivation-tree builder are specified here and consumed by [[inspection-protocol]], which carries the wire projection of the derivation forest and the control register.

## The method, which is the point

The construction follows the **functional correspondence between evaluators and abstract machines** [@ager-biernacki-danvy-midtgaard-2003-functional-correspondence]: start from the recursive bidirectional checker, transform it to continuation-passing style, then **defunctionalize** the continuations [@reynolds-1972-definitional-interpreters; @danvy-nielsen-2001-defunctionalization] into an explicit stack of frames.

Defunctionalization is what turns the checker into a state machine in which:

- the **typing stack is a data structure** — a list of frames, one constructor per recursive call site of the checker;
- the **solver is a separate machine**, with its own state and its own step relation;
- the **entire state is inspectable, serializable, and resumable**.

**The method is a commitment, not merely a description of how the first version was written.** The machine should be _derived_ from the recursive checker rather than hand-authored, and both implementations kept in the tree with property tests asserting step-for-step agreement.

The reason is specific: the derivation makes every fresh-variable allocation explicit and **mechanically prevents the classic specification bugs** of a hand-written abstract machine — mismatched frame payloads, lost continuations, and inconsistent stack conventions.
Every one of those is a bug that type-checks, so nothing but the derivation catches them.

**The same pipeline was intended for the evaluator**, sharing this frame infrastructure and its renderer hooks.
The evaluator that was built instead is the polarized command intermediate language's machine ([[../implementation#The checked language]]), which supersedes the design that would have shared these frames; the _method_ transfers even though the frames did not.

## The recursive checker, before defunctionalization

```text
infer_value : ctx → world → value → (vty,  out) result
check_value : ctx → world → value → vty → out result
infer_comp  : ctx → world → comp  → (cty,  out) result
check_comp  : ctx → world → comp  → cty → out result

solve : worklist → subst → trail → subst result
```

The `out` component collects **emitted constraints and linear-context consumption** — the two things a typing rule produces besides a type.

The continuation-passing version threads a continuation `k : ty → out → answer` through each recursive call, and **those continuations are the typing stack**.
The frames below are that continuation type, defunctionalized.

## Machine state

### Control

Control is an **explicit register**, which is the standard abstract-machine shape:

```text
type dir     = Infer | Check of ty        -- checking carries its expected type
type layer   = Val | Comp

type control =
  | Descend of expr * dir * layer         -- about to type a sub-expression
  | Return  of ty                          -- a sub-expression's type, propagating up
```

**Carrying the expected type inside `Check` is what makes every rule's inputs explicit**, and it removes a class of pseudo-frame that an earlier design needed in order to remember what a subtyping obligation was against.
An earlier design also matched a nullable result field against the stack instead of carrying a register; the register is what replaced it.

The direction type is built and carries its expected type exactly as above.
The renderer's own projection of the control register adds an idle sentinel that is **deliberately not one of the machine's control states** — [[inspection-protocol#The frame envelope]] records that divergence, and a reader comparing the projection against the machine should expect it.

### The state record

```text
type state = {
  control   : control;
  stack     : frame list;        -- HEAD IS TOP OF STACK, uniformly
  ctx       : ctx;               -- Γ : x ↦ (vty, world)
  kctx      : kctx;              -- Δ : X ↦ kind
  shared    : shared_ctx;        -- Θ : a ↦ (shared_session, world)
  linear    : linear_ctx;        -- Σ : endpoints + capabilities, world-located
  world     : world_id;
  solver    : solver_state;      -- the worklist, substitution, grade environment, trail
  deriv     : deriv_builder;
  steps     : step_id;           -- monotone counter
}
```

The four context zones are the four of [[type-system#The full judgment]], one field each.

**Frames record the context _deltas_ they introduced** — bindings added, linear resources split off — so popping a frame restores the intuitionistic and linear zones exactly.
That is what makes a state checkpointable **without snapshotting whole contexts per step**, and it is the same argument that makes the derivation tree cheap below.

## The frame inventory

One constructor per pending obligation, with payloads that are exactly what the resumed rule needs.

**The inventory is carried whole, with a status per frame, and the reason is worth stating.** The built frames are in `gandr-core-checker`'s `machine` module and are current there; the designed frames belong to features the checker does not build, so no source carries them and a specification is the only place they can live.
Splitting the inventory on that line would mean re-partitioning it every time a feature lands.

**The design record's own frames number forty-three, not the forty-one an earlier count recorded**: twenty in the core block and twenty-three for features outside it.
The undercount is stated because it is the sort of figure a later reader would take on trust, and it is checkable by counting the two blocks below.

### Designed core frames

These are the core block of the derivation, in the form the design record fixes.
Every one of them is **built**, though several are built under a different name or with a payload the sections below record.

```text
type frame =
  | KAbs        of var * vty * dir            -- λ: body pending; yield A → B, unbind
  | KAppFn      of value * dir                -- application: head pending; argument stored
  | KAppArg     of cty * dir                  -- application: argument check pending; yield B
  | KPairFst    of value * dir * dir          -- pair: first pending; second + its dir stored
  | KPairSnd    of vty * dir                  -- pair: second pending; first's type stored
  | KInj        of vty                        -- checked injection: payload pending; yield the sum
  | KSplit      of var * var * comp * (var * cty) option * value * dir
                                              -- split: scrutinee pending; body, optional motive,
                                              -- and the scrutinee value stored (for the answer)
  | KSplitBody  of cty * dir                  -- split: body pending; carries the precomputed answer
  | KCaseScrut  of (var * comp) * (var * comp) * cty  -- scrutinee pending (case is check-only)
  | KCaseArm1   of (var * comp) * vty * cty   -- arm 1 pending; arm 2 + right summand stored
  | KCaseArm2                                 -- arm 2 pending; pop restores Γ
  | KThunk      of grade * dir                -- thunk body pending; yield U_r B
  | KForce      of dir                        -- thunk value pending; yield B from U_r B
  | KRet        of dir                        -- ret argument pending; yield F A
  | KBind       of var * comp * dir           -- bound computation pending; continuation stored
  | KBindBody                                 -- bind continuation pending; pop restores Γ
  | KWith1      of comp * cty                 -- lazy pair: first pending; second + its type stored
  | KWith2      of cty                        -- lazy pair: second pending; first's type stored
  | KPrj        of side * dir
  | KAnnot      of dir                        -- annotation: check pending; finish in dir
```

**Frames do not carry continuation pointers — the stack itself is the continuation.** An earlier design embedded a `next` frame inside each frame _and_ kept a frame list, which encodes the continuation twice and lets the two disagree.

### Designed frames for features the checker does not build

Twenty-three frames, one block per feature.
**None of these exists in the tree**, and none can until its feature lands.

```text
  -- set operations
  | KInterAgain of comp * cty                 -- ∩I: re-check the same term at the second component
  | KUnionBr2   of var * comp * cty * vty     -- ∪E: re-check the continuation under the 2nd disjunct

  -- polymorphism
  | KGen        of tyvar                      -- Λ body pending; yield ∀X. B
  | KInst       of vty                        -- instantiation: head pending; substitute on return

  -- sessions (binary and multiparty share frames; the role is absent for binary)
  | KSendVal    of chan * role option * comp * dir   -- payload check pending
  | KSessCont   of unit                              -- continuation after a session action
  | KOfferBr    of chan * (label * comp) list * cty option   -- remaining branches
  | KForkChild  of chan * session * (chan * comp) * dir      -- child pending, parent stored
  | KForkParent of unit
  | KMCutRole   of global_ty * (role * comp) list * (role * comp) * dir

  -- sharing
  | KAcquire    of shchan * var * comp * dir
  | KRelease    of shchan * comp * dir
  | KShForkBody of shchan * shared_session * comp * dir

  -- worlds
  | KHold       of world                      -- hold body pending; yield @w A
  | KLeta       of var * comp * dir           -- leta: package value pending
  | KMigrate    of world * world              -- (destination, saved source world)

  -- modules
  | KModSelect  of label
  | KModSeal    of sig_ty
  | KModFunBody of sig_ty
  | KModApply   of mod_expr
  | KModPack    of sig_ty
  | KModUnpack  of sig_ty * var * comp * dir
  | KImplicit   of vty * fuel
```

The module frames serve [[../surface-language/proposed/modules#The typing rules]], whose own account of what it owes the machine should be read against this block.

### The built inventory

`gandr-core-checker`'s `machine` module carries **forty-four** frames.
Names are the crate's, which drop the `K` prefix; payloads are named fields rather than positional.

| block                 | frames                                                                                                                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| core                  | `Abs`, `AppFn`, `AppArg`, `PairFst`, `PairSnd`, `Inj`, `Thunk`, `Force`, `Ret`, `Bind`, `BindBody`, `CaseScrut`, `CaseArm1`, `CaseArm2`, `Split`, `SplitBody`, `With1`, `With2`, `Prj`, `Annot` |
| grades                | `Dup`, `Drop`                                                                                                                                                                                   |
| the value model       | `Ctor`, `List`, `Record`, `RecordProj`, `DataCaseScrut`, `DataCaseArm`, `ListCaseScrut`, `ListCaseNil`, `ListCaseCons`                                                                          |
| effects and control   | `Perform`, `HandleScrut`, `HandleRet`, `HandleOp`, `ResumeFn`, `ResumeArg`, `ResetBody`, `ShiftBody`, `StkArg`, `StkBind`                                                                       |
| the identity fragment | `Here`, `WalkScrut`, `WalkBase`                                                                                                                                                                 |

**The built set is not the designed core set plus extras — it diverges in both directions**, and reading it as an implementation of the designed block alone would miss half of it.
The designed core block is present entire.
Twenty-four further frames serve features the design record's inventory predates: the value model's literals, lists, records, and declared data; the two grade operations; the effect and control block; and the identity fragment.
And every one of the twenty-three frames in the previous section is absent.

Two payload details are worth carrying because they are where a reader would expect the crate to match the design and it does not.
`BindBody` is **not** payload-free: it carries the bound computation's effect row, which is unioned into the continuation's returner at the pop, and the bind's own direction.
`SplitBody` likewise carries the precomputed answer rather than echoing the body's type.

### Two frame conventions

**Frames that complete a rule carry the originating direction**, so the inlined subsumption rule runs at the frame pop.
An earlier inventory carried the direction only where a stored sub-term would itself be typed in it, which leaves the completing frame unable to discharge its own obligation.

**Context-restoring frames are dedicated**, because the intuitionistic zone is scope-shaped: popping a restore frame unbinds the innermost hypothesis, which is the general delta discipline degenerated to one binding.

**Binder-introducing frames carry the bound variable** — `KAbs`, `KBind`, `KSplit`, `KCaseScrut`.
The reason is not the typing rule, which does not need the name: it is the **failure payload**, which renders the frame stack as a partial derivation and must be able to name binders.

### Six corrections the derivation forced

The core block above is what the derivation produced.
An earlier, hand-written inventory differed in six places, and each difference is a defect the derivation found rather than a matter of taste.

| earlier form                      | derived form                            | why                                                                                                                              |
| --------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| a session frame reused after bind | `KBindBody`                             | the core must not borrow a session-stage frame; the context-restore obligation is bind's own                                     |
| absent                            | `KSplitBody`, `KCaseArm2`               | the analogous restore frames for split and case were simply missing                                                              |
| `KCaseBr2 of (var * comp) * cty`  | `KCaseArm1 of (var * comp) * vty * cty` | case is check-only, so arm sequencing needs the second arm, its summand, and the expectation — never "the first branch's type"   |
| `KCaseScrut of … * dir`           | `KCaseScrut of … * cty`                 | case is check-only, so the frame stores the expected type rather than a direction                                                |
| absent                            | `KInj of vty`                           | the checked injection's pending payload had no frame at all, though the prose descended into it                                  |
| `KAnnot of ty`                    | `KAnnot of dir`                         | the ascription becomes the payload's checking direction at the descend, so the frame must keep the _outer_ direction for the pop |

## The step function

```text
step : state → outcome

type outcome =
  | Done  of ty                  -- control = Return τ, stack = []
  | Step  of state
  | Error of typing_error        -- with the partial derivation attached
```

`step` dispatches on the pair of the control register and the stack head.

**Notation.** `finish T in d` means: if `d` is inference, `Return T`; if `d` is `Check T′`, discharge `T <: T′` and `Return T′`.
This **is** the subsumption rule of [[type-system#Notation and judgment forms]], inlined, and it runs where the direction-carrying frame pops.

Where no metavariable exists the constraint is **decided on the spot**, which is observationally equivalent; once the worklist solver exists it is **emitted** to the worklist at the same point.
The tree is in the first state.

Every transition below is the image of the correspondingly named rule under the transformation, so the two documents are read side by side.

### Core

```text
-- Variable:
(Descend(x, d, Val), s)
  → ctx(x) = (A, w₀);  require w₀ = world
  → finish A in d                                    -- a leaf rule: Sub inline

-- Abstraction, checking mode, unannotated binder:
(Descend(λx. t, Check (A → B), Comp), s)
  → Step { control = Descend(t, Check B, Comp);
           ctx     = ctx + (x : A @ world);
           stack   = KAbs(A, Check (A → B)) :: s }

-- Abstraction, annotated binder, in ANY direction — always infer the body;
-- a checking direction is discharged by Sub at the pop:
(Descend(λx:A. t, d, Comp), s)
  → Step { control = Descend(t, Infer, Comp);
           ctx     = ctx + (x : A @ world);
           stack   = KAbs(A, d) :: s }

(Return B, KAbs(A, d) :: s)
  → ctx := ctx - x
  → finish (A → B) in d

-- Application: infer the function, THEN check the argument
(Descend(t v, d, Comp), s)
  → Step { control = Descend(t, Infer, Comp); stack = KAppFn(v, d) :: s }

(Return (A → B), KAppFn(v, d) :: s)
  → Step { control = Descend(v, Check A, Val); stack = KAppArg(B, d) :: s }
(Return T, KAppFn(v, d) :: s)   where T is not an arrow
  → Error ShapeMismatch("an arrow type", T)

(Return _, KAppArg(B, d) :: s)
  → finish B in d
    -- the check-mode constraint discharges at THIS pop; an earlier text placed it
    -- at the final Return, where no frame carried the expectation to discharge against

-- Thunk and force — the adjunction, correctly oriented:
(Descend(thunk_r t, Check (U_s B), Val), s)
  → require s ⊑ r              -- the thunk rule fused with Sub: U_r <: U_s needs s ⊑ r
  → Step { control = Descend(t, Check B, Comp);
           stack   = KThunk(s, Check (U_s B)) :: s }
(Descend(thunk_r t, d, Val), s)                      -- any other direction: infer
  → Step { control = Descend(t, Infer, Comp); stack = KThunk(r, d) :: s }
(Return B, KThunk(r, d) :: s)
  → finish (U_r B) in d

(Descend(force v, d, Comp), s)
  → Step { control = Descend(v, Infer, Val); stack = KForce(d) :: s }
(Return (U_r B), KForce(d) :: s)
  → emit the constraint 1 ⊑ r
  → finish B in d

-- Returner and bind:
(Descend(ret v, d, Comp), s)
  → Step { control = Descend(v, arg_dir d, Val); stack = KRet(d) :: s }
(Return A, KRet(d) :: s)
  → finish (F A) in d

(Descend(t >>= x. u, d, Comp), s)
  → Step { control = Descend(t, Infer, Comp); stack = KBind(x, u, d) :: s }
(Return (F A), KBind(x, u, d) :: s)               -- the frame CARRIES the continuation
  → Step { control = Descend(u, d, Comp);
           ctx     = ctx + (x : A @ world);
           stack   = KBindBody :: s }              -- the pop restores the context
(Return B, KBindBody :: s)
  → ctx := ctx - x
  → Step { control = Return B; stack = s }
```

### Set operations

```text
-- Intersection introduction (check only): check the SAME term at both components
(Descend(t, Check (B₁ ∩ B₂), Comp), s)
  → Step { control = Descend(t, Check B₁, Comp); stack = KInterAgain(t, B₂) :: s }
(Return _, KInterAgain(t, B₂) :: s)
  → Step { control = Descend(t, Check B₂, Comp); stack = s }
    -- the final Return yields B₁ ∩ B₂

-- Union elimination at bind (check only): the continuation is checked under both disjuncts
(Return (F (A₁ ∪ A₂)), KBind(x, u, Check B) :: s)
  → Step { control = Descend(u, Check B, Comp);
           ctx     = ctx + (x : A₁ @ world);
           stack   = KUnionBr2(x, u, B, A₂) :: s }
(Return _, KUnionBr2(x, u, B, A₂) :: s)
  → Step { control = Descend(u, Check B, Comp);
           ctx     = (ctx - x) + (x : A₂ @ world);
           stack   = … }                          -- the second pass, then Return B
```

**An injection in inference mode is a stuck error carrying a hint** to annotate.
In checking mode against a sum it descends into the payload under `KInj`, which **rebuilds the sum at the pop** — never inventing the other summand, which is what an earlier design did by updating the substitution with a summand inference cannot know.

### Polymorphism

```text
(Descend(ΛX. t, d, Comp), s)
  → Step { control = Descend(t, d, Comp); kctx = kctx + (X : *); stack = KGen(X) :: s }
(Return B, KGen(X) :: s)
  → Step { control = Return (∀X. B); stack = s }

(Descend(t [A], Infer, Comp), s)
  → Step { control = Descend(t, Infer, Comp); stack = KInst(A) :: s }
(Return (∀X. B), KInst(A) :: s)
  → kind-check A against X's kind
  → Step { control = Return (B[A/X]); stack = s }
```

### Sessions

Binary and multiparty differ only in the role indices and in session initiation.

```text
(Descend(send c v; t, d, Comp), s)
  → Σ(c) = !A.S @ w₀; require w₀ = world
  → Step { control = Descend(v, Check A, Val);
           linear  = Σ[c ↦ S];                      -- the protocol advances
           stack   = KSendVal(c, None, t, d) :: s }
(Return _, KSendVal(c, _, t, d) :: s)
  → Step { control = Descend(t, d, Comp); stack = s }

(Descend(recv c as x; t, d, Comp), s)
  → Σ(c) = ?A.S
  → Step { control = Descend(t, d, Comp);
           ctx     = ctx + (x : A @ world);
           linear  = Σ[c ↦ S]; stack = … }

(Descend(fork (c:S). t in c′. u, d, Comp), s)
  → split Σ into Σ₁ (the free variables of t) and Σ₂
  → Step { control = Descend(t, Check (F 1), Comp);
           linear  = Σ₁ + (c : S @ world);
           stack   = KForkChild(c, S, (c′, u), d) :: s }
(Return _, KForkChild(c, S, (c′, u), d) :: s)
  → Step { control = Descend(u, d, Comp);
           linear  = Σ₂ + (c′ : dual(S) @ world); stack = KForkParent() :: s }
```

**A session frame's pop verifies that the endpoint reached the state its rule requires** — that an endpoint closed by `close` had reached `end`, for instance.
Linear residue at `Done` is a linearity error listing the unconsumed endpoints, **and that is how a fidelity violation surfaces as a partial derivation rather than as a crash**.

### Sharing

```text
(Descend(acquire a as c; t, d, Comp), s)
  → Θ(a) = ↑ˢₗ S_L;  emit esync(S_L, ↑ˢₗ S_L)
  → Step { control = Descend(t, d, Comp);
           linear  = Σ + (c : S_L @ world);
           stack   = KAcquire(a, c, t, d) :: s }

(Descend(release c as a; t, d, Comp), s)
  → Σ(c) = ↓ˢₗ S_S
  → Step { control = Descend(t, d, Comp);
           linear  = Σ - c;  shared = Θ + (a : S_S @ world); stack = … }
```

### Worlds

```text
(Descend(hold v, d, Val), s)
  → Step { control = Descend(v, payload_dir d, Val); stack = KHold(world) :: s }
(Return A, KHold(w) :: s)
  → Step { control = Return (@w A); stack = s }

(Descend(leta x = v in t, d, Comp), s)
  → Step { control = Descend(v, Infer, Val); stack = KLeta(x, t, d) :: s }
(Return (@w′ A), KLeta(x, t, d) :: s)
  → Step { control = Descend(t, d, Comp); ctx = ctx + (x : A @ w′); stack = … }

(Descend(migrate_{w′} t, Infer, Comp), s)
  → consume cap_{w′} from Σ (a capability error if absent)
  → require loc(Σ used by t) ⊆ {w′}
  → Step { control = Descend(t, Infer, Comp); world = w′;
           stack   = KMigrate(w′, world) :: s }
(Return (F A), KMigrate(w′, w₀) :: s)
  → emit mobile(A)
  → Step { control = Return (F A); world = w₀; stack = s }
```

**The world register is saved in the frame and restored at the pop**, which is what lets a renderer draw the world badge changing across a frame boundary rather than inferring it.

## The solver as a separate machine

```text
type choicepoint = {
  queue        : constraint list;       -- the worklist at the choice
  sigma        : subst;
  grades       : grade_env;
  alternatives : constraint list list;  -- the untried branches
  watermark    : step_id;               -- for checkpoint invalidation
}

type solver_state = {
  queue  : constraint list;
  sigma  : subst;
  grades : grade_env;
  trail  : choicepoint list;            -- a STACK, not a single backpoint
}

type solver_outcome =
  | Resolved   of subst
  | Progress   of solver_state
  | NeedsInput                          -- the typing machine must emit more constraints
  | Failed     of constraint            -- the trail is exhausted
```

Processing order follows [[type-system#Transitions]] exactly:

1. **instantiation** — occurs check, then substitute **and apply the substitution to the whole queue**;
2. **structural decompositions and the invertible union and intersection rules**, eagerly;
3. **choice points** — push a trail entry and try the alternatives in order;
4. **grade order** — decide in the semiring;
5. **mobility, equi-synchronization, well-formedness, and projection** — regular-tree checks with a visited set over contractive recursive types;
6. **on failure** — pop the trail to the nearest choice point with untried alternatives.

**Two things about this state are corrections rather than refinements**, and both matter to anyone implementing it.
An earlier design kept a **single backpoint**, which cannot implement the search that union and intersection subtyping require — the trail is essential, not an optimization.
And the choice point's **watermark** is what makes backtracking compatible with checkpointing, by recording the step at which the speculative region opened.

## Serialization and checkpointing

The whole state is serializable: frames are first-order data, contexts are finite maps, and the trail is a list.
Each checkpoint additionally records its **dependency footprint**:

```json
{
  "nodeId": "ast-417",
  "state": { "control": "…", "stack": "…", "ctx": "…", "solver": "…" },
  "deps": {
    "tyvars":     ["α3", "α7"],
    "gradeVars":  ["ρ1"],
    "trailDepth": 2,
    "stepId":     1042
  }
}
```

**A checkpoint is reusable exactly when two conditions hold**, and the second is the one that is easy to omit:

1. no type or grade variable in its footprint has been re-assigned since it was taken; **and**
2. the solver trail has not been popped below its recorded depth — **a checkpoint taken inside a speculative region dies with that region**.

This is what enables resumable typing, incremental re-checking, shareable state, and time-travel debugging.

A third condition — that the checkpoint's program position lie in the region an edit left unchanged — belongs to the loop that drives this machine rather than to the machine, because it is a diff that establishes an unchanged region: [[incremental-pipeline#The soundness condition]] states all three together.

**This mechanism is distinct from the transactional staging overlay on the normalizer arena** ([[performance-architecture#The machine cribs, ported to the L machine]]), which is the incremental lane's checkpoint mechanism for _elaboration_.
The two are named alike and are not the same thing: one guards a speculative solver region by trail depth, the other folds arena writes on success.
The incremental lane's standing gate — incremental equals from-scratch — is what would catch either being wrong.

## Derivation tree construction

```text
type derivation_node = {
  id        : step_id;
  rule      : rule_id;            -- "Abs⇓", "∪E", "Acquire", "Migrate", …
  expr      : expr;
  direction : dir;
  layer     : layer;
  world     : world_id;
  ctx_delta : binding list;       -- what this node added, not a full snapshot
  linear_in : chan list;          -- the endpoints consumed below this node
  result    : ty option;
  children  : derivation_node list;
  protocol  : (chan * session * session) option;  -- before and after, for session badges
  grade     : grade option;
}
```

**Each frame pop closes a node, so the tree mirrors the stack discipline exactly** — which is why the tree needs no separate construction pass.

**Storing context deltas rather than snapshots is what keeps large derivations cheap**, and an inspector reconstructs any node's full contexts by folding the deltas along the path from the root.
Without it a derivation forest is quadratic in the context.

This node is realized as the inspection protocol's wire projection, field for field, with the machine's monotone step counter as the node identity — which is what makes a node stable enough to key a patch against.
[[inspection-protocol#The frame envelope]] owns that projection and the scalar digest beside it.

## Execution modes

| mode         | description                                 | use case                 |
| ------------ | ------------------------------------------- | ------------------------ |
| `eager`      | step to completion or error                 | batch checking           |
| `single`     | one step                                    | animation, live feedback |
| `batch(N)`   | N steps                                     | throttled updates        |
| `resume`     | load a state and continue                   | incremental re-checking  |
| `checkpoint` | save the state and its dependency footprint | the incremental cache    |

## Error handling

```text
type typing_error =
  | TypeMismatch    of ty * ty             -- expected, actual (Sub failed)
  | ShapeMismatch   of shape * ty          -- expected shape (a fixed description), actual
  | KindMismatch    of kind * kind
  | StuckExpr       of expr * hint option  -- no rule applies (hint: "annotate this injection")
  | UnboundVariable of string
  | WorldMismatch   of world * world       -- a hypothesis or endpoint used at the wrong world
  | GradeError      of grade * grade       -- the grade order failed
  | LinearityError  of chan list           -- unconsumed or duplicated linear resources
  | SessionError    of chan * session * action   -- an action incompatible with the protocol state
  | EsyncError      of session * session   -- an equi-synchronization violation
  | ProjectionError of global_ty * role    -- a global type not projectable (merge failed)
  | CapabilityError of world               -- migration without the capability
  | MobilityError   of ty * world          -- an immobile type crossing worlds
  | OccursCheck     of tyvar * ty
  | SolverFailure   of constraint          -- the trail is exhausted
```

**Exactly one constructor per failure mode.** An earlier inventory declared the kind error twice, which is the kind of defect that survives review precisely because both declarations read as correct.

**The shape mismatch is not a weaker type mismatch, and the distinction is forced rather than stylistic.** At an elimination whose principal premise infers the wrong _constructor_ — applying a non-arrow, forcing a non-thunk — a complete expected type does not exist without metavariables, since the expectation would have to be a type with holes in it.
So the expectation is reported as a **shape**: a fixed description rather than a type.

**Every error carries the offending expression, the frame stack at failure, and the contexts at that point** — which is to say it carries the partial derivation, and a renderer draws it with the failing frame highlighted.
That is the payoff of the whole construction: a type error is a position in a derivation rather than a message.

## Source and confidence

Written against five sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot typing-machine design record** in full — its method, the recursive checker's signatures, the machine state, the frame inventory with its conventions and corrections, the step function, the solver machine, the checkpoint footprint, the derivation node, the execution modes, and the error inventory.
2. **The tree**, for the built inventory and every other as-built claim: `gandr-core-checker`'s `machine`, `checker`, and `subtype` modules.
3. **[[type-system]]**, of which this machine is the operational image; every transition here names the rule it is the image of.
4. The **corpus documents said by the 2026-08-01 sweep to carry this record** — the implementation track's checker account, the inspection protocol, and the performance architecture — re-measured against the record and now linked from the claims they genuinely carry.
5. The **pre-reboot programme's tracker**, for the intent that the evaluator be derived by this same pipeline over this same frame infrastructure.

**Confidence, by class.**

- **High** — the frame inventory, the transitions, the solver state, and the error inventory, all transcribed from the design record rather than re-derived.
- **High** — the built-versus-designed partition, counted against the named module at write time.
- **Medium** — the three method citations, whose identifiers were transcribed from the contributor's reference register at this pass but whose _claims_ were not re-read from the papers.
