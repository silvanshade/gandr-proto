# Higher cells — named 2-cells, declared 3-cells, and shape signatures

The dimension-named signature members in full, and the derived signature-former they exist to feed.
A `sign` block **presents a theory**; `Model(S)` is what any interpretation of that theory must supply — the laws as named 2-cells, the coherences between laws as named 3-cells, carried as typed fields rather than as comments.
The canonical block form and its member spellings are ruled at [[circuit-cells#The block form, ruled]] and the sugar relationship to `data`/`codata` blocks at [[signatures]]; the members and reserved slots as landed today are [[declarations#data declarations]]; the grammar machinery every form here must clear is [[grammar]]; the carrier-side rulings this surface rests on are the [[../metatheory#Cellular data — descriptions, cells, and computads|metatheory track's cellular-data section]].

- Status: **design pass, imported; nothing of this lane built.** The naming and declaration layers spend zero frozen core; `Model(S)` rides the stage-1 dependent era and is designed, not scoped, until then.
  One neighbouring piece is landed and is marked where it appears: the declaration table checks a derived boundary pair against the sphere its declaration fixes ([[#Sphere-typed boundaries]]), for members no surface can yet write.
- Everything user-facing here is new design surface. 3-cells exist in gandr today **only** as machine-derived confluence certificates — anonymous, replayed, machine-audience.
  [[#As-built impact]] states exactly what the tree does and does not carry, verified against it.
- Code sketches are surface-shaped pseudocode; the keyword table is a one-table change under the reservation policy of [[grammar#The keyword and operator tables]].

## Why the machinery exists

The customer, stated first because every section below serves it:

- A **shape** is a `sign` block read as a presentation: sorts, operations, laws, and coherences between laws.
- A **model** is a module ascribed to `Model(S)`: it supplies a carrier, the operations, a proof-relevant witness per law, and a proof-relevant witness per coherence.
- The pentagon is therefore written **once**, in the shape, and is thereafter an obligation on every interpretation — and, for the free model of a convergent presentation, one the machine discharges from its own completion certificate.

| what a Haskell class gives you      | what `Model(S)` gives you                                                                     |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| operations, with laws in comments   | operations, with laws as `Path`-typed fields                                                  |
| no notion of coherence between laws | coherences as iterated-`Path` fields, replay-validated                                        |
| instance equality is nominal        | two instances with equal operations and different coherence cells are **different instances** |
| law use is free and unaudited       | transporting along a coherence pays its fuel bill like any certificate replay                 |

This is the tagless-final reconciliation [@carette-kiselyov-shan-2009-tagless] applied to a rewriting presentation: **the block is syntax; the record is an interpreter.** The strictly stronger property is that laws and coherences are part of the semantics rather than of the documentation.

## Why 3-cells had no user story, and what changed

**Before.** The data layer's adopted theory is oriented _convergent_ presentations — the Squier-degenerate slice [@squier-1987-word-problems] [@squier-otto-kobayashi-1994-finiteness] where higher coherence is automatic and coherence computes.
In that slice a 3-cell is never something a user states: it is the machine's witness that a critical pair rejoins.

**What changed** — three things, jointly.

- **Identity landed.** `Path`/`here`/`walk` are as-built and without-K [@cockx-devriese-piessens-2014-without-k], so the tower does not collapse and `Path(Path(A, x, y), p, q)` is a type.
  Iterated `Path` types are exactly where user-stated 3-cells land.
- **Carried coherence became the house idiom.** Certificates are proof-relevant because two distinct witnesses are two distinct artifacts.
  One dimension up, the same argument says the pentagon of a weak structure is _data a program may consume_, not a proposition to discharge.
- **The interface customer appeared.** With first-class modules [@rossberg-2018-1ml] a signature can characterize _structure_ — "a monoid on `M`", "a category".
  Laws-as-fields at dimension 2 immediately raises the dimension-3 question, what relates the laws, and that is the first place a user-written 3-cell is load-bearing.

**The gap this fills in the sibling library's own encoding.** The internal-univalence library's `Category` record deliberately carries **no** pentagon or triangle field.
Its coherence-of-laws lives in a generic polygraph filler restricted to generator-free boundaries and discharged by Squier acyclicity — the library's one open hole. gandr's declared coherence rules are the complementary move: **per-law, named, asked-for coherence fields** in the shape, with the machine's completion certificates available to _discharge_ them for free models rather than a generic filler assumed for all.
Declared in the signature, derivable for the initial model — that division is the design.

## The design space, dispositioned

**What a 3-cell declaration is:**

| model                                  | a declared 3-cell is…                                                      | disposition                                                                                                              |
| -------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| machine-only certificate (status quo)  | a completion-emitted tracelet                                              | **carried**, unchanged — but no longer the only 3-cell                                                                   |
| asserted equation between rules        | `rule₁ ≡ rule₂` as a checker axiom                                         | **declined**; reversal condition: a proof-irrelevant reading of identity is adopted, which the identity ban list forbids |
| declared cell, replay-checked boundary | a named 3-cell whose two faces are rewrite composites, validated by replay | **carried** — the adopted reading; same species as a tracelet, distinct provenance, author-asserted peak                 |
| auto-adjoined filler                   | a filler between any two parallel composites                               | **declined**; no reversal condition — it entails uniqueness of identity proofs without K (see [[#The filler ban]])       |

**What a `sign` block presents when it sits in shape position:**

| model                          | the record's fields are…                            | disposition                                                                          |
| ------------------------------ | --------------------------------------------------- | ------------------------------------------------------------------------------------ |
| nothing — the block is a type  | unrelated; the block just declares a local datatype | **declined**; it answers scoping, not the shape question                             |
| an algebra of the presentation | one field per cell, typed by the cell's sphere      | **carried** — the adopted reading; the record _is_ the interpretation                |
| a free/initial model           | derived, not declared — the generative reading      | **carried as the other reading of the same block**; the higher-inductive growth path |

The block is the single source of truth for both readings, which is the anti-retrofit property: adding a law to the shape simultaneously extends the free model's theory **and** every instance's obligations.

## The member ladder, aligned to the ruled form

The ruled block form ([[circuit-cells#The block form, ruled]]) already gives every member kind a lowercase lead keyword that names its dimension, in judgment style:

```text
sign <Name> {
  sort <S> : Type              -- 0-cell: a declared sort
  oper <f> : sig               -- 1-cell: operation (defined symbol)
  rule <name> : lhs ==> rhs    -- 2-cell: named directed rewrite
  rule <name> : ρ ==> ρ′       -- 3-cell: the same member kind at rule-sorted endpoints
  cell …                       -- reserved: the dimension ≥ 4 tower (parse-and-decline)
}
```

(The ladder once carried a `data <C> : sig` constructor row at dimension 1.
The item-level `data` member is **retired** ([[declarations#data declarations]]): a family is declared once, whole, as a nested generator block, and in shape mode there was never a `cons`/`oper` distinction to preserve — a shape's constructors were always `oper`s, so no shape loses anything.)

This section's earlier draft proposed its own five-keyword respell (`cons` for constructors, a fresh `meta` member for 3-cells, a fresh `~>>` arrow); the ruled form **supersedes that draft**, and the deltas are worth recording because each dissolves a problem the draft had to argue around:

- **`data` was the constructor member's keyword, and the member is gone.** The draft's motivation — keyword-led members instead of the parser's uppercase-head case trick — was fully served by the ruled judgment style; the retirement then removed the row entirely, because the separated form's three side conditions (head uniformity, one head per block, family-wide positivity) are properties it must check where the nested form cannot express the violation.
  The internal description table deliberately does **not** follow the retired spelling (its field stays `ctors`): mirroring `data` inward would reintroduce exactly the block-versus-member ambiguity the surface's context resolves (gandr-ng9.18 ruling 4's one exception).
- **`oper` replaced `op`** as the 1-cell member — landed, in both the sign block's judgment form and the data-block sugar's applied form, with the retired `op` lead parse-and-declined by respelling hint; `op` is the operator-fixity declaration only, and no shared-keyword caveat is needed anywhere.
- **`sort` completes dimension 0**, and it behaves differently in the two block positions.
  In **shape position** a block may declare several sorts, including indexed ones — `sort Hom(dom: Ob, cod: Ob)` — because a presentation is generally multi-sorted (as landed, `sort` members are recorded into the declared sort set; indexed sorts remain reserved).
  In **generative position** — a `data`/`codata` block used as a type — the block's own name remains the unique implicit sort, exactly the desugaring of [[signatures#The desugarings]] (the nested block plus the polarity token).
- **There is no `meta` member and no third arrow.** A 3-cell is a `rule` between rule-sorted endpoints, with dimension read off the endpoints rather than off the glyph — see [[#Declared 3-cells are rules one dimension up]] for how this dissolves the draft's arrow question rather than answering it.
- **`rule` names are mandatory** in the ruled judgment form (`rule <name> : …`), which is the naming rung this lane owes.

**The migration this implies.** The landed data-block `rule` member is still the anonymous `rule lhs ==> rhs`; the ruled judgment form adds the mandatory name.
The migration is a decline-with-hint on the old form's missing name — "`rule` requires a name; write `rule <name> : …`" — pinned by a pathological corpus example when the naming rung lands, exactly as the `op`-to-`oper` and `~>`-to-`==>` respells were pinned when theirs did.

## Named 2-cells and the identity discipline

```text
rule assoc : comp(comp(f, g), h) ==> comp(f, comp(g, h))
```

The name closes a real gap: as-built, rules are **anonymous end to end**.
The engine's cell faces carry a source span, not a name, and engine cells are identified by store index and content-addressed deduplication.

Names buy four things, in order of force:

- **referability** — a 3-cell rule’s boundary must cite the 2-cells it composes, and cannot cite an anonymous one;
- **model fields** — `Model(S)` needs a field name per rule, and a signature cannot name a field after a store index;
- **diagnostics and inspection** — "declined at `assoc`" beats "declined at cell #3";
- **named loose-arrow generators** on the reflection face, which gives the query surface user vocabulary.

**Identity discipline, settled here:**

| question                                                     | ruling                                                         |
| ------------------------------------------------------------ | -------------------------------------------------------------- |
| what is content-addressed?                                   | the **cell**; the name is a decl-table binding to that content |
| do names affect deduplication or replay-equivalence?         | **never** — certificate identity is untouched                  |
| two names for one structurally identical face, in one block? | **declined** — a duplicate, and probably a typo                |
| the same content in different blocks under different names?  | **accepted** — names are block-scoped                          |

Mandatory naming also keeps the grammar honest.
After `rule` the second token is always an identifier and the third is always `:`, so no lookahead is needed to tell a name from an lhs term — the discriminating-tile discipline the landed `def` factoring already uses.

**A consequence for the boundary language, stated because it is easy to get backwards.** A rule's **pattern variables are bound by its lhs**, not by a parameter list on the name.
`rule assoc : comp(comp(f, g), h) ==> comp(f, comp(g, h))` binds `f`, `g`, `h` by their occurrence in the lhs.
An instantiation `assoc(x, unit(), y)` inside a 3-cell face then supplies terms for those variables **in declaration order**.
The name is not a binder; it is a reference.

**The one member kind this sits in tension with is the one whose declaration carries a telescope.** A circuit rule binds its variables in its **parameter list** — its ports, including the endpoints of rewrite-sorted ones — so its target boundary names variables its source does not bind, and the fresh-right-hand-side-variable rule that reads a rule's variables off its left-hand side refuses the ruled congruence block if it is read as binding there ([[circuit-cells#The derived pair meets the sphere by checking, not by synthesis]]).
The declaration table therefore does not run that rule against a circuit rule's boundary pair.
**Whether this claim scopes the rule to left-hand-side binders or admits the telescope as a second binder is owed an owner ruling**; until then it is a claim about one member kind, not a weakening of the discipline stated here.

## Declared 3-cells are rules one dimension up

```text
rule triangle : (assoc(f, id(b), g) then comp(f, unitL(g))) ==> comp(unitR(f), g)
```

A declared 3-cell is a named cell between two **parallel rewrite composites**: both faces must start at the same term and end at the same term, checked at elaboration.
Under the ruled arrow grid it needs **no new member kind and no new arrow**: it is a `rule` whose endpoints are rule-sorted — "dimension is read from the endpoints, never from the arrow" ([[circuit-cells#The block form, ruled]]) — so the member kind this section once minted (`meta`) and the glyph it once argued for (`~>>`) are both superseded, and this document records why the argument's own criteria are what the grid satisfies.

**The keyword question dissolves.** The earlier draft weighed `meta` against `cohr`, `sync`, `glue`, `weld`, and `coh` (the last colliding with CaTT-adjacent work, where `coh` is the coherence former itself).
The ruled form needs none of them: a rule _about_ rules is spelled `rule`, and what distinguishes it is its endpoints' sort — the same device that lets a rule binder appear in a rule's parameter list (a cell parameterized by rewrites) with no further vocabulary.

**The arrow question is answered by the draft's own collision analysis, one level up.** The draft's constraints were real: `~~>` is the ratified directed-transformation type former and must not be overloaded across two judgments; a fresh glyph must clash with none of the live arrows; and any tilde-led two-glyph pair (`~>` / `~>>`) carries a maximal-munch scanner obligation (`crates/surface-parser/src/label.rs`, `MULTI_PUNCT`, matched longest-first) that the molder cannot repair after the fact.
The grid satisfies every one of those constraints with **fewer** tokens rather than a new one: `~>` is retired outright, so no tilde-led rewrite arrow exists at any dimension, `~~>` loses its near-neighbour instead of gaining a second one, the scanner obligation lapses with the pair that created it, and `==>` at every dimension leaves the reader's dimension question to the endpoints and the checker's arrow-kind confirmation — three independent spellings of each fact, per the block-form ruling.
What the draft protected — that a 3-cell face is never confusable with a type former or a 2-cell face — is thereby carried by structure rather than by glyph inventory, which is the strictly stronger position.

## The boundary language

3-cell rule faces need a term language for _composites of named 2-cells_.
It is deliberately tiny — four constructions, each with an existing engine or prelude realization:

```text
ρ ::= r(t₁, …, tₙ)        -- rule instantiation: the named rule r with terms for its
                          --   pattern variables, in declaration order
    | here(t)             -- the identity (empty) rewrite at term t
    | ρ then ρ′           -- sequential composition (end of ρ = start of ρ′)
    | f(t̄, ρ, ū)          -- congruence / whiskering: the rewrite ρ applied in one
                          --   argument position of a cons/oper symbol f
```

The language has exactly two readings, and it is kept this small **because it is the largest fragment whose two readings are both already specified**:

| construction | engine reading (generative side)                                      | model reading (shape side)                       |
| ------------ | --------------------------------------------------------------------- | ------------------------------------------------ |
| `r(t̄)`       | the named cell fired at a position — one element of the rewrite path  | the rule's own field, applied at those arguments |
| `here(t)`    | the empty path; the reflection unit, where `refl` _is_ the empty path | `here`                                           |
| `ρ then ρ′`  | path concatenation, the engine's existing composition                 | `then`                                           |
| `f(t̄, ρ, ū)` | position extension into one argument slot                             | `cong` at the interpreted symbol                 |

**Congruence is written by juxtaposition, and `cong` is not surface syntax.** `cong` is the _model_ reading of the construction `f(t̄, ρ, ū)`.
Writing `cong f(…)` in a face is a category error: it names the interpretation inside the syntax.
A face's interpretation is a fold sending the boundary language into exactly the derived-combinator fragment of the `Path` algebra — each combinator one `walk`.

**One active position per application node.** `f(ρ₁, ρ₂)` — two simultaneous rewrite arguments — is **declined**.

- It denotes **horizontal composition**, whose two sequential readings agree only _up to interchange_, and adjudicating that silently is exactly the coherence smuggling this design refuses.
- Write the two whiskers in sequence instead; the interchange cell, where one is needed, is itself a declared 3-cell rule.
- The **principled future semantics already exists and is fenced**: accept exactly on **disjoint positions**, where the two readings are shift-equal, because the certificate algebra's shift-equivalence quotient is earned per pair by a trivial-overlap witness rather than imposed.
  Do not accept it any earlier and do not accept it any wider — the ruling and its reversal condition are [[../metatheory/guards#Horizontal-composition surface sugar]], and the interchange stratification behind it is [[../metatheory#Interchange, by layer]].
- The trigger that would reopen it is precise: **a construction that makes disjointness structural rather than analytic**, so that "these two redexes are disjoint" is a check the parser performs rather than a proof the reader supplies.

Nothing new is needed to _represent_ composites — the engine already has rewrite paths, concatenation, and position extension.
What is new is surface syntax and author-supplied instantiations.

**The grammar budget for all of this is zero new sorts**, which is the fact that makes the boundary language cheap:

- the landed `Sort` set is closed and small — five sorts — and rule faces are **already** `Expression` holes;
- rule-name application, `then`, and `here(t)` therefore all parse as **ordinary expression forms**, with the _cell namespace_ resolved at elaboration rather than by the grammar;
- so the boundary language spends no new sort, and its cost falls entirely on the mold checks and size budgets, which are the binding gate.

One collision rides with it, and it is the same kind of finding as the `Step` one below: **`then` is not a reserved keyword today, and it is separately a ratified-but-unlanded identity-family spelling** (composition).
Using it as an infix inside 3-cell faces puts one name on two jobs.
It joins the keyword-collision sweep before the boundary language lands.

## Sphere-typed boundaries

Boundaries are represented and checked as **globular telescopes**, the sibling library's sphere device:

```text
Φ ::= ⋆              -- the empty sphere: a sort's carrier
    | Φ ▸ x ⇴ y      -- one dimension up: the cells from x to y over Φ
```

A `rule` lives at the sphere `⋆ ▸ lhs ⇴ rhs` over its sort.
A 3-cell rule lives at `(⋆ ▸ s ⇴ t) ▸ ρ ⇴ ρ′`.

Two payoffs and one growth seam:

- **Globularity becomes judgmental.** A 3-cell rule's faces carry their shared endpoints in the type index, so "both composites are parallel" stops being a side condition re-checked downstream.
  Mis-glued boundaries fail to typecheck at the declaration table — once, at elaboration, with the sphere as the diagnostic.
- **Model typing becomes one recursion.** Interpret spheres by dimension: `⟦⋆⟧` is the sort's carrier, and `⟦Φ ▸ x ⇴ y⟧ = Path(⟦Φ⟧, ⟦x⟧, ⟦y⟧)`.
  A model's field for a cell at sphere `Φ` is an element of `⟦Φ⟧`, quantified over the cell's variable context.
  The 3-cell clause of `Model(S)` is then **not a special case at all** — it is literally the `rule` clause one dimension up, which the ruled one-member-kind spelling makes plain.
- **The recursion is dimension-generic.** The reserved tower therefore extends the declaration table and `Model` with no new clause; only surface syntax and the boundary language would have to grow.

**A filler that _computes_ its boundaries is checked against its sphere, never allowed to fill it.** Not every 2-cell member writes its boundary pair: a circuit rule's wiring determines both — the source by replacing every redex with its source, the target by replacing every redex with its target ([[circuit-cells#The derived pair meets the sphere by checking, not by synthesis]]).

**The sphere still comes from the declaration, and elaboration compares the computed pair against it.** That direction is what keeps the first payoff above true for derived boundaries: an index read off the filler it indexes cannot disagree with that filler, so a computed pair allowed to fill the sphere would leave a mis-glued boundary typechecking as a cell at the wrong sphere — unfalsifiable exactly where a derived form makes it easiest to get wrong.
The diagnostic stays the sphere's — it names the member, which of the two boundaries disagrees, the computed term, and the declared one — and a filler that computes no term at all is refused before any comparison is reported.
As built, the comparison is an endpoint-pair equality at dimension 2 rather than a telescope index, and of the two pieces once owed here only the representation is: the **surface route arrived on 2026-08-02** — a `sign` block's block-bodied `rule` members lower into `SignDesc::circuits` through `gandr-surface-engine`'s `circuit::desc`, reached from the item dispatch in `desc_elab`, and `check_desc` runs over what it writes.
What that route cannot yet write is the sphere itself, so the comparison it reaches is a re-derivation agreement rather than a constraint the declaration imposes, and `WfKind::DerivedBoundaryMismatch` stays unreachable from source until the two-sided boundary spelling lands ([[circuit-cells#The derived pair meets the sphere by checking, not by synthesis]]).
The **sphere-typed representation** is therefore the piece still owed, and it is owed to [[#Sphere-typed boundaries]] below.

## The `Model(S)` signature-former

`Model(S)` is a **signature expression** — a σ-former in the modules grammar — computed by elaboration.
It is not a frozen-core former.

Its clauses, by dimension, with `Γ_c` the cell's variable context:

| member of `S`                                | field of `Model(S)`                                                             |
| -------------------------------------------- | ------------------------------------------------------------------------------- |
| `sort X(Δ)`                                  | `type X : Δ → Type` — an abstract type member; indexed sorts need type families |
| `oper f : T̄ --> X`                           | `val f : U_ω (T̄ → F X)` — an operation of the algebra                           |
| `rule r : l ==> t` at sort `X`               | `val r : U_ω (Π Γ_r → F Path(X, ⟦l⟧, ⟦t⟧))`                                     |
| `rule m : ρ ==> ρ′` at rule-sorted endpoints | `val m : U_ω (Π Γ_m → F Path(Path(X, ⟦l⟧, ⟦t⟧), ⟦ρ⟧, ⟦ρ′⟧))`                    |

Function fields are computations under thunks because the core is call-by-push-value [@levy-cbpv].
`⟦ρ⟧` is the boundary language's model reading — a `then`/`cong`/`here` composite of _the rule fields themselves_, definable precisely because rules are named fields.

Three honest notes on what this costs.

- **In shape mode there is no `cons`/`oper` distinction in `Model`.** Both become operation fields.
  The distinction is meaningful only under the generative reading, where it is the constructor-versus-defined-symbol discipline and carries sufficient completeness.
  Both keywords are kept in shape blocks because one block serves both readings.
- **Rules interpret as `Path`, not as a directed former, for now.** A `rule` is directed; `Path` is groupoidal.
  `Model` therefore currently interprets every rule under the **invertible overlay** — semantically, the invertibility flag of the certificate algebra.
  When the directed family lands ([[../metatheory/directed-univalence]]), an orientation-respecting `Model` variant becomes possible, and it is a genuinely interesting one (lax and directed models).
  It is recorded as growth, not scoped.
- **The dependent-era gate is real, and shortcutting it is a named dead-end.** Rule fields at both dimensions quantify over their contexts, sort members are type-valued, and 3-cell fields have `Path`-typed endpoints.
  Shipping `Model` early via ad-hoc non-dependent encodings — law fields at closed instances only — would freeze a crippled field shape that instances then depend on.
  The design lands now; the implementation rides the era.

## The flagship examples

**The near-term flagship: the weak monoid.** Single-sorted, so no type families are needed and no indexed sorts appear.

```text
sign MonoidShape {
  sort M : Type
  oper unit : () --> M
  oper mul : (x : M, y : M) --> M

  rule unitL : mul(unit(), x) ==> x
  rule unitR : mul(x, unit()) ==> x
  rule assoc : mul(mul(x, y), z) ==> mul(x, mul(y, z))

  rule triangle : (assoc(x, unit(), y) then mul(x, unitL(y)))
              ==> mul(unitR(x), y)
  rule pentagon : (assoc(mul(w, x), y, z) then assoc(w, x, mul(y, z)))
              ==> (mul(assoc(w, x, y), z) then assoc(w, mul(x, y), z) then mul(w, assoc(x, y, z)))
}
```

`Model(MonoidShape)` is then, written out in the modules signature grammar:

```text
{ type M : Type,
  val unit : M,
  val mul  : U ω (M -> M -> F M),
  val unitL : U ω (Π(x: M) F Path(M, mul(unit, x), x)),
  val unitR : U ω (Π(x: M) F Path(M, mul(x, unit), x)),
  val assoc : U ω (Π(x y z: M) F Path(M, mul(mul(x, y), z), mul(x, mul(y, z)))),
  val triangle : U ω (Π(x y: M)
      F Path(Path(M, mul(mul(x, unit), y), mul(x, y)),
             then(assoc(x, unit, y), cong(mul(x, -), unitL(y))),
             cong(mul(-, y), unitR(x)))),
  val pentagon : …,  -- same shape, five 1-cell variables, both faces composites
}
```

That is a **bicategory-grade monoid as an ordinary module signature** — a monoidal-category shape when `M` is instantiated at a universe, one era later.
An instance is a module: `natAdd : Model(MonoidShape)` supplies `M = Nat`, the operations, `here`-based law fields, and coherence fields, with the coherences at grade `0` wherever the program never consumes them.

**The stage-1 flagship: the weak category.** This adds one indexed sort, which is what pushes it an era out.

```text
sign CatShape {
  sort Ob : Type
  sort Hom(dom: Ob, cod: Ob) : Type

  oper id : (a : Ob) --> Hom(a, a)
  oper comp : (f : Hom(a, b), g : Hom(b, c)) --> Hom(a, c)

  rule unitL : comp(id(a), f) ==> f
  rule unitR : comp(f, id(b)) ==> f
  rule assoc : comp(comp(f, g), h) ==> comp(f, comp(g, h))

  rule triangle : (assoc(f, id(b), g) then comp(f, unitL(g)))
              ==> comp(unitR(f), g)
  rule pentagon : (assoc(comp(f, g), h, k) then assoc(f, g, comp(h, k)))
              ==> (comp(assoc(f, g, h), k) then assoc(f, comp(g, h), k) then comp(f, assoc(g, h, k)))
}
```

`Model(CatShape)` needs `type Hom : Ob → Ob → Type`, a type family — hence universe formation and Π together, which is why the category is the stage-1 flagship and the monoid the near-term one.

## Programming with shapes

The type-class target — program with this like a Haskell class, at Agda-grade accuracy — decomposes over machinery this design does **not** itself build.

- **Instances are modules**: `module natAdd : Model(MonoidShape) { … }`.
  Modules already lower to record values with signature ascription, so shape signatures extend _what a signature can say_, never what a module _is_.
- **Explicit dictionary passing needs nothing new**: `def squareAll(M: Model(MonoidShape), xs: List(M.M)) -> …` is a functor in the modules sense — a thunked computation over a module argument.
- **Implicit retrieval rides the modules design verbatim**: `implicit module natAdd : Model(MonoidShape) = …`, with obligations resolved by world-scoped visibility, fuel-bounded search, and **coherence by canonicity** — at most one candidate per type per world, the modular-implicits discipline.
  This design adds no resolution machinery and takes exactly one position: **shape signatures must be legal implicit types.** As-built honesty: the tree has no metavariables and no type-directed search of any kind today, so the implicits lane is entirely the sequent program's.
- **Where the accuracy bites**: a Haskell instance carries its laws in comments; a `Model(S)` instance carries laws and coherences as typed fields.
  Two instances with equal operations but different coherence cells are _different instances_ — proof relevance is operational here exactly as it is for protocol adapters, because the pentagon field **is** the reassociation strategy a consumer may replay.

**Grades and erasure** compose exactly as identity evidence does.
The expected idiom is coherence cells at grade `0` in runtime-relevant instances: erased by the phase discipline, fully proof-relevant to the theory.
A dictionary whose coherence fields are 0-graded costs nothing at runtime; a program that transports along a coherence pays its fuel bill like any certificate replay.
"The cost of coherence is a measured number" extends from univalent transport to type-class law use with no new mechanism.

## The globular-carrier reading

The design is stated both finitely and over globular carriers, and the readings agree where they meet.

A globular set, coinductively, is one codata declaration away once type-valued fields exist:

```text
codata Glob {
  cells : Type,
  hom(x: cells, y: cells) : Glob,
}
```

A **carrier-general shape signature** `Model∞(S, Ξ: Glob)` interprets sorts as positions in `Ξ`, and each higher cell at `Ξ`'s own next dimension rather than at `Path`.
This is "structure over an ∞-graph": the algebraic structure and the carrier's globular structure are decoupled, so one shape has models in **any** globular world — types, setoid towers, reflected code universes.
The mathematical frame is the globular-operad account of weak ω-categories [@leinster-2003-higher-operads].

Every type is a weak ω-groupoid via its identity tower [@lumsdaine-2008-weak-omega-categories] [@vandenberg-garner-2008-types-weak-omega-groupoids].
In this vocabulary that fact is a corecursive definition:

```text
def PathGlob(A: Type) -> Glob = glob {
  cells = A,
  hom(x, y) = PathGlob(Path(A, x, y)),
}
```

and the reconciliation is one equation:

> **`Model(S)` over carrier `A` is `Model∞(S, PathGlob(A))`.**

The sphere denotation `⟦Φ ▸ x ⇴ y⟧ = Path(⟦Φ⟧, ⟦x⟧, ⟦y⟧)` is precisely "walk one dimension down `PathGlob(A)`".
The finite design is the general design specialized to the identity carrier, and nothing in the `Model` clauses forecloses the general carrier.

No claim is made that `PathGlob` is _constructible_ before the stage-1 era: it needs a type-valued codata field and `Path` in type position under corecursion, and both are gated.
**The equation is the design invariant those gates protect.**

## The dimension policy

User syntax stops at dimension 3, for reasons of substance rather than budget.

- **Dimension 3 is where user content becomes load-bearing.** A convergent presentation's coherence is generated by its critical-pair 3-cells: dimension 3 is the homotopy-basis dimension.
  For the classical weak structures, the named 3-cells _are_ the whole basis — pentagon and triangle generate all higher coherence [@maclane-2010-cwm] — so the finite block is not a truncation apology.
- **Each dimension needs a boundary language one dimension down.** dimension 3 needed composites of 2-cells.
  Dimension-4 cells would need composites of 3-cells: whiskering of 3-cells, and interchange among them.
  That is a real language-design bill with no near-term customer.
- **The tower is reserved, not refused.** A `cell` member production parses and declines with "higher cells reserved", pinned by a pathological corpus example — the same anti-retrofit device already used for `op` and `rule` themselves.
  The recorded growth theory is the CaTT line [@finster-mimram-2017-catt]: contexts-as-computads with a single dimension-generic coherence former, whose restriction to dimension ≤ 3 with author-asserted boundaries is exactly the rule ladder at dimensions 2 and 3.

**Two fences the tower must respect, stated here because they bound what a later pass may assume.**

- **The completion story does not lift.** Squier's citation is good at dimension one, where the completion loop lives.
  **Finite derivation type fails above dimension one** — there is an explicit finite convergent 3-polygraph with finite critical branchings that lacks it [@ara-burroni-guiraud-malbos-metayer-mimram-2025-polygraphs].
  So [[#Discharge by completion]] below is a dimension-one statement, and no rung above may assume it generalizes.
- **Computads are not a presheaf category for `n ≥ 3`** [@makkai-zawadowski-2008-computads], which is directly a hazard for "the tower is data".
  The counterexample's mechanism is Eckmann–Hilton, and its hypotheses — strictness, globular shapes, degenerate boundaries — are jointly required and individually unmet by gandr's pattern-to-pattern rules.
  The applicable escape hatch is **source-positivity** [@henry-2019-nonunital-polygraphs]: its Theorem 2.4.8 makes the class of _all_ source-positive polygraphs — those where the **source** of every generator is a non-identity arrow, an arrow being an identity exactly when no generator of its own dimension occurs in it (that source's section 1.2.3) — a good class of polygraphs, one conjunct of which is being an effective presheaf category.
  The elaborator is what puts gandr in that class: it refuses any rule whose left-hand side is not an operation application, so every generator's source carries its operation symbol.
  **The stronger _non-unital_ class of that source's Definition 1.3.2, which constrains targets too, is not available**, because a projection-shaped rule's target is a bare metavariable sent to the return continuation — a verdict resting on the reading that such a command is an identity 1-arrow on its sort, which the code does not state, and the one part of this that is not machine-checked.
  That source/target asymmetry — generator-headed on the left by elaborator enforcement, unconstrained on the right — is a design datum in its own right; the full statement with its locators, and the warning that "non-unital" also names gandr's unrelated circuit-algebra rung, is [[../metatheory#Cellular data — descriptions, cells, and computads|the metatheory track's cellular-data section]].

**One consequence for the surface, recorded because it constrains a neighbouring design.** `cell` is **spent** by this reservation.
A surface design that wants many-in/many-out cells at dimension 1 — the circuit-algebra direction — cannot claim that keyword and must grow the existing members instead.

## Elaboration and semantics

### Declaration-table extension

The description layer gains three things beyond what the signature unification already landed: a `name` on `RuleFace`; a sphere-indexed 3-cell member whose boundary carries composite terms over named cells (the same `rules` slot one dimension up, per the ruled one-member-kind reading — no separate 3-cell list is minted); and the shape-position reading of the declared sort set.
The sort set itself (`SignDesc.sorts`) landed with the unification's sort index, so what remains here is its shape-position semantics, not its representation.

This **extends the levitation anti-retrofit checklist**.
Its "2-cell faces" item generalizes to _faces at every dimension_, and the description-universe gap it flags now explicitly includes 3-cell faces and the composite language.
Same honesty as then: the description shape leaves room, and the typed-description story is not solved here.

### Boundary checking by replay

A 3-cell rule's two faces elaborate to engine rewrite paths.
Elaboration checks three things, in order:

1. each rule instantiation **matches** its named cell's pattern, by the engine's one-sided matcher;
2. the composites are **parallel** — judgmental under sphere indexing, and diagnostic-bearing at the surface;
3. the assembled 3-cell **replays**: both paths run from the shared source to the shared target.

The as-built home for this is direct.
Populate a tracelet with author-supplied paths and an **asserted** peak, then validate by the existing replay check.
Today every peak is derived by overlap enumeration, so **author-asserted boundaries are the one genuinely new engine behaviour** this lane introduces.
Boundary checking is replay, never a trusted assertion.
Directed composition through a 3-cell face inherits the acyclicity gate unchanged, declining with the cycle as the diagnostic.

**A wiring residual becomes load-bearing, and it is narrower than a first reading suggests.** Surface `rule` members do reach the cell layer as built: `gandr-surface-engine` carries a `gandr-theory-computads` dependency, its `desc_cells` module drives `elaborate_data_desc` over every description, and `gandr-surface-corpus`'s `desc` mode is that seam's pipeline caller — the same wire the `surface-engine` ↔ `theory-computads` row of [[#As-built impact]] records, and the wire `gandr-theory-computads`'s own `elaborate` module header names as how description-carried rules reach it today.
Two narrower absences are what remain, and only the second is this lane's: the **lowering** route (`gandr-surface-engine`'s `lower/codata.rs`) still declines `rule` members, so the description route is the only one that carries them; and the **completion engine** (`gandr-theory-computads`'s `completion` module) has no caller outside that crate and `gandr-theory-virtual-doctrines`' tests, so no surface-declared cell is completed today.
Replay checking _requires_ that second wire.
Landing it is this lane's first implementation obligation, and it is independently valuable, because it moves the reserved 2-cell slot toward the semantics its graduation rung already names.

### Discharge by completion

User coherence rules and machine tracelets are both 3-cells; **provenance separates them**, one dimension up from the existing cell-provenance discipline.
They interact in one valuable direction:

- for the **generative** reading of a convergent block, completion _derives_ confluence 3-cells;
- so a declared coherence rule whose boundary the completion certificate already fills is **discharged** — the free model's field is supplied by the machine, and the user wrote the pentagon once, in the shape, for every other model.

This is the Squier-degenerate slice showing up exactly where it was promised: for free models of convergent presentations, coherence computes — now with a user-visible field it computes _into_.

**The fence on this claim is the finite-derivation-type failure above dimension one** recorded under [[#The dimension policy]].
Discharge is a statement about the completion loop at dimension one.
Nothing here licenses assuming it at dimension 4 or above.

### The filler ban

The sibling library demonstrates, in a compile-only module, that adjoining a filler between **arbitrary** parallel 2-cells entails uniqueness of identity proofs without K. Its own filler is therefore restricted to generator-free boundaries. gandr adopts the stronger and simpler law:

> **The machine never adjoins a 3-cell the user did not declare or completion did not certify.**

Declared coherence rules are hypotheses of the shape — fields a model must fill — or certificates of the engine, replay-validated.
They are never ambient truncation.

This is the identity ban list extended one dimension.
**Definitional proof-irrelevance for identity was banned at dimension 2; a blanket 3-cell filler is the same collapse at dimension 3.** The standing discipline "variance is derived, never declared" gets its exact dual here — **fillers are declared or certified, never derived ambiently** — and each guards a collapse from the opposite side.

The corpus pins this with a pathological example — a surface attempt to _request_ an ambient filler between two parallel composites, which declines:

```text
sign MonoidShape {
  sort M : Type
  oper unit : () --> M
  oper mul : (x : M, y : M) --> M
  rule unitL : mul(unit(), x) ==> x
  rule assoc : mul(mul(x, y), z) ==> mul(x, mul(y, z))

  rule anyFiller : _ ==> _       -- declined: a 3-cell rule states its two faces.
                                 -- "fill whatever is parallel here" is the
                                 -- blanket filler, and it entails UIP without K.
}
```

The contrast with a legal 3-cell rule is the whole discipline in one line: `rule triangle : (assoc(…) then …) ==> …` **names both composites**, so the machine replays a boundary the author asserted; `rule anyFiller : _ ==> _` asks the machine to invent one.

### The reflection face

Named cells enrich the reflection face without changing its theory.

- Named rules become **named loose-arrow generators**, so the query surface gains user vocabulary.
- Declared coherence rules enter as proterms beside the certificate-embedded engine derivations already there, with the invertible-certificate surface unchanged.
- Names carry **no** semantic payload on the face: variance and linearity metadata stay derived, and replay-equivalence stays the identity.

One direction is recorded for the reflected layer, noted rather than scoped.
A shape block is precisely a presentation of a virtual-double-categorical theory [@nasu-2024-internal-logic], so the carrier-general signature at the reflected code universe is a candidate meeting point with the dependent-FVDblTT trajectory.
The 2-dimensional fragment separately has a theorem-backed home as a **cartesian double theory** with product-preserving lax-functor models [@lambert-patterson-2024-cartesian-double] [@patterson-2025-products]; the 3-cell layer sits above that literature, which is where this design routes it.

## As-built impact

Verified against this tree at the time of writing, symbol by symbol, and **re-verified after the signature unification landed** (the sort index, the vocabulary rename, the ruled inspection notation, and the `oper` respell) — several rows below record work that lane completed rather than work this one still owes.

| component                                          | as built today                                                                                                                                                                                                                                                                                                                                        | change this lane needs                                                                                                                                   |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `surface-grammar` (`src/surface/term.rs`)          | `data_member()` offers an uppercase-led constructor, `oper name(params) -> R?` (with the retired `op` lead parse-and-declined), and `rule <expr> ==> <expr>` with **no name slot** (the retired `~>` parse-and-declined); `codata_member()` carries the same `rule` alternative; the `sign` block parses `sort`/`data`/`oper`/`rule` judgment members | a mandatory `rule` name plus `:` in the data-block sugar, and a reserved `cell` member                                                                   |
| `surface-parser` (`src/mold.rs`, `src/label.rs`)   | `KEYWORDS` contains `op`, `oper`, `rule`, `data`, `codata`, and `sign`; `sort`/`node`/`feed` are deliberately contextual; `cell` is **not** reserved; `MULTI_PUNCT` carries the arrow grid (`-->`, `<->`, `==>`, `<=>`) longest-first, with `~>` still lexed so its decline can name the respelling                                                   | the `cell` keyword (H1 must lex it to decline it)                                                                                                        |
| `surface-engine` (`src/desc_elab.rs`)              | `rule` members elaborate: the dispatch builds a face from lhs, rhs, derived variable metadata, and a span, and pushes it onto the description's rule list; the retired `op` and `~>` spellings decline with their respellings                                                                                                                         | capture names; elaborate 3-cell faces from the composite language                                                                                        |
| `surface-engine` ↔ `theory-computads`              | **the wire exists**: `desc_cells.rs` drives `elaborate_data_desc` for every description, and the cell layer's declines surface as located diagnostics                                                                                                                                                                                                 | author-asserted boundaries through the same wire                                                                                                         |
| `theory-levitation` (`src/rule.rs`, `src/desc.rs`) | `SignDesc { id, sorts, params, ctors, opers, rules, circuits, polarity, attrs }` _is_ the declaration table, sort-indexed; `RuleFace { lhs, rhs, vars, provenance }` carries a **source span, not a name**; `circuits` carries the derived-boundary members, whose computed pair `check_desc` compares against the declared sphere (`src/circuit.rs`) | `RuleFace.name`; the sphere-indexed 3-cell boundary                                                                                                      |
| `theory-computads`                                 | `Tracelet { overlap, path_a, path_b, joins_at }` with `Tracelet::replay`; a content-addressed `CellStore` with `CellProvenance`; two-mode composition with the acyclicity gate, declining with the cycle as diagnostic; budgeted completion in `completion.rs`, peaks derived by overlap enumeration                                                  | a name registry beside `CellStore`; author-populated tracelets validated by the existing replay; a `SurfaceMeta` provenance beside `DerivedByCompletion` |
| `core-checker`                                     | `ValueType::Path` with both endpoints, `Value::Here`, and `Comp::Walk { scrut, motive, base }` are all present; `CompType::Arrow` is **non-dependent**; `ValueType::Universe` is carried with **no formation rule** in `checker.rs`; `subtype.rs` states stage 1 has **no unification variables**                                                     | **nothing** at the naming and declaration rungs; the stage-1 era later supplies Π, universe formation, and compound `Path` endpoints                     |
| modules lowering                                   | modules lower to record values with signature ascription; members are `def`-only over flat registries                                                                                                                                                                                                                                                 | later rungs: type members in signatures, local `data` in module bodies, `Model(S)` as a σ-former, `implicit module`                                      |

**The H2 rung's hardest-looking half already has a landed bridge, unwired.** `theory-levitation` carries `decode_desc` (descriptions to core types) and `PatternContext`/`TypedRuleFace` (a 2-cell's variable context decoded to core `ValueType`s), all library-complete with **zero callers outside the crate**.
`Model`'s `rule` clause is close to literally `TypedRuleFace` plus `ValueType::Path` — so the dependent-era gate is on Π and universe formation, not on the description-to-core decoding, which exists.

Three further as-built facts the design record did not carry:

- **`codata` blocks also parse `rule` members.** The aligned ladder and the 3-cell member must state their behaviour in codata position, or decline there explicitly.
  This document declines to guess and records it as an open question below.
- **Reserving `cell` is a source break** of exactly the kind `rec` was — an accepted, intended one, but it belongs in the migration story beside the anonymous-`rule` decline, not discovered at landing.
- **`Step` is taken by the abstract machine.** It is the successor-state outcome of the small-step driver (`machine.rs`).
  It is _not_ a collision with the identity family's reserved `Step` spelling — there, a directed former named `Step` **is** the reservation being cashed, not a clash with it.
  The collision is one-way, and with the machine.

## Staging

| rung   | content                                                                                                                                                       | gate                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| **H0** | mandatory rule names; declaration-table names (the member-ladder alignment itself landed with the signature unification). Zero frozen core.                   | grammar budgets and the mold checks                                                                      |
| **H1** | 3-cell rule members: boundary language, sphere-typed declaration table, replay validation through the landed wire; the reserved `cell` pin. Zero frozen core. | H0                                                                                                       |
| **H2** | `Model(S)` for single-sorted shapes — the monoid flagship; rule fields at both dimensions as `Path` and iterated `Path`                                       | the stage-1 dependent era: Π, universe formation, compound `Path` endpoints, plus context decoding wired |
| **H3** | indexed sorts and type families; the category flagship; local `data` in modules; module type members                                                          | H2 and the modules lane                                                                                  |
| **H4** | `implicit module` instances; type-class ergonomics                                                                                                            | the modules implicit lane                                                                                |
| **H∞** | reserved: the `cell` tower, `Model∞` over globular carriers, `PathGlob`, directed models                                                                      | owner-sequenced; each an addition by construction                                                        |

Every rung is an addition over the previous — names over anonymous cells, declared 3-cells over machine 3-cells, interpretation over declaration, general carriers over the identity carrier.

**The load-bearing asymmetry.** Naming and declaring cells is grammar, declaration-table, and replay work with **zero frozen-core spend**, because the engine's 3-cell shape and composite vocabulary already exist.
Interpreting them is where the dependent era is genuinely needed.
That half is designed now and staged behind the stage-1 bill, exactly as identity itself was.

## The corpus witness plan

**Model examples**, literate: a monoid shape with every dimension named; an explicit `Model(MonoidShape)` instance at H2; a generative convergent block whose declared coherence rule completion discharges; a 3-cell rule citing 2-cells by name, where the diagnostics show the names.

**Pathological pins**, one per guarded degeneracy:

| witness                                                | what it pins                                                                |
| ------------------------------------------------------ | --------------------------------------------------------------------------- |
| non-parallel 3-cell faces                              | the sphere diagnostic is the golden                                         |
| parallel but non-replaying composite                   | rejection at replay, not at assertion                                       |
| an unnamed `rule`                                      | the migration decline, with a rename hint                                   |
| duplicate rule **name**                                | the name discipline                                                         |
| duplicate rule **content** under two names             | the content discipline                                                      |
| a `cell` member                                        | the reserved-tower decline                                                  |
| a requested ambient filler between parallel composites | the filler ban                                                              |
| two rewrite arguments in one application node          | the horizontal-composition decline, with the interchange note as the golden |

The last two are the corpus witnesses this lane owes the binding-guards inventory, and they are its first entries beyond the existing K-derivation witness.

## Open questions, dispositioned

The design record left eight; verifying it against the tree raised two more.

| #   | question                                                                                                                                                          | disposition                                                                                                                                                                                                                                                                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | should a replay-equivalent but _structurally distinct_ face under a second name be declined, warned, or allowed?                                                  | **carried.** The structural case is settled — declined; this residual is the name-versus-content boundary, cheapest settled before the declaration table hardens                                                                                                                                                                     |
| 2   | the `~>>` spelling; `~=>` and `==>` carry different collision profiles                                                                                            | **retired, superseded by the arrow grid**: `==>` is the rewrite face at every dimension with dimension read off the endpoints, so no 3-cell glyph exists to choose — the collision analysis that motivated this question is re-derived against the grid at [[#Declared 3-cells are rules one dimension up]]                          |
| 3   | `sort` members in generative blocks — does the implicit-sort rule survive multi-sorted generative data?                                                           | **declined at first**, with the reversal condition that multi-sorted generative data gets its own design pass. Named as a scoped asymmetry that must not silently widen                                                                                                                                                              |
| 4   | directed models — does `Model` gain an orientation-aware mode, and what replaces `cong` there, given whiskering without inversion?                                | **parked** on the directed family ([[../metatheory/directed-univalence]]); the design records the seam, not the answer                                                                                                                                                                                                               |
| 5   | shape identity — does a shape block mint a nominal id, or is a shape purely structural description data?                                                          | **carried.** It couples to content-addressing; the nominal reading is what an implicit-resolution key would want                                                                                                                                                                                                                     |
| 6   | horizontal-composition sugar — accept `f(ρ₁, ρ₂)` once an interchange discipline is fixed, or decline permanently?                                                | **declined with its reversal condition already recorded** in [[../metatheory/guards#Horizontal-composition surface sugar]]: acceptance is licensed exactly on disjoint positions. The trigger is a construction making disjointness structural rather than analytic; this question and that construction's design must move together |
| 7   | instance coherence beyond canonicity — is anything owed for _definitional_ coherence between instances met through different paths?                               | **carried, and honestly fenced**: per-world canonicity avoids the global-uniqueness question rather than answering it                                                                                                                                                                                                                |
| 8   | the `Model` spelling — `Model`, `Alg`, or a postfix form                                                                                                          | **parked** with the keyword table                                                                                                                                                                                                                                                                                                    |
| 9   | _(raised by verification)_ what do the aligned ladder and the 3-cell member mean in **codata** position, given that `codata` blocks already parse `rule` members? | **carried, and newly visible.** The design record did not consider codata blocks; declining there is a legitimate answer but must be a decision, not an omission                                                                                                                                                                     |
| 10  | _(raised by verification)_ `Step` as a directed-family former name collides with the abstract machine's successor-state outcome                                   | **carried**; a rename decision is owed before a third `Step` appears, on the same footing as the existing two-`Rigid` collision. Note it is **not** a collision with the identity family's reserved `Step` spelling — a directed former of that name cashes the reservation rather than clashing with it                             |
| 11  | _(raised by verification)_ `then` is used as the boundary language's infix **and** is a ratified identity-family spelling, and is reserved as neither             | **carried**, into the keyword-collision sweep, before the boundary language lands                                                                                                                                                                                                                                                    |

Five items the design record raised in passing, dispositioned so none vanishes:

- **Matching-modulo is untouched, and `Model` is orthogonal to the oriented runtime story** — **carried** as a scope statement: nothing in the shape reading changes how the generative reading matches.
- **Shape instances are natural equivalence customers** — a shape isomorphism between two instances is carried certificate data, the algebraic case of the temporal reading.
  **Carried**, unscoped; it is a consequence of proof relevance, not a further obligation.
- **The unresolved module-surface conflict** (braces versus a `sig … end` form) is **inherited, not adjudicated** here — this design takes no position on the module surface's spelling.
- **`Glob`/`PathGlob` instantiate the codata design at type-valued fields and add no new codata machinery** — **parked** on the codata/corecursion lane, which owns the former they use.
- **The boundary language is another small language to maintain**, and its restrictions (one active position, no horizontal sugar) will be felt by anyone writing large pastings.
  The alternative — full pasting diagrams — is a far larger bill deliberately not run up.
  **Carried as a stated cost**, with the reversal condition of open question 6.

## Source and confidence

- The design record is a single well-curated source — medium confidence by the corpus's scale — explicit that it is a design pass with no decision face, and explicit that its user-facing content is new surface rather than as-built.
- Its **as-built claims were re-verified against this tree**, symbol by symbol, rather than carried across.
  **Every row held.** What verification added is what the record did not carry: that `codata` blocks also parse `rule` members; that reserving `cell` is a source break; that `Step` collides with the machine's successor-state outcome; and that `then` is doubly spoken for and reserved as neither.
- The flagship examples and every code block are the record's own, verified character by character against it.
  An intermediate summary that gave rules a parameter list and spelled congruence with a `cong` keyword was wrong on both counts, and is not what this document carries.
- This document was adversarially reviewed against the record on both axes before landing.
  That pass found three real defects, all fixed here: a lexing claim contradicted by `label.rs` (`~>>` is a maximal-munch scanner entry, exactly as the record said, not an obligation-ordering matter); a dropped grammar-budget analysis; and a dropped as-built bridge (`decode_desc`/`TypedCellFace`) that is the most actionable architecture fact in the record.
  It also caught two "corrections" this document had attributed to the record which the record never claimed — the source of those was an intermediate summary, and the misattribution is withdrawn above.
- **This document was aligned to the ruled block form under the signature unification** (gandr-ng9.18 ruling 6): the member ladder now cites [[circuit-cells#The block form, ruled]] rather than proposing its own respell, the superseded `cons`/`meta`/`~>>` draft is recorded with its supersession rationale rather than deleted, the 3-cell-arrow argument is re-derived against the arrow grid rather than mechanically respelled, and the as-built table was re-verified against the post-unification tree.
  The `Model(S)` clauses, the boundary language, the sphere discipline, the filler ban, and the dimension policy are carried unchanged in substance — the alignment moved spellings and their justifications, never the semantics.
- The sibling library's filler demonstration, sphere device, and record-tower precedent are absorbed as design shape under the standing no-vendoring gate.
  The division of labour between its generic filler and gandr's named coherence rules is a live conversation, not a settled comparison.
