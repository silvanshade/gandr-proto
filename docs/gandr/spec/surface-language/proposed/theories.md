# Theories and extension

**Proposed.
No theory layer exists in this tree: no `theory` or `extend` declaration form, no refinement operator, no coercion preorder register, and no elaborator to host them.** What exists is the machinery this design reconciles with and rides: the module layer's record rung and its proposed full ladder ([[modules]]), the ordered, rollback-carrying declaration substrate of the incremental pipeline ([[../../implementation/incremental-pipeline]]), the `sign` block and its description table ([[../signatures]]), the `Model(S)` shape-signature design ([[../higher-cells]]), and the three-scheduler elaborator rule that owns where coercions may live ([[../../implementation/proposed/elaboration-schedulers]]).

A **theory** is a named telescope with three properties that make it more than a record type: **generative identity** (the name matters), **retroactive definitional extension** (`extend` grafts derived fields onto an existing theory, resolving everywhere the theory does), and **shallow refinement** (`T / { … }` instantiates fields by display-map surgery, never by nesting).
The mode of use the design is absorbed for: set up a theory whose fields are the assumptions of a complicated result, then **extend it by the result** — the theory of the Eckmann–Hilton situation, extended by the Eckmann–Hilton theorem, specializes to every concrete instance with no re-packaging [@sterling-2026-pterodactyl-worklog, trees 01HN–01HW].

**The pain point this dissolves is in this tree, and is named.** The project's Agda metatheory development carries the re-packaging pattern directly: `metatheory/src/Gandr/Graph.agda`'s private `𝕊` submodule re-exports the base toolkit `public` under renames (`⊥-elim` to `¡`, `proj₁` to `fst`, `_∘′_` to `_∘_`) and adds derived operations (`seq` diagrammatic, `!` the terminal map), under the comment "Repackaged rather than imported flat" — verified against the file at write time.
The pattern's costs are the renaming boilerplate, the re-export discipline, and the structural one: the "extended" module is a **new name**, so nothing downstream of the old name sees the new operations.
Under the theory layer the same move is `extend 𝕊 { … }`, and `seq` resolves on anything that _is_ a `𝕊`, by name lookup.

The absorption's centre of gravity is the reconciliation with the module layer.
Theories are convenient but **strictly less powerful** than first-class modules, and the module layer is load-bearing for the distribution story — worlds, sessions, packaging ([[modules#Worlds and distribution]]) — so the two must not merely coexist.
This document answers that requirement with a choice and a boundary, states the extension semantics against the declaration chain, keeps the two-former spelling ruling with its species distinction intact, and shows the theorem-as-extension mode of use elaborating.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **Module declarations lower to checked canonical records.** `gandr-surface-grammar`'s `surface::term` module contributes the `module_declaration` rule; `gandr-surface-engine`'s `lower` module lowers a body to one named item whose term is a canonical record (or a bind chain returning one), with source-ordered exactly-once member evaluation and a transparent, value-only record ascription ([[modules#What is built, and what this document describes]]).
* **The declaration substrate is ordered, incremental, and rollback-carrying.** A source file is a flat ordered list of top-level items ([[../../surface-language#Items: what a file is]]); the incremental pipeline checkpoints each item with a dependency footprint, revalidates rather than blindly reuses, and maintains item identity across edits by splicing an order-maintenance structure — `gandr-core-checker`'s `checkpoint`/`footprint`/`region` modules, `gandr-surface-engine`'s `checkpoint`/`footprint`/`edit` modules (twelve localized actions, including insert and delete on the item list), and `gandr-theory-orders` entire ([[../../implementation/incremental-pipeline#Granularity: the item and the node]]).
* **The `sign` block and its sort-indexed description table exist.** `gandr-theory-levitation`'s `SignDesc` is the declaration table — sorts, constructors, `opers`, rule faces — and the `sign` grammar and sign-to-description lowering are landed ([[../signatures#As-built rung and witnesses]]).
* **The identity former the law fields type by exists.** `gandr-core-checker` carries `ValueType::Path` with both endpoints, `Value::Here`, and `Comp::Walk` ([[../higher-cells#As-built impact]]).
* **The vocabulary is free.** `gandr-surface-parser`'s `mold.rs` `KEYWORDS` table reserves neither `theory`, `extend`, `include`, nor `renaming`.

**Landed as contract, machinery unbuilt.** The three-scheduler elaborator rule is this corpus's binding statement of where coercions live — elaborator-side, user-sited, never kernel — and it explicitly carries the coercion preorder's register to _this_ document ([[../../implementation/proposed/elaboration-schedulers#What this rule binds]]).

**Designed, and not built.** The `theory` and `extend` formers, the extension log, refinement and its side condition, the coercion preorder register, and everything this design rides above the record rung: signatures as a sort, functors, sealing, packages, implicit resolution ([[modules#The staging ladder]]), `Model(S)` ([[../higher-cells#Staging]], rung H2), and the elaborator itself — no metavariable, no `cast` form exists.

## The reconciliation with the module layer

The requirement, restated so this document stands alone: theories and modules must be **one mechanism, or genuinely complementary mechanisms with the boundary stated**; "two unrelated mechanisms" is a refused outcome.
Three candidate readings were evaluated in the following order.

| reading                                   | the claim                                                                                                                                                                                                                                                                                                                                                                                                     | what it costs                                                                                                                                                                                                                                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **(a) theories ride modules**             | a `theory` block elaborates to the module layer's record form; a theory is a named signature whose instances are ordinary module values; `extend` appends definitional fields                                                                                                                                                                                                                                 | says _where_ theories elaborate, not _how_ the two mechanisms relate; extension as signature mutation would act at a distance on already-minted instances                                                                                                                       |
| **(b) two access disciplines, one value** | under the 1ML unification gandr's module proposal already takes — a structure is a value, a functor a thunked computation ([[modules#The correspondence, and what the polarity forces]]) [@rossberg-2018-1ml] — theories and modules are two _disciplines over the same record values_: name-directed with retroactive extension for theories, type-directed with functors, sealing, and packages for modules | the extension log must mean something under functor application, sealing, and packaging — the boundary stated below                                                                                                                                                             |
| **(c) telescopes as named rows**          | a telescope is a named row with generative identity and an extension log; refinement is row surgery                                                                                                                                                                                                                                                                                                           | needs a row layer the surface type system does not have (the tree's rows are sealed effect rows, structural rather than generative — [[../../implementation/proposed/solver-interface#The domains that already exist]]), and generative _names_ are the part row systems refuse |

### theory-decision-01

**Reading (b) is adopted: theories are the name-directed access discipline over the same values the type-directed module machine drives — and reading (a) is retained as its elaboration half.** A theory declaration elaborates to two things: a signature value in the module grammar's σ sort — record-shaped, and `Model(S)`-shaped for law-carrying theories ([[#The Model(S) correspondence]]) — and an entry in the name-directed registry (the extension log and the coercion preorder, both elaboration metadata).

The argument, in four steps.

1. **It is the reading the module layer already takes.** The module proposal's unification thesis is 1ML's: core and modules are one language, structures are records, functors are thunked computations ([[modules#The unification thesis]]).
   Under that unification a theory cannot be a new _kind_ of value without contradicting it; what remains for a theory to be is a _discipline_ over the values that exist.
2. **Generativity is a property of the discipline, not of the values.** The name matters because resolution keys on it — not because a theory instance is anything but an ordinary module value.
   This keeps the module layer's semantics untouched: no second value notion, no amended typing rule.
3. **Retroactivity becomes ordinary ascription.** An extension adds no data, so an instance minted before the extension already satisfies the extended telescope, and ascribing it to the extended theory elaborates the extension's definitions against it ([[#Why retroactivity typechecks]]).
   Reading (a) alone cannot explain this — a signature mutation would have to reach already-minted instances; under (b) nothing reaches anything, because the extension log is consulted at ascription and resolution sites, which are elaboration-time events.
4. **The disciplines partition by direction of information.** The module machine is type-directed: types drive resolution (implicit search), packaging (dynamic signature match), and abstraction (sealing).
   The theory layer is name-directed: names drive resolution, extension, and refinement.
   Each is weak exactly where the other is strong — a name-directed layer cannot package, a type-directed layer cannot extend retroactively — which is what "genuinely complementary" means here.

_What would change this ruling:_ a demonstrated **non-definitional** extension — one whose fields carry new data an old instance does not already determine — collapses (b)'s ascription mechanics and forces a per-instance extension store, at which point the design re-evaluates against (a) with real machinery.
And if sealing or packaging demonstrably needs name-directed penetration — a consumer that must resolve theory operations through a sealed boundary — the boundary below moves rather than the reading.
Revisit cost: the boundary is stated as a reading of the module layer's existing rules, not an amendment to them, so re-choosing re-reads this document, not the module design.

_Disposition of the other readings:_ (a) is absorbed as the elaboration half of the choice, not declined. (c) is **parked** — its reversal condition is the row layer landing in the surface type system (unions and intersections are an integration row of the module design, [[modules#Integration with the core features]]); recorded as [[#theory-question-04]].

### The boundary: extension across sealing and packaging

Under reading (b) the one thing owed is what the extension log means where names stop being available.
The boundary is five statements, each a reading of an existing module rule rather than a change to one.

* **Extension is elaboration metadata attached to a theory name.** Nothing crosses into the kernel, which receives fully elaborated terms — the same posture as the coercion boundary ([[../../implementation/proposed/elaboration-schedulers#What the kernel never sees]]).
* **Transparent ascription preserves membership.** `Mod-Transparent` is a meet: the ascribed signature is _strengthened_ with the identities the module's own type determines, and fields survive ([[modules#The typing rules]]).
  Membership in a theory is recorded at ascription to its signature, and an extension visible at the ascription site elaborates against the surviving fields — so extensions ride through `:` unchanged.
* **Sealing preserves an extension's applicability exactly for the fields the seal exposes.** `Mod-Seal` mints fresh existential identities for the target's abstract types ([[modules#The typing rules]]).
  An extension's fields are definitional over the telescope, so they still elaborate against a sealed module whenever the seal's target signature exposes the telescope's operations — abstract types stay abstract and the definitions do not care.
  A seal that _hides_ a field an extension's definitions mention turns the extension's later use sites into ordinary typing errors at the site, never silent misresolution: hiding is the user saying the name-directed layer may not look here.
* **Packaging forgets; unpack re-establishes.** A `Package σ` is an existential over a thunked returner, and `unpack v : σ as x` is the module design's one dynamic check ([[modules#First-class packages]]).
  Name-directed resolution is static, so it never reaches through a package; the unpacker's expected σ is where theory membership is re-asserted, and the runtime signature match is what backs it.
* **Functor application commutes with extension.** Extension happens at a declaration-chain position, before and independently of instantiation; a functor body elaborated against an extended theory applies to any instance of the theory, because the extension's definitions elaborate against the argument at each application exactly as they did at the definition site.
  The log is per-name, never per-application — this is what keeps the two disciplines from diverging on the same value.

Worlds add nothing to the boundary: theory membership is a static ascription fact, implicit resolution is already per-world with canonicity local to each world ([[modules#Cross-world resolution]]), and a located module's theory membership travels exactly as its signature does.

## The two formers, and the species distinction

### theory-rule-01

**Theory declarations are generative, and resolution is dictionary lookup by name — never higher-order unification.** Two theory declarations with identical fields are different theories, because extensions, refinements, and coercion edges hang off the name.
Unification remains for implicit arguments, under the unifier's own contract ([[../../implementation/proposed/elaboration-schedulers#sched-rule-02]]); it no longer steers structure resolution.
A future proposal that routes structure resolution through the unifier is refused in advance: the scheduling argument — a combined unification–coercion solver is order-sensitive by construction, a property of the mathematics rather than of any implementation — is the standing refutation, recorded with no reopening delta at [[../../implementation/proposed/elaboration-schedulers#The scheduling argument]].

### theory-rule-02

**Two formers, because two categorical species.** `theory`/`extend` names the telescope layer; `sign` stays for cellular presentations (sorts, constructors, operations, rules — [[../signatures#The rulings]]).
The spelling is the owner's ruling of 2026-08-06, and the words are the cheapest place the distinction lives; the one-word alternative was weighed and declined for blurring exactly it.

The distinction the spelling guards:

|                 | telescope extension (`extend`)                                          | signature growth (polygraph inclusion)                                                                                              |
| --------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| adds            | definitional fields over a fixed structure                              | sorts, constructors, operations, rules                                                                                              |
| semantically    | the extended telescope displays over the same carrier; models untouched | the presented object changes — a different initial algebra, a different polygraph                                                   |
| the induced map | refinements are display maps                                            | models of the extended signature _restrict_ to models of the old one                                                                |
| spelled         | `extend T { … }`                                                        | a larger `sign` block; inclusion is gandr's native operation ([[../metatheory#Cellular data — descriptions, cells, and computads]]) |

The rule layer sharpens the point.
Extending a signature by _rules_ changes the congruence, and the certificate discipline already distinguishes the two ways that can happen: a rule _declared_ — a new axiom, a genuinely different theory — from a rule _certified derivable_ by completion, the same theory with its cell discharged ([[../higher-cells#Discharge by completion]]).
An `extend` whose fields are derived operations is exactly a certified extension: every field's definition is a replayable certificate, which is this project's ordinary currency ([[../../metatheory#The certificate algebra]]).

**Higher-cells safety follows from content addressing.** Extension never removes or identifies cells — it is inclusion plus definitional fields — so every certificate, coherence cell, and content hash minted against the unextended theory replays unchanged.
The cellular layer is what makes the species distinction visible at all: a `sign` block presents a computad, whose models are a different notion from a record's inhabitants, so the two readings of "extension" that a pure-telescope system must disambiguate by convention are here different constructions.

## The surface, sketched

Bracket-based, keyword-led blocks — the ruled spelling posture for every sketch in this document, satisfying first-token discrimination and no layout significance by construction ([[../../surface-language#The design stance]]).
The universe keyword is `Type` (the vocabulary decisions of record, [[../../implementation#The surface pipeline]]), and laws are `Path`-typed fields, anticipating [[#The Model(S) correspondence]].

```text
theory Semigroup {
  car : Type
  mul : car -> car -> car
  assoc : (x y z : car) -> Path(car, mul(mul(x, y), z), mul(x, mul(y, z)))
}

theory Monoid {
  include Semigroup
  one : car
  leftUnit : (x : car) -> Path(car, mul(one, x), x)
}

extend Semigroup {
  square : car -> car
  square(x) := mul(x, x)
}
```

* An extension field is a **signature–definition pair**: the field's type, then its definition over the existing fields.
  The definition is what makes ascription retroactive — it is the certificate the ascription line replays.
* `include` merges a telescope's fields, with `renaming` for flat hierarchies ([[#Worked examples]]); `/ { … }` is shallow refinement ([[#Refinement]]).
* These sketches are design, not programs this tree accepts — the same register as the module design's abstract syntax ([[modules#The module grammar]]).
  The grammar slots and keyword reservations are carried as [[#theory-question-01]].

## Extension against the declaration chain

Retroactive extension presumes **ordered declarations**: a language whose definitions are an unordered global scope cannot say what `extend` means — extension at _which_ point, visible _where_?
gandr already gave up that model: its declaration discipline is an ordered item list with validated reuse and identity-stable splicing ([[../../implementation/incremental-pipeline#Derivation merging and identity stability]]), so "global" never described its scope, and the extension semantics is three readings of machinery that exists.

### theory-rule-03

**Position.** `extend T { … }` at declaration-chain position $n$ appends definitional fields to the telescope named $T$, visible at positions after $n$.
Name resolution at position $m$ sees exactly the extensions at positions before $m$, in chain order — which extension supplies `square` is a function of position, never of search.

### theory-rule-04

**Rollback.** Rollback across position $n$ removes the extension's fields.
An edit that deletes or moves the `extend` item invalidates exactly the items whose dependency footprints read it, by the pipeline's ordinary invalidation — revalidation, never blind reuse ([[../../implementation/incremental-pipeline#Checkpoints and the reuse rule]]) — so no stale resolution survives an edit, and the extension semantics adds no invalidation rule of its own.

### theory-rule-05

**Branch merge.** Two branches that extend the same theory are two chains.
The merge is **additive** when the extension names are disjoint; a content-identical extension on both branches merges to one field (extensions are content-addressed data, so identical content is one extension, not a duplicate); and two extensions of the same theory introducing the same field name with different content are **rejected as a conflict** — never a silent pick, the same posture as per-world canonicity in instance resolution ([[modules#Implicit resolution]]).

### Why retroactivity typechecks

The ascription line is the whole mechanics of retroactivity, and it deserves its own statement.

**An extension adds no data, so an instance minted before the extension already satisfies the extended telescope.** Ascribing the instance to the extended theory elaborates the extension's _definitions_ against the instance's existing fields — the module layer's ordinary transparent ascription, a meet that strengthens rather than hides ([[modules#The typing rules]]).
No new fields are stored, no instance is revisited, no migration runs.
What an extension changes is purely what _resolves_: after position $n$, the name-directed lookup of an extension field succeeds against anything that is a $T$ — an instance, an including theory, a refinement — because each of those ascription paths elaborates the definition.

## Refinement

### theory-rule-06

**Refinement is shallow, and a refinement is a display map.** `T / {ℓ => v}` is another theory, obtained by **field removal plus substitution**: remove the refined fields from the telescope and substitute their values through the remainder.
Nothing nests; two structures on one carrier need no ceremony because the specialized theory is still small.
A telescope is a context, and instantiating prefix fields is a display map — the telescope-as-schema reading the signature literature makes precise, where a context encodes a signature by listing its constructors [@kaposi-2020-signatures].

The instances compose: `Monoid / {car => Nat}` is telescope surgery on the signature, and an instance of the refined theory is a module ascribed to the strengthened signature — the module layer's ordinary meet, one mechanism wearing its third surface.

### theory-rule-07

**Mid-telescope refinement implicitly abstracts the skipped prefix, under a syntactic display-map side condition.** A refinement may start mid-telescope — `Monoid / {one => 0}` refines only `one`, leaving `car`, `mul`, and the laws as implicitly abstracted prefix fields, so the result is well-formed by construction.
The side condition refuses **diagonal** refinements syntactically, before typing: each instantiated value may depend only on fields that survive the refinement — the skipped prefix — never on a removed field, a later field, or the field under instantiation.
That is precisely the condition that the refinement read as a context map is a display map; a diagonal value is one whose dependencies cross the removal, and the check is a syntactic free-variable read.
The precise syntactic form of the check is this document's formalization of the design record's condition and is marked at that confidence in [[#Source and confidence]].

### theory-decline-01

**Deep refinement is declined.** Hierarchies are flat in the Mathematical Components style, with `include … renaming` instead of nesting, which makes the deep case unreachable rather than forbidden [@sterling-2026-pterodactyl-worklog, tree 01HV].

_Reversal condition:_ a demonstrated development whose natural form is deep refinement.
No candidate currently supplies one, and the labelled-preorder coercion register below is the mechanism that has historically absorbed the pressure.

## The Model(S) correspondence

The higher-cells design carries a third former, and the coherent story across all three is one sentence each — **`sign` presents, `Model(S)` structures, `theory` organizes**.

* **`sign` presents.** A cellular theory: sorts, operations, rules.
  Growth is inclusion of presentations — model-restricting — and identity is content-addressed ([[../signatures]]).
* **`Model(S)` structures.** The elaboration-computed signature of a shape's models: sorts as type members, operations as `val` members, rules as `Path`-typed law fields, 3-cells as iterated-`Path` coherence fields ([[../higher-cells#The Model(S) signature-former]]).
  Instances are ordinary modules ascribed to `Model(S)`: shape signatures extend _what a signature can say_, never _what a module is_ ([[../higher-cells#Programming with shapes]]).
* **`theory` organizes.** The name-directed, generative, retroactively extensible discipline over the same record values.
  **A theory block over algebraic structure elaborates to a `Model(S)`-shaped signature**, and its law-carrying instances are exactly `Model(S)` instances — proof-relevant laws and coherences as typed fields rather than comments, so two instances with equal operations but different coherence cells are different instances.

The correspondence is mechanical, shown on the weak monoid — the higher-cells near-term flagship ([[../higher-cells#The flagship examples]]), whose `sign MonoidShape` declares sorts `M`, operations `unit`/`mul`, rules `unitL`/`unitR`/`assoc`, and the `triangle`/`pentagon` 3-cells:

| `sign MonoidShape` (presents) | `Model(MonoidShape)` (structures)   | `theory Monoid` (organizes)        |
| ----------------------------- | ----------------------------------- | ---------------------------------- |
| `sort M`                      | `type M : Type`                     | field `M : Type`                   |
| `oper unit`                   | `val unit`                          | field `unit`                       |
| `rule unitL`                  | `val unitL`, `Path`-typed           | field `unitL`, `Path`-typed        |
| `rule triangle` (3-cell)      | `val triangle`, iterated `Path`     | coherence field, iterated `Path`   |
| growth by inclusion           | ascription and strengthening        | `extend` and `/ { … }`             |

The fit is load-bearing in both directions.
Downward: `Model(S)` gives `theory` its structure types for the law-carrying fragment, so laws are typed fields rather than comments — and the higher-cells open question of shape identity (carried there; the nominal reading is what an implicit-resolution key wants) aligns with [[#theory-rule-01]]'s generativity: two votes for names-matter from independent directions.
Upward: `extend` explains itself in `Model(S)` terms — an extension's fields are definitional, so an existing `Model(S)` instance already satisfies the extended telescope, and ascription elaborates the extension's definitions against it ([[#Why retroactivity typechecks]]).
Ascribing a pre-extension instance `natAdd : Model(MonoidShape)` to the _extended_ `Monoid` stores nothing new; `square` then resolves on `natAdd`, on any `Monoid`, and on `Monoid / {M => Nat}`, by name, without unification.

**The weak category's staging is unchanged under all three readings.** `CatShape`'s indexed sort `Hom(dom, cod)` needs a type family — `type Hom : Ob -> Ob -> Type` in `Model(CatShape)` — which is exactly the stage-1 dependent-era gate `Model(S)` already carries (Π, universe formation, compound `Path` endpoints; [[../higher-cells#Staging]], rungs H2–H3).
Under (a) the theory elaborates to the same gated signature; under the adopted (b) the name-directed discipline is orthogonal to the signature's formation rules; under the parked (c) a row representation would change the telescope's encoding, not its type-former needs.
The telescope discipline adds no expressive power and removes no gate — which is what "organizes, never structures" means, and is itself evidence the reconciliation is about discipline rather than power.

**The directed-models seam is inherited, not answered.** `Model` currently interprets every rule under the invertible overlay — `Path` is groupoidal; directed models are parked on the directed family ([[../higher-cells#Open questions, dispositioned]], its question 4).
A theory whose laws need directed readings inherits that parking exactly, and this document records the seam without scoping it ([[#Open dispositions]]).

## Coercions between refinements

Theories raise the question of when one structure _is_ another — `Monoid` to `Semigroup`, refined to unrefined — and this document owns the answer's register, handed off from the three-scheduler rule ([[../../implementation/proposed/elaboration-schedulers#Open dispositions]]).

### theory-rule-08

**The coercion register is a labelled preorder over theory names, elaborator-side, firing only at marked sites.**

* **Edges are built from theory names only** — `include`, refinement, and extension generate the edges; nothing is inferred from structure.
  This is Sakaguchi's refinement-and-extension discipline: coherence checked at edge _insertion_, expensive to install and a lookup to resolve, with incoherent backtracking resolution ruled out from the start [@sakaguchi-2023-refinement-extension].
* **Canonical maps between refinements are computed as syntactic constructors**, lifting through Π and Σ, **never through positive types** — the same shape as the polarity discipline, that introduction forms do not silently convert, and a boundary the three-scheduler rule records as machinery rather than convention ([[../../implementation/proposed/elaboration-schedulers#What the kernel never sees]]).
* **Selfification is handled explicitly**: a theory's edge to itself is the identity, installed rather than derived.
* **Link edges allow local coherence declarations** — edges added _locally_, so the preorder need not be fixed in advance — the modular-type-classes mechanism [@dreyer-2007-modular-type-classes]; **cycles are tolerated**, which is how Isabelle's locale graph in fact behaves.
* **The diamond discipline is this corpus's canonicity posture**, not a new rule: an ambiguous coercion — two candidates between the same refinements — is a reported failure naming all candidates, never a silent pick ([[modules#Implicit resolution]], [[../../implementation/proposed/elaboration-schedulers#sched-rule-03]]).
  The two diamond problems the design record separates [@sterling-2026-pterodactyl-worklog, tree 01HD] are **carried** under this discipline: the register makes ambiguity detectable at insertion, and the firing rule below makes it reportable at the site; whether that discharge suffices for the harder diamond shapes is owed to the register's first real hierarchy, not assumed here.
* **Firing is the three-scheduler rule's business**: a coercion fires at an explicit `cast` site or inside an explicitly ascribed refinement, and nowhere else — everywhere else it is an error at the site ([[../../implementation/proposed/elaboration-schedulers#sched-rule-04]]).
  The preorder is elaboration metadata with coherence certificates checked at edge insertion, a decide-tier operation.

**The candidate implementation reading is the cells facet** recorded by the three-scheduler document: declared coercions carried by the rewriting-cell machinery — the coercion graph is a graph the cell layer already tracks, and coherence checking over it is the same shape as completion ([[../../implementation/proposed/elaboration-schedulers#The cells facet for declared coercions]]).
That facet is a candidate, not a contract: what this document binds is the firing discipline and the register's shape, not the resolver's internals.

## The Eckmann–Hilton mode of use

The mode of use the whole design is absorbed for: **assumptions as a theory, the theorem as an extension**.

```text
theory EckmannHilton {
  X : Type
  M : Monoid / {car => X}
  N : Monoid / {car => X}
  dist : (u v w x : X) ->
    Path(X, M.mul(N.mul(u, v), N.mul(w, x)), N.mul(M.mul(u, w), M.mul(v, x)))
}

extend EckmannHilton {
  coincide : (u v : X) -> Path(X, M.mul(u, v), N.mul(u, v))
  coincide(u, v) := {- the Eckmann–Hilton argument, over the fields -}
}
```

The elaboration, step by step, each step one already-owned mechanism:

1. **The theory mints a generative name** with a telescope of two refined-`Monoid` fields and a distributivity law.
   Each `Monoid / {car => X}` is a legal refinement under [[#theory-rule-07]]: `X` is an earlier field of the same telescope, a surviving prefix variable, so each refinement is a display map.
2. **The extension appends a definitional field.** `coincide` is a `Path`-value _defined_ over the existing fields — the proof term consumes `dist`, the unit laws, and associativity.
   Under [[#theory-rule-03]] it is visible after its chain position; under [[#theory-rule-04]] an edit removing it removes exactly its downstream readers.
3. **Every instance already satisfies the extended telescope.** An `EckmannHilton` instance — two concrete monoids on one carrier with the distributivity witness — minted _before_ the extension ascribes to the extended theory by elaborating `coincide`'s definition against its fields ([[#Why retroactivity typechecks]]).
   The theorem was already true of the instance; the extension makes it _resolvable_ on it, by name.
4. **Specialization needs no ceremony.** `EckmannHilton / {X => Nat}` with `M`, `N` the addition and multiplication monoids yields the theorem for them by refinement and ascription alone — no re-packaged module, no renaming list, no new name that downstream code fails to see.

Nothing in the walkthrough is a fourth mechanism: telescope surgery, definitional ascription, and name lookup are the three moves, and each is owned by the module layer, the declaration chain, or this document's register.

## Worked examples

### The algebraic tower, flat

```text
theory AbGroup {
  car : Type
  join : car -> car -> car
  unit : car
  inv : car -> car
  -- laws as Path-typed fields
}

theory Ring {
  car : Type
  include AbGroup / {car => car} renaming (join => add, unit => zero, inv => neg)
  include Monoid / {car => car}
  distLeft : (x y z : car) -> Path(car, mul(x, add(y, z)), add(mul(x, y), mul(x, z)))
  distRight : (x y z : car) -> Path(car, mul(add(x, y), z), add(mul(x, z), mul(y, z)))
}
```

The hierarchy is flat, with inclusion and renaming instead of nesting — the discipline [[#theory-decline-01]] ratifies.
The refinement `car => car` on an `include` identifies the included theory's carrier with the including theory's, so both inclusions land on one carrier.

### Extension across inclusion and refinement

```text
extend Semigroup {
  square : car -> car
  square(x) := mul(x, x)
}

-- `square` now resolves on any Semigroup, on any Monoid (by inclusion),
-- and on Monoid / {car => Nat} (by refinement) — by name, no unification.
```

### Mid-telescope refinement

```text
Monoid / {one => 0}
-- refining only `one`: the skipped prefix (car, mul, the laws) is implicitly
-- abstracted; the display-map side condition passes because 0 mentions no
-- removed field — a theory of monoids pointed at zero.
```

## What this document binds

Stated as forward constraints on the designs that consume this one.

* **The grammar lane** owes the keyword reservations and grammar slots for `theory`, `extend`, `include`, and `renaming`; the blocks are keyword-led and bracket-delimited, so first-token discrimination and the no-layout posture hold by construction, and the reservation is a table change under the keyword-and-operator budget ([[#theory-question-01]]).
* **The eliminator-elaboration design** reads its assumption telescopes here: a theory's telescope is exactly the assumption list an eliminator-shaped constant quantifies over, and its resolution rule is [[#theory-rule-01]] — name lookup, never the unifier.
* **The module layer is unamended.** The boundary section is a reading of `Mod-Transparent`, `Mod-Seal`, `Mod-Pack`, and `Mod-Apply` as they stand; a future module-rungs change that alters those rules re-reads this document, and this document never re-reads the module rules.
* **The three-scheduler rule's carried register is discharged here.** The coercion preorder's shape, insert-time coherence, and firing discipline are [[#theory-rule-08]]; the `cast` keyword's spelling stays parked where the three-scheduler document parked it.

## Open dispositions

### theory-question-01

**The grammar slots and keyword reservations for the two formers.** The vocabulary is verified free (`gandr-surface-parser`'s `mold.rs`), and the blocks satisfy the design stance by construction; what is owed is the reservation pass and the slots, taken with the declaration-forms surface pass.

_Disposition:_ **carried.**

### theory-question-02

**The extension log's representation.** The log is elaboration metadata: a per-name, position-ordered list of definitional field blocks.
Whether it rides the cells facet — the candidate carrier for declared coercions, and the same candidate shape for extension blocks as certified additions — or a bespoke registry beside it, is unchosen.

_Disposition:_ **carried**; it moves with the cast resolver's mechanism, which the three-scheduler document carries as a candidate rather than a contract.

### theory-question-03

**Whether theory membership enters the kernel's export format.** The format already reserves the declaration kinds the module rungs need — abstract types, module signatures and definitions, functors ([[modules#module-rung-03]]) — so a theory-membership kind is additive if wanted; the alternative keeps membership purely elaboration-side, which the boundary section already suffices for.

_Disposition:_ **carried**, and cheap exactly because the boundary never needs the kernel to know.

### theory-question-04

**The row-theoretic reading.** Parked by [[#theory-decision-01]]: a telescope as a named row with generative identity and an extension log, refinement as row surgery.

_Disposition:_ **parked**, with the reversal condition that rows, unions, and intersections land in the surface type system — at which point generative naming, the part row systems refuse, is the sub-question to answer first.

### Collected declines and parkings

* **Deep refinement** — declined, reversal condition at [[#theory-decline-01]].
* **Unification-driven structure resolution** — refused in advance by [[#theory-rule-01]]; the standing refutation is the scheduling argument, recorded with no reopening delta by the three-scheduler document.
* **Coercions through positive types** — declined as machinery, inherited from the three-scheduler rule's kernel boundary ([[#theory-rule-08]]).
* **Directed models for theory laws** — parked on the directed family, inherited from the higher-cells design's own parking; this document records the seam and scopes nothing ([[#The Model(S) correspondence]]).

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **design record for the absorption**: the Pterodactyl worklog's theory design — labelled preorders (tree 01HB), the two diamond problems (tree 01HD), coercions between refinements (trees 01HN–01HS), specifying refinements including the deep-refinement refusal and mid-telescope refinement (trees 01HU–01HW) — the talk-demonstrated mode of use, and the owner's rulings of 2026-08-06 (the reconciliation requirement, the two-former spelling, the bracket-based posture), all restated here in full so the document stands alone [@sterling-2026-pterodactyl-worklog, trees 01HB–01HW].
2. **The tree**, for every as-built claim: `gandr-surface-grammar`'s `surface::term` and `gandr-surface-engine`'s `lower` (the record rung), `gandr-surface-engine`'s `edit`/`checkpoint`/`footprint` and `gandr-core-checker`'s `checkpoint`/`footprint`/`region` with `gandr-theory-orders` (the declaration substrate), `gandr-theory-levitation`'s `SignDesc` (the description table), `gandr-core-checker`'s `Path` former (law-field typing), `gandr-surface-parser`'s `mold.rs` `KEYWORDS` (the free vocabulary), and `metatheory/src/Gandr/Graph.agda`'s private `𝕊` submodule (the re-packaging pattern, verified line by line at write time).
3. The **corpus documents this one reconciles with and consumes** — the module system, the higher-cells design, the signature unification, the incremental pipeline, and the three-scheduler rule — each linked at the claim rather than restated.
4. The **published literature**: 1ML for the unification the choice rests on [@rossberg-2018-1ml], modular implicits for the canonicity posture the diamond discipline inherits [@white-bour-yallop-2015-modular-implicits], Sakaguchi's thesis for the labelled-preorder register [@sakaguchi-2023-refinement-extension], modular type classes for link edges [@dreyer-2007-modular-type-classes], and the theory of signatures for the telescope-as-schema reading of refinement [@kaposi-2020-signatures].

**Confidence, by class.**

* **High** — the as-built statements, each verified against the named module or file at write time; and the owner rulings (reconciliation requirement, two-former spelling, bracket posture), which are transcribed rather than derived.
* **High** — the extension semantics against the declaration chain ([[#theory-rule-03]]–[[#theory-rule-05]]) and the ascription line, which are readings of built machinery and of the module layer's stated rules rather than new mechanisms.
* **Medium** — the reconciliation choice itself: [[#theory-decision-01]] argues a ruling where the design record carried a lean, and its reversal conditions are stated with it.
* **Medium** — the labelled-preorder register's account of Sakaguchi's thesis, taken from the design record's use rather than from a fresh read of the thesis body; the register entry is held metadata.
* **Marked at the claim** — the display-map side condition's precise syntactic form ([[#theory-rule-07]]): the design record states the condition's existence and its purpose (refusing diagonal refinements); the free-variable check stated here is this document's formalization.
