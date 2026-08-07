# The predictable-fragment unifier

**Proposed.
No term-level unifier, no metavariable, and no elaborator exists in this tree.** What exists is the machinery this specification sits on: the worklist solver as a separate, serializable machine ([[../typing-machine#The solver as a separate machine]]), the constraint language whose instantiation row this document's domain owns ([[../type-system#The constraint language]]), the Squier-completed cell layer that is the unifier's adopted theory backend ([[../../implementation#The engines]]), the certified level oracle that is the oracle-as-backend instance already built ([[../../implementation#The trusted base]]), and the domain interface whose table lists unification as its first not-built row ([[solver-interface#The domains that already exist]]).

This document specifies the unifier the three-scheduler rule presupposes ([[elaboration-schedulers#sched-rule-02]]): higher-order nested pattern unification — Miller's pattern fragment extended to record/Σ structure, respecting η-equivalence and definitional singletons natively — as a solver-machine service whose solutions are checkable evidence.
A unification result is self-certifying: substitute and re-check.
That is exactly the shape of evidence the admission discipline already knows how to replay, so no opaque verdict enters the trusted base.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The solver is a separate machine from the recursive checker**, with its own serializable state — queue, substitution, and a trail that is a stack of choice points — and the two realizations are property-tested for step-for-step agreement on a control log: `gandr-core-checker`'s `machine` and `checker` modules ([[../typing-machine#The solver as a separate machine]], [[../typing-machine#What is built]]).
* **The constraint language already carries the unification row's shape**: `α := T` instantiation with an occurs check, substitution applied to the whole queue on instantiation ([[../type-system#The constraint language]]).
  What the tree decides today is the solver's **degenerate case** — `gandr-core-checker`'s `subtype` module runs the worklist with no metavariables in play, so the queue is observationally in-order structural recursion ([[../type-system#What the tree actually decides]]).
* **The completed cell layer is built**: `gandr-theory-computads` carries the cell store, overlap enumeration emitting both critical-pair kinds (confluence and composition), and budgeted Knuth–Bendix/Squier completion whose `complete` normalizes both reducts of every confluence pair, emits a coherence tracelet for a joinable pair, orients a non-joinable pair by the reduction order into a derived cell, and declines within budget with the pending overlaps reported — never divergence, never a guess (`completion` module).
  Normalization is deterministic and records which cell fired where; replay skolemizes the peak and re-runs both recorded paths by ground rewriting ([[../../implementation#The engines]]).
* **The engines already run a first-order pattern unifier, and it is not this document's unifier.** `theory-computads`' `subst` module carries one-sided matching (cell application) and two-sided `unify_cmd` (overlap superposition): iterative, occurs-checked, with triangular bindings resolved to an idempotent fixpoint before return.
  That unifier operates on the deliberately narrow cell-visible pattern grammar — one command-pattern variant, a linear consumer spine — and exists to compute critical pairs.
  The unifier specified here operates on the checked language's terms, is higher-order, and is a solver-machine domain; the two share a name family and nothing else, and reading the built one as an instance of the proposed one would be a mistake.
* **The certified level oracle is the oracle-as-backend instance that already exists**: `kernel-strata` decides level judgements by canonical finite joins, and every decision returns checkable evidence — a consistency witness or a replayable loop witness — so trust concentrates in the validators ([[../../implementation#The trusted base]]).

**Designed, and not built.** The unification domain itself: its constraint forms, its fragment test, its postponement bookkeeping, its registration under the domain interface, and every rule below.

## The fragment, stated precisely

The fragment discipline is the reliability contract: the unifier emits only most general unifiers within the predictable fragment, and postpones — never guesses — outside it.
Completeness within a well-defined fragment is the shadow property of reliability: a solver that sometimes finds non-general solutions is user-visible unreliability even when every answer it gives is correct ([[elaboration-schedulers#The scheduling argument]]).

### What is in

* **Miller patterns.** A metavariable applied to a renaming of distinct bound variables.
  Categorically this is exactly the restriction on substitutions under which the relevant equalizers exist and are computable, with unitary most general unifiers — "predictable" means MGUs only (see [[#Unification is an equalizer — the adopted frame]]).
* **Nested record/Σ patterns.** Patterns may nest record and dependent-pair structure, per nested pattern unification [@kovacs-2023-nested-pattern-unification]; the nesting is handled natively rather than by encoding into the simply-typed fragment.
* **η for functions and records.** The fragment respects η-equivalence natively: η-expansion is part of the decomposition discipline, not an equation the solver may overlook.
* **Definitional singletons.** A variable definitionally equal to a unique inhabitant is used as such during solving, per the same nested-pattern account [@kovacs-2023-nested-pattern-unification]; singletons are one of the two cases that kill freer fragments, and they are in.

### What postpones

* **Non-pattern constraints that may become patterns.** A flex-rigid or flex-flex pair outside the restriction postpones and resumes when further instantiation brings it inside — this is the dynamic-pattern discipline below, and postponement is a solver state, not a failure.
* **Constraints needing case analysis do not postpone; they leave the unifier entirely.** Richter–Böhler's boundary: such constraints synthesize no equalizer in any pattern fragment, because the solution is a definition by dependent pattern matching and must be constructed by a pattern-matching compiler [@richter-bohler-2026-pattern-matching-unification].
  The unifier therefore exports them to the pattern-matching lane as a named hand-off ([[#unifier-rule-05]]) — guessing here is exactly the non-general-solution failure the reliability contract exists to prevent.

### The boundary is a solve-time computation

Abel–Pientka's dynamic patterns are the observation that the fragment's boundary can be computed at solve time rather than fixed in advance [@abel-pientka-2011-dynamic-pattern-unification]: solve the sub-problems that satisfy the pattern restriction eagerly, delay the ones that do not until accumulated information makes them patterns, and do this per sub-problem rather than per constraint.
This is the discipline the postponement bookkeeping implements: the fragment is not a syntactic gate the constraint language passes or fails once, but a per-step test the solver re-runs as the substitution grows.

## Unification is an equalizer — the adopted frame

**Adopted (owner-ratified): the Squier-completed cell layer is the unifier's theory backend, and the equalizer account below is the specification frame.** What follows restates the ratified direction so this document stands alone, then names what remains open.

### The substitution category

Read in the substitution category — contexts as objects, substitutions as morphisms — a unifier of $s, t : Gamma -> Delta$ is a morphism $sigma$ equalizing them: $sigma ; s = sigma ; t$.
**The most general unifier is the equalizer**: the universal such morphism, initial in the category of unifiers.
Everything else in the vocabulary is a statement about which equalizers exist and are computable:

* **complete unification for a fragment** — the relevant equalizers exist, with a principal representative;
* **the Miller fragment** — a restriction on substitutions (renamings of distinct bound variables) making the equalizers exist and be computable;
* **higher-order unification modulo βη** — the same construction in the free cartesian closed category, where the computable subcategory is what the nested-pattern extension enlarges to record/Σ structure [@kovacs-2023-nested-pattern-unification];
* **unification modulo a theory** — the case that matters here, because the elaborator works over a presented universe: equality in the _presented_ category, where deciding equality is the semantic half of unification modulo that theory.

### Completion computes equalizers inside a presented theory

A convergent presentation decides equality by normalization, and completion is the process that manufactures convergent presentations: **critical pairs are overlaps** — computed by unifying left-hand-side patterns — and Knuth–Bendix closes the peaks the overlaps generate.
The relationship is a fixed point, not an analogy: unification produces critical pairs, and closing critical pairs makes more of the substitution category's equalizers computable.
Squier completion is the homotopical version and the one gandr runs: not merely a convergent system but a _coherent_ one, where every critical peak receives a filler cell, so that **derivations themselves have a decidable equality** — the step from unification-as-a-verdict to proof-relevant unification, and the reason a unification result can be an object the admission discipline replays rather than a verdict it trusts.

The solver machine, asked to unify modulo the declared rule layer, therefore **normalizes against completed cells and compares canonical forms** — the "decide" tier of the coherence economy doing unification work ([[../../metatheory#The coherence economy]]).
Where completion has declined within budget — completed means the worklist drained, with the three obstruction classes reported, never guessed ([[../../implementation#The engines]]) — the residual constraint postpones under the three-scheduler rule rather than being settled by a non-general pick.

### The two soundness templates of record

Two published results are the soundness templates the unifier's scheduling is specified against; both are adopted as the conditions under which the solver's steps are sound, and neither is re-derived here.

* **Conditions (Behr–Krivine).** Application conditions on rules compose only via shift-and-transport constructions, and associativity survives precisely when transport is functorial [@behr-krivine-2021-conditions-compositionality]. gandr's rule cells carry side conditions of exactly this kind — grades, polarity, sphere-typed boundaries — and fusion composes cells, so "what happens to a cell's conditions when cells fuse" is the template's question.
  The compositionality theorem is the shape the answer must have: conditions are **transported, not recomputed**.
  The conditions-transport lemma for gandr's fused cells is **owed** ([[#Open dispositions]]).
* **Nonlinear concurrency (Behr–Harmer–Krivine).** A metavariable repeated across a constraint is a nonlinear pattern; a solver step that fires two constraints sharing a metavariable is a fusion; and sesqui-pushout cloning is what happens operationally when one metavariable's solution is instantiated in two places [@behr-harmer-krivine-2021-nonlinear-concurrency].
  The concurrency theorems are the soundness template for the solver's parallelism: they say when parallel steps over shared metavariables are sound, which is exactly the situation a worklist solver with a shared substitution is in at every step.

### The oracle-as-backend precedent, and its trust boundary

The architectural precedent of record is the ReSMT line: a rewriting engine that delegates its decided fragments — graph-overlap computation among them — to an external SMT solver [@behr-heckel-saadat-2020-graph-overlaps-z3]. gandr's instance of the same shape is already built and is internal: the solver machine consults the certified level oracle for the level judgements, and the oracle's answers are checkable evidence ([[../../implementation#The trusted base]]).
The trust boundary is stated once and binds this whole document: **no external solver enters the unifier's pipeline.** The grade semiring is the natural second fragment a ReSMT-shaped delegation would serve — grade constraints are semiring equations pattern unification cannot express ([[elaboration-schedulers#sched-rule-03]]) — and it is a **priced candidate only**: an external solver is a trust expansion, the price is named, and it is not taken ([[#Open dispositions]]).

## The solver-machine interface

The unifier is the constraint domain the domain interface's table lists as `unification | Γ ⊢ T ≐ T′ | occurs check and substitution | not built` ([[solver-interface#The domains that already exist]]).
This section is that row's specification: the unifier is a domain in the sense of [[solver-interface#The domain interface]], and it satisfies the four rules that keep a domain from breaking inference.

* **Constraint forms.** The domain owns `Γ ⊢ T ≐ T′` and `α := T`, plus its postponement bookkeeping: a postponed constraint is domain state, serializable with the machine state, carrying the reason (non-pattern, awaiting instantiation; or a completion-declined residual) so diagnostics can show it.
* **`step` outcomes.** Decomposition inside the fragment emits sub-constraints; a solved metavariable binds through the domain's own interface; a non-pattern step returns the postponed form; a genuine clash returns `Failed` with the explanation built where the knowledge is.
* **The four rules, instantiated.** [[solver-interface#solver-rule-01]] is what makes this domain load-bearing for every other one: the unifier is the **one** domain that may bind a type-level metavariable, which is why every other domain is forbidden to. — [[solver-interface#solver-rule-02]]: postponement and resumption are monotone under trail discipline, restoring exact state, with watermark pairing enforced by the core. — [[solver-interface#solver-rule-03]]: total with fuel; exhaustion is a diagnostic carrying the obligation chain, never a hang. — [[solver-interface#solver-rule-04]]: constraints, the metavariable context, and the postponed set serialize end-to-end, so the unifier inherits resumability and the inspection surface without writing either.

### The evidence form

**A solution is a substitution that re-checks.** The unifier's output is never a verdict the checker must believe; it is a substitution $sigma$ together with the obligation — discharged by replay — that the instantiated sides are convertible.
This is the corpus's standing replayed-not-trusted discipline applied to solving: the admission choke point re-derives, the export is re-checkable ([[../../implementation#The trusted base]]), and a unification result is one more object that discipline already knows how to replay.
Because the theory backend is a _completed_ layer, the conversion half of that re-check is itself normalization against cells whose coherence fillers exist by construction — the evidence is self-certifying all the way down, and **no opaque verdict enters the trusted base**.

## The contract

### unifier-rule-01

**Most general unifiers, inside the fragment only.** The unifier emits an MGU when the constraint lies in the predictable fragment of [[#The fragment, stated precisely]]; outside it, it postpones or exports, and it never guesses.
This is [[elaboration-schedulers#sched-rule-02]] restated as the domain's own contract; completeness within the fragment is the shadow property of reliability.

### unifier-rule-02

**Solutions are self-certifying evidence.** Every solution the domain emits is a substitution whose application re-checks by replay; the domain's verdicts are never trusted, only replayed.

### unifier-rule-03

**The theory backend is the completed cell layer.** Unification modulo the declared rule layer normalizes against completed cells and compares canonical forms; a completion-declined residual postpones under the three-scheduler rule rather than settling by a non-general pick.

### unifier-rule-04

**The unifier calls no other scheduler, and no external solver enters its pipeline.** The one-directional call graph of [[elaboration-schedulers#sched-rule-05]] binds: instance resolution and the `cast` resolver may call the unifier; the unifier calls nothing.
Decided oracles with checkable evidence — the level oracle is the built instance — are backends, not schedulers, and are the only delegation the domain performs.

### unifier-rule-05

**Case analysis leaves the unifier.** A constraint whose solution needs case analysis synthesizes no equalizer in any pattern fragment [@richter-bohler-2026-pattern-matching-unification]; it is exported to the pattern-matching compiler's lane as a named hand-off, neither postponed nor guessed.

### unifier-rule-06

**Every unifier step is a solver-machine step.** Solutions, postponements, and exports are states of the shared solver machine, so they enter the solver/checker agreement property and the serialization discipline with no new mechanism ([[#The agreement property]]).

## Cells aid unification — the adopt record

**The question is answered: adopted.** Whether equations carried as cells give the unifier a rewrite system to consult rather than a black-box conversion — yes, and the equalizer frame of [[#Unification is an equalizer — the adopted frame]] is the specification of how: the completed layer computes the substitution category's equalizers inside the presented theory by normalization, the critical pairs the layer already solves are the unification problems of its own rule layer, and the solver machine consults the result exactly the way it consults the level oracle — as a backend whose answers carry checkable evidence.

Two pieces of supporting literature frame the consult-not-black-box half:

* **Allais–Boutillier–McBride's decision procedure for neutral terms** — a sound and complete, formalized procedure for deciding free extensions of an equational theory [@allais-boutillier-mcbride-2013-neutral-terms].
  It is the precedent that "decide by normalization against a free extension" is a buildable, mechanizable shape rather than a hope, and it is already the decision-procedure style gandr's level algebra sits in.
* **Hewer's thesis, types with extra structure** — predicates, equations, and composition carried on types [@hewer-2024-types-extra-structure] — the type-former-side account of equations riding on structure, which is the shape a declared rule layer presents to a unifier.

A worklog gesture at observational-type-theory-flavoured handling of postponed problems, attributed there to Kovács, is recorded as a **gesture**: the anchor is the disentanglement tree, which reports the proposal without developing it [@sterling-2026-pterodactyl-worklog, tree 01JQ], so the gesture is cited as a direction, never as a design ([[#Open dispositions]]).

### What this posture is not

Stated so the adoption cannot be misread as any of its neighbours — each comparable either trusts the rule engine or forgets the derivation, and this document's posture does neither.

* **Not user rewrite rules inside definitional equality.** The rewrite-rule line puts user-declared rules inside the kernel's definitional equality, with confluence trusted as a side condition [@cockx-tabareau-winterhalter-2021-taming-rew].
  Here the rule layer is outside the trusted base, completion is machine-run with a budget and a defined decline, and every equality the unifier derives is replayed — the replayed-cells posture is precisely the refusal of a trusted side condition.
* **Not narrowing as a language feature.** Unification modulo a convergent rewrite system is the field's oldest trick, and Maude's variant-unification line — folding variant narrowing with optimal variant termination [@escobar-sasse-meseguer-2012-variant-narrowing] — is the closest system-level precedent.
  The differences are the trust posture and that derivations are remembered as replayable objects.
* **Not deduction modulo.** Type-checking modulo a rewrite system [@dowek-hardin-kirchner-2003-theorem-proving-modulo] is the closest type-theoretic precedent; again the difference is that the modulo here is computed by a completed, coherent layer whose derivations are first-class, replayable evidence.
* **Not NbE-based conversion.** Normalization-by-evaluation conversion decides equality semantically and is proof-irrelevant — it answers and forgets [@smalltt].
  Unification evidence here is an object: replayable, serializable, and inspected.

### The tracelet normal form, as the corpus records it

Certificate identity is replay-equivalence, and the certificate normal form — content addressing, trivial-unit insertion and removal, shift equivalence — is recorded as **a performance fast path, never a decidability result**: normal-form equality decides shift equivalence (sound by the uniqueness of primitive factorization [@behr-2019-tracelets] [@behr-kock-2021-tracelet-hopf]), shift equivalence implies replay-equivalence, and the converse is constructibly false ([[../../metatheory#The certificate algebra]]).
As built, `cells_equal` decides boundary equality plus two replays and carries no normal-form fast path yet.
For the unifier this matters at exactly one place: when two unification evidences must be compared, the normal form is the decide-tier comparison — NF-equal implies replay-equal, a decidable under-approximation — and when the fast path is absent the replay path is the whole story and loses nothing but time.
This document assumes only the replay path; the fast path's enablement is recorded in the metatheory track and changes no contract here.

## The agreement property

**This document changes no code, and the machine/checker agreement suite is untouched.** The property — the defunctionalized machine and the recursive checker agree step-for-step on a control log ([[../typing-machine#What is built]]) — stands exactly as the tree carries it.

The specification nonetheless constrains the suite going forward, by [[#unifier-rule-06]]: the unifier runs as solver-machine steps, and the solver state — queue, substitution, trail — is part of the machine state the agreement property quantifies over.
So when the domain is implemented, its steps land **inside** the property, not beside it: every decomposition, binding, postponement, and export is a step the two realizations must agree on, every solution enters the agreement property by construction, and the control log gains the unifier's step forms without the property changing shape.
An implementation that solved constraints off-machine — in a helper the machine merely calls — would violate this document, not merely miss a test.

## Open dispositions

Each item below arrived open from the adopted direction and carries exactly one disposition.

* **The conditions-transport lemma for fused cells** — carried.
  The obligation is functoriality of side-condition transport (grades, polarity, sphere-typed boundaries) under fusion, in the shift/transport shape of the conditions template [@behr-krivine-2021-conditions-compositionality]; until it is discharged, every fusion the unifier's backend performs carries the obligation with it, and [[#unifier-rule-03]]'s normalization claim is conditional on it.
* **The postponed-problem surface** — parked.
  What the user _sees_ of a postponed constraint (the worklog's OTT-flavoured gesture, attributed to Kovács [@sterling-2026-pterodactyl-worklog, tree 01JQ]) is a display-layer design question whose anchor reports the gesture without developing it; it is revisited when the postponed-constraint surface is designed, and nothing in the contract depends on it.
* **An external solver for the grade semiring** — declined, with a reversal condition.
  The price is named: an external solver is a trust expansion, and it is not taken ([[#unifier-rule-04]]).
  The reversal condition is a demonstrated inability to decide the semiring's fragment internally with checkable evidence; if that is ever demonstrated, the priced candidate is re-priced, not silently adopted.
* **The fragment's statement against the full type formers** — carried.
  [[#What is in]] fixes the fragment's content; which of the checked language's connectives count as record/Σ structure for nesting purposes is fixed when the domain is implemented against the full formers of [[../type-system]], and no scheduling decision here depends on the answer.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **ratified adoption direction** for the unifier's theory backend — the equalizer frame, completion as the backend, the two soundness templates, and the oracle-as-backend precedent with its trust boundary — restated in full so this document stands alone.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `machine` and `checker` modules (the two realizations and their agreement testing), `gandr-theory-computads`' `completion` module (budgeted Squier completion, the defined decline, the three obstruction classes) and `subst` module (one-sided matching, the two-sided `unify_cmd` for superposition, the triangular-binding fixpoint), and `kernel-strata` (the certified level oracle's checkable evidence).
3. The **corpus documents that carry this design's premises** — the typing machine's solver separation and agreement property, the domain interface and its four rules, the three-scheduler rule, the type system's constraint language, the trusted base, the certificate algebra, and the coherence economy — which this document links rather than restates.
4. The **published literature**: nested pattern unification [@kovacs-2023-nested-pattern-unification], dynamic patterns [@abel-pientka-2011-dynamic-pattern-unification], the pattern-matching boundary [@richter-bohler-2026-pattern-matching-unification], the two soundness templates [@behr-krivine-2021-conditions-compositionality] [@behr-harmer-krivine-2021-nonlinear-concurrency], the Z3-delegation precedent [@behr-heckel-saadat-2020-graph-overlaps-z3], the neutral-terms decision procedure [@allais-boutillier-mcbride-2013-neutral-terms], the extra-structure thesis [@hewer-2024-types-extra-structure], and the related-work records named in [[#What this posture is not]].

**Confidence, by class.**

* **High** — the fragment statement, the interface reconciliation, and the six rules, which restate the ratified direction and existing corpus contracts rather than derive new ones.
* **High** — the as-built statements, each verified against the named module or document at write time, including the distinction between the engines' first-order `unify_cmd` and this document's higher-order domain.
* **Medium** — the reading of the solver's shared-substitution scheduling onto the nonlinear-concurrency template: a template application, not a proved theorem, and recorded as such.
* **Marked at the claim** — the OTT-flavoured postponed-problems gesture, whose anchor reports the proposal without developing it; and the two related-work entries not held in the research library (variant narrowing, deduction modulo), synthesized from sweep-verified locators with the works unread here.
