# The module system

**Proposed.
The module system described here is not built.** What exists today is a record-shaped module declaration — the bottom rung of the ladder below — and everything that makes a module system a module system rather than a namespace convention (signatures as a sort, functors, sealing, first-class packages, implicit resolution, located modules) is design.

The design is a module layer for call-by-push-value in which **the core and the module language are one language**: functors, functions, and type constructors are a single construct, and a structure is a value.
It carries an inference fence borrowed from 1ML, where that unification was first worked out; an implicit-resolution discipline that separates three things the obvious design conflates; a distribution story assembled entirely out of features the core already owes; and an ascription rule that makes generative and applicative sealing two readings of one construct.

Read the status line at each decision and each rung: this document says what the design _is_, and the next section says what the tree _has_.

## What is built, and what this document describes

The gap between the two is wide, and it is the first thing to carry away.

**Built, and verified against the tree at write time.**

- **A module declaration is a checked record.** `gandr-surface-grammar`'s `surface::term` module contributes a `module_declaration` rule — `module M (: #{ field: Type, … })? { … }` with a body of `def` members — and `gandr-surface-engine`'s `lower` module lowers it to **one named item** whose term is a canonical record, or a bind chain returning that record when members must be sequenced.
  Members evaluate **exactly once, in source order**; each non-duplicate definition contributes one field; earlier member binders scope over later member definitions, so a member sees its predecessors and never its successors.
  A duplicate member is a lowering error, and an unmatched member signature is a dangling-signature error in strict mode and a hole carrying a missing-definition note in total mode.
- **The record ascription is transparent and value-only.** The optional `: #{ … }` is checked against the returned record payload; it does not seal identities, and it does not constrain a member's effect row, so a member that performs a host effect keeps it while the module value still checks against the record shape.
  The runnable example that pins this is `crates/surface-corpus`'s `examples/model/29-modules.gandr`, which mixes a shell-command member into an ascribed module and asserts the module's evaluated result.
- **Path selection is record projection.** `M.inner.field` is ordinary projection once `M` is in value scope; there is no module-select form in the kernel.
- **`import "URI" as name ;` parses and is never lowered.** The rule is in the same grammar module; no lowering path consumes it, and no resolver exists.
  The URI is a plain string literal so that new schemes cost no grammar change.
- **Reserved module _namespaces_ exist and are not modules.** `gandr-surface-engine`'s `host` module reserves `fs`, `env`, and `proc` as source-level namespaces whose member calls elaborate to effect operations, and its `ffi` module gives an `extern` block's library namespace the same treatment.
  Neither is a value, neither is projectable, and a call selecting an unknown member of one is an error rather than a fall-through to record projection.
- **The typing machine has no module frames.** `gandr-core-checker`'s `machine` module carries thirty-odd frames for the core forms — abstraction, application, pairs, records and projection, thunk and force, bind, the case families, split, annotation, effect operations and handlers, the reified stack, and the identity eliminator — and none of them is module-specific, because a module declaration reaches the machine already lowered to records and binds.

**Designed, and not built.** Signatures as a sort of their own, functors, opaque sealing, first-class packages with dynamic signature matching, implicits in any form, located signatures, the ticket protocol, module migration, futures, and transparent-existential lifting.
Sessions, manifest sharing, and worlds — the three features the distribution story is assembled out of — are themselves designed and unbuilt, as [[modes-and-references#The substrate, and what is actually built]] records.

**Neither designed nor built here.** Resolution of any kind: no package identity, no lockfile, no cache, no fetch.
The toolchain half of that question — package identity, resolution, the cache, distribution — is [[../../implementation/proposed/packages]]; the language half is what an import _elaborates to_, which is the module value described here.

**The phase commitment, which is stronger than what is built.** The implementation track commits to **modules as their own primitive layer** rather than as sugar ([[../../implementation#The build-out at a glance]]), and records the historical two-way conflict — modules as compile-time namespaces against modules elaborating to canonical records — as superseded, with neither old reading citable as current ([[../../implementation/roadmap#Decisions of record pinned for their phases]]).
So the record lowering above is **the current rung's mechanism, not the design's destination**: the destination is a primitive module former in the kernel IL with coercive matching, sealing with export replay, generative functors, and the package existential.
Nothing in this document should be read as an argument that records are where modules end up.

## The unification thesis

The stratification an ML-family language usually carries — a core of types and expressions, and above it a separate higher-order functional language of signatures, structures, and functors — duplicates syntax and semantics and bounds expressiveness.
The unification that removes it makes functions, functors, and type constructors **one construct**, and structures and records **one thing** [@rossberg-2018-1ml].

Call-by-push-value does not merely permit that unification; it **sorts** it.

### The correspondence, and what the polarity forces

| the 1ML concept                | what it is here                                          |
| ------------------------------ | -------------------------------------------------------- |
| structure (a record of values) | a value-layer record, iterated `A × A′` with labels      |
| signature                      | a value type with existential packaging                  |
| functor                        | a **thunked computation** `U_r (σ₁ → F σ₂)`              |
| first-class module             | a packaged value `Package σ` ([[#First-class packages]]) |
| functor application            | `force f m`, a computation                               |

The last two rows are not stylistic.
**Functions are computations here**, so a functor — a function on modules — is a computation, and storing one, passing one, or putting one in a structure means thunking it.
The grade `r` on that thunk records how many times the functor may be applied, which is how module-level linearity gets expressed without a second mechanism.

A note on spelling, because two notations meet in this document.
The kernel notation `U_r B` is written `U[r] B` at the surface, and `U B` abbreviates `U[ω] B` ([[../../surface-language#Types: every former]]).
Typing rules below use the kernel notation because that is the notation the rules are stated in; program text uses the surface spelling.

### module-decision-01

**The predicativity fence is an inference policy, not a soundness requirement.**

1ML distinguishes **small** types — those with no quantifiers and no abstract types — from **large** ones, and admits only small types at _implicit_ instantiation.
That restriction is what makes its inference decidable.

This design adopts the small/large distinction as the **default inference fence**, with two calibrated escape hatches rather than a hard wall:

1. **Explicit large instantiation is always allowed.** Checking a _given_ impredicative instantiation is unproblematic; only _guessing_ one is hard.
   The undecidability lives in inference and signature matching, not in the types.
2. **Inferred impredicativity is fuel-bounded.** The implicit-search machinery ([[#Implicit resolution]]) is already an explicit semi-decision procedure with fuel and a trail, so large instantiation can ride the same mechanism — or a Quick-Look-style algorithm, whose deployment in a production compiler showed the required changes to be modest and localised [@serrano-hage-peytonjones-vytiniotis-2020-quick-look].
   Fuel exhaustion is reported as a diagnostic showing the obligation chain, and shown in the derivation surface rather than merely returned.

**Why the fence is not simply dropped.** One reason available elsewhere does not apply here and one does, and separating them matters because only the second is this document's to spend.

`Type : Type` threatens _logical consistency_, and that argument is settled outside this document: the universe is predicative and `Type : Type` is refused at the surface ([[../../surface-language#Types: every former]]), so the module layer inherits the refusal and does not re-argue it.

What remains, and what the fence is actually buying, is **termination and principality of inference** — a usability property rather than a soundness one.
Beyond the small/large line, different search orders find different and incomparable types, so a _successful_ inference becomes heuristic-dependent.
Semi-decidability is acceptable; silent heuristic-dependence of successful outcomes is the thing to ration.
Hence the shape: fence by default, explicitness or fuel to cross it deliberately.

_Status:_ adopted as the design's default.
The fence has no implementation to be measured against, and the choice between riding the implicit-search fuel and adopting a Quick-Look-style algorithm is [[#module-question-02]].

## Implicit resolution separates three things

The obvious design routes visibility, search effort, and usage through one mechanism: a **coeffect** on the implicit — an annotation on how a binding is _consumed_ from its context, the dual of an effect on what a computation _does_.
They are three different things, and conflating them mixes elaboration time with run time.

| axis              | the question it answers                    | where it lives                                                         |
| ----------------- | ------------------------------------------ | ---------------------------------------------------------------------- |
| **visibility**    | _which_ implicits are candidates           | the type system, scoped by worlds                                      |
| **search effort** | _how hard_ the solver may look             | the elaborator, as fuel — a solver parameter, never a program property |
| **usage**         | _how often_ the resolved value may be used | the type system, as a grade on the implicit's thunk                    |

**Visibility is world-scoped.** An implicit declared at world `w` is visible at `w`, so the candidate set falls out of the world discipline the language already has instead of being a separate scoping rule maintained for implicits alone.
**Search effort is fuel**, a solver parameter with a default depth bound, reported in diagnostics and absent from every type.
**Usage is a grade**: `U_ω` for an ordinary implicit, `U_1` for a linear one, so a linear implicit used twice is caught by the ordinary grade constraints at the use sites rather than by anything implicit-specific.

### module-decision-02

**Implicits resolve statically; resolution itself has no runtime cost.**

The alternative is a staged semantics in which resolution _is_ a computation that genuinely consumes a grade.
That alternative is coherent — it is the honest reading if you want resolution to be a first-class runtime event — and it is **declined**, because it commits the language to runtime implicit dictionaries.

_Status:_ declined, with a reversal condition.
It reopens if a use case requires resolution to observe runtime state — a dictionary chosen by a value rather than by a type — at which point the cost it was declined for is the cost the use case is asking to pay.

## Distribution is elaboration, not new primitives

The design's distribution vocabulary is taken from Alice ML, which shipped first-class packages with dynamic signature matching, futures, and distribution as primitive machinery [@rossberg-lebotlan-tack-brunklaus-smolka-2006-alice].
Here none of those is new machinery: each is an **elaboration** into a feature the core already owes.

| the Alice ML primitive | what it elaborates to here                                                            |
| ---------------------- | ------------------------------------------------------------------------------------- |
| `pack` / `unpack`      | a first-class package with a runtime signature match ([[#First-class packages]])      |
| `offer` / `take`       | a **shared session** serving a package ([[#Serving a package over a shared session]]) |
| `lazy` / `spawn`       | graded thunks and session-typed futures ([[#Futures and asynchronous modules]])       |
| module migration       | control migration with a mobile package ([[#Module migration]])                       |

The transparent-ascription half comes from a different line: **transparent existentials**, which lift through universals and arrows and thereby bring generative and applicative functors close together [@blaudeau-remy-radanne-2024-transparency].
That is [[#Transparent ascription and sealing]].

## The module grammar

```text
m ::= v                          value as module
    | { d₁, …, dₙ }              structure
    | m.l                        path selection
    | m : σ                      transparent ascription
    | m :> σ                     opaque ascription (sealing)
    | fun (x : σ₁) ⇒ m           functor (sugar for thunk_r (λx. ret m))
    | m₁ m₂                      functor application (sugar for force m₁ m₂ >>= …)
    | pack m : σ                 first-class package
    | unpack v : σ as x in m     dynamic unpacking
    | lazy m | spawn m           futures

d ::= val l = v | type l = A | module l = m | implicit val l = v

σ ::= { decl₁, …, declₙ }        signature
    | σ₁ → σ₂                    functor signature (elaborates to U_r (σ₁ → F σ₂))
    | ∀X. σ | ∃X. σ              polymorphic / abstract signature
    | @w σ                       world-located signature (the modal `at`, on the record)

decl ::= val l : A | type l : κ | type l = A | module l : σ | implicit val l : A
```

The typing judgment is `Γ; Δ; Θ; Σ ⊢_w m ⇕ σ`.
Its four context zones are the **core's** zones rather than module-specific ones — modules inherit the whole judgment, not a reduced one — and the two that matter below are the linear zone `Σ`, which holds obligations that must be used exactly once, and the world index `w`.
A module is not typed in a smaller context than an expression is.

**This grammar is not the surface.** The concrete surface for the built rung is `module M (: #{ … })? { def … }` ([[../declarations#module declarations]]); the notation above is the design record's abstract syntax, and the two are related by an elaboration that has not been written.
Fixing that relation is [[#module-question-01]], and it is the reason the signature family has not entered the grammar: the surface question of whether a signature is written `{ … }` or with a delimiting keyword pair is unresolved, and the grammar's own criteria — first-token discrimination, no unbounded lookahead ([[../../surface-language#The design stance]]) — will decide it.

## The typing rules

```text
Γ ⊢_w v ⇑ A
─────────────────────────── (Mod-Value)
Γ ⊢_w v ⇑ { val l : A }      (singleton module)

Γ ⊢_w m ⇑ { …, val l : A, … }
────────────────────────────── (Mod-Select)
Γ ⊢_w m.l ⇑ A

Γ ⊢_w m ⇑ σ′     σ′ <: σ
───────────────────────── (Mod-Transparent)
Γ ⊢_w (m : σ) ⇑ σ′ ⊓ σ      -- σ strengthened with m's type identities:
                            -- abstract `type l : κ` in σ becomes `type l = A`
                            -- whenever σ′ determines it

Γ ⊢_w m ⇓ σ     ᾱ = abstract types of σ     β̄ fresh
───────────────────────────────────────────────────── (Mod-Seal)
Γ ⊢_w (m :> σ) ⇑ ∃β̄. σ[β̄/ᾱ]                 (generative: fresh identities)

Γ, x:σ₁@w; Σ ⊢_w m ⇑ σ₂
───────────────────────────────────────── (Mod-Fun)
Γ; Σ ⊢_w fun (x:σ₁) ⇒ m ⇑ U_r (σ₁ → F σ₂)

Γ ⊢_w m₁ ⇑ U_r (σ₁ → F σ₂)     1 ⊑ r     Γ ⊢_w m₂ ⇓ σ₁
──────────────────────────────────────────────────────── (Mod-Apply)
Γ; · ⊢_w m₁ m₂ ⇑ F σ₂            (a computation — functor application can compute)
```

Two of these rules carry more weight than their size suggests.

**`Mod-Transparent` is a meet, not a coercion.** The result type is `σ′ ⊓ σ`: the ascribed signature _strengthened_ with the identities the module's own type determines, so an abstract `type l : κ` in `σ` becomes `type l = A` wherever `σ′` fixes `A`.
That is what makes transparent ascription an identity-preserving operation rather than an information-losing one.

**`Mod-Seal` is where module-level linearity earns its keep.** Sealing mints fresh existential identities, which is the generative reading.
If the sealed module governs a resource, give the package grade `1` and the type system enforces single instantiation — **by the grade, not by overloading "abstract" with "linear"**.
Two mechanisms that are usually tangled stay separate: abstraction hides a type, and a grade bounds a use count.

**`Mod-Apply` returns a computation.** Functor application is `F σ₂`, so applying a functor may _do_ something — run initialization, allocate, perform an effect.
A design in which functor application is a value operation has to either forbid that or smuggle it, and this one does neither.

**Two defects in the rules as recorded, carried rather than silently repaired.** The rules above are transferred from the design record, and two of them do not typecheck as written; naming them here is cheaper than letting an implementer discover them.

`Mod-Value` has the same term `v` synthesizing `A` in its premise and `{ val l : A }` in its conclusion, and the label `l` appears in the conclusion bound by nothing.
The intended reading is presumably that a bare value at a module position is coerced to a one-field structure whose label comes from the binding site, which means the rule is missing that label as an input.

The linear-zone bookkeeping is not uniform across the rules: `Mod-Fun` threads `Σ` through the functor body, `Mod-Apply` concludes under an **empty** zone `·` while its premises name none, and the remaining rules omit `Σ` entirely.
Since the whole point of the grade on a functor's thunk is to let module-level linearity be checked, the zone discipline is exactly the part that has to be stated precisely, and it is not.

_Disposition:_ **carried** — both are defects in the source, recorded at the claim and owed a repair when the rules are next stated, rather than settled here by guessing the intent.

### First-class packages

```text
Package σ  ≜  ∃β̄. U_r (F σ)      (r = ω by default; r = 1 for single-use packages)

Γ ⊢_w m ⇓ σ
────────────────────────────── (Mod-Pack)
Γ ⊢_w pack m : σ ⇑ Package σ

Γ ⊢_w v ⇑ Package σ′     runtime check: σ′ matches σ
Γ, x:σ@w; Σ ⊢_w m ⇕ σ″
────────────────────────────────────────────── (Mod-Unpack)
Γ; Σ ⊢_w unpack v : σ as x in m ⇕ σ″
```

**The package wraps a thunked _computation_, not a value.** `∃β̄. U_r (F σ)` puts the existential outside a thunk that _returns_ `σ`, so unpacking may run initialization effects before the module exists.
An encoding that packaged a bare `σ` would forbid that, and forbidding it is the kind of restriction nobody notices until a module needs to open a file.

**`unpack` is the one dynamic check in the whole design.** Everything else is static; here a runtime signature match stands between the package and the body.
The body's static type is governed by the _expected_ `σ`, and a mismatch raises **before** `m` runs, so a failed match is a boundary failure rather than a fault inside the module's own code.

**The word "package" carries a second, unrelated sense**, and the collision is worth naming once here rather than discovering it later.
`Package σ` is a _first-class module value_, packed and unpacked within a program.
The build and distribution unit — the addressable artifact a build resolves, fetches, verifies, and links — is a toolchain construct with its own design record, [[../../implementation/proposed/packages]].
Where this document says "package" unqualified it means the first-class module value.

## Implicit resolution

### Declaration and visibility

```text
implicit val ord : Ord Int        -- in a structure: visible where the structure is open
```

An implicit binding elaborates to a `U_ω`-graded thunk by default, or `U_1` when declared `implicit linear val`.

Visibility is world-scoped.
The implicit table `I` maps each world to its candidates, and the visible set at `w` is `scope(I, w) = { x ∈ I | loc(x) = w }`.

### Search

An implicit obligation is a constraint `⌈A⌉ @ w` in the solver's worklist.

```text
resolve(⌈A⌉, w, I, fuel) =
  if fuel = 0 → FAIL (FuelExhausted — report the obligation chain)
  candidates = { x ∈ scope(I, w) | x : U_r B with result type matching A }
  match candidates:
    []        → FAIL (no instance)
    [x]       → commit x; recursively resolve x's own implicit premises with fuel - 1
    [x, y, …] → FAIL (ambiguous — canonicity violation; report all candidates)
```

**Fuel is a solver parameter.** It has a default depth bound, it is reported in diagnostics, and it is not a coeffect of the program — the alternative reading is [[#module-decision-02]].

**Canonicity is coherence by uniqueness.** At most one candidate per type per world, in the manner of modular implicits, whose type-directed implicit module parameters elaborate into ordinary first-class functor applications [@white-bour-yallop-2015-modular-implicits].
Two candidates is a failure that reports _all_ of them, never a silent pick.

**The solver caches and backtracks together.** Commitments are cached keyed by `(A, w)`; a backtracking branch pops its cache entries with the solver trail, so a failed branch leaves no committed instance behind.

**Resolved implicits are used at their declared grade.** A `U_1` implicit used twice is a grade error at the use sites, found by the ordinary grade constraints — the implicit machinery adds no checking of its own.

### Cross-world resolution

Resolving `⌈A⌉ @ w₁` against a candidate at `w₂` requires two things: that `A` be **mobile** — that its values may travel between worlds, which is a judgment on the type, not a property of a particular value — and, if resolution must actually _fetch_ the witness, a capability for `w₂`, which elaborates to a migration.

Each world otherwise has its own implicit environment.
That is the payoff of scoping visibility by world rather than globally: **canonicity is per-world, so distributed instances raise no global coherence problem**.
Two worlds may each have their own canonical `Ord Int` without either being wrong.

## Worlds and distribution

### Located modules

```text
module Remote : @server { val process : U_ω (Request → F Response) }
```

A located signature is the modal `at` applied to the structure: **the record exists at `server`**, and a client holds a mobile handle rather than the record.
Selecting a field requires either being there — a migration — or the field itself being a mobile handle.

The world discipline this rests on is the type-safe distributed-programming design in which the whole distributed application is one program and a modal type system statically excludes unsafe uses of mobile resources [@murphy-crary-harper-2007-ml5].

### Serving a package over a shared session

Alice ML's ticket mechanism is **precisely a shared service** whose protocol serves packages.

```text
TicketProto σ  ≜  ↑ˢₗ !(Package σ). ↓ˢₗ (TicketProto σ)

offer  : Package σ → F (Ticket σ)      -- fork! a shared provider; ticket = shared name a
take   : Ticket σ → F (Package σ)      -- acquire a; recv the package; release
```

The two shifts are the modalities of manifest sharing, where a session type is stratified into a shared layer and a linear layer: `↑ˢₗ` is the acquire shift from shared to linear, and `↓ˢₗ` the release shift back [@balzer-pfenning-2017-manifest-sharing].
So the protocol reads: acquire the service, receive one package, release it back at the same protocol.

The elaborations are mechanical.
`offer m` becomes `fork! (a : TicketProto σ). loop (acquire-send-release)` and returns the shared name `a`; `take` becomes `acquire a as c; recv c as p; release c as a; ret p`.

**Multiple clients may `take` concurrently**, which is exactly what manifest sharing licenses.
The **equi-synchronizing** constraint — every release restores the session to the type it was acquired at — is what guarantees each client meets the service at its advertised protocol rather than at whatever state a previous client left.

Package payloads must be **mobile**, checked at the fork.
That is the static form of the pickling restriction Alice ML enforces dynamically, and moving it to the fork is a strict improvement: a payload that cannot travel is rejected where the service is defined, not where a client happens to be.

### Module migration

```text
migrate_server (unpack (take ticket) : σ as M in ret (hold M.result))
```

**There is no module-migration primitive, by construction.** Module _code_ moves by packaging — `pack` producing a mobile `Package σ` — and _control_ moves by migration, which is capability-gated and yields mobile results.
Composing the two is migration of a module, and the composition needs nothing new.

## Futures and asynchronous modules

| the Alice ML primitive | what it is here                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------ |
| `lazy m`               | `thunk_ω (eval m) : U_ω (F σ)` — forced on demand, memoized by the runtime                             |
| `spawn m`              | a session future: `fork (c : !σ.end). (eval m >>= x. send c x; close c; ret ())`, handle `c′ : ?σ.end` |
| `await f`              | `recv c′ as x; close c′; ret x` — a binary session receive _is_ data-flow synchronization              |
| `promise` / `fulfill`  | the same single-assignment channel, with the sender half handed out                                    |

**Strictness needs no side table.** A design that adds futures to a call-by-value language usually has to enumerate which operations are strict.
Here **computations are exactly the strict things**: forcing, application, and bind demand their inputs because they are computation forms, and values never do.
The enumeration is a consequence of the polarity rather than a table someone maintains.

A spawned module whose body runs at another world composes `spawn` with migration, and its payload `σ` must then be mobile.

## Transparent ascription and sealing

Opaque sealing is generative — fresh existentials — and transparent ascription preserves identities.
The bridge between them, and what makes _applicative_ functors expressible at all, is lifting existentials out of universals and arrows by **skolemization**:

```text
∀α. ∃β. σ(α, β)   ≅   ∃(b : κ_α → κ_β). ∀α. σ(α, b α)
```

**Read the dependency.** The abstract type `β` may genuinely depend on `α`, and it becomes a type _function_ `b`.
The weaker reading — the one that merely commutes the quantifiers, valid only when `β` does not depend on `α` — is a different and much smaller law, and it **excludes exactly the applicative-functor cases the feature exists for**.
A statement of this law that quietly assumes independence is therefore not a simplification; it is the wrong law.

One sealing construct plus the lifting law gives both functor flavours:

```text
(fun (x : σ₁) ⇒ m) :> (σ₁ → ∃β. σ₂)      -- generative: new β per application
∃b. (fun (x : σ₁) ⇒ m) : (σ₁ → σ₂[b x])  -- applicative: β a function of the argument
```

**This is the one place the kinding system genuinely needs higher kinds** `κ → κ′`, and it is why the core kinding rules — [[../../implementation/type-system#Kinds]] — carry a kind-application rule at all.
Every other use of higher kinds in the design is convenience; this one is load-bearing, because the skolem `b` has nowhere else to live.

## What the typing machine owes

The typing machine will need seven module frames, and none of them exists today.

| frame          | what it holds                                                          |
| -------------- | ---------------------------------------------------------------------- |
| `KModSelect`   | a pending path selection                                               |
| `KModSeal`     | a pending sealing, with the abstract-type renaming                     |
| `KModFunBody`  | a functor body under its parameter binding                             |
| `KModApply`    | a pending functor application                                          |
| `KModPack`     | a pending package construction                                         |
| `KModUnpack`   | a pending dynamic unpack, with the expected signature                  |
| `KImplicit`    | an implicit obligation and its **remaining fuel**                      |

Three notes on their behaviour, each of which is a constraint on the implementation rather than a description of one.

**`KImplicit` carries fuel, and nested resolution decrements it.** Implicit search participates in the solver trail like any other constraint: a failed branch pops the commitments it cached.

**Sealing records its renaming.** The frame pushes fresh existential identities and the derivation node records the `ᾱ ↦ β̄` map, so the derivation surface can answer "where was this abstract type born".

**There are no module-specific distribution frames.** The ticket protocol and the futures reuse the session and sharing frames, which is the machine-level statement of the elaboration discipline above: if distribution needed its own frames, the elaborations would not be elaborations.

### Derivation visualization

The design records four renderings, and they are requirements on the derivation surface, not decoration.

- **Ascription** — `σ′` against `σ` side by side, with the strengthened components highlighted, so a reader sees what the meet added.
- **Implicit resolution** — the search tree with the fuel remaining at each node, the committed candidate marked, and an ambiguity failure listing every candidate rather than the first two.
- **Sealing** — fresh-identity badges on the abstract types.
- **Distribution** — `offer` and `take` rendered as the underlying acquire, send, and release steps, with world badges.

## Worked examples

### A structure with implicits

```text
signature MONOID = sig
  type t : *
  implicit val unit_ : t
  implicit val op : U_ω (t × t → F t)
end

module IntAdd :> MONOID = struct
  type t = Int
  implicit val unit_ = 0
  implicit val op = thunk_ω (λp. split p as (a, b) in ret (a + b))
end

-- force (resolve op) applied twice; both obligations resolve to IntAdd.op
-- (unique candidate at this world → canonical)
combine = fun (M : MONOID) ⇒
  force ⌈op⌉ (force ⌈op⌉ (⌈unit_⌉, ⌈unit_⌉), ⌈unit_⌉)
```

### A distributed module service

```text
server_main = pack ComputeModule : COMPUTE >>= p.
              offer p >>= ticket.
              ret (hold ticket)                       -- @server (Ticket COMPUTE)

client (t : Ticket COMPUTE) =
  migrate_server (take t) >>= pkg.                    -- needs cap_server; Package mobile
  unpack pkg : COMPUTE as M in force M.apply 42
```

### Lazy and concurrent modules

```text
heavy   = thunk_ω (eval (compileHeavyLib ()))         -- U_ω (F HEAVY): lazy
use ()  = force heavy >>= H. force H.someFun ()

spawned = fork (c : !RESULT.end).
            (eval BuildModule >>= r. send c r; close c; ret ())
          in c′.  …  recv c′ as r; close c′; …        -- future + await
```

The examples use the design record's abstract syntax, including a `signature … sig … end` / `module … struct … end` spelling that the surface has **not** adopted — see [[#module-question-01]].
They are read as design, not as programs this tree accepts.

## Integration with the core features

| feature                      | how modules integrate                                                                                                          |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **core calculus**            | modules are values; functors are thunked computations `U_r (σ₁ → F σ₂)`                                                        |
| **unions and intersections** | `σ₁ ∪ σ₂` on signature _values_; intersections on functor _bodies_, giving overloaded functors — polarity-sorted like the core |
| **polymorphism**             | `∀X. σ` and `∃X. σ`; higher kinds `κ → κ′` for the skolem functions                                                            |
| **sessions**                 | futures and the ticket protocol are session elaborations, not new primitives                                                   |
| **sharing**                  | distribution tickets are shared channels; equi-synchronizing is service-protocol stability                                     |
| **grades**                   | functor applicability, package single-use, implicit usage                                                                      |
| **worlds**                   | located signatures `@w σ`; per-world implicit environments; capability-gated fetch                                             |

The union and intersection row is the one that is easy to misread.
Signatures are value types, so their union is a value-type union; a functor's _body_ is a computation, so an intersection over functors is an intersection of computation types.
The polarity sorting is inherited, not re-decided.

## The staging ladder

Six rungs, each with a stable identifier so it can be cited from a schedule.
Retiring a rung leaves its number unused.

### module-rung-01

**Checked record modules.** Structures as records, transparent record ascription, and path selection by projection, with source-ordered exactly-once member evaluation and the shared multi-item linker.

_Status:_ **landed** — this is the built rung described at the top of this document.

### module-rung-02

**Full structures and signatures.** Signatures as a sort, path selection as a module operation rather than a record projection, functors as thunked computations, and transparent ascription with the meet rule.

_Status:_ not started.
Gated on [[#module-question-01]], the signature surface.

### module-rung-03

**Sealing and first-class packages.** Generative opaque ascription, `pack` and `unpack` with the dynamic signature match, and the package existential over a thunked returner.

_Status:_ not started.
The kernel's export format already reserves the declaration kinds this needs — abstract types, module signatures and definitions, functors — so the rung is additive rather than a format change ([[../../implementation#Architectural commitments]]).

### module-rung-04

**Implicits.** World-scoped visibility, fuel-bounded search, canonicity, and graded usage.

_Status:_ not started.
Gated on [[#module-rung-02]] for signatures and on the solver trail for backtracking.

### module-rung-05

**Distribution.** The ticket protocol over shared sessions, located modules, and cross-world implicit resolution.

_Status:_ not started.
Gated on sessions, manifest sharing, and worlds, none of which is built ([[modes-and-references#The substrate, and what is actually built]]).

### module-rung-06

**Futures and transparent-existential lifting.** The `lazy`, `spawn`, and `await` elaborations, and the skolemization law that unifies applicative and generative functors.

_Status:_ not started.
The lifting half is gated on higher kinds in the kinding rules; the futures half on binary sessions.

## Open questions

### module-question-01

**What is a signature's surface spelling?**

The abstract syntax above writes a signature as a brace-delimited declaration list, and the worked examples write it with a keyword pair.
Neither is adopted, and the choice is the reason the signature family has not entered the grammar: a brace-delimited signature must be discriminable from a record type and from a block **on its first token in context**, with no unbounded lookahead and no declared conflict ([[../../surface-language#The design stance]]).

_Disposition:_ **carried** — open, and it blocks [[#module-rung-02]].
It is parked on the schedule as part of the full module family ([[../roadmap#Pending surface lanes]]).

### module-question-02

**Does inferred impredicativity ride the implicit-search fuel, or its own algorithm?**

[[#module-decision-01]] admits both: the implicit solver is already a fuelled semi-decision procedure, and a Quick-Look-style algorithm is the alternative with production evidence behind it [@serrano-hage-peytonjones-vytiniotis-2020-quick-look].
The two differ in what a failure looks like — a fuel-exhaustion diagnostic with an obligation chain, or an algorithm-specific rejection — and therefore in what the derivation surface can show.

_Disposition:_ **carried** — open, and it is downstream of [[#module-rung-04]], since only one of the two answers has a mechanism before implicits land.

### module-question-03

**Where does import resolution live relative to the trusted core?**

The design constraint recorded with the deferred typed-import work is that **resolution stays outside the trusted core**: a resolver supplies source and module data to the elaborator, and the elaborator checks the declared record signature.
Package identity, path and error diagnostics, and deterministic hermetic resolution are to be specified **before** any network or registry behaviour, and the acceptance instrument is a fixture resolver that imports a package as a typed module with no network access.

_Disposition:_ **carried** — the constraint is adopted and its answer is unwritten.
The toolchain half is [[../../implementation/proposed/packages#The import surface and its lowering]]; the language half is the ascription rule of [[#The typing rules]].

### module-question-04

**Does a module member take attributes?**

The grammar admits a leading `@[…]` block on a module member — `gandr-surface-grammar`'s `module_member` rule includes it — but the lowering collects attributes **only** for top-level items, so a member's attribute block parses and then vanishes: it is neither projected into the report nor diagnosed.
The module declaration itself takes no leading attribute block at all, which is why the manifest attribute schemas attach to a top-level definition today rather than to a unit root ([[../attributes#What is built]]).

_Disposition:_ **carried** — this is a gap in the built rung rather than a design question, and it is filed as a residual of this document's absorption.

## Source and confidence

This document is written against the pre-reboot module-system design record; the prior programme's tracker rows for the record-module rung and for the deferred typed-import work; and the tree itself — `gandr-surface-grammar`'s `surface::term` module for the grammar, `gandr-surface-engine`'s `lower`, `host`, and `ffi` modules for the lowering and the reserved namespaces, `gandr-core-checker`'s `machine` module for the frame inventory, and `crates/surface-corpus`'s module examples for the runnable behaviour.

**Confidence is high on the built account** — every as-built claim above names the module it was read from, and the module example was run rather than inspected.
**Confidence is medium on the design payload**, which is transferred from the design record and has had no independent pass.
**Confidence is medium on the literature attributions**: each carries a verified identifier, but the claims attributed to each work were checked against the design record's use of them rather than against the works themselves — with one exception marked at the claim, the distribution source, whose entry carries no stable numeric identifier because none was found.

Where the design record and the tree disagree, the tree wins on status and the record wins on payload, and both readings are stated at the claim.
