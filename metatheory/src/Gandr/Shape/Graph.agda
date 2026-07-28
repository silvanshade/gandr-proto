{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Graph — the cell shape, as an inductive family indexed by its
-- interfaces: `Shape Γ Δ` over `List Ob`.
--
-- A shape is a finite directed graph whose dangling ends are its INTERFACE.
-- `Γ` lists the input legs in order and `Δ` the output legs, so many-in/
-- many-out is what the index says rather than something a `Maybe` records.
--
-- ── WHY THIS IS INDEXED AND THE PREVIOUS PRESENTATION WAS NOT ───────────────
-- An earlier revision carried cardinalities plus `src tgt : Fin ∣E∣ → Maybe
-- (Fin ∣V∣)` and `inp out : Fin ∣V∣ → List (Fin ∣E∣)` with six laws tying the
-- listings to the incidence. Its content was right and it was green; its
-- presentation was tabular, and three things followed.
--
--   * The interface was implicit in which edges happened to dangle, so there
--     was no index for an arity abstraction to quantify over. `Gandr.Arity`
--     could therefore not be extracted alongside `Gandr.Arity.Path`, whose
--     `Path a b` IS indexed by the interface it spans. This is the decisive
--     reason for the re-presentation, not the funext symptom below.
--   * Lemmas appeared whose only job was to refute configurations the encoding
--     permitted and the object does not have.
--   * Propositional equality of shapes was out of reach and the header
--     explained that as the SETOID-not-SET commitment. That explanation was a
--     rationalisation of a representation choice; see the located wall below
--     for what the obstruction actually is.
--
-- The rule now in force is `docs/workflow/agda.md` §*Representation: familial
-- first*, and its STOP clause. This module is written under it.
--
-- ── THE CARRIER, AND WHY IT HAS EXACTLY TWO CONSTRUCTORS ────────────────────
-- Read `Shape Γ Δ` as a wiring problem. Every edge has one SOURCE end and one
-- SINK end. The source ends are the input legs together with the out-ports of
-- the vertices; the sink ends are the in-ports of the vertices together with
-- the output legs. A graph is precisely a colour-preserving bijection between
-- those two collections — nothing more.
--
--   * `node A B` declares a vertex with in-profile `A` and out-profile `B`.
--     The REST of the graph sees `B` as extra sources and `A` as extra sinks.
--   * `wires m` closes the graph off with no further vertex, matching the
--     remaining sources onto the remaining sinks.
--
-- That the vertex publishes its ports to the rest — rather than consuming from
-- a pool already accumulated — is what keeps the carrier general. A pool
-- discipline would force a topological order and would make wheel-freeness
-- STRUCTURAL, which sounds like a gain and is a loss: the wheel would become
-- inexpressible, `WheelFree` would have no refuter, and Rmk 2.36's separation
-- would have nothing left to separate. The refuters are the point.
--
-- ── THE LISTINGS ARE PRIMARY AND THE INCIDENCE IS DERIVED ───────────────────
-- This inverts the previous presentation. `A` and `B` are the chosen linear
-- orders on the vertex's ports, carried as data in the constructor, and the
-- matching is the chosen wiring; `origin` and `dest` are then computed by
-- tracing an edge outward through the node chain, splitting at each profile.
--
-- Two consequences worth stating, because they are what the re-presentation
-- bought. The six listing laws are GONE — there is nothing to state, because a
-- listing that enumerates its fibre exactly once is what `Match` is. And
-- colour agreement is by construction: an ill-coloured wiring has no term.
--
-- The ordering remains representation content and is deliberately not
-- quotiented (C3): two shapes differing only in the order of their vertices,
-- or in a vertex's port order, are different objects here. That is the section
-- discipline, and reconciling it is `Gandr.Rigid`'s job, not this module's.
--
-- The derived operations are written with the sum ELIMINATOR rather than with
-- a `with` on their recursive call, and that is a working rule rather than a
-- taste: a `with` compiles to an auxiliary function the caller cannot name, so
-- `split (cons p) (there i)` becomes a term stuck on something no lemma can
-- rewrite, and every fact about it has to be re-proved by matching at each use
-- site. Written compositionally the recursive call is a visible subterm and
-- one `cong` reaches it. `split`, `origin` and `dest` are all in that form, and
-- `split-left`/`split-right`/`swap-follow` are what it buys.
--
-- ── EQUALITY: WHAT THE OBSTRUCTION IS, AND WHAT IT IS NOT ───────────────────
-- The funext obstruction the tabular presentation reported is genuinely gone:
-- there are no function-typed fields left. A DIFFERENT obstruction sits behind
-- it and is easy to mistake for the same one, so it is stated precisely.
--
-- `Insert x ys zs` names a position and records the element sitting at it, so
-- one index is FORCED by the others. Comparing two inhabitants at fixed
-- indices therefore has to eliminate a reflexive equation, which `--without-K`
-- refuses. This was checked in both presentations — remainder carried as an
-- index, and remainder derived from the position — and both fail, the second
-- more narrowly, on `x = x` alone. It is not an artifact of the choice made
-- here, and the same shape recurs at every constructor carrying an existential
-- implicit.
--
-- **It is a limit of pattern matching, not of the setting, and it is
-- discharged rather than located.** Sending a witness to a recursively
-- computed code built from `⊥`/`×`/`⊎`/`≡` compares its inhabitants without
-- matching any index, and the round trip is the identity.
--
-- The hypothesis that closes the residual reflexive equation is UIP on the
-- COLOURS ALONE — an h-level condition, not decidability. Decidable equality
-- is the constructive supplier of that set-ness through Hedberg, and is needed
-- only where a decision is actually computed; nothing about the shapes
-- themselves, and nothing about their vertices or edges, has to form a set for
-- the uniqueness lemmas to go through. `docs/workflow/agda.md` §*Decidable
-- equality is spiked first* records the technique, the factoring, and why the
-- spike must be run before the representation is built on rather than after.
--
-- One earlier claim is retracted here rather than quietly dropped: witness
-- uniqueness for `Append` — `(p q : Append xs ys zs) → p ≡ q` — was recorded
-- as needing K. It does not. The FACT holds; only the pattern match fails, and
-- the code route proves it under `--safe --without-K`. Reaching for
-- functionality rather than uniqueness remains the right default, but it is a
-- convenience, not a necessity.
--
-- ── WHY `Append` MAY REPEAT AN INDEX VARIABLE ───────────────────────────────
-- `Append`'s `nil : Append [] ys ys` copies `ys` across two result indices,
-- which reads like the shape this tree forbids. It is the same clause as
-- `Gandr.Arity.Path`'s `Cat.nil`, and it is endorsed for the same reason: it
-- is a graph-of-multiplication unit clause, and the lemmas taken over it are
-- TOTALITY and FUNCTIONALITY, neither of which eliminates a reflexive
-- equation. `append-fun` below is `cat-fun`'s analogue and is checked here.
--
-- ── THE TWO CONNECTIVITY PREDICATES, AND WHAT STANDS IN FOR THE BETTI NUMBERS
-- Betti numbers are not defined homologically here. That would need chain
-- complexes over the graph and would be a large development whose only use is
-- to name two combinatorial conditions. The conditions are named directly, and
-- this is the exact correspondence claimed:
--
--   * `β₀ = 1` is read as `Connected`: there is a vertex, and every vertex is
--     reachable from it by an UNDIRECTED walk. "There is a vertex" is the `≥ 1`
--     half and is not vacuous — the empty graph has `β₀ = 0` and is refuted
--     below. Reachability through a chosen root is the `≤ 1` half; that it does
--     not depend on the root is proved (`reach-any`), so the record's choice of
--     root is a witness, not a strengthening.
--
--   * `β₁ = 0` is read as `Acyclic`: there is no nontrivial REDUCED undirected
--     closed walk. This is the right reading because an undirected multigraph
--     contains a cycle exactly when it admits such a walk — a shortest
--     nontrivial reduced closed walk is a cycle, and conversely. That
--     equivalence is the JUSTIFICATION for the reading and is not itself
--     mechanized; what is mechanized is the predicate and its consequences.
--     Reducedness is what excludes the degenerate "walk out along an edge and
--     back along the same edge", which every graph admits and which is not a
--     cycle. Parallel edges and self-loops are cycles under this reading, as
--     they must be.
--
-- Reducedness needs edge IDENTITIES, so `Walk` is indexed by the edge it last
-- traversed and its extension constructor refuses to repeat that edge. The
-- index is `Maybe (Edg S)`, headed by a constructor, and the freshness side
-- condition is itself an inductive relation — a defined function never enters a
-- matchable index, in keeping with `Gandr.Arity.Path`'s witness discipline.
--
-- Legs never link: `Arc` demands that both ends of an edge be attached, so an
-- edge with a free end cannot sit on an undirected cycle. That is the correct
-- reading — the interface of a cell is not part of its topology.
--
-- ── WHAT IS PROVED, AND WHY THE DIAMOND IS THE LOAD-BEARING EXAMPLE ─────────
-- `SimplyConn` is STRICTLY stronger than `WheelFree` (HRY Rmk 2.36), and both
-- halves of that are exhibited rather than asserted:
--
--   * the implication `simply-conn⇒wheel-free` is proved. The proof is not the
--     one-liner it looks like: reading a directed walk as an undirected one can
--     BREAK reducedness, and it does so exactly when consecutive arcs repeat an
--     edge, which forces that edge to be a self-loop. So the argument first
--     kills self-loops using acyclicity itself, then converts.
--
--   * the converse fails, and `diamond` is the counterexample: `0` fans out to
--     `1` and `2`, which reconverge on `3`. It is wheel-free, and connected,
--     and it carries a reduced undirected closed walk. This is the shape
--     Thm 5.9's finiteness forbids — RECONVERGENCE, not feedback. Branching
--     alone is fine and is what makes the shape properadic.
--
-- Two reusable certificates carry the examples, so no example argues from
-- scratch: `Ranked` (a height every arc strictly increases) implies
-- `WheelFree`, and `Matched` (no vertex carries two distinct link edges, no
-- edge links a vertex to itself) implies `Acyclic`.
--
-- Every predicate is exercised in both directions, so none of them is
-- vacuously true or vacuously false. `empty` refutes `Connected`; `diamond`
-- refutes `Acyclic`; `wheel` refutes `WheelFree` — that last one matters,
-- because without an inhabited closed directed walk `ranked⇒wheel-free` would
-- prove nothing.
--
-- ── THE COROLLA FAMILY, AND WHY IT IS GENERIC RATHER THAN AN EXAMPLE ────────
-- `corolla A B` is one vertex with in-profile `A` and out-profile `B`, and
-- `corolla-cell` promotes it to a `Cell` for EVERY pair of profiles over ANY
-- colour set. That is the difference between the worked examples, which show
-- the predicates are not vacuous at one colour set, and this, which shows the
-- cell class is populated at every arity — so nothing above can be a theorem
-- about a family with finitely many inhabitants.
--
-- Its wiring is `swap-match`, the matching that sends a concatenation to the
-- same two blocks in the other order: the corolla's sources are `B ++ A`, its
-- out-ports followed by its input legs, and its sinks are `A ++ B`, its
-- in-ports followed by its output legs, so every port crosses to the leg of
-- the same name. `swap-follow` is the fact this needs and the only one — an
-- edge of one block lands in that block on the other side — from which every
-- edge has a leg end, hence no arc, hence all four certificates at once.
--
-- The block swap is the arity's SYMMETRY and it is not an artifact of the
-- presentation. Grafting must interleave two operands' vertex blocks, and
-- `node` publishes its ports to one end of the interface; whichever end that
-- is, the other operand's block has to cross it. Choosing the opposite end
-- moves the crossing from one whiskering to the other and does not remove it.
--
-- ── WHAT THIS MODULE DOES NOT CLAIM ─────────────────────────────────────────
-- There are no graphical MAPS here, hence no `Γ`, no `Θ`, no Segal condition,
-- and no nerve. `Gandr.Shape.Decidable` builds the map layer. The arity
-- OPERATIONS — grafting through its inductive graph, and the heterogeneous
-- comparison — are not here either; `Gandr.Shape.Graft` builds them over the
-- listing algebra this module ends with. The unit is already DERIVED here
-- (`idn`), which is the one thing `Gandr.Arity.Path`'s header predicted would
-- not generalize from the linear kit, and the symmetry (`swap-match`) is here
-- because the corolla needs it, not because grafting does — though it does.
--
-- That the carrier's objects are exactly the finite directed graphs with legs
-- is a design claim, not a mechanized one: a graph determines the vertex list
-- and the source-to-sink bijection, and conversely, but no adequacy theorem
-- against an independent presentation is proved here.
------------------------------------------------------------------------------

module Gandr.Shape.Graph where

-- Arithmetic is repackaged under one qualified name, so the ordering lemmas
-- keep their standard-library names at the use site.
private
  module ℕ where
    open import Data.Nat public
      hiding (module ℕ)
    open import Data.Nat.Properties public
open ℕ
  using (ℕ)
  using (_<_)

open import Axiom.UniquenessOfIdentityProofs
  using (UIP)
  using (module Decidable⇒UIP)
open import Data.Empty
  using (⊥-elim)
open import Data.Empty.Polymorphic
  using ()
  renaming (⊥ to ⊥ℓ)
open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (_++_)
open import Data.List.Properties
  renaming (≡-dec to list-dec)
open import Data.Maybe.Base
  using (Maybe)
  using (just)
  using (maybe′)
  using (nothing)
open import Data.Maybe.Properties
  using (just-injective)
open import Data.Product.Base
  using (Σ)
  using (_×_)
  using (_,_)
  using (proj₁)
  using (proj₂)
open import Data.Product.Properties
  using (,-injectiveˡ)
  using (,-injectiveʳ)
  renaming (,-injectiveʳ-UIP to ,-injʳ-uip)
  renaming (≡-dec to pair-dec)
open import Relation.Binary.Definitions
  using (DecidableEquality)
open import Data.Bool.Base
  using (true)
  using (false)
open import Relation.Nullary.Decidable
  using (Dec)
  using (yes)
  using (no)
  using (does)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
  using (swap)
  renaming ([_,_]′ to case⊎)
  renaming (map to smap)
open import Function.Base
  using (id)
open import Data.Unit.Base
  using (⊤)
  using (tt)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (_≢_)
  using (refl)
  using (sym)
  using (trans)
  using (cong)
  using (cong₂)
open import Relation.Nullary.Negation
  using (¬_)

-- ════════════════════════════════════════════════════════════════════════════
-- THE LISTING RELATIONS. Everything the carrier needs about lists is an
-- inductive relation, so no defined function enters a matchable index and no
-- law has to be carried as a field.
--
-- These are list-generic and have exactly one consumer today. They stay here
-- rather than in a module of their own for the reason `Gandr.Arity.Path` gives
-- for the arity interface: one consumer does not determine an abstraction.
-- ════════════════════════════════════════════════════════════════════════════

-- A position in a list — the name of a vertex, of an edge, or of a leg. It is
-- generic in the element type because the vertex listing and the wire listing
-- are lists of different things and are indexed the same way.
data Ix {a} {A : Set a} : List A → Set a where
  -- the first position
  here
    : ∀ {x xs}
    → Ix (x ∷ xs)
  -- one position further along
  there
    : ∀ {x xs}
    → Ix xs
    → Ix (x ∷ xs)

-- Reading a listing at a position.
lookup
  : ∀ {a} {A : Set a}
  → (xs : List A)
  → Ix xs
  → A
lookup (x ∷ xs) here = x
lookup (x ∷ xs) (there i) = lookup xs i

-- ════════════════════════════════════════════════════════════════════════════
-- SUM PLUMBING. Every incidence value below is a sum — a vertex or a leg — and
-- the derived operations reindex one side at a time. These three facts are all
-- that is ever needed about that, and they are stated once rather than being
-- re-derived by `with` at each use site.
-- ════════════════════════════════════════════════════════════════════════════

-- The two one-sided reindexings commute, because each is the two-sided one
-- with the other component left alone.
smap-exch
  : ∀ {a b c d} {A : Set a} {B : Set b} {C : Set c} {D : Set d}
  → (f : A → C) (g : B → D)
  → (s : A ⊎ B)
  → smap f id (smap id g s) ≡ smap id g (smap f id s)
smap-exch f g (inj₁ i) = refl
smap-exch f g (inj₂ j) = refl

-- Exchanging the sides commutes with reindexing them.
swap-smap
  : ∀ {a b c d} {A : Set a} {B : Set b} {C : Set c} {D : Set d}
  → (f : A → C) (g : B → D)
  → (s : A ⊎ B)
  → swap (smap f g s) ≡ smap g f (swap s)
swap-smap f g (inj₁ i) = refl
swap-smap f g (inj₂ j) = refl

-- The two injections are distinct. This is what lets a leg end refute an arc,
-- which demands a vertex — an `inj₁` — at both ends.
inj₂≢inj₁
  : ∀ {a b} {A : Set a} {B : Set b} {x : B} {y : A}
  → inj₂ {A = A} x ≢ inj₁ y
inj₂≢inj₁ ()

module _ {ℓ} (Ob : Set ℓ) where

  -- A vertex's port listing: its input ports and its output ports, each in the
  -- chosen order. This is the ordered representation (C3) as carried data —
  -- not a condition checked about a graph, but the graph's own content.
  record Prof : Set ℓ where
    constructor prof
    field
      -- the ordered input ports
      dom : List Ob
      -- the ordered output ports
      cod : List Ob

  -- The graph of concatenation. `nil` is the unit clause of a
  -- graph-of-multiplication witness, which is why it may copy `ys` across two
  -- result indices; see the header for what that costs and what it does not.
  data Append : List Ob → List Ob → List Ob → Set ℓ where
    -- appending onto the empty prefix changes nothing
    nil
      : ∀ {ys}
      → Append [] ys ys
    -- extend the witness by the prefix's first element
    cons
      : ∀ {x xs ys zs}
      → Append xs ys zs
      → Append (x ∷ xs) ys (x ∷ zs)

  -- `Insert x ys zs` says `zs` is `ys` with one `x` inserted. The POSITION is
  -- the datum: this is where one edge's choice of partner lives.
  data Insert (x : Ob) : List Ob → List Ob → Set ℓ where
    -- insert at the front
    head
      : ∀ {ys}
      → Insert x ys (x ∷ ys)
    -- insert further along, past one element
    tail
      : ∀ {y ys zs}
      → Insert x ys zs
      → Insert x (y ∷ ys) (y ∷ zs)

  -- A colour-preserving bijection from sources to sinks, presented
  -- canonically: walk the sources in order, each choosing its partner out of
  -- the sinks that remain. One term per bijection — there is no derivation
  -- order to quotient by, which is what `Match` buys over a permutation
  -- presented by transpositions.
  data Match : List Ob → List Ob → Set ℓ where
    -- no sources left, and therefore no sinks either
    []
      : Match [] []
    -- the first source takes its partner, and the rest match what is left
    _∷_
      : ∀ {x xs ys zs}
      → Insert x ys zs
      → Match xs ys
      → Match (x ∷ xs) zs
    -- the first source takes another SOURCE as its partner, chosen out of the
    -- sources that remain; the position of that choice is the datum, exactly
    -- as for `_∷_`. This is the boundary `∩` — gandr's cut — and it is what
    -- makes the wiring all of the DOWNWARD category rather than only its
    -- bijective part. No constructor pairs two sinks: that would be the cup
    -- `∪`, and its absence is what keeps the nodeless loop inexpressible.
    -- Colours are unconstrained on purpose: which caps are legitimate is a
    -- question about polarity, kept expressible and refutable by predicate
    -- rather than inexpressible by typing — the choice `node` makes for wheels.
    cap
      : ∀ {x y xs xs′ ys}
      → Insert y xs xs′
      → Match xs ys
      → Match (x ∷ xs′) ys

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE CARRIER. `Shape Γ Δ` is a finite directed graph with input legs `Γ`
  -- and output legs `Δ`, both ordered.
  -- ══════════════════════════════════════════════════════════════════════════

  data Shape : List Ob → List Ob → Set ℓ where
    -- no vertex: the remaining sources are wired straight onto the remaining
    -- sinks. The identity arity is this constructor at the identity matching,
    -- so the unit is DERIVED rather than adjoined — see `idn`.
    wires
      : ∀ {Γ Δ}
      → Match Γ Δ
      → Shape Γ Δ
    -- a vertex with in-profile `A` and out-profile `B`, whose ports are
    -- published to the rest of the graph: `B` joins the sources, `A` the sinks
    node
      : ∀ {Γ Δ Γ′ Δ′}
      → (A B : List Ob)
      → Append B Γ Γ′
      → Append A Δ Δ′
      → Shape Γ′ Δ′
      → Shape Γ Δ

-- ════════════════════════════════════════════════════════════════════════════
-- THE DERIVED LAYER. The colour set is implicit from here on, since it is
-- recovered from the shapes' types.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} where

  -- Totality: the graph of concatenation holds at concatenation's own value.
  -- `cat-graph`'s analogue.
  append-graph
    : (xs ys : List Ob)
    → Append Ob xs ys (xs ++ ys)
  append-graph [] ys = nil
  append-graph (x ∷ xs) ys = cons (append-graph xs ys)

  -- Functionality: the witness really is a graph, determined by its inputs.
  -- `cat-fun`'s analogue, and the strongest statement available over `nil`'s
  -- shape — the second result index stays a variable, so nothing is deleted.
  append-fun
    : ∀ {xs ys zs zs′}
    → Append Ob xs ys zs
    → Append Ob xs ys zs′
    → zs ≡ zs′
  append-fun nil nil = refl
  append-fun (cons p) (cons q) = cong (_ ∷_) (append-fun p q)

  -- The identity matching: each source onto the sink beside it.
  idn-match
    : (Γ : List Ob)
    → Match Ob Γ Γ
  idn-match [] = []
  idn-match (x ∷ Γ) = head ∷ idn-match Γ

  -- The identity arity: no vertex, one leg-to-leg edge per wire. A
  -- CONSTRUCTION, which is precisely what the linear kit predicted would not
  -- generalize from it — there the unit is a constructor.
  idn
    : (Γ : List Ob)
    → Shape Ob Γ Γ
  idn Γ = wires (idn-match Γ)

  -- ══════════════════════════════════════════════════════════════════════════
  -- VERTICES AND EDGES. A vertex is a pointer into the node chain; an edge is
  -- a position in the innermost source pool, which by construction has exactly
  -- one element per edge.
  -- ══════════════════════════════════════════════════════════════════════════

  -- THE VERTEX LISTING, which is the shape's primary content: one profile per
  -- vertex, outermost first. Every notion below is read off this and the
  -- matching; nothing is reconstructed from an incidence table.
  verts
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → List (Prof Ob)
  verts (wires m) = []
  verts (node A B p q S) = prof A B ∷ verts S

  -- A vertex is a position in that listing.
  Vtx
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → Set ℓ
  Vtx S = Ix (verts S)

  -- The vertex's in-profile: its ordered input ports.
  ins
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Vtx S
    → List Ob
  ins {S} v = Prof.dom (lookup (verts S) v)

  -- The vertex's out-profile, on the same footing.
  outs
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Vtx S
    → List Ob
  outs {S} v = Prof.cod (lookup (verts S) v)

  -- The innermost source pool: one element per edge of the shape.
  pool
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → List Ob
  pool (wires {Γ} m) = Γ
  pool (node A B p q S) = pool S

  -- The edges of a shape, on the same footing as its vertices.
  Edg
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → Set ℓ
  Edg S = Ix (pool S)

  -- Splitting a position along an `Append` witness. Structural recursion on
  -- the witness, so no cardinality arithmetic and no re-indexing plumbing.
  --
  -- The recursive clause reindexes through `smap` rather than through a `with`
  -- on the recursive call, and that is deliberate: a `with` compiles to an
  -- auxiliary the caller cannot name, so `split (cons p) (there i)` is stuck on
  -- a term no lemma can rewrite. Written compositionally it reduces to an
  -- application whose argument IS the recursive call, and every fact below
  -- about `split` is then one `cong` away. `origin` and `dest` are written the
  -- same way for the same reason.
  split
    : ∀ {xs ys zs}
    → Append Ob xs ys zs
    → Ix zs
    → Ix xs ⊎ Ix ys
  split nil i = inj₂ i
  split (cons p) here = inj₁ here
  split (cons p) (there i) = smap there id (split p i)

  -- The two injections `split` inverts: a position of the prefix, and a
  -- position of the suffix, each read as a position of the concatenation.
  left
    : ∀ {xs ys zs}
    → Append Ob xs ys zs
    → Ix xs
    → Ix zs
  left nil ()
  left (cons p) here = here
  left (cons p) (there i) = there (left p i)

  right
    : ∀ {xs ys zs}
    → Append Ob xs ys zs
    → Ix ys
    → Ix zs
  right nil j = j
  right (cons p) j = there (right p j)

  -- Where a position of a concatenation came from. This is a VIEW rather than
  -- a sum: a consumer eliminates it by matching, which refines the position it
  -- was given, instead of carrying an equation it then has to transport along.
  data Part {xs ys zs} (p : Append Ob xs ys zs) : Ix zs → Set ℓ where
    -- a position of the prefix
    front
      : (i : Ix xs)
      → Part p (left p i)
    -- a position of the suffix
    back
      : (j : Ix ys)
      → Part p (right p j)

  part
    : ∀ {xs ys zs}
    → (p : Append Ob xs ys zs)
    → (e : Ix zs)
    → Part p e
  part nil e = back e
  part (cons p) here = front here
  part (cons p) (there e) with part p e
  ... | front i = front (there i)
  ... | back j = back j

  -- And the two injections are sections of `split`, which is what makes the
  -- view's two cases compute.
  split-left
    : ∀ {xs ys zs}
    → (p : Append Ob xs ys zs)
    → (i : Ix xs)
    → split p (left p i) ≡ inj₁ i
  split-left nil ()
  split-left (cons p) here = refl
  split-left (cons p) (there i) = cong (smap there id) (split-left p i)

  split-right
    : ∀ {xs ys zs}
    → (p : Append Ob xs ys zs)
    → (j : Ix ys)
    → split p (right p j) ≡ inj₂ j
  split-right nil j = refl
  split-right (cons p) j = cong (smap there id) (split-right p j)

  -- Reading an insertion as the position it chose.
  slot
    : ∀ {x ys zs}
    → Insert Ob x ys zs
    → Ix zs
  slot head = here
  slot (tail i) = there (slot i)

  -- Re-indexing the positions an insertion did not take.
  past
    : ∀ {x ys zs}
    → Insert Ob x ys zs
    → Ix ys
    → Ix zs
  past head j = there j
  past (tail i) here = here
  past (tail i) (there j) = there (past i j)

  -- `past`'s inverse view: a position in the extended list is either the
  -- inserted element itself, or one of the originals. `split`'s analogue for
  -- `Insert`, and it is what reads a cap from the partner's side.
  isplit
    : ∀ {x ys zs}
    → Insert Ob x ys zs
    → Ix zs
    → ⊤ ⊎ Ix ys
  isplit head here = inj₁ tt
  isplit head (there j) = inj₂ j
  isplit (tail i) here = inj₂ here
  isplit (tail i) (there j) = smap id there (isplit i j)

  -- Following an edge through the matching, from its source position to its
  -- sink position.
  follow
    : ∀ {xs zs}
    → Match Ob xs zs
    → Ix xs
    → Ix xs ⊎ Ix zs
  follow (i ∷ m) here = inj₂ (slot i)
  follow (i ∷ m) (there e) = smap there (past i) (follow m e)
  follow (cap j m) here = inj₁ (there (slot j))
  follow (cap j m) (there e) =
    case⊎
      (λ _ → inj₁ here)
      (λ e′ → smap (λ i → there (past j i)) id (follow m e′))
      (isplit j e)

  -- A wiring is FLOW-THROUGH when it has no cap: every source takes a sink.
  -- This is the pre-cap fragment — the bijective part `Σ = dBD ∩ uBD` of the
  -- downward category — named as a predicate rather than carved out as a type,
  -- which is the same discipline `WheelFree` follows.
  data CapFree : ∀ {xs zs} → Match Ob xs zs → Set ℓ where
    []
      : CapFree []
    _∷_
      : ∀ {x xs ys zs}
      → (i : Insert Ob x ys zs)
      → {m : Match Ob xs ys}
      → CapFree m
      → CapFree (i ∷ m)

  -- On a flow-through wiring, following an edge lands on a sink, so the sum
  -- collapses and the original total function is recovered. Every fact proved
  -- before the cap existed is a fact about THIS function.
  follow⁺
    : ∀ {xs zs}
    → {m : Match Ob xs zs}
    → CapFree m
    → Ix xs
    → Ix zs
  follow⁺ (i ∷ c) here = slot i
  follow⁺ (i ∷ c) (there e) = past i (follow⁺ c e)

  -- and the two agree, which is the bridge every consumer of the old lemmas
  -- crosses exactly once
  follow-capfree
    : ∀ {xs zs}
    → {m : Match Ob xs zs}
    → (c : CapFree m)
    → (e : Ix xs)
    → follow m e ≡ inj₂ (follow⁺ c e)
  follow-capfree (i ∷ c) here = refl
  follow-capfree (i ∷ c) (there e) =
    cong (smap there (past i)) (follow-capfree c e)


  -- ══════════════════════════════════════════════════════════════════════════
  -- THE BLOCK SWAP. `swap-match` matches a concatenation onto the same two
  -- blocks in the other order, and it is what wires a vertex to its own legs:
  -- the corolla's sources are `B ++ A` — its out-ports, then its input legs —
  -- and its sinks are `A ++ B`, its in-ports then its output legs.
  --
  -- It is the SYMMETRY of the arity, and it is not avoidable by presentation.
  -- Grafting has to interleave the two operands' vertex blocks, and `node`
  -- publishes its ports to one end of the interface, so whichever end that is,
  -- the other operand's block has to cross it.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Appending nothing renames nothing, so the witness IS a matching.
  match-nil
    : ∀ {ys zs}
    → Append Ob ys [] zs
    → Match Ob ys zs
  match-nil nil = []
  match-nil (cons p) = head ∷ match-nil p

  -- and it is flow-through: the pre-cap lemmas below are facts about `follow⁺`
  match-nil-capfree
    : ∀ {ys zs}
    → (p : Append Ob ys [] zs)
    → CapFree (match-nil p)
  match-nil-capfree nil = []
  match-nil-capfree (cons p) = head ∷ match-nil-capfree p

  -- and it is the identity on positions, read through `split`
  nil-follow
    : ∀ {ys zs}
    → (p : Append Ob ys [] zs)
    → (e : Ix ys)
    → split p (follow⁺ (match-nil-capfree p) e) ≡ inj₁ e
  nil-follow nil ()
  nil-follow (cons p) here = refl
  nil-follow (cons p) (there e) = cong (smap there id) (nil-follow p e)

  -- The position an element occupies when it is inserted between a prefix and
  -- a suffix. Both `Append` witnesses are supplied, so the position is named by
  -- the witnesses rather than computed from a length.
  insert-mid
    : ∀ {as x ys r s}
    → Append Ob as ys r
    → Append Ob as (x ∷ ys) s
    → Insert Ob x r s
  insert-mid nil nil = head
  insert-mid (cons p) (cons q) = tail (insert-mid p q)

  -- That position is the head of the suffix block.
  mid-slot
    : ∀ {as x ys r s}
    → (p : Append Ob as ys r)
    → (q : Append Ob as (x ∷ ys) s)
    → split q (slot (insert-mid p q)) ≡ inj₂ here
  mid-slot nil nil = refl
  mid-slot (cons p) (cons q) = cong (smap there id) (mid-slot p q)

  -- and every other position is where it was, with the suffix side shifted
  -- past the inserted element
  mid-past
    : ∀ {as x ys r s}
    → (p : Append Ob as ys r)
    → (q : Append Ob as (x ∷ ys) s)
    → (w : Ix r)
    → split q (past (insert-mid p q) w) ≡ smap id there (split p w)
  mid-past nil nil w = refl
  mid-past (cons p) (cons q) here = refl
  mid-past (cons p) (cons q) (there w) =
    trans
      (cong (smap there id) (mid-past p q w))
      (smap-exch there there (split p w))

  -- The block swap itself: each source of the prefix block takes the position
  -- its own block occupies on the other side, and the suffix block is then
  -- matched by recursion. The existential remainder is supplied by
  -- `append-graph`, which is the sanctioned way to speak concatenation's value.
  swap-match
    : ∀ {xs ys zs ws}
    → Append Ob xs ys zs
    → Append Ob ys xs ws
    → Match Ob zs ws
  swap-match nil q = match-nil q
  swap-match {ys} (cons {xs} p) q =
    insert-mid (append-graph ys xs) q ∷ swap-match p (append-graph ys xs)

  -- the block swap is flow-through too — it permutes sources onto sinks and
  -- never caps
  swap-match-capfree
    : ∀ {xs ys zs ws}
    → (p : Append Ob xs ys zs)
    → (q : Append Ob ys xs ws)
    → CapFree (swap-match p q)
  swap-match-capfree nil q = match-nil-capfree q
  swap-match-capfree {ys} (cons {xs} p) q =
    insert-mid (append-graph ys xs) q ∷ swap-match-capfree p (append-graph ys xs)

  -- AND WHAT IT DOES, which is the fact every consumer actually wants: an edge
  -- in one block lands in that same block on the other side. Stated through
  -- `split` on both sides, so it says the blocks are exchanged and nothing
  -- inside a block is permuted.
  swap-follow
    : ∀ {xs ys zs ws}
    → (p : Append Ob xs ys zs)
    → (q : Append Ob ys xs ws)
    → (e : Ix zs)
    → split q (follow⁺ (swap-match-capfree p q) e) ≡ swap (split p e)
  swap-follow nil q e = nil-follow q e
  swap-follow {ys} (cons {xs} p) q here = mid-slot (append-graph ys xs) q
  swap-follow {ys} (cons {xs} p) q (there e) =
    trans
      (mid-past
        (append-graph ys xs)
        q
        (follow⁺ (swap-match-capfree p (append-graph ys xs)) e))
      (trans
        (cong (smap id there) (swap-follow p (append-graph ys xs) e))
        (sym (swap-smap there id (split p e))))

  -- ══════════════════════════════════════════════════════════════════════════
  -- INCIDENCE, DERIVED. An edge's source end is at a vertex's out-port or at
  -- an input leg; its sink end is at a vertex's in-port or at an output leg.
  -- Both are computed by tracing the edge outward through the node chain.
  -- ══════════════════════════════════════════════════════════════════════════

  origin
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Edg S
    → Vtx S ⊎ Ix Γ
  origin (wires m) e = inj₂ e
  origin (node A B p q S) e =
    case⊎
      (λ v → inj₁ (there v))
      (λ i → smap (λ _ → here) id (split p i))
      (origin S e)

  -- A BOUNDARY POINT of a shape: an input leg or an output leg. Polarity is
  -- carried data here rather than a positional convention, because a capped
  -- edge ends at a SOURCE and the incidence has to be able to say so. This is
  -- the orientation datum arriving where it is actually needed.
  Leg
    : List Ob
    → List Ob
    → Set ℓ
  Leg Γ Δ = Ix Γ ⊎ Ix Δ

  dest
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Edg S
    → Vtx S ⊎ Leg Γ Δ
  dest (wires m) e = inj₂ (follow m e)
  dest (node A B p q S) e =
    case⊎
      (λ v → inj₁ (there v))
      (case⊎
        (λ i → smap (λ _ → here) inj₁ (split p i))
        (λ j → smap (λ _ → here) inj₂ (split q j)))
      (dest S e)

  -- `Arc` is the directed incidence — an edge with BOTH ends attached — and
  -- `Link` is the same edge read in either orientation. Only `Link` is a
  -- datatype, because only the orientation is ever case-analysed.
  record Arc {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) (u v : Vtx S) : Set ℓ where
    constructor arc
    field
      -- `e` leaves `u`
      from : origin S e ≡ inj₁ u
      -- `e` enters `v`
      into : dest S e ≡ inj₁ v

  data Link {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) : Vtx S → Vtx S → Set ℓ where
    -- traverse `e` in its own direction
    along
      : ∀ {u v}
      → Arc S e u v
      → Link S e u v
    -- traverse `e` against its direction
    against
      : ∀ {u v}
      → Arc S e v u
      → Link S e u v

  -- Links are symmetric, which is what makes undirected reachability an
  -- equivalence and what the diamond's cycle is built from.
  link-sym
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {e u v}
    → Link S e u v
    → Link S e v u
  link-sym (along a) = against a
  link-sym (against a) = along a

  -- Adjacency: linked by SOME edge. The edge stays explicit so a consumer can
  -- name it, but reachability never inspects it.
  record Adj {Γ Δ} (S : Shape Ob Γ Δ) (u v : Vtx S) : Set ℓ where
    constructor adj
    field
      -- the edge doing the joining
      edge : Edg S
      -- and the orientation it is traversed in
      step : Link S edge u v

  -- Adjacency is symmetric, by symmetry of links.
  adj-sym
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v}
    → Adj S u v
    → Adj S v u
  adj-sym (adj e l) = adj e (link-sym l)

  -- ══════════════════════════════════════════════════════════════════════════
  -- REACHABILITY. The reflexive–transitive closure of adjacency, in the snoc
  -- shape `Gandr.Arity.Path` uses. It forgets which edges were traversed,
  -- which is exactly why it composes and reverses freely — connectivity does
  -- not care about reducedness and should not pay for it.
  -- ══════════════════════════════════════════════════════════════════════════

  data Reach {Γ Δ} (S : Shape Ob Γ Δ) (u : Vtx S) : Vtx S → Set ℓ where
    -- the empty walk
    stop
      : Reach S u u
    -- extend by one adjacency on the right
    onward
      : ∀ {m v}
      → Reach S u m
      → Adj S m v
      → Reach S u v

  -- Reachability composes, by recursion on the second walk.
  reach-trans
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u m v}
    → Reach S u m
    → Reach S m v
    → Reach S u v
  reach-trans r stop = r
  reach-trans r (onward s a) = onward (reach-trans r s) a

  -- Reachability reverses, since every adjacency does.
  reach-sym
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v}
    → Reach S u v
    → Reach S v u
  reach-sym stop = stop
  reach-sym (onward r a) = reach-trans (onward stop (adj-sym a)) (reach-sym r)

  -- ══════════════════════════════════════════════════════════════════════════
  -- WALKS. Two walk notions, doing two jobs. `Dir` chains ARCS and is what a
  -- wheel is a closed instance of. `Walk` chains LINKS and is REDUCED: it is
  -- indexed by the edge it last traversed and refuses to traverse it again.
  -- ══════════════════════════════════════════════════════════════════════════

  -- The freshness side condition, as a constructor-headed relation rather than
  -- a defined function, so that it never has to reduce inside an index.
  data Fresh {A : Set ℓ} (x : A) : Maybe A → Set ℓ where
    -- nothing has been traversed yet, so anything is fresh
    opening
      : Fresh x nothing
    -- the previous traversal used a different edge
    apart
      : ∀ {y}
      → x ≢ y
      → Fresh x (just y)

  data Walk {Γ Δ} (S : Shape Ob Γ Δ) (u : Vtx S) : Vtx S → Maybe (Edg S) → Set ℓ where
    -- the empty walk, which has traversed nothing
    stay
      : Walk S u u nothing
    -- extend by one link, which must not be the link just traversed
    hop
      : ∀ {m v b}
      → (e : Edg S)
      → Walk S u m b
      → Link S e m v
      → Fresh e b
      → Walk S u v (just e)

  data Dir {Γ Δ} (S : Shape Ob Γ Δ) (u : Vtx S) : Vtx S → Maybe (Edg S) → Set ℓ where
    -- the empty directed walk
    idle
      : Dir S u u nothing
    -- extend by one arc; no freshness is imposed, since a directed walk cannot
    -- backtrack except through a self-loop, which `dir⇒walk` handles
    next
      : ∀ {m v b}
      → (e : Edg S)
      → Dir S u m b
      → Arc S e m v
      → Dir S u v (just e)

  -- Every reduced walk is in particular a reachability witness. This is the
  -- forgetful direction and it is cheap; the converse is not needed and is not
  -- claimed.
  walk⇒reach
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v b}
    → Walk S u v b
    → Reach S u v
  walk⇒reach stay = stop
  walk⇒reach (hop e w l _) = onward (walk⇒reach w) (adj e l)

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE PREDICATES. `WheelFree` forbids feedback; `Connected` and `Acyclic`
  -- are the combinatorial readings of `β₀ = 1` and `β₁ = 0` argued for in the
  -- header.
  -- ══════════════════════════════════════════════════════════════════════════

  -- No directed cycle: no closed directed walk that traversed at least one arc.
  WheelFree
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → Set ℓ
  WheelFree S = ∀ {v : Vtx S} {e : Edg S} → ¬ Dir S v v (just e)

  -- No undirected cycle: no closed REDUCED walk that traversed at least one
  -- edge. The `just e` index is what "nontrivial" means here.
  Acyclic
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → Set ℓ
  Acyclic S = ∀ {v : Vtx S} {e : Edg S} → ¬ Walk S v v (just e)

  record Connected {Γ Δ} (S : Shape Ob Γ Δ) : Set ℓ where
    field
      -- there is at least one vertex, so the component count is not zero
      root : Vtx S
      -- and there is at most one component
      span : (v : Vtx S) → Reach S root v

  -- Connectivity does not depend on the root: any two vertices are joined.
  -- This is what makes the record's `root` a witness rather than extra
  -- structure.
  reach-any
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Connected S
    → (u v : Vtx S)
    → Reach S u v
  reach-any c u v =
    reach-trans (reach-sym (Connected.span c u)) (Connected.span c v)

  record SimplyConn {Γ Δ} (S : Shape Ob Γ Δ) : Set ℓ where
    field
      -- `β₀ = 1`
      connected : Connected S
      -- `β₁ = 0`
      acyclic : Acyclic S

  -- ══════════════════════════════════════════════════════════════════════════
  -- CERTIFICATE 1: RANK. A height that every arc strictly increases refutes
  -- every wheel at once, and is what every worked example below uses.
  -- ══════════════════════════════════════════════════════════════════════════

  record Ranked {Γ Δ} (S : Shape Ob Γ Δ) : Set ℓ where
    field
      -- the height of a vertex
      rank : Vtx S → ℕ
      -- every arc goes strictly uphill
      climbs : (e : Edg S) (u v : Vtx S) → Arc S e u v → rank u < rank v

  -- A nonempty directed walk strictly increases rank. Recursion peels the last
  -- arc, so the two clauses are "one arc" and "more than one".
  dir-rank
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v e}
    → (R : Ranked S)
    → Dir S u v (just e)
    → Ranked.rank R u < Ranked.rank R v
  dir-rank R (next e idle a) = Ranked.climbs R e _ _ a
  dir-rank R (next e (next f d a′) a) =
    ℕ.<-trans (dir-rank R (next f d a′)) (Ranked.climbs R e _ _ a)

  -- Hence a ranked shape is wheel-free: a closed directed walk would put a
  -- rank strictly below itself.
  ranked⇒wheel-free
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Ranked S
    → WheelFree S
  ranked⇒wheel-free R d = ℕ.n≮n _ (dir-rank R d)

  -- ══════════════════════════════════════════════════════════════════════════
  -- CERTIFICATE 2: MATCHING. If the internal edges form a matching — at most
  -- one per vertex, none a loop — then a reduced closed walk cannot exist,
  -- because it would have to leave and return through two distinct edges at
  -- some vertex.
  --
  -- This is a genuinely restrictive certificate: it says the internal part of
  -- the shape is a matching. It discharges `Acyclic` for the corolla-shaped
  -- and two-vertex examples below, and it does NOT discharge a tree with a
  -- branching internal vertex, so the general forest certificate is missing
  -- and its absence is located here.
  --
  -- The obligation is: given a rooted parent structure on the vertices, refute
  -- every nontrivial reduced closed walk. The natural attempt is a depth
  -- function with "every link joins adjacent depths, and each vertex has one
  -- downward link", and it PROVABLY does not close. A reduced walk in a tree
  -- is up-phase then down-phase — once a step descends into a vertex through
  -- that vertex's parent edge, no later step may ascend, since ascending would
  -- have to reuse that same edge and reducedness forbids it. The up-phase
  -- strictly decreases depth and the down-phase strictly increases it, so a
  -- closed walk balances the two and depth alone derives no contradiction:
  -- `up up down down` returns to the starting DEPTH while a tree guarantees
  -- only that it reaches a different VERTEX. Depth cannot see that.
  --
  -- What discharges it is the address rather than the depth: assign each vertex
  -- the list of parent edges from it to the root and require that every link
  -- either extends or shortens that list by exactly its own edge. That is a
  -- list-bookkeeping induction of real size and it is deferred, not overlooked.
  -- Nothing below depends on it; it is needed only to certify richer examples.
  -- ══════════════════════════════════════════════════════════════════════════

  record Matched {Γ Δ} (S : Shape Ob Γ Δ) : Set ℓ where
    field
      -- no edge links a vertex to itself
      unlooped : (e : Edg S) (v : Vtx S) → ¬ Link S e v v
      -- no vertex carries two distinct link edges
      unbranched
        : (e f : Edg S) (u v w : Vtx S)
        → Link S e v u
        → Link S f v w
        → e ≡ f

  -- The refutation is two levels deep and no more: a closed reduced walk is
  -- either a single link from a vertex to itself, or has two consecutive links
  -- meeting at a vertex, and both are excluded.
  matched⇒acyclic
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Matched S
    → Acyclic S
  matched⇒acyclic M (hop e stay l opening) = Matched.unlooped M e _ l
  matched⇒acyclic M (hop e (hop f w l′ _) l (apart e≢f)) =
    e≢f (Matched.unbranched M e f _ _ _ l (link-sym l′))

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE CONTENT LEMMA — HRY Rmk 2.36's easy direction. Simple connectivity is
  -- strictly stronger than wheel-freeness; `diamond` below exhibits the
  -- strictly.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Acyclicity kills self-loops outright, since a loop at `v` is already a
  -- reduced closed walk of length one.
  loop-free
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {e v}
    → Acyclic S
    → ¬ Arc S e v v
  loop-free ac a = ac (hop _ stay (along a) opening)

  -- The last arc of a nonempty directed walk ends where the walk does.
  dir-into
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v e}
    → Dir S u v (just e)
    → dest S e ≡ inj₁ v
  dir-into (next _ _ a) = Arc.into a

  -- If a directed walk's next arc repeats its last edge then that edge is a
  -- self-loop: the repeated edge must both end and start at the shared vertex.
  -- This is the only obstruction to reading a directed walk as a reduced one.
  arc-repeat
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u m v : Vtx S} {e f : Edg S}
    → Dir S u m (just f)
    → Arc S e m v
    → e ≡ f
    → Arc S e m m
  arc-repeat d a refl = arc (Arc.from a) (dir-into d)

  -- So, given no self-loops, every directed walk IS a reduced undirected walk,
  -- with the same last-edge index.
  dir⇒walk
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {u v : Vtx S} {b}
    → ((e : Edg S) (w : Vtx S) → ¬ Arc S e w w)
    → Dir S u v b
    → Walk S u v b
  dir⇒walk nl idle = stay
  dir⇒walk nl (next e idle a) = hop e stay (along a) opening
  dir⇒walk nl (next e (next f d a′) a) =
    hop e
      (dir⇒walk nl (next f d a′))
      (along a)
      (apart (λ eq → nl e _ (arc-repeat (next f d a′) a eq)))

  -- Undirected acyclicity implies wheel-freeness. Note the order of the two
  -- steps: acyclicity is used FIRST to remove self-loops, and only then is the
  -- directed walk reinterpreted, because the reinterpretation is unsound in the
  -- presence of a loop.
  acyclic⇒wheel-free
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → Acyclic S
    → WheelFree S
  acyclic⇒wheel-free ac d = ac (dir⇒walk (λ _ _ → loop-free ac) d)

  -- HRY Rmk 2.36, easy direction.
  simply-conn⇒wheel-free
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
    → SimplyConn S
    → WheelFree S
  simply-conn⇒wheel-free sc = acyclic⇒wheel-free (SimplyConn.acyclic sc)

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE CELL SHAPE (commitment C1). The design sketch listed wheel-freeness
  -- AND simple connectivity as a pair of fields; the pair is redundant by the
  -- lemma just proved, so wheel-freeness is DERIVED here. That divergence is
  -- the lemma's whole point, and carrying both as fields would have let an
  -- inconsistent `Cell` be written.
  -- ══════════════════════════════════════════════════════════════════════════

  record Cell (Γ Δ : List Ob) : Set ℓ where
    field
      -- the underlying shape
      shape : Shape Ob Γ Δ
      -- connected, and free of undirected cycles
      simply : SimplyConn shape

    -- and therefore free of directed cycles, at no extra cost
    wheel-free : WheelFree shape
    wheel-free = simply-conn⇒wheel-free simply

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE COROLLA FAMILY. One vertex with in-profile `A` and out-profile `B`,
  -- every port wired straight to the leg beside it. This is `Cell`'s GENERIC
  -- inhabitant: one for every pair of profiles, over any colour set, so the
  -- cell class is not a statement about a class the tree cannot populate.
  --
  -- It is also the free generator of the arity — the shape of a single
  -- operation — so the pasting layer above will meet it as the base case of
  -- every generated term, and it is worth having its incidence proved once
  -- rather than re-read off each example.
  -- ══════════════════════════════════════════════════════════════════════════

  corolla
    : (A B : List Ob)
    → Shape Ob A B
  corolla A B =
    node A B (append-graph B A) (append-graph A B)
      (wires (swap-match (append-graph B A) (append-graph A B)))

  -- Its out-profile block runs from the vertex to an output leg. This is where
  -- `swap-follow` is spent: the edge is a source of the `B` block, so it lands
  -- in the `B` block of the sinks, which is the leg side.
  corolla-out
    : ∀ {A B}
    → (i : Ix B)
    → dest (corolla A B) (left (append-graph B A) i) ≡ inj₂ (inj₂ i)
  corolla-out {A} {B} i =
    trans
      (cong
        (case⊎
          (λ j → smap (λ _ → here) inj₁ (split (append-graph B A) j))
          (λ j → smap (λ _ → here) inj₂ (split (append-graph A B) j)))
        (follow-capfree
          (swap-match-capfree (append-graph B A) (append-graph A B))
          (left (append-graph B A) i)))
      (cong
        (smap (λ _ → here) inj₂)
        (trans
          (swap-follow (append-graph B A) (append-graph A B) (left (append-graph B A) i))
          (cong swap (split-left (append-graph B A) i))))

  -- and its in-profile block runs from an input leg to the vertex, which needs
  -- only the section law, since `origin` reads the source pool directly
  corolla-in
    : ∀ {A B}
    → (j : Ix A)
    → origin (corolla A B) (right (append-graph B A) j) ≡ inj₂ j
  corolla-in {A} {B} j =
    cong (smap (λ _ → here) id) (split-right (append-graph B A) j)

  -- So NO edge of a corolla has both ends at a vertex — every edge has a leg
  -- end. That single fact discharges every certificate below, and it is why
  -- the corolla is a cell for arbitrary profiles rather than only for the
  -- small ones that can be checked by hand.
  corolla-no-arc
    : ∀ {A B e u v}
    → ¬ Arc (corolla A B) e u v
  corolla-no-arc {A} {B} {e} a with part (append-graph B A) e
  ... | front i = inj₂≢inj₁ (trans (sym (corolla-out i)) (Arc.into a))
  ... | back j = inj₂≢inj₁ (trans (sym (corolla-in j)) (Arc.from a))

  corolla-no-link
    : ∀ {A B e u v}
    → ¬ Link (corolla A B) e u v
  corolla-no-link (along a) = corolla-no-arc a
  corolla-no-link (against a) = corolla-no-arc a

  -- The matching certificate holds for the strongest possible reason: there is
  -- no link at all, so neither a loop nor a branch can be exhibited.
  corolla-matched
    : ∀ {A B}
    → Matched (corolla A B)
  Matched.unlooped corolla-matched e v l = corolla-no-link l
  Matched.unbranched corolla-matched e f u v w l l′ = ⊥-elim (corolla-no-link l)

  -- One vertex, reachable from itself.
  corolla-connected
    : ∀ {A B}
    → Connected (corolla A B)
  Connected.root corolla-connected = here
  Connected.span corolla-connected here = stop
  Connected.span corolla-connected (there ())

  corolla-simply
    : ∀ {A B}
    → SimplyConn (corolla A B)
  SimplyConn.connected corolla-simply = corolla-connected
  SimplyConn.acyclic corolla-simply = matched⇒acyclic corolla-matched

  -- EVERY PROFILE HAS A CELL.
  corolla-cell
    : (A B : List Ob)
    → Cell A B
  Cell.shape (corolla-cell A B) = corolla A B
  Cell.simply (corolla-cell A B) = corolla-simply

-- ════════════════════════════════════════════════════════════════════════════
-- EQUALITY, LAYER 0: THE CODES. Each witness family is sent to a recursively
-- COMPUTED type built from `⊥`/`×`/`⊎`/`≡`, whose inhabitants can be compared
-- without matching an index, and the round trip is the identity. Every split
-- below is on a single argument or on a plain list, so none of them is the one
-- `--without-K` refuses.
--
-- This layer costs nothing: no hypothesis, no decidability, no h-level.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} where

  -- The code of a concatenation witness. It is built only from `≡`, which is
  -- why the witness turns out to be unique as soon as the colours form a set.
  ACode : (xs ys zs : List Ob) → Set ℓ
  ACode [] ys zs = ys ≡ zs
  ACode (x ∷ xs) ys [] = ⊥ℓ
  ACode (x ∷ xs) ys (z ∷ zs) = (x ≡ z) × ACode xs ys zs

  -- one split, on the witness
  aencode : ∀ {xs ys zs} → Append Ob xs ys zs → ACode xs ys zs
  aencode nil = refl
  aencode (cons p) = refl , aencode p

  -- splits on plain lists, then on `refl` at free indices
  adecode : (xs ys zs : List Ob) → ACode xs ys zs → Append Ob xs ys zs
  adecode [] ys zs refl = nil
  adecode (x ∷ xs) ys [] ()
  adecode (x ∷ xs) ys (z ∷ zs) (refl , c) = cons (adecode xs ys zs c)

  adecode-aencode
    : ∀ {xs ys zs}
    → (p : Append Ob xs ys zs)
    → adecode xs ys zs (aencode p) ≡ p
  adecode-aencode nil = refl
  adecode-aencode (cons p) = cong cons (adecode-aencode p)

  -- The code of an insertion. Unlike `ACode` this one has two branches, which
  -- is exactly the choice of position an insertion records.
  ICode : (x : Ob) (ys zs : List Ob) → Set ℓ
  ICode x ys [] = ⊥ℓ
  ICode x [] (z ∷ zs) = (x ≡ z) × ([] ≡ zs)
  ICode x (w ∷ ws) (z ∷ zs) = ((x ≡ z) × (w ∷ ws ≡ zs)) ⊎ ((w ≡ z) × ICode x ws zs)

  iencode : ∀ {x ys zs} → Insert Ob x ys zs → ICode x ys zs
  iencode (head {ys = []}) = refl , refl
  iencode (head {ys = w ∷ ws}) = inj₁ (refl , refl)
  iencode (tail i) = inj₂ (refl , iencode i)

  idecode : ∀ {x} (ys zs : List Ob) → ICode x ys zs → Insert Ob x ys zs
  idecode ys [] ()
  idecode [] (z ∷ zs) (refl , refl) = head
  idecode (w ∷ ws) (z ∷ zs) (inj₁ (refl , refl)) = head
  idecode (w ∷ ws) (z ∷ zs) (inj₂ (refl , c)) = tail (idecode ws zs c)

  idecode-iencode
    : ∀ {x ys zs}
    → (i : Insert Ob x ys zs)
    → idecode ys zs (iencode i) ≡ i
  idecode-iencode (head {ys = []}) = refl
  idecode-iencode (head {ys = w ∷ ws}) = refl
  idecode-iencode (tail i) = cong tail (idecode-iencode i)

  -- Projecting the recursive half of an insertion code, so that a refutation
  -- goes through `cong` rather than through a `refl` whose indices a previous
  -- `with` has already identified.
  itail : ∀ {x w ws z zs} → ICode x (w ∷ ws) (z ∷ zs) → ICode x ws zs → ICode x ws zs
  itail (inj₁ _) d = d
  itail (inj₂ (_ , c)) d = c

  -- `Match`'s cons carries an EXISTENTIAL remainder, and `Shape`'s node
  -- carries profiles that appear in its recursive argument's type. Neither
  -- constructor is injective by plain `refl`-matching for that reason, so both
  -- are viewed as (nested) pairs and taken apart with a UIP-based projection.
  mview
    : ∀ {x xs zs}
    → Match Ob (x ∷ xs) zs
    → Maybe (Σ (List Ob) (λ ys → Insert Ob x ys zs × Match Ob xs ys))
  mview (i ∷ m) = just (_ , i , m)
  mview (cap j m) = nothing

  -- the cap's view. It carries TWO existentials — the partner's colour and the
  -- sources that remain — where the cons view carries one, so taking it apart
  -- costs two UIP-based projections rather than one.
  cview
    : ∀ {x xs′ ys}
    → Match Ob (x ∷ xs′) ys
    → Maybe (Σ Ob (λ y → Σ (List Ob) (λ xs → Insert Ob y xs xs′ × Match Ob xs ys)))
  cview (i ∷ m) = nothing
  cview (cap j m) = just (_ , _ , j , m)

  -- the profiles a shape's outermost node declares, as a flat non-dependent
  -- projection: this is what refutes a profile mismatch
  profs : ∀ {Γ Δ} → Shape Ob Γ Δ → Maybe (List Ob × List Ob)
  profs (wires m) = nothing
  profs (node A B p q S) = just (A , B)

  -- A node's four lists — its two profiles and its two existential interfaces
  -- — bundled into ONE Σ index, so taking the node apart costs a single
  -- UIP-based projection rather than a nested chain of them.
  Quad : Set ℓ
  Quad = List Ob × List Ob × List Ob × List Ob

  Payload : (Γ Δ : List Ob) → Quad → Set ℓ
  Payload Γ Δ (A , B , Γ′ , Δ′) =
    Append Ob B Γ Γ′ × Append Ob A Δ Δ′ × Shape Ob Γ′ Δ′

  bundle : ∀ {Γ Δ} → Shape Ob Γ Δ → Maybe (Σ Quad (Payload Γ Δ))
  bundle (wires m) = nothing
  bundle (node A B p q S) = just ((A , B , _ , _) , p , q , S)

-- ════════════════════════════════════════════════════════════════════════════
-- EQUALITY, LAYER 1: UNIQUENESS. What closes the residual reflexive equation
-- is an h-level condition on the COLOURS, and nothing more — no decidability,
-- and nothing about shapes, vertices or edges. A consumer that needs only the
-- law layer stops here.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} (uipᵒ : UIP Ob) (uipˡ : UIP (List Ob)) where

  -- An insertion code is not unique — its two branches are the two choices —
  -- but its equality is decided with no decision on the colours at all, since
  -- every leaf is an equality proof and UIP settles those.
  icode? : ∀ {x} (ys zs : List Ob) (c d : ICode {Ob = Ob} x ys zs) → Dec (c ≡ d)
  icode? ys [] () ()
  icode? [] (z ∷ zs) (p , q) (p′ , q′) = yes (cong₂ _,_ (uipᵒ p p′) (uipˡ q q′))
  icode? (w ∷ ws) (z ∷ zs) (inj₁ (p , q)) (inj₁ (p′ , q′)) =
    yes (cong inj₁ (cong₂ _,_ (uipᵒ p p′) (uipˡ q q′)))
  icode? (w ∷ ws) (z ∷ zs) (inj₁ _) (inj₂ _) = no λ ()
  icode? (w ∷ ws) (z ∷ zs) (inj₂ _) (inj₁ _) = no λ ()
  icode? (w ∷ ws) (z ∷ zs) (inj₂ (p , c)) (inj₂ (p′ , d)) with icode? ws zs c d
  ... | no ¬e = no λ eq → ¬e (cong (λ e → itail e c) eq)
  ... | yes refl = yes (cong inj₂ (cong₂ _,_ (uipᵒ p p′) refl))

  -- Equality of insertions, decided. This is the statement the tabular
  -- presentation could not reach and the familial one can.
  insert?
    : ∀ {x ys zs}
    → (i j : Insert Ob x ys zs)
    → Dec (i ≡ j)
  insert? {ys} {zs} i j with icode? ys zs (iencode i) (iencode j)
  ... | no ¬e = no λ eq → ¬e (cong iencode eq)
  ... | yes e =
    yes
      (trans
        (sym (idecode-iencode i))
        (trans (cong (idecode ys zs) e) (idecode-iencode j)))

  -- A concatenation code IS unique — every leaf is an equality proof and there
  -- is no branch — so the witness is too. This retracts an earlier claim that
  -- witness uniqueness over `nil`'s shape needs K: the FACT holds, only the
  -- pattern match fails, and the code route is what shows the difference.
  acode-uniq
    : (xs ys zs : List Ob)
    → (c d : ACode {Ob = Ob} xs ys zs)
    → c ≡ d
  acode-uniq [] ys zs c d = uipˡ c d
  acode-uniq (x ∷ xs) ys [] () ()
  acode-uniq (x ∷ xs) ys (z ∷ zs) (p , c) (p′ , d) =
    cong₂ _,_ (uipᵒ p p′) (acode-uniq xs ys zs c d)

  append-uniq
    : ∀ {xs ys zs}
    → (p q : Append Ob xs ys zs)
    → p ≡ q
  append-uniq {xs} {ys} {zs} p q =
    trans
      (sym (adecode-aencode p))
      (trans
        (cong (adecode xs ys zs) (acode-uniq xs ys zs (aencode p) (aencode q)))
        (adecode-aencode q))

-- ════════════════════════════════════════════════════════════════════════════
-- EQUALITY, LAYER 2: THE DECISIONS. Only here is decidability of the colours
-- actually spent, and only because a decision is genuinely COMPUTED — the
-- remainder of a match and the profiles of a node have to be compared. Hedberg
-- supplies layer 1's h-level conditions from it.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} (_≟ᵒ_ : DecidableEquality Ob) where

  private
    -- the two h-level conditions layer 1 asks for, supplied constructively
    _≟ˡ_ : DecidableEquality (List Ob)
    _≟ˡ_ = list-dec _≟ᵒ_

    uipᵒ : UIP Ob
    uipᵒ = Decidable⇒UIP.≡-irrelevant _≟ᵒ_

    uipˡ : UIP (List Ob)
    uipˡ = Decidable⇒UIP.≡-irrelevant _≟ˡ_

    uipᵠ : UIP (Quad {Ob = Ob})
    uipᵠ =
      Decidable⇒UIP.≡-irrelevant
        (pair-dec _≟ˡ_ (pair-dec _≟ˡ_ (pair-dec _≟ˡ_ _≟ˡ_)))

  -- the match cons is injective in both components, via the pair view
  ∷-inj
    : ∀ {x xs ys zs} {i j : Insert Ob x ys zs} {m n : Match Ob xs ys}
    → (i ∷ m) ≡ (j ∷ n)
    → (i ≡ j) × (m ≡ n)
  ∷-inj eq =
    let e = ,-injʳ-uip uipˡ (just-injective (cong mview eq))
    in ,-injectiveˡ e , ,-injectiveʳ e

  -- and the cap likewise, through its own view and one projection more
  cap-inj
    : ∀ {x y xs xs′ ys} {i j : Insert Ob y xs xs′} {m n : Match Ob xs ys}
    → cap {x = x} i m ≡ cap {x = x} j n
    → (i ≡ j) × (m ≡ n)
  cap-inj eq =
    let e = ,-injʳ-uip uipˡ (,-injʳ-uip uipᵒ (just-injective (cong cview eq)))
    in ,-injectiveˡ e , ,-injectiveʳ e

  match?
    : ∀ {xs zs : List Ob}
    → (m n : Match Ob xs zs)
    → Dec (m ≡ n)
  match? [] [] = yes refl
  match? (_∷_ {ys} i m) (_∷_ {ys = ys′} j n) with ys ≟ˡ ys′
  ... | no ¬p = no λ eq → ¬p (cong (λ z → maybe′ proj₁ ys (mview z)) eq)
  ... | yes refl with insert? uipᵒ uipˡ i j | match? m n
  ...   | no ¬p | _ = no λ eq → ¬p (proj₁ (∷-inj eq))
  ...   | _ | no ¬p = no λ eq → ¬p (proj₂ (∷-inj eq))
  ...   | yes refl | yes refl = yes refl
  match? (_ ∷ _) (cap _ _) = no λ ()
  match? (cap _ _) (_ ∷ _) = no λ ()
  match? (cap {y = y} {xs = xs} i m) (cap {y = y′} {xs = xs″} j n)
    with y ≟ᵒ y′ | xs ≟ˡ xs″
  ... | no ¬p | _ =
    no λ eq → ¬p (cong (λ z → maybe′ proj₁ y (cview z)) eq)
  ... | _ | no ¬p =
    no λ eq → ¬p (cong (λ z → maybe′ (λ v → proj₁ (proj₂ v)) xs (cview z)) eq)
  ... | yes refl | yes refl with insert? uipᵒ uipˡ i j | match? m n
  ...   | no ¬p | _ = no λ eq → ¬p (proj₁ (cap-inj eq))
  ...   | _ | no ¬p = no λ eq → ¬p (proj₂ (cap-inj eq))
  ...   | yes refl | yes refl = yes refl

  -- and the node is injective in its recursive argument, once its profiles and
  -- its two witnesses have been identified
  node-inj
    : ∀ {Γ Δ A B Γ′ Δ′}
    → {p : Append Ob B Γ Γ′} {q : Append Ob A Δ Δ′}
    → {S T : Shape Ob Γ′ Δ′}
    → node A B p q S ≡ node A B p q T
    → S ≡ T
  node-inj eq =
    ,-injectiveʳ (,-injectiveʳ (,-injʳ-uip uipᵠ (just-injective (cong bundle eq))))

  -- THE OBJECT-LEVEL RESULT: decidable propositional equality of cell shapes.
  shape?
    : ∀ {Γ Δ : List Ob}
    → (S T : Shape Ob Γ Δ)
    → Dec (S ≡ T)
  shape? (wires m) (wires n) with match? m n
  ... | yes refl = yes refl
  ... | no ¬p = no λ { refl → ¬p refl }
  shape? (wires m) (node A B p q T) = no λ ()
  shape? (node A B p q S) (wires n) = no λ ()
  shape? (node A B p q S) (node A′ B′ p′ q′ T) with A ≟ˡ A′ | B ≟ˡ B′
  ... | no ¬e | _ =
    no λ eq → ¬e (,-injectiveˡ (just-injective (cong profs eq)))
  ... | _ | no ¬e =
    no λ eq → ¬e (,-injectiveʳ (just-injective (cong profs eq)))
  ... | yes refl | yes refl
    with append-fun p p′ | append-fun q q′
  ...   | refl | refl
    with append-uniq uipᵒ uipˡ p p′ | append-uniq uipᵒ uipˡ q q′
  ...     | refl | refl with shape? S T
  ...       | yes refl = yes refl
  ...       | no ¬e = no λ eq → ¬e (node-inj eq)

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLES. Uncoloured, so the colour set is the unit type: this is the
-- concrete witness at which the parameterized module above is instantiated,
-- which is what stops its predicates from being statements about an empty
-- class (`docs/workflow/agda.md` §*The done-rule*).
-- ════════════════════════════════════════════════════════════════════════════

-- one wire, and two wires
𝟙 : List ⊤
𝟙 = tt ∷ []

𝟚 : List ⊤
𝟚 = tt ∷ tt ∷ []

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 1: THE EMPTY SHAPE, and the EXCEPTIONAL EDGE. Both are vertex-free.
-- The empty shape is wheel-free and acyclic but NOT connected — `β₀ = 0`, not
-- `1` — which is why `Connected` carries a root: without the existence half,
-- it would satisfy every predicate vacuously.
-- ════════════════════════════════════════════════════════════════════════════

empty : Shape ⊤ [] []
empty = wires []

-- No vertices, so ranking and matching are vacuous.
empty-ranked : Ranked empty
Ranked.rank empty-ranked ()
Ranked.climbs empty-ranked ()

empty-matched : Matched empty
Matched.unlooped empty-matched ()
Matched.unbranched empty-matched ()

empty-wheel-free : WheelFree empty
empty-wheel-free = ranked⇒wheel-free empty-ranked

empty-acyclic : Acyclic empty
empty-acyclic = matched⇒acyclic empty-matched

-- The refutation: connectivity demands a vertex and there is none.
empty-unconnected : ¬ Connected empty
empty-unconnected c with Connected.root c
... | ()

-- The identity arity at one wire: no vertex, one leg-to-leg edge. This is the
-- unit the linear kit predicted would have to be derived here, and it is.
edge : Shape ⊤ 𝟙 𝟙
edge = idn 𝟙

-- Its one edge is a leg at both ends, so it is no arc and carries no topology.
edge-legs
  : origin edge here ≡ inj₂ here
edge-legs = refl

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 2: THE ARITY-ZERO COROLLA. One vertex, no ports at all — a
-- legitimate cell shape that is also an isolated vertex. It is what makes the
-- no-isolated-vertices hypothesis of `Gandr.Shape.Decidable` necessary rather
-- than decorative.
-- ════════════════════════════════════════════════════════════════════════════

point : Shape ⊤ [] []
point = node [] [] nil nil (wires [])

-- No edges, so no arcs, so both certificates are discharged by absurdity.
point-ranked : Ranked point
Ranked.rank point-ranked _ = 0
Ranked.climbs point-ranked ()

point-matched : Matched point
Matched.unlooped point-matched ()
Matched.unbranched point-matched ()

point-connected : Connected point
Connected.root point-connected = here
Connected.span point-connected here = stop
Connected.span point-connected (there ())

point-simply : SimplyConn point
SimplyConn.connected point-simply = point-connected
SimplyConn.acyclic point-simply = matched⇒acyclic point-matched

point-cell : Cell [] []
Cell.shape point-cell = point
Cell.simply point-cell = point-simply

-- It is the arity-zero member of the corolla family, on the nose: with both
-- profiles empty the two `Append` witnesses and the block swap all collapse to
-- their empty clauses. So the hand-written example and the generic family
-- agree, and neither is an independent definition of the other.
point≡corolla : point ≡ corolla [] []
point≡corolla = refl

-- The generic family at a nontrivial profile — `C(2;1)`, the shape `chain`'s
-- first vertex has — checked by computation rather than by typing.
corolla-2-1 : Shape ⊤ 𝟚 𝟙
corolla-2-1 = corolla 𝟚 𝟙

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 3: A TWO-VERTEX COMPOSITE. `C(2;1)` feeding `C(1;1)`. Reading the
-- innermost pools: the sources are `out₁ ++ out₀ ++ Γ` and the sinks are
-- `in₁ ++ in₀ ++ Δ`, so source `0` is vertex `1`'s output, source `1` is
-- vertex `0`'s output, and sources `2` and `3` are the two input legs.
-- ════════════════════════════════════════════════════════════════════════════

chain : Shape ⊤ 𝟚 𝟙
chain =
  node 𝟚 𝟙 (cons nil) (cons (cons nil))
    (node 𝟙 𝟙 (cons nil) (cons nil)
      (wires (tail (tail (tail head)) ∷ head ∷ head ∷ head ∷ [])))

-- the two vertices, named
c₀ c₁ : Vtx chain
c₀ = here
c₁ = there here

-- The internal edge is source `1`: vertex `0`'s output feeding vertex `1`.
chain-internal
  : origin chain (there here) ≡ inj₁ c₀
chain-internal = refl

chain-ranked : Ranked chain
Ranked.rank chain-ranked here = 0
Ranked.rank chain-ranked (there here) = 1
Ranked.rank chain-ranked (there (there ()))
Ranked.climbs chain-ranked here u v ()
Ranked.climbs chain-ranked (there here) _ _ (arc refl refl) = ℕ.n<1+n 0
Ranked.climbs chain-ranked (there (there here)) u v (arc () _)
Ranked.climbs chain-ranked (there (there (there here))) u v (arc () _)

chain-wheel-free : WheelFree chain
chain-wheel-free = ranked⇒wheel-free chain-ranked

-- The only linkable edge is the internal one: the other three each dangle at
-- one end. This single fact discharges both matching conditions.
chain-link
  : ∀ {e u v}
  → Link chain e u v
  → e ≡ there here
chain-link {e = here} (along (arc _ ()))
chain-link {e = here} (against (arc _ ()))
chain-link {e = there here} _ = refl
chain-link {e = there (there here)} (along (arc () _))
chain-link {e = there (there here)} (against (arc () _))
chain-link {e = there (there (there here))} (along (arc () _))
chain-link {e = there (there (there here))} (against (arc () _))

-- The internal edge runs between the two DISTINCT vertices, so it is no loop.
chain-unlooped : (e : Edg chain) (v : Vtx chain) → ¬ Link chain e v v
chain-unlooped here v (along (arc _ ()))
chain-unlooped here v (against (arc _ ()))
chain-unlooped (there here) here (along (arc refl ()))
chain-unlooped (there here) here (against (arc refl ()))
chain-unlooped (there here) (there here) (along (arc () _))
chain-unlooped (there here) (there here) (against (arc () _))
chain-unlooped (there here) (there (there ())) _
chain-unlooped (there (there here)) v (along (arc () _))
chain-unlooped (there (there here)) v (against (arc () _))
chain-unlooped (there (there (there here))) v (along (arc () _))
chain-unlooped (there (there (there here))) v (against (arc () _))

chain-matched : Matched chain
Matched.unlooped chain-matched = chain-unlooped
Matched.unbranched chain-matched e f u v w l l′ =
  trans (chain-link l) (sym (chain-link l′))

chain-connected : Connected chain
Connected.root chain-connected = c₀
Connected.span chain-connected here = stop
Connected.span chain-connected (there here) =
  onward stop (adj (there here) (along (arc refl refl)))
Connected.span chain-connected (there (there ()))

chain-simply : SimplyConn chain
SimplyConn.connected chain-simply = chain-connected
SimplyConn.acyclic chain-simply = matched⇒acyclic chain-matched

chain-cell : Cell 𝟚 𝟙
Cell.shape chain-cell = chain
Cell.simply chain-cell = chain-simply

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 4: THE DIAMOND — the separation. Vertex `0` fans out to `1` and `2`,
-- which reconverge on `3`. It is wheel-free and connected, and it carries a
-- reduced undirected closed walk, so it is NOT simply connected. This is what
-- makes `SimplyConn` strictly stronger than `WheelFree` (HRY Rmk 2.36), and it
-- is the shape Thm 5.9's finiteness rules out — reconvergence, not branching.
--
-- The sources are `out₃ ++ out₂ ++ out₁ ++ out₀`, so edge `0` runs `2 → 3`,
-- edge `1` runs `1 → 3`, edge `2` runs `0 → 1` and edge `3` runs `0 → 2`.
-- ════════════════════════════════════════════════════════════════════════════

diamond : Shape ⊤ [] []
diamond =
  node [] 𝟚 (cons (cons nil)) nil
    (node 𝟙 𝟙 (cons nil) (cons nil)
      (node 𝟙 𝟙 (cons nil) (cons nil)
        (node 𝟚 [] nil (cons (cons nil))
          (wires (tail head ∷ head ∷ tail head ∷ head ∷ [])))))

-- the four vertices, named
d₀ d₁ d₂ d₃ : Vtx diamond
d₀ = here
d₁ = there here
d₂ = there (there here)
d₃ = there (there (there here))

-- the four arcs, read off the derived incidence rather than asserted
diamond-2→3 : Arc diamond here d₂ d₃
diamond-2→3 = arc refl refl

diamond-1→3 : Arc diamond (there here) d₁ d₃
diamond-1→3 = arc refl refl

diamond-0→1 : Arc diamond (there (there here)) d₀ d₁
diamond-0→1 = arc refl refl

diamond-0→2 : Arc diamond (there (there (there here))) d₀ d₂
diamond-0→2 = arc refl refl

-- Heights `0 < 1 < 2`, with the two middle vertices at the same height.
diamond-ranked : Ranked diamond
Ranked.rank diamond-ranked here = 0
Ranked.rank diamond-ranked (there here) = 1
Ranked.rank diamond-ranked (there (there here)) = 1
Ranked.rank diamond-ranked (there (there (there here))) = 2
Ranked.rank diamond-ranked (there (there (there (there ()))))
Ranked.climbs diamond-ranked here _ _ (arc refl refl) = ℕ.n<1+n 1
Ranked.climbs diamond-ranked (there here) _ _ (arc refl refl) = ℕ.n<1+n 1
Ranked.climbs diamond-ranked (there (there here)) _ _ (arc refl refl) = ℕ.n<1+n 0
Ranked.climbs diamond-ranked (there (there (there here))) _ _ (arc refl refl) =
  ℕ.n<1+n 0

diamond-wheel-free : WheelFree diamond
diamond-wheel-free = ranked⇒wheel-free diamond-ranked

diamond-connected : Connected diamond
Connected.root diamond-connected = d₀
Connected.span diamond-connected here = stop
Connected.span diamond-connected (there here) =
  onward stop (adj (there (there here)) (along diamond-0→1))
Connected.span diamond-connected (there (there here)) =
  onward stop (adj (there (there (there here))) (along diamond-0→2))
Connected.span diamond-connected (there (there (there here))) =
  onward
    (onward stop (adj (there (there here)) (along diamond-0→1)))
    (adj (there here) (along diamond-1→3))
Connected.span diamond-connected (there (there (there (there ()))))

-- The cycle, read out: `0` along edge `2` to `1`, along edge `1` to `3`,
-- against edge `0` to `2`, against edge `3` back to `0`. Every step uses a
-- different edge from the one before it, so the walk is reduced.
diamond-cycle : Walk diamond d₀ d₀ (just (there (there (there here))))
diamond-cycle =
  hop (there (there (there here)))
    (hop here
      (hop (there here)
        (hop (there (there here)) stay (along diamond-0→1) opening)
        (along diamond-1→3)
        (apart (λ ())))
      (against diamond-2→3)
      (apart (λ ())))
    (against diamond-0→2)
    (apart (λ ()))

diamond-cyclic : ¬ Acyclic diamond
diamond-cyclic ac = ac diamond-cycle

-- The separation: wheel-free (above) but not simply connected. Both halves of
-- HRY Rmk 2.36 are therefore exhibited, not asserted — the implication by
-- `simply-conn⇒wheel-free`, its failure in the other direction by this.
diamond-not-simply : ¬ SimplyConn diamond
diamond-not-simply sc = SimplyConn.acyclic sc diamond-cycle

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 5: THE WHEEL. One vertex whose single output is wired straight back
-- into its own input. This is what `WheelFree` forbids, and exhibiting it is
-- what stops that predicate from being vacuously true — `ranked⇒wheel-free`
-- would prove nothing if no shape could carry a closed directed walk. It is
-- also the self-loop `dir⇒walk` had to be written around.
--
-- Its exclusion is C5's confinement of feedback to the term face, and it is
-- refuted here through the content lemma rather than directly: a wheel refutes
-- wheel-freeness, so by `simply-conn⇒wheel-free` it refutes simple
-- connectivity, which is the lemma used forwards.
-- ════════════════════════════════════════════════════════════════════════════

wheel : Shape ⊤ [] []
wheel = node 𝟙 𝟙 (cons nil) (cons nil) (wires (head ∷ []))

-- the loop, read off the derived incidence: one edge, both ends at the vertex
wheel-loop : Arc wheel here here here
wheel-loop = arc refl refl

-- The closed directed walk of length one.
wheel-turn : Dir wheel here here (just here)
wheel-turn = next here idle wheel-loop

wheel-wheeled : ¬ WheelFree wheel
wheel-wheeled wf = wf wheel-turn

-- The content lemma, used forwards.
wheel-not-simply : ¬ SimplyConn wheel
wheel-not-simply sc = wheel-wheeled (simply-conn⇒wheel-free sc)

-- ════════════════════════════════════════════════════════════════════════════
-- THE DECISION, AT A CONCRETE COLOUR SET. The three equality layers above are
-- parameterized modules, and Agda type-checks a parameterized module body
-- whether or not its parameters can ever be supplied. They are therefore
-- discharged here at the unit type — the colour set the worked examples use —
-- so that none of them is green and vacuous.
--
-- The two checks below are computational rather than merely well-typed: they
-- run the procedure and pin its verdict, so a decision that type-checked while
-- computing the wrong answer would fail here.
-- ════════════════════════════════════════════════════════════════════════════

-- the colours of the worked examples have decidable equality
_≟⊤_ : DecidableEquality ⊤
tt ≟⊤ tt = yes refl

-- hence so do the shapes over them
infix 4 _≟ˢ_
_≟ˢ_ : ∀ {Γ Δ : List ⊤} (S T : Shape ⊤ Γ Δ) → Dec (S ≡ T)
_≟ˢ_ = shape? _≟⊤_

-- The empty shape and the arity-zero corolla share an interface and are
-- distinct — the vertex is invisible to the interface and visible here.
empty≢point : ¬ (empty ≡ point)
empty≢point ()

-- and the procedure says so, by computation
decides-apart : does (empty ≟ˢ point) ≡ false
decides-apart = refl

-- while the diamond — four vertices, four edges, a nontrivial matching — is
-- recognised as itself, so the negative answer is not the only one in range
decides-same : does (diamond ≟ˢ diamond) ≡ true
decides-same = refl

-- The generic corolla is recognised as itself, which is what checks that its
-- block swap COMPUTES at a nontrivial profile rather than merely typing.
decides-corolla : does (corolla-2-1 ≟ˢ corolla-2-1) ≡ true
decides-corolla = refl

-- and it is not the chain, which spans the same interface with two vertices —
-- the interface does not determine the shape, and the decision sees that
decides-corolla-apart : does (corolla-2-1 ≟ˢ chain) ≡ false
decides-corolla-apart = refl
