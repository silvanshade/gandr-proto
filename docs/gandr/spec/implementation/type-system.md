# The type system

The checked language is a **call-by-push-value bidirectional type system** [@levy-cbpv] with polarity-sorted unions and intersections, explicit System-F-style polymorphism, graded thunks, binary and multiparty session types, manifest sharing, and world modalities with capability-gated migration.

The presentation is layered, and the layering is a dependency order rather than a narrative one.
The grade algebra and the context discipline are fixed first because everything else uses them; the call-by-push-value core comes next; the features are added in the order in which each presupposes the last; and the algorithmic subtyping and constraint solver come last, because they are what an implementation actually runs.

This document is the specification of the rules.
What consumes them is elsewhere: [[../implementation#The checked language]] describes the crate that realizes part of them, [[feature-staging]] fixes the order in which the features are built and what counts as done for each, and [[proposed/recursion-former]] specifies the one former user-level recursion needs, which this system does not carry.

## What is built, and what is specified

The gap between the two is wide here, and reading a rule below as a description of the tree would be wrong for most of them.
Every claim in this section was checked against the named module at write time.

**Built.** `gandr-core-checker` realizes the core call-by-push-value fragment.
Its `types` module carries thirteen value-type formers — atoms, unit, eager products, sums, lists, records, graded thunks, reified stacks (the evaluation context internalized as a value), the identity type, declared data, the universe, dependent pairs, and the gradual `Unknown` — and four computation-type formers: the returner carrying an effect row, arrows, lazy products, and `Unknown`.
Its `syntax` module carries the terms for those formers, plus the effect and control block (`Perform`, `Handle`, `Resume`, `Reset`, `Shift`) and the identity fragment's `Here` and `Walk`.
It carries **no** type abstraction or instantiation, no session term, and no world term, so the built term language is narrower than the grammar below rather than a renaming of it.
Its `grade` module carries a representation-sealed semiring over the naturals with infinity.
Its `ctx` module carries the two-zone context: the intuitionistic zone and a linear zone whose shape is committed and whose contents are empty.
Its `subtype` module decides subtyping as a worklist of goals, which is the degenerate case of the solver specified below.
The checker is realized twice — a recursive form and a defunctionalized machine — and the two are property-tested for step-for-step agreement.

**Specified and not built.** Unions and intersections; explicit polymorphism and kinding; binary session types; manifest sharing; multiparty session types; worlds, mobility, and migration; and every part of the solver that needs a metavariable, a choice point, or a trail.
The linear zone is **vacuous** for a precise reason worth carrying: every obligation source it was designed to hold — session endpoints, acquired shared channels, held capabilities — belongs to one of those unbuilt features, so nothing populates it yet.
The zone's shape was committed early on the reasoning that retrofitting a context shape is expensive; its discipline binds the moment a first obligation source lands.
[[../surface-language/proposed/modes-and-references#The substrate, and what is actually built]] carries the consequences of that vacuity for every claim that leans on linearity.

**A divergence between the two, stated rather than reconciled.** Where the design record and the tree disagree, the tree wins on status and the design record wins on payload, and the disagreement is stated at the claim.
The one that recurs below is direction: several rules specified as direction-polymorphic or inference-capable are check-only as built, because inference for them needs metavariables the built checker does not have.

## Notation and judgment forms

| convention | meaning                                                                          |
| ---------- | -------------------------------------------------------------------------------- |
| `A, A′`    | value types (positive)                                                           |
| `B, B′`    | computation types (negative)                                                     |
| `v`        | values                                                                           |
| `t, u`     | computations                                                                     |
| `r, s`     | grades, elements of the grade semiring                                           |
| `w, w′`    | world identifiers                                                                |
| `S`        | local binary session types; `L` role-indexed local types; `G` global types       |
| `Γ`        | intuitionistic context, of world-annotated value hypotheses `x : A @ w`          |
| `Θ`        | shared-channel context, unrestricted: `a : S_S @ w`                              |
| `Σ`        | linear context: endpoints `c : S @ w` and capabilities `cap_w`                   |
| `Δ`        | kinding context: `X : κ`                                                         |
| `W`        | the solver's worklist of constraints                                             |
| `⇑`        | inference direction                                                              |
| `⇓`        | checking direction                                                               |
| `⇕`        | either direction — the rule is direction-polymorphic                             |

The four judgments, with every feature enabled:

```text
Γ; Θ; Σ ⊢_w v ⇑ A     infer a value type at world w
Γ; Θ; Σ ⊢_w v ⇓ A     check a value against A at world w
Γ; Θ; Σ ⊢_w t ⇑ B     infer a computation type at world w
Γ; Θ; Σ ⊢_w t ⇓ B     check a computation against B at world w
```

Zones no enabled feature uses are elided, so the core is just `Γ ⊢ e ⇕ T`.
That elision is why the rules below are written with the smallest context each one needs: a rule that never touches `Θ` does not mention it, and reinstating the full judgment is mechanical.

**The context discipline is one structural decision, and it carries session fidelity and capability accounting on its own.** `Γ` and `Θ` admit weakening and contraction.
`Σ` is **linear**: no weakening, no contraction; a multi-premise rule splits `Σ` explicitly, and a complete derivation consumes `Σ` exactly.
Grades are **not** what polices `Σ` — that separation is load-bearing, and the reason is that in the default semiring the zero grade is below every grade, so every graded thunk is droppable and grades impose no must-consume constraint at all.

**The bidirectional discipline is the intro-checks, elim-infers recipe** [@dunfield-krishnaswami-2013-bidirectional-higher-rank].
Introduction forms check; elimination forms infer their principal premise.
One subsumption rule mediates the two directions:

```text
Γ; Θ; Σ ⊢_w e ⇑ T     T <: T′
────────────────────────────── (Sub)
Γ; Θ; Σ ⊢_w e ⇓ T′
```

Here `e` and `T` range over values with value types or computations with computation types.
Annotated introduction forms additionally infer.

**Every rule below is mode-correct**: each metavariable appearing in a conclusion's outputs is determined by the rule's inputs.
That is a property to preserve when adding a rule, not an observation about the ones already written — a rule that fails it turns the checker into a search.

The subsumption rule is **inlined** in the built checker rather than applied as a separate step: each rule finishes by fitting what it constructed to the direction it was invoked in, which is where the subtyping obligation is discharged.

## Grades

Grades track **how many times a thunk may be forced**.
They form a preordered semiring `(R, +, ·, 0, 1, ⊑)`:

```text
r ::= 0                  exactly unused
    | 1                  exactly once        (unit of ·)
    | n                  bounded usage (n ∈ ℕ)
    | ω                  unrestricted

+ : alternative or accumulated demand      · : nested or sequential demand
0 + r ≡ r        r + s ≡ s + r             1 · r ≡ r        r · (s + s′) ≡ r·s + r·s′
⊑ is a preorder; + and · are monotone.
```

The default instantiation is `ℕ ∪ {ω}` with `r ⊑ s` when `r ≤ s` or `s = ω`, so a grade is an **upper bound**: a thunk graded `r` may be forced at most `r` times.

**Other semirings slot in unchanged — and that claim is about the rules, not about the implementation.** Exact usage, security levels, and intervals are the standard alternatives of the graded-modal setup [@petricek-orchard-mycroft-2014-coeffects; @orchard-liepelt-eades-2019-graded], and no rule below inspects the carrier.
In the tree the carrier is a **single concrete, representation-sealed type**, not a type parameter: `gandr-core-checker`'s `grade` module exposes zero, one, unbounded, a bounded constructor, and the order, sum, and product operations, and swapping the carrier is a one-module edit.
The seal is deliberate, because arbitrary-semiring generality buys expressiveness and costs inference, error messages, and adoption.

**Terminology, because the renaming has confused readers of the older material.** Grades here **are** the system's coeffects.
"Graded modal types" is the umbrella term the literature converged on after the original coeffect papers, with graded comonads playing the coeffect role and graded monads the effect role.
Nothing was dropped in the renaming.
What changed relative to the earliest statement of this design is the algebra — a proper semiring, with zero and one distinct — and the division of labour, in which linearity is enforced structurally by `Σ` rather than by grades.

Grades appear in exactly one type constructor, the graded thunk `U_r B`.
Two structural operations move budget around:

```text
Γ ⊢_w v ⇑ U_{r+s} B
─────────────────────────────────── (Dup)
Γ ⊢_w dup v ⇑ F (U_r B × U_s B)

Γ ⊢_w v ⇑ U_r B      0 ⊑ r
─────────────────────────── (Drop)
Γ ⊢_w drop v ⇑ F 1
```

**Both are built, and both diverge from the rules above in ways an implementer needs.** `dup` is **check-only** in `gandr-core-checker`'s `checker` module: it reads the split grades `r` and `s` off the expected returner-of-product type — the sole source of them, since inference cannot invent a split — and enforces `r + s ⊑ grade` rather than requiring the input grade to be exactly `r + s`, which is the subtyping-compatible weakening of the rule as stated.
`drop`'s side condition `0 ⊑ r` is **not checked, because it is vacuous on the default carrier**: zero is the bottom of that order, so every graded thunk is droppable.
That is not a gap; it is the same finding as the division of labour above, arriving from the implementation side.

The two operations have normative signatures, and they are the ones a surface must present:

```text
dup  : U_{r+s} B → F (U_r B × U_s B)
drop : U_r B → F 1                      (under 0 ⊑ r)
```

Subtyping on the thunk is **grade-contravariant**: `U_r B <: U_s B′` requires `s ⊑ r` and `B <: B′`.
A thunk that promises more forcings is usable where fewer are needed.

**Grading here is thunk-local — a graded comonadic structure on `U` — and that is a choice with a named alternative.** See [[#type-extension-01]] for per-assumption grading with context scaling, which is strictly more expressive and reshapes every rule, and [[#type-extension-02]] for the symmetric option of grading effects on the returner.
The `F`/`U` split is exactly the seam where the two gradings would live.
There is **no per-assumption grading and no context scaling in the tree**; a binder carries a grade only derivatively, as the grade of its bound value's thunk type.

Placing grades on `U` in a call-by-push-value setting rather than on a linear function type is validated two ways: by the bounded-linear-logic decomposition `!_r A ≅ U_r (F A)`, and by work grading call-by-push-value directly [@torczon-suarez-acevedo-agrawal-velez-ginorio-weirich-2024-effects-coeffects-cbpv].
The distinction matters when reading the graded-modal literature across, because the practical graded-modal languages are linear lambda calculi with graded modalities rather than call-by-push-value systems [@orchard-liepelt-eades-2019-graded].

## The core call-by-push-value calculus

### Types

```text
Value types        A, A′ ::= X              type variable
                          | 1               unit
                          | A × A′          eager product
                          | A + A′          tagged sum (coproduct)
                          | U_r B           graded thunk of a computation
                          | A ∪ A′          untagged union
                          | @w A            "A at world w"
                          | Stk(B, C)       reified stack

Computation types  B, B′ ::= F A            returner (produces an A)
                          | F^ε A           effect-graded returner (F A ≡ F^⟨⟩ A)
                          | A → B           function (value argument)
                          | B & B′          lazy product ("with")
                          | B ∩ B′          intersection
                          | ∀X. B           polymorphism
```

`U` and `F` are **the adjunction**: `thunk` takes a computation to a value, `force` takes a thunk back to a computation, and `ret` with `>>=` mediates `F A`.
Functions, type abstraction, and sequencing are computations; pairs, injections, and thunks are values.

**There is deliberately no value-level function type and no computation-level `@w`.** The first is what makes the polarity discipline of the next section work at all; the second is recovered as an `F (@w A)` result or through migration.

The built type formers extend this grammar on the value side with the primitive literals, lists, and records of the surface's value model, the identity type, declared data, the universe, and dependent pairs, and with the gradual `Unknown` on both sides; they omit unions, intersections, the world modality, and the universal quantifier.

### Terms

```text
Values        v ::= x | () | (v, v′) | inj1 v | inj2 v | thunk_r t | hold v | stk K | (v : A)

Computations  t, u ::= λx. t | λx:A. t | t v
                    | ret v | t >>= x. u
                    | force v
                    | case v of { inj1 x → t | inj2 y → u }
                    | split v as (x, y) in t | split v as (x, y) [z. M] in t
                    | ⟨t, u⟩ | prj1 t | prj2 t
                    | ΛX. t | t [A]
                    | dup v | drop v
                    | session terms
                    | leta x = v in t | migrate_w t
                    | effect and control terms (perform, handle, reset, shift, resume)
```

`(v : A)` is a type annotation, the standard check-to-infer coercion.

**The surface ascription is sort-directed, and the routing keeps the core small.** An ascription at a computation type, `(t : B)`, elaborates to `force ((thunk t) : U_ω B)` — the same annotation rule routed through the thunk — so no computation-annotation node exists in the core.

**World subscripts on `hold` do not exist.** `hold v` always packages at the current world; the reason is in the world rules below, and it is what replaces an earlier, unsound rule that typed the body at the source world.

### Core rules

```text
Γ(x) = A @ w
───────────── (Var)
Γ ⊢_w x ⇑ A

Γ ⊢ v ⇓ A
──────────────── (Annot)
Γ ⊢ (v : A) ⇑ A

──────────── (Unit)
Γ ⊢ () ⇑ 1

Γ ⊢ v ⇑ A     Γ ⊢ v′ ⇑ A′            Γ ⊢ v ⇓ A     Γ ⊢ v′ ⇓ A′
───────────────────────── (Pair⇑)    ───────────────────────── (Pair⇓)
Γ ⊢ (v, v′) ⇑ A × A′                 Γ ⊢ (v, v′) ⇓ A × A′

Γ ⊢ v ⇓ A₁                            Γ ⊢ v ⇓ A₂
────────────────────── (Inj1⇓)        ────────────────────── (Inj2⇓)
Γ ⊢ inj1 v ⇓ A₁ + A₂                  Γ ⊢ inj2 v ⇓ A₁ + A₂
```

**Injections only check.** In inference mode the other summand is not determined, so an injection in inference position is stuck and the diagnostic asks for an annotation.
The built checker treats a declared-data constructor the same way, and for the same reason.

```text
Γ, x:A@w; Σ ⊢_w t ⇓ B
─────────────────────────── (Abs⇓)
Γ; Σ ⊢_w λx. t ⇓ A → B

Γ, x:A@w; Σ ⊢_w t ⇑ B
─────────────────────────── (Abs⇑, annotated binder)
Γ; Σ ⊢_w λx:A. t ⇑ A → B

Γ; Σ ⊢_w t ⇑ A → B     Γ ⊢_w v ⇓ A
─────────────────────────────────── (App⇑)
Γ; Σ ⊢_w t v ⇑ B
```

Read the directions carefully, because they are the whole discipline: in checking mode an abstraction **checks** against a given arrow, and with an annotated binder it may infer; an application **infers** the function, which is its principal premise, and then **checks** the argument.

```text
Γ; Σ ⊢_w t ⇓ B     Σ linear-free or moved into the thunk
──────────────────────────── (Thunk⇓)
Γ; Σ ⊢_w thunk_r t ⇓ U_r B

Γ; · ⊢_w t ⇑ B
──────────────────────────── (Thunk⇑, annotated grade)
Γ ⊢_w thunk_r t ⇑ U_r B

Γ ⊢_w v ⇑ U_r B     1 ⊑ r
────────────────────────── (Force⇑)
Γ; · ⊢_w force v ⇑ B
```

**A thunk that captures linear resources must itself be treated linearly, and the discipline adopted here is the simple one**: `Σ = ·` inside a thunk, so a thunk captures no linear obligations at all.
The relaxation — thunks as linear values consuming their captured `Σ` — is [[#type-extension-03]], and it is proposed rather than adopted.

Both grade side conditions are built and are checked **per site with no accumulator**, which is worth knowing before relying on one: a grade-`1` thunk forced twice along a single path passes both checks independently, because context splitting and scaling belong to the extension above.

```text
Γ ⊢_w v ⇑ A                          Γ ⊢_w v ⇓ A
──────────────────── (Ret⇑)          ──────────────────── (Ret⇓)
Γ; · ⊢_w ret v ⇑ F A                 Γ; · ⊢_w ret v ⇓ F A

Γ; Σ₁ ⊢_w t ⇑ F A     Γ, x:A@w; Σ₂ ⊢_w u ⇕ B
───────────────────────────────────────────── (Bind⇕)
Γ; Σ₁, Σ₂ ⊢_w t >>= x. u ⇕ B
```

**Bind is direction-polymorphic**: in checking mode the target `B` flows into the continuation, while the bound computation is always inferred.
It is also where `Σ` splits, and the split is what distinguishes it from the lazy pair below.
In the built checker the bound computation's effect row is unioned into the continuation's returner at the frame that restores the context.

```text
Γ ⊢ v ⇑ A₁ + A₂    Γ, x:A₁; Σ ⊢ t ⇓ B    Γ, y:A₂; Σ ⊢ u ⇓ B
───────────────────────────────────────────────────────────── (Case⇓)
Γ; Σ ⊢ case v of { inj1 x → t | inj2 y → u } ⇓ B

Γ ⊢ v ⇑ Σ(x:A). B     Γ, p:A, q:B[p/x]; Σ ⊢ t ⇓ M[(p,q)/z]
─────────────────────────────────────────────────────────── (SplitMotive⇑)
Γ; Σ ⊢ split v as (p, q) [z. M] in t ⇑ M[v/z]

Γ ⊢ v ⇑ Σ(x:A). B     Γ, p:A, q:B[p/x]; Σ ⊢ t ⇓ C
─────────────────────────────────────────────────────────── (Split⇓)
Γ; Σ ⊢ split v as (p, q) in t ⇓ C

Γ; Σ ⊢ t ⇓ B₁     Γ; Σ ⊢ u ⇓ B₂              Γ; Σ ⊢ t ⇑ B₁ & B₂
──────────────────────────────── (With⇓)     ────────────────── (Prjᵢ⇑)
Γ; Σ ⊢ ⟨t, u⟩ ⇓ B₁ & B₂                      Γ; Σ ⊢ prjᵢ t ⇑ Bᵢ
```

**The split eliminator takes an optional dependent motive `[z. M]` binding the scrutinee value, and the motive is what decides the direction.** The motive-bearing form **infers**: the body checks against `M[(p,q)/z]` under `p : A` and `q : B[p/x]`, and the rule delivers the outer-scoped `M[v/z]`, in which no split binder can occur.
The motive-less form is **check-only**: the outer expectation `C` is necessarily binder-free and is delivered verbatim, and a motive-less split in inference position is stuck with a needs-motive hint.

**There is no scope check, and its absence is the point.** No rule ever delivers a type synthesized under the split binders — the dependent answer is substituted from outside, and the check-only answer is the expectation itself.
This is what closes the binder-escape hazard for both the eager product and the dependent pair, and it does so **at the rule** rather than by a check bolted onto a rule that could escape.
The motive is also the eliminator shape that inductive families need, so the same decision unlocks them.
The eager product is the constant-tail degenerate case, `B[p/x] = B`, so one pair of rules reads both.

In the tree this is exactly what is built: the motive-carrying split is inference-capable precisely when the motive is present, and the motive-less form fires its stuck error at rule entry, before the scrutinee is touched.

The lazy pair `⟨t, u⟩` **shares** `Σ` between its components, because only one of them will run.
That is the additive conjunction of linear logic, and the contrast with bind — which splits — is the clearest statement of what the two connectives mean.

### Records

Records are the **labeled, n-ary generalization of the eager product**, and they are direction-polymorphic like the pair.

```text
Γ ⊢ vᵢ ⇑ Aᵢ  (each i)                 Γ ⊢ vᵢ ⇓ Aᵢ  (each i)
───────────────────────── (Rec⇑)      ───────────────────────── (Rec⇓)
Γ ⊢ {ℓᵢ=vᵢ} ⇑ {ℓᵢ:Aᵢ}                 Γ ⊢ {ℓᵢ=vᵢ} ⇓ {ℓᵢ:Aᵢ}

Γ ⊢ r ⇑ {…, ℓ:A, …}
────────────────────── (RecPrj⇑)
Γ; · ⊢ r.ℓ ⇑ F A
```

A record literal infers when each field infers, and checks field-by-field against an expected record; width and depth subtyping is then the inlined subsumption rule, so a **wider** literal checks against a **narrower** record type.
The eliminator is field projection, which eliminates the positive record as a returner — eliminating a positive type is a computation, and projection is the split discipline narrowed to one named field.

**The former is closed, and the row-typed generalization is a deliberate later refinement rather than a retrofit.** A record row variable, `{ℓ:A | ρ}`, belongs to the polymorphism stage; closed records are a special case of row-typed ones, so nothing has to be undone to get there, and the upgrade aligns with the algebraic-subtyping experiment of [[#type-extension-11]].
This is [[#type-extension-13]].

**Encoding a record as an intersection of single-field records is recorded as refuted, with its reason.** Intersection is negative-only in this system and expresses **overloading** rather than field combination, so the encoding is not merely inconvenient here — it is at the wrong polarity, and it would need a constraint domain the closed former does not.

## Unions and intersections

**Unions live on value (positive) types; intersections on computation (negative) types.** That is the polarity at which each connective is proof-theoretically well-behaved [@zeilberger-2008-unity-of-duality], and call-by-push-value is the natural setting in which to enforce it, because the polarity is already carried by the sorts rather than imposed on top of them.

The payoff is immediate and is the reason the sorting is not merely tidy.
Unrestricted union elimination is classically unsound in call-by-value, where it requires an evaluation-context restriction to recover [@dunfield-pfenning-2004-tridirectional].
Here the only union elimination site is `>>=`, where sequencing is **already explicit**, so the restriction is discharged by construction rather than imposed as a side condition.

### Subtyping, declaratively

```text
A₁ <: A₁ ∪ A₂        A₂ <: A₁ ∪ A₂        (∪ introduction on the right)
B₁ ∩ B₂ <: B₁        B₁ ∩ B₂ <: B₂        (∩ elimination on the left)
```

**These are the only primitive set-operation axioms, and the restriction is load-bearing.** The converse directions are derivable **rules with premises** — from `A₁ <: A` and `A₂ <: A` conclude `A₁ ∪ A₂ <: A`, and dually for intersection — and are never axioms.
Stating both directions as axioms collapses the subtyping relation to the total relation, which is exactly the defect an earlier statement of this system carried.
The corresponding regression test is that the subtype order is **not** total, and it is an assertion rather than an observation.

### Union rules

Introduction is subsumption, because unions are untagged and there are no injections to write:

```text
Γ ⊢ v ⇓ A₁
───────────────── (∪I via Sub)
Γ ⊢ v ⇓ A₁ ∪ A₂
```

Elimination happens at bind, in checking mode, by checking the continuation under **both** disjuncts:

```text
Γ; Σ₁ ⊢ t ⇑ F (A₁ ∪ A₂)     Γ, x:A₁; Σ₂ ⊢ u ⇓ B     Γ, x:A₂; Σ₂ ⊢ u ⇓ B
───────────────────────────────────────────────────────────────────────── (∪E⇓)
Γ; Σ₁, Σ₂ ⊢ t >>= x. u ⇓ B
```

### Intersection rules

Introduction types the **same** computation at both components, and that is precisely what distinguishes an intersection from a product:

```text
Γ; Σ ⊢ t ⇓ B₁     Γ; Σ ⊢ t ⇓ B₂
──────────────────────────────── (∩I⇓)
Γ; Σ ⊢ t ⇓ B₁ ∩ B₂
```

Elimination is subsumption, from the axioms above.
The canonical use is overloading a thunk:

```text
thunk_ω (λx. ret x)  ⇓  U_ω ((Int → F Int) ∩ (String → F String))
```

**Term-level merges are intentionally absent**, and their absence is a decision rather than an omission — see [[#type-extension-04]].

## Explicit polymorphism and kinding

This stage is **Church-style System F over computation types**: `∀X. B` with explicit abstraction `ΛX. t` and explicit instantiation `t [A]`.
Rank-n types are unremarkable when instantiation is explicit, which is what makes the explicit form the right first stage.

```text
Γ; Δ, X:*; Σ ⊢ t ⇓ B                  Γ; Δ, X:*; Σ ⊢ t ⇑ B
───────────────────── (Gen⇓)          ───────────────────── (Gen⇑)
Γ; Δ; Σ ⊢ ΛX. t ⇓ ∀X. B               Γ; Δ; Σ ⊢ ΛX. t ⇑ ∀X. B

Γ; Δ; Σ ⊢ t ⇑ ∀X. B     Δ ⊢ A : *
────────────────────────────────── (Inst⇑)
Γ; Δ; Σ ⊢ t [A] ⇑ B[A/X]
```

Note the premise context — `X` is bound in `Δ` while the body is typed — and note that the principal premise of instantiation is **inferred**, as the elimination discipline requires.

### Kinds

```text
κ ::= *           kind of (value) types
    | κ → κ′      kind of type constructors      (extension stage)

Δ ∋ X:κ                Δ ⊢ T₁ : κ → κ′    Δ ⊢ T₂ : κ
──────────── (K-Var)   ────────────────────────────── (K-App)
Δ ⊢ X : κ              Δ ⊢ T₁ T₂ : κ′
```

Type formation is the evident induction, `Δ ⊢ A : *` for each value-type former and dually for computation types.
**There is no `* : *`; kinds are not themselves classified.**

**The kind-application rule is the one part of this section another corpus document already depends on.** The module system needs higher kinds `κ → κ′` for the skolem functions its lifting produces, and [[../surface-language/proposed/modules#Transparent ascription and sealing]] names that dependency; the rule it needs is `K-App` above.

Implicit higher-rank instantiation is [[#type-extension-05]] and is not part of this stage.

## Binary session types

Session endpoints live in the **linear** context `Σ`, located at worlds; they are not value types, and nothing in the value grammar mentions them.
Payloads may be values, written `!A.S`, or endpoints, which is delegation, written `!S′.S`.

### Syntax and duality

```text
S ::= end                       terminated
    | !A.S                      send a value of type A, continue as S
    | ?A.S                      receive a value of type A, continue as S
    | !S′.S                     delegate (send an endpoint), continue as S
    | ?S′.S                     receive an endpoint, continue as S
    | ⊕{lᵢ : Sᵢ}ᵢ∈I             internal choice (we select)
    | &{lᵢ : Sᵢ}ᵢ∈I             external choice (we offer)
    | μX.S | X                  recursion (contractive)

dual(end)      = end
dual(!A.S)     = ?A.dual(S)            dual(?A.S)     = !A.dual(S)
dual(!S′.S)    = ?S′.dual(S)           dual(?S′.S)    = !S′.dual(S)
dual(⊕{lᵢ:Sᵢ}) = &{lᵢ:dual(Sᵢ)}        dual(&{lᵢ:Sᵢ}) = ⊕{lᵢ:dual(Sᵢ)}
dual(μX.S)     = μX.dual(S)            dual(X)        = X
```

**Duality flips polarities pointwise and does not reverse the order of actions.** So `dual(!Int.?String.end)` is `?Int.!String.end`, not `!String.?Int.end`.
This is worth stating explicitly because the reversing reading is a natural and wrong guess, and a session type is the kind of artifact where a wrong guess type-checks against itself.

### Session action rules

Every session action carries its continuation, which is what makes result types determined by a premise and so keeps the rules mode-correct:

```text
Γ ⊢_w v ⇓ A     Γ; Σ, c:S ⊢_w t ⇕ B
──────────────────────────────────── (Send⇕)
Γ; Σ, c:!A.S ⊢_w send c v; t ⇕ B

Γ, x:A@w; Σ, c:S ⊢_w t ⇕ B
──────────────────────────────────── (Recv⇕)
Γ; Σ, c:?A.S ⊢_w recv c as x; t ⇕ B

Γ; Σ ⊢_w t ⇕ B
──────────────────────────────────── (Close⇕)
Γ; Σ, c:end ⊢_w close c; t ⇕ B

j ∈ I     Γ; Σ, c:Sⱼ ⊢_w t ⇕ B
──────────────────────────────────────── (Select⇕)
Γ; Σ, c:⊕{lᵢ:Sᵢ} ⊢_w select c lⱼ; t ⇕ B

Γ; Σ, c:Sᵢ ⊢_w tᵢ ⇓ B     for all i ∈ I
──────────────────────────────────────── (Offer⇓)
Γ; Σ, c:&{lᵢ:Sᵢ} ⊢_w offer c {lᵢ ⇒ tᵢ} ⇓ B

Γ; Σ, d:S′ ⊢ t ⇕ B
──────────────────────────────────── (Deleg)
Γ; Σ, c:!S′.S, d:S′ ⊢ send c d; t ⇕ B
```

Delegation moves the endpoint `d` to the peer, and the receiving side is the symmetric rule on `?S′.S`.
Note what delegation moves: **endpoints, and only endpoints**.
No rule sends a capability or an arbitrary linear resident over a session, which bounds several attractive-sounding claims about what sessions can express.

**Channel creation is a cut**: the child owns one endpoint and the parent the dual.

```text
Γ; Σ₁, c:S ⊢_w t ⇓ F 1     Γ; Σ₂, c′:dual(S) ⊢_w u ⇕ B
──────────────────────────────────────────────────────── (Fork)
Γ; Σ₁, Σ₂ ⊢_w fork (c:S). t in c′. u ⇕ B
```

**Because `Σ` is linear and `fork` is the only rule introducing dual pairs, session fidelity is structural** rather than checked by a separate pass.

**One inherited guarantee is deliberately not inherited, and the reason bounds every progress claim made about this system.** In the linear-logic reading of sessions, fusing channel creation with parallel composition forces a well-typed process topology to be a **tree**, from which deadlock freedom and global progress follow by construction — at the price that genuinely cyclic networks are not typable at all.
This `fork` permits interleaving, so it does **not** inherit that guarantee.
What it has is fidelity and communication safety structurally, and deadlock freedom only within a single multiparty session, by projection coherence.

Session **subtyping** is the coinductive relation of Gay and Hole [@gay-hole-2005-session-subtyping]: covariant in the continuations of receive and external choice, contravariant in the payload of send, with width subtyping on choices.
It is decided algorithmically as a regular-tree check, below.

## Shared sessions: manifest sharing

Linear sessions cannot express a service with many concurrent clients — a linear endpoint has exactly one holder, which is the whole point of it.
**Manifest sharing** adds a shared layer with an acquire-and-release discipline [@balzer-pfenning-2017-manifest-sharing].
The sharing is called _manifest_ because the points at which a shared resource becomes linear, and returns, are **visible in the type**.

### Stratified session types

```text
Linear   S_L ::= (all of the binary grammar) | ↓ˢₗ S_S      release point: revert to shared
Shared   S_S ::= ↑ˢₗ S_L                                    acquire point: become linear
```

A shared channel `a : ↑ˢₗ S_L` sits in the unrestricted zone `Θ`, where it may be aliased freely.
Acquiring it yields a **linear** endpoint `c : S_L` in `Σ`, held by one client at a time — **acquire is mutual exclusion**, and that is the one dynamic-exclusivity mechanism this design already has.
A linear endpoint of type `↓ˢₗ S_S` is released back into `Θ`.

### The equi-synchronizing constraint

Not every stratified type is sensible.
When one client releases, **other clients resume interacting at the shared type they expect**, so a release that lands somewhere else silently breaks every other client's protocol.

A linear type `S_L` is **equi-synchronizing** with respect to `S_S` when every release point reachable in `S_L`, unfolding recursion, releases at exactly `S_S`:

```text
esync(S_L, S_S)  ⟺  every ↓ˢₗ S′_S reachable in S_L has S′_S = S_S
```

Acquire requires it.
The condition is generated as a constraint and discharged by the solver as a **regular-tree check**, decidable for contractive recursive types.

### Sharing rules

```text
Θ(a) = ↑ˢₗ S_L @ w     esync(S_L, ↑ˢₗ S_L)     Γ; Θ; Σ, c:S_L ⊢_w t ⇕ B
───────────────────────────────────────────────────────────────────────── (Acquire⇕)
Γ; Θ; Σ ⊢_w acquire a as c; t ⇕ B

Γ; Θ, a:S_S @ w; Σ ⊢_w t ⇕ B
─────────────────────────────────────── (Release⇕)
Γ; Θ; Σ, c:↓ˢₗ S_S ⊢_w release c as a; t ⇕ B

Γ; Θ, a:↑ˢₗ S_L @ w; Σ₁ ⊢_w t ⇓ F 1     Γ; Θ, a:↑ˢₗ S_L @ w; Σ₂ ⊢_w u ⇕ B
─────────────────────────────────────────────────────────────────────────── (ShFork)
Γ; Θ; Σ₁, Σ₂ ⊢_w fork! (a : ↑ˢₗ S_L). t in u ⇕ B
```

Shared fork spawns a **service**: the provider repeatedly waits to be acquired — its body typed against `S_L` after each acquire, operationally a recursive accept loop — and the shared name is unrestricted in the client.

**The cost of this expressiveness is stated by the design it follows, and it must not be lost in transfer.** Acquire and release reintroduce the possibility of **deadlock between competing acquisitions**; manifest sharing trades the deadlock freedom of pure linear sessions for the ability to write a service at all.
The refinement that restores deadlock freedom is [[#type-extension-06]], and this design reserves its hooks now so that absorbing it later is local to this section.

## Multiparty session types

Binary duality does not scale to a protocol with three or more roles: there is no single dual to check against, and pairwise duality does not imply global consistency.
The machinery that does scale is **global types with projection** [@honda-yoshida-carbone-2008-multiparty].

**This section replaces a rendezvous primitive wholesale.** An earlier statement of this design carried a three-party synchronization primitive; it is superseded by global types, and the primitive is not carried forward.

### Global and local types

```text
Global  G ::= p → q : ⟨A⟩. G          p sends q a value of type A
            | p → q : {lᵢ : Gᵢ}ᵢ∈I    p selects a label, q offers
            | μX. G | X | end

Local   L ::= !⟨q, A⟩. L              send A to role q
            | ?⟨p, A⟩. L              receive A from role p
            | ⊕⟨q, {lᵢ : Lᵢ}⟩         select toward q
            | &⟨p, {lᵢ : Lᵢ}⟩         offer from p
            | μX. L | X | end
```

**A binary session is exactly the two-role special case**: the role indices become redundant and the binary system is recovered verbatim, which is why the two sections share their frames in any implementation.

### Projection

Projection `G ↾ r` extracts role `r`'s obligations:

```text
(p → q : ⟨A⟩. G) ↾ r   = !⟨q, A⟩. (G ↾ r)                if r = p
                       = ?⟨p, A⟩. (G ↾ r)                if r = q
                       = G ↾ r                            otherwise

(p → q : {lᵢ : Gᵢ}) ↾ r = ⊕⟨q, {lᵢ : Gᵢ ↾ r}⟩            if r = p
                        = &⟨p, {lᵢ : Gᵢ ↾ r}⟩            if r = q
                        = ⊓ᵢ (Gᵢ ↾ r)                     otherwise

(μX. G) ↾ r = μX. (G ↾ r)   (= end if r does not occur in G)
end ↾ r     = end
```

`⊓` is the **merge** operator, applied to roles not involved in a branch, and the choice of merge is a real expressiveness decision rather than a detail.
The baseline is **plain merge**: every branch must project identically for that role.
The implementation supports **full merge**, under which external choices with distinct labels may be unioned, and which accepts strictly more protocols.

Well-formedness `wf(G)` is contractivity of the recursion plus projectability onto every role.

### Multiparty rules

Endpoints in `Σ` carry local types, and actions are role-indexed:

```text
Γ ⊢ v ⇓ A     Γ; Σ, c:L ⊢ t ⇕ B
─────────────────────────────────────── (MSend⇕)
Γ; Σ, c:!⟨q,A⟩.L ⊢ send c[q] v; t ⇕ B

Γ, x:A; Σ, c:L ⊢ t ⇕ B
─────────────────────────────────────── (MRecv⇕)
Γ; Σ, c:?⟨p,A⟩.L ⊢ recv c[p] as x; t ⇕ B
```

Select and offer are the evident role-indexed analogues of the binary rules.
**Session initiation replaces the binary cut**: one process per role, each typed against its own projection.

```text
wf(G)     roles(G) = {p₁ … pₙ, q}
Γ; Σᵢ, c:G↾pᵢ ⊢ tᵢ ⇓ F 1     for each i
Γ; Σ₀, c:G↾q ⊢ u ⇕ B
──────────────────────────────────────────── (MCut)
Γ; Σ₀, Σ₁, …, Σₙ ⊢ session G { pᵢ ⇒ tᵢ } in q. u ⇕ B
```

**Coherence is by construction**: every endpoint originates from a projection of one well-formed global type, which is what delivers communication safety and session fidelity, and deadlock freedom **within** a single session.
Deadlock across sessions is governed by the sharing layer's discipline instead, and is not delivered here.

The projection-and-coherence approach is taken as primary; the projection-free alternative is [[#type-extension-08]].

## Worlds, mobility, and migration

This section follows the located-modality tradition [@murphy-crary-harper-2007-ml5], on the judgmental modal base [@pfenning-davies-2001-judgmental-modal].
Typing judgments are **located**, written `⊢_w`; hypotheses record where they make sense, `x : A @ w`; and `@w A` internalizes "A is true at `w`" as a value type usable anywhere.

### Locatedness

The variable rule already enforces locality: `x : A @ w` is usable only at `w`.
Endpoints and capabilities in `Σ` and shared channels in `Θ` are located the same way, and the session rules implicitly require that a channel's location is the current world.

### Mobility

Some types denote data meaningful at every world, and only those may cross:

```text
mobile(1)    mobile(base)
mobile(A × A′), mobile(A + A′), mobile(A ∪ A′)    if both components mobile
mobile(@w A)                                       always
¬mobile(U_r B)                                     in general (code may capture local behaviour)
endpoints and capabilities are never mobile
```

A reified stack is likewise immobile — a continuation is the most world-bound object there is.

**The default polarity is the safe one, and that is a design property worth naming.** Mobility must be **established** to transport a value; code-bearing and resource-bearing types are conservatively immobile, so a continuation cannot be shipped by accident.

**And the bound on that claim must ride with it every time it is made: mobility is world-level, never address-level.** It says a value may not be transported to another **place**; it says nothing about whether its bytes may move within one address space.
The two read alike and are a category error apart.
[[../surface-language/proposed/modes-and-references#mode-decision-09]] carries the consequences for the foreign interface.

Genuinely mobile **code** is [[#type-extension-09]]; here, remote code is handled by reference, since `@w (U_r B)` is a mobile handle to code that runs at `w`.

### World rules

`hold` packages a value at the world it inhabits; `leta` opens such a package, binding a hypothesis located at the package's world; `migrate` runs a computation at another world and brings back a mobile result.

```text
Γ ⊢_w v ⇓ A                            Γ ⊢_w v ⇑ A
──────────────────── (Hold⇓)           ──────────────────── (Hold⇑)
Γ ⊢_w hold v ⇓ @w A                    Γ ⊢_w hold v ⇑ @w A

Γ ⊢_w v ⇑ @w′ A     Γ, x:A@w′; Θ; Σ ⊢_w t ⇕ B
─────────────────────────────────────────────── (Leta⇕)
Γ; Θ; Σ ⊢_w leta x = v in t ⇕ B

Γ; Θ; Σ ⊢_{w′} t ⇓ F A     mobile(A)     loc(Σ) ⊆ {w′}
──────────────────────────────────────────────────────── (Migrate⇑)
Γ; Θ; Σ, cap_{w′} ⊢_w migrate_{w′} t ⇑ F A
```

Five things about these rules are load-bearing, and each is a place a plausible variant would be wrong.

**There is no way to fabricate `@w′ A` from afar.** A package at `w′` is obtained by _being_ at `w′` — perhaps by migrating there — and holding.
This is what replaces an earlier boxing rule that unsoundly typed the body at the source world.

**Opening a package is free; what is gated is moving control.** So `leta` needs no capability, while `migrate` consumes a **linear capability `cap_{w′}` drawn from `Σ`**.
Capabilities are **context assumptions, never term constants** — a linear constant is a contradiction, since a constant may be written twice.
Reusable capabilities, if wanted, are graded assumptions, and the grade semiring applies unchanged.

**Migration's body may use linear resources only if they are located at the destination**, which is what `loc(Σ) ⊆ {w′}` says, and its result must be mobile.

**The capability is this design's own addition to the tradition it follows.** ML5's pure modal setting has no capabilities at all [@murphy-crary-harper-2007-ml5]: dropping `cap` recovers exactly its retrieval operation.
The addition is deliberate, for resource-controlled migration.

**Cross-world channels are permitted and constrain their payloads.** A fork whose child immediately migrates yields endpoints at different worlds, and the payload types of such a channel must be mobile — generated as mobility constraints at the fork.

Operationally, migration is a session exchange in disguise, and taking that seriously would shrink the core: see [[#type-extension-10]].

## Algorithmic subtyping and the worklist solver

This is what an implementation runs.
The declarative relations above are specifications; the solver below is the decision procedure, and the two are related by the standing property that **reflexivity and transitivity are admissible properties of the system, never solver rules**.

**Each constraint form below is decided by its own procedure, and the design that names those procedures as registrable _domains_ — and adds SMT-backed refinements as the first external one — is [[proposed/solver-interface]].** That design is not built; what the solver states here is the language it would plug into.

### The constraint language

```text
C ::= A <: A′ | B <: B′         subtyping
    | α := T                    instantiation of a unification variable
    | r ⊑ s                     grade order
    | mobile(A)                 mobility check
    | esync(S_L, S_S)           equi-synchronization
    | wf(G) | proj(G, p, L)     global-type well-formedness and projection
```

Solver state is a triple `(W, σ, trail)`: the worklist, the substitution, and a **stack** of choice points, each recording the full worklist and substitution at the choice plus its untried alternatives.

**The trail is a stack rather than a single backpoint, and that is not a refinement — it is a correctness condition.** A single saved state cannot implement the search that union and intersection subtyping require, because a failure may have to unwind past more than one choice.

### Subtyping decomposition

Goals are decomposed **structurally**:

```text
α <: T  /  T <: α     →  α := T  (after occurs check; apply σ to W)
X <: X, 1 <: 1        →  ✓
A₁×A₂ <: A₁′×A₂′      →  A₁ <: A₁′,  A₂ <: A₂′         (likewise +, covariant)
List A <: List A′     →  A <: A′                       (covariant)
{ℓᵢ:Aᵢ} <: {mⱼ:Bⱼ}    →  ∀j ∃i. ℓᵢ=mⱼ ∧ Aᵢ <: Bⱼ        (record width and depth)
U_r B <: U_s B′       →  s ⊑ r,  B <: B′               (grade-contravariant)
Stk(B,C) <: Stk(B′,C′) →  B′ <: B,  C <: C′            (contravariant in B, covariant in C)
@w A <: @w A′         →  A <: A′                       (worlds must match exactly)
F A <: F A′           →  A <: A′
A → B <: A′ → B′      →  A′ <: A,  B <: B′
B₁&B₂ <: B₁′&B₂′      →  componentwise
∀X.B <: ∀X.B′         →  B <: B′  (α-converted)

Invertible (run eagerly, no choice):
A₁ ∪ A₂ <: A′         →  A₁ <: A′,  A₂ <: A′
B <: B₁′ ∩ B₂′        →  B <: B₁′,  B <: B₂′

Choice points (push a trail entry, try left then right):
A <: A₁′ ∪ A₂′        →  A <: A₁′  ⫾  A <: A₂′          (A not a union or a variable)
B₁ ∩ B₂ <: B′         →  B₁ <: B′  ⫾  B₂ <: B′          (B′ not an intersection)
```

In the choice-point rules `⫾` separates the alternatives, which are tried in the order written.

**Processing invertible rules before choice points is a focusing discipline, and it eliminates spurious backtracking** rather than merely reordering work.

Session subtyping, in both the binary and role-indexed forms, is the coinductive Gay–Hole relation, decided with a **visited set over the regular trees** of contractive recursive types; equi-synchronization, well-formedness, and projection are likewise regular-tree checks.

### Transitions

```text
(W ∪ {α := T}, σ)   →  occurs(α, T) ? FAIL : (σ(W), σ[α ↦ T])
(W ∪ {C_dec},  σ)   →  (W ∪ decompose(C_dec), σ)
(W ∪ {C_choice}, σ) →  push trail; (W ∪ first-alternative, σ)
(W ∪ {r ⊑ s}, σ)    →  decide in the grade semiring; FAIL if ¬(r ⊑ s)
FAIL                →  pop the trail to the most recent choice point with untried
                       alternatives; resume there; FAIL overall if the trail is empty
```

**Backtracking interacts with incremental checkpoints, and the interaction is a hazard rather than a convenience**: any checkpoint created after a choice point is **invalidated** when that choice point is popped, because the checkpoint was taken inside a speculative region that no longer exists.
The incrementality lane's standing gate — incremental equals from-scratch — is what would catch a violation.

### What the tree actually decides

`gandr-core-checker`'s `subtype` module is this solver in its **degenerate case**, and reading it as a different algorithm would be a mistake: subtyping obligations sit in a worklist of goals, are popped, are decided against the decompositions above, and are replaced by their child goals.
With no metavariables in play a last-in-first-out queue is observationally the in-order structural recursion, which is why the built code reads as a recursive descent.

Built decompositions, verified against that module: atom equality; unit and universe; products and sums componentwise covariant; lists covariant; records by width and depth; graded thunks grade-contravariant with covariant body; reified stacks contravariant in the first component and covariant in the second; the identity type invariant in its type with structural endpoint equality; declared data by nominal identity with covariant arguments; dependent pairs invariant in both components with binder alignment; returners covariant with **effect-row inclusion**; arrows contravariant in the argument and covariant in the result; lazy products componentwise.

Not built: variable instantiation and the occurs check, the union and intersection rules, the world modality, the quantifier, emitted grade constraints — grade order is decided on the spot rather than emitted — session subtyping, and the mobility, equi-synchronization, well-formedness, and projection checks.
There are therefore **no choice points and no trail**, which is exactly why the degenerate reading holds.

**One property of the built relation is deliberate and is not a defect to fix.** Once the gradual `Unknown` participates, subtyping is _consistent subtyping_ in the gradual-typing sense: it is reflexive by rule and **not transitive by rule**, since `Int` relates to `Unknown` and `Unknown` to `String` while `Int` and `String` are unrelated.
Transitivity holds on `Unknown`-free types, where it is admissible as the specification requires.

## The full judgment

```text
Γ; Δ; Θ; Σ ⊢_w v ⇕ A          Γ; Δ; Θ; Σ ⊢_w t ⇕ B
```

- `Γ : x ↦ A @ w` — intuitionistic, world-annotated value hypotheses
- `Δ : X ↦ κ` — the kinding context
- `Θ : a ↦ S_S @ w` — shared channels, unrestricted
- `Σ` — linear: endpoints `c : S @ w` or `c : L @ w`, and capabilities `cap_w`
- `w` — the current world; `⇑` and `⇓` — the direction

Every rule above composes under this judgment, and the solver discharges the constraints they emit.

**The system carries four modal-flavoured structures, and they can be presented as one** — see [[#type-extension-12]].

## Feature staging

The features compose additively, and the table below is the per-feature statement of what each one adds to the rules and to the contexts and solver.
It is the rule-level companion to [[feature-staging]], which fixes the build order, the deliverables, and the acceptance criteria; where the two are read together, this table says what a feature _is_ and that document says when it is _done_.

| feature       | rules added                                                                                              | context and solver extensions                            | status                         |
| ------------- | -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------ |
| `core`        | Var, Sub, Annot, Unit, Pair, Inj, Case, Split, SplitMotive, Abs, App, Thunk, Force, Ret, Bind, With, Prj | `Γ`                                                      | built                          |
| `+grades`     | Dup, Drop; the graded thunk                                                                              | the grade semiring; grade-order constraints              | built                          |
| `+setops`     | union introduction and elimination, intersection introduction; set-operation subtyping                   | choice points and the trail                              | not built                      |
| `+poly`       | Gen, Inst; K-Var, K-App                                                                                  | `Δ`                                                      | not built                      |
| `+sessions`   | Send, Recv, Close, Select, Offer, Deleg, Fork                                                            | linear `Σ`; Gay–Hole subtyping                           | not built; `Σ` shape exists    |
| `+sharing`    | Acquire, Release, ShFork                                                                                 | `Θ`; equi-synchronization constraints                    | not built                      |
| `+multiparty` | MSend, MRecv, MSelect, MOffer, MCut                                                                      | global types; well-formedness and projection constraints | not built                      |
| `+worlds`     | Hold, Leta, Migrate; located Var                                                                         | world annotations; mobility constraints                  | not built                      |
| `+effects`    | Perform, Handle; the effect-graded returner                                                              | effect rows; row unification                             | built as sealed rows           |
| `+control`    | Reify, Resume, Reset, Shift; the reified stack                                                           | one-shot linear stacks, resident in `Σ`                  | built; stacks not `Σ`-resident |
| `+dynamics`   | none — this is operational                                                                               | the runtime and its process soup                         | sequential core built          |
| `+modules`    | the module judgment                                                                                      | signature contexts, the implicit table                   | in progress                    |
| `+evidence`   | none — an erasible sublanguage erasing to the non-dependent core                                         | an evidence phase                                        | decided direction              |
| `full`        | all of the above                                                                                         | the full judgment                                        | —                              |

The module layer's own rules are [[../surface-language/proposed/modules]]; the effect and control block's own rules — the operation and handle rules, the stack judgment, the control operators, and the linearity discipline behind those two status cells — are [[effects-and-control]], its surface is [[../surface-language/shell]], and its runtime seam is [[../implementation#The runtime host]].

**Two decided directions extend this system without changing any rule above, and both are recorded with their construction obligations in [[feature-staging#The decided directions as construction milestones]].** The first is an in-language, phase-separated **erasible-evidence** layer reaching toward full dependent types, which **erases** to this non-dependent core.
The second is a unifying semantic model — the wheeled, polarity-sorted, graded nominal structure — which consolidates the adjunction, the grades, and the sessions this document already carries.
Both are decided; their construction is the open research obligation; and the type system specified here is their non-dependent runnable core, unchanged.

## Designated extensions and recorded alternatives

Each item below is an open disposition carried from the design record.
The numbering is **stable**: retiring one leaves its number unused.
None of them is adopted, and none is refuted; each says what it would change and what it would cost.

### type-extension-01

**Per-assumption grading with context scaling.** Grade every context assumption rather than only the thunk, writing `x :_r A` and scaling contexts by `r · Γ`.
Strictly more expressive, and it reshapes every rule.

**In call-by-push-value, counting uses of ground data buys nothing** — usage of behaviour is already counted at the thunk and resources are policed linearly by `Σ` — so the upgrade earns its keep exactly when the grade means something other than a count.
Three motivations are known in _this_ system and are recorded because each is a real capability rather than a generalization for its own sake:

1. **Information flow for migration.** A lattice-graded assumption, read as "observable by worlds below `w`", composes with the located judgment to give migration-aware information-flow control: migrating to `w` would require every assumption the computation touches to be flowable to `w`.
   The mobility judgment is the boolean shadow of exactly this; graded assumptions are its refinement.
2. **Serialization trimming.** A zero-graded capture provably need not ship when code migrates, which is erasure as an optimization licence, and it bears directly on the payload size of migration and holding.
3. **Sensitivity.** Real-valued grades give sensitivity and differential-privacy tracking [@reed-pierce-2010-fuzz], should numeric analysis enter scope.

**The upgrade path is mechanical rather than architectural**: context entries already carry an annotation column, namely the world, and the semiring is already a parameter of the rules.

### type-extension-02

**Grading effects on the returner.** Symmetrically to the thunk grading adopted here, effects can be graded on `F` by graded monads.
The `F`/`U` split is exactly the seam at which the two gradings live, and the tree already carries a **sealed effect row** on the returner, which is the shape such a grading would refine.

### type-extension-03

**Thunks that capture linear resources.** The adopted discipline is that a thunk captures no linear obligations.
The standard relaxation makes a capturing thunk a linear value that consumes its captured `Σ`.
A first-class borrow would need this generalized further — from capturing a linear _name_ to capturing a value _origin_ — which is a strictly larger change; [[../surface-language/proposed/modes-and-references#mode-decision-05]] owns that question.

### type-extension-04

**Term-level merges.** A merge `(v | v′)` selects between two different implementations by type.
It is intentionally absent, because coherence requires a **disjointness** judgment [@oliveira-shi-alpuim-2016-disjoint-intersection].
If overloading by merge is wanted later, it arrives **together with** that judgment, never before it.

### type-extension-05

**Implicit higher-rank instantiation.** The designated extension of the explicit polymorphism stage [@dunfield-krishnaswami-2013-bidirectional-higher-rank], mechanized as a worklist algorithm [@zhao-oliveira-schrijvers-2019-worklist].
The solver above is already the right shape for it, which is the argument for staging explicit instantiation first.

### type-extension-06

**Manifest deadlock freedom for the sharing layer.** The refinement that restores deadlock freedom to acquire and release [@balzer-toninho-pfenning-2019-manifest-deadlock-freedom].
Its essence is the classical resource-hierarchy criterion made static: shared channels carry order levels drawn from a partial order, and a process may acquire only a channel whose level is strictly above every level it currently holds, so an acyclic order on acquisitions makes the wait-for graph acyclic.

**This design reserves the hooks now, so that absorbing it later is local to the sharing section.** Four of them:

1. Shared-context entries carry an optional, currently unused **rank** slot, written `a : S_S @ w [ℓ]`.
   Ranks are deliberately **not** identified with worlds: the source calls its levels "worlds", but they are order levels rather than places, and conflating them would couple two unrelated disciplines.
2. The constraint language admits a form `ℓ ≺ ℓ′` alongside the grade order, and the solver already decides preorders.
   Acquire then emits one such constraint per currently-held rank, and the only state needed — the set of held channels — is already in `Σ`.
3. Ranks should be **inferred**: collect the ordering constraints and topologically sort them; a cycle is reported as the potential deadlock, **with the cycle as the diagnostic**, which for an inspectable checker is a feature rather than a failure message.
4. Since acquire and release are the only ports between the shared and linear zones, no other rule changes.

The rank constraint is the first genuinely new domain in the registration design's adoption plan, for the reason that it is small enough to test the interface and closes this hook at the same time ([[proposed/solver-interface#solver-stage-02]]).

### type-extension-07

**Priority-based typing for interleaved linear sessions.** The related alternative for deadlocks among the interleaved _linear_ sessions that `fork` already permits.
The same rank machinery generalizes to priorities on actions.

The two lines it draws on are **Dardha and Gay's priority-annotated classical linear-logic session calculus** and **Padovani's priority-based deadlock freedom for a functional session language**.
Both are **locator-pending**: they are carried by author and subject, neither is held in the corpus bibliography, and neither locator has been verified — so neither may be cited until it is obtained and its metadata checked against the work itself.

### type-extension-08

**Projection-free multiparty safety.** Dropping projection and checking the safety of arbitrary local-type assignments directly [@scalas-yoshida-2019-less-is-more].
Strictly more general than the projection-and-coherence route taken above, and it would live **entirely in the solver** rather than in the rules — which is why taking projection as primary now costs little later.

### type-extension-09

**Validity hypotheses and world quantification, for genuinely mobile code.** The full modal system this world discipline follows also has _valid_ hypotheses — a modality for values portable to every world — and quantification over worlds.
That is the right mechanism for mobile **code**, as opposed to the mobile **handle** to remote code that `@w (U_r B)` already gives.

### type-extension-10

**Migration as a definitional session elaboration.** Operationally, migrating a computation to `w′` is "send the thunk to `w′` and await the result" — it elaborates to a session exchange on a distinguished channel, sending the thunk, receiving the result, and returning it.

**Taking that elaboration as definitional would shrink the core**: worlds would reuse the session metatheory wholesale, with the migration capability as the endpoint.
It is kept primitive here for legibility in the derivation renderer, where a migration should read as one step rather than three.

### type-extension-11

**Biunification in place of the backtracking trail.** Principal type inference with unions and intersections is achievable with **no choice points at all**, by restricting unions to output positions and intersections to input positions [@dolan-mycroft-2017-mlsub], with the essence isolated as a functional pearl [@parreaux-2020-simple-essence] and extended to a Boolean algebra of structural types [@parreaux-chau-2022-mlstruct].

**The polarity sorting of this system is closely related** — sort-level rather than position-level, with the two aligning through focusing.
The designated experiment is to check whether the constraints _generated by inference_ here are always polar.
The conjecture is yes, given where the rules introduce unions and intersections; if it holds, biunification handles every inferred constraint deterministically and the trail survives only for user-written non-polar types.
**Dolan's thesis-length treatment of algebraic subtyping** is the extended account of the same line; it is **locator-pending** on the same terms — carried by author and subject, not held in the corpus bibliography, and not verified.

### type-extension-12

**One mode theory instead of four bespoke features.** This system carries four modal-flavoured structures: the adjunction between values and computations, which is two modes; the world modality, an indexed family; the graded thunk, a graded modality; and prospectively phases, an ordered modality.
Multimodal dependent type theory hosts all of them as a single **mode theory** — a 2-category of modes, modalities, and transformations [@gratzer-kavvos-nuyts-birkedal-2021-multimodal].

**This is a presentation and metatheory tool, not an implementation directive.** When a unified soundness argument is wanted, state the mode theory once rather than proving four bespoke substitution lemmas.

### type-extension-13

**Row-typed records at the polymorphism stage.** A record row variable, `{ℓ:A | ρ}`, generalizing the closed former.
Closed records are a special case of row-typed ones, so this is a refinement rather than a retrofit, and it is the natural companion of [[#type-extension-11]].

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot type-system design record** in full — its notation and judgment forms, its grade algebra, its core calculus and every rule, its set-operation, polymorphism, session, sharing, multiparty, and world sections, its algorithmic subtyping and solver, its full judgment, and its feature staging.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `types`, `syntax`, `grade`, `ctx`, `checker`, `machine`, and `subtype` modules.
3. The **corpus documents carrying fragments of this material** — the implementation track's checked-language account, the mode-and-reference calculus, the module design, and the feature staging — which this document now states in full and which link here rather than restating it.
4. The **pre-reboot programme's tracker**, for two decisions taken against the record after it was written: the closed record former with row typing as its bracketed later refinement, and the recorded refutation of the intersection encoding of records.

**Confidence, by class.**

- **High** — the rules, which are transcribed from the design record rather than re-derived.
- **High** — the as-built statements, each verified against the named module at write time.
- **Medium** — the literature attributions, whose identifiers were transcribed from the contributor's reference register at this pass but whose _claims_ were not re-read from the papers.
- **Marked at the claim** — the two locator-pending attributions in [[#type-extension-07]] and [[#type-extension-11]].
