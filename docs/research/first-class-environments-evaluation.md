# First-class environments and the merge operator — an evaluation against gandr's core

> **Status: evaluation for owner decision — not a decision record.** Deliverable of the evaluation spike `gandr-iysp` (`core: evaluate first-class environments and dependent merges`), routed there by the 2026-08-07 literature intake recorded on the buildout wayfinder `gandr-e08j`.
> Nothing here is adopted.
> [the recommendations below](#the-recommendations) states one verdict per work and prices it; [the owner-decision section](#owner-decisions-this-evaluation-raises) lists the questions filed on the queue bead, which is where the rulings are taken.
>
> **Every claim about the three works is grounded in the works themselves**, read in full from the held artifacts, with the theorem, figure, and page numbers given inline.
> **Every claim about gandr is grounded in the tree**, by module path and, where the claim is about a specific definition, by symbol.
> Where the tracked prose and the tree disagree the disagreement is stated at the claim.
>
> **Register keys** are rows of [the consolidated literature register](bibliography-v2.md): `C-4` (disjoint intersection types), `C-9`, `C-10`, `C-11`, `C-12`.
> The corpus Hayagriva register `docs/gandr/spec/bibliography.yml` carries `C-4` as `oliveira-shi-alpuim-2016-disjoint-intersection` and does not yet carry `C-9`–`C-12`; adding them is deferred to whichever corpus document first cites them, so that no unused entry lands in a register that is migrating.

## The question, and the short answer

The intake routed three works as one cluster.
They are not one proposal, they do not stand or fall together, and the largest error available here is to answer them with a single verdict.

| work                                                               | what it actually proposes                                                                                    | verdict                                             |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| `C-11` Xu, Huang & Oliveira 2025 — apartness and guarded subtyping | a **relaxation of the disjointness judgment** that gandr's own `type-extension-04` names as its precondition | **adopt, narrowed** — as that precondition's answer |
| `C-10` Tan & Oliveira 2024 — the `λ_E` calculus                    | a **metatheory-cost** result: environment semantics makes binding cheap to mechanize                         | **adopt, narrowed** — as a proof technique only     |
| `C-9` Tan & Oliveira 2023 — the `E_i` calculus                     | typing contexts **are** types and environments **are** values                                                | **defer**, with two named re-activation conditions  |

The one-sentence version: **gandr already declined term-level merges conditionally, and `C-11` supplies a better version of the condition than the one the decline names** — that is the finding, and everything else in this document is either the evidence for it or the pricing of the two weaker leads beside it.

## What was read, and what was not

All three routed works were located in the contributor's held-artifact corpus and read cover to cover from the artifacts.

| work   | pages read                                                                                       | not read                                                                                |
| ------ | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| `C-9`  | 34:1–34:32 — the whole paper, appendices A (algorithmic disjointness) and B (fixpoints) included | its companion Coq artifact                                                              |
| `C-10` | 360:1–360:30 — the whole paper                                                                   | its companion Coq artifact                                                              |
| `C-11` | 279:1–279:27 — the whole paper                                                                   | its companion Coq artifact, and the appendix the artifact carries rather than the paper |

**Nothing routed was unobtainable.** The three companion artifacts were deliberately not opened: this repository's standing rule is that external research artifacts are reference-only, never vendored, ported, or depended on ([`docs/WORKFLOW.md`](../WORKFLOW.md#standing-principles-the-short-forms)).
One work is cited without being read and says so at every citation: `C-12`, whose locator was taken from `C-11`'s own reference list and which is not held here.

## The three works, in brief

### `C-9` — Tan & Oliveira 2023, the `E_i` calculus

Two identifications carry the whole design.

**Typing contexts are types.** The grammar for types and contexts is one grammar — `A, B, Γ ::= Int | Top | A → B | A & B | {l : A}` (§3.1, p. 34:12) — so a typing assumption `x : A` is a single-field record type `{x : A}`, a context is an intersection of them, and `Top` is the empty context.
**Environments are values.** At run time an environment is a merge of values, and the reduction relation is `v ⊢ e ↪ e'` with `v` the environment (§3.4, p. 34:19).

On top of those, four operators (§2.3, pp. 34:8–34:9):

* `?` — the **query**, which reifies the current environment; `Γ ⊢ ? ⇒ Γ`.
* `? : A` — the **annotated query**, which reifies only the part of the environment of type `A`; this is environment restriction, done type-directedly.
* `e₁ ▷ e₂` — the **box**, which evaluates `e₂` under the environment `e₁`; this is reflection, and it is the inverse of the query.
* `e₁ ,, e₂` — the **merge**, which concatenates environments.

The merge is **dependent**, and that is the paper's title claim: the right branch is typed under the context extended by the left branch's synthesized type, so a later declaration may refer to an earlier one.
Rule `Typ-dmerge` (Fig. 2, p. 34:15) is

```text
Γ ⊢ e₁ ⇒ A     Γ & A ⊢ e₂ ⇒ B     A * B     A * Γ
──────────────────────────────────────────────────
Γ ⊢ e₁ ,, e₂ ⇒ A & B
```

with **two** disjointness premises, not one.
`A * B` is the ordinary disjointness of `C-4` (Def. 1, p. 34:14: `A * B` iff no ordinary type is a supertype of both).
`A * Γ` is the new one, and it exists solely to keep reduction deterministic: without it a query in the right branch could resolve either to a value from the left branch or to one already in the context (the worked counterexample is at p. 34:16).

Two consequences worth naming because they are easy to miss.
First, `E_i` **has no variables at all** — labels replace variable names, and abstractions `{e}^m` abstract over their _input type_ rather than over a variable (§3.1, p. 34:13).
Second, the semantics is a **type-directed operational semantics**: a casting relation `v ↪_A v'` selects components of a merge by type, so **types affect run-time behaviour** (§3.4, p. 34:18).

Results: determinism (Thm. 7, Cor. 8), progress and preservation in generalized form (Thms. 13–14, Cors. 15–16), type safety (Cors. 17–18), and a type-directed encoding of the whole of `λ_i` (Thm. 20, §5) — all mechanized in Coq.
What is **not** claimed: conservativity over the lambda calculus, and termination, neither of which is investigated (`C-10` says so of `C-9` explicitly, §7, p. 360:25).

### `C-10` — Tan & Oliveira 2024, the `λ_E` calculus

Same authors, different thesis.
`C-9` argues environments should be first class; `C-10` argues that **formalizing with a small-step environment semantics is cheaper than formalizing with substitution**, and that once you have done it, first-class environments come "almost for free" (Abstract; §1, p. 360:2).

The calculus keeps `C-9`'s query, box, merge, and label selection, and changes three things.
It uses **de Bruijn indices**, so α-equivalence is syntactic identity (§2.4, p. 360:7).
It has **closures** `⟨v, λA. e⟩` as values, and beta reduction is substitution-free — `v ⊢ ⟨v₁, λA. e⟩ v₂ ↪ (v₁ ,, v₂) ▷ e` (rule `Step-beta`, Fig. 3, p. 360:14): the argument is _merged into the environment_, never substituted into the body.
It keeps `&` as a **limited** intersection used for context and record concatenation, with `ε` the empty environment (Fig. 1, p. 360:12).

**Ambiguity is a static error**, enforced by a containment judgment `l : A ∈ B` (Fig. 2, p. 360:13, rules `CTM-RCD` / `CTM-ANDL` / `CTM-ANDR`): `CTM-ANDL` requires `l ∉ label(C)` before it will look left, so a label occurring twice in an environment makes _selection_ ill-typed while the environment itself stays well-typed (§2.4, p. 360:8).
The authors compare this directly to how Haskell handles names imported from multiple modules.

The results are the point, and they are the strongest metatheory package of the three:

* determinism of selection and of reduction (Lem. 4.1, Thm. 4.2, Cor. 4.3);
* syntactic type soundness (Thms. 4.8–4.9, Cors. 4.10–4.11) — and, stated explicitly at p. 360:15, **the substitution lemma is not needed, because there is no substitution in the semantics**;
* semantic type soundness and **termination** by a logical predicate (Fig. 4; Thm. 4.12, Thm. 4.16), where the runtime environment value plays the role the simultaneous substitution `γ ⊨ Γ` plays in the standard proof (p. 360:16–17);
* small-step and big-step semantics equivalent (Thm. 4.21);
* **completeness and conservativity over the call-by-value STLC** (Thms. 4.22–4.23 for typing, 4.25–4.26 for dynamics) — and the paper notes at p. 360:19 that the conservativity proof is **the only place in the entire development where substitution has to be reasoned about at all**;
* completeness and conservativity over the closure calculus `λρ̂` (Thms. 4.27–4.28);
* compilation to a SECD-style abstract machine with semantic preservation (Thm. 5.1, §5);
* an extension with mutable references retaining determinism, progress, preservation, and machine correctness (Thms. 6.1–6.4, §6.1).

All in Coq. §6.2 then shows how to **remove** first-class environments and keep only the environment semantics, yielding a plain closure calculus — the paper's own statement that the two halves separate.

Its stated limits (§6.3, pp. 360:24–25) matter more here than its results:

* only simply-typed calculi are studied; extending to System F needs type-level substitution and "how to redesign the type syntax of System F and its type system with this closure formulation is non-obvious";
* the reduction strategy is **call-by-value and weak** (Table 1, p. 360:26), and **full reduction is not studied** — the paper names dependent types' conversion checking as exactly the setting that needs it ("to check `Vec (1 + 1)` is equal to `Vec 2` we need to reduce `1 + 1` to `2`… Dealing with equality requires reducing sub-terms at any position including those inside lambdas.
  However we have not studied full reduction in our work").

### `C-11` — Xu, Huang & Oliveira 2025, apartness and guarded subtyping

This one is about the merge operator and has nothing to do with environments.

Disjointness (`C-4`) makes a merge legal only when **no** future upcast could be ambiguous.
That is strictly stronger than needed, and the paper's opening example is the one that matters: `show = showInt ,, showBool` where both have return type `String` is _rejected_ by disjointness, because `Int & Bool → String` is a common supertype — even though every actual call site disambiguates (§2.2, p. 279:6).
Disjointness therefore rules out conventional function overloading.

**Apartness** `A * B` relaxes this: overlapping types are admitted so long as the overlapping parts stay distinguishable at the point of use.
Its specification (Def. 2.4, p. 279:8) quantifies over _minimal ordinary type components_ (`MinOrd`, Def. 2.3): `A *_s B` holds when, for every minimal ordinary component `C₁` of `A` and `C₂` of `B`, either one is `⊤` or neither is a subtype of the other — that is, **no shadowing**.
An algorithmic formulation is derived as the negation of a shadowing relation (Fig. 5, p. 279:14), sound (Thm. 3.2) and, under a type well-formedness restriction, complete (Thm. 3.3).

Apartness alone is not enough, because it _defers_ the ambiguity check rather than discharging it.
**Guarded subtyping** `A ≾ B` (Fig. 6, p. 279:15) is the deferred half: a restricted subtyping that holds only when the coercion from `A` to `B` is **unique**.
Ordinary subtyping asserts a coercion exists; guarded subtyping asserts exactly one does.
Soundness is Thm. 3.4 and safe casting Thm. 3.5.

**Guarded subtyping is deliberately not transitive**, and the paper is explicit that this is a trade, not an oversight (§2.4, pp. 279:8–279:9): prior work preserves transitivity and loses determinism, or keeps disjointness and loses flexibility; this design sacrifices transitivity to keep both determinism and flexibility.
It aligns the choice with gradual typing, where consistency and consistent subtyping are non-transitive for the same kind of reason.

Two further pieces:

* **Type normalization** `|A|_B` (Fig. 10, §5) rewrites unrestricted intersections into apart canonical forms; idempotent, total, deterministic, sound (Thms. 5.1–5.5).
  It is what lets the _source_ calculus `λ_|*|` admit types like `Int & Int` that the target `λ_*` forbids, with a type-safe translation and operational correspondence (Thms. 5.11–5.12).
* **Type difference becomes total** (§6).
  The partial subtraction of `C-12` is recovered as `A \_s B = |A|_B` (Thm. 6.4), and any two types' conflicts are resolvable by normalizing twice (Thm. 6.5).

The result is the first calculus supporting function overloading, return-type overloading, extensible records, and nested composition **simultaneously and deterministically**.
Open: completeness of the type-dispatching relation, which the paper calls intractable and leaves open (§4.1, p. 279:18); union types and disjoint polymorphism are future work.

One encoding decision must be flagged, because it collides with gandr: `C-11` encodes a record type as an **overloaded function over a singleton label type**, `{l : A} ≜ Sig l → A` (§3.1, p. 279:11).
That is a deliberate simplification bought by apartness, and it is the one part of the design gandr must not take — see [fce-finding-05](#fce-finding-05).

## What gandr's core actually does today

Verified against the tree, not against the prose describing it.
Where a tracked document and the tree disagree, the disagreement is recorded here rather than repaired silently.

### fce-finding-01

**gandr's operational core is already environment-based, and is already almost substitution-free.**

The L machine is the live driver and reduces by **environment extension only, never textual substitution** — stated in its module contract and realized in its state (`crates/core-sequent/src/machine.rs`, module documentation and `LMachine`).
The runtime environment is `gandr_core_sequent::machine::LEnv`, a persistent innermost-first association `x ↦ LValue` with `extend` and shadowing `lookup`; its covalue twin is `LContEnv`, `α ↦ Continuation`.
Closures and handler frames capture both (`LFrame::Handle` and the U-shift thunk closure both carry `env` and `coenv`), and readback closes a decoded computation under an environment by folding substitutions over it (`gandr_core_sequent::unfocus`).

The substitution that used to sit beside this is **already retired**: `gandr_core_checker::subst`'s module documentation records that its computation-level companion `subst_comp` retired with the CEK machine, and the surviving `subst_value` exists for one purpose — instantiating the motive of the identity type at each `ValueType::Path` endpoint.

**So the benefit `C-10` argues for at the implementation layer, gandr already has.** What gandr does not have is `C-10`'s _proof_ organized that way, and that is where the remaining value is.

### fce-finding-02

**Typing contexts are a two-zone stack of named hypotheses; they are not types, and they are not first class in any sense.**

`gandr_core_checker::ctx::Ctx` holds `entries: Vec<(String, ValueType)>` — the intuitionistic zone `Γ`, a binding stack whose `lookup` scans from the innermost binding so shadowing behaves — and `sigma: Sigma`, the linear zone, whose hypotheses admit **neither weakening nor contraction** and whose `consume` is single-shot.
`Σ` is vacuous in v0 by design and its discipline is pinned by direct unit tests rather than left vacuously satisfied.

Three things follow that bear directly on `C-9`.
Contexts are **named**, not positional, and the names are load-bearing downstream: the pipeline's goals report slices `Ctx::bindings` to show the bindings local to a hole.
Contexts are **two-zone**, and a linear obligation is not a component of an intersection — there is no reading of `Γ & A` that carries "must be consumed exactly once".
And the full designed judgment is wider still: the module design states it as `Γ; Δ; Θ; Σ ⊢_w m ⇕ σ` with a world index, and records that "a module is not typed in a smaller context than an expression is" ([the module design](../gandr/spec/surface-language/proposed/modules.md#the-module-grammar)).

The kernel is separate and different again: `gandr_kernel_core::env::Environment` is a **global declaration** environment with an admission choke point and unforgeable `CheckedId` handles, and kernel terms are **nameless de Bruijn** (`gandr_kernel_core::term::DeBruijnIndex`).
So gandr already has the index representation `C-10` recommends — in the kernel, one layer below the checker that would have to adopt it.

### fce-finding-03

**No merge exists, no intersection former exists, and both absences are recorded decisions rather than gaps.**

The value-type former list (`gandr_core_checker::types::ValueType`) carries a **primitive closed record** `Record(BTreeMap<String, Rc<ValueType>>)`, canonical in field order, with structural width-and-depth subtyping decided in `gandr_core_checker::subtype`.
The computation-type former list (`CompType`) carries `With` — the lazy product `B & B′` of linear logic's additive conjunction — which is **not** an intersection: a `With` types two _different_ computations, an intersection types the _same_ computation twice.
No intersection former exists on either sort; the module documentation of both `types` and `subtype` records intersections as a later-stage extension.

At the surface, `/\` parses as a type operator in its own precedence band (`gandr_surface_parser`'s label table; the band is pinned by `surface-grammar`'s contract tests) and lowering rejects it as `LowerError::Unsupported` (`gandr_surface_engine::lower::types`).
There is no merge operator at the surface at all.

The corresponding decisions in the corpus:

* **Unions are positive, intersections are negative**, on proof-theoretic grounds, with call-by-push-value enforcing the sorting by sort rather than by side condition ([the type-system track](../gandr/spec/implementation/type-system.md#unions-and-intersections)).
* **The intersection encoding of records is refuted, with its reason**: intersection here is negative-only and expresses _overloading_ rather than field combination, "so the encoding is not merely inconvenient here — it is at the wrong polarity" ([the type-system track](../gandr/spec/implementation/type-system.md#records)).
* **Term-level merges are intentionally absent**, and this is the load-bearing one: `type-extension-04` says a merge "is intentionally absent, because coherence requires a **disjointness** judgment", citing `C-4`, and that "if overloading by merge is wanted later, it arrives **together with** that judgment, never before it" ([the type-system track](../gandr/spec/implementation/type-system.md#type-extension-04)).

### fce-finding-04

**gandr has already paid the non-transitivity price that `C-11`'s central trade demands.**

With the hole type `Unknown` on both sorts, the relation `gandr_core_checker::subtype` decides is **consistent subtyping**, which is reflexive and deliberately **not transitive**; the module documentation records the cost explicitly, and `gandr_core_checker::conformance`'s `consistency_is_not_transitive` is a live witness asserting that `Int ≲ ? ≲ Str` does not compose.

This is the single largest thing in gandr's favour for `C-11`.
A project whose subtyping relation is transitive would find guarded subtyping expensive precisely at the point where the relation stops composing, and would have to relearn what that breaks. gandr's solver, its conformance suite, and its recorded reasoning already contain that entry.

### fce-finding-05

**The module layer's landed rung already has dependent-declaration behaviour, expressed operationally rather than in the type system.**

What is built is `module-rung-01`: a module declaration lowers to one named item whose term is a canonical record, or a bind chain returning that record when members must be sequenced, with members evaluated **exactly once in source order** and **earlier member binders scoping over later member definitions**, so a member sees its predecessors and never its successors ([the module design](../gandr/spec/surface-language/proposed/modules.md#what-is-built-and-what-this-document-describes)).

That is exactly the behaviour `C-9`'s dependent merge exists to type — `let x = 2; let y = x + x; let main = x + y` is `C-9`'s own motivating example (§2.2, p. 34:7) — and gandr obtains it from the bind chain instead.
The difference is not cosmetic: gandr's version is a _lowering_ property, so it cannot be stated as a type, cannot be reasoned about compositionally, and does not survive a module being passed as a value.
`C-9`'s version is a typing rule.

The rest of the module design is not built.
Functors are `U_r (σ₁ → F σ₂)`, first-class packages are `Package σ ≜ ∃β̄. U_r (F σ)`, and the design's own integration table already assigns unions to signature _values_ and intersections to functor _bodies_, "polarity-sorted like the core".

### fce-finding-06

**The theory layer is held on a question whose stated reversal condition is, in shape, what these calculi supply.**

`theories.md` is held by owner ruling of 2026-08-07.
The reason is import visibility: an `extend` in one module against a theory declared in another "is an effect with no name", so a client imports the extending module for its effect, and resolution then depends on which effects are in scope and potentially on import order — "where the property this project requires is **order-free resolution with ambiguity a type error**".
The recorded reversal condition is "a design under which an extension is a named, importable entity — **or an order-free coherence regime for extension visibility — with ambiguity a type error**".

`C-10`'s containment judgment and `C-11`'s apartness-plus-guarded-subtyping are both exactly that regime — order-free, type- or label-directed resolution over a concatenation operator, with ambiguity a static error — but at the **value and type** level rather than at module-import scope.
Whether the analogy transports is the substance of [fce-question-03](#fce-question-03); this document does not settle it, and it is worth stating plainly that these works do **not** supply the other disjunct (an extension as a named importable entity), so at best they answer half the condition.

### Description-versus-artifact conflicts found

**None.** Every tracked claim checked in the course of this evaluation held against the tree: the intersection refutation, the merge decline, `module-rung-01`'s as-built description, the two-zone context, the environment-only L machine, and the recorded non-transitivity of consistent subtyping.
The two defects `modules.md` already carries at `Mod-Value` and in its linear-zone bookkeeping are pre-existing, are flagged in that document at the claim, and are untouched here.

## The evaluation

### What the cluster would buy

**`C-11` closes a precondition gandr wrote for itself, and improves on it.** `type-extension-04` is not a refusal of merges; it is a **conditional** refusal naming disjointness as the missing judgment.
`C-11` supplies a judgment that is strictly better for gandr's purposes than the one named: apartness admits the overloading disjointness rejects, guarded subtyping restores determinism, and type normalization plus total type difference give a conflict-resolution story `C-4` does not have.
Under the review contract this is an **opportunist lead**, not a proposal: while evaluating the routed cluster, the thing found is an improvement to a decision gandr has already taken.

**`C-10` offers a cheaper metatheory, in the lane where gandr's costs actually are.** gandr's Agda metatheory is the oracle for the L machine, and the L machine is already environment-based.
`C-10` demonstrates — mechanized, with normalization and conservativity, not merely asserted — that stating soundness over a runtime environment value removes the substitution lemma and the shifting/renaming lemma family entirely, with the environment value `v ∈ V[Γ]` doing the work of the simultaneous-substitution relation.
That is a direct saving against a cost gandr will otherwise pay, and it costs nothing at the language level because gandr's dynamics already agree with the shape.

**`C-9` offers environment restriction and reification as consequences rather than features.** Under "contexts are types", `? : A` gives type-directed environment restriction and `▷` gives sandboxed evaluation with no new machinery.
The capability reading `C-10` builds on that (§2.5) — a boxed module can only see the environment it was handed, so ambient authority is impossible by construction — is a genuinely attractive property.

### What it would cost, and what it interacts with

**`C-9` wholesale costs two commitments, not two representations, and that distinction is the whole verdict.**

1. **The polarity sorting.** `C-9` requires contexts to be intersections of single-field record types. gandr's records are positive and its intersections are negative, and the record-as-intersection encoding is refuted _on that ground_.
   Reversing this is not a representation swap: the sorting is inherited from the proof theory, and it is what discharges the evaluation-context restriction on union elimination _by construction_ rather than by side condition ([the type-system track](../gandr/spec/implementation/type-system.md#unions-and-intersections)).
   Trading it away to get contexts-as-types would be paying a proof-theoretic price for an ergonomic gain.
2. **Type-erased execution.** `C-9` and `C-10` both use a type-directed operational semantics, where a casting relation selects merge components **by type at run time**. gandr's IL is focused and its machine is type-erased; `core-sequent` exists to make execution a flat loop over commands with the checker in a different crate under a differential contract.
   Putting types into the runtime relation is the deepest incompatibility in this cluster, and it is a fact about the machinery rather than about gandr's preferences.

`C-9` also has no home for the linear zone: `Γ & A` cannot express "consume exactly once", and `Ctx`'s `Sigma` is a committed, frozen shape.

**`C-11` costs much less than it looks, provided one thing is not taken.** It needs an intersection former (already planned, on the negative sort), an apartness judgment, a normalization procedure, and a second, non-transitive relation in the solver beside consistent subtyping.
Its subtyping is BCD, whose distributivity `(A → B₁) & (A → B₂) ≤ A → (B₁ & B₂)` is _negative-sort behaviour_ — so apartness is orthogonal to gandr's polarity sorting rather than in tension with it.
What must **not** be taken is `C-11`'s record encoding `{l : A} ≜ Sig l → A`: that is the positive-record-as-negative-intersection collision again, and gandr's primitive `ValueType::Record` stays.
The paper's own framing supports the split — the encoding is a convenience apartness _enables_, not a load-bearing part of apartness.

**`C-10` as an implementation directive costs gandr's names.** Moving the checker to de Bruijn would move a load-bearing identity notion: `Ctx` entries are named, the goals report slices them, and the incremental pipeline's footprints (`gandr_core_checker::footprint`) key on binder names.
`C-10` itself concedes the point from the other side — it keeps labels beside indices precisely because "the names matter" for declarations and modules (§2.4, p. 360:8).

### What it forecloses

Adopting `C-9` wholesale forecloses the two-zone judgment and the world and grade annotation columns, which are the language's committed shape and are cheap to _extend_ only because they are annotation columns on a context that is a list.
Adopting a type-directed operational semantics forecloses erasure at the IL, and with it the differential contract between checker and machine that `core-sequent` is organized around.
Neither is worth foreclosing for what is on offer.

Adopting `C-11` narrowly forecloses nothing: it fills a slot the design already left open, on the sort the design already assigned to intersections.

## The recommendations

### fce-recommendation-01

**`C-11` — adopt, in a narrowed form, as the answer to `type-extension-04`'s named precondition; schedule it with the `+setops` feature stage, not now.**

The narrowing, stated so that adopting it is not adopting the paper:

1. **Apartness replaces disjointness as the judgment `type-extension-04` waits on.** The recorded decision changes from "merges arrive with a disjointness judgment" to "merges arrive with an apartness judgment and a guarded-subtyping relation".
2. **It applies to the negative intersection only.** gandr's primitive positive `ValueType::Record` is retained and `C-11`'s `Sig l → A` record encoding is **not** adopted.
   The polarity sorting is unamended.
3. **Guarded subtyping enters the solver as a second relation beside consistent subtyping**, both non-transitive, with `conformance`'s `consistency_is_not_transitive` as the standing precedent for how a non-transitive relation is pinned here.
4. **Type normalization and total type difference are recorded as available, not mandatory.** They are what makes unrestricted intersections writable at the surface; whether gandr wants unrestricted intersections is a separate question that need not be answered to take the judgment.

Why now and not later: the cost of recording this is one amendment to a recorded decision, and the cost of _not_ recording it is that the next session to reach `type-extension-04` reads a precondition naming a judgment that has since been superseded, and either builds against the weaker one or re-derives this evaluation.

This is an owner ruling because it amends a recorded decision — [fce-question-01](#fce-question-01).

### fce-recommendation-02

**`C-10` — adopt, in a narrowed form, as a metatheory-presentation candidate for the L machine; decline it as a change to the checker or the surface.**

What is adopted is the **proof technique**: state the L machine's soundness and normalization over a runtime environment value, with `v ∈ V[Γ]` in the role the simultaneous substitution plays, and drop the substitution and shifting lemma families. gandr's dynamics already have the required shape, so this costs no language change; it is a change to how the Agda development is organized.
The natural home is the metatheory spike and obligation queue `gandr-hpck` — [fce-question-02](#fce-question-02).

The decline of the implementation half is a claim, so it is answered against the four questions the review contract requires ([`docs/workflow/review.md`](../workflow/review.md#before-a-decline-binds-answer-four-questions)).

1. **Should it apply?** Partly, and the split is clean.
   If gandr's setting were free, an environment-based _specified_ semantics would be the right choice — it is what the machine already does, and the gap between specification and machine is the differential contract's entire burden.
   What would not apply even in the ideal is the nameless checker: gandr's inspectable-checker, hole-report, and incremental-footprint stories are built on binder names being visible in `Γ`, and `C-10` itself keeps labels beside indices for the same reason.
2. **What exact delta would make it apply?** `λ_E` extended with **full reduction** — reduction under binders — preserving its normalization and conservativity results, plus a polymorphic extension.
   Both are named as unstudied in the paper's own §6.3, and the first is named there against precisely gandr's use case: deciding definitional equality of dependent types.
   With those two, `λ_E` would be a candidate presentation for gandr's specified dynamics, not merely for its machine's.
   This delta is a fact about the machinery — a case the source does not cover — not a fact about gandr.
3. **What does the delta cost, and what kind of change is it?** The metatheory half is a **representation** change and is cheap: it changes how the Agda development states things, not what the language is.
   The checker half is a **commitment** change: `Ctx`'s named entries feed the goals report and the incremental pipeline's footprints, so moving to indices moves an identity notion other subsystems key on.
   The two halves are separable, which is why the recommendation splits rather than declining outright.
4. **What would it unlock, or eliminate?** Unlock: a substitution-free soundness argument for the L machine, and a specification already in the machine's shape.
   Eliminate: the shifting and renaming lemma family in the Agda development, and — this is the striking one — most of what `gandr_core_checker::subst` exists for.
   That elimination is **already half-complete**: `subst_comp` retired with the CEK, and the surviving `subst_value` has one consumer, the identity type's motive instantiation.
   Retiring machinery is the payoff most often missed, and here it is measurable.

### fce-recommendation-03

**`C-9` — defer, with two named re-activation conditions.
This is not a decline.**

`C-9` is a better idea than gandr's current arrangement in one specific respect, and saying so is the point of deferring rather than declining: environment restriction and reification fall out of "contexts are types" instead of being built.
Its cost is concentrated in two commitments gandr holds for reasons that have nothing to do with `C-9` — the polarity sorting and the type-erased IL — so the deferral rests on a genuine conflict of commitments rather than on a preference, and it will look different if either commitment ever moves.

Two re-activation conditions, either sufficient:

* **`fce-reactivation-01` — the intersection former lands.** When `+setops` brings `∩` and (under [fce-recommendation-01](#fce-recommendation-01)) apartness, re-read `Typ-dmerge` **for its dependent half specifically**.
  The `A * Γ` premise, which makes a merge's right branch see the left branch's synthesized type, is the typing rule for exactly the dependent-declaration behaviour gandr's landed record-module rung already has operationally ([fce-finding-05](#fce-finding-05)).
  Whether the module layer wants that behaviour _as a type_ rather than as a lowering property is a real question and it becomes cheap to answer at that point.
* **`fce-reactivation-02` — the theory layer is unheld.** If a design for order-free extension visibility is taken up, `C-9`'s and `C-10`'s ambiguity-as-static-error discipline is the closest published precedent of the right shape and should be read then, alongside whatever answers the named-importable-entity half.

## Owner decisions this evaluation raises

Three questions are filed on the queue bead `gandr-iysp.1` (`core: decision queue for first-class environments and dependent merges`), each self-contained, each with the recommendation first.
They are named here so this document and the queue cross-reference; the questions themselves live on the bead and the rulings land in whichever artifact each names.

### fce-question-01

Whether apartness and guarded subtyping supersede disjointness as the precondition `type-extension-04` names for term-level merges, and if so where that amendment is recorded given the corpus migration.
Filed as `gandr-iysp.1-question-01`.

### fce-question-02

Whether the `λ_E` environment-semantics presentation is evaluated for the metatheory lane, and under which bead.
Filed as `gandr-iysp.1-question-02`.

### fce-question-03

Whether the ambiguity-as-static-error discipline of these calculi counts as a candidate against the theory layer's recorded reversal condition, given that it answers the order-free-resolution disjunct at value scope and does not answer the named-importable-entity disjunct at all.
Filed as `gandr-iysp.1-question-03`.

## Source and confidence

Written against three source classes, named because a change with no declared source set cannot be fidelity-reviewed.

1. **The three routed works, read in full from the held artifacts** — `C-9` (32 pages), `C-10` (30 pages), `C-11` (27 pages).
   Every theorem, figure, rule, and page number in this document was read off the artifact.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `ctx`, `types`, `subtype`, `syntax`, `subst`, `footprint`, and `conformance` modules; `gandr-core-sequent`'s `machine` and `unfocus` modules; `gandr-kernel-core`'s `env`, `term`, and `conv` modules; `gandr-surface-engine`'s `lower::types`; `gandr-surface-parser`'s label table.
3. **The corpus documents carrying the decisions this evaluation bears on**: the type-system track's records, unions-and-intersections, and `type-extension-04` sections; the module design's built-rung account, typing rules, integration table, and staging ladder; the theory design's hold banner and reversal condition.

**Confidence, by class.**

* **High on what the three works say.** Each claim carries its locator inside the work, and each was read rather than recalled.
* **High on the as-built account.** Every claim names the module it was read from; the record-subtyping, non-transitivity, and environment-only claims were read at the definition, not at the doc comment.
* **Medium on the cost estimates.** The polarity and type-erasure conflicts are argued from the corpus's own recorded reasons and are as firm as those reasons; the estimate that `C-11` costs "much less than it looks" rests on apartness being stated over BCD subtyping and therefore orthogonal to sort assignment, which is an argument from the paper's structure and has had no independent pass.
* **Low, and marked as such, on `fce-finding-06`.** That the theory layer's reversal condition and these calculi's ambiguity discipline have the same _shape_ is a reading, not a result; the two operate at different scopes and the mismatch is stated rather than argued away.
