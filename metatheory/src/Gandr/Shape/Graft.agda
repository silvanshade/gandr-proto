{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Graft — the arity OPERATION on the cell shape: plugging one
-- shape's output legs into another's input legs.
--
-- `Gandr.Shape.Graph` landed the objects and the derived unit; this module
-- lands the multiplication. Together they are the graph-shaped arity kit that
-- `Gandr.Arity` is to be extracted from alongside `Gandr.Arity.Path`.
--
-- ── WHAT GRAFTING IS HERE, AND WHY THAT IS THE RIGHT ANALOGUE ───────────────
-- The linear kit's multiplication is `Path a b → Path b c → Path a c`, chaining
-- two arities end to end along the object they share. A `Shape Γ Δ` is indexed
-- by its interfaces in exactly the same way, so the analogue is
--
--   graft : Shape Γ Δ → Shape Δ Θ → Shape Γ Θ
--
-- composing along the whole shared interface `Δ`. Connectivity is a PREDICATE
-- on objects here, not a restriction the operation carries, so nothing has to
-- be said about whether the composite is a cell — and it usually is not.
-- `bigon` below is the witness: two corollas, each a cell, grafted along two
-- legs into a composite that reconverges and so is not simply connected.
--
-- ── THE INTERLEAVING, WHICH IS THE WHOLE DIFFICULTY ─────────────────────────
-- `node A B` publishes `B` to the front of the source interface and `A` to the
-- front of the sink interface. So peeling a vertex off ONE operand changes the
-- interface the OTHER operand is stated over, and the other operand has to be
-- extended along that change. That extension is whiskering, and it is not
-- avoidable by presentation: peeling the second operand's vertex instead needs
-- the first operand whiskered, and publishing ports to the far end of the
-- interface moves the crossing from one whiskering to the other.
--
-- What IS avoidable is paying for the crossing with a block swap. Whiskering a
-- shape by a whole block at once has to commute a block past each vertex's
-- published ports — a permutation. Whiskering by ONE wire at a time does not:
-- `insert-shift` walks a single new position past a published block by adding
-- one `tail` per element, with no matching to permute. So `lwhisk` is `wire-in`
-- iterated, and the block swap `Gandr.Shape.Graph.swap-match` — which the
-- corolla genuinely needs — is not used here at all.
--
-- ── THE MULTIPLICATION IS A COMPOSITE, AND THAT HAS A COST ──────────────────
-- Stated plainly because it bears on the interface extraction. In the linear
-- kit concatenation is directly structurally recursive, so ONE inductive graph
-- (`Cat`) speaks it. Here `graft` is built from seven operations — `preplug`,
-- `lwhisk`, `wire-in`, `match-comp`, `match-lwhisk`, `match-insert`,
-- `insert-shift`, over `match-remove` and `insert-swap` — and the witness
-- discipline does not stop at the outermost one: a defined function may not
-- head a matchable index, and each auxiliary's result sits in the index of the
-- next one's graph. So "speak the multiplication through its graph" propagates
-- to every operation the composite is assembled from.
--
-- That is a real difference between the two instances and it is what
-- `Gandr.Arity` has to be designed against: an interface that asks only for "a
-- multiplication and its graph" is satisfied cheaply by the linear kit and
-- expensively by this one.
--
-- The graphs are NOT built in this revision, deliberately and on the owner's
-- ruling: the interface extraction is done first, so that what gets built is
-- the graph the interface turns out to need rather than nine graphs built on
-- the assumption that it needs them. What IS built here is the operation, its
-- computational checks, the structural theorem below, and the unit laws — and
-- those unit laws are what MEASURE the cost of not having the graph, which is
-- the input the extraction wanted. The extraction itself, and the list of what
-- this kit still owes it, is `Gandr.Arity.Path`'s header §*What the second
-- instance actually showed*.
--
-- ── WHAT IS PROVED ─────────────────────────────────────────────────────────
-- `verts-graft` — grafting CONCATENATES the vertex listings, in order. This is
-- more than bookkeeping: it says the operation neither duplicates nor drops a
-- vertex, and it pins the composite's vertex ORDER, which is representation
-- content under C3 and therefore the thing `Gandr.Rigid` would later have to
-- reconcile. The redundancy that reconciliation is for is visible right here —
-- grafting three shapes two ways gives the same vertex listing, so if the
-- listings determine the shape then associativity is on the nose, and if they
-- do not then the difference is exactly a section-discipline obligation.
--
-- `graft-idnˡ` and `graft-idnʳ` — the derived unit is a two-sided unit, in
-- general, at the exact price of `UIP Ob` and nothing more. That price is the
-- second half of the finding above and it is measured rather than asserted:
-- `preplug` and `graft` rebuild each vertex's `Append` witness with
-- `append-graph`, so the two sides of each law are the same shape carrying
-- possibly-different witnesses at EQUAL indices. `append-fun` closes the
-- indices free; closing the witnesses is `append-uniq`, whose hypothesis is
-- set-ness of the colours. The listing-algebra unit lemmas underneath pay
-- nothing at all, so the charge is located precisely and is not diffuse.
--
-- Both laws also hold by `refl` at the worked examples, where the indices are
-- closed lists and the witness comparison computes away. So the hypothesis is
-- buying that one comparison and the general statement is not stronger than the
-- concrete one in any other respect.
--
-- Stated on the GRAPH instead, the same laws would pay nothing: the whiskered
-- operand is an existential the relation never commits to, so the unit case
-- picks the operand's own witness and the comparison never arises. That is the
-- sharpest available statement of what the graph buys, and it is a REASON
-- INDEPENDENT of keeping defined functions out of matchable indices — which is
-- the only reason `Gandr.Arity.Path`'s header records.
------------------------------------------------------------------------------

module Gandr.Shape.Graft where

open import Gandr.Shape.Graph
  using (here)
  using (there)
  using (prof)
  using (Append)
  using (nil)
  using (cons)
  using (Insert)
  using (head)
  using (tail)
  using (Match)
  using ([])
  using (_∷_)
  using (Shape)
  using (wires)
  using (node)
  using (append-graph)
  using (append-fun)
  using (append-uniq)
  using (idn-match)
  using (verts)
  using (Vtx)
  using (Arc)
  using (arc)
  using (along)
  using (against)
  using (Walk)
  using (stay)
  using (hop)
  using (opening)
  using (apart)
  using (SimplyConn)
  using (idn)
  using (corolla)
  using (chain)
  using (𝟙)
  using (𝟚)
  using (_≟ˢ_)
  using (_≟⊤_)

open import Axiom.UniquenessOfIdentityProofs
  using (UIP)
  using (module Decidable⇒UIP)
open import Data.Bool.Base
  using (true)
  using (false)
open import Data.List.Properties
  renaming (≡-dec to list-dec)
open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (_++_)
open import Data.Maybe.Base
  using (just)
open import Data.Unit.Base
  using (⊤)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (trans)
  using (cong)
open import Relation.Nullary.Decidable
  using (does)
open import Relation.Nullary.Negation
  using (¬_)

module _ {ℓ} {Ob : Set ℓ} where

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE LISTING ALGEBRA. Five operations on `Insert` and `Match`, none of them
  -- about shapes. They sit beside their consumer rather than in a module of
  -- their own for the reason `Gandr.Arity.Path`'s header gives about the arity
  -- interface: one consumer does not determine an abstraction. If a second
  -- appears they move up beside `Match` itself.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Two nested insertions, exchanged: the same two elements put into the same
  -- list in the other order. The list they meet in is not determined by the
  -- inputs, so it is carried rather than computed.
  record Exchange (x y : Ob) (ds dˣ : List Ob) : Set ℓ where
    constructor exchange
    field
      -- what the list looks like with only one of them in
      mid : List Ob
      -- the element that now goes in second
      outer : Insert Ob y mid dˣ
      -- and the one that now goes in first
      inner : Insert Ob x ds mid

  -- Reindexing an exchange past an element both lists lead with. This exists so
  -- that `insert-swap`'s recursive clause can be an APPLICATION rather than a
  -- `with` on the recursive call: records have eta, so the projections below
  -- compute on any term at all, and the recursive call stays a visible subterm
  -- that a later `cong` can reach. Same rule as `Gandr.Shape.Graph`'s `split`,
  -- and the unit laws are what it is spent on.
  exchange-tail
    : ∀ {x y w ds dˣ}
    → Exchange x y ds dˣ
    → Exchange x y (w ∷ ds) (w ∷ dˣ)
  Exchange.mid (exchange-tail {w} e) = w ∷ Exchange.mid e
  Exchange.outer (exchange-tail e) = tail (Exchange.outer e)
  Exchange.inner (exchange-tail e) = tail (Exchange.inner e)

  insert-swap
    : ∀ {x y ds d dˣ}
    → Insert Ob x d dˣ
    → Insert Ob y ds d
    → Exchange x y ds dˣ
  insert-swap head k = exchange _ (tail k) head
  insert-swap (tail j) head = exchange _ head j
  insert-swap (tail j) (tail k) = exchange-tail (insert-swap j k)

  -- Threading one matched wire through a matching: a new source at `i`, a new
  -- sink at `j`, joined to each other, with every existing pair left alone.
  -- The exchange is what keeps the existing pairs aligned as the new sink
  -- shifts the positions after it.
  match-insert
    : ∀ {x Γ Γˣ Δ Δˣ}
    → Insert Ob x Γ Γˣ
    → Insert Ob x Δ Δˣ
    → Match Ob Γ Δ
    → Match Ob Γˣ Δˣ
  match-insert head j m = j ∷ m
  match-insert (tail i) j (k ∷ m) =
    Exchange.outer (insert-swap j k)
      ∷ match-insert i (Exchange.inner (insert-swap j k)) m

  -- Whiskering a matching by a block of wires on the left. Each element of the
  -- block takes the position beside itself, which is one `head` per element and
  -- no permutation at all.
  match-lwhisk
    : ∀ {A Γ Γ′ Δ Δ′}
    → Append Ob A Γ Γ′
    → Append Ob A Δ Δ′
    → Match Ob Γ Δ
    → Match Ob Γ′ Δ′
  match-lwhisk nil nil m = m
  match-lwhisk (cons p) (cons q) m = head ∷ match-lwhisk p q m

  -- Deleting a source and its partner. What is left of the codomain is not
  -- determined by the inputs, so it is carried.
  record Removed (x : Ob) (Γ Θ : List Ob) : Set ℓ where
    constructor removed
    field
      -- the codomain with the partner taken out
      rest : List Ob
      -- where that partner sat
      spot : Insert Ob x rest Θ
      -- and the matching that survives
      body : Match Ob Γ rest

  -- Lifting a removal past a leading matched pair, on the same footing as
  -- `exchange-tail`: an application rather than a `with`, so the recursion
  -- stays reachable.
  removed-tail
    : ∀ {x y Γ us Θ}
    → Insert Ob y us Θ
    → Removed x Γ us
    → Removed x (y ∷ Γ) Θ
  Removed.rest (removed-tail k r) = Exchange.mid (insert-swap k (Removed.spot r))
  Removed.spot (removed-tail k r) = Exchange.outer (insert-swap k (Removed.spot r))
  Removed.body (removed-tail k r) =
    Exchange.inner (insert-swap k (Removed.spot r)) ∷ Removed.body r

  match-remove
    : ∀ {x Γ Δ Θ}
    → Insert Ob x Γ Δ
    → Match Ob Δ Θ
    → Removed x Γ Θ
  match-remove head (j ∷ n) = removed _ j n
  match-remove (tail i) (k ∷ n) = removed-tail k (match-remove i n)

  -- Composing two matchings: each source takes its partner's partner, and the
  -- rest is the composite of what remains on both sides.
  match-comp
    : ∀ {Γ Δ Θ}
    → Match Ob Γ Δ
    → Match Ob Δ Θ
    → Match Ob Γ Θ
  match-comp [] [] = []
  match-comp (i ∷ m) n =
    Removed.spot (match-remove i n)
      ∷ match-comp m (Removed.body (match-remove i n))

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHISKERING, ONE WIRE AT A TIME. This is the crossing, paid for without a
  -- permutation: `insert-shift` walks a single new position past a published
  -- block by adding one `tail` per element of it.
  -- ══════════════════════════════════════════════════════════════════════════

  insert-shift
    : ∀ {a B Γ Γ′ Γˣ Γˣ′}
    → Append Ob B Γ Γ′
    → Append Ob B Γˣ Γˣ′
    → Insert Ob a Γ Γˣ
    → Insert Ob a Γ′ Γˣ′
  insert-shift nil nil i = i
  insert-shift (cons p) (cons q) i = tail (insert-shift p q i)

  -- Threading one wire straight through a shape: a new input leg at `i`, a new
  -- output leg at `j`, joined to each other and touching no vertex.
  wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → Insert Ob a Γ Γˣ
    → Insert Ob a Δ Δˣ
    → Shape Ob Γ Δ
    → Shape Ob Γˣ Δˣ
  wire-in i j (wires m) = wires (match-insert i j m)
  wire-in {Γˣ} {Δˣ} i j (node A B p q S) =
    node A B (append-graph B Γˣ) (append-graph A Δˣ)
      (wire-in
        (insert-shift p (append-graph B Γˣ) i)
        (insert-shift q (append-graph A Δˣ) j)
        S)

  -- Whiskering a shape by a block of wires on the left, wire by wire.
  lwhisk
    : ∀ {A Γ Γ′ Δ Δ′}
    → Append Ob A Γ Γ′
    → Append Ob A Δ Δ′
    → Shape Ob Γ Δ
    → Shape Ob Γ′ Δ′
  lwhisk nil nil S = S
  lwhisk (cons p) (cons q) S = wire-in head head (lwhisk p q S)

  -- ══════════════════════════════════════════════════════════════════════════
  -- GRAFTING. Recursion on the FIRST operand: each of its vertices is
  -- republished at the front of the composite, and the second operand is
  -- whiskered by that vertex's in-profile — which is the interface change the
  -- vertex causes, and nothing more.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Pre-composing a shape by a pure wiring. No vertex of the wiring exists, so
  -- this is a relabelling: the shape's vertices are kept and only the matching
  -- at the bottom changes.
  preplug
    : ∀ {Γ Δ Θ}
    → Match Ob Γ Δ
    → Shape Ob Δ Θ
    → Shape Ob Γ Θ
  preplug m (wires n) = wires (match-comp m n)
  preplug {Γ} m (node A B p q T) =
    node A B (append-graph B Γ) q
      (preplug (match-lwhisk (append-graph B Γ) p m) T)

  graft
    : ∀ {Γ Δ Θ}
    → Shape Ob Γ Δ
    → Shape Ob Δ Θ
    → Shape Ob Γ Θ
  graft (wires m) T = preplug m T
  graft {Θ} (node A B p q S) T =
    node A B p (append-graph A Θ)
      (graft S (lwhisk q (append-graph A Θ) T))

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT GRAFTING DOES TO THE VERTEX LISTING. The listing is the shape's
  -- primary content, so this is the structural statement available without the
  -- graph relation, and it has teeth: no vertex is duplicated, none is dropped,
  -- and the composite's vertex ORDER is the concatenation rather than any other
  -- interleaving. That order is representation content under C3.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Threading a wire adds no vertex.
  verts-wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (S : Shape Ob Γ Δ)
    → verts (wire-in i j S) ≡ verts S
  verts-wire-in i j (wires m) = refl
  verts-wire-in {Γˣ} {Δˣ} i j (node A B p q S) =
    cong
      (prof A B ∷_)
      (verts-wire-in
        (insert-shift p (append-graph B Γˣ) i)
        (insert-shift q (append-graph A Δˣ) j)
        S)

  -- and neither does whiskering by a block of them
  verts-lwhisk
    : ∀ {A Γ Γ′ Δ Δ′}
    → (p : Append Ob A Γ Γ′)
    → (q : Append Ob A Δ Δ′)
    → (S : Shape Ob Γ Δ)
    → verts (lwhisk p q S) ≡ verts S
  verts-lwhisk nil nil S = refl
  verts-lwhisk (cons p) (cons q) S =
    trans
      (verts-wire-in head head (lwhisk p q S))
      (verts-lwhisk p q S)

  -- nor does pre-composing by a wiring, which has no vertex to contribute
  verts-preplug
    : ∀ {Γ Δ Θ}
    → (m : Match Ob Γ Δ)
    → (T : Shape Ob Δ Θ)
    → verts (preplug m T) ≡ verts T
  verts-preplug m (wires n) = refl
  verts-preplug {Γ} m (node A B p q T) =
    cong
      (prof A B ∷_)
      (verts-preplug (match-lwhisk (append-graph B Γ) p m) T)

  -- THE STATEMENT: grafting concatenates the listings, first operand first.
  verts-graft
    : ∀ {Γ Δ Θ}
    → (S : Shape Ob Γ Δ)
    → (T : Shape Ob Δ Θ)
    → verts (graft S T) ≡ verts S ++ verts T
  verts-graft (wires m) T = verts-preplug m T
  verts-graft {Θ} (node A B p q S) T =
    cong
      (prof A B ∷_)
      (trans
        (verts-graft S (lwhisk q (append-graph A Θ) T))
        (cong (verts S ++_) (verts-lwhisk q (append-graph A Θ) T)))

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE UNIT, ON THE LISTING ALGEBRA. Everything here is h-level free: the
  -- identity matching is a two-sided unit for composition and is fixed by
  -- whiskering, and no witness is ever compared. The h-level condition arrives
  -- only at the shape level, and only for the reason the next section states.
  -- ══════════════════════════════════════════════════════════════════════════

  match-comp-idnˡ
    : ∀ {Γ Θ}
    → (n : Match Ob Γ Θ)
    → match-comp (idn-match Γ) n ≡ n
  match-comp-idnˡ [] = refl
  match-comp-idnˡ (j ∷ n) = cong (j ∷_) (match-comp-idnˡ n)

  -- Removing a source from the identity matching leaves the identity on what
  -- remains, and hands back the very position that was removed.
  match-remove-idn
    : ∀ {x Γ Δ}
    → (i : Insert Ob x Γ Δ)
    → match-remove i (idn-match Δ) ≡ removed Γ i (idn-match Γ)
  match-remove-idn head = refl
  match-remove-idn (tail i) = cong (removed-tail head) (match-remove-idn i)

  match-comp-idnʳ
    : ∀ {Γ Δ}
    → (m : Match Ob Γ Δ)
    → match-comp m (idn-match Δ) ≡ m
  match-comp-idnʳ [] = refl
  match-comp-idnʳ (i ∷ m) =
    trans
      (cong
        (λ r → Removed.spot r ∷ match-comp m (Removed.body r))
        (match-remove-idn i))
      (cong (i ∷_) (match-comp-idnʳ m))

  -- Whiskering the identity matching by a block gives the identity again.
  match-lwhisk-idn
    : ∀ {A Γ Γ′}
    → (p : Append Ob A Γ Γ′)
    → match-lwhisk p p (idn-match Γ) ≡ idn-match Γ′
  match-lwhisk-idn nil = refl
  match-lwhisk-idn (cons p) = cong (head ∷_) (match-lwhisk-idn p)

  -- and so does whiskering the identity SHAPE, since it is a wiring
  lwhisk-idn
    : ∀ {A Δ Δ′}
    → (q : Append Ob A Δ Δ′)
    → lwhisk q q (idn Δ) ≡ idn Δ′
  lwhisk-idn nil = refl
  lwhisk-idn (cons q) = cong (wire-in head head) (lwhisk-idn q)

-- ════════════════════════════════════════════════════════════════════════════
-- THE UNIT LAWS, AND THE EXACT PRICE OF STATING THEM ON THE FUNCTION.
--
-- The derived unit is a two-sided unit for grafting — that is the claim the
-- carrier's `idn` has to make good on, since `Gandr.Arity.Path`'s header named
-- the unit's status as the one thing it expected NOT to generalize.
--
-- Both laws hold, and they cost `UIP Ob` and nothing else. The reason is local
-- and worth naming: `preplug` and `graft` rebuild each vertex's `Append`
-- witness with `append-graph`, so the two sides of each law are the same shape
-- carrying possibly-different witnesses at EQUAL indices. `append-fun` closes
-- the indices with no hypothesis; closing the witnesses is `append-uniq`, whose
-- price is set-ness of the colours. Nothing else in either proof pays anything.
--
-- This is what the same laws stated on the graph of grafting would NOT pay:
-- there the witness is an existential the relation never commits to, so the
-- unit constructor picks the operand's own witness and the question does not
-- arise. That is the sharpest available statement of what the graph is for,
-- and it is a second reason beyond keeping defined functions out of indices.
--
-- The instantiation is at the worked examples below, where both laws hold by
-- `refl` — at concrete indices the witness comparison computes away, so the
-- hypothesis is doing exactly the work the general case needs and no more.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} (uipᵒ : UIP Ob) (uipˡ : UIP (List Ob)) where

  graft-idnˡ
    : ∀ {Γ Θ}
    → (T : Shape Ob Γ Θ)
    → graft (idn Γ) T ≡ T
  graft-idnˡ (wires n) = cong wires (match-comp-idnˡ n)
  graft-idnˡ {Γ} (node A B p q T) with append-fun p (append-graph B Γ)
  ... | refl with append-uniq uipᵒ uipˡ p (append-graph B Γ)
  ...   | refl =
    cong
      (node A B (append-graph B Γ) q)
      (trans
        (cong (λ m → preplug m T) (match-lwhisk-idn (append-graph B Γ)))
        (graft-idnˡ T))

  graft-idnʳ
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → graft S (idn Δ) ≡ S
  graft-idnʳ (wires m) = cong wires (match-comp-idnʳ m)
  graft-idnʳ {Δ} (node A B p q S) with append-fun q (append-graph A Δ)
  ... | refl with append-uniq uipᵒ uipˡ q (append-graph A Δ)
  ...   | refl =
    cong
      (node A B p (append-graph A Δ))
      (trans
        (cong (graft S) (lwhisk-idn (append-graph A Δ)))
        (graft-idnʳ S))

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED CHECKS, at the unit colour set. Each of these RUNS the operation and
-- pins its answer, so a definition that type-checked while computing the wrong
-- composite fails here. This is what stands in for the unit laws, which cannot
-- be stated generally on the function without an h-level condition — see the
-- header.
-- ════════════════════════════════════════════════════════════════════════════

-- Two corollas grafted along one leg: `C(2;1)` feeding `C(1;1)`. This is the
-- worked two-vertex composite, now CONSTRUCTED rather than written out.
chain-grafted : Shape ⊤ 𝟚 𝟙
chain-grafted = graft (corolla 𝟚 𝟙) (corolla 𝟙 𝟙)

-- and it is the hand-written `chain`, on the nose
chain-graft-agrees : does (chain-grafted ≟ˢ chain) ≡ true
chain-graft-agrees = refl

-- The positive check above would be worthless if the decision said `true` of
-- everything at this interface, so here is one it says `false` of: the single
-- corolla spans `𝟚 ⟶ 𝟙` too, and is not the composite.
chain-graft-apart : does (chain-grafted ≟ˢ corolla 𝟚 𝟙) ≡ false
chain-graft-apart = refl

-- The unit laws, at concrete data, hold by `refl`: the witness comparison the
-- general proof spends `UIP Ob` on computes away once the indices are closed
-- lists. So the hypothesis buys exactly that comparison and nothing else.
corolla-idnˡ : graft (idn 𝟚) (corolla 𝟚 𝟙) ≡ corolla 𝟚 𝟙
corolla-idnˡ = refl

corolla-idnʳ : graft (corolla 𝟚 𝟙) (idn 𝟙) ≡ corolla 𝟚 𝟙
corolla-idnʳ = refl

-- and a two-vertex composite past a unit still lands on itself
chain-idnˡ : graft (idn 𝟚) chain ≡ chain
chain-idnˡ = refl

chain-idnʳ : graft chain (idn 𝟙) ≡ chain
chain-idnʳ = refl

-- AND THE GENERAL LAWS ARE DISCHARGED HERE, which is what stops the
-- `UIP`-parameterized module above from being green and vacuous: the unit type
-- has decidable equality, so Hedberg supplies both hypotheses.
uipᵒ : UIP ⊤
uipᵒ = Decidable⇒UIP.≡-irrelevant _≟⊤_

uipˡ : UIP (List ⊤)
uipˡ = Decidable⇒UIP.≡-irrelevant (list-dec _≟⊤_)

graft-unitˡ : (T : Shape ⊤ 𝟚 𝟙) → graft (idn 𝟚) T ≡ T
graft-unitˡ = graft-idnˡ uipᵒ uipˡ

graft-unitʳ : (S : Shape ⊤ 𝟚 𝟙) → graft S (idn 𝟙) ≡ S
graft-unitʳ = graft-idnʳ uipᵒ uipˡ

-- ════════════════════════════════════════════════════════════════════════════
-- GRAFTING IS TOTAL AND DOES NOT PRESERVE THE CELL PREDICATES. That is the
-- design's claim rather than a defect: on graphs, connectivity is a predicate
-- on OBJECTS, so the operation composes any two shapes and the predicate is
-- checked of the result. Here is a composite of two cells that is not one.
-- ════════════════════════════════════════════════════════════════════════════

-- A source with two outputs grafted onto a sink with two inputs: two vertices
-- joined by two parallel edges. It reconverges, so it carries a reduced
-- undirected closed walk — the diamond's failure at two vertices instead of
-- four, reached by grafting rather than written out.
bigon : Shape ⊤ [] []
bigon = graft (corolla [] 𝟚) (corolla 𝟚 [])

-- its two vertices, read off `verts-graft` rather than asserted
bigon-verts : verts bigon ≡ prof [] 𝟚 ∷ prof 𝟚 [] ∷ []
bigon-verts = verts-graft (corolla [] 𝟚) (corolla 𝟚 [])

-- the two vertices, named
g₀ g₁ : Vtx bigon
g₀ = here
g₁ = there here

-- and the two parallel edges, read off the DERIVED incidence — grafting put
-- them there, nothing here asserts them
bigon-arc₀ : Arc bigon here g₀ g₁
bigon-arc₀ = arc refl refl

bigon-arc₁ : Arc bigon (there here) g₀ g₁
bigon-arc₁ = arc refl refl

-- The cycle: out along one edge and back against the other. The two edges are
-- distinct, so the walk is reduced and the composite is not acyclic.
bigon-cycle : Walk bigon g₀ g₀ (just (there here))
bigon-cycle =
  hop (there here)
    (hop here stay (along bigon-arc₀) opening)
    (against bigon-arc₁)
    (apart (λ ()))

-- SO THE GRAFT OF TWO CELLS NEED NOT BE A CELL. Both operands are corollas and
-- both are cells by `corolla-cell`; the composite is not. This is what makes
-- grafting's totality a design claim with content — the operation is defined on
-- all of `Shape`, and `Cell` is cut out of the result afterwards.
bigon-not-simply : ¬ SimplyConn bigon
bigon-not-simply sc = SimplyConn.acyclic sc bigon-cycle
