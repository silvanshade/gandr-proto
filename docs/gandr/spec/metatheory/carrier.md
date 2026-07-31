# The circuit-algebra carrier

The full record of the shape substrate as landed in `metatheory/src/Gandr/Shape/{Graph,Graft,Decidable,Structure}.agda`, under the generality ruling of [[../metatheory#The substrate is the full circuit-algebra rung|the metatheory track]].
Everything here is machine-checked; the section names the theorem where one exists.

## The data

```agda
data Match : List Ob → List Ob → Set where
  []  : Match [] []
  _∷_ : Insert x ys zs → Match xs ys → Match (x ∷ xs) zs      -- a source chooses a SINK
  cap : Insert y xs xs′ → Match xs ys → Match (x ∷ xs′) ys    -- a source chooses a SOURCE: the cut

data Shape : List Ob → List Ob → Set where
  wires : Match Γ Δ → Shape Γ Δ
  node  : (A B : List Ob) → Append B Γ Γ′ → Append A Δ Δ′ → Shape Γ′ Δ′ → Shape Γ Δ
```

A shape is a list of corollas terminated by one wiring; each `node` publishes a corolla's ports into the shared interface (rather than consuming from a pool — a pool discipline would make wheel-freeness structural and cost the predicates their refuters), and `wires` closes with the matching.
Listings are primary and incidence is derived; the interface is in the index, which is what lets an arity abstraction quantify over it.
Witness relations (`Insert`, `Append`, `Regroup`, `Widened`) are carried in constructors rather than computed, so nothing downstream unifies against concatenation.

`cap` is gandr's cut at the wiring layer: a vertexless `Shape (c ∷ ω c ∷ []) []` is a producer of $c$ cut against a consumer.
`Match Γ Δ` is inhabited only when $|Γ| ≥ |Δ|$, the difference paid in caps — the downward condition reproduced by the carrier rather than imposed on it.
The flow-through fragment is the predicate `CapFree` (the bijective wirings), with `follow⁺` and the bridge lemma carrying the pre-cut lemmas across unchanged.

## The operations

* **grafting** (`graft : Shape Γ Δ → Shape Δ Θ → Shape Γ Θ`) — composition along the whole shared interface; total; vertex listings concatenate (`verts-graft`); the derived identity wiring is a two-sided unit at the cost of uniqueness of identity proofs on the colours and nothing more.
* **the merger** (`merge : Append Γ₁ Γ₂ Γ → Append Δ₁ Δ₂ Δ → Shape Γ₁ Δ₁ → Shape Γ₂ Δ₂ → Shape Γ Δ`) — parallel composition, **derived, not a constructor** (a constructor would give one graph many terms and manufacture exactly the orbit problem canonicalization exists to solve).
  It recurses on the first operand, threading that operand's wiring into the second one position at a time (`wire-in` per wire, `cap-in` per cut), because a single position commutes past a published block (`insert-shift`) while a block does not.
  `verts-merge`: the composite's vertex listing is the concatenation, first operand first.
  **`merge-idn`: whiskering is the merger at an identity operand, definitionally** — an independently landed operation falling out by `refl`, the strongest evidence available that the operation is right.
* **contraction** arrives through `cap`; the presentation result that grafting is derivable from merger plus contraction is deliberately _not_ taken as the carrier definition — it is a second, pasting-layer commit.

## The edge identity and the palette

The original edge datum named **half-edges** while every consumer read it as edges; the coincidence held only pre-cut (every edge had exactly one source end) and broke in three places at once when the cut landed (double-counted cuts, one wire presenting as two antiparallel arcs, reducedness keyed on the wrong identity).
The repair, under the generality ruling:

* an **edge listing** (`Wire`/`edges`), one entry per pair the wiring makes, on the same footing as the vertex listing; `ends`/`route` give a wire two ends of one type; `Attach` is incidence without a direction claim;
* a **palette**: the colour involution `dual` with its involutivity law and the orientation `pole` with `pole-dual` — the involutive colour set together with the orientation morphism $θ : (C, ω) → \{↑, ↓\}$ that _is_ gandr's CBPV polarity;
* **`cut-oriented`**: a legitimate cut (joining $c$ to $ω c$) has ends of opposite poles, hence runs one way — the involution is load-bearing for the incidence, not only a legitimacy predicate; caps stay colour-unconstrained in the constructor (the published choice), so legitimacy remains a predicate;
* **`mono-unoriented : ¬ Palette ⊤`**: one self-dual colour admits no orientation — the free compact closed category on one self-dual object appearing as an empty type, and the theorem that orientation is genuinely extra structure;
* `forth-not-back`: a wire runs at most one way, which is what makes a directed walk mean anything.

The predicate stratification that results (and that discharges the decoration criterion's structure-on-colours half by quantification):

| layer                                                                          | takes a polarity? | holds on                                   |
| ------------------------------------------------------------------------------ | ----------------- | ------------------------------------------ |
| `Link`, `Adj`, `Walk`, `Acyclic`, `Connected`, `SimplyConn`, `Cell`, `Matched` | no                | every shape, cuts included                 |
| `Arc`, `Dir`, `WheelFree`, `Ranked`                                            | yes               | every shape, once its colours are oriented |

`Cell` carries the undirected predicate and _derives_ wheel-freeness at every polarity (`simply-conn⇒wheel-free`); `Ranked` (`rank` with `climbs`) is a discrete Morse-style ordering with `ranked⇒wheel-free`.

## The incidence theorems

**No cup, three consequences that stand or fall together** (proved, not cited): the wiring is downward; the nodeless loop is inexpressible; composition cannot manufacture a closed component.
If a cup is ever added all three go at once — make the operation say what it means instead.

**The merger's incidence theorem, both directions** (`merge-apart`/`merge-disconnected`; `merge-components`): no edge of a merge joins the two operands, and each operand's own adjacencies survive, so reachability in a merge of two connected shapes is exactly agreement of the side — **exactly two components, and they are the operands**.
The technique is reusable and cheaper than the obvious dictionary of injections: carry a _side_ (a boolean on vertices-plus-legs) and prove both ends of every edge agree about it; three facts carry it (the side survives wire-threading, cap-threading, and the node step), and the dictionary is never built.
The work lives in the threading, not the merger: `split-shift` (a position shifted past a published block never lands in the block) is the load-bearing lemma.
The two halves of the converse are asymmetric — the second operand's edges are the ones threading left alone (nearly free), the first operand's are the fresh ones (the work) — and the fresh-cut case **cannot** be stated as an equation on ordered incidence, because a cut's two ends are an unordered pair; it must be stated about the symmetric `Link`.
Presentation symmetry in the substrate propagates to the _shape of the statable theorem_, not only to proofs.

**The cut's port asymmetry, absorbed three times, never by canonicalization**: at the wiring (`cap-swap` — the same cut written at either port is the same term, by `refl`, because the canonical `Match` writes a capped pair once), at the edge listing (the entry is carried, not computed), at the incidence (`Ends`, the explicit unordered pair).
This is the identification-sorting test holding in practice: presentation symmetry is free; automorphism symmetry (the merger swap, false on the nose for the ordered carrier) is canonicalization's.

## Gate witnesses

The counterexample suite is part of the carrier's content — the predicates need refuters:

* `bigon` — a graft of two cells that reconverges, refuted with its cycle (grafting does not preserve cell-ness);
* `two-points`, `corollas-apart` (which _has_ edges), `bigon-point` — merges that disconnect, including the smallest case where "each operand's adjacencies survive" has content;
* `gluing` — two vertices joined by one contracted wire: connected, acyclic, a cell (what the graph says, which the pre-repair edge identity denied);
* the same self-gluing at two palettes — over one self-dual colour the cut runs nowhere; over the two poles it runs producer-to-consumer, against the listing;
* two computational pins predicted by hand before the checker saw them (a crossing beside a plain wire; a cut beside a plain wire), because over one colour every wiring at an interface has the same type and typechecking alone would not catch a wrong wiring.

## Engineering lessons priced for the next change of this shape

* The cost of well-founded recursion is in the proofs, not the recursion: facts about a well-founded function must be generalized over the accessibility witness and instantiated at the end.
* The house rule against `with` on a recursive call earned its keep: the inverse lookup had to be restructured into applied liftings before its identity lemma could be proved by congruence.
* The morphism record already carried the input-leg action, unused until the incidence could reach an input leg — the polarity was latent in the design, not imposed on it.
* Small named substrate pieces (`step-out`, `incid`, the `Slot`/`islot` view on `Insert`) exist because every statement of this kind has to commute with them; name them rather than inlining.

## Remaining carrier work

None queued.
The open _directions_ (deleting the cell record's simple-connectivity demand; the surface-language question; the translation lemma to the graphical-species presentation; canonicalization) are in [[roadmap]].
