# Strengthening via unification

**Proposed.
No strengthening service, no unifier domain, no metavariable, and no elaborator exists in this tree.** What exists is everything this specification is a client of: the solver as a separate, serializable machine with its agreement property ([[../typing-machine#The solver as a separate machine]]), the constraint language whose instantiation row the unifier domain owns ([[../type-system#The constraint language]]), the Squier-completed cell layer that is the unifier's adopted theory backend ([[../../implementation#The engines]]), the certified level oracle and the replayed-not-trusted admission discipline ([[../../implementation#The trusted base]]), the built grade system with its usage accounting and erasure (`gandr-core-checker`'s `grade`, `checker`, and `kernel_bridge` modules), and the identity fragment's kernel formers at rung 1 — `Path` with an explicit motive, introduced by `here` and eliminated by `walk` ([[../../implementation#The checked language]], [[../../metatheory#Directed univalence]]).

This document specifies **strengthening as a solver-machine service over the unifier**: deciding that a term in a context is equal, up to definitional equality, to a weakened term from a smaller context — and it specifies that decision as two ordinary unification problems through the interface of [[pattern-unifier]], never as a free-variable analysis, which is refuted as a design below.
The service arrives ahead of need: the definitional equality it presumes — walk-β, congruence, substitution laws, reduction inside types — is the identity layer's own phase, not the current rung ([[../../implementation#The checked language]]), and the dependent-core phase that owns the use sites is open ([[../../implementation#The build-out at a glance]]).

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The solver is a separate machine from the recursive checker**, with its own serializable state, and the two realizations are property-tested for step-for-step agreement on a control log — `gandr-core-checker`'s `machine` and `checker` modules ([[../typing-machine#The solver as a separate machine]]).
* **The grade system is built.** `gandr-core-checker`'s `grade` module carries the preordered semiring over the naturals with infinity; `checker.rs`'s `rule_dup`/`rule_drop` and the machine's `Dup`/`Drop` frames enforce the conservation $r + s subset.eq g$; and the kernel bridge erases `dup v` to `return (v, v)` and `drop v` to `return ()` ([[../type-system#Grades]]).
* **The identity fragment is built at rung 1** — `Path A x y` introduced by `here(v)`, eliminated by the full dinatural `walk` with an explicit motive (β only; no η, no K; Paulin–Mohring forms derived, never primitive) — alongside `Σ` dependent pairs with the motive-carrying split eliminator ([[../../implementation#The checked language]]).
* **The unifier's interface is specified**, with its fragment, its step outcomes, its postponement bookkeeping, and its evidence form ([[pattern-unifier]]); the scheduling discipline that governs its postponements is [[elaboration-schedulers]].

**Designed, and not built.** The strengthening service itself: the two emitted problems, the outcome mapping, and the grade interaction.

## The problem is judgemental, not syntactic

**Strengthening.** Given $Gamma, x : A, y : B tack t : T$, decide whether $t$ and $T$ _factor through the weakening_ from $x : A$ to $x : A, y : B$ — whether there exist $S^*$ and $s^*$ in the smaller context with $T equiv S^* med x$ and $t equiv s^* med x$, where $equiv$ is the system's own definitional equality.

The question is judgemental: "lives in a smaller context" means _equal, up to definitional equality, to a weakened term_.
The obvious implementation — compute the set of free variables and check that $y$ is absent — answers a different, syntactic question, and the design record's critique of it is adopted here as a **standing refutation** [@sterling-2026-pterodactyl-worklog, tree 01KU], in escalating order:

1. **β-instability.** A dependency can be removable by β-reduction, so a variable that _occurs_ is not necessarily a dependency; free-variable sets are not stable under definitional equivalence.
2. **η-laws and definitional singletons.** Filtering occurrences through definitional singletons fails too, because relevance is about _use_, not occurrence: $x : "Unit" times bb(N)$ is not an irrelevant variable, yet it is used _irrelevantly_ in the projection $x .1$.
3. **The normal-form gap.** The only non-broken free-variable computation runs on the β-short η-long normal form — and essentially no implementation computes it there.

The failure's shape is one: free-variable sets are a _syntactic_ answer to a _judgemental_ question, and any procedure that does not itself respect definitional equality answers a different question.
These three classes are this document's acceptance tests ([[#The acceptance tests the design owes]]).

## The construction

To strengthen $Gamma, x : A, y : B tack t : T$ into the context $x : A$, the service emits two ordinary unification problems, in the constraint forms the unifier domain owns (`Γ ⊢ T ≐ T′` and `α := T`, [[pattern-unifier#The solver-machine interface]]) [@sterling-2026-pterodactyl-worklog, tree 01KV]:

1. **The type factors.** Emit a metavariable `?S : (x : A) → Type` and unify `Γ ⊢ ?S x ≐ T`.
   A solution $sigma$ with $sigma(?S) = S^*$ is exactly a strengthening of the _type_ away from $y$.
2. **The term factors.** Emit a metavariable `?s : (x : A) → ?S x` and unify `Γ ⊢ ?s x ≐ t`.
   A solution is the strengthened term.

Both constraints sit inside the predictable fragment by shape: a metavariable applied to distinct bound variables is a Miller pattern, and the record/Σ nesting a retained telescope may carry is native to the fragment ([[pattern-unifier#What is in]]).
The two-step order is load-bearing, not an implementation detail: the term problem's own type `?S x` is well-formed only once the type problem has solved.

**Why this is semantics-respecting by construction.** A solution is a _proof, in the system's own definitional equality, that the type and the term factor through weakening_: the unifier's output is a substitution whose application re-checks by replay ([[pattern-unifier#The evidence form]]), so the service's positive answer is checkable evidence in the form the admission discipline already replays ([[../../implementation#The trusted base]]) — the pragmatic instance of the proof-relevant reading of unification, where a unifier is itself evidence rather than a verdict [@cockx-devriese-2018-proof-relevant-unification].
No occurrence is ever inspected; there is no syntactic proxy in the construction to be wrong about.
Read categorically, the answer is an equalizer in the substitution category — a morphism, whose universal property does the work an occurrence analysis only gestures at ([[pattern-unifier#Unification is an equalizer — the adopted frame]]).

**Why the fragment's own properties are exactly the missing ones.** The fragment respects η for functions and records natively — η-expansion is part of the fragment, so an equation that holds only up to η does not postpone — and definitional singletons are accounted in constraint solving ([[pattern-unifier#What is in]], per nested pattern unification [@kovacs-2023-nested-pattern-unification]).
Those are precisely failure classes two and three, dissolved by the choice of machine rather than by a patch.

## The contract

### strengthen-rule-01

**Strengthening is decided only by the two unification problems.** No free-variable or occurrence analysis enters the strengthening pipeline at any layer — kernel, solver, or elaborator.
The refutation of [[#The problem is judgemental, not syntactic]] is the standing reason; where a cheaper check is wanted it is the grade system ([[#The grade interaction runs one way]]), not a second syntactic analysis with the same defect.

### strengthen-rule-02

**The service is a client of the unifier domain, not a new domain and not new machinery.** It emits constraints in the domain's own forms and consumes the domain's step outcomes; every step is a solver-machine step ([[pattern-unifier#unifier-rule-06]]), so strengthening derivations enter the solver/checker agreement property and the serialization discipline with no new mechanism.
The service binds no metavariable outside the two it emits, and those two are ordinary solver cells.

### strengthen-rule-03

**The outcome mapping has three cases, and each is honest.**

| unifier outcome on `Γ ⊢ ?S x ≐ T` | the service's answer                                                                                                                            |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| a solution $sigma$                | the strengthened type $S^* = sigma(?S)$; proceed to the term problem                                                                            |
| a postponement                    | **block the refinement step** — no coercion escape hatch ([[#Postponement and the scheduler boundary]])                                         |
| failure inside the fragment       | a _proof of dependence_: no MGU exists, so no factorization exists; the consumer is told "genuinely depends on $y$" — information, not an error |

The term problem's outcomes map the same way, with the strengthened term $s^*$ as the positive case.

### strengthen-rule-04

**The evidence is the unifier's evidence.** A successful strengthening returns the substitutions and their replay obligation, exactly the evidence form of [[pattern-unifier#The evidence form]]; the admission choke point re-derives, and no verdict enters the trusted base.
A strengthening is self-certifying where a free-variable computation is an opaque verdict.

### strengthen-rule-05

**A successful strengthening may justify a grade assignment; a grade never justifies a strengthening.** The one permitted direction, stated in full at [[#The grade interaction runs one way]].

## The acceptance tests the design owes

The three failure classes of the refuted design are this service's acceptance tests.
Each is stated with the by-construction argument that the failure class cannot occur, then the observable behaviour the implementation must demonstrate.

### strengthen-test-01

**β-instability.** _Class:_ a dependency removable by β-reduction is still an occurrence, and free-variable analysis sees it.
_By construction:_ no occurrence analysis runs.
The unifier decides convertibility modulo the declared rule layer — it normalizes against completed cells and compares canonical forms ([[pattern-unifier#unifier-rule-03]]) — so a β-removable occurrence of $y$ is absent from the canonical forms the comparison inspects, and a solution exists exactly when the factorization does.
_Behaviour:_ a target in which $y$ occurs only under β-redexes — the shape $(lambda z. c) med y$ with $z$ not free in $c$, definitionally $c$ — strengthens positively, with replayed evidence.

### strengthen-test-02

**η-laws and definitional singletons.** _Class:_ $x : "Unit" times bb(N)$ used only as $x .1$ — relevance is about use, not occurrence, and filtering occurrences through singleton types does not recover that.
_By construction:_ the fragment respects η for functions and records and accounts definitional singletons in constraint solving ([[pattern-unifier#What is in]] [@kovacs-2023-nested-pattern-unification]); "use up to definitional equality" is precisely the relation the unifier decides, so an occurrence whose content an η-law or a singleton pins contributes nothing to the solution's dependence.
_Behaviour:_ a target whose every use of the eliminated variable lands at a definitional-singleton type — $y : "Unit"$, where η gives $y equiv "tt"$ — strengthens positively.

### strengthen-test-03

**The normal-form gap.** _Class:_ the only correct free-variable computation runs on the β-short η-long normal form, and essentially nobody computes it there.
_By construction:_ the construction has no representative to be wrong about.
The comparison the service runs _is_ the theory's equality — there is no separate syntactic pass that could be run on the wrong representative, so the gap between the normal form the check needs and the term the check sees has no counterpart.
_Behaviour:_ targets whose dependence vanishes only under combined β- and η-laws strengthen positively, with no caller-side normalization discipline.

## Where strengthening arises

Every eliminator-shaped elaboration asks, of a motive or a telescopic equation, _does this actually depend on that variable?_; where the answer is no, the context contracts and the problem simplifies.
The use sites, stated in prose against the phase vocabulary rather than against a sibling eliminator design that does not yet exist:

* **Telescope contraction in eliminator refinement.** The elimination-with-a-motive transformation generalises a goal over the eliminated hypothesis's indices and then specialises; splitting the telescope into the part the motive depends on and the part it does not is a strengthening per variable [@mcbride-2002-elimination-motive].
  The case-analysis primitives this serves descend from the calculus-of-constructions tradition's treatment of (co)inductive elimination [@gimenez-1996-thesis].
* **Pruning unused hypotheses after case analysis.** A case split that instantiates an index can leave later hypotheses definitionally independent of earlier ones; pruning them is strengthening, and skipping the prune is how telescopes grow quadratically through a refinement.
* **Generalising motives over indices.** For `walk` specifically — the identity former's motive-carrying eliminator, built at rung 1 ([[../../implementation#The checked language]], [[../../metatheory#Directed univalence]]) — specialising the motive against a constructor-form target asks whether the motive's family depends on the path variable; on `here`, a detectable constructor, it frequently does not, and strengthening is the step that discovers this _definitionally_ rather than by convention.
  The same question recurs for every (higher) inductive declaration, whose induction principles are computed uniformly from signatures [@kaposi-kovacs-2020-hiit-signatures]; the motive generalisation each one needs is a strengthening instance.

The non-trivial case the design record names: a family in which the path variable occurs, but whose every use is a transport along a path that computes to `here` — definitionally the constant family, once walk-β lands with the identity layer's own phase.
Free-variable analysis sees the variable and refuses; the unification reduction accepts, because the solution is found against the family's _normal form_.

## Postponement and the scheduler boundary

A postponement of the type problem **blocks the refinement step**: the constraint resumes when further instantiation brings it inside the fragment — the dynamic-pattern discipline, computed per sub-problem at solve time ([[pattern-unifier#The boundary is a solve-time computation]] [@abel-pientka-2011-dynamic-pattern-unification]) — and until then the consumer waits.
There is **no coercion escape hatch**: the design record's parenthetical alternative — block, or insert a coercion — is taken in its first disjunct only, because coercion fires only at a marked site, and no coercion layer exists for a strengthening postponement to fire in ([[elaboration-schedulers#sched-rule-04]]).
The reversal condition is recorded at [[#Open dispositions]].

Two outcomes are deliberately distinct from postponement:

* **Failure inside the fragment is a proof of dependence** ([[#strengthen-rule-03]]), and it is sharp: in a unitary fragment, the MGU's absence is exactly the factorization's non-existence.
* **A strengthening whose solution needs case analysis is not postponed; it leaves the unifier entirely** — exported to the pattern-matching compiler's lane as a named hand-off ([[pattern-unifier#unifier-rule-05]]).

## The grade interaction runs one way

gandr's graded typing is built and tracks _usage counts_: grades form a preordered semiring ([[../type-system#Grades]] — `gandr-core-checker`'s `grade` module), `Dup` enforces that split grades sum below the thunk's grade, and a binding whose grade licenses discarding erases freely — `drop v` lowers to `return ()` in the kernel bridge, and the zero-graded capture of [[../type-system#type-extension-01]] provably need not ship.
Grades are therefore the cheap first tier — the coherence economy's "don't generate" ([[../../metatheory#The coherence economy]]): where a grade already says _unused_, no strengthening question needs asking.

But **grades see usage, not definitional removability.** A variable used once, at grade 1, whose every use is β-removable or lands in a definitional singleton, is genuinely independent — and its grade says so nowhere.
Strengthening-via-unification is the general mechanism; grades remain the fast path that avoids invoking it.

The interaction is therefore one-directional ([[#strengthen-rule-05]]): **a successful strengthening, evidence in hand, may justify a grade assignment** — the factorization proof is exactly the licence an erasure or a prune needs — **and a grade never justifies a strengthening**, because grade-0 is a usage claim, not a factorization proof.
Treating it as one would reintroduce the syntactic-answer-to-a-judgemental-question defect one layer down.

## Two designs this is not

**Agda-style occurrence analysis.** The field's default implementation of strengthening is a free-variable traversal, and the design record's critique applies to it in full: it carries all three failure classes.
Its bug record is cited here as the design record's assessment [@sterling-2026-pterodactyl-worklog, tree 01KU], not as an independently audited issue list — marked accordingly.
[[#strengthen-rule-01]] forbids importing the design at any layer.

**Observational-type-theory functorial strength.** The semantic extreme: where types are read as functors — containers, in the strict-positive case [@abbott-altenkirch-ghani-2005-containers] — every type expression carries a functorial _strength_, and a type expression not depending on a variable is a constant functor, for which the strength is an isomorphism; that isomorphism _is_ the strengthening, a theorem about the functor rather than a syntactic check. gandr's construction sits on this side of the comparison in kind — the answer is a morphism (an equalizer in the substitution category) with a universal property — but it is computed by one generic solver run rather than by per-former theorems, which is what makes it an elaboration-time service rather than a metatheorem library.

## Open dispositions

Every open item the design record leaves carries exactly one disposition.

* **Strengthening under metavariables** — carried.
  When the target mentions unsolved metavariables, the emitted problems are not yet pattern problems; the interaction with the postponed set — whether the service enqueues behind it or emits into it — is decided when the domain is implemented, and [[#strengthen-rule-03]]'s three-case mapping is stated to survive either answer.
* **Functoriality and identity strengthening** — carried.
  Two composites the design record leaves unexamined: strengthening along `here` (the identity factorization, which should short-circuit rather than solve), and functoriality of iterated strengthening under successive context splits.
  Neither changes the contract; both are composition-and-cost questions for the implementation.
* **The definitional-singleton interplay** — carried, gated on the unifier's twin item ([[pattern-unifier#Open dispositions]]): which of the checked language's connectives count as record/Σ structure, and which types are definitional singletons, is fixed when the domain is implemented against the full type formers, and no scheduling decision here depends on the answer.
* **The coercion escape hatch** — declined, with a reversal condition.
  The design record's "block, or insert a coercion" is taken as block-only ([[#Postponement and the scheduler boundary]]).
  The condition that reopens it: a coercion layer existing at all — the derived `Path`→`Flow` form is the standing candidate, gated on the core-coincidence theorem ([[../../metatheory#Directed univalence]]) — at which point [[elaboration-schedulers#sched-rule-04]] says where the crossing is marked.
* **A free-variable fast path in the kernel** — declined, with a reversal condition.
  The refutation is standing ([[#The problem is judgemental, not syntactic]]), and the cheap-check slot is occupied by the grade system.
  Reopens if a demonstrated need arises that the grade system cannot serve — a reason about the machinery, not about taste.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **design record for the absorption**: the Pterodactyl worklog's strengthening trees [@sterling-2026-pterodactyl-worklog, trees 01KT, 01KU, 01KV], read live at registration (2026-08-07); the three failure classes, the two-problem reduction, and the declined alternatives are restated in full so this document stands alone.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `machine` and `checker` modules (the two realizations and their agreement testing), its `grade` module (the preordered semiring), `checker.rs`'s `rule_dup`/`rule_drop` and the machine's `Dup`/`Drop` frames (the conservation $r + s subset.eq g$), and `kernel_bridge.rs` (the erasures of `dup v` and `drop v`).
3. The **corpus documents that carry this design's premises** — the typing machine's solver separation, the constraint language, the trusted base, the grade rules with the per-assumption grading extension, the coherence economy's tier ordering, the identity former's kernel record, and the three sibling specifications this one is a client of ([[pattern-unifier]], [[elaboration-schedulers]], [[solver-interface]]) — linked rather than restated.
4. The **published literature**: nested pattern unification [@kovacs-2023-nested-pattern-unification], dynamic patterns [@abel-pientka-2011-dynamic-pattern-unification], elimination with a motive [@mcbride-2002-elimination-motive], the (co)inductive-elimination thesis lineage [@gimenez-1996-thesis], HIIT signatures [@kaposi-kovacs-2020-hiit-signatures], the proof-relevant reading of unification [@cockx-devriese-2018-proof-relevant-unification], and containers for the functorial-strength comparison [@abbott-altenkirch-ghani-2005-containers].

**Confidence, by class.**

* **High** — the construction, the contract rules, and the three by-construction arguments, which restate the adopted direction against the unifier's specified interface rather than derive new machinery.
* **High** — the as-built statements, each verified against the named module or document at write time.
* **Medium** — the functorial-strength comparison and the motive-over-indices generality claim, read from the cited works' accounts rather than from experience with the systems.
* **Marked at the claim** — the "known bugs" characterization of occurrence-analysis strengthening (the design record's assessment, not an audited issue list), and the proof-relevant-unification citation, whose locator was verified at registration but whose text is unread here.
