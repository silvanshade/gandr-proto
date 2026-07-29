{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

-- `--guardedness` is here for the REASONING VOCABULARY and for nothing else:
-- `Gandr.Setoid` sits over the coinductive ∞-graph carrier and the flag is
-- infective, so every module that reasons acquires it. That is the accepted
-- trade — ONE reasoning vocabulary everywhere is worth more than one flag
-- saved, since a second style is a standing invitation to a third.
--
-- What the flag does NOT license is reshaping anything to avoid it. After the
-- role split `…/Properties` and `…/Structure` will carry it for exactly this
-- reason; only `…/Base`, which proves nothing, comes out free of it, and that
-- is a precision worth taking because it is free there rather than a boundary
-- worth moving a definition for.

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
-- `cap-swap` — a cut does not care which of its two ports is named first, over
-- ANY wiring. The content of the base cases is that they hold by `refl`:
-- `Match` writes a capped pair once, at its earlier port, so the
-- identification is discharged by the representation being canonical rather
-- than imposed on it. The case of a cut over a wiring that ALREADY caps is
-- the three-way coherence of `insert-swap`, and that is now stated and proved
-- as the listing algebra's own law — see the next paragraph.
--
-- THE LISTING ALGEBRA IS SYMMETRIC, and saying so is what closed `cap-swap`.
-- `insert-swap-invol` says swapping two nested positions twice returns them,
-- and `insert-swap-braid` says a three-layer tower reversed low-high-low and
-- reversed high-low-high agree — the braid relation `σ₁σ₂σ₁ = σ₂σ₁σ₂`, which
-- with the involution is the SYMMETRIC group's presentation on a tower's two
-- generators. An earlier revision called the missing piece a hexagon; it is
-- not one, since a hexagon is the braiding against an associator and there is
-- no tensor here to associate. Naming the structure is what turned the open
-- case from a chase into one `cong`, and `Tower` — three insertions with their
-- intermediate lists as FIELDS — is what let the coherence be stated as a
-- homogeneous equation instead of a transport.
--
-- `merge-apart` — the merger's INCIDENCE theorem, and the general form of what
-- `two-points` could previously only exhibit. `verts-merge` says both operands'
-- vertices survive; this says no edge of the composite runs from one operand's
-- vertex to the other's, so a merge of two shapes that each have a vertex is
-- DISCONNECTED, whatever their interfaces and whatever their wirings do.
--
-- The proof carries a SIDE. Every vertex and every leg of the composite belongs
-- to one operand or the other, and the content is that a wire's two ends always
-- agree about which. Reading the side off a leg is `split`'s job; reading it off
-- a vertex is a recursion on the first operand, since a wiring contributes no
-- vertex. What the argument rests on is that the two threading operations meet
-- no vertex, and that is where the work is: a threaded wire has to pass every
-- vertex's published ports on the way in, and a position shifted past a
-- published block never lands in the block.
--
-- `merge-components` — the converse half, and the two together. Each operand's
-- own adjacencies also SURVIVE into the composite, so reachability in a merge of
-- two connected shapes is exactly agreement of the side: no walk crosses, and
-- every pair on one side is joined. That is "exactly two components, and they
-- are the operands", and neither half implies the other.
--
-- The two halves of the converse are not symmetric, and the asymmetry is the
-- merger's own. The SECOND operand's edges are the ones the threading left
-- alone, so its half is `ends-wire-in-past` and `ends-cap-in-past` composed down
-- the recursion. The FIRST operand's are the FRESH edges the threading added, so
-- its half relates a threaded wire's ends to that wire's ends in the operand —
-- and there a cut's two ends are an unordered pair, so what is true is a
-- statement about `Link`, which is symmetric, and not one about the ordered
-- incidence. `ends-link` is where that is spent.
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

open import Gandr.Setoid
  using (≡ˢ)
  using (bundle)
  using (step-≈·)
  using (step-≈⁻¹)
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
  using (left)
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
  using (step-out)
  using (end₀)
  using (end₁)
  using (incid)
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
  using (Link)
  using (link-sym)
  using (Reach)
  using (stop)
  using (onward)
  using (reach-any)
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
open import Data.Empty
  using (⊥-elim)
open import Data.Empty.Polymorphic
  using ()
  renaming (⊥ to ⊥°)
open import Data.Bool.Base
  using (Bool)
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
  using (n≤1+n)
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

-- The reasoning vocabulary for this module's multi-step equational arguments.
-- It is the tree's own, unchanged: `Reasoning.MultiSetoid` over a bundle, with
-- `Gandr.Profunctor.Yoneda` as the worked example. `bundle (≡ˢ _)` is the
-- `Set`-level bundle — the discrete setoid on the identity type — which is
-- what every structure in this module presents, so a chain here reads exactly
-- as a chain over a category's hom-setoid does.
open import Relation.Binary.Reasoning.MultiSetoid

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

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE EXCHANGE'S OWN LAWS: THE LISTING ALGEBRA IS SYMMETRIC. What
  -- `insert-swap` is, is a SYMMETRY on towers of nested insertions — the
  -- symmetric group acting on a tower's LAYERS — and the two laws here are
  -- that characterization rather than a description of it:
  --
  --   * `insert-swap-invol` — swapping twice returns the two positions it was
  --     given. This is what separates a SYMMETRIC structure from a merely
  --     braided one, and it is what lets `cap-swap` be an equation between the
  --     two namings of ONE cut rather than a statement about an orbit.
  --   * `insert-swap-braid` — the three-way coherence: three nested insertions
  --     reversed by swapping low, high, low agree with the same three reversed
  --     high, low, high.
  --
  -- The three readings in the incidence section below — `swap-slotˡ`,
  -- `swap-slotʳ`, `swap-past` — complete the picture from the other side: the
  -- action moves no ELEMENT, only the order in which the positions were named.
  --
  -- ON THE NAME, because this coherence has been called a hexagon and it is
  -- not one. A hexagon is the braiding against an ASSOCIATOR, and there is no
  -- tensor here to associate: inserting is not a monoidal product on lists.
  -- What the three-way coherence is, is the BRAID RELATION `σ₁σ₂σ₁ = σ₂σ₁σ₂` —
  -- the Yang–Baxter equation — for the family `insert-swap`; and together with
  -- the involution it is the SYMMETRIC group's presentation on the two
  -- generators of a three-layer tower, not the braid group's. The obligation
  -- has the shape the hexagon reading predicted, which is why establishing the
  -- frame first was worth it; only the name was wrong.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Swapping twice returns the two positions it was given. Stated over
  -- `Exchange` rather than as two equations, so the intermediate list is
  -- compared ALONG WITH the positions instead of being fixed in advance —
  -- which is what makes this a homogeneous equation and not a transport.
  insert-swap-invol
    : ∀ {x y ds d dˣ}
    → (i : Insert Ob x d dˣ)
    → (j : Insert Ob y ds d)
    → insert-swap
        (Exchange.outer (insert-swap i j))
        (Exchange.inner (insert-swap i j))
      ≡ exchange d i j
  insert-swap-invol head j = refl
  insert-swap-invol (tail i) head = refl
  insert-swap-invol (tail i) (tail j) =
    cong exchange-tail (insert-swap-invol i j)

  -- Three nested insertions as ONE package. The two intermediate lists are
  -- FIELDS rather than indices — `Exchange`'s device one dimension up — and
  -- that is what makes two towers comparable by an ordinary equation: the
  -- lists a reversal passes through are not determined by its inputs, so a
  -- statement that pinned them as indices could not be homogeneous, and the
  -- coherence below would need a transport before it could be stated at all.
  record Tower (x y z : Ob) (ds dˣ : List Ob) : Set ℓ where
    constructor tower
    field
      -- the list with the first element in
      low : List Ob
      -- and with the second one in as well
      high : List Ob
      -- the element that goes in first
      base : Insert Ob x ds low
      -- the one that goes in second
      step : Insert Ob y low high
      -- and the one that goes in last
      peak : Insert Ob z high dˣ

  -- Reindexing a tower past an element every list leads with: `exchange-tail`
  -- one dimension up, and copatterns for the same reason — the reversals below
  -- have to stay visible subterms of each other under `tail`.
  tower-tail
    : ∀ {x y z w ds dˣ}
    → Tower x y z ds dˣ
    → Tower x y z (w ∷ ds) (w ∷ dˣ)
  Tower.low (tower-tail {w} t) = w ∷ Tower.low t
  Tower.high (tower-tail {w} t) = w ∷ Tower.high t
  Tower.base (tower-tail t) = tail (Tower.base t)
  Tower.step (tower-tail t) = tail (Tower.step t)
  Tower.peak (tower-tail t) = tail (Tower.peak t)

  -- REVERSING A TOWER, LOW PAIR FIRST. `k` puts `z` in, then `j` puts `y` in,
  -- then `i` puts `x` in; this hands back the same three positions with the
  -- order of naming reversed, reached by swapping the low pair, then the high
  -- pair, then the low pair again.
  tower-lo
    : ∀ {x y z ds d d˘ dˣ}
    → (i : Insert Ob x d˘ dˣ)
    → (j : Insert Ob y d d˘)
    → (k : Insert Ob z ds d)
    → Tower x y z ds dˣ
  tower-lo {ds} {d˘} {dˣ} i j k =
    tower
      (Exchange.mid lo₃)
      (Exchange.mid lo₂)
      (Exchange.inner lo₃)
      (Exchange.outer lo₃)
      (Exchange.outer lo₂)
    where
      lo₁ : Exchange _ _ ds d˘
      lo₁ = insert-swap j k
      lo₂ : Exchange _ _ (Exchange.mid lo₁) dˣ
      lo₂ = insert-swap i (Exchange.outer lo₁)
      lo₃ : Exchange _ _ ds (Exchange.mid lo₂)
      lo₃ = insert-swap (Exchange.inner lo₂) (Exchange.inner lo₁)

  -- and the same reversal reached the other way round, swapping the high pair
  -- first. The two routes are the two sides of the braid relation.
  tower-hi
    : ∀ {x y z ds d d˘ dˣ}
    → (i : Insert Ob x d˘ dˣ)
    → (j : Insert Ob y d d˘)
    → (k : Insert Ob z ds d)
    → Tower x y z ds dˣ
  tower-hi {ds} {d} {dˣ} i j k =
    tower
      (Exchange.mid hi₂)
      (Exchange.mid hi₃)
      (Exchange.inner hi₂)
      (Exchange.inner hi₃)
      (Exchange.outer hi₃)
    where
      hi₁ : Exchange _ _ d dˣ
      hi₁ = insert-swap i j
      hi₂ : Exchange _ _ ds (Exchange.mid hi₁)
      hi₂ = insert-swap (Exchange.inner hi₁) k
      hi₃ : Exchange _ _ (Exchange.mid hi₂) dˣ
      hi₃ = insert-swap (Exchange.outer hi₁) (Exchange.outer hi₂)

  -- THE BRAID RELATION. The two reversals agree, layer for layer and list for
  -- list. Three of the four cases hold by `refl` — the reversal is forced as
  -- soon as any of the three positions is at the front — and the fourth is the
  -- recursion under `tail`, so the whole coherence costs one `cong`.
  --
  -- The `refl` at `(tail i) (tail j) head` is worth naming: it is eta for
  -- `Exchange` doing the work, since the low route reaches the tower as an
  -- `insert-swap` and the high route reaches it as an explicitly built
  -- `exchange` of that same swap's three components.
  insert-swap-braid
    : ∀ {x y z ds d d˘ dˣ}
    → (i : Insert Ob x d˘ dˣ)
    → (j : Insert Ob y d d˘)
    → (k : Insert Ob z ds d)
    → tower-lo i j k ≡ tower-hi i j k
  insert-swap-braid head j k = refl
  insert-swap-braid (tail i) head k = refl
  insert-swap-braid (tail i) (tail j) head = refl
  insert-swap-braid (tail i) (tail j) (tail k) =
    cong tower-tail (insert-swap-braid i j k)

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

  -- THE VIEW'S TWO VALUES, NAMED. `insert-view` is the only `with` in the
  -- listing algebra that is not on a recursive call, and a proof cannot see
  -- past it either: every fact about `match-remove` or `match-unhit` at a cap
  -- has to say which verdict the view returned. These two say it at the only
  -- positions the algebra ever compares, so no consumer re-derives them.

  -- A position compared with itself is the same slot.
  insert-view-refl
    : ∀ {x A Δ}
    → (i : Insert Ob x A Δ)
    → insert-view i i ≡ same
  insert-view-refl head = refl
  insert-view-refl (tail i) with insert-view i i | insert-view-refl i
  ... | .same | refl = refl

  -- And a position compared with one read past it is apart, with the exchange
  -- supplying both halves of the verdict. This is the ANALYSIS direction
  -- agreeing with the CONSTRUCTION direction: `insert-swap` builds the two
  -- slots in the other order, and `insert-view` recovers exactly those.
  insert-view-swap
    : ∀ {x y ds d dˣ}
    → (i : Insert Ob x d dˣ)
    → (k : Insert Ob y ds d)
    → insert-view i (Exchange.outer (insert-swap i k))
      ≡ apart (Exchange.inner (insert-swap i k)) k
  insert-view-swap head k = refl
  insert-view-swap (tail i) head = refl
  insert-view-swap (tail i) (tail k)
    with insert-view i (Exchange.outer (insert-swap i k)) | insert-view-swap i k
  ... | .(apart (Exchange.inner (insert-swap i k)) k) | refl = refl

  -- And the same fact read the other way round: an apart verdict IS an
  -- exchange. `insert-view-swap` says the analysis agrees with a construction
  -- that was already given; this says every analysis comes from one, so a
  -- proof that split on the view can go back to `insert-swap` and use its laws.
  insert-view-apart-swap
    : ∀ {x y A B C Δ}
    → (i : Insert Ob x A Δ)
    → (k : Insert Ob y B Δ)
    → (i′ : Insert Ob x C B)
    → (k′ : Insert Ob y C A)
    → insert-view i k ≡ apart i′ k′
    → insert-swap i k′ ≡ exchange B k i′
  insert-view-apart-swap head head i′ k′ ()
  insert-view-apart-swap head (tail k) .head .k refl = refl
  insert-view-apart-swap (tail i) head .i .head refl = refl
  insert-view-apart-swap (tail i) (tail k) i′ k′ e with insert-view i k in v | e
  ... | same | ()
  ... | apart i₀ k₀ | refl =
    cong exchange-tail (insert-view-apart-swap i k i₀ k₀ v)

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

  -- Re-applying a leading cap around whatever a removal left. Its partner
  -- still sits in the tail, and if the removal itself capped, that partner has
  -- to be read past this one. Top-level rather than local to the clause that
  -- uses it, because every fact about `match-remove` at a cap is stated
  -- through it.
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

  -- THE TWO CAP CLAUSES, RESTATED SO A PROOF CAN REACH THEM. The clause above
  -- is a `with` on `insert-view`, so its right-hand side is an auxiliary no
  -- lemma can name — the same wall a `with` on a recursive call builds, from
  -- the same cause. These two say what the clause does once the verdict is
  -- known, taking the verdict as an ARGUMENT with its defining equation.
  match-remove-cap-same
    : ∀ {x w A Δ Θ}
    → (i : Insert Ob x A Δ)
    → (n : Match Ob A Θ)
    → match-remove (tail {y = w} i) (cap i n) ≡ capped head n
  match-remove-cap-same i n with insert-view i i | insert-view-refl i
  ... | .same | refl = refl

  match-remove-cap-apart
    : ∀ {x y w A B C Δ Θ}
    → (i : Insert Ob x A Δ)
    → (k : Insert Ob y B Δ)
    → (n : Match Ob B Θ)
    → (i′ : Insert Ob x C B)
    → (k′ : Insert Ob y C A)
    → insert-view i k ≡ apart i′ k′
    → match-remove (tail {y = w} i) (cap k n) ≡ removal-recap k′ (match-remove i′ n)
  match-remove-cap-apart i k n i′ k′ e with insert-view i k | e
  ... | .(apart i′ k′) | refl = refl

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

  -- and the same two, for the same reason
  match-unhit-∷-same
    : ∀ {y xs ys Δˣ}
    → (j : Insert Ob y ys Δˣ)
    → (m : Match Ob xs ys)
    → match-unhit j (j ∷ m) ≡ unhit head m
  match-unhit-∷-same j m with insert-view j j | insert-view-refl j
  ... | .same | refl = refl

  match-unhit-∷-apart
    : ∀ {x y xs ys C Δ Δˣ}
    → (j : Insert Ob y Δ Δˣ)
    → (i : Insert Ob x ys Δˣ)
    → (m : Match Ob xs ys)
    → (j′ : Insert Ob y C ys)
    → (i′ : Insert Ob x C Δ)
    → insert-view j i ≡ apart j′ i′
    → match-unhit j (i ∷ m) ≡ unhit-tail i′ (match-unhit j′ m)
  match-unhit-∷-apart j i m j′ i′ e with insert-view j i | e
  ... | .(apart j′ i′) | refl = refl

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE TWO LOOKUPS ARE VIEWS, NOT SUMMARIES — and that is the characterization
  -- the rest of this file leans on.
  --
  -- `match-insert` threads a wire and `match-cap` threads a cut; those are the
  -- CONSTRUCTION direction, and `match-remove` / `match-unhit` are the ANALYSIS
  -- direction, one for a marked source and one for a marked sink. Each pair is
  -- INVERSE:
  --
  --   Removal x Γ Θ  ≅  Match (Γ with x at i) Θ        at a fixed source `i`
  --   Unhit y Γ Δ    ≅  Match Γ (Δ with y at j)        at a fixed sink `j`
  --
  -- Both directions are proved below. What that buys is the ability to answer
  -- "what does an operation do to a matching, given what one lookup returned"
  -- by REBUILDING the matching from the lookup and computing — which is how
  -- every fact about composition against a lookup is reached, since the lookup
  -- itself is defined by recursion and the rebuild is not.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Putting a removed source back where it came from.
  removal→match
    : ∀ {x Γ Γˣ Θ}
    → Insert Ob x Γ Γˣ
    → Removal x Γ Θ
    → Match Ob Γˣ Θ
  removal→match i (through spot body) = match-insert i spot body
  removal→match i (capped ins body) = match-cap i ins body

  -- and putting a traced-back sink back where it came from
  unhit→match
    : ∀ {y Γ Δ Δˣ}
    → Insert Ob y Δ Δˣ
    → Unhit y Γ Δ
    → Match Ob Γ Δˣ
  unhit→match j (unhit p body) = match-insert p j body

  -- ── ANALYSIS AFTER CONSTRUCTION ────────────────────────────────────────────

  -- Threading a wire and then removing its source hands back exactly what was
  -- threaded. The recursive clause is the exchange's INVOLUTION: the position
  -- pair `match-insert` built is read back by `match-remove` in the other
  -- order, and `insert-swap-invol` is the statement that this returns them.
  match-remove-insert
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (m : Match Ob Γ Δ)
    → match-remove i (match-insert i j m) ≡ through j m
  match-remove-insert head j m = refl
  match-remove-insert (tail i) j (k ∷ m) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ r ↦ removal-tail (Exchange.outer (insert-swap j k)) r ]·
        match-remove i (match-insert i (Exchange.inner (insert-swap j k)) m)
    ≈·⟨ match-remove-insert i (Exchange.inner (insert-swap j k)) m ⟩
      [ e ↦ through (Exchange.outer e) (Exchange.inner e ∷ m) ]·
        insert-swap (Exchange.outer (insert-swap j k)) (Exchange.inner (insert-swap j k))
    ≈·⟨ insert-swap-invol j k ⟩
      through j (k ∷ m)
    ∎
  match-remove-insert (tail i) j (cap k m) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-remove
        (tail i)
        (cap
          (Exchange.outer (insert-swap i k))
          (match-insert (Exchange.inner (insert-swap i k)) j m))
    ≈⟨ match-remove-cap-apart i
         (Exchange.outer (insert-swap i k))
         (match-insert (Exchange.inner (insert-swap i k)) j m)
         (Exchange.inner (insert-swap i k))
         k
         (insert-view-swap i k) ⟩
      [ r ↦ removal-recap k r ]·
        match-remove
          (Exchange.inner (insert-swap i k))
          (match-insert (Exchange.inner (insert-swap i k)) j m)
    ≈·⟨ match-remove-insert (Exchange.inner (insert-swap i k)) j m ⟩
      through j (cap k m)
    ∎

  -- Threading a cut and then removing one of its two sources hands back the
  -- other one and the matching underneath.
  match-remove-cut
    : ∀ {x y Γ Γ˘ Γˣ Θ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Θ)
    → match-remove i (match-cap i j m) ≡ capped j m
  match-remove-cut head j m = refl
  match-remove-cut (tail i) head m = match-remove-cap-same i m
  match-remove-cut (tail i) (tail j) (k ∷ m) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ r ↦ removal-tail k r ]· match-remove i (match-cap i j m)
    ≈·⟨ match-remove-cut i j m ⟩
      capped (tail j) (k ∷ m)
    ∎
  match-remove-cut (tail i) (tail j) (cap k m) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-remove
        (tail i)
        (cap
          (Exchange.outer (insert-swap i (Exchange.outer (insert-swap j k))))
          (match-cap
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
            (Exchange.inner (insert-swap j k))
            m))
    ≈⟨ match-remove-cap-apart i
         (Exchange.outer (insert-swap i (Exchange.outer (insert-swap j k))))
         (match-cap
           (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
           (Exchange.inner (insert-swap j k))
           m)
         (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
         (Exchange.outer (insert-swap j k))
         (insert-view-swap i (Exchange.outer (insert-swap j k))) ⟩
      [ r ↦ removal-recap (Exchange.outer (insert-swap j k)) r ]·
        match-remove
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (match-cap
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
            (Exchange.inner (insert-swap j k))
            m)
    ≈·⟨ match-remove-cut
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m ⟩
      [ e ↦ capped (tail (Exchange.outer e)) (cap (Exchange.inner e) m) ]·
        insert-swap (Exchange.outer (insert-swap j k)) (Exchange.inner (insert-swap j k))
    ≈·⟨ insert-swap-invol j k ⟩
      capped (tail j) (cap k m)
    ∎

  -- Threading a wire and then tracing its SINK back hands back the source it
  -- was threaded at. The mirror of `match-remove-insert`, and the exchange
  -- laws it uses are the same two.
  match-unhit-insert
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (m : Match Ob Γ Δ)
    → match-unhit j (match-insert i j m) ≡ unhit i m
  match-unhit-insert head j m = match-unhit-∷-same j m
  match-unhit-insert (tail i) j (k ∷ m) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-unhit
        j
        (Exchange.outer (insert-swap j k)
          ∷ match-insert i (Exchange.inner (insert-swap j k)) m)
    ≈⟨ match-unhit-∷-apart j
         (Exchange.outer (insert-swap j k))
         (match-insert i (Exchange.inner (insert-swap j k)) m)
         (Exchange.inner (insert-swap j k))
         k
         (insert-view-swap j k) ⟩
      [ u ↦ unhit-tail k u ]·
        match-unhit
          (Exchange.inner (insert-swap j k))
          (match-insert i (Exchange.inner (insert-swap j k)) m)
    ≈·⟨ match-unhit-insert i (Exchange.inner (insert-swap j k)) m ⟩
      unhit (tail i) (k ∷ m)
    ∎
  match-unhit-insert (tail i) j (cap k m) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ unhit-cap (Exchange.outer (insert-swap i k)) u ]·
        match-unhit j (match-insert (Exchange.inner (insert-swap i k)) j m)
    ≈·⟨ match-unhit-insert (Exchange.inner (insert-swap i k)) j m ⟩
      [ e ↦ unhit (tail (Exchange.outer e)) (cap (Exchange.inner e) m) ]·
        insert-swap (Exchange.outer (insert-swap i k)) (Exchange.inner (insert-swap i k))
    ≈·⟨ insert-swap-invol i k ⟩
      unhit (tail i) (cap k m)
    ∎

  -- The three above, packaged as the two retractions they are. With the
  -- recovery below, each is an inverse and the pair is the isomorphism this
  -- section claims.
  match-remove-roundtrip
    : ∀ {x Γ Γˣ Θ}
    → (i : Insert Ob x Γ Γˣ)
    → (r : Removal x Γ Θ)
    → match-remove i (removal→match i r) ≡ r
  match-remove-roundtrip i (through spot body) = match-remove-insert i spot body
  match-remove-roundtrip i (capped ins body) = match-remove-cut i ins body

  match-unhit-roundtrip
    : ∀ {y Γ Δ Δˣ}
    → (j : Insert Ob y Δ Δˣ)
    → (u : Unhit y Γ Δ)
    → match-unhit j (unhit→match j u) ≡ u
  match-unhit-roundtrip j (unhit p body) = match-unhit-insert p j body

  -- ── CONSTRUCTION AFTER ANALYSIS ────────────────────────────────────────────
  --
  -- The other round trip, and the one that does the work below: a lookup loses
  -- nothing, so the matching can be REBUILT from it. Every fact of the form
  -- "what does an operation do to `o`, given what a lookup on `o` returned" is
  -- reached by rebuilding `o` and computing, because the rebuild is a
  -- construction and the lookup is a recursion.
  --
  -- The hypothesis is oriented `r ≡ match-remove i o`, against the unfolding
  -- lemmas' own direction. That is deliberate: a rebuild consumes the lookup's
  -- value on its LEFT — `r` is refined by matching, and the chain then reads
  -- from the rebuilt term towards `o` — so the other orientation would put a
  -- reversed congruence in every step.
  mutual

    match-remove-recover
      : ∀ {x Γ Γˣ Θ}
      → (i : Insert Ob x Γ Γˣ)
      → (o : Match Ob Γˣ Θ)
      → (r : Removal x Γ Θ)
      → r ≡ match-remove i o
      → removal→match i r ≡ o
    match-remove-recover head (j ∷ o) .(through j o) refl = refl
    match-remove-recover head (cap k o) .(capped k o) refl = refl
    match-remove-recover (tail i) (k ∷ o) .(removal-tail k (match-remove i o)) refl =
      recover-∷ i k o (match-remove i o) refl
    match-remove-recover (tail i) (cap c o) r eq =
      recover-cap i c o (insert-view i c) refl r eq

    -- the removed source ran past a matched pair, which the rebuild re-threads
    recover-∷
      : ∀ {x w Γ Γˣ ys Θ}
      → (i : Insert Ob x Γ Γˣ)
      → (k : Insert Ob w ys Θ)
      → (o : Match Ob Γˣ ys)
      → (r : Removal x Γ ys)
      → r ≡ match-remove i o
      → removal→match (tail i) (removal-tail k r) ≡ k ∷ o
    recover-∷ i k o (through s b) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ e ↦ Exchange.outer e ∷ match-insert i (Exchange.inner e) b ]·
          insert-swap
            (Exchange.outer (insert-swap k s))
            (Exchange.inner (insert-swap k s))
      ≈·⟨ insert-swap-invol k s ⟩
        [ m ↦ k ∷ m ]· match-insert i s b
      ≈·⟨ match-remove-recover i o (through s b) eq ⟩
        k ∷ o
      ∎
    recover-∷ i k o (capped ins b) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ m ↦ k ∷ m ]· match-cap i ins b
      ≈·⟨ match-remove-recover i o (capped ins b) eq ⟩
        k ∷ o
      ∎

    -- or it met a cap, and the verdict decides whether the cap was its own
    recover-cap
      : ∀ {x y w A B Γˣ Θ}
      → (i : Insert Ob x A Γˣ)
      → (c : Insert Ob y B Γˣ)
      → (o : Match Ob B Θ)
      → (v : InsertView i c)
      → insert-view i c ≡ v
      → (r : Removal x (w ∷ A) Θ)
      → r ≡ match-remove (tail {y = w} i) (cap c o)
      → removal→match (tail i) r ≡ cap c o
    recover-cap i .i o same _ r eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ r₀ ↦ removal→match (tail i) r₀ ]· r
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             r
           ≈⟨ eq ⟩
             match-remove (tail i) (cap i o)
           ≈⟨ match-remove-cap-same i o ⟩
             capped head o
           ∎) ⟩
        cap i o
      ∎
    recover-cap i c o (apart i′ c′) ev r eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ r₀ ↦ removal→match (tail i) r₀ ]· r
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             r
           ≈⟨ eq ⟩
             match-remove (tail i) (cap c o)
           ≈⟨ match-remove-cap-apart i c o i′ c′ ev ⟩
             removal-recap c′ (match-remove i′ o)
           ∎) ⟩
        removal→match (tail i) (removal-recap c′ (match-remove i′ o))
      ≈⟨ recover-recap i c o i′ c′ ev (match-remove i′ o) refl ⟩
        cap c o
      ∎

    -- and re-applying that cap around the rebuild is where the exchange's own
    -- two laws are spent: the involution to read the recapped pair back, and
    -- the apart verdict as an exchange to put the cap where it came from
    recover-recap
      : ∀ {x y w A B C Γˣ Θ}
      → (i : Insert Ob x A Γˣ)
      → (c : Insert Ob y B Γˣ)
      → (o : Match Ob B Θ)
      → (i′ : Insert Ob x C B)
      → (c′ : Insert Ob y C A)
      → insert-view i c ≡ apart i′ c′
      → (r : Removal x C Θ)
      → r ≡ match-remove i′ o
      → removal→match (tail {y = w} i) (removal-recap c′ r) ≡ cap c o
    recover-recap i c o i′ c′ ev (through s b) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ e ↦ cap (Exchange.outer e) (match-insert (Exchange.inner e) s b) ]·
          insert-swap i c′
      ≈·⟨ insert-view-apart-swap i c i′ c′ ev ⟩
        [ m ↦ cap c m ]· match-insert i′ s b
      ≈·⟨ match-remove-recover i′ o (through s b) eq ⟩
        cap c o
      ∎
    recover-recap i c o i′ c′ ev (capped ins b) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ e ↦ cap
                (Exchange.outer (insert-swap i (Exchange.outer e)))
                (match-cap
                  (Exchange.inner (insert-swap i (Exchange.outer e)))
                  (Exchange.inner e)
                  b) ]·
          insert-swap
            (Exchange.outer (insert-swap c′ ins))
            (Exchange.inner (insert-swap c′ ins))
      ≈·⟨ insert-swap-invol c′ ins ⟩
        [ e ↦ cap (Exchange.outer e) (match-cap (Exchange.inner e) ins b) ]·
          insert-swap i c′
      ≈·⟨ insert-view-apart-swap i c i′ c′ ev ⟩
        [ m ↦ cap c m ]· match-cap i′ ins b
      ≈·⟨ match-remove-recover i′ o (capped ins b) eq ⟩
        cap c o
      ∎

  -- and the same for the inverse lookup, whose single constructor makes it
  -- shorter by exactly the case analysis `Removal` needs
  mutual

    match-unhit-recover
      : ∀ {y Γ Δ Δˣ}
      → (j : Insert Ob y Δ Δˣ)
      → (m : Match Ob Γ Δˣ)
      → (u : Unhit y Γ Δ)
      → u ≡ match-unhit j m
      → unhit→match j u ≡ m
    match-unhit-recover j (i ∷ m) u eq =
      recover-hit j i m (insert-view j i) refl u eq
    match-unhit-recover j (cap c m) .(unhit-cap c (match-unhit j m)) refl =
      recover-uncap j c m (match-unhit j m) refl

    recover-hit
      : ∀ {x y xs ys Δ Δˣ}
      → (j : Insert Ob y Δ Δˣ)
      → (i : Insert Ob x ys Δˣ)
      → (m : Match Ob xs ys)
      → (v : InsertView j i)
      → insert-view j i ≡ v
      → (u : Unhit y (x ∷ xs) Δ)
      → u ≡ match-unhit j (i ∷ m)
      → unhit→match j u ≡ i ∷ m
    recover-hit j .j m same _ u eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ u₀ ↦ unhit→match j u₀ ]· u
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             u
           ≈⟨ eq ⟩
             match-unhit j (j ∷ m)
           ≈⟨ match-unhit-∷-same j m ⟩
             unhit head m
           ∎) ⟩
        j ∷ m
      ∎
    recover-hit j i m (apart j′ i′) ev u eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ u₀ ↦ unhit→match j u₀ ]· u
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             u
           ≈⟨ eq ⟩
             match-unhit j (i ∷ m)
           ≈⟨ match-unhit-∷-apart j i m j′ i′ ev ⟩
             unhit-tail i′ (match-unhit j′ m)
           ∎) ⟩
        unhit→match j (unhit-tail i′ (match-unhit j′ m))
      ≈⟨ recover-untail j i m j′ i′ ev (match-unhit j′ m) refl ⟩
        i ∷ m
      ∎

    recover-untail
      : ∀ {x y xs ys C Δ Δˣ}
      → (j : Insert Ob y Δ Δˣ)
      → (i : Insert Ob x ys Δˣ)
      → (m : Match Ob xs ys)
      → (j′ : Insert Ob y C ys)
      → (i′ : Insert Ob x C Δ)
      → insert-view j i ≡ apart j′ i′
      → (u : Unhit y xs C)
      → u ≡ match-unhit j′ m
      → unhit→match j (unhit-tail i′ u) ≡ i ∷ m
    recover-untail j i m j′ i′ ev (unhit p t) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ e ↦ Exchange.outer e ∷ match-insert p (Exchange.inner e) t ]· insert-swap j i′
      ≈·⟨ insert-view-apart-swap j i j′ i′ ev ⟩
        [ m₀ ↦ i ∷ m₀ ]· match-insert p j′ t
      ≈·⟨ match-unhit-recover j′ m (unhit p t) eq ⟩
        i ∷ m
      ∎

    recover-uncap
      : ∀ {y z w xs xs′ Δ Δˣ}
      → (j : Insert Ob y Δ Δˣ)
      → (c : Insert Ob z xs xs′)
      → (m : Match Ob xs Δˣ)
      → (u : Unhit y xs Δ)
      → u ≡ match-unhit j m
      → unhit→match j (unhit-cap {w = w} c u) ≡ cap {x = w} c m
    recover-uncap j c m (unhit p t) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ e ↦ cap (Exchange.outer e) (match-insert (Exchange.inner e) j t) ]·
          insert-swap (Exchange.outer (insert-swap c p)) (Exchange.inner (insert-swap c p))
      ≈·⟨ insert-swap-invol c p ⟩
        [ m₀ ↦ cap c m₀ ]· match-insert p j t
      ≈·⟨ match-unhit-recover j m (unhit p t) eq ⟩
        cap c m
      ∎

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

  -- AND THE MEASURE STEP ITSELF, NAMED. Every recursion below that descends
  -- through an insertion needs exactly this, and it is an ordinary induction on
  -- the insertion rather than arithmetic over `insert-length`: transporting the
  -- length equation and then chaining two `n<1+n`s is three lines of noise at
  -- five sites, and it says less than the one line it replaces.
  insert-shrink
    : ∀ {x ys zs}
    → Insert Ob x ys zs
    → length ys < suc (length zs)
  insert-shrink head = s≤s (n≤1+n _)
  insert-shrink (tail i) = s≤s (insert-shrink i)

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
  match-comp-acc (acc rec) (cap i m) n =
    cap i (match-comp-acc (rec (insert-shrink i)) m n)
  match-comp-acc (acc rec) (i ∷ m) n with match-remove i n
  ... | through spot body = spot ∷ match-comp-acc (rec (n<1+n _)) m body
  ... | capped ins body with match-unhit ins m
  ...   | unhit p m′ = cap p (match-comp-acc (rec (insert-shrink p)) m′ body)

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
  -- WHAT THE THREADING OPERATIONS DO TO THE INCIDENCE. `verts-wire-in` says
  -- they add no vertex; this says what they do to the edges' ENDS, which is the
  -- shape-level half of the merger's incidence theorem.
  --
  -- Everything is stated about `incid` — both ends at once — because both are
  -- routed outward by the same node step and reindexed by the same maps, so
  -- one statement carries what two would say twice. The node step is where the
  -- work is: a position shifted past a published block never lands IN the
  -- block, so a threaded wire touches no vertex, and the positions it did not
  -- take split exactly as they did before, so nothing else moves.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Shifting a position past a published block leaves it where it was, read in
  -- the longer list.
  slot-shift
    : ∀ {a B Γ Γ′ Γˣ Γˣ′}
    → (p : Append Ob B Γ Γ′)
    → (r : Append Ob B Γˣ Γˣ′)
    → (i : Insert Ob a Γ Γˣ)
    → slot (insert-shift p r i) ≡ right r (slot i)
  slot-shift nil nil i = refl
  slot-shift (cons p) (cons r) i = cong there (slot-shift p r i)

  -- so it never lands in the block — which is why a threaded wire meets no
  -- vertex, however many vertices it is threaded past
  split-shift
    : ∀ {a B Γ Γ′ Γˣ Γˣ′}
    → (p : Append Ob B Γ Γ′)
    → (r : Append Ob B Γˣ Γˣ′)
    → (i : Insert Ob a Γ Γˣ)
    → split r (slot (insert-shift p r i)) ≡ inj₂ (slot i)
  split-shift p r i =
    trans (cong (split r) (slot-shift p r i)) (split-right r (slot i))

  -- and the positions it did not take are split exactly as they were
  split-shift-past
    : ∀ {a B Γ Γ′ Γˣ Γˣ′}
    → (p : Append Ob B Γ Γ′)
    → (r : Append Ob B Γˣ Γˣ′)
    → (i : Insert Ob a Γ Γˣ)
    → (z : Ix Γ′)
    → split r (past (insert-shift p r i) z) ≡ smap id (past i) (split p z)
  split-shift-past nil nil i z = refl
  split-shift-past (cons p) (cons r) i here = refl
  split-shift-past (cons p) (cons r) i (there z) =
    trans
      (cong (smap there id) (split-shift-past p r i z))
      (smap-exch there (past i) (split p z))

  -- Threading a wire keeps every vertex, in place. This is `verts-wire-in`'s
  -- content as a map rather than as an equation, which is what a statement
  -- about the incidence needs: the ends are positions, and a position has to be
  -- carried across by a function rather than transported along a list equality.
  vtx-wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (U : Shape Ob Γ Δ)
    → Vtx U
    → Vtx (wire-in i j U)
  vtx-wire-in i j (wires m) ()
  vtx-wire-in i j (node A B p q U) here = here
  vtx-wire-in {Γˣ} {Δˣ} i j (node A B p q U) (there v) =
    there
      (vtx-wire-in
        (insert-shift p (append-graph B Γˣ) i)
        (insert-shift q (append-graph A Δˣ) j)
        U
        v)

  vtx-cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (U : Shape Ob Γ Δ)
    → Vtx U
    → Vtx (cap-in i j U)
  vtx-cap-in i j (wires m) ()
  vtx-cap-in i j (node A B p q U) here = here
  vtx-cap-in {Γ˘} {Γˣ} i j (node A B p q U) (there v) =
    there
      (vtx-cap-in
        (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
        (insert-shift p (append-graph B Γ˘) j)
        U
        v)

  -- and each adds its one entry to the shape's edge listing, at the position
  -- the listing algebra already located. Nodes contribute no edge, so this is
  -- the wiring's own answer carried outward unchanged.
  edge-wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (U : Shape Ob Γ Δ)
    → Threaded (edges U) (edges (wire-in i j U))
  edge-wire-in i j (wires m) = match-insert-edges i j m
  edge-wire-in {Γˣ} {Δˣ} i j (node A B p q U) =
    edge-wire-in
      (insert-shift p (append-graph B Γˣ) i)
      (insert-shift q (append-graph A Δˣ) j)
      U

  edge-cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (U : Shape Ob Γ Δ)
    → Threaded (edges U) (edges (cap-in i j U))
  edge-cap-in i j (wires m) = match-cap-edges i j m
  edge-cap-in {Γ˘} {Γˣ} i j (node A B p q U) =
    edge-cap-in
      (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
      (insert-shift p (append-graph B Γ˘) j)
      U

  -- THE THREADED WIRE TOUCHES NO VERTEX. Both of its ends are legs of the whole
  -- shape — the input leg and the output leg it was given — however many
  -- vertices it was threaded past. This is the statement the merger needs, and
  -- it is what makes placing two shapes side by side join nothing.
  ends-wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (U : Shape Ob Γ Δ)
    → incid (wire-in i j U) (slot (Threaded.spot (edge-wire-in i j U)))
      ≡ (inj₂ (inj₁ (slot i)) , inj₂ (inj₂ (slot j)))
  ends-wire-in i j (wires m) = cong (pmap inj₂ inj₂) (ends-match-insert i j m)
  ends-wire-in {Γˣ} {Δˣ} i j (node A B p q U) =
    trans
      (cong
        (pmap
          (step-out (append-graph B Γˣ) (append-graph A Δˣ))
          (step-out (append-graph B Γˣ) (append-graph A Δˣ)))
        (ends-wire-in
          (insert-shift p (append-graph B Γˣ) i)
          (insert-shift q (append-graph A Δˣ) j)
          U))
      (cong₂
        _,_
        (cong (smap (λ _ → here) inj₁) (split-shift p (append-graph B Γˣ) i))
        (cong (smap (λ _ → here) inj₂) (split-shift q (append-graph A Δˣ) j)))

  -- and the threaded CUT touches no vertex either, with both ends input legs
  ends-cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (U : Shape Ob Γ Δ)
    → Ends
        (inj₂ (inj₁ (slot i)))
        (inj₂ (inj₁ (past i (slot j))))
        (incid (cap-in i j U) (slot (Threaded.spot (edge-cap-in i j U))))
  ends-cap-in i j (wires m) = ends-map inj₂ (ends-match-cap i j m)
  ends-cap-in {Γ˘} {Γˣ} i j (node A B p q U) =
    ends-cast
      (cong
        (smap (λ _ → here) inj₁)
        (split-shift (append-graph B Γ˘) (append-graph B Γˣ) i))
      (cong
        (smap (λ _ → here) inj₁)
        (trans
          (split-shift-past
            (append-graph B Γ˘)
            (append-graph B Γˣ)
            i
            (slot (insert-shift p (append-graph B Γ˘) j)))
          (cong
            (smap id (past i))
            (split-shift p (append-graph B Γ˘) j))))
      (ends-map
        (step-out (append-graph B Γˣ) q)
        (ends-cap-in
          (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
          (insert-shift p (append-graph B Γ˘) j)
          U))

  -- AND EVERY OTHER EDGE KEEPS ITS INCIDENCE, at the same vertex and the same
  -- leg, read in the extended interfaces. So threading changes the graph in
  -- exactly one place, which is the other half of what the merger needs.
  ends-wire-in-past
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (U : Shape Ob Γ Δ)
    → (e : Edg U)
    → incid (wire-in i j U) (past (Threaded.spot (edge-wire-in i j U)) e)
      ≡ pmap
          (smap (vtx-wire-in i j U) (smap (past i) (past j)))
          (smap (vtx-wire-in i j U) (smap (past i) (past j)))
          (incid U e)
  ends-wire-in-past i j (wires m) e =
    cong (pmap inj₂ inj₂) (ends-match-insert-past i j m e)
  ends-wire-in-past {Γˣ} {Δˣ} i j (node A B p q U) e =
    trans
      (cong
        (pmap
          (step-out (append-graph B Γˣ) (append-graph A Δˣ))
          (step-out (append-graph B Γˣ) (append-graph A Δˣ)))
        (ends-wire-in-past
          (insert-shift p (append-graph B Γˣ) i)
          (insert-shift q (append-graph A Δˣ) j)
          U
          e))
      (legs-cong step (incid U e))
    where
      step
        : (z : Vtx U ⊎ Leg _ _)
        → step-out (append-graph B Γˣ) (append-graph A Δˣ)
            (smap
              (vtx-wire-in
                (insert-shift p (append-graph B Γˣ) i)
                (insert-shift q (append-graph A Δˣ) j)
                U)
              (smap
                (past (insert-shift p (append-graph B Γˣ) i))
                (past (insert-shift q (append-graph A Δˣ) j)))
              z)
          ≡ smap
              (vtx-wire-in i j (node A B p q U))
              (smap (past i) (past j))
              (step-out p q z)
      step (inj₁ v) = refl
      step (inj₂ (inj₁ z))
        with split p z | split-shift-past p (append-graph B Γˣ) i z
      ... | inj₁ b | eq = cong (smap (λ _ → here) inj₁) eq
      ... | inj₂ w | eq = cong (smap (λ _ → here) inj₁) eq
      step (inj₂ (inj₂ z))
        with split q z | split-shift-past q (append-graph A Δˣ) j z
      ... | inj₁ b | eq = cong (smap (λ _ → here) inj₂) eq
      ... | inj₂ w | eq = cong (smap (λ _ → here) inj₂) eq

  ends-cap-in-past
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (U : Shape Ob Γ Δ)
    → (e : Edg U)
    → incid (cap-in i j U) (past (Threaded.spot (edge-cap-in i j U)) e)
      ≡ pmap
          (smap (vtx-cap-in i j U) (smap (λ z → past i (past j z)) id))
          (smap (vtx-cap-in i j U) (smap (λ z → past i (past j z)) id))
          (incid U e)
  ends-cap-in-past i j (wires m) e =
    cong (pmap inj₂ inj₂) (ends-match-cap-past i j m e)
  ends-cap-in-past {Γ˘} {Γˣ} i j (node A B p q U) e =
    trans
      (cong
        (pmap (step-out (append-graph B Γˣ) q) (step-out (append-graph B Γˣ) q))
        (ends-cap-in-past
          (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
          (insert-shift p (append-graph B Γ˘) j)
          U
          e))
      (legs-cong step (incid U e))
    where
      step
        : (z : Vtx U ⊎ Leg _ _)
        → step-out (append-graph B Γˣ) q
            (smap
              (vtx-cap-in
                (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
                (insert-shift p (append-graph B Γ˘) j)
                U)
              (smap
                (λ w →
                  past
                    (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
                    (past (insert-shift p (append-graph B Γ˘) j) w))
                id)
              z)
          ≡ smap
              (vtx-cap-in i j (node A B p q U))
              (smap (λ w → past i (past j w)) id)
              (step-out p q z)
      step (inj₁ v) = refl
      step (inj₂ (inj₁ z))
        with split p z
           | trans
               (split-shift-past
                 (append-graph B Γ˘)
                 (append-graph B Γˣ)
                 i
                 (past (insert-shift p (append-graph B Γ˘) j) z))
               (cong
                 (smap id (past i))
                 (split-shift-past p (append-graph B Γ˘) j z))
      ... | inj₁ b | eq = cong (smap (λ _ → here) inj₁) eq
      ... | inj₂ w | eq = cong (smap (λ _ → here) inj₁) eq
      step (inj₂ (inj₂ z)) with split q z
      ... | inj₁ b = refl
      ... | inj₂ w = refl

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE MERGER'S INCIDENCE THEOREM: IT JOINS NOTHING. `verts-merge` says both
  -- operands' vertices survive; this says no edge of the composite runs from
  -- one operand's vertex to the other's. Together they are the general form of
  -- what `two-points` could previously only exhibit — a merge of two shapes
  -- that each have a vertex is DISCONNECTED, whatever their interfaces and
  -- whatever their wirings do.
  --
  -- The proof carries a SIDE. Every vertex and every leg of the composite
  -- belongs to one operand or the other, and the content is that a wire's two
  -- ends always agree about which. Reading the side off a leg is `split`'s job;
  -- reading it off a vertex is a recursion on the first operand, since a
  -- wiring contributes no vertex and every vertex the merger threads a wiring
  -- into came from the second operand.
  --
  -- Stating it this way rather than as a dictionary of injections is what keeps
  -- it to one induction: the side is preserved by both threading operations and
  -- by the node step, and those three facts are the whole argument.
  -- ══════════════════════════════════════════════════════════════════════════

  is-left
    : ∀ {a b} {A : Set a} {B : Set b}
    → A ⊎ B
    → Bool
  is-left = case⊎ (λ _ → true) (λ _ → false)

  is-left-smap
    : ∀ {a b c d} {A : Set a} {B : Set b} {C : Set c} {D : Set d}
    → (f : A → C)
    → (g : B → D)
    → (s : A ⊎ B)
    → is-left (smap f g s) ≡ is-left s
  is-left-smap f g (inj₁ x) = refl
  is-left-smap f g (inj₂ y) = refl

  -- Which operand a LEG belongs to. The interfaces are concatenated, so this is
  -- what the concatenation witnesses already say.
  side-leg
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → Append Ob Γ₁ Γ₂ Γ
    → Append Ob Δ₁ Δ₂ Δ
    → Leg Γ Δ
    → Bool
  side-leg p q (inj₁ i) = is-left (split p i)
  side-leg p q (inj₂ j) = is-left (split q j)

  -- and which operand an END of a shape the merger threaded a WIRING into
  -- belongs to. Every vertex of such a shape came from the second operand,
  -- because the first contributed none.
  sideʷ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ Γ′ Δ′} {U : Shape Ob Γ′ Δ′}
    → Append Ob Γ₁ Γ₂ Γ
    → Append Ob Δ₁ Δ₂ Δ
    → Vtx U ⊎ Leg Γ Δ
    → Bool
  sideʷ p q = case⊎ (λ _ → false) (side-leg p q)

  -- At the empty first operand every end is on the second operand's side, which
  -- is the base of the induction below.
  nil-side
    : ∀ {Γ₂ Δ₂ Γ′ Δ′} {U : Shape Ob Γ′ Δ′}
    → (x : Vtx U ⊎ Leg Γ₂ Δ₂)
    → sideʷ nil nil x ≡ false
  nil-side (inj₁ v) = refl
  nil-side (inj₂ (inj₁ i)) = refl
  nil-side (inj₂ (inj₂ j)) = refl

  -- A widened insertion still splits to the side it came from, and so do the
  -- positions it did not take. These are what carry the side across one
  -- threading step.
  widen-slot
    : ∀ {x ys zs Ξ Δ}
    → (i : Insert Ob x ys zs)
    → (q : Append Ob zs Ξ Δ)
    → split q (slot (Widened.spot (insert-widen i q))) ≡ inj₁ (slot i)
  widen-slot head (cons q) = refl
  widen-slot (tail i) (cons q) = cong (smap there id) (widen-slot i q)

  widen-past
    : ∀ {x ys zs Ξ Δ}
    → (i : Insert Ob x ys zs)
    → (q : Append Ob zs Ξ Δ)
    → (z : Ix (Widened.rest (insert-widen i q)))
    → split q (past (Widened.spot (insert-widen i q)) z)
      ≡ smap (past i) id (split (Widened.keep (insert-widen i q)) z)
  widen-past head (cons q) z = refl
  widen-past (tail i) (cons q) here = refl
  widen-past (tail i) (cons q) (there z)
    with split (Widened.keep (insert-widen i q)) z | widen-past i q z
  ... | inj₁ b | eq = cong (smap there id) eq
  ... | inj₂ w | eq = cong (smap there id) eq

  -- Both ends of a cut are on one side, whichever order the listing names them
  -- in. This is what makes the unordered pair enough.
  ends-both
    : ∀ {a} {A : Set a} {u v : A} {x : A × A} {b : Bool}
    → (f : A → Bool)
    → f u ≡ b
    → f v ≡ b
    → Ends u v x
    → f (proj₁ x) ≡ f (proj₂ x)
  ends-both f eu ev forwards = trans eu (sym ev)
  ends-both f eu ev backwards = trans ev (sym eu)

  -- MERGING A WIRING JOINS NOTHING. Each of the first operand's wires and cuts
  -- is threaded in with both of its ends on the first operand's side, and every
  -- edge the second operand already had keeps both of its ends on the second's.
  sides-wires-in
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Edg (wires-in p q m T))
    → sideʷ p q (end₀ (wires-in p q m T) e)
      ≡ sideʷ p q (end₁ (wires-in p q m T) e)
  sides-wires-in nil nil [] T e =
    trans (nil-side (end₀ T e)) (sym (nil-side (end₁ T e)))
  sides-wires-in (cons p) q (i ∷ m) T e = aux (islot _ e)
    where
      jʷ = Widened.spot (insert-widen i q)
      qʷ = Widened.keep (insert-widen i q)
      U = wires-in p qʷ m T

      carry
        : (x : Vtx U ⊎ Leg _ _)
        → sideʷ (cons p) q
            (smap (vtx-wire-in head jʷ U) (smap (past head) (past jʷ)) x)
          ≡ sideʷ p qʷ x
      carry (inj₁ v) = refl
      carry (inj₂ (inj₁ z)) = is-left-smap there id (split p z)
      carry (inj₂ (inj₂ z)) =
        trans
          (cong is-left (widen-past i q z))
          (is-left-smap (past i) id (split qʷ z))

      aux
        : {e : Edg (wire-in head jʷ U)}
        → Slot (Threaded.spot (edge-wire-in head jʷ U)) e
        → sideʷ (cons p) q (end₀ (wire-in head jʷ U) e)
          ≡ sideʷ (cons p) q (end₁ (wire-in head jʷ U) e)
      aux taken =
        trans
          (cong (λ x → sideʷ (cons p) q (proj₁ x)) (ends-wire-in head jʷ U))
          (sym
            (trans
              (cong (λ x → sideʷ (cons p) q (proj₂ x)) (ends-wire-in head jʷ U))
              (cong is-left (widen-slot i q))))
      aux (spare e′) =
        trans
          (cong (λ x → sideʷ (cons p) q (proj₁ x)) (ends-wire-in-past head jʷ U e′))
          (trans
            (carry (end₀ U e′))
            (trans
              (sides-wires-in p qʷ m T e′)
              (trans
                (sym (carry (end₁ U e′)))
                (cong
                  (λ x → sideʷ (cons p) q (proj₂ x))
                  (sym (ends-wire-in-past head jʷ U e′))))))
  sides-wires-in (cons p) q (cap j m) T e = aux (islot _ e)
    where
      jᶜ = Widened.spot (insert-widen j p)
      pᶜ = Widened.keep (insert-widen j p)
      U = wires-in pᶜ q m T

      carry
        : (x : Vtx U ⊎ Leg _ _)
        → sideʷ (cons p) q
            (smap
              (vtx-cap-in head jᶜ U)
              (smap (λ z → past head (past jᶜ z)) id)
              x)
          ≡ sideʷ pᶜ q x
      carry (inj₁ v) = refl
      carry (inj₂ (inj₁ z)) =
        trans
          (is-left-smap there id (split p (past jᶜ z)))
          (trans
            (cong is-left (widen-past j p z))
            (is-left-smap (past j) id (split pᶜ z)))
      carry (inj₂ (inj₂ z)) = refl

      aux
        : {e : Edg (cap-in head jᶜ U)}
        → Slot (Threaded.spot (edge-cap-in head jᶜ U)) e
        → sideʷ (cons p) q (end₀ (cap-in head jᶜ U) e)
          ≡ sideʷ (cons p) q (end₁ (cap-in head jᶜ U) e)
      aux taken =
        ends-both
          (sideʷ (cons p) q)
          refl
          (trans
            (is-left-smap there id (split p (slot jᶜ)))
            (cong is-left (widen-slot j p)))
          (ends-cap-in head jᶜ U)
      aux (spare e′) =
        trans
          (cong (λ x → sideʷ (cons p) q (proj₁ x)) (ends-cap-in-past head jᶜ U e′))
          (trans
            (carry (end₀ U e′))
            (trans
              (sides-wires-in pᶜ q m T e′)
              (trans
                (sym (carry (end₁ U e′)))
                (cong
                  (λ x → sideʷ (cons p) q (proj₂ x))
                  (sym (ends-cap-in-past head jᶜ U e′))))))

  -- Which operand a VERTEX of a merge belongs to. The first operand's vertices
  -- are the ones the merger republishes, so this is a recursion on it, and at
  -- the bottom every remaining vertex is the second operand's.
  side-vtx
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx (merge p q S T)
    → Bool
  side-vtx p q (wires m) T v = false
  side-vtx p q (node A B p₁ q₁ S) T here = true
  side-vtx p q (node A B p₁ q₁ S) T (there v) =
    side-vtx
      (Regroup.back (append-regroup p₁ p))
      (Regroup.back (append-regroup q₁ q))
      S
      T
      v

  side
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx (merge p q S T) ⊎ Leg Γ Δ
    → Bool
  side p q S T = case⊎ (side-vtx p q S T) (side-leg p q)

  -- Re-associating the two concatenations does not move the boundary between
  -- the operands: a position of the merged interface is the republished
  -- vertex's port, or it is a leg that was already on one side.
  regroup-side
    : ∀ {B Γ₁ Γ′ Γ₂ Γ}
    → (p₁ : Append Ob B Γ₁ Γ′)
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (z : Ix (Regroup.whole (append-regroup p₁ p)))
    → is-left (split (Regroup.back (append-regroup p₁ p)) z)
      ≡ case⊎
          (λ _ → true)
          (λ w → is-left (split p w))
          (split (Regroup.front (append-regroup p₁ p)) z)
  regroup-side nil p z = refl
  regroup-side (cons p₁) p here = refl
  regroup-side (cons p₁) p (there z)
    with split (Regroup.front (append-regroup p₁ p)) z | regroup-side p₁ p z
  ... | inj₁ b | eq =
    trans (is-left-smap there id (split (Regroup.back (append-regroup p₁ p)) z)) eq
  ... | inj₂ w | eq =
    trans (is-left-smap there id (split (Regroup.back (append-regroup p₁ p)) z)) eq

  -- THE THEOREM. A wire's two ends belong to the same operand -- always, for
  -- every merge, at every interface.
  sides-merge
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Edg (merge p q S T))
    → side p q S T (end₀ (merge p q S T) e)
      ≡ side p q S T (end₁ (merge p q S T) e)
  sides-merge p q (wires m) T e = sides-wires-in p q m T e
  sides-merge p q (node A B p₁ q₁ S) T e =
    trans
      (carry (end₀ M e))
      (trans (sides-merge pᵣ qᵣ S T e) (sym (carry (end₁ M e))))
    where
      pᵣ = Regroup.back (append-regroup p₁ p)
      qᵣ = Regroup.back (append-regroup q₁ q)
      M = merge pᵣ qᵣ S T

      carry
        : (x : Vtx M ⊎ Leg _ _)
        → side p q (node A B p₁ q₁ S) T
            (step-out
              (Regroup.front (append-regroup p₁ p))
              (Regroup.front (append-regroup q₁ q))
              x)
          ≡ side pᵣ qᵣ S T x
      carry (inj₁ v) = refl
      carry (inj₂ (inj₁ z))
        with split (Regroup.front (append-regroup p₁ p)) z | regroup-side p₁ p z
      ... | inj₁ b | eq = sym eq
      ... | inj₂ w | eq = sym eq
      carry (inj₂ (inj₂ z))
        with split (Regroup.front (append-regroup q₁ q)) z | regroup-side q₁ q z
      ... | inj₁ b | eq = sym eq
      ... | inj₂ w | eq = sym eq

  -- ── WHAT THE SIDE BUYS ─────────────────────────────────────────────────────
  -- Naming each operand's vertices in the composite, then reading the theorem
  -- back as a statement about the composite's components.

  vtx-wires-in
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx T
    → Vtx (wires-in p q m T)
  vtx-wires-in nil nil [] T v = v
  vtx-wires-in (cons p) q (i ∷ m) T v =
    vtx-wire-in
      head
      (Widened.spot (insert-widen i q))
      (wires-in p (Widened.keep (insert-widen i q)) m T)
      (vtx-wires-in p (Widened.keep (insert-widen i q)) m T v)
  vtx-wires-in (cons p) q (cap j m) T v =
    vtx-cap-in
      head
      (Widened.spot (insert-widen j p))
      (wires-in (Widened.keep (insert-widen j p)) q m T)
      (vtx-wires-in (Widened.keep (insert-widen j p)) q m T v)

  vtxˡ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx S
    → Vtx (merge p q S T)
  vtxˡ p q (wires m) T ()
  vtxˡ p q (node A B p₁ q₁ S) T here = here
  vtxˡ p q (node A B p₁ q₁ S) T (there v) =
    there
      (vtxˡ
        (Regroup.back (append-regroup p₁ p))
        (Regroup.back (append-regroup q₁ q))
        S
        T
        v)

  vtxʳ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx T
    → Vtx (merge p q S T)
  vtxʳ p q (wires m) T v = vtx-wires-in p q m T v
  vtxʳ p q (node A B p₁ q₁ S) T v =
    there
      (vtxʳ
        (Regroup.back (append-regroup p₁ p))
        (Regroup.back (append-regroup q₁ q))
        S
        T
        v)

  -- and each lands on the side it came from, which is what makes the side a
  -- statement about the operands rather than about the composite's own order
  side-vtxˡ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (u : Vtx S)
    → side-vtx p q S T (vtxˡ p q S T u) ≡ true
  side-vtxˡ p q (wires m) T ()
  side-vtxˡ p q (node A B p₁ q₁ S) T here = refl
  side-vtxˡ p q (node A B p₁ q₁ S) T (there v) =
    side-vtxˡ
      (Regroup.back (append-regroup p₁ p))
      (Regroup.back (append-regroup q₁ q))
      S
      T
      v

  side-vtxʳ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (v : Vtx T)
    → side-vtx p q S T (vtxʳ p q S T v) ≡ false
  side-vtxʳ p q (wires m) T v = refl
  side-vtxʳ p q (node A B p₁ q₁ S) T v =
    side-vtxʳ
      (Regroup.back (append-regroup p₁ p))
      (Regroup.back (append-regroup q₁ q))
      S
      T
      v

  -- A traversal stays on one side, since a wire has both ends there…
  link-side
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {e : Edg (merge p q S T)} {u v : Vtx (merge p q S T)}
    → Link (merge p q S T) e u v
    → side-vtx p q S T u ≡ side-vtx p q S T v
  link-side p q S T {e} (along a) =
    trans
      (sym (cong (side p q S T) (Attach.from a)))
      (trans (sides-merge p q S T e) (cong (side p q S T) (Attach.into a)))
  link-side p q S T {e} (against a) =
    sym
      (trans
        (sym (cong (side p q S T) (Attach.from a)))
        (trans (sides-merge p q S T e) (cong (side p q S T) (Attach.into a))))

  -- …and therefore so does reachability
  reach-side
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {u v : Vtx (merge p q S T)}
    → Reach (merge p q S T) u v
    → side-vtx p q S T u ≡ side-vtx p q S T v
  reach-side p q S T stop = refl
  reach-side p q S T (onward r (adj e l)) =
    trans (reach-side p q S T r) (link-side p q S T l)

  true≢false : ¬ (true ≡ false)
  true≢false ()

  -- NO EDGE OF A MERGE JOINS THE TWO OPERANDS. The general form of what the
  -- worked example could only exhibit at a shape with no edge at all.
  merge-apart
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {e : Edg (merge p q S T)}
    → (u : Vtx S)
    → (v : Vtx T)
    → ¬ Link (merge p q S T) e (vtxˡ p q S T u) (vtxʳ p q S T v)
  merge-apart p q S T u v l =
    true≢false
      (trans
        (sym (side-vtxˡ p q S T u))
        (trans (link-side p q S T l) (side-vtxʳ p q S T v)))

  -- SO A MERGE OF TWO SHAPES THAT EACH HAVE A VERTEX IS DISCONNECTED. Axis A as
  -- a theorem about the operation rather than a fact about one worked shape:
  -- whatever the interfaces, whatever the wirings, placing two nonempty shapes
  -- side by side produces something the substrate can refute the connectivity
  -- of.
  merge-disconnected
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Vtx S
    → Vtx T
    → ¬ Connected (merge p q S T)
  merge-disconnected p q S T u v c =
    true≢false
      (trans
        (sym (side-vtxˡ p q S T u))
        (trans
          (reach-side p q S T (reach-any c (vtxˡ p q S T u) (vtxʳ p q S T v)))
          (side-vtxʳ p q S T v)))

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE CONVERSE: EACH OPERAND'S OWN ADJACENCIES SURVIVE. `merge-apart` says
  -- the composite has no edge the operands did not; this says it has no fewer.
  -- Together they say the composite's components are exactly the two operands.
  --
  -- The two halves are not symmetric and the asymmetry is the merger's own. The
  -- SECOND operand's edges are the ones the threading left alone, so its half
  -- is `ends-wire-in-past` and `ends-cap-in-past` composed down the recursion.
  -- The FIRST operand's edges are the FRESH ones the threading added, so its
  -- half has to relate a threaded wire's ends to that wire's ends in the
  -- operand — and there a cut's two ends are an unordered pair, so what is true
  -- is a statement about `Link`, which is symmetric, and not one about the
  -- ordered incidence.
  -- ══════════════════════════════════════════════════════════════════════════

  -- A widened insertion takes the position the un-widened one did, and leaves
  -- the appended block where it was.
  widen-left
    : ∀ {x ys zs Ξ Δ}
    → (i : Insert Ob x ys zs)
    → (q : Append Ob zs Ξ Δ)
    → slot (Widened.spot (insert-widen i q)) ≡ left q (slot i)
  widen-left head (cons q) = refl
  widen-left (tail i) (cons q) = cong there (widen-left i q)

  widen-left-past
    : ∀ {x ys zs Ξ Δ}
    → (i : Insert Ob x ys zs)
    → (q : Append Ob zs Ξ Δ)
    → (w : Ix ys)
    → past (Widened.spot (insert-widen i q))
        (left (Widened.keep (insert-widen i q)) w)
      ≡ left q (past i w)
  widen-left-past head (cons q) w = refl
  widen-left-past (tail i) (cons q) here = refl
  widen-left-past (tail i) (cons q) (there w) =
    cong there (widen-left-past i q w)

  widen-right
    : ∀ {x ys zs Ξ Δ}
    → (i : Insert Ob x ys zs)
    → (q : Append Ob zs Ξ Δ)
    → (w : Ix Ξ)
    → past (Widened.spot (insert-widen i q))
        (right (Widened.keep (insert-widen i q)) w)
      ≡ right q w
  widen-right head (cons q) w = refl
  widen-right (tail i) (cons q) w = cong there (widen-right i q w)

  -- and re-associating the two concatenations leaves each operand's positions
  -- where they were — `regroup-side`'s exact form, which the converse needs
  regroup-left
    : ∀ {B Γ₁ Γ′ Γ₂ Γ}
    → (p₁ : Append Ob B Γ₁ Γ′)
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (z : Ix Γ′)
    → split (Regroup.front (append-regroup p₁ p))
        (left (Regroup.back (append-regroup p₁ p)) z)
      ≡ smap id (left p) (split p₁ z)
  regroup-left nil p z = refl
  regroup-left (cons p₁) p here = refl
  regroup-left (cons p₁) p (there z) =
    trans
      (cong (smap there id) (regroup-left p₁ p z))
      (smap-exch there (left p) (split p₁ z))

  regroup-right
    : ∀ {B Γ₁ Γ′ Γ₂ Γ}
    → (p₁ : Append Ob B Γ₁ Γ′)
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (w : Ix Γ₂)
    → split (Regroup.front (append-regroup p₁ p))
        (right (Regroup.back (append-regroup p₁ p)) w)
      ≡ inj₂ (right p w)
  regroup-right nil p w = refl
  regroup-right (cons p₁) p w = cong (smap there id) (regroup-right p₁ p w)

  -- ── THE SECOND OPERAND'S HALF ──────────────────────────────────────────────

  -- Its edges are the ones the threading left alone, so naming them in the
  -- composite is iterating "the position this insertion did not take".
  thread-edgʳ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Edg T
    → Edg (wires-in p q m T)
  thread-edgʳ nil nil [] T e = e
  thread-edgʳ (cons p) q (i ∷ m) T e =
    past
      (Threaded.spot
        (edge-wire-in
          head
          (Widened.spot (insert-widen i q))
          (wires-in p (Widened.keep (insert-widen i q)) m T)))
      (thread-edgʳ p (Widened.keep (insert-widen i q)) m T e)
  thread-edgʳ (cons p) q (cap j m) T e =
    past
      (Threaded.spot
        (edge-cap-in
          head
          (Widened.spot (insert-widen j p))
          (wires-in (Widened.keep (insert-widen j p)) q m T)))
      (thread-edgʳ (Widened.keep (insert-widen j p)) q m T e)

  -- and each keeps the vertex and the leg it had, read on the second operand's
  -- side of the merged interfaces
  ends-thread-right
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Edg T)
    → incid (wires-in p q m T) (thread-edgʳ p q m T e)
      ≡ pmap
          (smap (vtx-wires-in p q m T) (smap (right p) (right q)))
          (smap (vtx-wires-in p q m T) (smap (right p) (right q)))
          (incid T e)
  ends-thread-right nil nil [] T e = legs-cong step (incid T e)
    where
      step
        : (z : _)
        → z
          ≡ smap
              (vtx-wires-in nil nil [] T)
              (smap (right nil) (right nil))
              z
      step (inj₁ v) = refl
      step (inj₂ (inj₁ w)) = refl
      step (inj₂ (inj₂ w)) = refl
  ends-thread-right (cons p) q (i ∷ m) T e =
    trans
      (ends-wire-in-past head jʷ U (thread-edgʳ p qʷ m T e))
      (trans
        (cong (pmap F F) (ends-thread-right p qʷ m T e))
        (legs-cong step (incid T e)))
    where
      jʷ = Widened.spot (insert-widen i q)
      qʷ = Widened.keep (insert-widen i q)
      U = wires-in p qʷ m T
      F = smap (vtx-wire-in head jʷ U) (smap (past head) (past jʷ))

      step
        : (z : _)
        → F (smap (vtx-wires-in p qʷ m T) (smap (right p) (right qʷ)) z)
          ≡ smap
              (vtx-wires-in (cons p) q (i ∷ m) T)
              (smap (right (cons p)) (right q))
              z
      step (inj₁ v) = refl
      step (inj₂ (inj₁ w)) = refl
      step (inj₂ (inj₂ w)) = cong (λ z → inj₂ (inj₂ z)) (widen-right i q w)
  ends-thread-right (cons p) q (cap j m) T e =
    trans
      (ends-cap-in-past head jᶜ U (thread-edgʳ pᶜ q m T e))
      (trans
        (cong (pmap G G) (ends-thread-right pᶜ q m T e))
        (legs-cong step (incid T e)))
    where
      jᶜ = Widened.spot (insert-widen j p)
      pᶜ = Widened.keep (insert-widen j p)
      U = wires-in pᶜ q m T
      G = smap (vtx-cap-in head jᶜ U) (smap (λ z → past head (past jᶜ z)) id)

      step
        : (z : _)
        → G (smap (vtx-wires-in pᶜ q m T) (smap (right pᶜ) (right q)) z)
          ≡ smap
              (vtx-wires-in (cons p) q (cap j m) T)
              (smap (right (cons p)) (right q))
              z
      step (inj₁ v) = refl
      step (inj₂ (inj₁ w)) =
        cong (λ z → inj₂ (inj₁ (there z))) (widen-right j p w)
      step (inj₂ (inj₂ w)) = refl

  edgʳ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Edg T
    → Edg (merge p q S T)
  edgʳ p q (wires m) T e = thread-edgʳ p q m T e
  edgʳ p q (node A B p₁ q₁ S) T e =
    edgʳ
      (Regroup.back (append-regroup p₁ p))
      (Regroup.back (append-regroup q₁ q))
      S
      T
      e

  ends-merge-right
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Edg T)
    → incid (merge p q S T) (edgʳ p q S T e)
      ≡ pmap
          (smap (vtxʳ p q S T) (smap (right p) (right q)))
          (smap (vtxʳ p q S T) (smap (right p) (right q)))
          (incid T e)
  ends-merge-right p q (wires m) T e = ends-thread-right p q m T e
  ends-merge-right p q (node A B p₁ q₁ S) T e =
    trans
      (cong (pmap So So) (ends-merge-right pᵣ qᵣ S T e))
      (legs-cong step (incid T e))
    where
      pᵣ = Regroup.back (append-regroup p₁ p)
      qᵣ = Regroup.back (append-regroup q₁ q)
      So =
        step-out
          (Regroup.front (append-regroup p₁ p))
          (Regroup.front (append-regroup q₁ q))

      step
        : (z : _)
        → So (smap (vtxʳ pᵣ qᵣ S T) (smap (right pᵣ) (right qᵣ)) z)
          ≡ smap
              (vtxʳ p q (node A B p₁ q₁ S) T)
              (smap (right p) (right q))
              z
      step (inj₁ v) = refl
      step (inj₂ (inj₁ w)) =
        cong (smap (λ _ → here) inj₁) (regroup-right p₁ p w)
      step (inj₂ (inj₂ w)) =
        cong (smap (λ _ → here) inj₂) (regroup-right q₁ q w)

  attach-right
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {e : Edg T} {u v : Vtx T}
    → Attach T e u v
    → Attach (merge p q S T) (edgʳ p q S T e) (vtxʳ p q S T u) (vtxʳ p q S T v)
  attach-right p q S T {e} a =
    attach
      (trans
        (cong proj₁ (ends-merge-right p q S T e))
        (cong
          (smap (vtxʳ p q S T) (smap (right p) (right q)))
          (Attach.from a)))
      (trans
        (cong proj₂ (ends-merge-right p q S T e))
        (cong
          (smap (vtxʳ p q S T) (smap (right p) (right q)))
          (Attach.into a)))

  link-right
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {e : Edg T} {u v : Vtx T}
    → Link T e u v
    → Link (merge p q S T) (edgʳ p q S T e) (vtxʳ p q S T u) (vtxʳ p q S T v)
  link-right p q S T (along a) = along (attach-right p q S T a)
  link-right p q S T (against a) = against (attach-right p q S T a)

  reach-right
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {u v : Vtx T}
    → Reach T u v
    → Reach (merge p q S T) (vtxʳ p q S T u) (vtxʳ p q S T v)
  reach-right p q S T stop = stop
  reach-right p q S T (onward r (adj e l)) =
    onward (reach-right p q S T r) (adj (edgʳ p q S T e) (link-right p q S T l))

  -- ── THE FIRST OPERAND'S HALF ───────────────────────────────────────────────
  -- Its edges are the FRESH ones the threading added, one per wire and one per
  -- cut of its wiring, so naming them is reading the threading's own position.

  ends-eq
    : ∀ {a} {A : Set a} {u v : A} {x : A × A}
    → x ≡ (u , v)
    → Ends u v x
  ends-eq refl = forwards

  ends-index
    : ∀ {a} {A : Set a} {u v : A} {x y : A × A}
    → x ≡ y
    → Ends u v x
    → Ends u v y
  ends-index refl e = e

  thread-edgˡ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Ix (match-edges m)
    → Edg (wires-in p q m T)
  thread-edgˡ nil nil [] T ()
  thread-edgˡ (cons p) q (i ∷ m) T here =
    slot
      (Threaded.spot
        (edge-wire-in
          head
          (Widened.spot (insert-widen i q))
          (wires-in p (Widened.keep (insert-widen i q)) m T)))
  thread-edgˡ (cons p) q (i ∷ m) T (there e) =
    past
      (Threaded.spot
        (edge-wire-in
          head
          (Widened.spot (insert-widen i q))
          (wires-in p (Widened.keep (insert-widen i q)) m T)))
      (thread-edgˡ p (Widened.keep (insert-widen i q)) m T e)
  thread-edgˡ (cons p) q (cap j m) T here =
    slot
      (Threaded.spot
        (edge-cap-in
          head
          (Widened.spot (insert-widen j p))
          (wires-in (Widened.keep (insert-widen j p)) q m T)))
  thread-edgˡ (cons p) q (cap j m) T (there e) =
    past
      (Threaded.spot
        (edge-cap-in
          head
          (Widened.spot (insert-widen j p))
          (wires-in (Widened.keep (insert-widen j p)) q m T)))
      (thread-edgˡ (Widened.keep (insert-widen j p)) q m T e)

  -- and each keeps the ends it had in the wiring, read on the FIRST operand's
  -- side of the merged interfaces — as an unordered pair, because a cut's are
  ends-thread-left
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Ix (match-edges m))
    → Ends
        (inj₂ (smap (left p) (left q) (proj₁ (ends m e))))
        (inj₂ (smap (left p) (left q) (proj₂ (ends m e))))
        (incid (wires-in p q m T) (thread-edgˡ p q m T e))
  ends-thread-left nil nil [] T ()
  ends-thread-left (cons p) q (i ∷ m) T here =
    ends-eq
      (trans
        (ends-wire-in
          head
          (Widened.spot (insert-widen i q))
          (wires-in p (Widened.keep (insert-widen i q)) m T))
        (cong
          (λ z → inj₂ (inj₁ here) , inj₂ (inj₂ z))
          (widen-left i q)))
  ends-thread-left (cons p) q (i ∷ m) T (there e) =
    ends-cast
      (step (proj₁ (ends m e)))
      (step (proj₂ (ends m e)))
      (ends-index
        (sym (ends-wire-in-past head jʷ U (thread-edgˡ p qʷ m T e)))
        (ends-map F (ends-thread-left p qʷ m T e)))
    where
      jʷ = Widened.spot (insert-widen i q)
      qʷ = Widened.keep (insert-widen i q)
      U = wires-in p qʷ m T
      F = smap (vtx-wire-in head jʷ U) (smap (past head) (past jʷ))

      step
        : (z : _)
        → F (inj₂ (smap (left p) (left qʷ) z))
          ≡ inj₂
              (smap (left (cons p)) (left q) (smap there (past i) z))
      step (inj₁ w) = refl
      step (inj₂ w) = cong (λ y → inj₂ (inj₂ y)) (widen-left-past i q w)
  ends-thread-left (cons p) q (cap j m) T here =
    ends-cast
      refl
      (cong (λ z → inj₂ (inj₁ (there z))) (widen-left j p))
      (ends-cap-in
        head
        (Widened.spot (insert-widen j p))
        (wires-in (Widened.keep (insert-widen j p)) q m T))
  ends-thread-left (cons p) q (cap j m) T (there e) =
    ends-cast
      (step (proj₁ (ends m e)))
      (step (proj₂ (ends m e)))
      (ends-index
        (sym (ends-cap-in-past head jᶜ U (thread-edgˡ pᶜ q m T e)))
        (ends-map G (ends-thread-left pᶜ q m T e)))
    where
      jᶜ = Widened.spot (insert-widen j p)
      pᶜ = Widened.keep (insert-widen j p)
      U = wires-in pᶜ q m T
      G = smap (vtx-cap-in head jᶜ U) (smap (λ z → past head (past jᶜ z)) id)

      step
        : (z : _)
        → G (inj₂ (smap (left pᶜ) (left q) z))
          ≡ inj₂
              (smap
                (left (cons p))
                (left q)
                (smap (λ w → there (past j w)) id z))
      step (inj₁ w) =
        cong (λ y → inj₂ (inj₁ (there y))) (widen-left-past j p w)
      step (inj₂ w) = refl

  edgˡ
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Edg S
    → Edg (merge p q S T)
  edgˡ p q (wires m) T e = thread-edgˡ p q m T e
  edgˡ p q (node A B p₁ q₁ S) T e =
    edgˡ
      (Regroup.back (append-regroup p₁ p))
      (Regroup.back (append-regroup q₁ q))
      S
      T
      e

  ends-merge-left
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (e : Edg S)
    → Ends
        (smap (vtxˡ p q S T) (smap (left p) (left q)) (end₀ S e))
        (smap (vtxˡ p q S T) (smap (left p) (left q)) (end₁ S e))
        (incid (merge p q S T) (edgˡ p q S T e))
  ends-merge-left p q (wires m) T e = ends-thread-left p q m T e
  ends-merge-left p q (node A B p₁ q₁ S) T e =
    ends-cast
      (step (end₀ S e))
      (step (end₁ S e))
      (ends-map So (ends-merge-left pᵣ qᵣ S T e))
    where
      pᵣ = Regroup.back (append-regroup p₁ p)
      qᵣ = Regroup.back (append-regroup q₁ q)
      So =
        step-out
          (Regroup.front (append-regroup p₁ p))
          (Regroup.front (append-regroup q₁ q))

      step
        : (z : _)
        → So (smap (vtxˡ pᵣ qᵣ S T) (smap (left pᵣ) (left qᵣ)) z)
          ≡ smap
              (vtxˡ p q (node A B p₁ q₁ S) T)
              (smap (left p) (left q))
              (step-out p₁ q₁ z)
      step (inj₁ v) = refl
      step (inj₂ (inj₁ z)) with split p₁ z | regroup-left p₁ p z
      ... | inj₁ b | eq = cong (smap (λ _ → here) inj₁) eq
      ... | inj₂ w | eq = cong (smap (λ _ → here) inj₁) eq
      step (inj₂ (inj₂ z)) with split q₁ z | regroup-left q₁ q z
      ... | inj₁ b | eq = cong (smap (λ _ → here) inj₂) eq
      ... | inj₂ w | eq = cong (smap (λ _ → here) inj₂) eq

  -- An edge whose two ends are the same two vertices in SOME order is a
  -- traversal between them, in the order the traversal chooses. This is why
  -- the first operand's half is about `Link` and cannot be about `incid`.
  ends-link
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {e : Edg S} {u v : Vtx S}
      {x : (Vtx S ⊎ Leg Γ Δ) × (Vtx S ⊎ Leg Γ Δ)}
    → Ends (inj₁ u) (inj₁ v) x
    → incid S e ≡ x
    → Link S e u v
  ends-link forwards eq = along (attach (cong proj₁ eq) (cong proj₂ eq))
  ends-link backwards eq = against (attach (cong proj₁ eq) (cong proj₂ eq))

  link-left
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {e : Edg S} {u v : Vtx S}
    → Link S e u v
    → Link (merge p q S T) (edgˡ p q S T e) (vtxˡ p q S T u) (vtxˡ p q S T v)
  link-left p q S T {e} (along a) =
    ends-link
      (ends-cast
        (cong (smap (vtxˡ p q S T) (smap (left p) (left q))) (Attach.from a))
        (cong (smap (vtxˡ p q S T) (smap (left p) (left q))) (Attach.into a))
        (ends-merge-left p q S T e))
      refl
  link-left p q S T {e} (against a) =
    link-sym
      (ends-link
        (ends-cast
          (cong (smap (vtxˡ p q S T) (smap (left p) (left q))) (Attach.from a))
          (cong (smap (vtxˡ p q S T) (smap (left p) (left q))) (Attach.into a))
          (ends-merge-left p q S T e))
        refl)

  reach-left
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → {u v : Vtx S}
    → Reach S u v
    → Reach (merge p q S T) (vtxˡ p q S T u) (vtxˡ p q S T v)
  reach-left p q S T stop = stop
  reach-left p q S T (onward r (adj e l)) =
    onward (reach-left p q S T r) (adj (edgˡ p q S T e) (link-left p q S T l))

  -- ── AND EVERY VERTEX OF THE COMPOSITE IS ONE OPERAND'S ─────────────────────
  -- The last thing the components statement needs, and the reason it is a view
  -- rather than an inverse: eliminating it refines the vertex instead of
  -- handing back an equation the consumer then has to transport a walk along.

  data Onto {Γ Δ Γ′ Δ′} {U : Shape Ob Γ Δ} {V : Shape Ob Γ′ Δ′}
      (f : Vtx U → Vtx V)
    : Vtx V → Set ℓ where
    hits
      : (v : Vtx U)
      → Onto f (f v)

  onto-wire-in
    : ∀ {a Γ Γˣ Δ Δˣ}
    → (i : Insert Ob a Γ Γˣ)
    → (j : Insert Ob a Δ Δˣ)
    → (U : Shape Ob Γ Δ)
    → (x : Vtx (wire-in i j U))
    → Onto (vtx-wire-in i j U) x
  onto-wire-in i j (wires m) ()
  onto-wire-in i j (node A B p q U) here = hits here
  onto-wire-in {Γˣ} {Δˣ} i j (node A B p q U) (there x)
    with onto-wire-in
           (insert-shift p (append-graph B Γˣ) i)
           (insert-shift q (append-graph A Δˣ) j)
           U
           x
  ... | hits v = hits (there v)

  onto-cap-in
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (U : Shape Ob Γ Δ)
    → (z : Vtx (cap-in i j U))
    → Onto (vtx-cap-in i j U) z
  onto-cap-in i j (wires m) ()
  onto-cap-in i j (node A B p q U) here = hits here
  onto-cap-in {Γ˘} {Γˣ} i j (node A B p q U) (there z)
    with onto-cap-in
           (insert-shift (append-graph B Γ˘) (append-graph B Γˣ) i)
           (insert-shift p (append-graph B Γ˘) j)
           U
           z
  ... | hits v = hits (there v)

  onto-wires-in
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (m : Match Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (x : Vtx (wires-in p q m T))
    → Onto (vtx-wires-in p q m T) x
  onto-wires-in nil nil [] T x = hits x
  onto-wires-in (cons p) q (i ∷ m) T x
    with onto-wire-in
           head
           (Widened.spot (insert-widen i q))
           (wires-in p (Widened.keep (insert-widen i q)) m T)
           x
  ... | hits w
    with onto-wires-in p (Widened.keep (insert-widen i q)) m T w
  ... | hits v = hits v
  onto-wires-in (cons p) q (cap j m) T x
    with onto-cap-in
           head
           (Widened.spot (insert-widen j p))
           (wires-in (Widened.keep (insert-widen j p)) q m T)
           x
  ... | hits w
    with onto-wires-in (Widened.keep (insert-widen j p)) q m T w
  ... | hits v = hits v

  data VtxOf {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
      (p : Append Ob Γ₁ Γ₂ Γ)
      (q : Append Ob Δ₁ Δ₂ Δ)
      (S : Shape Ob Γ₁ Δ₁)
      (T : Shape Ob Γ₂ Δ₂)
    : Vtx (merge p q S T) → Set ℓ where
    -- a vertex of the first operand
    fromˡ
      : (u : Vtx S)
      → VtxOf p q S T (vtxˡ p q S T u)
    -- or one of the second
    fromʳ
      : (v : Vtx T)
      → VtxOf p q S T (vtxʳ p q S T v)

  vtx-of
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → (x : Vtx (merge p q S T))
    → VtxOf p q S T x
  vtx-of p q (wires m) T x with onto-wires-in p q m T x
  ... | hits v = fromʳ v
  vtx-of p q (node A B p₁ q₁ S) T here = fromˡ here
  vtx-of p q (node A B p₁ q₁ S) T (there x)
    with vtx-of
           (Regroup.back (append-regroup p₁ p))
           (Regroup.back (append-regroup q₁ q))
           S
           T
           x
  ... | fromˡ u = fromˡ (there u)
  ... | fromʳ v = fromʳ v

  -- ── THE COMPONENTS OF A MERGE ARE EXACTLY THE TWO OPERANDS ─────────────────

  -- Two vertices on the same side are joined, if the operands are.
  merge-reach
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Connected S
    → Connected T
    → (x y : Vtx (merge p q S T))
    → side-vtx p q S T x ≡ side-vtx p q S T y
    → Reach (merge p q S T) x y
  merge-reach p q S T cS cT x y eq
    with vtx-of p q S T x | vtx-of p q S T y
  ... | fromˡ u | fromˡ u′ = reach-left p q S T (reach-any cS u u′)
  ... | fromʳ v | fromʳ v′ = reach-right p q S T (reach-any cT v v′)
  ... | fromˡ u | fromʳ v =
    ⊥-elim
      (true≢false
        (trans
          (sym (side-vtxˡ p q S T u))
          (trans eq (side-vtxʳ p q S T v))))
  ... | fromʳ v | fromˡ u =
    ⊥-elim
      (true≢false
        (trans
          (sym (side-vtxˡ p q S T u))
          (trans (sym eq) (side-vtxʳ p q S T v))))

  -- SO REACHABILITY IN A MERGE OF TWO CONNECTED SHAPES IS EXACTLY AGREEMENT OF
  -- THE SIDE. Both directions together are "exactly two components, and they
  -- are the operands": no walk crosses (`reach-side`, the merger joins
  -- nothing), and every pair on one side is joined (`merge-reach`, each
  -- operand's own adjacencies survive). Neither half implies the other, and
  -- `merge-disconnected` is the first half read at one pair of vertices.
  merge-components
    : ∀ {Γ₁ Γ₂ Γ Δ₁ Δ₂ Δ}
    → (p : Append Ob Γ₁ Γ₂ Γ)
    → (q : Append Ob Δ₁ Δ₂ Δ)
    → (S : Shape Ob Γ₁ Δ₁)
    → (T : Shape Ob Γ₂ Δ₂)
    → Connected S
    → Connected T
    → (x y : Vtx (merge p q S T))
    → (Reach (merge p q S T) x y → side-vtx p q S T x ≡ side-vtx p q S T y)
      × (side-vtx p q S T x ≡ side-vtx p q S T y → Reach (merge p q S T) x y)
  merge-components p q S T cS cT x y =
    reach-side p q S T , merge-reach p q S T cS cT x y

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

  -- ══════════════════════════════════════════════════════════════════════════
  -- ASSOCIATIVITY, ON THE LISTING ALGEBRA — and first, the price of leaving
  -- structural recursion, paid ONCE.
  --
  -- `match-comp` recurses on a well-founded measure and splits on two `with`s,
  -- so a proof about it faces two obstructions at every use site: the
  -- accessibility witness is threaded through the term, and the composite is
  -- stuck behind an auxiliary that no lemma can name. The unit laws paid both
  -- costs directly, by re-deriving over an arbitrary `Acc`. That does not scale
  -- to associativity, which meets the same obstruction three times over.
  --
  -- So it is paid once here, as an INTERFACE, and every proof after it reads as
  -- if `match-comp` were defined by structural recursion:
  --
  --   * `match-comp-acc-irr` — the witness does not matter. Two runs at the
  --     same measure agree, so a statement never has to fix one.
  --   * the three UNFOLDING lemmas — what `match-comp` does on each head form,
  --     stated about `match-comp` itself and taking the `with` scrutinee's
  --     value as an ARGUMENT with its defining equation. Past these, no proof
  --     below mentions an accessibility witness or the internal auxiliary.
  --
  -- ── AND THE RULE THE PROOFS THEMSELVES OBEY, WHICH IS THE SAME RULE ────────
  -- The associativity proof passes every scrutinee as an argument rather than
  -- meeting it with a `with`. A `with` there rewrites the goal into
  -- `match-comp`'s own internal auxiliary — measured, not guessed — and the
  -- unfolding lemmas can no longer reach it. This is the file's standing rule
  -- about `with` on a recursive call, met from the consumer's side.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Two accessibility witnesses at the same measure give the same composite.
  -- Not a fact about `Acc` in general — it is proved by the recursion, exactly
  -- as the unit laws are.
  match-comp-acc-irr
    : ∀ {Γ Δ Θ}
    → (a b : Acc _<_ (length Γ))
    → (m : Match Ob Γ Δ)
    → (n : Match Ob Δ Θ)
    → match-comp-acc a m n ≡ match-comp-acc b m n
  match-comp-acc-irr a b [] [] = refl
  match-comp-acc-irr (acc r) (acc s) (cap i m) n =
    cong (cap i) (match-comp-acc-irr _ _ m n)
  match-comp-acc-irr (acc r) (acc s) (i ∷ m) n with match-remove i n
  ... | through spot body = cong (spot ∷_) (match-comp-acc-irr _ _ m body)
  ... | capped ins body with match-unhit ins m
  ...   | unhit p m′ = cong (cap p) (match-comp-acc-irr _ _ m′ body)

  -- A cap survives composition untouched: it is already the composite's own.
  match-comp-cap
    : ∀ {x y xs xs′ ys Θ}
    → (i : Insert Ob y xs xs′)
    → (m : Match Ob xs ys)
    → (n : Match Ob ys Θ)
    → match-comp (cap {x = x} i m) n ≡ cap i (match-comp m n)
  match-comp-cap i m n = cong (cap i) (match-comp-acc-irr _ _ m n)

  -- A source that ran through takes its partner's partner.
  match-comp-∷-through
    : ∀ {x xs ys zs Θ rest}
    → (i : Insert Ob x ys zs)
    → (m : Match Ob xs ys)
    → (n : Match Ob zs Θ)
    → (spot : Insert Ob x rest Θ)
    → (body : Match Ob ys rest)
    → match-remove i n ≡ through spot body
    → match-comp (i ∷ m) n ≡ spot ∷ match-comp m body
  match-comp-∷-through i m n spot body e with match-remove i n | e
  ... | .(through spot body) | refl =
    cong (spot ∷_) (match-comp-acc-irr _ _ m body)

  -- And a source whose partner the second wiring capped fuses two through
  -- strands into one cap of the composite.
  match-comp-∷-capped
    : ∀ {x y xs xs′ ys zs Θ rest}
    → (i : Insert Ob x ys zs)
    → (m : Match Ob xs ys)
    → (n : Match Ob zs Θ)
    → (ins : Insert Ob y xs′ ys)
    → (body : Match Ob xs′ Θ)
    → (p : Insert Ob y rest xs)
    → (m′ : Match Ob rest xs′)
    → match-remove i n ≡ capped ins body
    → match-unhit ins m ≡ unhit p m′
    → match-comp (i ∷ m) n ≡ cap p (match-comp m′ body)
  match-comp-∷-capped i m n ins body p m′ e e′ with match-remove i n | e
  ... | .(capped ins body) | refl with match-unhit ins m | e′
  ...   | .(unhit p m′) | refl =
    cong (cap p) (match-comp-acc-irr _ _ m′ body)

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT COMPOSITION DOES TO A REMOVAL AND TO AN INVERSE LOOKUP. These are the
  -- shape of the two facts associativity turns on, written as operations so
  -- that the facts can be stated as equations rather than as case analyses.
  --
  -- Every clause is an APPLICATION and every split is on an ARGUMENT, so a
  -- later `cong` reaches through them — the same discipline `split`, `origin`
  -- and the listing algebra follow, and for the same reason.
  --
  -- `Removal` already carries its intermediate list as a constructor argument,
  -- so these are homogeneous equations with nothing to transport along. That
  -- is the device the exchange's `Tower` introduced, arriving where it was
  -- predicted to.
  -- ══════════════════════════════════════════════════════════════════════════

  -- The second wiring capped the strand's partner, so the partner is one of
  -- ITS sources — a sink of the first — and has to be traced back through the
  -- first wiring. Two through strands fuse into one cap.
  removal-fuse
    : ∀ {x y Γ Γ′ Θ}
    → Match Ob Γ′ Θ
    → Unhit y Γ Γ′
    → Removal x Γ Θ
  removal-fuse body₂ (unhit p body′) = capped p (match-comp body′ body₂)

  -- The strand ran through the first wiring to a sink; what the second wiring
  -- does with that sink is the whole of the case analysis.
  removal-plug
    : ∀ {x Γ rest Θ}
    → Match Ob Γ rest
    → Removal x rest Θ
    → Removal x Γ Θ
  removal-plug body (through spot₂ body₂) = through spot₂ (match-comp body body₂)
  removal-plug body (capped ins₂ body₂) = removal-fuse body₂ (match-unhit ins₂ body)

  -- Composing a removal with a further wiring. A cap already made by the first
  -- wiring stands; a through strand is passed to the second.
  removal-comp
    : ∀ {x Γ Δ Θ}
    → Removal x Γ Δ
    → Match Ob Δ Θ
    → Removal x Γ Θ
  removal-comp (through spot body) o = removal-plug body (match-remove spot o)
  removal-comp (capped ins body) o = capped ins (match-comp body o)

  -- Carrying an inverse lookup's result forward along a further wiring.
  unhit-post
    : ∀ {y Γ Δ Θ}
    → Match Ob Δ Θ
    → Unhit y Γ Δ
    → Unhit y Γ Θ
  unhit-post o (unhit q m′) = unhit q (match-comp m′ o)

  -- and the composite's own inverse lookup: trace back through the second
  -- wiring, then trace that result back through the first.
  unhit-comp
    : ∀ {y Γ Δ Θ}
    → Match Ob Γ Δ
    → Unhit y Δ Θ
    → Unhit y Γ Θ
  unhit-comp m (unhit p′ body′) = unhit-post body′ (match-unhit p′ m)

  -- ══════════════════════════════════════════════════════════════════════════
  -- ASSOCIATIVITY, OVER THE TWO COMMUTATION LAWS IT TURNS ON.
  --
  -- The theorem reduces to exactly two facts, and this module is that
  -- reduction — machine-checked, so what remains of the wiring category's
  -- associativity is those two and nothing else:
  --
  --   * `match-remove-comp` — removing a source from a composite is removing
  --     it from the first wiring and then composing that removal with the
  --     second;
  --   * `match-unhit-comp` — tracing a sink back through a composite is
  --     tracing it back through the second and then through the first.
  --
  -- Both are statements about the listing algebra alone, with no accessibility
  -- witness in sight, which is what the interface above bought. They are
  -- PARAMETERS rather than comments so that what is assumed is in the
  -- signature, and they are discharged at the empty colour set below the
  -- examples — which shows the module is not vacuous and nothing more.
  -- ══════════════════════════════════════════════════════════════════════════

  module _
    (match-remove-comp
      : ∀ {x Γ Γˣ Δ Θ}
      → (i : Insert Ob x Γ Γˣ)
      → (n : Match Ob Γˣ Δ)
      → (o : Match Ob Δ Θ)
      → match-remove i (match-comp n o) ≡ removal-comp (match-remove i n) o)
    (match-unhit-comp
      : ∀ {y Γ Δ Θ Θˣ}
      → (j : Insert Ob y Θ Θˣ)
      → (m : Match Ob Γ Δ)
      → (n : Match Ob Δ Θˣ)
      → match-unhit j (match-comp m n) ≡ unhit-comp m (match-unhit j n))
    where

    -- The recursion is on the source list's length, as `match-comp`'s own is.
    -- The five clauses below are its three cases and the two further splits the
    -- cap forces, each one an auxiliary taking the scrutinee and its equation.
    mutual

      match-comp-assoc-acc
        : ∀ {Γ Δ Θ Ξ}
        → Acc _<_ (length Γ)
        → (m : Match Ob Γ Δ)
        → (n : Match Ob Δ Θ)
        → (o : Match Ob Θ Ξ)
        → match-comp (match-comp m n) o ≡ match-comp m (match-comp n o)
      match-comp-assoc-acc a [] [] [] = refl
      match-comp-assoc-acc (acc rec) (cap i m) n o =
        begin⟨ bundle (≡ˢ _) ⟩
          [ z ↦ match-comp z o ]· match-comp (cap i m) n
        ≈·⟨ match-comp-cap i m n ⟩
          match-comp (cap i (match-comp m n)) o
        ≈⟨ match-comp-cap i (match-comp m n) o ⟩
          [ x ↦ cap i x ]· match-comp (match-comp m n) o
        ≈·⟨ match-comp-assoc-acc (rec (insert-shrink i)) m n o ⟩
          cap i (match-comp m (match-comp n o))
        ≈⁻¹⟨ match-comp-cap i m (match-comp n o) ⟩
          match-comp (cap i m) (match-comp n o)
        ∎
      match-comp-assoc-acc a (i ∷ m) n o =
        assoc-∷ a i m n o (match-remove i n) refl

      -- the leading source either ran through the first wiring or was capped
      assoc-∷
        : ∀ {x xs ys zs Θ Ξ}
        → Acc _<_ (length (x ∷ xs))
        → (i : Insert Ob x ys zs)
        → (m : Match Ob xs ys)
        → (n : Match Ob zs Θ)
        → (o : Match Ob Θ Ξ)
        → (r : Removal x ys Θ)
        → match-remove i n ≡ r
        → match-comp (match-comp (i ∷ m) n) o ≡ match-comp (i ∷ m) (match-comp n o)
      assoc-∷ a i m n o (through spot body) eq₁ =
        assoc-∷-through a i m n o spot body eq₁ (match-remove spot o) refl
      assoc-∷ a i m n o (capped ins body) eq₁ =
        assoc-∷-capped a i m n o ins body eq₁ (match-unhit ins m) refl

      -- CAPPED BY THE FIRST WIRING: the cap is the composite's, and both sides
      -- are that cap over a composite one source shorter
      assoc-∷-capped
        : ∀ {x y xs xs′ ys zs Θ Ξ}
        → Acc _<_ (length (x ∷ xs))
        → (i : Insert Ob x ys zs)
        → (m : Match Ob xs ys)
        → (n : Match Ob zs Θ)
        → (o : Match Ob Θ Ξ)
        → (ins : Insert Ob y xs′ ys)
        → (body : Match Ob xs′ Θ)
        → match-remove i n ≡ capped ins body
        → (u : Unhit y xs xs′)
        → match-unhit ins m ≡ u
        → match-comp (match-comp (i ∷ m) n) o ≡ match-comp (i ∷ m) (match-comp n o)
      assoc-∷-capped (acc rec) i m n o ins body eq₁ (unhit p m′) eq₄ =
        begin⟨ bundle (≡ˢ _) ⟩
          [ z ↦ match-comp z o ]· match-comp (i ∷ m) n
        ≈·⟨ match-comp-∷-capped i m n ins body p m′ eq₁ eq₄ ⟩
          match-comp (cap p (match-comp m′ body)) o
        ≈⟨ match-comp-cap p (match-comp m′ body) o ⟩
          [ x ↦ cap p x ]· match-comp (match-comp m′ body) o
        ≈·⟨ match-comp-assoc-acc (rec (insert-shrink p)) m′ body o ⟩
          cap p (match-comp m′ (match-comp body o))
        ≈⁻¹⟨ match-comp-∷-capped i m (match-comp n o) ins (match-comp body o) p m′
          (begin⟨ bundle (≡ˢ _) ⟩
            match-remove i (match-comp n o)
          ≈⟨ match-remove-comp i n o ⟩
            [ r ↦ removal-comp r o ]· match-remove i n
          ≈·⟨ eq₁ ⟩
            removal-comp (capped ins body) o
          ∎)
          eq₄
        ⟩
          match-comp (i ∷ m) (match-comp n o)
        ∎

      -- RAN THROUGH THE FIRST: what becomes of it is the second wiring's
      -- business, and it is the same question one wiring along
      assoc-∷-through
        : ∀ {x xs ys zs Θ Ξ rest}
        → Acc _<_ (length (x ∷ xs))
        → (i : Insert Ob x ys zs)
        → (m : Match Ob xs ys)
        → (n : Match Ob zs Θ)
        → (o : Match Ob Θ Ξ)
        → (spot : Insert Ob x rest Θ)
        → (body : Match Ob ys rest)
        → match-remove i n ≡ through spot body
        → (r₂ : Removal x rest Ξ)
        → match-remove spot o ≡ r₂
        → match-comp (match-comp (i ∷ m) n) o ≡ match-comp (i ∷ m) (match-comp n o)
      assoc-∷-through (acc rec) i m n o spot body eq₁ (through spot₂ body₂) eq₂ =
        begin⟨ bundle (≡ˢ _) ⟩
          [ z ↦ match-comp z o ]· match-comp (i ∷ m) n
        ≈·⟨ match-comp-∷-through i m n spot body eq₁ ⟩
          match-comp (spot ∷ match-comp m body) o
        ≈⟨ match-comp-∷-through spot (match-comp m body) o spot₂ body₂ eq₂ ⟩
          [ x ↦ spot₂ ∷ x ]· match-comp (match-comp m body) body₂
        ≈·⟨ match-comp-assoc-acc (rec (n<1+n _)) m body body₂ ⟩
          spot₂ ∷ match-comp m (match-comp body body₂)
        ≈⁻¹⟨ match-comp-∷-through i m (match-comp n o) spot₂ (match-comp body body₂)
          (begin⟨ bundle (≡ˢ _) ⟩
            match-remove i (match-comp n o)
          ≈⟨ match-remove-comp i n o ⟩
            [ r ↦ removal-comp r o ]· match-remove i n
          ≈·⟨ eq₁ ⟩
            [ r ↦ removal-plug body r ]· match-remove spot o
          ≈·⟨ eq₂ ⟩
            through spot₂ (match-comp body body₂)
          ∎)
        ⟩
          match-comp (i ∷ m) (match-comp n o)
        ∎
      assoc-∷-through a i m n o spot body eq₁ (capped ins₂ body₂) eq₂ =
        assoc-∷-fuse a i m n o spot body eq₁ ins₂ body₂ eq₂
          (match-unhit ins₂ body) refl

      -- AND CAPPED BY THE SECOND, which is the case the cap introduced: the
      -- partner is a source of the second wiring, so it is traced back through
      -- the first — twice, once to reach that source's own partner
      assoc-∷-fuse
        : ∀ {x y xs ys zs Θ Ξ rest rest′}
        → Acc _<_ (length (x ∷ xs))
        → (i : Insert Ob x ys zs)
        → (m : Match Ob xs ys)
        → (n : Match Ob zs Θ)
        → (o : Match Ob Θ Ξ)
        → (spot : Insert Ob x rest Θ)
        → (body : Match Ob ys rest)
        → match-remove i n ≡ through spot body
        → (ins₂ : Insert Ob y rest′ rest)
        → (body₂ : Match Ob rest′ Ξ)
        → match-remove spot o ≡ capped ins₂ body₂
        → (u : Unhit y ys rest′)
        → match-unhit ins₂ body ≡ u
        → match-comp (match-comp (i ∷ m) n) o ≡ match-comp (i ∷ m) (match-comp n o)
      assoc-∷-fuse a i m n o spot body eq₁ ins₂ body₂ eq₂ (unhit p body′) eq₃ =
        assoc-∷-fuse′ a i m n o spot body eq₁ ins₂ body₂ eq₂ p body′ eq₃
          (match-unhit p m) refl

      assoc-∷-fuse′
        : ∀ {x y xs ys ys′ zs Θ Ξ rest rest′}
        → Acc _<_ (length (x ∷ xs))
        → (i : Insert Ob x ys zs)
        → (m : Match Ob xs ys)
        → (n : Match Ob zs Θ)
        → (o : Match Ob Θ Ξ)
        → (spot : Insert Ob x rest Θ)
        → (body : Match Ob ys rest)
        → match-remove i n ≡ through spot body
        → (ins₂ : Insert Ob y rest′ rest)
        → (body₂ : Match Ob rest′ Ξ)
        → match-remove spot o ≡ capped ins₂ body₂
        → (p : Insert Ob y ys′ ys)
        → (body′ : Match Ob ys′ rest′)
        → match-unhit ins₂ body ≡ unhit p body′
        → (u : Unhit y xs ys′)
        → match-unhit p m ≡ u
        → match-comp (match-comp (i ∷ m) n) o ≡ match-comp (i ∷ m) (match-comp n o)
      assoc-∷-fuse′ (acc rec) i m n o spot body eq₁ ins₂ body₂ eq₂ p body′ eq₃
        (unhit q m₀′) eq₄ =
        begin⟨ bundle (≡ˢ _) ⟩
          [ z ↦ match-comp z o ]· match-comp (i ∷ m) n
        ≈·⟨ match-comp-∷-through i m n spot body eq₁ ⟩
          match-comp (spot ∷ match-comp m body) o
        ≈⟨ match-comp-∷-capped spot (match-comp m body) o ins₂ body₂ q (match-comp m₀′ body′) eq₂
          (begin⟨ bundle (≡ˢ _) ⟩
            match-unhit ins₂ (match-comp m body)
          ≈⟨ match-unhit-comp ins₂ m body ⟩
            [ u ↦ unhit-comp m u ]· match-unhit ins₂ body
          ≈·⟨ eq₃ ⟩
            [ u ↦ unhit-post body′ u ]· match-unhit p m
          ≈·⟨ eq₄ ⟩
            unhit q (match-comp m₀′ body′)
          ∎)
        ⟩
          [ x ↦ cap q x ]· match-comp (match-comp m₀′ body′) body₂
        ≈·⟨ match-comp-assoc-acc (rec (insert-shrink q)) m₀′ body′ body₂ ⟩
          cap q (match-comp m₀′ (match-comp body′ body₂))
        ≈⁻¹⟨ match-comp-∷-capped i m (match-comp n o) p (match-comp body′ body₂) q m₀′
          (begin⟨ bundle (≡ˢ _) ⟩
            match-remove i (match-comp n o)
          ≈⟨ match-remove-comp i n o ⟩
            [ r ↦ removal-comp r o ]· match-remove i n
          ≈·⟨ eq₁ ⟩
            [ r ↦ removal-plug body r ]· match-remove spot o
          ≈·⟨ eq₂ ⟩
            [ u ↦ removal-fuse body₂ u ]· match-unhit ins₂ body
          ≈·⟨ eq₃ ⟩
            capped p (match-comp body′ body₂)
          ∎)
          eq₄
        ⟩
          match-comp (i ∷ m) (match-comp n o)
        ∎

    -- ASSOCIATIVITY OF THE WIRING COMPOSITION, with the accessibility witness
    -- supplied. This is the `mon-α` of `Gandr.Shape.Structure.WIRING`.
    match-comp-assoc
      : ∀ {Γ Δ Θ Ξ}
      → (m : Match Ob Γ Δ)
      → (n : Match Ob Δ Θ)
      → (o : Match Ob Θ Ξ)
      → match-comp (match-comp m n) o ≡ match-comp m (match-comp n o)
    match-comp-assoc = match-comp-assoc-acc (<-wellFounded _)

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
  -- IT HOLDS IN GENERAL, over any wiring. An earlier revision proved it only
  -- where the wiring underneath was flow-through, because the remaining case —
  -- a cut over a wiring that ALREADY caps — pushes the two new ports and the
  -- existing cap's partner past each other in two orders whose intermediate
  -- lists differ. That is `insert-swap-braid`, and with the exchange's laws
  -- stated as the listing algebra's symmetry the case is no longer a chase: it
  -- is one `cong` over the braid, under one `cong` over the recursion. The
  -- hypothesis is gone, and what removed it was naming the structure rather
  -- than finding a trick.
  cap-swap
    : ∀ {x y Γ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (j : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Δ)
    → match-cap i j m
        ≡ match-cap
            (Exchange.outer (insert-swap i j))
            (Exchange.inner (insert-swap i j))
            m
  cap-swap head j m = refl
  cap-swap (tail i) head m = refl
  cap-swap (tail i) (tail j) (k ∷ m) = cong (k ∷_) (cap-swap i j m)
  cap-swap (tail i) (tail j) (cap k m) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ c ↦ cap (Tower.peak (tower-lo i j k)) c ]·
        match-cap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m
    ≈·⟨ cap-swap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m ⟩
      [ t ↦ cap (Tower.peak t) (match-cap (Tower.step t) (Tower.base t) m) ]· tower-lo i j k
    ≈·⟨ insert-swap-braid i j k ⟩
      cap
        (Tower.peak (tower-hi i j k))
        (match-cap
          (Tower.step (tower-hi i j k))
          (Tower.base (tower-hi i j k))
          m)
    ∎

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

-- ══════════════════════════════════════════════════════════════════════════════
-- THE TWO COMMUTATION LAWS ARE SATISFIABLE, and this is the whole of what that
-- shows. Over the EMPTY colour set every `Insert` is absurd — each of its
-- constructors carries a colour, and there are none — so both laws hold
-- vacuously and the associativity theorem is available there.
--
-- A parameterized module type-checks whether or not its hypotheses can ever be
-- met, so a module whose assumptions are unsatisfiable is green and vacuous.
-- This is what rules that out, and it is NO evidence for the general case: the
-- wiring at `Ob = ⊥` has one profile and one matching. Proving the two laws in
-- general is the open work, and it is the whole of what the wiring category's
-- associativity still owes.
-- ══════════════════════════════════════════════════════════════════════════════

match-remove-comp-⊥
  : ∀ {ℓ} {x : ⊥° {ℓ}} {Γ Γˣ Δ Θ}
  → (i : Insert ⊥° x Γ Γˣ)
  → (n : Match ⊥° Γˣ Δ)
  → (o : Match ⊥° Δ Θ)
  → match-remove i (match-comp n o) ≡ removal-comp (match-remove i n) o
match-remove-comp-⊥ {x = ()} i n o

match-unhit-comp-⊥
  : ∀ {ℓ} {y : ⊥° {ℓ}} {Γ Δ Θ Θˣ}
  → (j : Insert ⊥° y Θ Θˣ)
  → (m : Match ⊥° Γ Δ)
  → (n : Match ⊥° Δ Θˣ)
  → match-unhit j (match-comp m n) ≡ unhit-comp m (match-unhit j n)
match-unhit-comp-⊥ {y = ()} j m n

-- and so the theorem, at the witness that discharges it
match-comp-assoc-⊥
  : ∀ {ℓ} {Γ Δ Θ Ξ}
  → (m : Match (⊥° {ℓ}) Γ Δ)
  → (n : Match ⊥° Δ Θ)
  → (o : Match ⊥° Θ Ξ)
  → match-comp (match-comp m n) o ≡ match-comp m (match-comp n o)
match-comp-assoc-⊥ = match-comp-assoc match-remove-comp-⊥ match-unhit-comp-⊥

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

-- AND THE SAME FACT OFF THE GENERAL THEOREM rather than off this shape's own
-- emptiness. The argument above works because `two-points` has no edge to rule
-- out; `merge-disconnected` rules out a crossing edge whatever the operands'
-- wirings do, so it says the same thing without looking at the shape.
two-points-apart : ¬ Connected two-points
two-points-apart = merge-disconnected nil nil point point here here


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

-- AND THIS IS WHERE THE GENERAL DISCONNECTION THEOREM EARNS ITS KEEP. Unlike
-- `two-points`, this composite HAS edges — each corolla's ports are wired to
-- the merged interface — so ruling out a crossing is a claim about them rather
-- than about their absence, which is the case the worked example could not
-- cover and the reason `merge-apart` had to be proved rather than exhibited.
corollas-apart-edge : Edg corollas-apart
corollas-apart-edge = here

corollas-apart-disconnected : ¬ Connected corollas-apart
corollas-apart-disconnected =
  merge-disconnected
    (append-graph 𝟚 𝟙)
    (append-graph 𝟙 𝟙)
    (corolla 𝟚 𝟙)
    (corolla 𝟙 𝟙)
    here
    here

-- AND THE CONVERSE HALF, EXERCISED. Every operand above has a single vertex, so
-- "each operand's own adjacencies survive" says nothing about them. `bigon` is
-- two vertices joined by two parallel edges, so merging it with a point is the
-- smallest case where the claim has content: its own adjacency must reappear in
-- a composite that now has a second component beside it.
bigon-point : Shape ⊤ [] []
bigon-point = merge nil nil bigon point

bigon-point-joined
  : Reach
      bigon-point
      (vtxˡ nil nil bigon point g₀)
      (vtxˡ nil nil bigon point g₁)
bigon-point-joined =
  reach-left nil nil bigon point (onward stop (adj here (along bigon-att₀)))

-- and the composite is disconnected all the same, which is the other half. The
-- two together are `merge-components` at this shape: three vertices, two
-- components, and which is which is read off the side.
bigon-point-disconnected : ¬ Connected bigon-point
bigon-point-disconnected = merge-disconnected nil nil bigon point g₀ here

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
