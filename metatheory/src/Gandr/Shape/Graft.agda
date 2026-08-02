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
-- (`Cat`) speaks it. Here `graft` is built from eight operations — `preplug`,
-- `lwhisk`, `wire-in`, `match-comp`, `match-lwhisk`, `match-insert`,
-- `insert-shift`, `match-unhit`, over `match-remove` and `insert-swap` — and
-- the witness discipline does not stop at the outermost one: a defined function
-- may not head a matchable index, and each auxiliary's result sits in the index
-- of the next one's graph. So "speak the multiplication through its graph"
-- propagates to every operation the composite is assembled from.
--
-- `match-unhit` is in that list because the cut put it there: `match-comp`'s
-- fused clause traces a capped strand back through the first wiring. An earlier
-- revision of this paragraph counted nine, from before the cut existed.
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
-- THE LISTING ALGEBRA IS SYMMETRIC, and it says so ONCE. `Tower` is a tower of
-- nested insertions indexed by the LIST of colours it puts in, so one structure
-- serves every arity; `tower-swap` names its two lowest layers in the other
-- order; and the whole of what the symmetry is, is that this generator satisfies
-- the SYMMETRIC group's Coxeter presentation —
--
--   `tower-swap-invol`  naming twice returns
--   `tower-swap-braid`  the braid relation `σ₁σ₂σ₁ = σ₂σ₁σ₂` on three layers
--   `tower-swap-far`    layers two apart do not interfere
--
-- Every coherence above them is a WORD EQUATION derived by chaining these three,
-- at any arity, with no new structure. `tower-coh⁴` is the four-layer one and is
-- five steps: commute a far pair, braid, braid a layer up, commute back.
--
-- AT WHICH ARITY THE PRESENTATION IS CHECKED, since naming a presentation claims
-- the relations are ALL of them and that claim is arity-scoped. All three are
-- stated and proved over a tower carrying an ARBITRARY tail, and the third is
-- stated against an arbitrary action two layers up, so what is proved holds at
-- every arity rather than at the arity a consumer happened to want. That is the
-- difference from the ladder this replaced, whose relations were complete at
-- three layers and silently incomplete at four — three layers admit no
-- non-adjacent pair, so the missing relation had no instance to fail at.
--
-- WHAT IS NOT MECHANIZED, and must not be read into the word "presentation".
-- That these relations HOLD is proved here. That they GENERATE every relation
-- of the symmetric group is Coxeter's theorem, and it is used as a guide for
-- finding a chain, not as a decision procedure: each coherence is still
-- exhibited as an explicit word chain, and a normal form for the words is not
-- built. Building one is what would make a coherence hold by computation, and
-- nothing so far has needed it.
--
-- An earlier revision called the three-way relation a hexagon; it is not one,
-- since a hexagon is the braiding against an associator and there is no tensor
-- here to associate. And an earlier revision reached each arity by writing the
-- next fixed-arity RECORD with its own pair of routes and its own coherence.
-- That ladder was measured and does not terminate cheaply — a fifth rung is
-- owed by two cuts commuting, and its bases are a four-layer coherence for a
-- permutation the fourth rung never performs, so the ladder gains a family
-- rather than a rung. The presentation replaces it.
--
-- WHAT MADE THE PACKAGING POSSIBLE was already recorded here and not exploited:
-- the action moves no ELEMENT, only the order in which the positions were named.
-- So the layers are fixed in the final list and reordering them is a group
-- acting on a tower's layers. Of the three relations, the first two are the
-- fixed-arity rungs' own content migrated; the THIRD was never stated, and its
-- absence is exactly why the four-layer coherence's bases had looked like new
-- work rather than like far-apart transpositions commuting.
--
-- `Match` is NOT presented by permutations and that is not in question. What
-- takes the permutation vocabulary is the POSITIONS, one level below the
-- matchings.
--
-- AND THE TWO LOOKUPS ARE VIEWS OF THE TWO THREADINGS, which is the
-- characterization the composition proofs run on. `match-remove i` is inverse
-- to `removal→match i` and `match-unhit j` to `unhit→match j`, both round trips
-- proved, so `Removal` and `Unhit` lose nothing and a matching can be REBUILT
-- from either lookup. That is what makes a fact of the form "what does an
-- operation do to `o`, given what a lookup on `o` returned" reachable at all: a
-- lookup is a recursion and no lemma reaches past it, while a rebuild is a
-- construction and computes.
--
-- `match-cap-insert` — threading a cut and threading a wire COMMUTE, up to
-- reindexing by nested `insert-swap`s. No hypothesis, no composition and no
-- lookup appears in it, and it is the bottom of the development above: through
-- the views, both of the commutation laws below reduce to it. Its cut-meets-cut
-- clause is what asked for the four-layer coherence, and caps meeting caps is
-- where every coherence debt in this file has landed, `cap-swap`'s included.
--
-- `match-unhit-cut` and `match-comp-cut` are what it was proved FOR, and they
-- are the first two rungs above it. Tracing a sink back through a cut wiring
-- is tracing it back through the wiring underneath and re-applying the cut
-- (`unhit-recap`); and a cut commutes with composition on the left, because a
-- cut consumes no sink and the second wiring never sees it. The second is the
-- first proof in the file to mirror `match-comp`'s own recursion, and it pays
-- the same measure for the same reason: the fused case recurses on a matching
-- that `match-unhit` produced rather than on a subterm.
--
-- `match-insert-insert` and `match-insert-cap` complete that table's other two
-- entries, and the second of them is FREE. Two threaded wires commute, and the
-- arity rule predicts its cost exactly — a wire, a wire, and the head they
-- meet, so the braid, spent once per side of the matching. A threaded wire past
-- a threaded cut reads as a fresh four-layer debt and owes nothing: it is
-- `match-cap-insert` instantiated where its own right-hand side puts the
-- arguments, with the involution undoing the reindexing twice. So a member of
-- this family is free whenever the permutation it asks for is the INVERSE of
-- one already spent, and the arity rule bounds the cost of a fresh statement
-- rather than of every statement.
--
-- WHAT REMAINS OPEN, stated where a reader meets it. Associativity of
-- `match-comp` is a module parameter twice over — `match-remove-comp` and
-- `match-unhit-comp`, discharged at `Ob = ⊥` for satisfiability only. The cut
-- half of the route to them is the two lemmas above; the WIRE half is
-- instrumented but not closed. `match-comp-cut` closes because a cut leaves the
-- second wiring's sources alone, so the composite looks up the same source the
-- operand did. Threading a WIRE moves the lookup: the composite's leading
-- source sits at `Exchange.outer (insert-swap spot c)` where the operand's sat
-- at `spot`, so the two sides consult the second wiring at two DIFFERENT
-- positions. What closes that gap is not another coherence but two
-- construction-level statements in the family `match-cap-insert` belongs to —
-- what a lookup sees of a threading at a position APART from the one threaded.
-- This is the fact recorded as "removing two different sources in either
-- order", located precisely rather than estimated.
--
-- BOTH ARE WRITTEN NOW, and by ONE instrument. `match-remove-insert-apart` and
-- `match-unhit-insert-apart` say it for a threaded WIRE, on both lookups;
-- `match-remove-cut-apart` and `match-remove-cut-same` say it for a threaded
-- CUT, split on the verdict of comparing the removed source with the cut's
-- inner port. Every one of them is the same three steps — rebuild the wiring
-- from the lookup's own value, commute the threading past the rebuild, read the
-- answer back through the round trip — so each costs exactly one entry of the
-- commutation table and nothing else.
--
-- `removal-thread` and `unhit-thread` are what "thread a wire through what a
-- lookup left" is, and `removal-recut` is the same for a cut; `removal-tail`
-- and `unhit-tail` turn out to BE the first two at the head rather than
-- separate devices. The inverse lookup's half was not claimed symmetric with
-- the removal's; on this step it is, and it is `match-insert-insert` twice.
--
-- WHAT THE CUT HALF COST, since it was the reason the presentation was taken.
-- Its rebuild turns the lookup's capped branch into a second `match-cap`, so it
-- lands on two threaded cuts commuting — a cut, a cut, and the head they meet,
-- FIVE layers. Under the fixed-arity records that was a new rung AND a new
-- permutation family. Under the presentation it is one more word:
-- `tower-cycle-pair` says the pair exchange commutes with the cycle one height
-- down, it is four applications of a single conjugation law, and that law is
-- three relation steps. The alternative route — a direct induction staying at
-- four layers and paying in view algebra instead — was not needed and is not
-- written.
--
-- The earlier estimate that no further coherence was owed on this route was
-- withdrawn and stays withdrawn: one was owed. What the packaging changed is
-- what it cost to pay.
--
-- AND THE ARITY OF SUCH A DEBT IS NOT ARBITRARY, which is worth knowing before
-- reaching for heavier machinery: it is the number of positions the operations
-- in play THREAD, plus the head they meet. `match-insert` threads one and
-- `match-cap` threads two, while the block operations — `insert-shift` and
-- `match-lwhisk` — thread NONE, being pure `tail` and `head` with no exchange
-- in them at all. So `cap-swap` is a cut plus an underlying cap, three; the
-- residual above is a cut plus a wire plus an underlying cap, four; and
-- whiskering and merging contribute nothing to this ladder however large their
-- blocks. A fifth layer needs two cuts to commute.
--
-- What the arity now bounds is the LENGTH OF A WORD, not the cost of a new
-- structure: since the presentation is complete, a statement at any arity is
-- derived by chaining the same three relations. The rule also bounds a FRESH
-- statement's cost and not every statement's, which `match-insert-cap` is the
-- standing counterexample to: read fresh it is four, and it is free.
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
  using (step-≈·⁻¹)
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
  -- THE EXCHANGE'S OWN LAW. What `insert-swap` is, is a SYMMETRY on towers of
  -- nested insertions, and the next section is that characterization stated
  -- once and for all arities. This one law comes first because it is the only
  -- one an ordinary consumer of `insert-swap` needs, and because most of the
  -- file's proofs spend it directly rather than through the action.
  --
  -- The three readings in the incidence section below — `swap-slotˡ`,
  -- `swap-slotʳ`, `swap-past` — say the same thing from the other side, and
  -- they are what the packaging rests on: the action moves no ELEMENT, only the
  -- order in which the positions were named.
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

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE ACTION, PACKAGED ONCE. A tower of nested insertions is indexed by the
  -- LIST of colours it puts in, so one structure serves every arity; the
  -- intermediate lists are constructor ARGUMENTS, which is `Exchange`'s device
  -- carried up, and it is what makes two towers over the same three indices
  -- comparable by an ordinary equation rather than through a transport.
  --
  -- WHAT THIS REPLACES, AND WHY IT WAS TAKEN. Three fixed-arity records with a
  -- bespoke pair of routes and a bespoke coherence apiece stood here — two
  -- layers, then three, then four — and each rung was reached by writing the
  -- next record. The ladder was measured and does NOT terminate cheaply: a
  -- fifth rung is owed by two cuts commuting, its bases are a FOUR-layer
  -- coherence for a permutation the fourth rung never performs, and the ladder
  -- would gain a family rather than a rung. So the action is packaged instead.
  --
  -- WHAT MAKES THE PACKAGING POSSIBLE is the observation the fixed-arity
  -- sections already recorded and did not exploit: the action moves no ELEMENT,
  -- only the order in which the positions were named. So the layers are
  -- physically fixed in `dˣ` and reordering them is a SYMMETRIC GROUP acting on
  -- a tower's layers — presented, as symmetric groups are, by adjacent
  -- transpositions and three relations. `tower-swap` is the generator, and
  --
  --   * `tower-swap-invol` — naming twice returns;
  --   * `tower-swap-braid` — the braid relation on three adjacent layers;
  --   * `tower-swap-far`  — layers two apart do not interfere;
  --
  -- are the relations. Every coherence above them is a WORD EQUATION and is
  -- derived by chaining these three, at any arity, with no new structure. The
  -- four-layer coherence is `tower-coh⁴` below and is five steps; the fifth and
  -- every later one is the same kind of chain.
  --
  -- The first two are the fixed-arity rungs' own content, migrated: the braid
  -- is the same four-clause induction `insert-swap-braid` was. The THIRD was
  -- never stated, and it is the one that makes the presentation complete —
  -- it holds by `refl`, and its absence is why the fourth rung's bases looked
  -- like new work.
  --
  -- ON THE NAME, because this coherence has been called a hexagon and it is
  -- not one. A hexagon is the braiding against an ASSOCIATOR, and there is no
  -- tensor here to associate: inserting is not a monoidal product on lists.
  -- What the three-way relation is, is the braid relation `σ₁σ₂σ₁ = σ₂σ₁σ₂` —
  -- the Yang–Baxter equation — for the family `insert-swap`; and together with
  -- the involution and the far commutation it is the SYMMETRIC group's Coxeter
  -- presentation, not the braid group's.
  --
  -- `Match` is NOT presented by permutations and that is not in question here.
  -- What takes the permutation vocabulary is the POSITIONS, one level below the
  -- matchings, which is where the redundancy a canonical presentation refuses
  -- has to be paid for instead.
  -- ══════════════════════════════════════════════════════════════════════════

  infixr 5 _◂_

  data Tower : List Ob → List Ob → List Ob → Set ℓ where
    -- nothing put in, so the two ends are the same list
    ⟨⟩
      : ∀ {ds}
      → Tower [] ds ds
    -- one more colour put in below everything already there
    _◂_
      : ∀ {x xs ds d dˣ}
      → Insert Ob x ds d
      → Tower xs d dˣ
      → Tower (x ∷ xs) ds dˣ

  -- Reindexing past an element every list leads with, which is `exchange-tail`
  -- at any arity at all.
  tower-tail
    : ∀ {v xs ds dˣ}
    → Tower xs ds dˣ
    → Tower xs (v ∷ ds) (v ∷ dˣ)
  tower-tail ⟨⟩ = ⟨⟩
  tower-tail (i ◂ t) = tail i ◂ tower-tail t

  -- Stacking one tower on another along the graph of concatenation, so that a
  -- statement proved about a FIXED number of layers reaches a tower with more
  -- above them. Over `Append` rather than `_++_`, for the reason `merge` is:
  -- no index has to unify against a computed list.
  tower-graft
    : ∀ {xs ys zs ds d dˣ}
    → Append Ob xs ys zs
    → Tower xs ds d
    → Tower ys d dˣ
    → Tower zs ds dˣ
  tower-graft nil ⟨⟩ t = t
  tower-graft (cons p) (i ◂ s) t = i ◂ tower-graft p s t

  -- Acting on everything above the lowest layer, which is how a generator at
  -- one height is read at the next. An application rather than a `with`, for
  -- the file's standing reason.
  tower-cong
    : ∀ {x xs ys ds dˣ}
    → (f : ∀ {d} → Tower xs d dˣ → Tower ys d dˣ)
    → Tower (x ∷ xs) ds dˣ
    → Tower (x ∷ ys) ds dˣ
  tower-cong f (i ◂ t) = i ◂ f t

  -- THE GENERATOR: the two lowest layers named in the other order. This IS
  -- `insert-swap`, read as an action rather than as a pair of positions.
  tower-swap
    : ∀ {x y xs ds dˣ}
    → Tower (x ∷ y ∷ xs) ds dˣ
    → Tower (y ∷ x ∷ xs) ds dˣ
  tower-swap (i ◂ (j ◂ t)) =
    Exchange.inner (insert-swap j i) ◂ (Exchange.outer (insert-swap j i) ◂ t)

  -- and the same generator read one and two layers up
  tower-swap₁
    : ∀ {x y z xs ds dˣ}
    → Tower (x ∷ y ∷ z ∷ xs) ds dˣ
    → Tower (x ∷ z ∷ y ∷ xs) ds dˣ
  tower-swap₁ = tower-cong tower-swap

  tower-swap₂
    : ∀ {w x y z xs ds dˣ}
    → Tower (w ∷ x ∷ y ∷ z ∷ xs) ds dˣ
    → Tower (w ∷ x ∷ z ∷ y ∷ xs) ds dˣ
  tower-swap₂ = tower-cong tower-swap₁

  -- RELATION ONE: naming twice returns. This is what separates a SYMMETRIC
  -- structure from a merely braided one, and it is `insert-swap-invol` with the
  -- tower carried along rather than a second statement.
  tower-swap-invol
    : ∀ {x y xs ds dˣ}
    → (t : Tower (x ∷ y ∷ xs) ds dˣ)
    → tower-swap (tower-swap t) ≡ t
  tower-swap-invol (i ◂ (j ◂ t)) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ Exchange.inner e ◂ (Exchange.outer e ◂ t) ]·
        insert-swap
          (Exchange.outer (insert-swap j i))
          (Exchange.inner (insert-swap j i))
    ≈·⟨ insert-swap-invol j i ⟩
      i ◂ (j ◂ t)
    ∎

  -- RELATION TWO, at exactly three layers, which is where its induction lives:
  -- three of the four cases hold by `refl` — the reordering is forced as soon
  -- as any of the three positions is at the front — and the fourth is the
  -- recursion under `tail`, so the whole relation costs one `cong`.
  --
  -- The `refl` at `(tail i) (tail j) head` is worth naming: it is eta for
  -- `Exchange` doing the work, since one route reaches the tower as an
  -- `insert-swap` and the other reaches it as that same swap's components put
  -- back together.
  tower-swap-braid⋆
    : ∀ {x y z ds d₁ d₂ dˣ}
    → (i : Insert Ob x ds d₁)
    → (j : Insert Ob y d₁ d₂)
    → (k : Insert Ob z d₂ dˣ)
    → tower-swap (tower-swap₁ (tower-swap (i ◂ j ◂ k ◂ ⟨⟩)))
      ≡ tower-swap₁ (tower-swap (tower-swap₁ (i ◂ j ◂ k ◂ ⟨⟩)))
  tower-swap-braid⋆ i j head = refl
  tower-swap-braid⋆ i head (tail k) = refl
  tower-swap-braid⋆ head (tail j) (tail k) = refl
  tower-swap-braid⋆ (tail i) (tail j) (tail k) =
    cong tower-tail (tower-swap-braid⋆ i j k)

  -- and with whatever else the tower carries above them
  tower-swap-braid
    : ∀ {x y z xs ds dˣ}
    → (t : Tower (x ∷ y ∷ z ∷ xs) ds dˣ)
    → tower-swap (tower-swap₁ (tower-swap t))
      ≡ tower-swap₁ (tower-swap (tower-swap₁ t))
  tower-swap-braid (i ◂ (j ◂ (k ◂ t))) =
    cong
      (λ u → tower-graft (cons (cons (cons nil))) u t)
      (tower-swap-braid⋆ i j k)

  -- the same relation read one layer up, which every word above three layers
  -- needs and which costs one `cong`
  tower-swap-braid₁
    : ∀ {w x y z xs ds dˣ}
    → (t : Tower (w ∷ x ∷ y ∷ z ∷ xs) ds dˣ)
    → tower-swap₁ (tower-swap₂ (tower-swap₁ t))
      ≡ tower-swap₂ (tower-swap₁ (tower-swap₂ t))
  tower-swap-braid₁ (i ◂ t) = cong (i ◂_) (tower-swap-braid t)

  -- RELATION THREE: layers two apart do not interfere. Never stated while the
  -- rungs were fixed-arity records, and it is the one that completes the
  -- presentation — without it a four-layer coherence's bases read as new work
  -- rather than as far-apart transpositions commuting.
  tower-swap-far
    : ∀ {x y xs ys ds dˣ}
    → (f : ∀ {d} → Tower xs d dˣ → Tower ys d dˣ)
    → (t : Tower (x ∷ y ∷ xs) ds dˣ)
    → tower-swap (tower-cong (tower-cong f) t)
      ≡ tower-cong (tower-cong f) (tower-swap t)
  tower-swap-far f (i ◂ (j ◂ t)) = refl

  -- THE FOUR-LAYER COHERENCE, AS A WORD. This is what used to be a record, a
  -- reindexing, two plantings, two routes and a five-clause induction. It is
  -- now the observation that both reorderings are reduced words for one
  -- permutation, and the chain IS that derivation, run in the group the three
  -- relations present: commute a far pair, braid, braid one layer up, commute
  -- the far pair back.
  tower-coh⁴
    : ∀ {w x y z xs ds dˣ}
    → (t : Tower (w ∷ x ∷ y ∷ z ∷ xs) ds dˣ)
    → tower-swap₁ (tower-swap (tower-swap₂ (tower-swap₁ (tower-swap t))))
      ≡ tower-swap₂ (tower-swap₁ (tower-swap (tower-swap₂ (tower-swap₁ t))))
  tower-coh⁴ t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ tower-swap₁ u ]·
        tower-swap (tower-swap₂ (tower-swap₁ (tower-swap t)))
    ≈·⟨ tower-swap-far tower-swap (tower-swap₁ (tower-swap t)) ⟩
      [ u ↦ tower-swap₁ (tower-swap₂ u) ]·
        tower-swap (tower-swap₁ (tower-swap t))
    ≈·⟨ tower-swap-braid t ⟩
      [ u ↦ u ]·
        tower-swap₁ (tower-swap₂ (tower-swap₁ (tower-swap (tower-swap₁ t))))
    ≈·⟨ tower-swap-braid₁ (tower-swap (tower-swap₁ t)) ⟩
      [ u ↦ tower-swap₂ (tower-swap₁ u) ]·
        tower-swap₂ (tower-swap (tower-swap₁ t))
    ≈·⁻¹⟨ tower-swap-far tower-swap (tower-swap₁ t) ⟩
      tower-swap₂ (tower-swap₁ (tower-swap (tower-swap₂ (tower-swap₁ t))))
    ∎

  tower-swap₃
    : ∀ {v w x y z xs ds dˣ}
    → Tower (v ∷ w ∷ x ∷ y ∷ z ∷ xs) ds dˣ
    → Tower (v ∷ w ∷ x ∷ z ∷ y ∷ xs) ds dˣ
  tower-swap₃ = tower-cong tower-swap₂

  -- the lifted relations this height needs
  tower-swap-braid₂
    : ∀ {v w x y z xs ds dˣ}
    → (t : Tower (v ∷ w ∷ x ∷ y ∷ z ∷ xs) ds dˣ)
    → tower-swap₂ (tower-swap₃ (tower-swap₂ t))
      ≡ tower-swap₃ (tower-swap₂ (tower-swap₃ t))
  tower-swap-braid₂ (i ◂ t) = cong (i ◂_) (tower-swap-braid₁ t)

  -- MOVING THE BOTTOM LAYER TO THE TOP of a five-layer tower.
  tower-cycle
    : ∀ {a b c d e xs ds dˣ}
    → Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ
    → Tower (b ∷ c ∷ d ∷ e ∷ a ∷ xs) ds dˣ
  tower-cycle t = tower-swap₃ (tower-swap₂ (tower-swap₁ (tower-swap t)))

  -- TRADING TWO ADJACENT PAIRS, which is what two cuts commuting asks of the
  -- positions.
  tower-pair
    : ∀ {b c d e xs ds dˣ}
    → Tower (b ∷ c ∷ d ∷ e ∷ xs) ds dˣ
    → Tower (d ∷ e ∷ b ∷ c ∷ xs) ds dˣ
  tower-pair t = tower-swap₁ (tower-swap₂ (tower-swap (tower-swap₁ t)))

  -- CONJUGATING A GENERATOR BY THE CYCLE lowers its height by one.
  tower-cycle-swap₁
    : ∀ {a b c d e xs ds dˣ}
    → (t : Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ)
    → tower-cycle (tower-swap₁ t) ≡ tower-swap (tower-cycle t)
  tower-cycle-swap₁ t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ tower-swap₃ (tower-swap₂ u) ]· tower-swap₁ (tower-swap (tower-swap₁ t))
    ≈·⁻¹⟨ tower-swap-braid t ⟩
      [ u ↦ tower-swap₃ u ]· tower-swap₂ (tower-swap (tower-swap₁ (tower-swap t)))
    ≈·⁻¹⟨ tower-swap-far tower-swap (tower-swap₁ (tower-swap t)) ⟩
      tower-swap₃ (tower-swap (tower-swap₂ (tower-swap₁ (tower-swap t))))
    ≈⁻¹⟨ tower-swap-far tower-swap₁ (tower-swap₂ (tower-swap₁ (tower-swap t))) ⟩
      tower-swap (tower-cycle t)
    ∎

  tower-swap-far₁
    : ∀ {v x y xs ys ds dˣ}
    → (f : ∀ {d} → Tower xs d dˣ → Tower ys d dˣ)
    → (t : Tower (v ∷ x ∷ y ∷ xs) ds dˣ)
    → tower-swap₁ (tower-cong (tower-cong (tower-cong f)) t)
      ≡ tower-cong (tower-cong (tower-cong f)) (tower-swap₁ t)
  tower-swap-far₁ f (i ◂ t) = cong (i ◂_) (tower-swap-far f t)

  tower-cycle-swap₂
    : ∀ {a b c d e xs ds dˣ}
    → (t : Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ)
    → tower-cycle (tower-swap₂ t) ≡ tower-swap₁ (tower-cycle t)
  tower-cycle-swap₂ t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ tower-swap₃ (tower-swap₂ (tower-swap₁ u)) ]· tower-swap (tower-swap₂ t)
    ≈·⟨ tower-swap-far tower-swap t ⟩
      [ u ↦ tower-swap₃ u ]· tower-swap₂ (tower-swap₁ (tower-swap₂ (tower-swap t)))
    ≈·⁻¹⟨ tower-swap-braid₁ (tower-swap t) ⟩
      tower-swap₃ (tower-swap₁ (tower-swap₂ (tower-swap₁ (tower-swap t))))
    ≈⁻¹⟨ tower-swap-far₁ tower-swap (tower-swap₂ (tower-swap₁ (tower-swap t))) ⟩
      tower-swap₁ (tower-cycle t)
    ∎

  tower-cycle-swap₃
    : ∀ {a b c d e xs ds dˣ}
    → (t : Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ)
    → tower-cycle (tower-swap₃ t) ≡ tower-swap₂ (tower-cycle t)
  tower-cycle-swap₃ t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ tower-swap₃ (tower-swap₂ (tower-swap₁ u)) ]· tower-swap (tower-swap₃ t)
    ≈·⟨ tower-swap-far tower-swap₁ t ⟩
      [ u ↦ tower-swap₃ (tower-swap₂ u) ]· tower-swap₁ (tower-swap₃ (tower-swap t))
    ≈·⟨ tower-swap-far₁ tower-swap (tower-swap t) ⟩
      tower-swap₃ (tower-swap₂ (tower-swap₃ (tower-swap₁ (tower-swap t))))
    ≈⁻¹⟨ tower-swap-braid₂ (tower-swap₁ (tower-swap t)) ⟩
      tower-swap₂ (tower-cycle t)
    ∎

  tower-cong-pair
    : ∀ {a b c d e xs ds dˣ}
    → (t : Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ)
    → tower-cong tower-pair t
      ≡ tower-swap₂ (tower-swap₃ (tower-swap₁ (tower-swap₂ t)))
  tower-cong-pair (i ◂ t) = refl

  -- THE PAIR EXCHANGE COMMUTES WITH THE CYCLE, one height down. This is the
  -- whole of what two threaded cuts commuting asks of the positions.
  tower-cycle-pair
    : ∀ {a b c d e xs ds dˣ}
    → (t : Tower (a ∷ b ∷ c ∷ d ∷ e ∷ xs) ds dˣ)
    → tower-pair (tower-cycle t) ≡ tower-cycle (tower-cong tower-pair t)
  tower-cycle-pair t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ u ↦ tower-swap₁ (tower-swap₂ (tower-swap u)) ]· tower-swap₁ (tower-cycle t)
    ≈·⁻¹⟨ tower-cycle-swap₂ t ⟩
      [ u ↦ tower-swap₁ (tower-swap₂ u) ]· tower-swap (tower-cycle (tower-swap₂ t))
    ≈·⁻¹⟨ tower-cycle-swap₁ (tower-swap₂ t) ⟩
      [ u ↦ tower-swap₁ u ]·
        tower-swap₂ (tower-cycle (tower-swap₁ (tower-swap₂ t)))
    ≈·⁻¹⟨ tower-cycle-swap₃ (tower-swap₁ (tower-swap₂ t)) ⟩
      tower-swap₁ (tower-cycle (tower-swap₃ (tower-swap₁ (tower-swap₂ t))))
    ≈⁻¹⟨ tower-cycle-swap₂ (tower-swap₃ (tower-swap₁ (tower-swap₂ t))) ⟩
      [ u ↦ tower-cycle u ]·
        tower-swap₂ (tower-swap₃ (tower-swap₁ (tower-swap₂ t)))
    ≈·⁻¹⟨ tower-cong-pair t ⟩
      tower-cycle (tower-cong tower-pair t)
    ∎

  -- READING A TOWER OFF AT A FIXED HEIGHT. The records this replaced were
  -- carried for their PROJECTIONS, which a list-indexed family cannot have
  -- because the intermediate lists are existential. These are the projections
  -- as eliminators instead — one clause each, no coherence, and nothing about
  -- them scales with arity, which is the whole difference from the ladder of
  -- records: a statement at a new height wants one more of these and no new
  -- structure and no new relation.
  tower₃
    : ∀ {x y z ds dˣ} {A : Set ℓ}
    → (∀ {d₁ d₂}
       → Insert Ob x ds d₁
       → Insert Ob y d₁ d₂
       → Insert Ob z d₂ dˣ
       → A)
    → Tower (x ∷ y ∷ z ∷ []) ds dˣ
    → A
  tower₃ f (a ◂ (b ◂ (c ◂ ⟨⟩))) = f a b c

  tower₄
    : ∀ {x y z w ds dˣ} {A : Set ℓ}
    → (∀ {d₁ d₂ d₃}
       → Insert Ob x ds d₁
       → Insert Ob y d₁ d₂
       → Insert Ob z d₂ d₃
       → Insert Ob w d₃ dˣ
       → A)
    → Tower (x ∷ y ∷ z ∷ w ∷ []) ds dˣ
    → A
  tower₄ f (a ◂ (b ◂ (c ◂ (d ◂ ⟨⟩)))) = f a b c d

  tower₅
    : ∀ {x y z w u ds dˣ} {A : Set ℓ}
    → (∀ {d₁ d₂ d₃ d₄}
       → Insert Ob x ds d₁
       → Insert Ob y d₁ d₂
       → Insert Ob z d₂ d₃
       → Insert Ob w d₃ d₄
       → Insert Ob u d₄ dˣ
       → A)
    → Tower (x ∷ y ∷ z ∷ w ∷ u ∷ []) ds dˣ
    → A
  tower₅ f (a ◂ (b ◂ (c ◂ (d ◂ (e ◂ ⟨⟩))))) = f a b c d e

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

  insert-view-apart-swapʳ
    : ∀ {w u Δ Δ′ Δ″ C₀}
    → (c : Insert Ob w Δ′ Δ)
    → (ins : Insert Ob u Δ″ Δ)
    → (c₀ : Insert Ob w C₀ Δ″)
    → (ins₀ : Insert Ob u C₀ Δ′)
    → insert-view c ins ≡ apart c₀ ins₀
    → insert-swap ins c₀ ≡ exchange Δ′ c ins₀
  insert-view-apart-swapʳ c ins c₀ ins₀ ev =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ insert-swap (Exchange.outer e) (Exchange.inner e) ]· exchange _ ins c₀
    ≈·⁻¹⟨ insert-view-apart-swap c ins c₀ ins₀ ev ⟩
      insert-swap
        (Exchange.outer (insert-swap c ins₀))
        (Exchange.inner (insert-swap c ins₀))
    ≈⟨ insert-swap-invol c ins₀ ⟩
      exchange _ c ins₀
    ∎

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

  -- Threading a WIRE through whatever a removal left: a new source at `q` and
  -- a new sink at `s`, joined to each other. `removal-recap` is the same for a
  -- cut applied at the head; this is `match-insert`'s counterpart one level up,
  -- and it is what says where a removal lands when the matching it was read off
  -- had a wire threaded through it somewhere else.
  --
  -- Both branches reindex, and they reindex on OPPOSITE sides: a strand that
  -- ran through has its sink pushed past the new one, while a capped pair sits
  -- entirely on the source side and has its partner pushed past `q` instead.
  removal-thread
    : ∀ {v x Γ Γˣ Θ Θˣ}
    → Insert Ob x Γ Γˣ
    → Insert Ob x Θ Θˣ
    → Removal v Γ Θ
    → Removal v Γˣ Θˣ
  removal-thread q s (through spot body) =
    through
      (Exchange.outer (insert-swap s spot))
      (match-insert q (Exchange.inner (insert-swap s spot)) body)
  removal-thread q s (capped ins body) =
    capped
      (Exchange.outer (insert-swap q ins))
      (match-insert (Exchange.inner (insert-swap q ins)) s body)

  -- and `removal-tail` IS that operation with the new source at the front,
  -- proved rather than observed: the reindexing on either branch computes away
  -- once the position is `head`, so the block lifting is the special case and
  -- not a second device to keep in step with this one.
  removal-thread-head
    : ∀ {v x Γ Θ Θˣ}
    → (s : Insert Ob x Θ Θˣ)
    → (r : Removal v Γ Θ)
    → removal-thread head s r ≡ removal-tail s r
  removal-thread-head s (through spot body) = refl
  removal-thread-head s (capped ins body) = refl

  removal-recut
    : ∀ {v x y Γ Γ˘ Γˣ Θ}
    → Insert Ob x Γ˘ Γˣ
    → Insert Ob y Γ Γ˘
    → Removal v Γ Θ
    → Removal v Γˣ Θ
  removal-recut i k (through spot body) = through spot (match-cap i k body)
  removal-recut i k (capped ins body) =
    capped
      (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k ins))))
      (match-cap
        (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k ins))))
        (Exchange.inner (insert-swap k ins))
        body)

  -- the apart verdict as an exchange, read from the other side

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

  -- Re-applying a cut at ANY pair of ports, rather than at the head. This is
  -- to `unhit-cap` what `match-cap` is to the `cap` constructor, and its body
  -- is `match-cap-insert`'s right-hand side: the traced-back source is pushed
  -- past both new ports, and the cut is re-applied at the positions that
  -- reindexing leaves.
  unhit-recap
    : ∀ {x y z Γ Γ˘ Γˣ Δ}
    → Insert Ob x Γ˘ Γˣ
    → Insert Ob z Γ Γ˘
    → Unhit y Γ Δ
    → Unhit y Γˣ Δ
  unhit-recap i k (unhit p body) =
    unhit
      (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k p))))
      (match-cap
        (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
        (Exchange.inner (insert-swap k p))
        body)

  -- And threading a WIRE through what an inverse lookup left, which is to
  -- `unhit-recap` what `removal-thread` is to `removal-recap`. `unhit-tail` is
  -- this operation at the head — `unhit-thread head j` and `unhit-tail j` are
  -- the same function — so the block operation is the special case rather than
  -- a separate device.
  unhit-thread
    : ∀ {v x Γ Γˣ Δ Δˣ}
    → Insert Ob x Γ Γˣ
    → Insert Ob x Δ Δˣ
    → Unhit v Γ Δ
    → Unhit v Γˣ Δˣ
  unhit-thread i j (unhit p t) =
    unhit
      (Exchange.outer (insert-swap i p))
      (match-insert (Exchange.inner (insert-swap i p)) j t)

  -- and that identification, proved rather than observed
  unhit-thread-head
    : ∀ {v x Γ Δ Δˣ}
    → (j : Insert Ob x Δ Δˣ)
    → (u : Unhit v Γ Δ)
    → unhit-thread head j u ≡ unhit-tail j u
  unhit-thread-head j (unhit p t) = refl

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

  -- ── THE TWO THREADINGS COMMUTE ─────────────────────────────────────────────
  --
  -- Threading a cut and threading a wire commute, up to reindexing by nested
  -- `insert-swap`s. This is the bottom of the whole development: both
  -- commutation laws for `match-comp` reduce, through the views above, to this
  -- one statement, and it mentions neither composition nor a lookup — it is a
  -- fact about the two CONSTRUCTIONS alone, with no hypothesis.
  --
  -- WHERE ITS ARITY COMES FROM, which is what bounds the coherence it needs.
  -- `match-cap` threads two positions and `match-insert` threads one, and the
  -- matching underneath contributes the head they meet. So the cut-meets-cut
  -- clause has four nested insertions in play and spends `tower-coh⁴` — exactly
  -- as `cap-swap`, a cut against an underlying cap, spends the braid relation
  -- itself. Since the presentation above is complete, the arity now bounds the
  -- LENGTH of the word a clause spends and no longer the cost of reaching it:
  -- two cuts commuting would be five layers and would be one more word.
  --
  -- The clauses divide the way the arity predicts. Where either of the cut's
  -- two ports is at the front the reindexing is undone by the INVOLUTION
  -- alone, and those two clauses are one backward congruence each. Where the
  -- wire's position is at the front nothing has moved. Where the matching
  -- underneath threads a wire the statement is its own induction. Only where
  -- the matching underneath CAPS are four positions in play at once.
  match-cap-insert
    : ∀ {x y z Γ Γ′ Γ˘ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (k : Insert Ob y Γ Γ˘)
    → (p : Insert Ob z Γ′ Γ)
    → (j : Insert Ob z Δ Δˣ)
    → (t : Match Ob Γ′ Δ)
    → match-cap i k (match-insert p j t)
      ≡ match-insert
          (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k p))))
          j
          (match-cap
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
            (Exchange.inner (insert-swap k p))
            t)
  match-cap-insert head k p j t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ cap (Exchange.outer e) (match-insert (Exchange.inner e) j t) ]·
        exchange _ k p
    ≈·⁻¹⟨ insert-swap-invol k p ⟩
      match-insert
        (Exchange.outer (insert-swap head (Exchange.outer (insert-swap k p))))
        j
        (match-cap
          (Exchange.inner (insert-swap head (Exchange.outer (insert-swap k p))))
          (Exchange.inner (insert-swap k p))
          t)
    ∎
  match-cap-insert (tail i) head p j t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ cap (Exchange.outer e) (match-insert (Exchange.inner e) j t) ]·
        exchange _ i p
    ≈·⁻¹⟨ insert-swap-invol i p ⟩
      match-insert
        (Exchange.outer (insert-swap (tail i) (Exchange.outer (insert-swap head p))))
        j
        (match-cap
          (Exchange.inner (insert-swap (tail i) (Exchange.outer (insert-swap head p))))
          (Exchange.inner (insert-swap head p))
          t)
    ∎
  match-cap-insert (tail i) (tail k) head j t = refl
  match-cap-insert (tail i) (tail k) (tail p) j (c ∷ t) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ Exchange.outer (insert-swap j c) ∷ m ]·
        match-cap i k (match-insert p (Exchange.inner (insert-swap j c)) t)
    ≈·⟨ match-cap-insert i k p (Exchange.inner (insert-swap j c)) t ⟩
      match-insert
        (Exchange.outer
          (insert-swap (tail i) (Exchange.outer (insert-swap (tail k) (tail p)))))
        j
        (match-cap
          (Exchange.inner
            (insert-swap (tail i) (Exchange.outer (insert-swap (tail k) (tail p)))))
          (Exchange.inner (insert-swap (tail k) (tail p)))
          (c ∷ t))
    ∎
  match-cap-insert (tail i) (tail k) (tail p) j (cap c t) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ cap
              (Exchange.outer
                (insert-swap i
                  (Exchange.outer
                    (insert-swap k (Exchange.outer (insert-swap p c))))))
              m ]·
        match-cap
          (Exchange.inner
            (insert-swap i
              (Exchange.outer (insert-swap k (Exchange.outer (insert-swap p c))))))
          (Exchange.inner (insert-swap k (Exchange.outer (insert-swap p c))))
          (match-insert (Exchange.inner (insert-swap p c)) j t)
    ≈·⟨ match-cap-insert
          (Exchange.inner
            (insert-swap i
              (Exchange.outer (insert-swap k (Exchange.outer (insert-swap p c))))))
          (Exchange.inner (insert-swap k (Exchange.outer (insert-swap p c))))
          (Exchange.inner (insert-swap p c))
          j
          t ⟩
      [ T ↦ tower₄
              (λ a b c′ d → cap d (match-insert c′ j (match-cap b a t)))
              T ]·
        tower-swap₁
          (tower-swap (tower-swap₂ (tower-swap₁ (tower-swap (c ◂ p ◂ k ◂ i ◂ ⟨⟩)))))
    ≈·⟨ tower-coh⁴ (c ◂ p ◂ k ◂ i ◂ ⟨⟩) ⟩
      match-insert
        (Exchange.outer
          (insert-swap (tail i) (Exchange.outer (insert-swap (tail k) (tail p)))))
        j
        (match-cap
          (Exchange.inner
            (insert-swap (tail i) (Exchange.outer (insert-swap (tail k) (tail p)))))
          (Exchange.inner (insert-swap (tail k) (tail p)))
          (cap c t))
    ∎

  -- TWO THREADED WIRES COMMUTE, which is the same table's other entry and the
  -- cheapest of the three: no cut is in play, so the arity is a wire plus a
  -- wire plus the head they meet — three — and the coherence is the braid,
  -- spent once on each side of the matching. The two ends reindex
  -- independently, `insert-swap j c` on the sources and `insert-swap s σ` on
  -- the sinks, which is why one statement covers both.
  match-insert-insert
    : ∀ {x w Δ Δˣ Δ′ rest rest₂ Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (c : Insert Ob w Δ′ Δ)
    → (s : Insert Ob x rest Θ)
    → (σ : Insert Ob w rest₂ rest)
    → (β : Match Ob Δ′ rest₂)
    → match-insert j s (match-insert c σ β)
      ≡ match-insert
          (Exchange.outer (insert-swap j c))
          (Exchange.outer (insert-swap s σ))
          (match-insert
            (Exchange.inner (insert-swap j c))
            (Exchange.inner (insert-swap s σ))
            β)
  match-insert-insert head c s σ β =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ Exchange.outer e ∷ match-insert c (Exchange.inner e) β ]· exchange _ s σ
    ≈·⁻¹⟨ insert-swap-invol s σ ⟩
      match-insert
        (Exchange.outer (insert-swap head c))
        (Exchange.outer (insert-swap s σ))
        (match-insert
          (Exchange.inner (insert-swap head c))
          (Exchange.inner (insert-swap s σ))
          β)
    ∎
  match-insert-insert (tail j) head s σ β = refl
  match-insert-insert (tail j) (tail c) s σ (f ∷ β) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ Exchange.outer (insert-swap s (Exchange.outer (insert-swap σ f))) ∷ m ]·
        match-insert
          j
          (Exchange.inner (insert-swap s (Exchange.outer (insert-swap σ f))))
          (match-insert c (Exchange.inner (insert-swap σ f)) β)
    ≈·⟨ match-insert-insert
          j
          c
          (Exchange.inner (insert-swap s (Exchange.outer (insert-swap σ f))))
          (Exchange.inner (insert-swap σ f))
          β ⟩
      [ T ↦ tower₃
              (λ a b c′ →
                c′
                  ∷ match-insert
                      (Exchange.outer (insert-swap j c))
                      b
                      (match-insert (Exchange.inner (insert-swap j c)) a β))
              T ]·
        tower-swap (tower-swap₁ (tower-swap (f ◂ σ ◂ s ◂ ⟨⟩)))
    ≈·⟨ tower-swap-braid (f ◂ σ ◂ s ◂ ⟨⟩) ⟩
      match-insert
        (Exchange.outer (insert-swap (tail j) (tail c)))
        (Exchange.outer (insert-swap s σ))
        (match-insert
          (Exchange.inner (insert-swap (tail j) (tail c)))
          (Exchange.inner (insert-swap s σ))
          (f ∷ β))
    ∎
  match-insert-insert (tail j) (tail c) s σ (cap f β) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ cap
              (Exchange.outer (insert-swap j (Exchange.outer (insert-swap c f))))
              m ]·
        match-insert
          (Exchange.inner (insert-swap j (Exchange.outer (insert-swap c f))))
          s
          (match-insert (Exchange.inner (insert-swap c f)) σ β)
    ≈·⟨ match-insert-insert
          (Exchange.inner (insert-swap j (Exchange.outer (insert-swap c f))))
          (Exchange.inner (insert-swap c f))
          s
          σ
          β ⟩
      [ T ↦ tower₃
              (λ a b c′ →
                cap
                  c′
                  (match-insert
                    b
                    (Exchange.outer (insert-swap s σ))
                    (match-insert a (Exchange.inner (insert-swap s σ)) β)))
              T ]·
        tower-swap (tower-swap₁ (tower-swap (f ◂ c ◂ j ◂ ⟨⟩)))
    ≈·⟨ tower-swap-braid (f ◂ c ◂ j ◂ ⟨⟩) ⟩
      match-insert
        (Exchange.outer (insert-swap (tail j) (tail c)))
        (Exchange.outer (insert-swap s σ))
        (match-insert
          (Exchange.inner (insert-swap (tail j) (tail c)))
          (Exchange.inner (insert-swap s σ))
          (cap f β))
    ∎

  -- AND THE SAME COMMUTATION READ FROM THE WIRE'S SIDE, which costs NOTHING.
  -- `match-cap-insert` states it with the cut applied last; this states it with
  -- the wire applied last, and the two are the same equation at reindexed
  -- positions: instantiate `match-cap-insert` where this one's right-hand side
  -- puts its arguments, and the exchange's INVOLUTION undoes the reindexing,
  -- once for the cut's pair and once for the wire against the cut's outer port.
  --
  -- That is worth naming because the arity rule does NOT predict it. Read as a
  -- fresh statement this is a cut plus a wire plus a head — four — and it would
  -- owe the four-layer coherence a second time. It owes nothing, because the
  -- reordering it asks for is the one `match-cap-insert` already performed,
  -- taken backwards. A member of this family is free whenever its permutation
  -- is the inverse of one already spent.
  match-insert-cap
    : ∀ {x w u Δ Δˣ Δ′ Δ″ rest Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (c : Insert Ob w Δ′ Δ)
    → (ι : Insert Ob u Δ″ Δ′)
    → (s : Insert Ob x rest Θ)
    → (β : Match Ob Δ″ rest)
    → match-insert j s (match-cap c ι β)
      ≡ match-cap
          (Exchange.outer (insert-swap j c))
          (Exchange.outer
            (insert-swap (Exchange.inner (insert-swap j c)) ι))
          (match-insert
            (Exchange.inner
              (insert-swap (Exchange.inner (insert-swap j c)) ι))
            s
            β)
  match-insert-cap j c ι s β =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ match-insert (Exchange.outer e) s (match-cap (Exchange.inner e) ι β) ]·
        exchange _ j c
    ≈·⁻¹⟨ insert-swap-invol j c ⟩
      [ e ↦ match-insert
              (Exchange.outer
                (insert-swap (Exchange.outer (insert-swap j c)) (Exchange.outer e)))
              s
              (match-cap
                (Exchange.inner
                  (insert-swap (Exchange.outer (insert-swap j c)) (Exchange.outer e)))
                (Exchange.inner e)
                β) ]·
        exchange _ (Exchange.inner (insert-swap j c)) ι
    ≈·⁻¹⟨ insert-swap-invol (Exchange.inner (insert-swap j c)) ι ⟩
      match-insert
        (Exchange.outer
          (insert-swap
            (Exchange.outer (insert-swap j c))
            (Exchange.outer
              (insert-swap
                (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
                (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))))))
        s
        (match-cap
          (Exchange.inner
            (insert-swap
              (Exchange.outer (insert-swap j c))
              (Exchange.outer
                (insert-swap
                  (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
                  (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))))))
          (Exchange.inner
            (insert-swap
              (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
              (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))))
          β)
    ≈⁻¹⟨ match-cap-insert
           (Exchange.outer (insert-swap j c))
           (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
           (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))
           s
           β ⟩
      match-cap
        (Exchange.outer (insert-swap j c))
        (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
        (match-insert
          (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))
          s
          β)
    ∎

  -- AND TWO THREADED CUTS COMMUTE, which closes the table. Its coherence is
  -- five layers — a cut, a cut, and the head they meet — and under the
  -- presentation that is one more WORD rather than one more rung:
  -- `tower-cycle-pair` is the whole of what it asks of the positions.
  match-cap-cap
    : ∀ {x₁ y₁ x₂ y₂ Γ Γ₁ Γ₂ Γ₃ Γ₄ Δ}
    → (i₁ : Insert Ob x₁ Γ₃ Γ₄)
    → (k₁ : Insert Ob y₁ Γ₂ Γ₃)
    → (i₂ : Insert Ob x₂ Γ₁ Γ₂)
    → (k₂ : Insert Ob y₂ Γ Γ₁)
    → (β : Match Ob Γ Δ)
    → match-cap i₁ k₁ (match-cap i₂ k₂ β)
      ≡ tower₄
          (λ a b c d → match-cap d c (match-cap b a β))
          (tower-pair (k₂ ◂ i₂ ◂ k₁ ◂ i₁ ◂ ⟨⟩))
  match-cap-cap head k₁ i₂ k₂ β =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ cap (Exchange.outer e) (match-cap (Exchange.inner e) k₂ β) ]·
        exchange _ k₁ i₂
    ≈·⁻¹⟨ insert-swap-invol k₁ i₂ ⟩
      [ e ↦ cap
              (Exchange.outer
                (insert-swap (Exchange.outer (insert-swap k₁ i₂)) (Exchange.outer e)))
              (match-cap
                (Exchange.inner
                  (insert-swap (Exchange.outer (insert-swap k₁ i₂)) (Exchange.outer e)))
                (Exchange.inner e)
                β) ]·
        exchange _ (Exchange.inner (insert-swap k₁ i₂)) k₂
    ≈·⁻¹⟨ insert-swap-invol (Exchange.inner (insert-swap k₁ i₂)) k₂ ⟩
      tower₄
        (λ a b c d → match-cap d c (match-cap b a β))
        (tower-pair (k₂ ◂ i₂ ◂ k₁ ◂ head ◂ ⟨⟩))
    ∎
  match-cap-cap (tail i) head i₂ k₂ β =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ cap (Exchange.outer e) (match-cap (Exchange.inner e) k₂ β) ]·
        exchange _ i i₂
    ≈·⁻¹⟨ insert-swap-invol i i₂ ⟩
      [ e ↦ cap
              (Exchange.outer
                (insert-swap (Exchange.outer (insert-swap i i₂)) (Exchange.outer e)))
              (match-cap
                (Exchange.inner
                  (insert-swap (Exchange.outer (insert-swap i i₂)) (Exchange.outer e)))
                (Exchange.inner e)
                β) ]·
        exchange _ (Exchange.inner (insert-swap i i₂)) k₂
    ≈·⁻¹⟨ insert-swap-invol (Exchange.inner (insert-swap i i₂)) k₂ ⟩
      tower₄
        (λ a b c d → match-cap d c (match-cap b a β))
        (tower-pair (k₂ ◂ i₂ ◂ head ◂ tail i ◂ ⟨⟩))
    ∎
  match-cap-cap (tail i) (tail k) head k₂ β = refl
  match-cap-cap (tail i) (tail k) (tail i′) head β = refl
  match-cap-cap (tail i) (tail k) (tail i′) (tail k′) (c ∷ t) =
    cong (c ∷_) (match-cap-cap i k i′ k′ t)
  match-cap-cap (tail i) (tail k) (tail i′) (tail k′) (cap c t) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ cap
              (Exchange.outer
                (insert-swap i
                  (Exchange.outer
                    (insert-swap k
                      (Exchange.outer
                        (insert-swap i′ (Exchange.outer (insert-swap k′ c))))))))
              m ]·
        match-cap
          (Exchange.inner
            (insert-swap i
              (Exchange.outer
                (insert-swap k
                  (Exchange.outer
                    (insert-swap i′ (Exchange.outer (insert-swap k′ c))))))))
          (Exchange.inner
            (insert-swap k
              (Exchange.outer (insert-swap i′ (Exchange.outer (insert-swap k′ c))))))
          (match-cap
            (Exchange.inner (insert-swap i′ (Exchange.outer (insert-swap k′ c))))
            (Exchange.inner (insert-swap k′ c))
            t)
    ≈·⟨ match-cap-cap
          (Exchange.inner
            (insert-swap i
              (Exchange.outer
                (insert-swap k
                  (Exchange.outer
                    (insert-swap i′ (Exchange.outer (insert-swap k′ c))))))))
          (Exchange.inner
            (insert-swap k
              (Exchange.outer (insert-swap i′ (Exchange.outer (insert-swap k′ c))))))
          (Exchange.inner (insert-swap i′ (Exchange.outer (insert-swap k′ c))))
          (Exchange.inner (insert-swap k′ c))
          t ⟩
      [ T ↦ tower₅ (λ a b c′ d e → cap e (match-cap d c′ (match-cap b a t))) T ]·
        tower-pair (tower-cycle (c ◂ k′ ◂ i′ ◂ k ◂ i ◂ ⟨⟩))
    ≈·⟨ tower-cycle-pair (c ◂ k′ ◂ i′ ◂ k ◂ i ◂ ⟨⟩) ⟩
      tower₄
        (λ a b c′ d → match-cap d c′ (match-cap b a (cap c t)))
        (tower-pair (tail k′ ◂ tail i′ ◂ tail k ◂ tail i ◂ ⟨⟩))
    ∎

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
  -- lists differ. That is `tower-swap-braid`, and with the symmetry stated as
  -- a presentation the case is no longer a chase: it is one step over the braid
  -- relation, under one `cong` over the recursion. The hypothesis is gone, and
  -- what removed it was naming the structure rather than finding a trick.
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
      [ c ↦ cap
              (Exchange.outer (insert-swap i (Exchange.outer (insert-swap j k))))
              c ]·
        match-cap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m
    ≈·⟨ cap-swap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap j k))))
          (Exchange.inner (insert-swap j k))
          m ⟩
      [ t ↦ tower₃ (λ a b c′ → cap c′ (match-cap b a m)) t ]·
        tower-swap (tower-swap₁ (tower-swap (k ◂ j ◂ i ◂ ⟨⟩)))
    ≈·⟨ tower-swap-braid (k ◂ j ◂ i ◂ ⟨⟩) ⟩
      tower₃
        (λ a b c′ → cap c′ (match-cap b a m))
        (tower-swap₁ (tower-swap (tower-swap₁ (k ◂ j ◂ i ◂ ⟨⟩))))
    ∎

  -- re-applying a cut around a rebuild, with no hypothesis: the two branches
  -- are the commutation table's cut-against-wire and cut-against-cut entries
  recut-recover⋆
    : ∀ {x u w Δ Δˣ Δ″ C₀ Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (ins : Insert Ob u Δ″ Δ)
    → (c₀ : Insert Ob w C₀ Δ″)
    → (r : Removal w C₀ Θ)
    → removal→match
        (Exchange.outer (insert-swap j (Exchange.outer (insert-swap ins c₀))))
        (removal-recut
          (Exchange.inner (insert-swap j (Exchange.outer (insert-swap ins c₀))))
          (Exchange.inner (insert-swap ins c₀))
          r)
      ≡ match-cap j ins (removal→match c₀ r)
  recut-recover⋆ j ins c₀ (through σ β) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-insert
        (Exchange.outer (insert-swap j (Exchange.outer (insert-swap ins c₀))))
        σ
        (match-cap
          (Exchange.inner (insert-swap j (Exchange.outer (insert-swap ins c₀))))
          (Exchange.inner (insert-swap ins c₀))
          β)
    ≈⁻¹⟨ match-cap-insert j ins c₀ σ β ⟩
      match-cap j ins (match-insert c₀ σ β)
    ∎
  recut-recover⋆ j ins c₀ (capped ι β) =
    begin⟨ bundle (≡ˢ _) ⟩
      tower₄
        (λ a b c d → match-cap d c (match-cap b a β))
        (tower-pair (ι ◂ c₀ ◂ ins ◂ j ◂ ⟨⟩))
    ≈⁻¹⟨ match-cap-cap j ins c₀ ι β ⟩
      match-cap j ins (match-cap c₀ ι β)
    ∎

  recut-recover
    : ∀ {x u w Δ Δˣ Δ′ Δ″ C₀ Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (ins : Insert Ob u Δ″ Δ)
    → (c : Insert Ob w Δ′ Δ)
    → (c₀ : Insert Ob w C₀ Δ″)
    → (ins₀ : Insert Ob u C₀ Δ′)
    → insert-view c ins ≡ apart c₀ ins₀
    → (r : Removal w C₀ Θ)
    → removal→match
        (Exchange.outer (insert-swap j c))
        (removal-recut (Exchange.inner (insert-swap j c)) ins₀ r)
      ≡ match-cap j ins (removal→match c₀ r)
  recut-recover j ins c c₀ ins₀ ev r =
    begin⟨ bundle (≡ˢ _) ⟩
      [ e ↦ removal→match
              (Exchange.outer (insert-swap j (Exchange.outer e)))
              (removal-recut
                (Exchange.inner (insert-swap j (Exchange.outer e)))
                (Exchange.inner e)
                r) ]·
        exchange _ c ins₀
    ≈·⁻¹⟨ insert-view-apart-swapʳ c ins c₀ ins₀ ev ⟩
      removal→match
        (Exchange.outer (insert-swap j (Exchange.outer (insert-swap ins c₀))))
        (removal-recut
          (Exchange.inner (insert-swap j (Exchange.outer (insert-swap ins c₀))))
          (Exchange.inner (insert-swap ins c₀))
          r)
    ≈⟨ recut-recover⋆ j ins c₀ r ⟩
      match-cap j ins (removal→match c₀ r)
    ∎

  -- ── AND WHAT THE INVERSE LOOKUP SEES OF A CUT ──────────────────────────────
  --
  -- `match-unhit-cut` is `match-cap-insert`'s consumer and the reason it was
  -- proved: tracing a sink back through a wiring that has just been cut is
  -- tracing it back through the wiring underneath and re-applying the cut.
  -- Composition needs exactly this, because the fused case of `match-comp`
  -- traces a sink back through the FIRST wiring, and when that wiring is a cut
  -- over something else the trace has to be pushed inside it.
  --
  -- The four small facts below say what `unhit-recap` does at each shape the
  -- induction meets. Three of them hold by `refl` once the lookup's result is
  -- in constructor form; the fourth is the four-layer coherence, taken
  -- backwards, and it is where `match-cap-insert`'s own hard clause is spent a
  -- second time.

  -- the cut's outer port at the front, so the cut IS the head
  unhit-recap-outer
    : ∀ {x y z Γ Γ˘ Δ}
    → (k : Insert Ob z Γ Γ˘)
    → (u : Unhit y Γ Δ)
    → unhit-cap {w = x} k u ≡ unhit-recap {x = x} head k u
  unhit-recap-outer k (unhit p body) = refl

  -- and its inner port at the front, which is the same cut named the other way
  unhit-recap-inner
    : ∀ {x y z Γ Γˣ Δ}
    → (i : Insert Ob x Γ Γˣ)
    → (u : Unhit y Γ Δ)
    → unhit-cap {w = z} i u ≡ unhit-recap (tail {y = z} i) head u
  unhit-recap-inner i (unhit p body) = refl

  -- recapping past a matched pair the trace stepped over
  unhit-recap-tail
    : ∀ {v x y z Γ Γ˘ Γˣ Δ Δ˘}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (k : Insert Ob z Γ Γ˘)
    → (c : Insert Ob v Δ Δ˘)
    → (u : Unhit y Γ Δ)
    → unhit-tail c (unhit-recap i k u)
      ≡ unhit-recap (tail i) (tail k) (unhit-tail c u)
  unhit-recap-tail i k c (unhit p body) = refl

  -- and past a CAP it stepped over, which is where four positions meet: the
  -- cut's two ports, the cap's partner, and the traced-back source
  unhit-recap-cap
    : ∀ {v w x y z Γ Γ₀ Γ˘ Γˣ Δ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (k : Insert Ob z Γ₀ Γ˘)
    → (c : Insert Ob v Γ Γ₀)
    → (u : Unhit y Γ Δ)
    → unhit-cap {w = w}
        (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
        (unhit-recap
          (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
          (Exchange.inner (insert-swap k c))
          u)
      ≡ unhit-recap (tail {y = w} i) (tail {y = w} k) (unhit-cap {w = w} c u)
  unhit-recap-cap i k c (unhit p body) =
    begin⟨ bundle (≡ˢ _) ⟩
      [ T ↦ tower₄
              (λ a b c′ d →
                unhit (tail d) (cap c′ (match-cap b a body)))
              T ]·
        tower-swap₂
          (tower-swap₁
            (tower-swap (tower-swap₂ (tower-swap₁ (p ◂ c ◂ k ◂ i ◂ ⟨⟩)))))
    ≈·⁻¹⟨ tower-coh⁴ (p ◂ c ◂ k ◂ i ◂ ⟨⟩) ⟩
      unhit-recap (tail i) (tail k) (unhit-cap c (unhit p body))
    ∎

  -- Tracing a sink back through a cut wiring. The `∷` clause splits on the
  -- position comparison `match-unhit` itself splits on, so it is met from the
  -- consumer's side — the verdict passed as an argument with its defining
  -- equation, never as a `with`.
  mutual

    match-unhit-cut
      : ∀ {x y z Γ Γ˘ Γˣ Δ Δˣ}
      → (i : Insert Ob x Γ˘ Γˣ)
      → (k : Insert Ob z Γ Γ˘)
      → (j : Insert Ob y Δ Δˣ)
      → (m : Match Ob Γ Δˣ)
      → match-unhit j (match-cap i k m) ≡ unhit-recap i k (match-unhit j m)
    match-unhit-cut head k j m = unhit-recap-outer k (match-unhit j m)
    match-unhit-cut (tail i) head j m = unhit-recap-inner i (match-unhit j m)
    match-unhit-cut (tail i) (tail k) j (c ∷ m) =
      unhit-cut-∷ i k j c m (insert-view j c) refl
    match-unhit-cut (tail i) (tail k) j (cap c m) =
      begin⟨ bundle (≡ˢ _) ⟩
        [ u ↦ unhit-cap
                (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
                u ]·
          match-unhit
            j
            (match-cap
              (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
              (Exchange.inner (insert-swap k c))
              m)
      ≈·⟨ match-unhit-cut
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
            (Exchange.inner (insert-swap k c))
            j
            m ⟩
        unhit-cap
          (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
          (unhit-recap
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
            (Exchange.inner (insert-swap k c))
            (match-unhit j m))
      ≈⟨ unhit-recap-cap i k c (match-unhit j m) ⟩
        unhit-recap (tail i) (tail k) (unhit-cap c (match-unhit j m))
      ∎

    -- the leading pair either IS the sink being traced, or is stepped over
    unhit-cut-∷
      : ∀ {v x y z Γ Γ˘ Γˣ ys Δ Δˣ}
      → (i : Insert Ob x Γ˘ Γˣ)
      → (k : Insert Ob z Γ Γ˘)
      → (j : Insert Ob y Δ Δˣ)
      → (c : Insert Ob v ys Δˣ)
      → (m : Match Ob Γ ys)
      → (w : InsertView j c)
      → insert-view j c ≡ w
      → match-unhit j (c ∷ match-cap i k m)
        ≡ unhit-recap (tail i) (tail k) (match-unhit j (c ∷ m))
    unhit-cut-∷ i k j .j m same _ =
      begin⟨ bundle (≡ˢ _) ⟩
        match-unhit j (j ∷ match-cap i k m)
      ≈⟨ match-unhit-∷-same j (match-cap i k m) ⟩
        [ u ↦ unhit-recap (tail i) (tail k) u ]· unhit head m
      ≈·⁻¹⟨ match-unhit-∷-same j m ⟩
        unhit-recap (tail i) (tail k) (match-unhit j (j ∷ m))
      ∎
    unhit-cut-∷ i k j c m (apart j′ c′) ev =
      begin⟨ bundle (≡ˢ _) ⟩
        match-unhit j (c ∷ match-cap i k m)
      ≈⟨ match-unhit-∷-apart j c (match-cap i k m) j′ c′ ev ⟩
        [ u ↦ unhit-tail c′ u ]· match-unhit j′ (match-cap i k m)
      ≈·⟨ match-unhit-cut i k j′ m ⟩
        unhit-tail c′ (unhit-recap i k (match-unhit j′ m))
      ≈⟨ unhit-recap-tail i k c′ (match-unhit j′ m) ⟩
        [ u ↦ unhit-recap (tail i) (tail k) u ]· unhit-tail c′ (match-unhit j′ m)
      ≈·⁻¹⟨ match-unhit-∷-apart j c m j′ c′ ev ⟩
        unhit-recap (tail i) (tail k) (match-unhit j (c ∷ m))
      ∎

  -- ── CONSTRUCTION AFTER ANALYSIS ────────────────────────────────────────────
  --
  -- The other round trip, and the one that does the work below: a lookup loses
  -- nothing, so the matching can be REBUILT from it. Every fact of the form
  -- "what does an operation do to `o`, given what a lookup on `o` returned" is
  -- reached by rebuilding `o` and computing, because the rebuild is a
  -- construction and the lookup is a recursion.
  --
  -- The hypothesis reads `match-remove i o ≡ r`, the same direction the
  -- unfolding lemmas use, so there is one orientation in the file and not two.
  -- A rebuild consumes the lookup's value on the LEFT — `r` is refined by
  -- matching and the chain reads from the rebuilt term towards `o` — so the
  -- steps that carry the hypothesis take it backwards, which is what the two
  -- inverse markers are for.
  mutual

    match-remove-recover
      : ∀ {x Γ Γˣ Θ}
      → (i : Insert Ob x Γ Γˣ)
      → (o : Match Ob Γˣ Θ)
      → (r : Removal x Γ Θ)
      → match-remove i o ≡ r
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
      → match-remove i o ≡ r
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
      → match-remove (tail {y = w} i) (cap c o) ≡ r
      → removal→match (tail i) r ≡ cap c o
    recover-cap i .i o same _ r eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ r₀ ↦ removal→match (tail i) r₀ ]· r
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             r
           ≈⁻¹⟨ eq ⟩
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
           ≈⁻¹⟨ eq ⟩
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
      → match-remove i′ o ≡ r
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
      → match-unhit j m ≡ u
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
      → match-unhit j (i ∷ m) ≡ u
      → unhit→match j u ≡ i ∷ m
    recover-hit j .j m same _ u eq =
      begin⟨ bundle (≡ˢ _) ⟩
        [ u₀ ↦ unhit→match j u₀ ]· u
      ≈·⟨ (begin⟨ bundle (≡ˢ _) ⟩
             u
           ≈⁻¹⟨ eq ⟩
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
           ≈⁻¹⟨ eq ⟩
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
      → match-unhit j′ m ≡ u
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
      → match-unhit j m ≡ u
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

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT A LOOKUP SEES OF A THREADED WIRE, AT A POSITION APART FROM THE ONE
  -- THREADED. This is the instrument the composition laws' wire half runs on,
  -- and it is where the views above are spent: the lookup is a recursion and
  -- no lemma reaches past it, so the matching is REBUILT from the lookup's own
  -- value, the threading is commuted past the rebuild — which is a fact about
  -- constructions and computes — and the round trip reads the answer back.
  --
  -- The two rebuild lemmas below are that middle step, and each is one of the
  -- commutation table's entries applied backwards. Nothing here is new work:
  -- `match-insert-insert` carries the through branch and `match-insert-cap` the
  -- capped one, and the second of those was free.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Threading a wire and rebuilding commute, on both of a removal's branches.
  thread-recover
    : ∀ {x w Δ Δˣ Δ′ rest Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (c : Insert Ob w Δ′ Δ)
    → (s : Insert Ob x rest Θ)
    → (r : Removal w Δ′ rest)
    → removal→match
        (Exchange.outer (insert-swap j c))
        (removal-thread (Exchange.inner (insert-swap j c)) s r)
      ≡ match-insert j s (removal→match c r)
  thread-recover j c s (through σ β) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-insert
        (Exchange.outer (insert-swap j c))
        (Exchange.outer (insert-swap s σ))
        (match-insert
          (Exchange.inner (insert-swap j c))
          (Exchange.inner (insert-swap s σ))
          β)
    ≈⁻¹⟨ match-insert-insert j c s σ β ⟩
      match-insert j s (match-insert c σ β)
    ∎
  thread-recover j c s (capped ι β) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-cap
        (Exchange.outer (insert-swap j c))
        (Exchange.outer (insert-swap (Exchange.inner (insert-swap j c)) ι))
        (match-insert
          (Exchange.inner (insert-swap (Exchange.inner (insert-swap j c)) ι))
          s
          β)
    ≈⁻¹⟨ match-insert-cap j c ι s β ⟩
      match-insert j s (match-cap c ι β)
    ∎

  -- Removing a source from a wiring that has a wire threaded through it
  -- ELSEWHERE is removing it from the wiring underneath and threading the wire
  -- through the removal. This is the fact recorded as "removing two different
  -- sources in either order", at the shape composition actually asks for it.
  match-remove-insert-apart
    : ∀ {x w Δ Δˣ Δ′ rest Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (c : Insert Ob w Δ′ Δ)
    → (s : Insert Ob x rest Θ)
    → (t : Match Ob Δ rest)
    → match-remove (Exchange.outer (insert-swap j c)) (match-insert j s t)
      ≡ removal-thread (Exchange.inner (insert-swap j c)) s (match-remove c t)
  match-remove-insert-apart j c s t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ match-remove (Exchange.outer (insert-swap j c)) (match-insert j s m) ]· t
    ≈·⁻¹⟨ match-remove-recover c t (match-remove c t) refl ⟩
      [ m ↦ match-remove (Exchange.outer (insert-swap j c)) m ]·
        match-insert j s (removal→match c (match-remove c t))
    ≈·⁻¹⟨ thread-recover j c s (match-remove c t) ⟩
      match-remove
        (Exchange.outer (insert-swap j c))
        (removal→match
          (Exchange.outer (insert-swap j c))
          (removal-thread
            (Exchange.inner (insert-swap j c))
            s
            (match-remove c t)))
    ≈⟨ match-remove-roundtrip
         (Exchange.outer (insert-swap j c))
         (removal-thread (Exchange.inner (insert-swap j c)) s (match-remove c t)) ⟩
      removal-thread (Exchange.inner (insert-swap j c)) s (match-remove c t)
    ∎

  -- The same for the inverse lookup, and it is the SAME instrument: the
  -- `Unhit` rebuild is a single `match-insert`, so its commutation is
  -- `match-insert-insert` again rather than a second entry of the table. The
  -- mirror was not claimed symmetric with the removal's; on this step it is.
  thread-unhit-recover
    : ∀ {x u Γ Γˣ Δ′ Δ″ mid}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ′ mid)
    → (ι : Insert Ob u Δ″ Δ′)
    → (u₀ : Unhit u Γ Δ″)
    → unhit→match
        (Exchange.outer (insert-swap j ι))
        (unhit-thread i (Exchange.inner (insert-swap j ι)) u₀)
      ≡ match-insert i j (unhit→match ι u₀)
  thread-unhit-recover i j ι (unhit p b) =
    begin⟨ bundle (≡ˢ _) ⟩
      match-insert
        (Exchange.outer (insert-swap i p))
        (Exchange.outer (insert-swap j ι))
        (match-insert
          (Exchange.inner (insert-swap i p))
          (Exchange.inner (insert-swap j ι))
          b)
    ≈⁻¹⟨ match-insert-insert i p j ι b ⟩
      match-insert i j (match-insert p ι b)
    ∎

  -- Tracing a sink back through a wiring that has a wire threaded through it
  -- elsewhere is tracing it back through the wiring underneath and threading
  -- the wire through the result.
  match-unhit-insert-apart
    : ∀ {x u Γ Γˣ Δ′ Δ″ mid}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ′ mid)
    → (ι : Insert Ob u Δ″ Δ′)
    → (b : Match Ob Γ Δ′)
    → match-unhit (Exchange.outer (insert-swap j ι)) (match-insert i j b)
      ≡ unhit-thread i (Exchange.inner (insert-swap j ι)) (match-unhit ι b)
  match-unhit-insert-apart i j ι b =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ match-unhit (Exchange.outer (insert-swap j ι)) (match-insert i j m) ]· b
    ≈·⁻¹⟨ match-unhit-recover ι b (match-unhit ι b) refl ⟩
      [ m ↦ match-unhit (Exchange.outer (insert-swap j ι)) m ]·
        match-insert i j (unhit→match ι (match-unhit ι b))
    ≈·⁻¹⟨ thread-unhit-recover i j ι (match-unhit ι b) ⟩
      match-unhit
        (Exchange.outer (insert-swap j ι))
        (unhit→match
          (Exchange.outer (insert-swap j ι))
          (unhit-thread i (Exchange.inner (insert-swap j ι)) (match-unhit ι b)))
    ≈⟨ match-unhit-roundtrip
         (Exchange.outer (insert-swap j ι))
         (unhit-thread i (Exchange.inner (insert-swap j ι)) (match-unhit ι b)) ⟩
      unhit-thread i (Exchange.inner (insert-swap j ι)) (match-unhit ι b)
    ∎

  -- REMOVING A SOURCE FROM A WIRING THAT CAPS ELSEWHERE, when the source is not
  -- one of the cut's own two ports
  match-remove-cut-apart
    : ∀ {x u w Δ Δˣ Δ′ Δ″ C₀ Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (ins : Insert Ob u Δ″ Δ)
    → (c : Insert Ob w Δ′ Δ)
    → (c₀ : Insert Ob w C₀ Δ″)
    → (ins₀ : Insert Ob u C₀ Δ′)
    → insert-view c ins ≡ apart c₀ ins₀
    → (t : Match Ob Δ″ Θ)
    → match-remove (Exchange.outer (insert-swap j c)) (match-cap j ins t)
      ≡ removal-recut (Exchange.inner (insert-swap j c)) ins₀ (match-remove c₀ t)
  match-remove-cut-apart j ins c c₀ ins₀ ev t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ match-remove (Exchange.outer (insert-swap j c)) (match-cap j ins m) ]· t
    ≈·⁻¹⟨ match-remove-recover c₀ t (match-remove c₀ t) refl ⟩
      [ m ↦ match-remove (Exchange.outer (insert-swap j c)) m ]·
        match-cap j ins (removal→match c₀ (match-remove c₀ t))
    ≈·⁻¹⟨ recut-recover j ins c c₀ ins₀ ev (match-remove c₀ t) ⟩
      match-remove
        (Exchange.outer (insert-swap j c))
        (removal→match
          (Exchange.outer (insert-swap j c))
          (removal-recut
            (Exchange.inner (insert-swap j c))
            ins₀
            (match-remove c₀ t)))
    ≈⟨ match-remove-roundtrip
         (Exchange.outer (insert-swap j c))
         (removal-recut (Exchange.inner (insert-swap j c)) ins₀ (match-remove c₀ t)) ⟩
      removal-recut (Exchange.inner (insert-swap j c)) ins₀ (match-remove c₀ t)
    ∎

  -- and when it IS one of them: the cut leaves together with its partner, and
  -- the partner is the cut's other port read in what remains
  match-remove-cut-same
    : ∀ {x w Δ Δˣ Δ′ Θ}
    → (j : Insert Ob x Δ Δˣ)
    → (c : Insert Ob w Δ′ Δ)
    → (t : Match Ob Δ′ Θ)
    → match-remove (Exchange.outer (insert-swap j c)) (match-cap j c t)
      ≡ capped (Exchange.inner (insert-swap j c)) t
  match-remove-cut-same j c t =
    begin⟨ bundle (≡ˢ _) ⟩
      [ m ↦ match-remove (Exchange.outer (insert-swap j c)) m ]· match-cap j c t
    ≈·⟨ cap-swap j c t ⟩
      match-remove
        (Exchange.outer (insert-swap j c))
        (match-cap
          (Exchange.outer (insert-swap j c))
          (Exchange.inner (insert-swap j c))
          t)
    ≈⟨ match-remove-cut
         (Exchange.outer (insert-swap j c))
         (Exchange.inner (insert-swap j c))
         t ⟩
      capped (Exchange.inner (insert-swap j c)) t
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
  -- A CUT COMMUTES WITH COMPOSITION ON THE LEFT. Cutting two sources together
  -- and then composing is composing and then cutting: the cut consumes no
  -- sink, so the second wiring never sees it.
  --
  -- The proof is the first one to mirror `match-comp`'s OWN recursion, and it
  -- pays the same measure — the fused case recurses on a matching that
  -- `match-unhit` produced rather than on a subterm, so it descends on the
  -- length of the source list. The statement itself carries no accessibility
  -- witness, so no irrelevance lemma is needed above it: two witnesses prove
  -- the same equation.
  --
  -- What it costs beyond that is exactly `match-unhit-cut`, in the fused case:
  -- the trace back through the first wiring meets the cut, and pushing the
  -- trace inside the cut is what needed the four-layer coherence.
  -- ══════════════════════════════════════════════════════════════════════════

  mutual

    match-comp-cut-acc
      : ∀ {x y Γ Γ˘ Γˣ Ξ Θ}
      → Acc _<_ (length Γ)
      → (i : Insert Ob x Γ˘ Γˣ)
      → (k : Insert Ob y Γ Γ˘)
      → (m : Match Ob Γ Ξ)
      → (o : Match Ob Ξ Θ)
      → match-comp (match-cap i k m) o ≡ match-cap i k (match-comp m o)
    -- either port at the front makes the cut the composite's own head
    match-comp-cut-acc ac head k m o = match-comp-cap k m o
    match-comp-cut-acc ac (tail i) head m o = match-comp-cap i m o
    -- the leading source is matched, so the composite's head is whatever the
    -- second wiring does with it
    match-comp-cut-acc ac (tail i) (tail k) (c ∷ m) o =
      comp-cut-∷ ac i k c m o (match-remove c o) refl
    -- or the leading source is already capped, and that cap stands
    match-comp-cut-acc (acc rec) (tail i) (tail k) (cap c m) o =
      begin⟨ bundle (≡ˢ _) ⟩
        match-comp
          (cap
            (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
            (match-cap
              (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
              (Exchange.inner (insert-swap k c))
              m))
          o
      ≈⟨ match-comp-cap
           (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
           (match-cap
             (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
             (Exchange.inner (insert-swap k c))
             m)
           o ⟩
        [ M ↦ cap
                (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k c))))
                M ]·
          match-comp
            (match-cap
              (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
              (Exchange.inner (insert-swap k c))
              m)
            o
      ≈·⟨ match-comp-cut-acc
            (rec (insert-shrink c))
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k c))))
            (Exchange.inner (insert-swap k c))
            m
            o ⟩
        [ M ↦ match-cap (tail i) (tail k) M ]· cap c (match-comp m o)
      ≈·⁻¹⟨ match-comp-cap c m o ⟩
        match-cap (tail i) (tail k) (match-comp (cap c m) o)
      ∎

    -- what the second wiring does with the leading source, taken as an
    -- argument with its defining equation rather than met by a `with`
    comp-cut-∷
      : ∀ {v x y Γ Γ˘ Γˣ ys zs Θ}
      → Acc _<_ (suc (length Γ))
      → (i : Insert Ob x Γ˘ Γˣ)
      → (k : Insert Ob y Γ Γ˘)
      → (c : Insert Ob v ys zs)
      → (m : Match Ob Γ ys)
      → (o : Match Ob zs Θ)
      → (r : Removal v ys Θ)
      → match-remove c o ≡ r
      → match-comp (c ∷ match-cap i k m) o
        ≡ match-cap (tail i) (tail k) (match-comp (c ∷ m) o)
    comp-cut-∷ (acc rec) i k c m o (through spot body) eq =
      begin⟨ bundle (≡ˢ _) ⟩
        match-comp (c ∷ match-cap i k m) o
      ≈⟨ match-comp-∷-through c (match-cap i k m) o spot body eq ⟩
        [ M ↦ spot ∷ M ]· match-comp (match-cap i k m) body
      ≈·⟨ match-comp-cut-acc (rec (n<1+n _)) i k m body ⟩
        [ M ↦ match-cap (tail i) (tail k) M ]· (spot ∷ match-comp m body)
      ≈·⁻¹⟨ match-comp-∷-through c m o spot body eq ⟩
        match-cap (tail i) (tail k) (match-comp (c ∷ m) o)
      ∎
    comp-cut-∷ ac i k c m o (capped ins body) eq =
      comp-cut-fuse ac i k c m o ins body eq (match-unhit ins m) refl

    -- and the trace back through the cut wiring, likewise
    comp-cut-fuse
      : ∀ {u v x y Γ Γ˘ Γˣ ys zs xs′ Θ}
      → Acc _<_ (suc (length Γ))
      → (i : Insert Ob x Γ˘ Γˣ)
      → (k : Insert Ob y Γ Γ˘)
      → (c : Insert Ob v ys zs)
      → (m : Match Ob Γ ys)
      → (o : Match Ob zs Θ)
      → (ins : Insert Ob u xs′ ys)
      → (body : Match Ob xs′ Θ)
      → match-remove c o ≡ capped ins body
      → (u₀ : Unhit u Γ xs′)
      → match-unhit ins m ≡ u₀
      → match-comp (c ∷ match-cap i k m) o
        ≡ match-cap (tail i) (tail k) (match-comp (c ∷ m) o)
    comp-cut-fuse (acc rec) i k c m o ins body eq (unhit p m′) eq′ =
      begin⟨ bundle (≡ˢ _) ⟩
        match-comp (c ∷ match-cap i k m) o
      ≈⟨ match-comp-∷-capped c (match-cap i k m) o ins body
           (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k p))))
           (match-cap
             (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
             (Exchange.inner (insert-swap k p))
             m′)
           eq
           (begin⟨ bundle (≡ˢ _) ⟩
              match-unhit ins (match-cap i k m)
            ≈⟨ match-unhit-cut i k ins m ⟩
              [ u ↦ unhit-recap i k u ]· match-unhit ins m
            ≈·⟨ eq′ ⟩
              unhit
                (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k p))))
                (match-cap
                  (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
                  (Exchange.inner (insert-swap k p))
                  m′)
            ∎) ⟩
        [ M ↦ cap
                (Exchange.outer (insert-swap i (Exchange.outer (insert-swap k p))))
                M ]·
          match-comp
            (match-cap
              (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
              (Exchange.inner (insert-swap k p))
              m′)
            body
      ≈·⟨ match-comp-cut-acc
            (rec (insert-shrink p))
            (Exchange.inner (insert-swap i (Exchange.outer (insert-swap k p))))
            (Exchange.inner (insert-swap k p))
            m′
            body ⟩
        [ M ↦ match-cap (tail i) (tail k) M ]· cap p (match-comp m′ body)
      ≈·⁻¹⟨ match-comp-∷-capped c m o ins body p m′ eq eq′ ⟩
        match-cap (tail i) (tail k) (match-comp (c ∷ m) o)
      ∎

  -- and the theorem, at the witness the measure supplies
  match-comp-cut
    : ∀ {x y Γ Γ˘ Γˣ Ξ Θ}
    → (i : Insert Ob x Γ˘ Γˣ)
    → (k : Insert Ob y Γ Γ˘)
    → (m : Match Ob Γ Ξ)
    → (o : Match Ob Ξ Θ)
    → match-comp (match-cap i k m) o ≡ match-cap i k (match-comp m o)
  match-comp-cut {Γ} = match-comp-cut-acc (<-wellFounded (length Γ))

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
