# The directed family — one-way identification at the surface

The directed identity family's surface: what a one-way identification is for, how you write one, and what its eliminator refuses.
The family is **designed, not landed** — the identity-layer phase owns the landing — and every spelling here is decided surface vocabulary whose as-built status is stated where it is used.
Why this surface is the right one, and what it models, is the metatheory lane's question, answered in full at [[../metatheory/directed-univalence]] and summarized at [[../metatheory#Directed univalence]]; this document answers the surface one.
The two accounts are written for two readers, so the shared syntax is stated in both rather than delegated to either.

## Why a one-way identification exists

A `Path(A, x, y)` says `x` and `y` are the same: transport goes both ways, and inversion (`back`) is derivable.
A `Flow(A, x, y)` says something weaker and more common: **`x` transports into `y`, in this direction** — with no promise of a way back.
The identifications a working program actually meets are mostly one-way:

* **a deprecation** — the old record flows into the new one; the new one carries information the old never had, so there is no backward map to find;
* **a backend migration** — terms of the old presentation replay onto the new one, certificate in hand; the new backend's inhabitants need not be reachable from the old;
* **a refinement** — a specification flows into anything implementing it; an implementation does not flow back into its spec.

In each, a two-way claim would be _false_, not merely unproven: the round trip does not exist.
So the language needs a former that says exactly the true thing — and refuses, by construction, the moves that would pretend otherwise.

## The two families, side by side

|                 | groupoidal (rung 1, built)                           | directed (designed)                                                                               |
| --------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| former          | `Path(A, x, y)`                                      | `Flow(A, x, y)` — spelled `A ~~> B` at the type level (ratified spelling of record)               |
| intro           | `here(v)`                                            | `diag(v)` — the diagonal intro: every point flows to itself                                       |
| elim            | `walk(p, motive, base)` — the full dinatural J       | `walk(p, motive, base)` — the **same eliminator**, under the **motive-covariance side condition** |
| composition     | `then`, derived by one walk                          | `then`, the **same spelling** — directed composition _is_ covariant transport                     |
| inversion       | `back`, derived                                      | **underivable by construction** — the refused motive shape is the symmetry shape                  |
| iso spelling    | `A <~> B`                                            | `A ~~> B` — a one-way certificate, **not** an iso                                                 |
| permanent guard | a K-derivation witness must fail elaboration (built) | a symmetry-derivation witness must fail elaboration (designed; the refused motive is the check)   |

The two formers are **independent primitives; no kernel coercion between them**.
The comparison is a theorem — the core-coincidence obligation of the directed univalence statement — and a coercion before that theorem would assume it as an axiom ([[#The derived coercion from Path to Flow, when it comes]]).

The shared eliminator and composition names are the settled answer to the open vocabulary question (the metatheory roadmap's open question 9, settled by owner decision 2026-07-31): the directed eliminator _is_ `walk` run under a side condition the groupoid case never needs, so one name does both jobs; and directed composition _is_ `then`, because composition is covariant transport and directedness costs nothing at dimension 1.

## The spellings of record

Decided surface vocabulary, none of it landed yet:

| spelling        | meaning                                                 | status                                                                                                                                                                                                                                                                                                                                                            |
| --------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Flow(A, x, y)` | the directed former, prefix form                        | designed; nothing in `core-checker` carries it today                                                                                                                                                                                                                                                                                                              |
| `A ~~> B`       | the directed former at the type level                   | ratified; absent from the scanner today — landing it is a `MULTI_PUNCT` table addition ordered before `~>` (the `~>>` scanner-change precedent of [[higher-cells]], at `crates/surface-parser/src/label.rs`)                                                                                                                                                      |
| `diag(v)`       | the diagonal intro                                      | settled 2026-07-31 (the metatheory roadmap's open question 9)                                                                                                                                                                                                                                                                                                     |
| `walk`          | the eliminator, shared by both families                 | groupoid case built (`Comp::Walk`, `crates/core-checker/src/syntax.rs`); the directed side condition is designed                                                                                                                                                                                                                                                  |
| `then`          | composition, shared by both families                    | ratified                                                                                                                                                                                                                                                                                                                                                          |
| `back`          | inversion, groupoid family only                         | derived as built — no kernel form ([[#Why inversion is absent rather than unimplemented]])                                                                                                                                                                                                                                                                        |
| `A <~> B`       | the isomorphism spelling (groupoid family)              | ratified                                                                                                                                                                                                                                                                                                                                                          |
| `Step`          | the identity family's reserved directed-former spelling | **reserved, and colliding**: `Outcome::Step` is the abstract machine's successor-state outcome (`crates/core-checker/src/machine.rs`). A directed former named `Step` cashes the reservation rather than clashing with it; the collision is one-way, with the machine. The rename decision is owed before a third `Step` appears ([[#Open items, dispositioned]]) |

The former names are the current ratified spellings (a rename from the earlier `Id`/`refl`/`J` family; the rename itself is recorded as a ratified surface decision).

## The diagonal intro

```gandr
diag(3) : Flow(Nat, 3, 3)     -- every point flows to itself
```

`diag` is the directed family's reflexivity, exactly as `here(3) : Path(Nat, 3, 3)` is the groupoid family's.
It is the diagonal the walk's base checks against: a walk's base supplies a witness where the endpoints coincide, so `diag` is what the base returns at the points where nothing moves.

## The directed walk, and the motive-covariance condition

`walk` eliminates a `Flow(A, x, y)` exactly the way it eliminates a `Path(A, x, y)`: an explicit motive `fn(a, b, q) => C` naming both endpoints and the certificate, and a diagonal base `fn(z) => c` checked at `C[z/x][z/y][diag(z)/q]` — both endpoint binders map to the base binder, as in the as-built `base_diagonal_type` of `crates/core-checker/src/checker.rs`.
One condition, with no analogue in the groupoid case:

* **the motive may use the moving endpoint only in covariant position.** A motive placing it in a contravariant slot is **refused**.

Covariant use — the moving endpoint appears only in positive position; the motive delivers the migration map itself:

```gandr
-- migrating along a deprecation: the walk delivers Old -> New,
-- and applying it to cfg : Old migrates the value
walk(deprecation, fn(Old, New, q) => (Old -> New), fn(x) => fn(v) => v)(cfg)
```

The base `fn(x) => fn(v) => v` is the identity map at the diagonal instance `x -> x`; the result type is the motive at the real endpoints, `Old -> New`.

Refused — the moving endpoint appears as a _source_.
The canonical refused shape is the **symmetry motive**:

```gandr
-- REFUSED: asks a one-way certificate to run backwards
walk(p, fn(a, b, q) => Flow(A, b, a), fn(z) => diag(z))
```

and so is any motive that hides the same shape — here the moving endpoint sits in an arrow's domain:

```gandr
-- REFUSED: `b` in negative position; a non-diagonal target cannot be supplied there
walk(p, fn(a, b, q) => (b -> b), fn(z) => fn(v) => v)
```

This one's diagonal instance `z -> z` is perfectly inhabitable, so the base checks — the refusal is purely the covariance condition: the domain occurrence of `b` is contravariant, and a one-way certificate cannot supply the target in negative position.

* The check is **term-structural and total** at this phase — no variance-sorted contexts (those arrive with the reflected layer; the general dipresheaf variance judgment is metatheory work).
* Why a program would hit the refusal: the natural spelling of "go backwards" _is_ the symmetry motive, and contravariant uses sneak in through function arguments — a one-way certificate cannot supply a function **from** the target **back to** the source.
* **The permanent guard**: a symmetry-derivation witness must fail elaboration — the directed twin of the K-derivation witness that guards `Path` (`crates/surface-corpus/examples/pathological/identity/k-derivation.gandr`, which asserts the `without-k` spelling on every diagnostic of its elaboration path).
  Its designed shape:

  ```gandr
  def sym(A: Type, x: A, y: A, p: Flow(A, x, y)) -> F(Flow(A, y, x)) {
    walk(p, fn(a, b, q) => Flow(A, b, a), fn(z) => diag(z))
  }
  ```

  The witness itself is **owed** — one of the binding guards without a pathological example in [[../implementation/roadmap#The corpus witness inventory|the corpus witness inventory]] ("the symmetry-derivation refusal for the directed former").
  Its diagnostic is the motive-covariance decline, whose exact spelling lands with the identity-layer phase.

## Why inversion is absent rather than unimplemented

For `Path`, `back` is not a kernel form: it is **derived** — one `walk` with the flipped motive:

```gandr
-- the groupoid `back`, derived: a walk with the symmetry motive
back(p) = walk(p, fn(a, b, q) => Path(A, b, a), fn(z) => here(z))
```

The as-built tree exercises exactly this script: the conformance suite eliminates `here(7)` with the motive `F(Path Integer y x)` — the `back`-at-a-point row of `crates/core-checker/src/conformance.rs`.

For `Flow`, that same script **is** the refused motive of [[#The directed walk, and the motive-covariance condition|the previous section]].
So `back` is not missing from the directed family; it is the thing the eliminator exists to refuse.
Where a genuine round trip exists, the certificate carries inverse-plus-round-trip evidence and lives in the invertible core — the groupoid family's home, where `ua` applies.

## Composition is covariant transport

`then` composes in both families, derived by the same script — one walk:

```gandr
p then q : Flow(A, x, z)     -- from p : Flow(A, x, y) and q : Flow(A, y, z)
```

Directedness costs nothing at dimension 1: sequential composition is covariant in its moving endpoint, so the groupoid script already _is_ the directed script — `then` never asks the eliminator for a contravariant use.

## What ua-dir is for, at the surface

The two univalence statements, side by side:

```text
ua      : Equiv a b → Terms a ≅ Terms b       -- the groupoid statement (rung 1's shape)
ua-dir  : FlowEquiv a b → Terms a ~~> Terms b -- the directed statement (the fenced statement of record)
```

`ua` says: an invertible identification between presented structures **is** an isomorphism of their term algebras.
`ua-dir` says the one-way version: a leaf-natural one-way certificate between presented structures **is** a directed identification of their term algebras.
What that buys a program: a directed identification **becomes usable transport** — replay along the one-way certificate.
The three cases of [[#Why a one-way identification exists]] are exactly its targets:

* a **deprecation** is a `FlowEquiv` from the old presentation to the new, so `ua-dir` makes old terms replay as new ones;
* a **backend migration** is the same shape with a different name;
* a **refinement** is the same shape with the specification at the source.

What the statement costs and why it is true — the four obligations (sound, full, sectioned, core-coincident), the alphabets it quantifies over, the one-way generator classes, and the two permanent degeneracy witnesses that fence it — are the metatheory lane's account at [[../metatheory/directed-univalence]].

## The derived coercion from Path to Flow, when it comes

A derived `Path → Flow` coercion is **wanted** — after `ua-dir` lands, as a surface form at a named stratum (the metatheory roadmap's open question 20).
Never before: the comparison is the core-coincidence theorem, and a kernel coercion ahead of it would assume the theorem as an axiom.
What the coercion will say, once earned: every invertible identification is, in particular, a one-way one — the groupoid family is the directed family's invertible core.

## Open items, dispositioned

| item                                          | disposition                                                                                                                                                                                   |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the `Step` rename decision                    | **carried** — a rename decision is owed before a third `Step` appears, on the same footing as the two-`Rigid` and `then` collision decisions ([[higher-cells#Open questions, dispositioned]]) |
| variance-sorted contexts for the motive check | **carried** — they arrive with the reflected layer; the general dipresheaf variance judgment is metatheory work                                                                               |
| the symmetry-derivation corpus witness        | **carried** — owed, recorded in the corpus witness inventory; the designed shape is in [[#The directed walk, and the motive-covariance condition]]                                            |
| `~>` in type position beside `~~>`            | **carried** as a recorded hazard — `~>` relates terms of one sort, `~~>` relates types; distinguishable but confusable ([[circuit-cells#Open questions, dispositioned]])                      |

## Source and confidence

* Written from the phase-3 design draft (the `Flow`/`~~>` family, the eliminator's discipline, the two univalence statements with their obligations and alphabets) and the landed metatheory lane [[../metatheory/directed-univalence]].
  The draft overlaps the landed document substantially; this document carries what the draft adds for the surface reader and states the shared kernel vocabulary in full rather than deferring to the metatheory account.
* The vocabulary settlement (shared `walk`, shared `then`, `diag`) is the owner decision of 2026-07-31, recorded where the question lived (the metatheory roadmap's open question 9).
* Every as-built claim was verified against this tree at write time: `ValueType::Path` with its two value endpoints and `Value::Here` and `Comp::Walk` (`crates/core-checker/src/types.rs`, `crates/core-checker/src/syntax.rs`); `back` derived, exercised as a flipped-motive `walk` in the conformance suite (`crates/core-checker/src/conformance.rs`); no `Flow` and no `diag` former anywhere in `core-checker`; the scanner's `MULTI_PUNCT` table carrying `~>` and neither `~~>` nor `~>>` (`crates/surface-parser/src/label.rs`); `Outcome::Step` as the machine's successor-state outcome (`crates/core-checker/src/machine.rs`); the K-rejection witness at `crates/surface-corpus/examples/pathological/identity/k-derivation.gandr`.
