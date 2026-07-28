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
-- Read `Shape Γ Δ` as a wiring problem. The ENDS are of two kinds: the source
-- ends are the input legs together with the out-ports of the vertices, and the
-- sink ends are the in-ports of the vertices together with the output legs. A
-- graph is a colour-preserving PAIRING of those ends — nothing more.
--
--   * `node A B` declares a vertex with in-profile `A` and out-profile `B`.
--     The REST of the graph sees `B` as extra sources and `A` as extra sinks.
--   * `wires m` closes the graph off with no further vertex, pairing up the
--     remaining ends.
--
-- The pairing is DOWNWARD: a source takes a sink, or it takes another source —
-- the cut — and no constructor pairs two sinks. So an edge has two ends, but
-- not one of each kind, and the two facts that follow are the difference
-- between this carrier and the one an earlier revision had:
--
--   * `Match Γ Δ` is inhabited only when `Γ` is at least as long as `Δ`, the
--     difference paid in cuts. The downward category's emptiness for `n > m` is
--     REPRODUCED here rather than imposed.
--   * the nodeless loop is inexpressible, because closing a circle with no
--     vertex needs a cut composed with its opposite, and the opposite — a
--     pairing of two SINKS — has no constructor. Composition cannot manufacture
--     one either: it fuses two through-strands into a cut, and that is the only
--     new pairing it makes.
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
-- matching is the chosen wiring; the two ends of an edge are then computed by
-- tracing outward through the node chain, splitting at each profile.
--
-- There are TWO listings and they are on the same footing. `verts` lists the
-- vertices, one entry per `node`; `edges` lists the edges, one entry per pair
-- the wiring makes — a `flow` for a source that took a sink, a `cut` for a
-- source that took another source.
--
-- **The edge listing is not the source pool, and conflating them was a real
-- defect.** Before the cut existed the two coincided, because every edge had
-- exactly one source end, so an earlier revision took `Ix (pool S)` for the
-- edge set. A cut has TWO source ends and no sink end, so under that reading
-- one wire occupied two positions and the derived incidence gave it two
-- antiparallel arcs: the edge count was wrong, and `WheelFree` and `Acyclic`
-- reported cycles in graphs that have none. Listing the edges directly fixes
-- both at once, and it is what lets reducedness — which is keyed on edge
-- identity — mean what it says.
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
-- one `cong` reaches it. `split`, `ends` and `route` are all in that form, and
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
-- Legs never link: `Attach` demands that both ends of an edge be attached, so
-- an edge with a free end cannot sit on an undirected cycle. That is the
-- correct reading — the interface of a cell is not part of its topology.
--
-- **Both readings hold on EVERY shape, cuts included**, and that is a property
-- of the edge listing rather than a restriction: a cut is one edge, so walking
-- out along it and back is a repeat, which reducedness rejects. `gluing` below
-- is the worked case — two vertices joined by one contracted wire, connected
-- and acyclic and a cell, exactly as the graph says.
--
-- ── DIRECTION IS A SEPARATE QUESTION, AND IT IS THE PALETTE'S ───────────────
-- `Attach` says where an edge's ends are. `Arc` says which way it RUNS, and
-- those come apart at the cut: a flow-through wire runs from its source end to
-- its sink end, and the wiring alone settles that, but a cut joins two SOURCE
-- ends and nothing about the graph says which of them produces.
--
-- What says it is the colour. A `Palette` is an involution on colours — the
-- dual colour is the same wire seen from its other end — together with an
-- ORIENTATION assigning each colour a pole, with dual colours opposite. Then a
-- legitimate cut joins `c` to its dual, its two ends have opposite poles, and
-- `cut-oriented` reads its direction off them.
--
-- So the directed notions — `Arc`, `Dir`, `WheelFree`, `Ranked` — take the
-- polarity and the undirected ones do not, and that asymmetry is the point:
-- everything that does not need an orientation is stated without one and holds
-- on every shape. The involution is therefore not merely a predicate saying
-- which cuts are legitimate; it is what gives a cut's incidence a direction at
-- all, and `mono-unoriented` shows the distinction has teeth — one self-dual
-- colour admits no palette, so at that colour set a cut genuinely runs nowhere.
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
  using (⊥)
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
  renaming (map to pmap)
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
  using (subst)
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

-- ════════════════════════════════════════════════════════════════════════════
-- POLARITY. A colour either produces or consumes, and that is the datum — the
-- ORIENTATION of the palette, in the source's vocabulary — which decides which
-- way a cut runs. It is not needed to say a shape exists, nor to say what its
-- edges are, nor to traverse one undirected; it is needed exactly to give a
-- cut a direction, and it appears exactly there.
--
-- A flow-through wire needs none of this: the wiring already runs it from its
-- source end to its sink end. So the polarity is the price of the CUT, and of
-- nothing else.
-- ════════════════════════════════════════════════════════════════════════════

data Pole : Set where
  -- the colour of something that produces
  produces : Pole
  -- and of something that consumes
  consumes : Pole

-- the other one
flip : Pole → Pole
flip produces = consumes
flip consumes = produces

-- A PALETTE: an involution on the colours — the dual colour is the same wire
-- seen from its other end — together with an ORIENTATION of it, which says
-- which end of a dual pair produces.
--
-- The involution alone does not orient anything: `c` and `dual c` are
-- interchangeable until something breaks the symmetry, and `pole` is what
-- breaks it. That is why both fields are here and why `pole-dual` is the law
-- that matters: it is what makes a palette ORIENTED rather than merely
-- involutive, and `mono-unoriented` at the bottom of this module shows the
-- distinction is real by exhibiting a colour set that admits no palette at all.
record Palette {ℓ} (Ob : Set ℓ) : Set ℓ where
  field
    -- the same wire, seen from the other end
    dual : Ob → Ob
    -- seeing it from the other end twice is seeing it
    dual² : (c : Ob) → dual (dual c) ≡ c
    -- which end produces
    pole : Ob → Pole
    -- and the other end therefore consumes
    pole-dual : (c : Ob) → pole (dual c) ≡ flip (pole c)

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

  -- AN EDGE, listed by what the wiring pairs. This is the second listing, on
  -- the same footing as the vertex listing, and it is what makes the edge set
  -- an object the carrier can name rather than a coincidence of the source
  -- pool: a flow-through wire has one source end and one sink end, and a cut
  -- has two source ends and none. Before the cap those coincided, which is why
  -- an earlier revision could take the source pool for the edge set.
  data Wire : Set ℓ where
    -- a source taken to a sink, of its own colour
    flow
      : Ob
      → Wire
    -- two sources taken to each other. The colours are unconstrained here for
    -- the reason `node` leaves wheels expressible: which cuts are legitimate is
    -- a question about polarity and is answered by a refutable predicate, not
    -- by what can be written
    cut
      : Ob
      → Ob
      → Wire

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

  -- The innermost pools — the shape's ENDS. Every source end is an input leg
  -- or a vertex out-port; every sink end is a vertex in-port or an output leg.
  -- These are half-edges, not edges: the wiring is what pairs them, and a cut
  -- pairs two of the first kind.
  pool
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → List Ob
  pool (wires {Γ} m) = Γ
  pool (node A B p q S) = pool S

  copool
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → List Ob
  copool (wires {Δ} m) = Δ
  copool (node A B p q S) = copool S

  -- and the wiring itself, which is the only thing at the bottom of the chain
  wiring
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Match Ob (pool S) (copool S)
  wiring (wires m) = m
  wiring (node A B p q S) = wiring S

  -- THE EDGE LISTING, which is the shape's second primary content. One entry
  -- per pair the wiring makes: a `flow` per source that took a sink, a `cut`
  -- per source that took another source. A cut is ONE entry, which is the
  -- whole point — it has two ends, and an edge is not an end.
  match-edges
    : ∀ {Γ Δ}
    → Match Ob Γ Δ
    → List (Wire Ob)
  match-edges [] = []
  match-edges (_∷_ {x} i m) = flow x ∷ match-edges m
  match-edges (cap {x} {y} j m) = cut x y ∷ match-edges m

  edges
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → List (Wire Ob)
  edges S = match-edges (wiring S)

  -- The edges of a shape, on the same footing as its vertices.
  Edg
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → Set ℓ
  Edg S = Ix (edges S)

  -- Splitting a position along an `Append` witness. Structural recursion on
  -- the witness, so no cardinality arithmetic and no re-indexing plumbing.
  --
  -- The recursive clause reindexes through `smap` rather than through a `with`
  -- on the recursive call, and that is deliberate: a `with` compiles to an
  -- auxiliary the caller cannot name, so `split (cons p) (there i)` is stuck on
  -- a term no lemma can rewrite. Written compositionally it reduces to an
  -- application whose argument IS the recursive call, and every fact below
  -- about `split` is then one `cong` away. `ends` and `route` are written the
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

  -- and the same question as a VIEW rather than a sum. `Part` is `Append`'s;
  -- this is `Insert`'s, and it is what a recursion over the EXTENDED listing
  -- needs: eliminating it refines the position into the one the insertion took
  -- or one of the ones it did not, instead of handing back an equation the
  -- consumer then has to transport along.
  data Slot {x ys zs} (i : Insert Ob x ys zs) : Ix zs → Set ℓ where
    -- the position the insertion took
    taken
      : Slot i (slot i)
    -- and one of the positions it did not
    spare
      : (j : Ix ys)
      → Slot i (past i j)

  islot
    : ∀ {x ys zs}
    → (i : Insert Ob x ys zs)
    → (e : Ix zs)
    → Slot i e
  islot head here = taken
  islot head (there j) = spare j
  islot (tail i) here = spare here
  islot (tail i) (there e) with islot i e
  ... | taken = taken
  ... | spare j = spare (there j)

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

  -- A BOUNDARY POINT of a shape: an input leg or an output leg. Polarity is
  -- carried data here rather than a positional convention, because a capped
  -- edge ends at a SOURCE and the incidence has to be able to say so.
  Leg
    : List Ob
    → List Ob
    → Set ℓ
  Leg Γ Δ = Ix Γ ⊎ Ix Δ

  -- THE TWO ENDS OF A WIRE, as positions in the innermost pools. A flow wire's
  -- ends are its source and its sink, in that order; a cut's are its two
  -- sources, the earlier one first. Both ends have the same type, which is the
  -- correction the cap forced: an end is an end, and which of them the wire
  -- runs FROM is a separate question that `Forth`/`Back` below answer.
  ends
    : ∀ {Γ Δ}
    → (m : Match Ob Γ Δ)
    → Ix (match-edges m)
    → Leg Γ Δ × Leg Γ Δ
  ends (i ∷ m) here = inj₁ here , inj₂ (slot i)
  ends (i ∷ m) (there e) =
    pmap (smap there (past i)) (smap there (past i)) (ends m e)
  ends (cap j m) here = inj₁ here , inj₁ (there (slot j))
  ends (cap j m) (there e) =
    pmap
      (smap (λ w → there (past j w)) id)
      (smap (λ w → there (past j w)) id)
      (ends m e)

  -- Routing an end outward through the node chain: it is at a vertex's port,
  -- or it is a leg of the whole shape. This is the recursion the incidence
  -- used to carry inline, named once now that both ends need it.
  route
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Leg (pool S) (copool S)
    → Vtx S ⊎ Leg Γ Δ
  route (wires m) l = inj₂ l
  route (node A B p q S) l =
    case⊎
      (λ v → inj₁ (there v))
      (case⊎
        (λ i → smap (λ _ → here) inj₁ (split p i))
        (λ j → smap (λ _ → here) inj₂ (split q j)))
      (route S l)

  -- and the two ends of an edge of a SHAPE, which is the incidence
  end₀
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Edg S
    → Vtx S ⊎ Leg Γ Δ
  end₀ S e = route S (proj₁ (ends (wiring S) e))

  end₁
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Edg S
    → Vtx S ⊎ Leg Γ Δ
  end₁ S e = route S (proj₂ (ends (wiring S) e))

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE FLOW-THROUGH BRIDGE. On a wiring with no cut the wires and the sources
  -- are one listing, position for position, because every source takes exactly
  -- one sink. That identification is what re-points every fact proved before
  -- the cap — `swap-follow` above, and the corolla's incidence below — at the
  -- WIRE rather than at the source end, with its proof term untouched.
  -- ══════════════════════════════════════════════════════════════════════════

  wire⁺
    : ∀ {xs zs} {m : Match Ob xs zs}
    → CapFree m
    → Ix xs
    → Ix (match-edges m)
  wire⁺ (i ∷ c) here = here
  wire⁺ (i ∷ c) (there e) = there (wire⁺ c e)

  src⁺
    : ∀ {xs zs} {m : Match Ob xs zs}
    → CapFree m
    → Ix (match-edges m)
    → Ix xs
  src⁺ (i ∷ c) here = here
  src⁺ (i ∷ c) (there e) = there (src⁺ c e)

  wire-src
    : ∀ {xs zs} {m : Match Ob xs zs}
    → (c : CapFree m)
    → (e : Ix (match-edges m))
    → wire⁺ c (src⁺ c e) ≡ e
  wire-src (i ∷ c) here = refl
  wire-src (i ∷ c) (there e) = cong there (wire-src c e)

  -- and the ends of such a wire are its source and the sink it follows to
  ends-capfree
    : ∀ {xs zs} {m : Match Ob xs zs}
    → (c : CapFree m)
    → (e : Ix xs)
    → ends m (wire⁺ c e) ≡ (inj₁ e , inj₂ (follow⁺ c e))
  ends-capfree (i ∷ c) here = refl
  ends-capfree (i ∷ c) (there e) =
    cong
      (pmap (smap there (past i)) (smap there (past i)))
      (ends-capfree c e)

  -- ══════════════════════════════════════════════════════════════════════════
  -- ATTACHMENT, THEN DIRECTION. These are two questions and the cap is what
  -- separated them. `Attach` says where a wire's ends are, in the listing's own
  -- order, and claims nothing about which way it runs; `Arc` adds the
  -- direction, and for a cut the direction is a fact about COLOURS.
  -- ══════════════════════════════════════════════════════════════════════════

  record Attach {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) (u v : Vtx S) : Set ℓ where
    constructor attach
    field
      -- `e` meets `u` at its first end
      from : end₀ S e ≡ inj₁ u
      -- and `v` at its second
      into : end₁ S e ≡ inj₁ v

  -- `e` runs along the listing, from its first end to its second.
  data Forth (pol : Ob → Pole) : Wire Ob → Set ℓ where
    -- every flow-through wire, with no appeal to colours: the wiring itself
    -- orients it, source end to sink end
    flowing
      : ∀ {x}
      → Forth pol (flow x)
    -- and a cut whose EARLIER end is the producer
    earlier
      : ∀ {x y}
      → pol x ≡ produces
      → pol y ≡ consumes
      → Forth pol (cut x y)

  -- and against it, from its second end to its first. Only a cut can: a flow
  -- wire's second end is a sink, which consumes by construction.
  data Back (pol : Ob → Pole) : Wire Ob → Set ℓ where
    later
      : ∀ {x y}
      → pol x ≡ consumes
      → pol y ≡ produces
      → Back pol (cut x y)

  -- A LEGITIMATE CUT IS ORIENTED — which is what the involution is FOR, and
  -- the reason it is not merely a predicate saying which cuts are allowed. A
  -- cut joining a colour to its dual runs one way or the other, and which way
  -- is read off the poles.
  --
  -- Without this the directed layer would be silent on every cut: a cut has two
  -- source ends, so the wiring cannot orient it, and nothing else in the
  -- carrier could. With it, the wire's direction is a computed consequence of
  -- the colours the shape already carries.
  -- The case analysis, taking the pole it splits on as data plus the equation
  -- naming it, so the two branches can be read back into the constructors. A
  -- `with` would abstract the pole out of the goal, where it does not appear,
  -- and leave the equations unreachable.
  cut-oriented-at
    : (P : Palette Ob)
    → (x : Ob)
    → (p : Pole)
    → Palette.pole P x ≡ p
    → Forth (Palette.pole P) (cut x (Palette.dual P x))
      ⊎ Back (Palette.pole P) (cut x (Palette.dual P x))
  cut-oriented-at P x produces eq =
    inj₁ (earlier eq (trans (Palette.pole-dual P x) (cong flip eq)))
  cut-oriented-at P x consumes eq =
    inj₂ (later eq (trans (Palette.pole-dual P x) (cong flip eq)))

  cut-oriented
    : (P : Palette Ob)
    → (x : Ob)
    → Forth (Palette.pole P) (cut x (Palette.dual P x))
      ⊎ Back (Palette.pole P) (cut x (Palette.dual P x))
  cut-oriented P x = cut-oriented-at P x (Palette.pole P x) refl

  -- A WIRE RUNS AT MOST ONE WAY, which is what makes a directed walk mean
  -- anything. This is the property the pre-`Wire` incidence silently lacked:
  -- reading a cut as two antiparallel edges let a walk leave along a wire and
  -- come back along the same one.
  forth-not-back
    : ∀ {pol w}
    → Forth pol w
    → Back pol w
    → ⊥
  forth-not-back (earlier p q) (later p′ q′)
    with trans (sym p) p′
  ... | ()

  -- `Arc S e u v`: the wire `e` runs from `u` to `v`, both ends attached.
  data Arc (pol : Ob → Pole) {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) (u v : Vtx S)
    : Set ℓ where
    -- oriented along the listing, so its first end is the tail
    forth
      : Forth pol (lookup (edges S) e)
      → Attach S e u v
      → Arc pol S e u v
    -- oriented against it, so its second end is
    back
      : Back pol (lookup (edges S) e)
      → Attach S e v u
      → Arc pol S e u v

  data Link {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) : Vtx S → Vtx S → Set ℓ where
    -- traverse `e` from its first end to its second
    along
      : ∀ {u v}
      → Attach S e u v
      → Link S e u v
    -- and the other way. NO POLARITY IS NEEDED: an undirected traversal does
    -- not ask which end produces, which is why the undirected layer below is
    -- correct on every shape while the directed one needs a polarity
    against
      : ∀ {u v}
      → Attach S e v u
      → Link S e u v

  -- an arc is in particular a traversal, whichever way it was oriented
  arc⇒link
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {e u v}
    → Arc pol S e u v
    → Link S e u v
  arc⇒link (forth _ a) = along a
  arc⇒link (back _ a) = against a

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

  data Dir (pol : Ob → Pole) {Γ Δ} (S : Shape Ob Γ Δ) (u : Vtx S)
    : Vtx S → Maybe (Edg S) → Set ℓ where
    -- the empty directed walk
    idle
      : Dir pol S u u nothing
    -- extend by one arc; no freshness is imposed, since a directed walk cannot
    -- backtrack except through a self-loop, which `dir⇒walk` handles
    next
      : ∀ {m v b}
      → (e : Edg S)
      → Dir pol S u m b
      → Arc pol S e m v
      → Dir pol S u v (just e)

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
  -- Stated AT A POLARITY, because a cut's direction is a fact about colours and
  -- there is no answer without one. That is not a scope cut hidden in a name:
  -- at a polarity that orients nothing — the monochrome one, where every colour
  -- produces — this says only that the flow-through fragment has no feedback,
  -- and `mono-unoriented` below exhibits exactly that.
  WheelFree
    : ∀ {Γ Δ}
    → (Ob → Pole)
    → Shape Ob Γ Δ
    → Set ℓ
  WheelFree pol S = ∀ {v : Vtx S} {e : Edg S} → ¬ Dir pol S v v (just e)

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

  record Ranked (pol : Ob → Pole) {Γ Δ} (S : Shape Ob Γ Δ) : Set ℓ where
    field
      -- the height of a vertex
      rank : Vtx S → ℕ
      -- every arc goes strictly uphill
      climbs : (e : Edg S) (u v : Vtx S) → Arc pol S e u v → rank u < rank v

  -- A nonempty directed walk strictly increases rank. Recursion peels the last
  -- arc, so the two clauses are "one arc" and "more than one".
  dir-rank
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {u v e}
    → (R : Ranked pol S)
    → Dir pol S u v (just e)
    → Ranked.rank R u < Ranked.rank R v
  dir-rank R (next e idle a) = Ranked.climbs R e _ _ a
  dir-rank R (next e (next f d a′) a) =
    ℕ.<-trans (dir-rank R (next f d a′)) (Ranked.climbs R e _ _ a)

  -- Hence a ranked shape is wheel-free: a closed directed walk would put a
  -- rank strictly below itself.
  ranked⇒wheel-free
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ}
    → Ranked pol S
    → WheelFree pol S
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
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {e v}
    → Acyclic S
    → ¬ Arc pol S e v v
  loop-free ac a = ac (hop _ stay (arc⇒link a) opening)

  -- The last arc of a nonempty directed walk is an arc INTO where the walk
  -- ends. Which of the wire's two ends that is depends on how the wire is
  -- oriented, so the arc is handed back whole rather than projected.
  dir-last
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {u v e}
    → Dir pol S u v (just e)
    → Σ (Vtx S) (λ w → Arc pol S e w v)
  dir-last (next _ _ a) = _ , a

  -- If a directed walk's next arc repeats its last edge then that edge is a
  -- self-loop: the repeated wire must both end and start at the shared vertex.
  -- This is the only obstruction to reading a directed walk as a reduced one.
  --
  -- The mixed cases are where the corrected edge identity earns its keep: they
  -- would have the ONE wire oriented both ways at once, which `forth-not-back`
  -- refutes. Under the earlier reading — a cut presented as two antiparallel
  -- edges — they were not refutable, because they were not false.
  arc-repeat
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {u m v : Vtx S} {e f : Edg S}
    → Dir pol S u m (just f)
    → Arc pol S e m v
    → e ≡ f
    → Arc pol S e m m
  arc-repeat d a refl with dir-last d
  arc-repeat d (forth o a) refl | _ , forth o′ a′ =
    forth o (attach (Attach.from a) (Attach.into a′))
  arc-repeat d (back o a) refl | _ , back o′ a′ =
    back o (attach (Attach.from a′) (Attach.into a))
  arc-repeat d (forth o a) refl | _ , back o′ a′ = ⊥-elim (forth-not-back o o′)
  arc-repeat d (back o a) refl | _ , forth o′ a′ = ⊥-elim (forth-not-back o′ o)

  -- So, given no self-loops, every directed walk IS a reduced undirected walk,
  -- with the same last-edge index.
  dir⇒walk
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ} {u v : Vtx S} {b}
    → ((e : Edg S) (w : Vtx S) → ¬ Arc pol S e w w)
    → Dir pol S u v b
    → Walk S u v b
  dir⇒walk nl idle = stay
  dir⇒walk nl (next e idle a) = hop e stay (arc⇒link a) opening
  dir⇒walk nl (next e (next f d a′) a) =
    hop e
      (dir⇒walk nl (next f d a′))
      (arc⇒link a)
      (apart (λ eq → nl e _ (arc-repeat (next f d a′) a eq)))

  -- Undirected acyclicity implies wheel-freeness, AT EVERY POLARITY. The
  -- quantifier is the content: no orientation of the cuts can produce feedback
  -- in a shape that has no undirected cycle, so the undirected predicate is
  -- the stronger one and it is the one `Cell` carries.
  acyclic⇒wheel-free
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ}
    → Acyclic S
    → WheelFree pol S
  acyclic⇒wheel-free ac d = ac (dir⇒walk (λ _ _ → loop-free ac) d)

  -- HRY Rmk 2.36, easy direction.
  simply-conn⇒wheel-free
    : ∀ {pol Γ Δ} {S : Shape Ob Γ Δ}
    → SimplyConn S
    → WheelFree pol S
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

    -- and therefore free of directed cycles under EVERY polarity, at no extra
    -- cost — which is why the cell carries the undirected predicate and not
    -- this one
    wheel-free : (pol : Ob → Pole) → WheelFree pol shape
    wheel-free pol = simply-conn⇒wheel-free simply

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

  -- Its wiring is flow-through — a corolla has no cut — so its wires are its
  -- sources, and the bridge above carries the block-swap lemma across.
  corolla-capfree
    : ∀ {A B}
    → CapFree (swap-match (append-graph B A) (append-graph A B))
  corolla-capfree {A} {B} =
    swap-match-capfree (append-graph B A) (append-graph A B)

  -- Its out-profile block runs from the vertex to an output leg. This is where
  -- `swap-follow` is spent: the wire's source end is in the `B` block, so its
  -- sink end lands in the `B` block on the other side, which is the leg side.
  corolla-out
    : ∀ {A B}
    → (i : Ix B)
    → end₁ (corolla A B) (wire⁺ (corolla-capfree {A} {B}) (left (append-graph B A) i))
      ≡ inj₂ (inj₂ i)
  corolla-out {A} {B} i =
    trans
      (cong
        (λ z →
          case⊎
            (λ j → smap (λ _ → here) inj₁ (split (append-graph B A) j))
            (λ j → smap (λ _ → here) inj₂ (split (append-graph A B) j))
            (proj₂ z))
        (ends-capfree (corolla-capfree {A} {B}) (left (append-graph B A) i)))
      (cong
        (smap (λ _ → here) inj₂)
        (trans
          (swap-follow (append-graph B A) (append-graph A B) (left (append-graph B A) i))
          (cong swap (split-left (append-graph B A) i))))

  -- and its in-profile block runs from an input leg to the vertex, which needs
  -- only the section law, since a wire's first end is its source
  corolla-in
    : ∀ {A B}
    → (j : Ix A)
    → end₀ (corolla A B) (wire⁺ (corolla-capfree {A} {B}) (right (append-graph B A) j))
      ≡ inj₂ (inj₁ j)
  corolla-in {A} {B} j =
    trans
      (cong
        (λ z →
          case⊎
            (λ w → smap (λ _ → here) inj₁ (split (append-graph B A) w))
            (λ w → smap (λ _ → here) inj₂ (split (append-graph A B) w))
            (proj₁ z))
        (ends-capfree (corolla-capfree {A} {B}) (right (append-graph B A) j)))
      (cong (smap (λ _ → here) inj₁) (split-right (append-graph B A) j))

  -- So NO wire of a corolla has both ends at a vertex — every wire has a leg
  -- end. That single fact discharges every certificate below, and it is why
  -- the corolla is a cell for arbitrary profiles rather than only for the
  -- small ones that can be checked by hand.
  corolla-no-attach-at
    : ∀ {A B} {u v}
    → (s : Ix (B ++ A))
    → ¬ Attach (corolla A B) (wire⁺ (corolla-capfree {A} {B}) s) u v
  corolla-no-attach-at {A} {B} s a with part (append-graph B A) s
  ... | front i = inj₂≢inj₁ (trans (sym (corolla-out i)) (Attach.into a))
  ... | back j = inj₂≢inj₁ (trans (sym (corolla-in j)) (Attach.from a))

  corolla-no-attach
    : ∀ {A B e u v}
    → ¬ Attach (corolla A B) e u v
  corolla-no-attach {A} {B} {e} {u} {v} a =
    corolla-no-attach-at
      {u = u}
      {v = v}
      (src⁺ (corolla-capfree {A} {B}) e)
      (subst
        (λ z → Attach (corolla A B) z u v)
        (sym (wire-src (corolla-capfree {A} {B}) e))
        a)

  corolla-no-link
    : ∀ {A B e u v}
    → ¬ Link (corolla A B) e u v
  corolla-no-link (along a) = corolla-no-attach a
  corolla-no-link (against a) = corolla-no-attach a

  corolla-no-arc
    : ∀ {pol A B e u v}
    → ¬ Arc pol (corolla A B) e u v
  corolla-no-arc a = corolla-no-link (arc⇒link a)

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

-- THE MONOCHROME POLARITY, at which every colour produces. It is the only
-- polarity available on one self-dual colour, and it orients NO cut — which is
-- the honest state of the examples below, all of which are flow-through and so
-- need no orientation at all. `mono-unoriented` is what stops this from being
-- a quiet weakening: at this colour set there is no palette to be had.
mono : ⊤ → Pole
mono _ = produces

-- and the smallest colour set that DOES admit one: the two poles themselves,
-- dual to each other. This is the oriented palette, and `polar` below is the
-- shape whose cut it orients.
duality : Palette Pole
Palette.dual duality = flip
Palette.dual² duality produces = refl
Palette.dual² duality consumes = refl
Palette.pole duality = id
Palette.pole-dual duality produces = refl
Palette.pole-dual duality consumes = refl

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 1: THE EMPTY SHAPE, and the EXCEPTIONAL EDGE. Both are vertex-free.
-- The empty shape is wheel-free and acyclic but NOT connected — `β₀ = 0`, not
-- `1` — which is why `Connected` carries a root: without the existence half,
-- it would satisfy every predicate vacuously.
-- ════════════════════════════════════════════════════════════════════════════

empty : Shape ⊤ [] []
empty = wires []

-- No vertices, so ranking and matching are vacuous.
empty-ranked : Ranked mono empty
Ranked.rank empty-ranked ()
Ranked.climbs empty-ranked ()

empty-matched : Matched empty
Matched.unlooped empty-matched ()
Matched.unbranched empty-matched ()

empty-wheel-free : WheelFree mono empty
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

-- Its one wire is a leg at both ends — an input leg and an output leg — so it
-- is no arc and carries no topology.
edge-legs
  : end₀ edge here ≡ inj₂ (inj₁ here)
edge-legs = refl

edge-legs′
  : end₁ edge here ≡ inj₂ (inj₂ here)
edge-legs′ = refl

-- THE CUT — the boundary `∩`, and the shape this carrier could not write
-- before the cap. Two input legs wired to each other, no vertex: take a
-- producer and a consumer and cut them. Its interface is two sources against
-- NO sink, which is the downward category's `dBD(m,n) = ∅ for n > m` appearing
-- as a fact about terms rather than as an imposed condition.
cutting : Shape ⊤ 𝟚 []
cutting = wires (cap head [])

cutting-no-vertex : verts cutting ≡ []
cutting-no-vertex = refl

-- ONE wire, whose two ends are the two source legs. That it is one and not two
-- is the edge listing doing its job: a cut has two ends and is one edge.
cutting-one-wire : edges cutting ≡ cut tt tt ∷ []
cutting-one-wire = refl

-- and both of its ends are SOURCE legs, which is what `Leg`'s polarity was
-- added for: without it the incidence has nowhere to land, since neither end
-- is a sink and the shape has no sinks at all.
cutting-end₀ : end₀ cutting here ≡ inj₂ (inj₁ here)
cutting-end₀ = refl

cutting-end₁ : end₁ cutting here ≡ inj₂ (inj₁ (there here))
cutting-end₁ = refl

-- and it is no cell, for the empty shape's reason rather than the cap's:
-- connectivity wants a vertex and a wiring has none
cutting-unconnected : ¬ Connected cutting
cutting-unconnected c with Connected.root c
... | ()

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 2: THE ARITY-ZERO COROLLA. One vertex, no ports at all — a
-- legitimate cell shape that is also an isolated vertex. It is what makes the
-- no-isolated-vertices hypothesis of `Gandr.Shape.Decidable` necessary rather
-- than decorative.
-- ════════════════════════════════════════════════════════════════════════════

point : Shape ⊤ [] []
point = node [] [] nil nil (wires [])

-- No edges, so no arcs, so both certificates are discharged by absurdity.
point-ranked : Ranked mono point
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
  : end₀ chain (there here) ≡ inj₁ c₀
chain-internal = refl

chain-ranked : Ranked mono chain
Ranked.rank chain-ranked here = 0
Ranked.rank chain-ranked (there here) = 1
Ranked.rank chain-ranked (there (there ()))
Ranked.climbs chain-ranked here u v (forth flowing (attach _ ()))
Ranked.climbs chain-ranked here u v (back () _)
Ranked.climbs chain-ranked (there here) _ _ (forth flowing (attach refl refl)) =
  ℕ.n<1+n 0
Ranked.climbs chain-ranked (there here) _ _ (back () _)
Ranked.climbs chain-ranked (there (there here)) u v (forth flowing (attach () _))
Ranked.climbs chain-ranked (there (there here)) u v (back () _)
Ranked.climbs
  chain-ranked (there (there (there here))) u v (forth flowing (attach () _))
Ranked.climbs chain-ranked (there (there (there here))) u v (back () _)

chain-wheel-free : WheelFree mono chain
chain-wheel-free = ranked⇒wheel-free chain-ranked

-- The only linkable edge is the internal one: the other three each dangle at
-- one end. This single fact discharges both matching conditions.
chain-link
  : ∀ {e u v}
  → Link chain e u v
  → e ≡ there here
chain-link {e = here} (along (attach _ ()))
chain-link {e = here} (against (attach _ ()))
chain-link {e = there here} _ = refl
chain-link {e = there (there here)} (along (attach () _))
chain-link {e = there (there here)} (against (attach () _))
chain-link {e = there (there (there here))} (along (attach () _))
chain-link {e = there (there (there here))} (against (attach () _))

-- The internal edge runs between the two DISTINCT vertices, so it is no loop.
chain-unlooped : (e : Edg chain) (v : Vtx chain) → ¬ Link chain e v v
chain-unlooped here v (along (attach _ ()))
chain-unlooped here v (against (attach _ ()))
chain-unlooped (there here) here (along (attach refl ()))
chain-unlooped (there here) here (against (attach refl ()))
chain-unlooped (there here) (there here) (along (attach () _))
chain-unlooped (there here) (there here) (against (attach () _))
chain-unlooped (there here) (there (there ())) _
chain-unlooped (there (there here)) v (along (attach () _))
chain-unlooped (there (there here)) v (against (attach () _))
chain-unlooped (there (there (there here))) v (along (attach () _))
chain-unlooped (there (there (there here))) v (against (attach () _))

chain-matched : Matched chain
Matched.unlooped chain-matched = chain-unlooped
Matched.unbranched chain-matched e f u v w l l′ =
  trans (chain-link l) (sym (chain-link l′))

chain-connected : Connected chain
Connected.root chain-connected = c₀
Connected.span chain-connected here = stop
Connected.span chain-connected (there here) =
  onward stop (adj (there here) (along (attach refl refl)))
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
diamond-at-2→3 : Attach diamond here d₂ d₃
diamond-at-2→3 = attach refl refl

diamond-at-1→3 : Attach diamond (there here) d₁ d₃
diamond-at-1→3 = attach refl refl

diamond-at-0→1 : Attach diamond (there (there here)) d₀ d₁
diamond-at-0→1 = attach refl refl

diamond-at-0→2 : Attach diamond (there (there (there here))) d₀ d₂
diamond-at-0→2 = attach refl refl

-- and every one of them is a flow-through wire, so the wiring orients it with
-- no appeal to colours
diamond-2→3 : Arc mono diamond here d₂ d₃
diamond-2→3 = forth flowing diamond-at-2→3

diamond-1→3 : Arc mono diamond (there here) d₁ d₃
diamond-1→3 = forth flowing diamond-at-1→3

diamond-0→1 : Arc mono diamond (there (there here)) d₀ d₁
diamond-0→1 = forth flowing diamond-at-0→1

diamond-0→2 : Arc mono diamond (there (there (there here))) d₀ d₂
diamond-0→2 = forth flowing diamond-at-0→2

-- Heights `0 < 1 < 2`, with the two middle vertices at the same height.
diamond-ranked : Ranked mono diamond
Ranked.rank diamond-ranked here = 0
Ranked.rank diamond-ranked (there here) = 1
Ranked.rank diamond-ranked (there (there here)) = 1
Ranked.rank diamond-ranked (there (there (there here))) = 2
Ranked.rank diamond-ranked (there (there (there (there ()))))
Ranked.climbs diamond-ranked here _ _ (forth flowing (attach refl refl)) =
  ℕ.n<1+n 1
Ranked.climbs diamond-ranked here _ _ (back () _)
Ranked.climbs
  diamond-ranked (there here) _ _ (forth flowing (attach refl refl)) =
  ℕ.n<1+n 1
Ranked.climbs diamond-ranked (there here) _ _ (back () _)
Ranked.climbs
  diamond-ranked (there (there here)) _ _ (forth flowing (attach refl refl)) =
  ℕ.n<1+n 0
Ranked.climbs diamond-ranked (there (there here)) _ _ (back () _)
Ranked.climbs
  diamond-ranked
  (there (there (there here)))
  _
  _
  (forth flowing (attach refl refl)) =
  ℕ.n<1+n 0
Ranked.climbs diamond-ranked (there (there (there here))) _ _ (back () _)

diamond-wheel-free : WheelFree mono diamond
diamond-wheel-free = ranked⇒wheel-free diamond-ranked

diamond-connected : Connected diamond
Connected.root diamond-connected = d₀
Connected.span diamond-connected here = stop
Connected.span diamond-connected (there here) =
  onward stop (adj (there (there here)) (along diamond-at-0→1))
Connected.span diamond-connected (there (there here)) =
  onward stop (adj (there (there (there here))) (along diamond-at-0→2))
Connected.span diamond-connected (there (there (there here))) =
  onward
    (onward stop (adj (there (there here)) (along diamond-at-0→1)))
    (adj (there here) (along diamond-at-1→3))
Connected.span diamond-connected (there (there (there (there ()))))

-- The cycle, read out: `0` along edge `2` to `1`, along edge `1` to `3`,
-- against edge `0` to `2`, against edge `3` back to `0`. Every step uses a
-- different edge from the one before it, so the walk is reduced.
diamond-cycle : Walk diamond d₀ d₀ (just (there (there (there here))))
diamond-cycle =
  hop (there (there (there here)))
    (hop here
      (hop (there here)
        (hop (there (there here)) stay (along diamond-at-0→1) opening)
        (along diamond-at-1→3)
        (apart (λ ())))
      (against diamond-at-2→3)
      (apart (λ ())))
    (against diamond-at-0→2)
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

-- the loop, read off the derived incidence: one flow-through wire, both ends
-- at the vertex, oriented by the wiring with no appeal to colours
wheel-loop : Arc mono wheel here here here
wheel-loop = forth flowing (attach refl refl)

-- The closed directed walk of length one.
wheel-turn : Dir mono wheel here here (just here)
wheel-turn = next here idle wheel-loop

wheel-wheeled : ¬ WheelFree mono wheel
wheel-wheeled wf = wf wheel-turn

-- The content lemma, used forwards.
wheel-not-simply : ¬ SimplyConn wheel
wheel-not-simply sc = wheel-wheeled (simply-conn⇒wheel-free sc)

-- ════════════════════════════════════════════════════════════════════════════
-- EXAMPLE 6: A SELF-GLUING, AT TWO PALETTES. Two vertices, each with one
-- output, their two out-ports capped to each other. As a graph that is ONE
-- edge between two vertices: connected, with `β₁ = 0`, and with no feedback.
--
-- The pair of examples below is the whole argument for separating attachment
-- from direction. The graph is the same at both palettes and every UNDIRECTED
-- fact about it is the same — one wire, one link, connected, acyclic, a cell.
-- What differs is whether the wire RUNS anywhere, and that is not a property
-- of the graph at all: it is the palette's orientation, which the monochrome
-- colour set does not have and the two-pole one does.
--
-- Read the two together and the arc's claim is discharged in the carrier: an
-- orientation is extra structure, the involution is what supplies it, and a
-- shape is expressible with or without one.
-- ════════════════════════════════════════════════════════════════════════════

gluing : Shape ⊤ [] []
gluing =
  node [] 𝟙 (cons nil) nil
    (node [] 𝟙 (cons nil) nil
      (wires (cap head [])))

-- the two vertices, named
n₀ n₁ : Vtx gluing
n₀ = here
n₁ = there here

-- ONE wire — not two — and its two ends are the two vertices.
gluing-one-wire : edges gluing ≡ cut tt tt ∷ []
gluing-one-wire = refl

gluing-attach : Attach gluing here n₁ n₀
gluing-attach = attach refl refl

gluing-link : Link gluing here n₁ n₀
gluing-link = along gluing-attach

-- It is connected, through that one wire.
gluing-connected : Connected gluing
Connected.root gluing-connected = n₀
Connected.span gluing-connected here = stop
Connected.span gluing-connected (there here) =
  onward stop (adj here (against gluing-attach))
Connected.span gluing-connected (there (there ()))

-- And ACYCLIC, which is the correction: one wire cannot be walked out along
-- and back, because reducedness is keyed on the wire and there is only one.
gluing-matched : Matched gluing
Matched.unlooped gluing-matched here here (along (attach () _))
Matched.unlooped gluing-matched here here (against (attach () _))
Matched.unlooped gluing-matched here (there here) (along (attach _ ()))
Matched.unlooped gluing-matched here (there here) (against (attach _ ()))
Matched.unlooped gluing-matched here (there (there ())) _
Matched.unbranched gluing-matched here here u v w l l′ = refl

gluing-simply : SimplyConn gluing
SimplyConn.connected gluing-simply = gluing-connected
SimplyConn.acyclic gluing-simply = matched⇒acyclic gluing-matched

-- so a self-gluing of two cells IS a cell, which is what the graph says
gluing-cell : Cell [] []
Cell.shape gluing-cell = gluing
Cell.simply gluing-cell = gluing-simply

-- AND AT THIS PALETTE THE WIRE RUNS NOWHERE. Both its ends produce, because
-- the one colour is self-dual and `mono` is the only polarity available, so
-- neither direction is inhabited. This is not a gap in the carrier — it is
-- what an unoriented palette means, and `mono-unoriented` says the colour set
-- admits no orientation at all.
gluing-no-arc
  : ∀ {e u v}
  → ¬ Arc mono gluing e u v
gluing-no-arc {e = here} (forth (earlier _ ()) _)
gluing-no-arc {e = here} (back (later () _) _)
gluing-no-arc {e = there ()}

-- ════════════════════════════════════════════════════════════════════════════
-- THE SAME SHAPE OVER THE ORIENTED PALETTE, where the cut does run. The two
-- out-ports are dual — one produces, one consumes — so the wire has a
-- direction, and it is the one the polarity dictates rather than one the
-- representation chose.
-- ════════════════════════════════════════════════════════════════════════════

gluing⁺ : Shape Pole [] []
gluing⁺ =
  node [] (produces ∷ []) (cons nil) nil
    (node [] (consumes ∷ []) (cons nil) nil
      (wires (cap head [])))

-- the producer and the consumer
m₀ m₁ : Vtx gluing⁺
m₀ = here
m₁ = there here

-- its one wire joins a colour to its dual, which is what makes the cut
-- legitimate
gluing⁺-one-wire : edges gluing⁺ ≡ cut consumes produces ∷ []
gluing⁺-one-wire = refl

-- AND SO IT RUNS, from the producing end to the consuming one. Note the
-- direction is `back`: the wire's first end is the consumer, so the arc goes
-- against the listing — which is exactly the datum the listing does not have
-- and the palette does.
gluing⁺-arc : Arc (Palette.pole duality) gluing⁺ here m₀ m₁
gluing⁺-arc = back (later refl refl) (attach refl refl)

-- one arc, no cycle: the shape is wheel-free at the palette that orients it,
-- and it is so for the undirected reason rather than by vacuity
gluing⁺-matched : Matched gluing⁺
Matched.unlooped gluing⁺-matched here here (along (attach () _))
Matched.unlooped gluing⁺-matched here here (against (attach () _))
Matched.unlooped gluing⁺-matched here (there here) (along (attach _ ()))
Matched.unlooped gluing⁺-matched here (there here) (against (attach _ ()))
Matched.unlooped gluing⁺-matched here (there (there ())) _
Matched.unbranched gluing⁺-matched here here u v w l l′ = refl

gluing⁺-wheel-free : WheelFree (Palette.pole duality) gluing⁺
gluing⁺-wheel-free = acyclic⇒wheel-free (matched⇒acyclic gluing⁺-matched)

-- ════════════════════════════════════════════════════════════════════════════
-- AND THE PALETTE THAT DOES NOT EXIST. One self-dual colour admits no
-- orientation: its dual is itself, so its pole would have to be its own
-- opposite. This is what makes the monochrome examples' silence about cuts a
-- fact rather than an omission, and it is why the two-pole colour set had to
-- be introduced to exhibit an oriented cut at all.
-- ════════════════════════════════════════════════════════════════════════════

-- no pole is its own opposite
pole-fix : (p : Pole) → ¬ (p ≡ flip p)
pole-fix produces ()
pole-fix consumes ()

mono-unoriented : ¬ Palette ⊤
mono-unoriented P = pole-fix (Palette.pole P tt) (Palette.pole-dual P tt)

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
