# The fuss-free universe discipline

**Proposed.
No smallness judgement, no lift-inserting elaborator, and no displacement machinery exists in this tree.** What exists is everything the layer decides with: the certified level oracle and the written-lift discipline of [[../implementation#The trusted base]], plus the per-declaration level interface the kernel checks against.
The elaborator itself is also unbuilt — the dependent-core phase that owns it is open ([[../implementation#The build-out at a glance]]) — so this document specifies a discipline whose decision machinery is built and whose host is not.

The design does three things.
It **states the elaborator's level policy as a three-rung ladder with a fixed fallback order**: the smallness judgement, then displacement, then explicit prenex level polymorphism.
It **states the smallness judgement's rules against the kernel's existing evidence forms**, so the layer costs no new decision procedure and no kernel change.
And it **draws the boundary with explicit polymorphism by example**: smallness absorbs subsumption and nothing more, and the presheaf/Yoneda development is the witness of what it does not absorb.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The level algebra and its canonical form** — `gandr-kernel-strata`'s `level` module: a level is the canonical finite join over the algebra of zero, successor, and join, canonical form a constructor invariant, so derived structural equality _is_ level equality.
* **The free order oracle, with evidence both ways** — the crate's `order` module: `Level::leq_with_evidence` and `Level::lt_with_evidence` decide by domination and return a `LeqWitness` (each left atom paired with its dominating bound) or a `LeqRefutation` (a concrete counter-valuation), with `validate_witness` and `validate_refutation` checking either against the two levels.
* **Landmark-poset admission and entailment** — the `poset` and `entail` modules: `LandmarkPoset::admit` returns an admitted poset carrying a `ConsistencyWitness` (an explicit ℕ-homomorphism, validated by evaluating every constraint under it) or refuses with a replayable `LoopWitness` (a pumping derivation); `entails_leq_with_evidence` and `entails_lt_with_evidence` return an `EntailmentWitness` (a replayable forward derivation) or an `EntailmentCountermodel` (the minimal model refuting a goal atom), each with its validator [@bezem-coquand-2022-loop-checking].
  With no constraints declared, entailment agrees with the free oracle on every input — pinned by the crate's property differential.
* **The per-declaration level interface** — `gandr-kernel-core`'s `decl` module: a declaration's `LevelSignature` is a prenex parameter count plus declared landmark constraints, admitted at `LevelContext::admit` in the `levels` module, where an inconsistent constraint set is rejected carrying the `LoopWitness`.
* **The universe rule and written lifts** — `LevelContext::check_universe_below` decides $U_ell : U_m$ iff $ell < m$ (landmark entailment when hypotheses exist, the free oracle otherwise); `ValueType::Lift` and `Value::Lift` in the `types` and `term` modules are explicit formers whose strictness is checked the same way.
  The lift is written, never inferred, and a failure surfaces as `KernelError::UniverseViolation` carrying the oracle's refutation evidence.

**Designed, and not built.** Everything else here: the smallness judgement and its rules, large-by-default elaboration, lift insertion at checked sites, displacement shunting, the rung ladder itself, and every diagnostic this document describes.

**One as-built refusal is load-bearing for the whole design, and is worth stating precisely.** The kernel crate's own documentation lists what it refuses to hold: level inference, unification, **generalization**, and **displacement**, alongside cumulativity, `imax`, `Prop`, and constants in declared constraints.
So none of the three rungs is a kernel feature in waiting — rung 2's displacement and rung 3's generalization are refused by name in the trusted base, which is exactly what makes this an elaborator layer rather than a kernel extension.
The kernel changes nothing.

## The three rungs

The elaborator's level policy is a ladder, tried in order, each rung a superset of the previous rung's cost.

| rung                  | mechanism                              | what the author writes | what the elaborator does                                            | cost                                        |
| --------------------- | -------------------------------------- | ---------------------- | ------------------------------------------------------------------- | ------------------------------------------- |
| [[#universe-rung-01]] | the smallness judgement                | nothing                | discharges smallness side-conditions; inserts written lifts         | one oracle call per checked site            |
| [[#universe-rung-02]] | displacement                           | nothing                | reuses a definition shunted along the level preorder                | one uniform shift per reuse; zero solving   |
| [[#universe-rung-03]] | explicit prenex level polymorphism     | the level signature    | checks the declaration against its own admitted landmark poset      | the declaration's declared constraints      |

The fallback order is itself a coherence-economy tiering ([[../../metatheory#The coherence economy]]) applied to levels: **don't generate** (rung 1 — decide cheaply, and no level is ever materialized for the author), **dissolve** (rung 2 — one uniform shift settles reuse for a whole development, at a cost independent of its size), **decide** (rung 3 — the oracle decides the declared poset's entailments, with evidence).
The economy's fourth tier, **generate**, is the one this policy refuses outright: there is no level inference anywhere in the stack for a fallback to fall into.

### universe-rung-01

**The smallness judgement is the default.** Types are formed large by default — the _stay large_ discipline of the fuss-free formulation [@sterbac-sterling-2026-fuss-free, sec 1.2.3]: a construction carries no level annotation, and nothing in a definition constrains the size of what it defines.
Smallness is asked only where a type is used as data — checked against a universe — and the elaborator discharges it, writing an explicit lift where the levels differ.
Its rules are [[#The smallness judgement, against the kernel's evidence forms|stated below]].

### universe-rung-02

**Displacement is the default reuse mode** — the elaborator half the implementation roadmap already pins ([[../roadmap#The elaborator half of universe stratification]]): reuse a definition by shunting its whole level signature along the level preorder, with no solving.
This is McBride's crude-but-effective stratification [@mcbride-2012-crude-stratification] — every top-level definition behaves as if polymorphic in a secret level parameter, and whole developments are displaced upward as needed — in the form the order-theoretic analysis makes precise: a displacement is an order-preserving action on levels, and displacement algebras characterize which policies are sound [@hou-favonia-angiuli-mullanix-2023-order-theoretic].
A shift applies to every level in the development uniformly, written lift targets included, so elaborated terms are preserved under it rather than re-derived; and admissibility reduces to re-admission of the shifted landmark poset, which loop-checking already decides — a shift that would break a declared constraint is refused with the loop witness, not discovered downstream.

### universe-rung-03

**Explicit prenex level polymorphism is for genuine level parameters.** The declaration carries its own prenex level parameters and its own declared landmark poset — the interface the kernel already builds, admits, and checks against — and the level is an input to the meaning of the declaration, not a fact about any one type.
This is the corpus's adopted explicit-polymorphism line [@bezem-coquand-dybjer-escardo-2022-explicit-universe-polymorphism], and it costs no new machinery: rung 3 is the kernel's built declaration posture, surfaced to the author.

## The smallness judgement, against the kernel's evidence forms

The judgement is $"small"(A, ell)$ — a **judgemental smallness relation** on types, the design pattern of the fuss-free formulation [@sterbac-sterling-2026-fuss-free, sec 1.2.1].
That formulation's starting observation is that the injectivity of decoding [@sterbac-sterling-2026-fuss-free, prop 1.2] makes the decoding's image a derivable sort, so a universe can be replaced by its image: rather than axiomatizing per-level codes, decodings, and lifts with their functoriality and naturality equations, axiomatize which types are small at $ell$ — lifts and their equational theory then become admissible rather than primitive, and the equivalence with the coherent-lift hierarchy is proved by a normalization theorem.
In the design record's accounting, the universe-related core operations drop by two-thirds [@sterling-2026-pterodactyl-worklog].

gandr's instance of the pattern is sharper still, because the layer does not even need the formulation's closure rules: **smallness is defined derivatively as a level inequality** — $A$ is small at $ell$ exactly when $A$'s natural level is at most $ell$ under the declaration's landmark poset — and every question that leaves is one the kernel's oracle already decides, with evidence:

| level question the layer asks                       | decided by                                                  | evidence, either way                                                                   | home                                   |
| --------------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------- |
| level equality                                      | the canonical form — structural equality is level equality  | —                                                                                      | `gandr-kernel-strata`, `level`         |
| $ell lt.eq m$, no hypotheses                        | domination                                                  | `LeqWitness` / `LeqRefutation` (a counter-valuation), with validators                  | `gandr-kernel-strata`, `order`         |
| $ell lt.eq m$ under the landmark poset              | loop-checked entailment                                     | `EntailmentWitness` / `EntailmentCountermodel`, with validators                        | `gandr-kernel-strata`, `entail`        |
| landmark-poset admission                            | the loop-checking dichotomy                                 | `ConsistencyWitness` (an ℕ-homomorphism) / `LoopWitness` (a replayable pumping loop)   | `gandr-kernel-strata`, `poset`         |
| $U_ell : U_m$; a lift's strictness                  | `LevelContext::check_universe_below`                        | `KernelError::UniverseViolation`, carrying the refutation                              | `gandr-kernel-core`, `levels`          |

The paper's smallness rules have direct readings under the definition: closure under the connectives is a theorem of the natural-level walk (the arrow join, the universe successor) rather than an axiom; smallness monotonicity is the lift insertion of [[#smallness-rule-03]]; and a universe's own smallness one level up is the kernel's universe rule.

That the judgements this leaves behind are decidable is the level-oracle mapping of the design record: gandr's levels are the free semilattice with successor on the level variables, and the smallness and level-inequality judgements over it are decided in the neutral-terms decision-procedure style of [@allais-boutillier-mcbride-2013-neutral-terms], in its NbE-with-free-extensions reformulation [@corbyn-kammar-valliappan-yallop-2022-nbe-free-extensions] — realized here as the canonical form maintained by construction, with ordering and entailment decided by domination and loop-checking rather than by normalization.

### smallness-rule-01

**Formation never asks a smallness question.** A type is elaborated at its natural level, computed compositionally by the same walk the kernel's checker already performs (`gandr-kernel-core`'s `type_level`: the universe successor, the arrow join, the lift's target).
Nothing in a definition is required to be small; a type that never leaves type position never meets the judgement at all.

### smallness-rule-02

**Smallness is demanded only at checked sites.** The side-condition arises exactly where the bidirectional discipline checks a type against a universe $U_ell$ — a type used as data.
The elaborator discharges it as an entailment — the type's natural level against $ell$ under the declaration's landmark poset — decided by `Level::leq_with_evidence` in the free case and by `LandmarkPoset::entails_leq_with_evidence` under hypotheses.
Either way the answer carries checkable evidence, and no level is solved for.

### smallness-rule-03

**Where the levels differ, the elaborator writes the lift.** A type checked against $U_ell$ whose natural level is strictly below $ell$ elaborates to the kernel's explicit `Lift` former with target $ell$ — a node in the checked term, strictness-verified by `check_universe_below`, visible to export and replay like any other written lift.
The kernel's rule — lifts written, never inferred — is untouched; the elaborator is simply where the writing happens.
Inserted lifts are cast-visible where [[elaboration-schedulers|the three-scheduler rule]] wants a coercion marked: lift insertion is the cast-sited coercion scheduler's business, it never calls and is never called implicitly by unification or by canonical-instance resolution, and the absence of two competing insertion paths at one site is owed by that discipline rather than by equations in conversion.

### smallness-rule-04

**Checked, never solved.** The elaborator creates no level metavariables and poses no most-general-unifier problem over level expressions: every level it manipulates is ground in the declaration's level signature, and every level question it asks is an entailment the oracle decides.
This is what absorbs the surface [[../../metatheory/roadmap#meta-question-13]] names — stuck max-plus level equations as an unsolved user-experience surface gandr must own.
The stuck equation cannot arise here, because nothing is solved for: ordinary code states no levels, so the elaborator is never asked to find any.
Where an entailment genuinely fails, the answer is not a solver that guesses but a development that moves up a rung — and at rung 3 the level is something the author said, which is the honest form of owning the surface.

### smallness-rule-05

**Failure is a witness, not a stuck constraint.** A refused smallness side-condition surfaces the oracle's refutation evidence — the `LeqRefutation`'s counter-valuation in the free case, the `EntailmentCountermodel`'s minimal model under hypotheses — and a declaration whose landmark constraints have no model is refused at admission with the replayable `LoopWitness`.
Trust stays concentrated in the validators, per the kernel's evidence posture, and the diagnostic is the evidence rendered, not a solver state.

## The boundary: presheaves and the Yoneda embedding

Smallness is a judgement on concrete types, and it absorbs **subsumption** — a construction usable at a larger level than the one it was formed at — which is the vast majority of what level annotations are ever written for.
It does not absorb **abstraction over a level itself**: one definition instantiated at two different levels within a single scope, where the relationship between the instances is a function of the level and not a fact about a type.
The witness is the standard library-grade one, elaborated on paper at all three rungs.

**Rung 1 — any one use, zero annotations.** A `Category` development defines a category as composition-and-identity structure on a carrier, with no level anywhere: the carrier is a type, the homs are types, and nothing in the definition constrains their size.
Used at any single level — a concrete category of sets, of groups, of the types of one universe — the elaborator discharges every smallness side-condition the use raises; the author writes none.

**Rung 2 — the development reused, zero annotations.** The same `Category` development, needed one universe up because the ambient development grew, is displaced: the whole signature shunted along the preorder, written lifts shifted with it, no solving, no annotations, and the shifted landmark poset — here empty — re-admitted by the same oracle.

**Rung 3 — one statement, two instantiations, the level as input.** The presheaf construction and the Yoneda embedding relate adjacent levels, in paper notation rather than surface syntax:

```text
-- Category, made level-polymorphic: its level signature declares
-- one parameter and its landmark poset (here the trivial one)
Category { level params (l); landmark poset () } where
  carrier : Type(l)
  hom     : carrier -> carrier -> Type(l)
  ...

-- the presheaf category of a level-l category is a level-(l+1) category:
PSh(C) : Category at l+1        -- objects are presheaves over C

-- Yoneda relates the two levels inside one statement:
yoneda(C) : Functor(C, PSh(C))  -- Category instantiated at l and at l+1
```

Stating `yoneda` for a single polymorphic `Category` requires the level variable, because `Category` is instantiated at $ell$ and at $ell + 1$ inside one theorem — and there is no concrete smallness judgement that relates the two instances, since the relationship is a function of the level.
The declaration's own landmark poset — trivial in this instance, and non-trivial the moment the development also relates an ambient category at a constrained level — is declared, admitted by loop-checking, and every universe question inside the declaration is landmark entailment the oracle decides with evidence.

The rule the example teaches: **fuss-free covers every use where the level could have been left unwritten; explicit prenex covers the uses where the level is a genuine input.** Library-grade structure towers — categories, functors, presheaves, completions, the descent constructions — are the second kind; application code is the first.

**The working bet, recorded as a bet.** That rungs 1 and 2 eliminate the level-polymorphism burden in ordinary developments almost entirely, with rung 3 remaining for structure towers, is a hypothesis the executable corpus tests — a development class that routinely needs rung-3 signatures in application-shaped code refutes it.
Nothing in this document's rules assumes it.

## What this layer refuses

* **Kernel cumulativity stays refused.** Silent subsumption in the trusted base is the coherent-hierarchy price — per-connective codes at every level, decoding laws, definitional functoriality and naturality of lifts, definitional injectivity of both [@sterbac-sterling-2026-fuss-free, sec 1.1.2] — that gandr's kernel does not pay in either its ruled or its fuss-free form.
  The elaborator buys cumulativity's ergonomics by inserting written lifts at checked sites.
  **Reversal condition:** a demonstrated class of real developments where elaborator-inserted lifts are observably the wrong default — a representation question about elaborator policy, never a commitment question about the kernel's rules.
* **Levels as a type stay declined.** Agda-style first-class levels would internalize the level semilattice as a datatype; the semantic caveat that settles the question is that the strict semilattice laws are not guaranteed in higher-topos models, and gandr's declaration-level prenex discipline is the stratified line the corpus already builds.
  Recorded as settled, not reopened.
* **Implicit universe polymorphism is not the answer.** Typical ambiguity and constraint-graph inference do not remove the complexity; they move it somewhere the author cannot see, and somebody inevitably debugs the constraint graph. gandr refuses level inference at the kernel and does not reintroduce it at the elaborator — what rung 3 makes explicit is exactly what such systems leave implicit.

## Open dispositions

Each item below arrived open from the design record and carries exactly one disposition.

**The working bet is carried as open**, with its falsifier named in the boundary example above.

**The kernel-cumulativity decline carries its reversal condition**, recorded with the decline above.

**The levels-as-a-type decline is recorded as settled** and is not reopened here; that the fuss-free discipline composes with declaration-level explicit polymorphism, rather than presupposing first-class levels, is what the three-rung ladder states.

**Cast-visibility of inserted lifts is delegated, not restated.** [[elaboration-schedulers|The three-scheduler rule]] owns where a coercion is user-marked; this document states only that lift insertion is that scheduler's business and never an implicit call between schedulers.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **design record** for this adoption, in full — the coherent-hierarchy checklist read as a price list, the judgemental-smallness reformulation and what it makes admissible, the stay-large discipline, the level-oracle mapping, the two declines with their deltas, the three-rung ladder, and the boundary example.
   It digests Jon Sterling's Pterodactyl worklog [@sterling-2026-pterodactyl-worklog], the public source of record for the trees it summarizes.
2. **The tree**, for every as-built claim: `gandr-kernel-strata`'s `level`, `order`, `poset`, and `entail` modules, and `gandr-kernel-core`'s `decl`, `levels`, `types`, `term`, `check`, and `error` modules, read at write time.
3. The **corpus documents that carry this design's premises** — the trusted-base account, the implementation roadmap's elaborator-half pinning, the metatheory roadmap's open question, and the coherence economy — linked rather than restated.
4. The **published foundation**, read for this pass at the grades named: the fuss-free paper's introduction and key-ideas sections in the held extended version [@sterbac-sterling-2026-fuss-free, sec 1, sec 1.2.1, sec 1.2.3]; the remaining entries at abstract or metadata grade only.

**Confidence, by class.**

* **High** — the as-built statements, each verified against the named module at write time.
* **High** — the three-rung policy, the smallness rules, the boundary example, and the declines, which are transcribed from the design record rather than re-derived.
* **Medium-high** — the claims about the fuss-free formulation itself: the checklist, the injectivity propositions, the smallness judgement, and the stay-large discipline were read directly in the held extended version; the equivalence-by-normalization claim is cited from its statement, and the proof was not read.
* **Medium** — the loop-checking dichotomy and the entailment evidence forms, stated from the kernel crates' own documentation of what they implement [@bezem-coquand-2022-loop-checking]; the paper itself was not read this pass.
* **Marked at the claim** — the McBride slides' date (the held artifact prints none) and the Corbyn et al. entry (held, with no stable identifier recorded).
