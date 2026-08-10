# The pluggable solver interface and its refinement domain

**Proposed.
No constraint-domain interface, no domain registration, and no refinement or SMT machinery exists in this tree.** What exists is the worklist solver those domains would plug into — specified in [[../type-system#Algorithmic subtyping and the worklist solver]], and built in its degenerate case — plus one decision procedure per constraint form, each written directly into the checker rather than behind an interface.

The design does two things.
It **names the interface that the solver's existing constraint forms already satisfy**, and opens that interface to registration.
It then adds the first heavyweight external domain: **SMT-backed refinement types**, with contracts as their surface sugar.

**Surfacing a solver as a language-level facility is a deep retrofit in most languages, and is not one here.** Elsewhere the solver is buried inside the typechecker, so exposing it means exhuming it.
In this system the solver is already a reified, serializable, steppable machine with an explicit constraint language and an inspection protocol pointed at it ([[../typing-machine]], [[../inspection-protocol]]), so a first-class solver interface extends what is already there rather than opening a hole in it.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The worklist solver's constraint language, decomposition, and transitions are specified**, and its subtyping fragment is built in the degenerate case where no metavariable is in play — `gandr-core-checker`'s `subtype` module, whose goals sit in an explicit worklist and are replaced by their child goals ([[../type-system#What the tree actually decides]]).
* **One constraint form's decision procedure is sealed behind a carrier signature**, which is the closest thing in the tree to a domain boundary: `gandr-core-checker`'s `grade` module newtypes `Grade` over a module-private representation, so the semiring signature — the zero, one and unbounded elements, the finite constructor, and the order, addition and multiplication operations — is the entire cross-module surface.
* **A dependency-footprint mechanism exists**, at item granularity rather than per constraint: `gandr-core-incremental`'s `footprint` module captures the ambient-context names a lowered item read, and over-approximates by marking a footprint opaque on any node it cannot represent ([[../incremental-pipeline]]).
* **A fuel-bounded semi-decision procedure with a trail is specified**, for implicit resolution, and is the mechanism this design reuses ([[../../surface-language/proposed/modules#Implicit resolution]]).

**Designed, and not built.** Everything else here: the domain interface itself, the registration surface, the per-domain trail watermarks, the per-domain fuel budget, the refinement types, the obligation generation, the SMT discharge and its query cache, the contracts sugar, the query cards, and every row of the interaction table.

**Two as-built claims are worth stating precisely, because the design record they come from states them more strongly than the tree supports.**

**The grade domain is not parametric, and the design record's "already a plugin in all but name" is not true of this tree.** `gandr-core-checker`'s `grade` module says so in its own terms: `Grade` is the single concrete carrier across the whole crate and **there is no semiring type parameter**.
What the seal buys is the weaker and still useful property that **swapping in a different semiring is an edit to that one module** — which is what makes the grade row the cheapest domain to extract first, not evidence that the extraction has happened.

**The checker has no extension point of this shape at all.** It carries two public traits — a host-handler seam and an item source — and neither is a constraint-domain interface; there is no registration surface anywhere in the workspace, and no file in any crate mentions SMT, refinement typing, or a solver backend.

## The domains that already exist

Every row below is a constraint form the solver already carries, paired with the procedure that decides it.
The interface this document specifies is the one these rows already satisfy; naming it is what makes the seventh and eighth rows additions rather than surgery.

| domain       | constraint forms                      | decision procedure                                     | status                       |
| ------------ | ------------------------------------- | ------------------------------------------------------ | ---------------------------- |
| subtyping    | `A <: A′`, `B <: B′`                  | structural decomposition, with choice points           | built in the degenerate case |
| unification  | `α := T`                              | occurs check and substitution                          | not built                    |
| grades       | `r ⊑ s`                               | the preordered semiring's order                        | built, decided on the spot   |
| mobility     | `mobile(A)`                           | syntax-directed                                        | not built                    |
| sharing      | `esync(S_L, S_S)`                     | regular-tree check over contractive recursive types    | not built                    |
| multiparty   | `wf(G)`, `proj(G, p, L)`              | regular-tree checks                                    | not built                    |
| ranks        | `ℓ ≺ ℓ′`                              | cycle detection, then a topological order              | designed, reserved           |
| effect rows  | `ε₁ ≃ ε₂`                             | row unification                                        | built as sealed rows         |

The first six rows are the constraint language of [[../type-system#The constraint language]] and are specified there.
The **ranks** row is the reserved hook of [[../type-system#type-extension-06]], whose deadlock-freedom refinement emits one order constraint per currently-held rank and reports a cycle **as** the diagnostic.
The **effect rows** row is built in its sealed form and specified in [[../effects-and-control]].

**The grade row is the one to extract first**, and the reason is the seal rather than the design: its decision procedure is already the only thing any other module may call, so lifting it to a registered domain moves code without changing behaviour.

## The domain interface

A domain owns a set of constraint forms and answers six questions about them.

```text
trait ConstraintDomain {
  /// The constraint forms this domain owns: a tag plus a payload schema. Payloads
  /// must be serializable — they appear in checkpoints and in the inspection surface.
  forms: ConstraintForm[];

  /// Decompose or decide one constraint. May emit sub-constraints (its own or another
  /// domain's), bind solver variables it owns, or report choice alternatives.
  step(c: Constraint, ctx: SolverCtx): Decided | Subgoals(Constraint[])
                                     | Choice(Constraint[][]) | Failed(Explanation);

  /// Trail integration. A domain carries its own state; both operations are O(1)-ish.
  push(): Watermark;
  popTo(w: Watermark): void;

  /// Diagnostics: a human-readable account of a failure, carrying source spans.
  explain(f: Failed): Diagnostic;

  /// Incremental integration: the dependency footprint of a decided constraint — which
  /// variables and inputs it read — for checkpoint validation.
  footprint(c: Constraint): Footprint;
}
```

**The `step` outcome is a sum of four cases, and the fourth is what keeps a failure legible**: a domain that fails returns an explanation rather than a bare rejection, so the diagnostic is built where the knowledge is instead of being reconstructed by the core.

**`push` and `popTo` are the domain's half of the solver trail.** The solver's trail is a stack of choice points rather than a single backpoint, because a failure may have to unwind past more than one choice ([[../type-system#The constraint language]]); a domain that carries state must therefore be able to unwind to an arbitrary depth, not merely to the last one.

**`footprint` is what makes an SMT query cache survive an edit.** The incremental pipeline reuses a checkpoint exactly when the dependency edges it consulted are untouched, so a domain that reports which inputs a decided constraint read inherits invalidation rather than implementing it.

## The four rules that keep a domain from breaking inference

Each rule is a fence, and each fence has a failure it exists to prevent.

**GHC's typechecker-plugin experience is the cautionary precedent for the first, and the reading is this design's rather than a published finding.** The claim as the design record states it: plugins able to manufacture arbitrary evidence make inference unpredictable and error messages inscrutable, because a type can then be explained only by replaying which plugin produced it.
The precedent is carried by name and subject and is **locator-pending**: the concrete candidate to check is Gundry, _A Typechecker Plugin for Units of Measure_ (Haskell Symposium 2015), which is not held in the reference register and whose identifier has not been verified.
Until it is obtained, the precedent supports the rule's motivation and grounds no claim about what GHC's plugin interface actually permits.

### solver-rule-01

**Domains accept or reject; they never invent types.** A domain may refine **its own** variables — grades, ranks, refinement indices — but may not bind a type-level metavariable except through the unification domain's interface.

**This is what keeps inferred types principal and independent of the order domains run in.** A domain that could bind `α` would make the inferred type a function of the registration order, which is the property that turns a plugin system into a source of irreproducible errors.

### solver-rule-02

**Monotone under trail discipline.** `push` and `popTo` must restore exact state, and the core enforces watermark pairing.

**Exactness is what buys two properties at once**: a domain that unwinds precisely is compatible with backtracking search, and it is compatible with checkpoint invalidation for free, because a checkpoint taken inside a speculative region is invalidated by the same pop that discards the region ([[../type-system#Transitions]]).

### solver-rule-03

**Total with fuel.** Every domain receives a fuel budget — the mechanism already specified for implicit resolution ([[../../surface-language/proposed/modules#Implicit resolution]]) — and exhaustion is a diagnostic carrying the obligation chain, never a hang.

**This is the rule that makes an external, semi-decidable domain safe to include at all.** An SMT backend has no total decision procedure to offer; what it can offer is a bounded one whose exhaustion is reported like any other failure.

### solver-rule-04

**Serializable end-to-end.** Constraints, domain state, and failures serialize with the machine state.

**A domain that satisfies this inherits resumability and the inspection surface without writing either**, since both consume the serialized machine rather than the checker's memory ([[../inspection-protocol]]).

## The refinement domain

Refinement types are the first external domain: the first whose decision procedure is a separate solver rather than a rule in the checker.

### Types and obligations

```text
A ::= … | { ν : A | φ }        a refinement of a value type, where A is a "small" base —
                               the same predicativity fence the module layer draws,
                               and deliberately the same one

φ ::= ν ⋈ e | φ ∧ φ | φ ∨ φ | ¬φ | …
                               ⋈ ∈ {=, ≤, <, …}; e ranges over ν, the in-scope
                               refinement-relevant variables, and literals
```

**The fence is a default with visible crossings, not a wall**, and it is the same shape as the module layer's predicativity fence ([[../../surface-language/proposed/modules]]): small bases only, predicates drawn from the decidable fragment, and a deliberate crossing where one is wanted.

Subtyping decomposition emits an implication obligation to the domain:

```text
{ ν : A | φ } <: { ν : A′ | ψ }   →   A <: A′,  smt⟦ φ ⟹ ψ ⟧
```

**The obligation is emitted, not decided**, which is what places refinements in the solver rather than in the subtyping rules: the structural half decomposes as usual and the logical half leaves for a domain that owns it.

### Contracts are sugar

```text
fn divide (n : Int) (d : { ν : Int | ν ≠ 0 }) : F { ν : Int | ν ≤ n } = …
requires/ensures p  ≡  refinements on the argument and result positions
```

**There are no loop invariants to annotate, because there are no loops.** Iteration is a recursion former ([[recursion-former]]), so the annotation burden a contract system usually carries at loop headers does not arise.

**A recursive computation gets the standard treatment**: the declared refinement of the recursive binding **is** the inductive hypothesis at its recursive call sites.

### Discharge and diagnostics

* **Backend.** Z3 [@moura-bjorner-2008-z3], or an equivalent such as CVC5, compiled to the target runtime and run on a background thread for interactive exploration, and as a separate process for the command-line and interactive surfaces.
* **Caching is free, given footprints.** Queries are hash-consed and cached by query hash, and `footprint` makes a cached result valid across every edit that does not touch its inputs — so the incremental story is inherited from the pipeline rather than built.
* **Visualization is the differentiator.** Each obligation renders as a query card in the derivation surface, carrying the premise refinements, the goal, and on failure the model.
  A failing obligation shows its counterexample **as a binding environment** — "with `n = -1, d = 3` the postcondition fails" — which is the form the interactive surface already renders bindings in.
* **Candidate refinements proposed by a language model are an editor affordance, and nothing in the core depends on one.** They arrive as suggested annotations whose query cards are pre-run, so the solver remains the checker and the proposer remains outside the trusted path.

### Interactions

| feature  | interaction                                                                                                                     |
| -------- | ------------------------------------------------------------------------------------------------------------------------------- |
| sessions | refined payloads `!{ν:Int \| ν>0}.S`; with multiparty this is refinement MPST, giving protocol-level value constraints          |
| grades   | grades stay in their own semiring domain and refinements do not subsume them — different variable sorts, by [[#solver-rule-01]] |
| worlds   | obligations are located: a refinement mentioning world-local state is discharged with that world's axioms in scope              |
| holes    | a hole inherits its expected refinement as a **goal card**, making the interactive surface a lightweight synthesis scratchpad   |

The multiparty row names Zhou, Ferreira, Hu, Neykova and Yoshida's statically verified refinements for multiparty protocols [@zhou-ferreira-hu-neykova-yoshida-2020-refinement-multiparty] as the construction to follow.
**That work is not held here and its mechanism is not stated in this corpus**, so the row records the design intent — payload refinements that survive into the protocol — and not a claim about how that work achieves it.

## The published foundation, and what it settles

The refinement layer is not a fresh design: it is the **focusing refinement typing** line, whose relationship to this system is closer than the usual adoption because that line is already call-by-push-value.

**The core system is Economou, Krishnaswami and Dunfield's** [@economou-krishnaswami-dunfield-2023-focusing-refinement]: a focalized variant of call-by-push-value with bidirectional typing, whose declarative system is proved semantically sound against an elementary domain-theoretic model and whose algorithmic system is proved sound, complete and decidable.

**Its load-bearing mechanism is value-determined indices**, and the authors' claim is precise: value-determined existentials of input types under focus are guaranteed to be solved at the end of the focusing stages, so the system emits SMT constraints **without existential (unification) variables** in them.

**The connection to [[#solver-rule-01]] is this document's reading, not theirs**, and it is worth stating because it is the difference between a house rule and a proved property.
That rule forbids a domain from binding a type-level metavariable; a constraint generator that provably leaves no unsolved existential in what it emits cannot violate the rule by accident.
Whether the guarantee transfers to this setting is unestablished — it is proved for their calculus, and gandr's is a different one — so the mechanism is recorded as the candidate route to the rule and not as a discharge of it.

**The extended treatment is Economou's thesis** [@economou-2024-modular-refinement], and it carries three things the paper does not.

1. **Refinement of algebraic data types, modularly, by measures** — recursive predicates over inductive data, expressive enough for properties like "this list is in increasing order".
   The technique is the liquid-types line's, and the thesis credits it to Kawaguchi, Rondon and Jhala's type-based data structure verification [@kawaguchi-rondon-jhala-2009-type-based-data-structures].
2. **An explicit index-program phase distinction**, syntactically separating index terms, which may appear in types, from program terms, which may not.
   The thesis's assessment of what that buys is specific and is worth carrying in its own terms rather than generalized: Liquid Haskell has the more powerful features, including modular recursive refinement of inductive data, and lacking the distinction is what makes it unclear how to give liquid typing a denotational semantics [@vazou-seidel-jhala-vytiniotis-peytonjones-2014-refinement-haskell].
3. **Total correctness even for non-structurally recursive programs**, obtained from typing soundness against the denotational model — the thesis's worked example verifies a mergesort that terminates and returns an ordered list of the input's length.
   **The thesis qualifies this and the qualification is load-bearing**: what typing soundness gives directly is total correctness _denotationally_, and operational total correctness is a corollary of it together with computational adequacy.

**That third point is why this matters to gandr beyond the refinement types themselves.** The user-level recursion former ([[recursion-former]]) records termination as an obligation and names a solver-assisted route to discharging it, and a system carrying total correctness without a structural-recursion requirement is a concrete candidate for that route rather than a hope that one exists.
**The candidacy is this document's proposal**; the thesis makes no claim about gandr, and the route is unbuilt on both sides.

**Two cautions about these two works, stated because the difference is easy to lose.**

**They are not a duplicate pair.** The paper is jointly authored and is the core system; the thesis is single-authored, states which of its material is adapted from the paper, and credits its co-authors' contributions explicitly.
Cite the paper for the core calculus and its metatheory, and the thesis for the modular, inductive-data extension.

**The held thesis is a corrected revision, and one correction is load-bearing.** Its own updates section, dated 1 May 2025, records a bug fix in the main completeness lemma — restricting the form of unrolled values — along with corrections to the cut-elimination presentations and to the computational-adequacy proof.
A claim taken from the completeness result should be taken from this revision.

## Refinements compose with the evidence layer; they do not replace it

**The relationship is composition, and stating it the other way round would be a category error.** The corpus carries an in-language, phase-separated **erasible-evidence** layer as a decided direction, erasing to the non-dependent runnable core and reaching toward full dependent types ([[../feature-staging#The decided directions as construction milestones]], [[../type-system#Feature staging]]).

Refinements are **one constraint domain alongside** that sublanguage, not an alternative erected instead of it.
As a domain they stay value-indexed, decidable modulo fuel, and principality-preserving by [[#solver-rule-01]] — and the light indices this system already carries, namely worlds, roles, labels, grades and ranks, live in exactly that regime.

**Where the two meet is a named construction obligation rather than a conflict.** Full dependency, if it is taken, and SMT refinements coexist as distinct composed domains; what has to be constructed is the interaction between the evidence phase's erasure and a domain that reads indices, and that construction has not been done.

## The adoption stages

Each stage is separately landable, and the first is a refactor with no behaviour change.

### solver-stage-01

**Extract the interface from the built-ins.** Lift `ConstraintDomain` out of the existing decision procedures with no behaviour change, and have the inspection surface tag each constraint with the domain that owns it.
The grade module is the natural first extraction, for the reason its seal already gives.

### solver-stage-02

**Register the rank domain**, the first genuinely new one: the order constraint of [[../type-system#type-extension-06]], small enough to be a real test of the interface and closing the deadlock-freedom hook it was reserved for.

### solver-stage-03

**The SMT domain**: refinement syntax, obligation generation from subtyping decomposition, the backend on a background thread, and query cards in the derivation surface.

### solver-stage-04

**Contracts sugar, refined session payloads, and hole goal cards** — the three surfaces that make the domain visible to a program author rather than to a checker author.

### solver-stage-05

**User-registered domains in the interactive surface**, letting an author write a semiring or a custom domain and watch the solver run it.
This stage is optional and is the one whose value is least established.

## Open dispositions

Each item below arrived open from the design record and carries exactly one disposition.

**The SMT backend choice is open, and is carried as open.** The record names Z3 with CVC5 as an equivalent alternative and does not choose; nothing downstream has been written that depends on which.

**The predicate fragment is fenced but not delimited.** "The decidable fragment" names a policy, not a set: which theories a predicate may draw on, and therefore which queries are guaranteed to terminate, is not fixed here and must be fixed before [[#solver-stage-03]] lands.

**Whether the rank domain's ranks are inferred or declared is settled elsewhere and not reopened here.** The type system records that they should be inferred, by collecting the order constraints and topologically sorting them, with a cycle reported as the potential deadlock.

**The interaction between a refinement domain and the evidence phase's erasure is a construction obligation**, carried above and not discharged.

**The liquid-types line is cited by name, and one of its citations is transcribed rather than held.** Rondon, Kawaguchi and Jhala's liquid types [@rondon-kawaguchi-jhala-2008-liquid-types] is in the reference register; Vazou, Seidel, Jhala, Vytiniotis and Peyton-Jones's refinement types for Haskell [@vazou-seidel-jhala-vytiniotis-peytonjones-2014-refinement-haskell] is **not**, and its metadata here is transcribed from the bibliography of a work that is held, not recalled.
The same holds for Kawaguchi, Rondon and Jhala [@kawaguchi-rondon-jhala-2009-type-based-data-structures].

**One motivating claim about another language is carried and marked.** The design record cites a 2026 MoonBit release as having shipped SMT-backed contracts and machine-checked, model-proposed invariants as a native language feature; the claim is carried because it is the evidence that this design is not exotic, and it is **locator-pending** — no version of those release notes has been obtained, and the register holds nothing for it.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot solver-interface design record** in full — its domain inventory, its interface, its four rules, its refinement domain with the type and predicate grammars and the contracts sugar, its discharge and diagnostics, its interaction table, its composition argument, its staging, and its references.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `subtype`, `grade`, `effect` and `machine` modules, `gandr-core-incremental`'s `footprint` and `region` modules, and a workspace-wide check that no crate carries SMT, refinement, or solver-backend machinery.
3. The **corpus documents that carry this design's premises** — the type system's constraint language and its reserved rank hook, the module layer's fuel-bounded implicit resolution and its predicativity fence, the incremental pipeline's footprints, and the inspection protocol — which this document links rather than restates.
4. The **published foundation**, read for this pass: the focusing-refinement paper's abstract and the thesis's title page, abstract, contributions, corrections section, and worked example.

**Confidence, by class.**

* **High** — the interface, the rules, the grammars, and the staging, which are transcribed from the design record rather than re-derived.
* **High** — the as-built statements, each verified against the named module at write time, including the correction to the design record's claim about the grade domain.
* **Medium** — the account of the published foundation, whose abstract, contributions and corrections were read and whose body was not.
* **Marked at the claim** — the typechecker-plugin precedent and the MoonBit release, both locator-pending; and the three liquid-types citations, whose metadata is transcribed from a held work's bibliography rather than from the works themselves.
