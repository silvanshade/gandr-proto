{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Graft — the arity OPERATIONS on the cell shape: plugging one
-- shape's output legs into another's input legs, and placing two shapes side
-- by side.
--
-- `Gandr.Shape.Graph` landed the objects and the derived unit; this module
-- lands the multiplication. Together they are the graph-shaped arity kit that
-- `Gandr.Arity` is to be extracted from alongside `Gandr.Arity.Path`.
--
-- The module's NAME is now narrower than its content, and that is recorded
-- rather than fixed: `graft` and `merge` are the two operations of one arity
-- and belong together, so the boundary to draw when a third arrives is
-- "listing algebra" against "shape operations", not "graft" against "merge".
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
-- ── THE SECOND OPERATION: THE MERGER, AND WHY IT IS DERIVED ────────────────
-- `merge` places two shapes side by side, concatenating both interfaces:
--
--   merge : Append Γ₁ Γ₂ Γ → Append Δ₁ Δ₂ Δ → Shape Γ₁ Δ₁ → Shape Γ₂ Δ₂
--         → Shape Γ Δ
--
-- It is stated over `Append` WITNESSES rather than over `_++_` so that no
-- index downstream has to unify against a computed list — the discipline
-- `node` already follows — and it is DERIVED rather than adjoined as a
-- constructor. That choice is load-bearing in the same direction as `idn`
-- being derived: as a constructor, one graph would have many terms, one per
-- merge order and association, and the carrier would stop being canonical.
-- Derived, a merge computes to an ordinary corollas-plus-matching term.
--
-- Only the FIRST operand's vertices peel cleanly, and the asymmetry is the
-- same one grafting has. `node` publishes its ports to the front of the
-- interface, so a vertex of the second operand would have to publish in front
-- of the first operand's whole block — a block crossing, hence a permutation.
-- The first operand's wiring is threaded into the second instead, one position
-- at a time through `wire-in` and `cap-in`, which is `lwhisk`'s technique; and
-- `merge-idn` proves `lwhisk` IS this operation at an identity operand rather
-- than merely resembling it.
--
-- The cut needed one new piece of listing algebra, `match-cap`, standing to
-- `match-insert` as a cut stands to a wire. Everything else was already here.
--
-- ── WHAT IS PROVED ─────────────────────────────────────────────────────────
-- `verts-merge` — merging CONCATENATES the vertex listings too, first operand
-- first, so both operands survive intact and the composite's order is pinned.
-- Two consequences are exhibited rather than asserted: a merge of two cells
-- need not be a cell, and the predicate it loses is CONNECTIVITY (`bigon` is
-- the grafting counterpart, losing acyclicity); and `⊠` is NOT commutative on
-- the nose. The second is not a defect. The source's identification of
-- `⊠(φ,ψ)` with `⊠(ψ,φ)` on closed components is an identification of
-- ISOMORPHISM CLASSES, and gandr's representation is an ordered section rather
-- than the quotient (C3), so that identification is `Gandr.Rigid`'s obligation
-- and stating it as an equation here would be false. `merge-swap-apart`
-- exhibits the failure.
--
-- `cap-swap` — a cut does not care which of its two ports is named first. The
-- content is that it holds by `refl`: `Match` writes a capped pair once, at
-- its earlier port, so the identification is discharged by the representation
-- being canonical rather than imposed on it. It is proved where the wiring
-- underneath is flow-through; the case of a cut over a wiring that already
-- caps needs a three-way `insert-swap` coherence which no consumer has asked
-- for, and that residue is stated at the lemma.
--
-- What the merger still OWES: an incidence theorem. `verts-merge` says what
-- happens to the vertices, and nothing yet says what happens to `origin` and
-- `dest` — so "a merge of two connected shapes has exactly two components" can
-- only be exhibited at concrete data (`two-points`), not proved in general.
-- That is the next lemma of this module, and it is what a general Axis A
-- statement would be built on.
--
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
  using (cap)
  using (CapFree)
  using (Shape)
  using (wires)
  using (node)
  using (append-graph)
  using (append-fun)
  using (append-uniq)
  using (idn-match)
  using (Ix)
  using (Wire)
  using (slot)
  using (past)
  using (split)
  using (right)
  using (split-right)
  using (smap-exch)
  using (Slot)
  using (taken)
  using (spare)
  using (islot)
  using (match-edges)
  using (ends)
  using (pool)
  using (copool)
  using (wiring)
  using (edges)
  using (Edg)
  using (Leg)
  using (route)
  using (end₀)
  using (end₁)
  using (verts)
  using (Vtx)
  using (Attach)
  using (attach)
  using (along)
  using (against)
  using (Walk)
  using (stay)
  using (hop)
  using (opening)
  using (apart)
  using (adj)
  using (Reach)
  using (stop)
  using (onward)
  using (Connected)
  using (SimplyConn)
  using (idn)
  using (corolla)
  using (point)
  using (wheel)
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
  using (length)
open import Data.Nat.Base
  using (ℕ)
  using (suc)
  using (_<_)
  using (s≤s)
open import Data.Nat.Induction
  using (<-wellFounded)
open import Data.Nat.Properties
  using (n<1+n)
  using (<-trans)
open import Induction.WellFounded
  using (Acc)
  using (acc)
open import Data.Maybe.Base
  using (just)
open import Data.Unit.Base
  using (⊤)
  using (tt)
open import Data.Product.Base
  using (_×_)
  using (_,_)
  using (proj₁)
  using (proj₂)
  renaming (map to pmap)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
  renaming ([_,_]′ to case⊎)
  renaming (map to smap)
open import Function.Base
  using (id)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (trans)
  using (cong)
  using (cong₂)
  using (subst)
  using (sym)
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
  match-insert (tail i) j (cap k m) =
    cap (Exchange.outer (insert-swap i k))
      (match-insert (Exchange.inner (insert-swap i k)) j m)

  -- Threading one CAPPED pair through a matching: two new sources, joined to
  -- each other and to no sink, with every existing pair left alone. This is
  -- `match-insert`'s partner — that one adds a wire, this one adds a cut — and
  -- the sinks are untouched because a cap consumes none.
  --
  -- The two positions are given as nested insertions rather than as two
  -- positions in one list, for the reason `Insert` is a relation at all: the
  -- second position is only meaningful once the first element is in.
  match-cap
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → Insert Ob x Γ˘ Γˣ
    → Insert Ob y Γ Γ˘
    → Match Ob Γ Δ
    → Match Ob Γˣ Δ
  -- the capped source is the first one, so it is the cap constructor's own
  -- shape and its partner is the datum
  match-cap head j m = cap j m
  -- and the same cut named the other way round: the PARTNER now comes first,
  -- and the constructor is symmetric in exactly this way — `cap-swap` below
  -- is that observation as an equation
  match-cap (tail i) head m = cap i m
  -- otherwise the leading source keeps its own partner and the cut happens
  -- further along
  match-cap (tail i) (tail j) (k ∷ m) = k ∷ match-cap i j m
  match-cap (tail i) (tail j) (cap k m) =
    cap
      (Exchange.outer
        (insert-swap i (Exchange.outer (insert-swap j k))))
      (match-cap
        (Exchange.inner
          (insert-swap i (Exchange.outer (insert-swap j k))))
        (Exchange.inner (insert-swap j k))
        m)

  -- An insertion re-read after a block is appended on its right. The position
  -- does not move — everything it inserts past is still in front of it — but
  -- the list it inserts into grows, and that longer list is not determined by
  -- the inputs, so it is carried rather than computed.
  record Widened (x : Ob) (ys Ξ Δ : List Ob) : Set ℓ where
    constructor widened
    field
      -- the remainder, with the block appended
      rest : List Ob
      -- the witness that it is that
      keep : Append Ob ys Ξ rest
      -- and the position, read in the extended list
      spot : Insert Ob x rest Δ

  -- Lifting past an element both lists lead with, as an application rather
  -- than a `with` on the recursive call — the same rule as `exchange-tail`.
  widened-tail
    : ∀ {x w ys Ξ Δ}
    → Widened x ys Ξ Δ
    → Widened x (w ∷ ys) Ξ (w ∷ Δ)
  Widened.rest (widened-tail {w} r) = w ∷ Widened.rest r
  Widened.keep (widened-tail r) = cons (Widened.keep r)
  Widened.spot (widened-tail r) = tail (Widened.spot r)

  insert-widen
    : ∀ {x ys zs Ξ Δ}
    → Insert Ob x ys zs
    → Append Ob zs Ξ Δ
    → Widened x ys Ξ Δ
  insert-widen head (cons q) = widened _ q head
  insert-widen (tail i) (cons q) = widened-tail (insert-widen i q)

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT THE TWO THREADING OPERATIONS DO TO THE EDGE LISTING AND TO THE
  -- INCIDENCE. `match-insert` adds one wire and `match-cap` adds one cut, so
  -- each adds exactly one entry to the edge listing and moves no other. This
  -- section says WHERE the entry goes and WHAT its two ends are, which is the
  -- listing-algebra half of the merger's incidence theorem.
  --
  -- The two facts are stated separately for the fresh entry and for the stale
  -- ones because they are used separately: the fresh wire is what the merger
  -- contributes, and the stale ones are what the second operand keeps.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Swapping two nested insertions moves neither element. Three readings of
  -- that, and every reindexing in this section is spent on one of them.
  swap-slotˡ
    : ∀ {x y ds d dˣ}
    → (j : Insert Ob x d dˣ)
    → (k : Insert Ob y ds d)
    → past (Exchange.outer (insert-swap j k))
        (slot (Exchange.inner (insert-swap j k)))
      ≡ slot j
  swap-slotˡ head k = refl
  swap-slotˡ (tail j) head = refl
  swap-slotˡ (tail j) (tail k) = cong there (swap-slotˡ j k)

  swap-slotʳ
    : ∀ {x y ds d dˣ}
    → (j : Insert Ob x d dˣ)
    → (k : Insert Ob y ds d)
    → slot (Exchange.outer (insert-swap j k)) ≡ past j (slot k)
  swap-slotʳ head k = refl
  swap-slotʳ (tail j) head = refl
  swap-slotʳ (tail j) (tail k) = cong there (swap-slotʳ j k)

  swap-past
    : ∀ {x y ds d dˣ}
    → (j : Insert Ob x d dˣ)
    → (k : Insert Ob y ds d)
    → (z : Ix ds)
    → past (Exchange.outer (insert-swap j k))
        (past (Exchange.inner (insert-swap j k)) z)
      ≡ past j (past k z)
  swap-past head k z = refl
  swap-past (tail j) head z = refl
  swap-past (tail j) (tail k) here = refl
  swap-past (tail j) (tail k) (there z) = cong there (swap-past j k z)

  -- Both ends of an edge are reindexed by the same map, so the pair-level
  -- congruence is stated once. Eta for pairs is what makes this enough.
  legs-cong
    : ∀ {a b} {A : Set a} {B : Set b} {f g : A → B}
    → ((z : A) → f z ≡ g z)
    → (x : A × A)
    → (f (proj₁ x) , f (proj₂ x)) ≡ (g (proj₁ x) , g (proj₂ x))
  legs-cong e x = cong₂ _,_ (e (proj₁ x)) (e (proj₂ x))

  -- AN ENTRY THREADED INTO THE EDGE LISTING. The entry itself is carried rather
  -- than computed, because `match-cap` names a cut by whichever of its two
  -- ports comes first and the two namings of one cut differ — which is
  -- `cap-swap`'s observation showing up in the listing.
  record Threaded (es fs : List (Wire Ob)) : Set ℓ where
    constructor threaded
    field
      -- the entry that was added
      wire : Wire Ob
      -- and where it went
      spot : Insert (Wire Ob) wire es fs

  threaded-tail
    : ∀ {w es fs}
    → Threaded es fs
    → Threaded (w ∷ es) (w ∷ fs)
  Threaded.wire (threaded-tail t) = Threaded.wire t
  Threaded.spot (threaded-tail t) = tail (Threaded.spot t)

  -- Threading a wire adds one `flow` entry, at the position the new source
  -- takes in the order the wiring consumes its sources.
  match-insert-edges
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (m : Match Ob Γ Δ)
    → Threaded (match-edges m) (match-edges (match-insert i j m))
  match-insert-edges head j m = threaded _ head
  match-insert-edges (tail i) j (k ∷ m) =
    threaded-tail (match-insert-edges i (Exchange.inner (insert-swap j k)) m)
  match-insert-edges (tail i) j (cap k m) =
    threaded-tail (match-insert-edges (Exchange.inner (insert-swap i k)) j m)

  -- and threading a cut adds one `cut` entry, on the same footing
  match-cap-edges
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Δ)
    → Threaded (match-edges m) (match-edges (match-cap i j m))
  match-cap-edges head j m = threaded _ head
  match-cap-edges (tail i) head m = threaded _ head
  match-cap-edges (tail i) (tail j) (k ∷ m) =
    threaded-tail (match-cap-edges i j m)
  match-cap-edges (tail i) (tail j) (cap k m) =
    threaded-tail
      (match-cap-edges
        (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
        (Exchange.inner (insert-swap j k))
        m)

  -- THE FRESH WIRE'S ENDS are the source and the sink it was given, and
  -- nothing else. This is the statement that would fail if a widening had
  -- moved a position it should not have.
  ends-match-insert
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (m : Match Ob Γ Δ)
    → ends (match-insert i j m)
        (slot (Threaded.spot (match-insert-edges i j m)))
      ≡ (inj₁ (slot i) , inj₂ (slot j))
  ends-match-insert head j m = refl
  ends-match-insert (tail i) j (k ∷ m) =
    trans
      (cong
        (pmap
          (smap there (past (Exchange.outer (insert-swap j k))))
          (smap there (past (Exchange.outer (insert-swap j k)))))
        (ends-match-insert i (Exchange.inner (insert-swap j k)) m))
      (cong (λ z → inj₁ (there (slot i)) , inj₂ z) (swap-slotˡ j k))
  ends-match-insert (tail i) j (cap k m) =
    trans
      (cong
        (pmap
          (smap (λ w → there (past (Exchange.outer (insert-swap i k)) w)) id)
          (smap (λ w → there (past (Exchange.outer (insert-swap i k)) w)) id))
        (ends-match-insert (Exchange.inner (insert-swap i k)) j m))
      (cong (λ z → inj₁ (there z) , inj₂ (slot j)) (swap-slotˡ i k))

  -- and every OTHER wire keeps its ends, read in the extended pools
  ends-match-insert-past
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (m : Match Ob Γ Δ)
    → (e : Ix (match-edges m))
    → ends (match-insert i j m)
        (past (Threaded.spot (match-insert-edges i j m)) e)
      ≡ pmap (smap (past i) (past j)) (smap (past i) (past j)) (ends m e)
  ends-match-insert-past head j m e = refl
  ends-match-insert-past (tail i) j (k ∷ m) here =
    cong (λ z → inj₁ here , inj₂ z) (swap-slotʳ j k)
  ends-match-insert-past (tail i) j (k ∷ m) (there e) =
    trans
      (cong
        (pmap
          (smap there (past (Exchange.outer (insert-swap j k))))
          (smap there (past (Exchange.outer (insert-swap j k)))))
        (ends-match-insert-past i (Exchange.inner (insert-swap j k)) m e))
      (legs-cong step (ends m e))
    where
      step
        : (z : _)
        → smap there (past (Exchange.outer (insert-swap j k)))
            (smap (past i) (past (Exchange.inner (insert-swap j k))) z)
          ≡ smap (past (tail i)) (past j) (smap there (past k) z)
      step (inj₁ w) = refl
      step (inj₂ w) = cong inj₂ (swap-past j k w)
  ends-match-insert-past (tail i) j (cap k m) here =
    cong (λ z → inj₁ here , inj₁ (there z)) (swap-slotʳ i k)
  ends-match-insert-past (tail i) j (cap k m) (there e) =
    trans
      (cong
        (pmap
          (smap (λ w → there (past (Exchange.outer (insert-swap i k)) w)) id)
          (smap (λ w → there (past (Exchange.outer (insert-swap i k)) w)) id))
        (ends-match-insert-past (Exchange.inner (insert-swap i k)) j m e))
      (legs-cong step (ends m e))
    where
      step
        : (z : _)
        → smap (λ w → there (past (Exchange.outer (insert-swap i k)) w)) id
            (smap (past (Exchange.inner (insert-swap i k))) (past j) z)
          ≡ smap (past (tail i)) (past j) (smap (λ w → there (past k w)) id z)
      step (inj₁ w) = cong (λ z → inj₁ (there z)) (swap-past i k w)
      step (inj₂ w) = refl

  -- A CUT'S TWO ENDS, AS AN UNORDERED PAIR. A wire's ends are its source and
  -- its sink and the listing names them in that order; a cut's are two sources
  -- and the listing names them in whichever order they stand in the pool. Both
  -- orders occur — `match-cap (tail i) head` is the clause that flips them —
  -- so this is what the statement below can claim, and it is `cap-swap`'s
  -- observation appearing in the incidence rather than in the wiring.
  data Ends {a} {A : Set a} (u v : A) : A × A → Set a where
    -- named in the pool's own order
    forwards
      : Ends u v (u , v)
    -- and named the other way round, which is the same cut
    backwards
      : Ends u v (v , u)

  ends-map
    : ∀ {a b} {A : Set a} {B : Set b} {u v : A} {x : A × A}
    → (f : A → B)
    → Ends u v x
    → Ends (f u) (f v) (f (proj₁ x) , f (proj₂ x))
  ends-map f forwards = forwards
  ends-map f backwards = backwards

  ends-cast
    : ∀ {a} {A : Set a} {u u′ v v′ : A} {x : A × A}
    → u ≡ u′
    → v ≡ v′
    → Ends u v x
    → Ends u′ v′ x
  ends-cast refl refl e = e

  -- THE FRESH CUT'S ENDS are the two sources it was given — both of them
  -- SOURCES, which is what the cap was added to be able to say, and neither of
  -- them a sink.
  ends-match-cap
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Δ)
    → Ends {A = Leg Γˣ Δ}
        (inj₁ (slot i))
        (inj₁ (past i (slot j)))
        (ends (match-cap i j m) (slot (Threaded.spot (match-cap-edges i j m))))
  ends-match-cap head j m = forwards
  ends-match-cap (tail i) head m = backwards
  ends-match-cap (tail i) (tail j) (k ∷ m) =
    ends-map (smap there (past k)) (ends-match-cap i j m)
  ends-match-cap (tail i) (tail j) (cap k m) =
    ends-cast
      (cong (λ z → inj₁ (there z)) (swap-slotˡ i (Exchange.outer (insert-swap j k))))
      (cong
        (λ z → inj₁ (there z))
        (trans
          (swap-past
            i
            (Exchange.outer (insert-swap j k))
            (slot (Exchange.inner (insert-swap j k))))
          (cong (past i) (swap-slotˡ j k))))
      (ends-map
        (smap
          (λ w →
            there
              (past
                (Exchange.outer
                  (insert-swap i (Exchange.outer (insert-swap j k))))
                w))
          id)
        (ends-match-cap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m))

  -- and every other wire keeps its ends: the cut consumes no sink, so only the
  -- source pool is reindexed, and by both new positions at once
  ends-match-cap-past
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Δ)
    → (e : Ix (match-edges m))
    → ends (match-cap i j m) (past (Threaded.spot (match-cap-edges i j m)) e)
      ≡ pmap
          (smap (λ z → past i (past j z)) id)
          (smap (λ z → past i (past j z)) id)
          (ends m e)
  ends-match-cap-past head j m e = refl
  ends-match-cap-past (tail i) head m e = refl
  ends-match-cap-past (tail i) (tail j) (k ∷ m) here = refl
  ends-match-cap-past (tail i) (tail j) (k ∷ m) (there e) =
    trans
      (cong
        (pmap (smap there (past k)) (smap there (past k)))
        (ends-match-cap-past i j m e))
      (legs-cong step (ends m e))
    where
      step
        : (z : _)
        → smap there (past k) (smap (λ w → past i (past j w)) id z)
          ≡ smap (λ w → past (tail i) (past (tail j) w)) id
              (smap there (past k) z)
      step (inj₁ w) = refl
      step (inj₂ w) = refl
  ends-match-cap-past (tail i) (tail j) (cap k m) here =
    cong
      (λ z → inj₁ here , inj₁ (there z))
      (trans
        (swap-slotʳ i (Exchange.outer (insert-swap j k)))
        (cong (past i) (swap-slotʳ j k)))
  ends-match-cap-past (tail i) (tail j) (cap k m) (there e) =
    trans
      (cong
        (pmap
          (smap
            (λ w →
              there
                (past
                  (Exchange.outer
                    (insert-swap i (Exchange.outer (insert-swap j k))))
                  w))
            id)
          (smap
            (λ w →
              there
                (past
                  (Exchange.outer
                    (insert-swap i (Exchange.outer (insert-swap j k))))
                  w))
            id))
        (ends-match-cap-past
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m
          e))
      (legs-cong step (ends m e))
    where
      step
        : (z : _)
        → smap
            (λ w →
              there
                (past
                  (Exchange.outer
                    (insert-swap i (Exchange.outer (insert-swap j k))))
                  w))
            id
            (smap
              (λ w →
                past
                  (Exchange.inner
                    (insert-swap i (Exchange.outer (insert-swap j k))))
                  (past (Exchange.inner (insert-swap j k)) w))
              id
              z)
          ≡ smap (λ w → past (tail i) (past (tail j) w)) id
              (smap (λ w → there (past k w)) id z)
      step (inj₁ w) =
        cong
          (λ z → inj₁ (there z))
          (trans
            (swap-past
              i
              (Exchange.outer (insert-swap j k))
              (past (Exchange.inner (insert-swap j k)) w))
            (cong (past i) (swap-past j k w)))
      step (inj₂ w) = refl

  -- ══════════════════════════════════════════════════════════════════════════
  -- COMPARING TWO INSERTIONS INTO ONE LIST. Composition with caps has to ask
  -- whether the source it is removing IS the one a cap already took, so the
  -- construction direction (`insert-swap`) is not enough and the ANALYSIS
  -- direction is owed: two insertions into the same list are either the same
  -- slot or two different slots, and in the second case each can be re-read
  -- past the other.
  -- ══════════════════════════════════════════════════════════════════════════

  data InsertView
    : ∀ {x y A B Δ}
    → Insert Ob x A Δ
    → Insert Ob y B Δ
    → Set ℓ where
    -- the same slot, so the insertions are equal and so are their remainders
    same
      : ∀ {x A Δ}
      → {i : Insert Ob x A Δ}
      → InsertView i i
    -- different slots: `C` is the list with BOTH removed, and each insertion
    -- reappears past the other
    apart
      : ∀ {x y A B C Δ}
      → {i : Insert Ob x A Δ}
      → {k : Insert Ob y B Δ}
      → Insert Ob x C B
      → Insert Ob y C A
      → InsertView i k

  insert-view
    : ∀ {x y A B Δ}
    → (i : Insert Ob x A Δ)
    → (k : Insert Ob y B Δ)
    → InsertView i k
  insert-view head head = same
  insert-view head (tail k) = apart head k
  insert-view (tail i) head = apart i head
  insert-view (tail i) (tail k) with insert-view i k
  ... | same = same
  ... | apart i′ k′ = apart (tail i′) (tail k′)

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

  -- Removing a source is no longer single-valued: the source either ran
  -- THROUGH to a sink, as it always did before the cap existed, or it was
  -- CAPPED to another source, in which case both leave together and what is
  -- carried is where the partner sat.
  data Removal (x : Ob) : List Ob → List Ob → Set ℓ where
    through
      : ∀ {Γ Θ rest}
      → Insert Ob x rest Θ
      → Match Ob Γ rest
      → Removal x Γ Θ
    capped
      : ∀ {Γ Γ′ Θ y}
      → Insert Ob y Γ′ Γ
      → Match Ob Γ′ Θ
      → Removal x Γ Θ

  -- lifting a removal past a leading matched pair, on both branches
  removal-tail
    : ∀ {x y Γ us Θ}
    → Insert Ob y us Θ
    → Removal x Γ us
    → Removal x (y ∷ Γ) Θ
  removal-tail k (through spot body) =
    through
      (Exchange.outer (insert-swap k spot))
      (Exchange.inner (insert-swap k spot) ∷ body)
  removal-tail k (capped ins body) = capped (tail ins) (k ∷ body)

  match-remove
    : ∀ {x Γ Δ Θ}
    → Insert Ob x Γ Δ
    → Match Ob Δ Θ
    → Removal x Γ Θ
  match-remove head (j ∷ n) = through j n
  match-remove head (cap k n) = capped k n
  match-remove (tail i) (k ∷ n) = removal-tail k (match-remove i n)
  match-remove (tail i) (cap k n) with insert-view i k
  -- the source being removed IS the one the cap took: it is capped to the
  -- head, which sits at the front of what remains
  ... | same = capped head n
  -- otherwise the cap stands and the removal happens inside it
  ... | apart i′ k′ = removal-recap k′ (match-remove i′ n)
    where
      -- the leading cap is re-applied around whatever the removal left: its
      -- partner still sits in the tail, and if the removal itself capped, that
      -- partner has to be read past this one
      removal-recap
        : ∀ {x y w C Γ₀ Θ}
        → Insert Ob y C Γ₀
        → Removal x C Θ
        → Removal x (w ∷ Γ₀) Θ
      removal-recap j (through spot body) = through spot (cap j body)
      removal-recap j (capped ins body) =
        capped
          (tail (Exchange.outer (insert-swap j ins)))
          (cap (Exchange.inner (insert-swap j ins)) body)

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE INVERSE LOOKUP. Composition needs the other direction too: when the
  -- second wiring caps, the strand that arrives has to be traced BACK through
  -- the first to the source that sent it, and both leave together. Every sink
  -- is hit by exactly one source and only a through-wire reaches a sink, so
  -- this is total and the colour is determined.
  -- ══════════════════════════════════════════════════════════════════════════

  data Unhit (y : Ob) (Γ Δ : List Ob) : Set ℓ where
    unhit
      : ∀ {Γ′}
      → Insert Ob y Γ′ Γ
      → Match Ob Γ′ Δ
      → Unhit y Γ Δ

  -- the two liftings, as applications rather than `with`s on the recursive
  -- call, so a later `cong` can reach through them
  unhit-tail
    : ∀ {y x C Γ Δ}
    → Insert Ob x C Δ
    → Unhit y Γ C
    → Unhit y (x ∷ Γ) Δ
  unhit-tail i (unhit p body) = unhit (tail p) (i ∷ body)

  unhit-cap
    : ∀ {y w z us us′ Δ}
    → Insert Ob z us us′
    → Unhit y us Δ
    → Unhit y (w ∷ us′) Δ
  unhit-cap k (unhit p body) =
    unhit
      (tail (Exchange.outer (insert-swap k p)))
      (cap (Exchange.inner (insert-swap k p)) body)

  match-unhit
    : ∀ {y Γ Δ Δˣ}
    → Insert Ob y Δ Δˣ
    → Match Ob Γ Δˣ
    → Unhit y Γ Δ
  match-unhit j (i ∷ m) with insert-view j i
  ... | same = unhit head m
  ... | apart j′ i′ = unhit-tail i′ (match-unhit j′ m)
  match-unhit j (cap k m) = unhit-cap k (match-unhit j m)

  -- Removing a sink from the identity hands back the position it was given,
  -- and leaves the identity on what remains.
  match-unhit-idn
    : ∀ {y Γ Δ}
    → (j : Insert Ob y Γ Δ)
    → match-unhit j (idn-match Δ) ≡ unhit j (idn-match Γ)
  match-unhit-idn head = refl
  match-unhit-idn (tail j) = cong (unhit-tail head) (match-unhit-idn j)

  -- the source list shrinks by one per insertion, which is the measure the
  -- composition below recurses on
  insert-length
    : ∀ {x ys zs}
    → Insert Ob x ys zs
    → length zs ≡ suc (length ys)
  insert-length head = refl
  insert-length (tail i) = cong suc (insert-length i)

  -- ══════════════════════════════════════════════════════════════════════════
  -- COMPOSITION. Three ways a source can leave: capped already by the first
  -- wiring, run through both, or run through the first into a cap of the
  -- second — and the third is the one the cap introduced, where two through
  -- strands fuse into a cap of the composite. That last case recurses on a
  -- matching produced by `match-unhit` rather than on a subterm, so the
  -- recursion is well-founded on the length of the source list rather than
  -- structural.
  -- ══════════════════════════════════════════════════════════════════════════

  match-comp-acc
    : ∀ {Γ Δ Θ}
    → Acc _<_ (length Γ)
    → Match Ob Γ Δ
    → Match Ob Δ Θ
    → Match Ob Γ Θ
  match-comp-acc a [] [] = []
  match-comp-acc (acc rec) (cap {xs = xs} i m) n =
    cap i
      (match-comp-acc
        (rec
          (subst
            (λ z → length xs < suc z)
            (sym (insert-length i))
            (<-trans (n<1+n _) (n<1+n _))))
        m
        n)
  match-comp-acc (acc rec) (_∷_ {xs = xs} i m) n with match-remove i n
  ... | through spot body = spot ∷ match-comp-acc (rec (n<1+n _)) m body
  ... | capped ins body with match-unhit ins m
  ...   | unhit {Γ′ = xs′} p m′ =
    cap p
      (match-comp-acc
        (rec
          (subst
            (λ z → length xs′ < suc z)
            (sym (insert-length p))
            (<-trans (n<1+n _) (n<1+n _))))
        m′
        body)

  -- Composing two matchings: each source takes its partner's partner, and the
  -- rest is the composite of what remains on both sides.
  match-comp
    : ∀ {Γ Δ Θ}
    → Match Ob Γ Δ
    → Match Ob Δ Θ
    → Match Ob Γ Θ
  match-comp {Γ} = match-comp-acc (<-wellFounded (length Γ))

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

  -- Threading one CUT through a shape: two new input legs, capped to each
  -- other and touching no vertex. `wire-in`'s partner, and the only place the
  -- cap enters a shape from outside.
  --
  -- Both positions shift past each published block exactly as a wire's do —
  -- one `tail` per element, no permutation — because a cut is two legs and a
  -- leg is a leg. That the cap costs no more than a wire here is the concrete
  -- form of the claim that the boundary `∩` is a wiring notion.
  cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → Insert Ob x Γ˘ Γˣ
    → Insert Ob y Γ Γ˘
    → Shape Ob Γ Δ
    → Shape Ob Γˣ Δ
  cap-in i j (wires m) = wires (match-cap i j m)
  cap-in {Γ˘} {Γˣ} i j (node A B p q S) =
    node A B (append-graph B Γˣ) q
      (cap-in
        (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
        (insert-shift p (append-graph B Γ˘) j)
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
  -- THE MERGER `⊠`. The arity's SECOND operation: two shapes side by side,
  -- their interfaces concatenated. Grafting composes along a shared interface;
  -- merging shares nothing, and the composite is disconnected exactly when its
  -- operands do not already reach each other.
  --
  -- It is DERIVED rather than a constructor, and that is the load-bearing
  -- choice. As a constructor the same graph would have one term per merge
  -- order and association, so the carrier would stop being canonical and
  -- `Gandr.Rigid` would inherit a quotient it does not have today. Derived, a
  -- merge computes to an ordinary corollas-plus-matching term — the one the
  -- listings already name — and `verts-merge` below says which one.
  --
  -- The interfaces are related by `Append` WITNESSES rather than by `_++_`, so
  -- no index anywhere downstream has to unify against a computed list. That is
  -- the same discipline `node` itself follows.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Re-associating two concatenations. A vertex publishes its ports to the
  -- front of ITS operand's interface; in the merged shape it must publish to
  -- the front of the whole one, and what the recursive merge is then stated
  -- over is the other bracketing. The two readings are the same list, and this
  -- carries both witnesses that say so without ever writing `_++_`.
  record Regroup (B Γ′ Ξ Γ : List Ob) : Set ℓ where
    constructor regroup
    field
      -- the interface the recursive merge spans
      whole : List Ob
      -- the block, published to the MERGED interface
      front : Append Ob B Γ whole
      -- and the operand's own extended interface, still a prefix of it
      back : Append Ob Γ′ Ξ whole

  -- the lifting, applied rather than `with`-ed, as everywhere else here
  regroup-cons
    : ∀ {b B Γ′ Ξ Γ}
    → Regroup B Γ′ Ξ Γ
    → Regroup (b ∷ B) (b ∷ Γ′) Ξ Γ
  Regroup.whole (regroup-cons {b} r) = b ∷ Regroup.whole r
  Regroup.front (regroup-cons r) = cons (Regroup.front r)
  Regroup.back (regroup-cons r) = cons (Regroup.back r)

  append-regroup
    : ∀ {B Γ₁ Γ′ Ξ Γ}
    → Append Ob B Γ₁ Γ′
    → Append Ob Γ₁ Ξ Γ
    → Regroup B Γ′ Ξ Γ
  append-regroup nil r = regroup _ nil r
  append-regroup (cons p) r = regroup-cons (append-regroup p r)

  -- Merging a pure WIRING into a shape: the wiring has no vertex, so this
  -- threads its wires and its cuts into the second operand one at a time and
  -- keeps that operand's vertices untouched.
  --
  -- One at a time is what avoids a permutation. Placing the whole block `Γ₁`
  -- in front of `Γ₂` would have to commute that block past every published
  -- port block on the way in; a single position commutes past a block by
  -- `insert-shift`, which adds one `tail` per element and permutes nothing.
  -- This is `lwhisk`'s technique, generalized from identity wires to an
  -- arbitrary wiring — and `merge-idn` below proves that it IS the
  -- generalization, not merely an analogue.
  wires-in
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → Append Ob Γ₁ Γ₂ Γ
    → Append Ob Δ₁ Δ₂ Δ
    → Match Ob Γ₁ Δ₁
    → Shape Ob Γ₂ Δ₂
    → Shape Ob Γ Δ
  wires-in nil nil [] T = T
  wires-in (cons p) q (i ∷ m) T =
    wire-in
      head
      (Widened.spot (insert-widen i q))
      (wires-in p (Widened.keep (insert-widen i q)) m T)
  wires-in (cons p) q (cap j m) T =
    cap-in
      head
      (Widened.spot (insert-widen j p))
      (wires-in (Widened.keep (insert-widen j p)) q m T)

  -- THE MERGER. Recursion on the FIRST operand: each of its vertices is
  -- republished at the front of the MERGED interface, which is the only thing
  -- that changes, and the second operand is untouched until the first runs out
  -- of vertices.
  --
  -- Only the first operand's vertices peel cleanly. A vertex of the SECOND
  -- would have to publish its ports in front of the first operand's whole
  -- interface, which is the block crossing, so the second operand is never
  -- recursed on — `wires-in` threads the first operand's wiring into it
  -- instead, and pays for the crossing one position at a time.
  merge
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → Append Ob Γ₁ Γ₂ Γ
    → Append Ob Δ₁ Δ₂ Δ
    → Shape Ob Γ₁ Δ₁
    → Shape Ob Γ₂ Δ₂
    → Shape Ob Γ Δ
  merge p q (wires m) T = wires-in p q m T
  merge p q (node A B p₁ q₁ S) T =
    node A B
      (Regroup.front (append-regroup p₁ p))
      (Regroup.front (append-regroup q₁ q))
      (merge
        (Regroup.back (append-regroup p₁ p))
        (Regroup.back (append-regroup q₁ q))
        S
        T)

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

  -- and neither does threading a cut, for the same reason
  verts-cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (S : Shape Ob Γ Δ)
    → verts (cap-in i j S) ≡ verts S
  verts-cap-in i j (wires m) = refl
  verts-cap-in {Γ˘} {Γˣ} i j (node A B p q S) =
    cong
      (prof A B ∷_)
      (verts-cap-in
        (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
        (insert-shift p (append-graph B Γ˘) j)
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

  -- AND THE SAME STATEMENT FOR THE MERGER, which is what makes Axis A a
  -- theorem rather than a discipline: the two operands' vertices both survive,
  -- neither is duplicated, and the composite's order is the concatenation —
  -- first operand first, exactly as for grafting.
  --
  -- The order is the content. `verts (merge p q S T) ≡ verts T ++ verts S` is
  -- FALSE, and `merge-swap-apart` at the bottom of this file exhibits a pair
  -- it fails on. So the merger is not commutative on the nose, and it must not
  -- be: under C3 the vertex order is representation content, so commutativity
  -- of `⊠` is a `Gandr.Rigid` obligation and not an equation here.
  verts-wires-in
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → verts (wires-in p q m T) ≡ verts T
  verts-wires-in nil nil [] T = refl
  verts-wires-in (cons p) q (i ∷ m) T =
    trans
      (verts-wire-in
        head
        (Widened.spot (insert-widen i q))
        (wires-in p (Widened.keep (insert-widen i q)) m T))
      (verts-wires-in p (Widened.keep (insert-widen i q)) m T)
  verts-wires-in (cons p) q (cap j m) T =
    trans
      (verts-cap-in
        head
        (Widened.spot (insert-widen j p))
        (wires-in (Widened.keep (insert-widen j p)) q m T))
      (verts-wires-in (Widened.keep (insert-widen j p)) q m T)

  verts-merge
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → verts (merge p q S T) ≡ verts S ++ verts T
  verts-merge p q (wires m) T = verts-wires-in p q m T
  verts-merge p q (node A B p₁ q₁ S) T =
    cong
      (prof A B ∷_)
      (verts-merge
        (Regroup.back (append-regroup p₁ p))
        (Regroup.back (append-regroup q₁ q))
        S
        T)

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE UNIT, ON THE LISTING ALGEBRA. Everything here is h-level free: the
  -- identity matching is a two-sided unit for composition and is fixed by
  -- whiskering, and no witness is ever compared. The h-level condition arrives
  -- only at the shape level, and only for the reason the next section states.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Both unit laws are proved over an ARBITRARY accessibility witness and then
  -- instantiated, because two witnesses for the same measure are equal only
  -- propositionally: a proof that fixed one could not be applied under the
  -- other, which is the standard price of leaving structural recursion.
  match-comp-acc-idnˡ
    : ∀ {Γ Θ}
    → (a : Acc _<_ (length Γ))
    → (n : Match Ob Γ Θ)
    → match-comp-acc a (idn-match Γ) n ≡ n
  match-comp-acc-idnˡ a [] = refl
  match-comp-acc-idnˡ (acc rec) (j ∷ n) =
    cong (j ∷_) (match-comp-acc-idnˡ (rec (n<1+n _)) n)
  match-comp-acc-idnˡ (acc rec) (cap k n)
    with match-unhit k (idn-match _) | match-unhit-idn k
  ... | .(unhit k (idn-match _)) | refl =
    cong (cap k) (match-comp-acc-idnˡ _ n)

  match-comp-idnˡ
    : ∀ {Γ Θ}
    → (n : Match Ob Γ Θ)
    → match-comp (idn-match Γ) n ≡ n
  match-comp-idnˡ n = match-comp-acc-idnˡ _ n

  -- Removing a source from the identity matching leaves the identity on what
  -- remains, and hands back the very position that was removed.
  match-remove-idn
    : ∀ {x Γ Δ}
    → (i : Insert Ob x Γ Δ)
    → match-remove i (idn-match Δ) ≡ through i (idn-match Γ)
  match-remove-idn head = refl
  match-remove-idn (tail i) = cong (removal-tail head) (match-remove-idn i)

  match-comp-acc-idnʳ
    : ∀ {Γ Δ}
    → (a : Acc _<_ (length Γ))
    → (m : Match Ob Γ Δ)
    → match-comp-acc a m (idn-match Δ) ≡ m
  match-comp-acc-idnʳ a [] = refl
  match-comp-acc-idnʳ (acc rec) (i ∷ m)
    with match-remove i (idn-match _) | match-remove-idn i
  ... | .(through i (idn-match _)) | refl =
    cong (i ∷_) (match-comp-acc-idnʳ (rec (n<1+n _)) m)
  match-comp-acc-idnʳ (acc rec) (cap k m) =
    cong (cap k) (match-comp-acc-idnʳ _ m)

  match-comp-idnʳ
    : ∀ {Γ Δ}
    → (m : Match Ob Γ Δ)
    → match-comp m (idn-match Δ) ≡ m
  match-comp-idnʳ m = match-comp-acc-idnʳ _ m

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

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT THE MERGER SUBSUMES, AND WHAT IT LEAVES TO `Gandr.Rigid`. Both facts
  -- are h-level free — no witness is compared in either — which is why they
  -- sit here rather than below.
  -- ══════════════════════════════════════════════════════════════════════════

  -- WHISKERING IS THE MERGER AT AN IDENTITY OPERAND, on the nose. `lwhisk` was
  -- written for grafting, before there was a merger to state it as; this says
  -- the two agree definitionally clause for clause, so nothing has been
  -- duplicated and the older operation is the special case it looked like.
  merge-idn
    : ∀ {A Γ Γ′ Δ Δ′}
    → (p : Append Ob A Γ Γ′)
    → (q : Append Ob A Δ Δ′)
    → (T : Shape Ob Γ Δ)
    → merge p q (idn A) T ≡ lwhisk p q T
  merge-idn nil nil T = refl
  merge-idn (cons p) (cons q) T = cong (wire-in head head) (merge-idn p q T)

  -- THE CONTRACTION SWAP `ζ_{x,y} = ζ_{y,x}`, as an equation. A cut does not
  -- care which of its two ports is named first: capping `x` at the outer slot
  -- to `y` at the inner one is the same TERM as capping `y` outer to `x`
  -- inner, where "the same two slots, in the other order" is what
  -- `insert-swap` computes.
  --
  -- The first two clauses are the content and hold by `refl`, because `Match`
  -- presents a cut canonically — the capped pair is written once, at its
  -- earlier port, so there is no second term to identify. That is the same
  -- reason the pre-cap wiring had one term per bijection, and it means the
  -- identification is discharged by the representation rather than imposed on
  -- it.
  --
  -- SCOPE, stated rather than assumed: the wiring underneath is required to be
  -- flow-through. The remaining case — a cut over a wiring that ALREADY caps —
  -- needs the three-way coherence of `insert-swap` (two ports and the existing
  -- cap's partner, pushed past each other in two orders, whose intermediate
  -- lists differ), which is a hexagon this file does not have and which no
  -- consumer has yet asked for. It is an owed lemma, not a wall.
  cap-swap
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → {m : Match Ob Γ Δ}
    → CapFree m
    → match-cap i j m
        ≡ match-cap
            (Exchange.outer (insert-swap i j))
            (Exchange.inner (insert-swap i j))
            m
  cap-swap head j c = refl
  cap-swap (tail i) head c = refl
  cap-swap (tail i) (tail j) (k ∷ c) = cong (k ∷_) (cap-swap i j c)

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
bigon-att₀ : Attach bigon here g₀ g₁
bigon-att₀ = attach refl refl

bigon-att₁ : Attach bigon (there here) g₀ g₁
bigon-att₁ = attach refl refl

-- The cycle: out along one edge and back against the other. The two edges are
-- distinct, so the walk is reduced and the composite is not acyclic.
bigon-cycle : Walk bigon g₀ g₀ (just (there here))
bigon-cycle =
  hop (there here)
    (hop here stay (along bigon-att₀) opening)
    (against bigon-att₁)
    (apart (λ ()))

-- SO THE GRAFT OF TWO CELLS NEED NOT BE A CELL. Both operands are corollas and
-- both are cells by `corolla-cell`; the composite is not. This is what makes
-- grafting's totality a design claim with content — the operation is defined on
-- all of `Shape`, and `Cell` is cut out of the result afterwards.
bigon-not-simply : ¬ SimplyConn bigon
bigon-not-simply sc = SimplyConn.acyclic sc bigon-cycle

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED CHECKS FOR THE MERGER. Same discipline as the grafting checks above:
-- each of these RUNS the operation, so a definition that type-checked while
-- computing the wrong composite fails here.
--
-- Three things are checked and they are the three claims the merger makes:
-- the composite carries BOTH operands' vertices in order; the composite is
-- DISCONNECTED when the operands do not reach each other, which is Axis A
-- becoming an object of the theory rather than a discipline outside it; and
-- the merger is NOT commutative on the nose, which is the section discipline
-- showing up exactly where the source says it must.
-- ════════════════════════════════════════════════════════════════════════════

-- FIRST, THE WIRING ITSELF, pinned on both constructors. These are the checks
-- that would fail if the positions were widened wrongly — the types alone do
-- not see it, because over one colour every wiring at an interface has the
-- same type.
--
-- A CROSSING beside a plain wire. Source `0` takes sink `1` and source `1`
-- takes sink `0`, and the merged-in wire takes the sink beside itself: the two
-- operands do not interleave, and the crossing is not disturbed by the
-- widening that moves its sink positions past the second operand's block.
cross-merge
  : merge
      (append-graph 𝟚 𝟙)
      (append-graph 𝟚 𝟙)
      (wires (tail head ∷ head ∷ []))
      (idn 𝟙)
    ≡ wires (tail head ∷ head ∷ head ∷ [])
cross-merge = refl

-- A CUT beside a plain wire, which is the same check on the other constructor:
-- the cap keeps its partner and the wire keeps its sink.
cut-wire-merge
  : merge (append-graph 𝟚 𝟙) nil (wires (cap head [])) (idn 𝟙)
    ≡ wires (cap head (head ∷ []))
cut-wire-merge = refl

-- Two isolated vertices — the smallest disconnected shape, built rather than
-- written out. Neither operand has a port, so the composite has no edge at
-- all and its two components are visibly separate.
two-points : Shape ⊤ [] []
two-points = merge nil nil point point

-- both vertices, in order, off `verts-merge` rather than asserted
two-points-verts : verts two-points ≡ prof [] [] ∷ prof [] [] ∷ []
two-points-verts = verts-merge nil nil point point

-- the two vertices, named
p₀ p₁ : Vtx two-points
p₀ = here
p₁ = there here

p₀≢p₁ : ¬ (p₀ ≡ p₁)
p₀≢p₁ ()

-- There is no edge, so there is no adjacency, so reachability is equality.
two-points-still : ∀ {u v} → Reach two-points u v → u ≡ v
two-points-still stop = refl
two-points-still (onward r (adj () _))

-- SO THE MERGE OF TWO CELLS NEED NOT BE A CELL, and the failing predicate is
-- the OTHER one this time: `bigon` above is a graft that loses acyclicity;
-- this is a merge that loses connectivity. Both operands are cells by
-- `point-cell`. Disconnection is now something the substrate can say.
two-points-disconnected : ¬ Connected two-points
two-points-disconnected c =
  p₀≢p₁
    (trans
      (sym (two-points-still (Connected.span c p₀)))
      (two-points-still (Connected.span c p₁)))

two-points-not-simply : ¬ SimplyConn two-points
two-points-not-simply sc = two-points-disconnected (SimplyConn.connected sc)

-- THE MERGER IS NOT COMMUTATIVE ON THE NOSE. `⊠(φ,ψ) = ⊠(ψ,φ)` holds of the
-- ISOMORPHISM CLASSES the source's monad takes, and the two sides here differ
-- by exactly that isomorphism — the swap of two closed components. gandr's
-- representation is an ordered section and not the quotient (C3), so the
-- decision procedure sees the difference and reports it. This identification
-- is therefore `Gandr.Rigid`'s obligation, not an equation on `merge`.
merge-swap-apart
  : does (merge nil nil point wheel ≟ˢ merge nil nil wheel point) ≡ false
merge-swap-apart = refl

-- and the negative check is not vacuous: the composite is the second operand
-- with the first operand's vertex republished in front of it
merge-point-wheel : merge nil nil point wheel ≡ node [] [] nil nil wheel
merge-point-wheel = refl

-- A merge at a NONTRIVIAL interface: `C(2;1) ⊠ C(1;1)`, spanning the two
-- interfaces concatenated. This is the shape Axis A keeps outside the
-- substrate today — two independent redexes side by side — as one object.
corollas-apart : Shape ⊤ (tt ∷ tt ∷ tt ∷ []) 𝟚
corollas-apart =
  merge (append-graph 𝟚 𝟙) (append-graph 𝟙 𝟙) (corolla 𝟚 𝟙) (corolla 𝟙 𝟙)

corollas-apart-verts : verts corollas-apart ≡ prof 𝟚 𝟙 ∷ prof 𝟙 𝟙 ∷ []
corollas-apart-verts =
  verts-merge (append-graph 𝟚 𝟙) (append-graph 𝟙 𝟙) (corolla 𝟚 𝟙) (corolla 𝟙 𝟙)

-- The same two corollas merged the other way round span the SAME interface —
-- `𝟙 ++ 𝟚` and `𝟚 ++ 𝟙` are one list — and are a different shape. So
-- non-commutativity is not an artifact of the closed case above; it is the
-- vertex order, which is representation content everywhere.
corollas-swapped : Shape ⊤ (tt ∷ tt ∷ tt ∷ []) 𝟚
corollas-swapped =
  merge (append-graph 𝟙 𝟚) (append-graph 𝟙 𝟙) (corolla 𝟙 𝟙) (corolla 𝟚 𝟙)

corollas-swap-apart : does (corollas-apart ≟ˢ corollas-swapped) ≡ false
corollas-swap-apart = refl

-- MERGING A CUT. The left operand is the vertexless `Shape 𝟚 []` — gandr's
-- own cut — and the composite has three sources against one sink, an
-- imbalance no cap-free wiring can span. So inhabiting this type is itself the
-- check that the cap survived the merge; `cap-in` is what carried it.
cut-merge : Shape ⊤ (tt ∷ tt ∷ tt ∷ []) 𝟙
cut-merge =
  merge (append-graph 𝟚 𝟙) nil (wires (cap head [])) (corolla 𝟙 𝟙)

-- the wiring operand contributes no vertex, as a wiring never does
cut-merge-verts : verts cut-merge ≡ prof 𝟙 𝟙 ∷ []
cut-merge-verts =
  verts-merge (append-graph 𝟚 𝟙) nil (wires (cap head [])) (corolla 𝟙 𝟙)

-- and the composite is not the vertexless cut at the same interface, which is
-- the other inhabitant of this imbalanced type
cut-merge-apart : does (cut-merge ≟ˢ wires (cap head (head ∷ []))) ≡ false
cut-merge-apart = refl
