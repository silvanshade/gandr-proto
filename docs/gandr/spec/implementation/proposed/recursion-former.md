# The recursion former

**Proposed.
No fixpoint former exists in this tree.** The kernel's computation vocabulary has six formers and none of them binds a self-reference, so a program that recurses cannot be represented, let alone run.

This document fixes the **one core former user-level recursion needs** — a computation fixpoint whose self-reference is a thunk — together with its checker rule, its operational rule, its machine realization, the memoization argument that keeps a divergent fixpoint from being cached, and the boundary that keeps it distinct from the linear feedback wheel it superficially resembles.

The surface that opens a recursive scope is already designed and partly built, and it is [[../../surface-language/recursion|the (co)recursion surface]]'s subject, not this one.
This document is what that surface lowers **into**.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The kernel's term vocabulary has no fixpoint.** `gandr-kernel-core`'s `term` module declares `Computation` with exactly six formers — `Lambda`, `Application`, `Return`, `Bind`, `Force`, `Case` — and `Value` with seven, of which `Thunk` is the only one that embeds a computation.
  Its `types` module declares `CompType` with exactly two formers, `Returner` and `Arrow`.
  Nothing in either enum binds a name in its own body.
* **The thunk is graded upstream and ungraded in the kernel.** `gandr-core-checker`'s `syntax` module carries `Value::Thunk(Grade, …)` and `ValueType::Thunk(Grade, …)`; `gandr-kernel-core`'s `ValueType::Thunk` takes only the computation type, because grades are erased before the certified stage and survive only in the export format.
  `Grade` itself is a sealed newtype in `gandr-core-checker`'s `grade` module whose entire cross-module surface is a semiring signature — `ZERO`, `ONE`, `OMEGA`, `fin`, `leq`, `plus`, `times`.
* **Call-by-need forcing, with black-holing, is built and running.** `gandr-core-sequent`'s `store` module declares `MemoState` as `Unforced | InProgress | Forced` behind an interior-mutable `Cell` with nominal identity; its `machine` module's `force`, `force_inline`, and `force_probe` implement the probe-and-write-back discipline this document's memoization argument depends on.
  The memo policy is uniform across grades.
* **The step budget is one shared constant.** `gandr-core-checker`'s `outcome` module declares `STEP_BUDGET` as `1_000_000` and documents itself as the single source of truth: the L machine and the shell and foreign-interface drivers all run the same budget.
* **The recursive-scope surface pass runs before lowering.** `gandr-surface-engine`'s `lower/recursion_surface` module resolves which bare names are fix-bound and classifies instantiation-slot residents, item by item, before ordinary lowering sees the expression.
* **A recursive definition does not lower.** In `gandr-surface-engine`'s `lower` module, a `def rec` whose body is a **copattern clause list** is treated as a codata introduction and elaborates to a `Cosplit` record of thunks; a `def rec` whose body is a **statement block** — the user-recursion case — falls through to the total-mode hole.
  So the surface parses, scope-checks, and then declines.

**Designed, and not built.** Everything else here: the former itself, its checker and operational rules, its machine realization, the recursive-definition elaboration, the derived terminating eliminators, and every rung of the termination ladder past the budget.

**One consequence of the two-stage split worth stating before the rules, because the source design could not have anticipated it.** The pre-reboot design had one core term language with graded thunks, so "which language does the former live in" was not a question.
This tree has two — the checker's graded language and the kernel's erased one — and the former's self-reference is graded, so the former's home is a **genuine ambiguity this document inherits rather than a formality it neglected**.
It is stated as one, with its readings, at [[#recursion-former-question-06]] below.

## The design space

Recursion is the one facility the pure call-by-push-value core deliberately withholds [@levy-cbpv].
The fragment without it is strongly normalizing, so **general recursion is an addition rather than a derivation**, and the addition has four axes.

**Where the recursive knot lives.** Call-by-push-value threads recursion through the thunk/returner adjunction: a self-reference of computation type `B` is available only as a value of thunk type `U B`, forced at each use.
The alternative — a value-level fixpoint of a function — needs a value-level function type the calculus does not have, since `Arrow` is a computation type whose domain is a value type.
**The thunk route is forced by the substrate**, and it is the right one.

**A former or an encoding.** A dedicated fixpoint former, or a polymorphic builtin combinator over the existing opaque-primitive node that unrolls in Rust.
The combinator avoids touching the certified vocabulary but makes recursion an **axiom** and hides the binder.
Resolved under [[#A former, not a builtin combinator|"A former, not a builtin combinator"]].

**How the surface marks it.** Explicit marking against implicit self-reference detection.
This axis is settled and built on the surface side: the marker is required at the **declaration** as a recursive scope and again at **every recursive occurrence** as a direction sigil, and an unmarked self-reference is a hard error carrying the marked spelling as structured data ([[../../surface-language/recursion#The decision: a hybrid of two sites]]).
What this document owes is the elaboration that marker opens.

**How iteration reads.** Loop constructs are surface sugar over the former and over bounded native folds, and the honest question is only what `break` and `continue` desugar to.
Resolved under [[#Loops elaborate through the former|"Loops elaborate through the former"]].

Two boundaries constrain every choice.
**The machine iterates**, so the former must not reintroduce host-stack recursion; and **a cartesian trace is already a fixpoint**, so the linear zone's feedback wheel is fenced off the value model rather than made to serve as recursion ([[#Recursion is not a feedback wheel|"Recursion is not a feedback wheel"]]).

## The former

One computation former, binding one self-reference:

```text
t ::= … | fix x. t              recursion (x : U_ω B bound in t : B)
```

The self-reference `x` is a **graded thunk** at grade `ω`: every use of the recursion is `force x`, and a force is a machine step.

**The thunk delay is the guard.** Each self-use is separated from the definition by a genuine step, so there is no infinite regress at a single point — the same causality principle a dataflow feedback loop's unit delay supplies, met here in the value zone.

**Guardedness in this sense is well-formedness, not termination.** `fix x. force x` is well-formed and diverges; termination is [[#Termination|"Termination"]]'s subject, and at the current rung it is unchecked.

**The grade is `ω` because a recursive call forces the knot an unbounded number of times.** A self-reference at grade `1` would type only tail and linear recursion — which is a real refinement, and it is on the growth ladder rather than in the cut, because the checker rule generalizes to any grade above one without changing shape.

## The checker rule

The former is **check-primary**, like the other type-directed introducers: the self-binding needs the computation type in order to state the self-reference's type, so the type must arrive from the context rather than be synthesized from the body.

```text
    Γ, x : U_ω B ⊢ t ⇐ B
  ─────────────────────────  (fix, checking)
    Γ ⊢ fix x. t ⇐ B

    Γ ⊢ (fix x. t : B) ⇒ B     (inference by ascription)
```

Inference is available only through the ascription coercion, and the recursive definition's declared signature is what supplies it in practice.
**Both directions are exercised**, per the both-directions discipline the checker's soundness tests already follow.

**The former adds no subtyping seam.** It introduces no atom relation, no row relaxation, and no grade relaxation, so it needs no coherence obligation of its own.
Its differential obligation is [[#The differential obligation|"The differential obligation"]]'s subject, and it wants a **productive-fixpoint generator strategy** in the conformance suite — a generator biased toward fixpoints that reach a weak-head normal form, because an unbiased one spends its whole budget on divergence.

## The operational rule

The specification oracle is substitution: the former unfolds by replacing its self-reference with a thunk of the whole fixpoint.

```text
fix x. t   ⤳   t[ thunk_ω (fix x. t) / x ]
```

Every `force x` in the unfolded body re-enters the fixpoint.
**This rewrite does not terminate in general, deliberately** — it is the specification, and the machine is what bounds it.

## The machine realization

The L machine realizes the same rule **without host-stack recursion**, which is what the iterate-don't-recurse policy requires and what the differential then has to reconcile.

Forcing a fixpoint thunk pushes the body with the environment extended by binding the self-reference to a **first-order recursive closure** — an environment paired with the fixpoint's own node identity, never a host closure.
Forcing that closure **re-enters the same node in the same environment**.

**Knot-tying is by re-entry, not by a heap cycle**, and that is the load-bearing property rather than an implementation preference: a cyclic heap value would make dropping non-structural, whereas re-entry leaves the store's ownership discipline exactly as it is.

Each unfold is one machine transition, so **recursion depth is bounded by the step budget and never by the Rust stack**.

The substitution reading and the environment re-entry **differ in shape and agree in result**, which is precisely the divergence the machine's differential is built to tolerate: it compares outcomes, not call graphs.

## A former, not a builtin combinator

The existing opaque-primitive node could host a combinator of type `U_ω (U_ω B → B) → B` that unrolls in Rust.
That avoids touching the certified vocabulary and would land sooner.
**It is declined as the core form**, and recorded as the challenged alternative rather than as a dead one.

Three objections, and each is about **visibility** rather than about typing — granting the combinator a schematic type, as the list primitives effectively have, answers none of them.

| the objection              | what an opaque unroll costs                                                                                                                                    |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **the metatheory**         | a builtin is an axiom typed by its declared scheme, so the mechanized face gets an opaque constant where it most needs a fixpoint rule to state theorems over  |
| **the termination ladder** | size-change analysis, a cost effect, and derived eliminators all have to **read the recursion's structure**; an unroll hides the binder from every one of them |
| **the inspectable term**   | the derivation surface and the rewriting layers address nodes, and a visible binder un-sugars and renders where a Rust unroll does not                         |

**The reversal condition, stated so the decline stays cheap to reopen.** The combinator survives as a possible interim if a runnable recursive surface is wanted before the former's certified-stage change can be taken.
That is a schedule trade, not a design one, and taking it obliges recording that the metatheory, termination, and inspection consequences above are accepted for the interim's duration.

## The recursive-definition elaboration

A recursive definition wraps its body in the former over the definition's own name, so that in-body calls resolve to forces of the self-reference:

```text
def rec f(x: A) -> B { body }
  ⤳  def f = thunk_ω (fix f. fn (x: A) { body[ f(e) ↦ (force f)(e) ] })
```

**The elaboration records its provenance**, as every other desugaring in the surface does, so a diagnostic un-sugars back to the recursive definition the author wrote rather than reporting the fixpoint.

**Why the marker is explicit is settled on the surface side, and the reason worth restating here is the scoping one.** An ordinary definition binds a value with no self-scope; implicit self-reference detection would change that scoping rule for every definition, and it would do so with a shadowing hazard — whether a self-mention denotes the definition being written or an outer binding of the same name becomes a question the reader cannot answer locally.
The surface answers it by making the unmarked occurrence an error rather than a silent resolution, which is strictly stronger than the marker the pre-reboot design proposed and dissolves the hazard rather than documenting it.

**Mutual recursion elaborates to one fixpoint over a bundle.** A recursive block opens one scope for the group, and the group desugars to a single fixpoint over a record of the mutually recursive thunks, each member projecting its sibling — the standard reduction of mutual recursion to a single knot.
The surface for it is designed and parse-and-decline today; the elaboration is gated on the record former and on this document's former, and neither is a retrofit of the other.

## Structural recursion

Structural recursion at the current cut is **the general former plus case analysis** over declared constructors — no derived eliminators.
A list length is written directly, and the recursion is the former's while the elimination is the data surface's:

```text
def rec len(xs: List(Integer)) -> F Integer {
  case xs { Nil => ret 0, Cons(h, t) => run n <- len[<](t); ret plus(1, n) }
}
```

**The growth path is derived terminating eliminators.** Once declared data is available as convergent presentations, a catamorphism or paramorphism can be **derived per declaration**: an intrinsically terminating eliminator whose recursion is structurally guarded, giving checked structural recursion **without** the step budget.

That is the bridge from the unchecked former to the checked-termination rungs, and it is the surface track's roadmap item as well as this document's.

## Loops elaborate through the former

The loop constructs are surface sugar producing a unit returner, spliced into bind chains as statements.
Their surface reading and their desugaring targets are [[../../surface-language/recursion#Loops, and the break/continue discipline|the surface document's]]; what belongs here is the **bounded/unbounded split** and the one elaboration that is too dense to state in a row.

| surface               | elaborates to                                                                             | why                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `for x in e { body }` | a native fold over `e`, the body a thunk applied per element                              | a concrete collection is finite and iterates natively — bounded, no fixpoint |
| `while c { body }`    | `fix self. force c >>= b. case b { false => ret unit \| true => body >>= _. force self }` | genuinely unbounded — needs the fixpoint                                     |
| `loop { body }`       | `while true { body }`                                                                     | the always-true case                                                         |

Iteration over a finite collection therefore **stays off the fixpoint path entirely**, which is both the common case and the ergonomic one; only the unbounded forms incur the budget.

**The escape operations are effect operations, and the reason is a property of the delimited-control fragment.** The reset/shift pair is single-prompt — a capture always reaches the nearest delimiter — so it cannot give an outermost exit and an innermost continue distinct targets in one loop.
Algebraic handlers are keyed by operation name, so they can, and the desugaring installs two of them:

```text
break     ⤳  perform Break unit          continue  ⤳  perform Continue unit

while c { body }  ⤳
  handle (fix self. force c >>= b. case b {
            false => ret unit
          | true  => handle body { Continue _ k => ret unit } >>= _. force self })
         { Break _ k => ret unit }
```

**The nesting is the whole mechanism.** The escape operation is caught only by the **outer** handler, because the inner handler lists the continue operation alone and lets the other propagate past it; catching it discards its continuation and ends the loop.
The continue operation is caught by the **inner, per-iteration** handler, which discards the rest of the body so control falls through to the next iteration.
Bounded iteration wraps the same two handlers around the native fold and around the per-element body thunk respectively, so the two operations mean the same thing in both loop forms.

**Discarding a continuation is sound at the current rung because the linear zone is vacuous**, and it stays sound when it is not: sessions and stack-owned obligations inherit the discard-runs-unwind discipline for free, since the escape operations already route through the handler mechanism that owns it.

**Labeled multi-level exit is deferred.** Unlabeled escapes target the nearest loop, so a nested escape leaves only the innermost.
Labels need distinct operation **instances** — a fresh atom per labeled loop, or named handlers — and the label sigil collides with the character-literal lexer, so the spelling is an open surface question and not only an elaboration one.

## Memoization, black-holing, and what a divergent fixpoint cannot do

The hazard a reader expects here is that a recursive thunk memoizes divergence and serves a bogus value forever.
**The machine already answers it, and the former adds no mechanism** — it makes an existing, deliberately built path reachable.

**The discipline as built.** Forcing a thunk consults its cell.
A cached weak-head form is reused.
An in-progress cell — the **black hole** — means a re-entrant force, and the machine falls back to running the body inline against the live continuation.
An unforced cell is marked in progress and the body is **probed** on a nested machine at the empty continuation; the probe caches only a **pure, continuation-free terminal**, and on anything else it clears the black hole and runs the body inline instead.
The probe carries its own step counter against the shared budget and **declines rather than halting** when it exhausts it.

Three consequences follow, and the third is the one that is easy to state backwards.

**A divergent fixpoint is never mis-cached.** The cell is written only when the probe actually reaches a weak-head normal form.
A body that never reaches one never writes the cell, and the outer step counter is what ends the run.

**A productive fixpoint caches once and the black hole never fires.** Its body reaches a lambda before it forces itself, so the probe completes, the weak-head form caches, and every later force reuses it.
That is the efficient path, and it is observationally the same as unfolding, because the cached form is continuation-independent by construction.

**The specification is unfold-to-fresh; sharing the weak-head form of a productive fixpoint is a growth optimization the same black hole makes safe.** Stating it the other way round would make the optimization load-bearing for correctness, which it must not be.

**One doc-sync obligation lands with the former, and it is precise.** `gandr-core-sequent`'s `store` module documents the in-progress state as "unreachable in v0 (no recursion), where a genuine cycle degrades to the shared step budget", and `gandr-core-checker`'s `outcome` module documents the step-limit stuck reason as "not reachable on the bounded, non-recursive v0 fragment".
**Both are true today and both become false the moment the former lands**, so correcting them is part of landing it rather than a follow-up.

## The differential obligation

The pre-reboot design owed this former a totality obligation against a **direct-style recursive reference evaluator**, which realized the operational rule by substitution and could not report divergence because it had no budget.
That evaluator no longer exists: the reference machine was physically removed once the migration closed, and `gandr-core-sequent`'s `differential` module now compares the live machine's outcome against **frozen outcome snapshots** that captured the final oracle-agreeing run.
The canonicalization is unchanged; its reference is a fixture rather than a second machine.

**So the obligation transforms rather than lapsing, and it transforms in a way that makes it cheaper.** There is no second implementation to keep total over divergent input.
What the former owes instead is that its outcomes are **canonicalizable** — a divergent fixpoint must reach the step-limit stuck outcome rather than any other stuck reason, and a productive one must return a terminal the readback carries to a commuting normal form like any other.

**The unsupported-by-reference stuck reason is not a hole to route the former into.** It now names a former the focusing translation does not yet realize, and a fixpoint the translation does not realize is an unfinished port, not a defined outcome.
Recording that distinction here is the point: the name is inherited from the retired reference machine, and its meaning changed underneath it.

## Recursion is not a feedback wheel

The former is **cartesian value and computation recursion**.
The feedback wheel of the circuit layer is **linear hiding in the resource zone**.
They are the same causality principle — a delay makes feedback well-defined — at two different zones, and they are kept as **distinct constructs** because the firewall between the zones depends on it.

| axis               | the fixpoint former                                              | the feedback wheel                                                     |
| ------------------ | ---------------------------------------------------------------- | ---------------------------------------------------------------------- |
| **zone**           | value and computation, cartesian                                 | linear wires in the resource zone only                                 |
| **self-reference** | a grade-`ω` thunk — duplicable, forced any number of times       | one linear hidden wire, threaded sequentially                          |
| **semantics**      | unfold to a fresh thunk; unrestricted general recursion          | interface-arity dissolution with structural retention                  |
| **delay is**       | the force step                                                   | the wire's unit delay                                                  |

**The bridge is the cyclic-sharing result: a cartesian trace _is_ a fixpoint** [@hasegawa-1997-recursion-cyclic-sharing].
That is exactly **why** the circuit layer fences wheels out of the cartesian zone — a value-zone wheel would _be_ general recursion, smuggled past the value model — and it is why general recursion belongs in the value and computation zone explicitly, as this former.

This document touches no circuit shape, adds no wheel marker, and leaves the firewall intact.

**This boundary was previously absent from the corpus, and its absence was found by a reviewer rather than by an author.** The circuit-cells document records an adversarial finding that a claimed fixpoint-versus-wheel firewall "exists nowhere in the corpus" ([[../../surface-language/circuit-cells]]).
It exists here now, and it rests on the cited theorem rather than on how this project happens to have arranged its zones — which is what makes the fence a fact about the machinery, and therefore load-bearing, rather than a convention that could be relaxed by renaming something.

## Termination

Termination at the current rung is **unchecked**, guarded operationally by the shared step budget.
A non-terminating program **halts** at the budget with a stuck outcome; it does not hang, and it does not overflow the host stack.

**The budget is one constant, and the pre-reboot design's synchronization hazard is retired rather than carried.** That design had the budget written twice — once in the evaluator and once in the shell driver — and it named keeping the two in sync as a standing obligation and planned a corpus example to pin it.
The migration de-duplicated them: the constant now lives in `gandr-core-checker`'s `outcome` module and every driver reads it.
**The obligation is retired with its cause, and the example that would have pinned it is retired with it** — an example asserting that two constants agree is vacuous when there is one constant.

Until a checked rung lands, a recursive definition is honestly a partial definition, and **the marker discipline is the acknowledgement**: the declaration says the definition is self-referential and every occurrence says which direction it claims.

The ladder past the budget, each rung designed-in and none in the cut:

| rung                       | what it checks                                                                                                           | what it needs first                                       |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| **structural termination** | derived terminating eliminators over declared data — recursion that cannot diverge because the eliminator is guarded     | declared data as convergent presentations                 |
| **guarded corecursion**    | the direction sigils become checked claims: descent into an inductive argument, production under a copattern observation | recursive lowering, and both checks total on the fragment |
| **a cost effect**          | a cost bound becomes a typed program, turning the budget from a runtime net into a static obligation                     | the effect layer, plus the design pass named below        |
| **sized types**            | both sigils elaborate to erased size instantiations strictly below the ambient size                                      | a deliberate sized-types design pass                      |
| **solver-assisted**        | measure and decreasing-argument checking through the solver interface                                                    | a surface for termination measures                        |

The guarded and sized rungs are **the surface track's ladder**, stated there with their go and no-go conditions ([[../../surface-language/recursion#The productivity ladder]]); they appear here because the former is what they eventually constrain.

**The cost-effect rung descends from a named line of work whose locator this corpus does not hold.** The pre-reboot design named the cost-as-effect calculus and its directed refinement — a step-counting effect and a directed inequality with a boundedness predicate — as the vehicle.
That naming is carried; **no bibliography key is minted for it, because neither work is held here and a citation from recall is what the corpus's citation discipline exists to prevent.**

## The current rung and the growth ladder

| capability               | current                                          | growth                                                                  |
| ------------------------ | ------------------------------------------------ | ----------------------------------------------------------------------- |
| **core form**            | none — the vocabulary has no fixpoint            | `fix x. t` with the self-reference at grade `ω`                         |
| **grade of the knot**    | not applicable                                   | grade `1` for tail and linear recursion, giving guaranteed reuse        |
| **recursive surface**    | scope-checked and declined at lowering           | single recursive definitions, then mutual blocks over a bundle          |
| **structural recursion** | not expressible                                  | general fixpoint plus case, then derived terminating eliminators        |
| **loops**                | keywords reserved and grammar-admitted           | bounded folds and unbounded fixpoints, unlabeled escapes, then labels   |
| **termination**          | unchecked; no fixpoint exists to diverge through | the ladder above                                                        |
| **memoization**          | built, with the recursive path unreachable       | the same discipline, with the path reachable and the comments corrected |

## Interactions

* **Data and patterns.** The case eliminators structural recursion consumes belong to the data surface; the derived terminating eliminators are a **joint** obligation with it, since they are generated per declaration.
* **Effects and control.** The loop escapes are algebraic operations on the handler machinery; the delimited-control pair stays reserved for user-level control and is untouched.
* **Value semantics and modes.** The grade-`1` tail refinement and the discard-runs-unwind path both touch the mode and linearity calculus ([[../../surface-language/proposed/modes-and-references]]); neither is in the cut.
* **The kernel's certified stage.** Adding a former to the innermost stage is the most expensive kind of change available here, and the graded-versus-erased split makes the placement question real rather than procedural.
* **Self-hosting.** User recursion is a prerequisite for writing gandr in gandr; the gate scripts themselves need no core recursion, so this unblocks the **language user** rather than the bootstrap.

## The example plan

Landing the former lands these, in the runnable example corpus.
**They are landing evidence, not residual work**, and the split between them is the corpus's own model-versus-pathological one.

**Model examples — fully commented, learn-by-example.**

| example                 | what it witnesses                                                                     |
| ----------------------- | ------------------------------------------------------------------------------------- |
| factorial               | the canonical single self-recursion; un-sugars to a thunk of the fixpoint             |
| fibonacci               | tree recursion — re-entry with no cross-unfold memoization needed for correctness     |
| list length             | structural recursion by fixpoint and case, with the derived-eliminator path previewed |
| bounded iteration       | iteration over a collection folding natively — the off-the-fixpoint path              |
| iteration with continue | the inner per-iteration handler                                                       |
| countdown with break    | the unbounded form and the outer handler                                              |
| nested recursion        | depth tied explicitly to the budget                                                   |

**Pathological examples — the stress subtree.**

| example                        | what it pins                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------- |
| divergent fixpoint             | halts at the budget and **never writes a cached value** — the no-mis-memoize witness              |
| deep recursion                 | depth on the order of a hundred thousand succeeds without host-stack overflow                     |
| productive beside unproductive | one fixpoint caching its weak-head form beside one that never reaches it — the black-hole witness |
| escape outside a loop          | an unhandled escape operation, rejected rather than silently ignored                              |
| nested loops, unlabeled escape | the escape leaves the innermost loop only — the limitation, documented by a program               |

**One planned example is retired with its reason**, and the reason is not that it was hard: an example pinning that the evaluator's budget and the shell driver's budget agree tested a synchronization that the single-constant de-duplication removed.

## Open questions

### recursion-former-question-01

**Whether canonicalization needs a divergence-specific arm.** A divergent fixpoint reaches the step-limit stuck outcome; the frozen-snapshot differential compares outcome kind, stuck reason, and returned value.
Nothing more is obviously owed, but no snapshot in the current set can exercise a divergent run, because no divergent program can be written without the former.
**Disposition: carried**, as an obligation on the change that lands the former — the first divergent snapshot is written by that change, not discovered later.

### recursion-former-question-02

**The grade of the self-reference beyond `ω`.** Whether a grade-`1` tail-recursion mode is worth surfacing at the first cut, for guaranteed reuse, or strictly later.
The checker rule generalizes to any grade above one without changing shape, so the cost is the surface and the analysis rather than the rule.
**Disposition: parked, with its reason** — the mode is a refinement of an unbuilt former, and refining something unbuilt is how a cut grows without anyone deciding it should.

### recursion-former-question-03

**The shape of a bounded iteration that produces a value.** Whether iteration over a collection desugars uniformly to a unit-returning fold, or whether a body ending in a value should desugar to an accumulating reduction, making the loop an expression.
**Disposition: declined for the first cut, with a reversal condition** — the unit fold plus an explicit accumulator is proposed, and the accumulating form reopens if the explicit accumulator turns out to be the common case in the example corpus rather than the exceptional one.

### recursion-former-question-04

**Whether the escape operation carries a payload.** The proposed operations carry unit.
Carrying a value would make a loop an expression rather than a statement, and the operation's payload type is where it would live.
**Disposition: declined for the first cut, with a reversal condition** — the payload type is designed in, and the question reopens together with the value-producing iteration above, since the two share one motivation: making a loop yield something.

### recursion-former-question-05

**Interaction with gradual holes.** Whether a fixpoint body containing a hole type-checks with the self-reference's type unknown, and whether an interactive session evaluates a hole-free recursive term as usual.
Holes are expected to be orthogonal, and the machine already blames a computation hole rather than getting stuck.
**Disposition: carried**, to be pinned by an example rather than argued — an expectation is not a finding.

### recursion-former-question-06

**Which term language the former is added to — and this one is genuinely ambiguous rather than merely unanswered.**

The former's self-reference is a **graded** thunk, and that grade is load-bearing: it is what distinguishes a recursion that may be entered an unbounded number of times from one that may not.
This tree has **two** term languages, and the grade survives in only one of them — the checker's syntax carries a graded thunk in both its term and type vocabularies, while the certified stage's thunk type takes only the computation type, because grades are erased before that stage and survive only in the export format.

Three readings, none of them currently coherent:

| reading                                                | what it asks, and why it does not close                                                                                                                           |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **the graded language only**, erasing to nothing below | the certified stage would then be unable to type a recursive program at all, which cannot be the intent of a stage whose whole purpose is to re-derive everything |
| **both languages**, ungraded below                     | what does the certified stage check _in place of_ the grade? The grade is the only thing distinguishing bounded from unbounded entry, and erasing it erases that  |
| **grade erasure itself moves**                         | coherent, and the most expensive: erasure is a commitment of the two-stage split rather than a representation choice inside it                                    |

**Disposition: carried as an inherited ambiguity, and recorded as one.** This is **migrated content**: the design it comes from was written against a single core term language with graded thunks, so the question could not arise there and no answer to it exists in the source.
The ambiguity is a product of the migration meeting a tree the source predates, not an omission in either.

**What is owed is reconciliation with the current direction**, not archaeology.
Whoever schedules the former decides it, deliberately and with the two-stage split's own commitments in view; nothing here provisionally picks a reading, because a provisional pick reads as a decision to the next reader and this is exactly the kind of decision that should not be made by momentum.

## Source and confidence

The design descends from the pre-reboot recursion-and-iteration design record, absorbed here as a superset: its decision table, its four design axes, its rules, its elaborations, its memoization argument, its zone boundary, its ladder, and its example plan are all carried, and its five open questions are each dispositioned above.

**The as-built account is high confidence and was read from definitions rather than from prose**: the term and type vocabularies, the grade carrier, the memo cell and the force path, the shared budget constant, the recursion-surface pass, and the lowering's treatment of a recursive definition were each verified against the named module in this tree.

**Three claims in the source did not survive that verification and are recorded as changes rather than carried.** The reference evaluator the differential obligation was written against no longer exists, so the obligation is restated against frozen snapshots.
The budget's two-place mirror was de-duplicated into one constant, so its synchronization obligation and the example that would have pinned it are retired.
And the single graded core the source assumed is now two languages, which is [[#recursion-former-question-06]] — the one place where absorbing the source raised a question instead of answering one, recorded there as an inherited ambiguity owing reconciliation with the current direction rather than as an open item nobody reached.

**The surface half of the source is not repeated here.** Its scope discipline, its call-site evidence markers, and its productivity ladder were superseded in presentation by a later design and live in [[../../surface-language/recursion|the (co)recursion surface]]; the loop desugarings are stated in both places because the surface owns their reading and this document owns their target.
