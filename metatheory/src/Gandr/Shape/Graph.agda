{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Graph — the cell shape: finite directed graphs with dangling
-- legs, carrying a chosen linear order on each vertex's ports as DATA.
--
-- A `Graph` here is a finite directed multigraph in which an edge is allowed to
-- lack a source, a target, or both. Those free ends are the object's INTERFACE:
-- an edge with no source is an input port, an edge with no target is an output
-- port. That is what makes the shape genuinely many-in/many-out rather than
-- tree-shaped, and it is the whole reason this module exists.
--
-- ── WHERE THIS SITS IN THE PROPERAD PLAN ────────────────────────────────────
-- This module is ALSO the second arity kit, not a separate object beside one.
-- HRY's graphical category is `Θ = Γ(Gr↑di)`, whose OBJECTS are graphs, so the
-- arity of a properadic composition is a graph in exactly this sense. The
-- linear kit `Gandr.Arity.Path` is the other instance: there an arity is a path,
-- chained end to end.
--
-- An earlier sketch called `Web` is dead, and the reason is worth recording so
-- it is not re-derived. `Web` is a FOREST — many-in, one-out per component,
-- assembled in parallel — hence the operad/PROP-of-trees arity, with no
-- genuinely multi-output vertex and no shared edge. That is precisely the
-- content a properadic cell needs, so `Web` cannot supply it. What survives of
-- `Web` is its `Append`/graph-of-multiplication witness discipline, which
-- `Gandr.Arity.Path` carries and which governs this module too.
--
-- ── WHAT IS DELIBERATELY NOT HERE ───────────────────────────────────────────
-- The arity OPERATIONS — the graph-of-substitution witness, the derived unit,
-- the heterogeneous `Same` — are the next increment and are not attempted. The
-- reason is the one `Gandr.Arity.Path`'s header gives: one instance does not
-- determine an abstraction, and the object layer must be settled before the
-- interface is extracted from two instances rather than invented from one.
--
-- Recorded so the extraction has a target. The shared interface is expected to
-- be: a carrier, a unit, a multiplication spoken only through its inductive
-- graph, and the four derived lemmas (totality, functionality, associativity,
-- whiskering). One thing is expected NOT to be shared, and this module is why:
-- in the linear kit the unit is a CONSTRUCTOR (`here`), whereas here the unit
-- must be DERIVED — the identity arity is the graph with no vertex and one
-- leg-to-leg edge per wire, which is a construction, not a constructor. An
-- identity-shaped constructor repeating a frame variable across its result
-- indices is exactly the discipline this tree forbids.
--
-- ── WHY FINITENESS IS STRUCTURAL RATHER THAN AXIOMATIC ──────────────────────
-- The real constraint on the cell shape is finiteness, not symmetry. HRY
-- Thm 5.9: `Γ(G)` is finite IFF `G` is simply connected — which is what pins
-- the shape class down to the dioperad rung. HRY Cor 6.62: a graphical map is
-- determined by its action on the FINITE edge set, which is where decidable
-- equality will come from downstream.
--
-- So finiteness is carried as data and not abstracted: a graph carries two
-- cardinalities and its vertices and edges ARE `Fin` of those. No finite-set
-- interface is introduced. Every predicate below can therefore be discharged
-- by ordinary structural case analysis on `Fin`, as the worked examples show,
-- and nothing has to be postulated to make the carrier behave.
--
-- ── THE LISTING IS DATA, AND WHAT ITS WITNESS PINS DOWN ─────────────────────
-- `inp v` and `out v` are the CHOSEN linear orders on the edges entering and
-- leaving `v`. They are fields, not derived functions: this is the "structure,
-- not a property" point made operational, and it is the ordered representation
-- the surrounding design commits to (a section of the symmetric quotient, not a
-- planarization of the theory).
--
-- The witness chosen for each listing is a triple — membership-sound,
-- membership-complete, duplicate-free — rather than a packaged bijection with a
-- subtype. The triple was chosen because it is exactly what a consumer uses,
-- and because each conjunct is checkable by case analysis at a concrete graph;
-- a bijection would have to be built and then taken apart again at every use.
--
-- What the triple pins down: the listing enumerates the fibre of `tgt` (resp.
-- `src`) over `v` exactly once, so it determines a linear order on that fibre.
-- Two listings satisfying the triple for the same graph are consequently
-- permutations of one another — a consequence, not a mechanized lemma, since
-- nothing below needs it. What the triple does NOT pin down, deliberately:
-- WHICH order. Two graphs agreeing in every field but the listings are
-- different objects here, and that is the point — the order is the
-- representation content, which is what makes it a section of the symmetric
-- quotient rather than a quotient.
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
--     they must be: two distinct edges between one pair of vertices form a
--     reduced closed walk of length two.
--
-- Reducedness needs edge IDENTITIES, so `Walk` is indexed by the edge it last
-- traversed and its extension constructor refuses to repeat that edge. The
-- index is `Maybe (Edg G)`, headed by a constructor, and the freshness side
-- condition is itself an inductive relation — a defined function never enters a
-- matchable index, in keeping with `Gandr.Arity.Path`'s witness discipline.
--
-- Legs never link: `Link` demands that both ends of an edge be attached, so an
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
--     and it carries the reduced undirected closed walk `0-1-3-2-0`. This is
--     the shape Thm 5.9's finiteness forbids — RECONVERGENCE, not feedback.
--     Branching alone is fine and is what makes the shape properadic.
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
-- prove nothing. `corolla n m` and `chain` satisfy all of them and are
-- packaged as `Cell`s.
--
-- ── A LOCATED WALL, NOT A GAP ───────────────────────────────────────────────
-- `Matched` is a genuinely restrictive certificate: it says each vertex is
-- incident to at most one INTERNAL edge, i.e. the internal part of the graph is
-- a matching. It discharges `Acyclic` for every acyclic example here, because
-- a corolla has no internal edge and the two-vertex composite has exactly one.
-- It does NOT discharge a tree with a branching internal vertex, so the general
-- forest certificate is missing and its absence is located here.
--
-- The obligation is: given a rooted parent structure on the vertices, refute
-- every nontrivial reduced closed walk. The natural attempt is a depth
-- function with "every link joins adjacent depths, and each vertex has one
-- downward link", and it PROVABLY does not close. A reduced walk in a tree is
-- up-phase then down-phase — once a step descends into a vertex through that
-- vertex's parent edge, no later step may ascend, since ascending would have to
-- reuse that same edge and reducedness forbids it. The up-phase strictly
-- decreases depth and the down-phase strictly increases it, so a closed walk
-- balances the two and depth alone derives no contradiction: `up up down down`
-- returns to the starting DEPTH while a tree guarantees only that it reaches a
-- different VERTEX. Depth cannot see that.
--
-- What discharges it is the address rather than the depth: assign each vertex
-- the list of parent edges from it to the root and require that every link
-- either extends or shortens that list by exactly its own edge. Then a reduced
-- walk decomposes as `addr u ≡ p ++ c`, `addr v ≡ q ++ c` with `p` and `q`
-- disagreeing at their heads, and closing the walk forces `p ≡ q`, hence both
-- empty, hence no steps. That is a list-bookkeeping induction of real size and
-- it is deferred, not overlooked. Nothing below depends on it; it is needed
-- only to certify richer worked examples.
--
-- ── WHAT THIS MODULE DOES NOT CLAIM ─────────────────────────────────────────
-- There are no graphical MAPS here, hence no `Γ`, no `Θ`, no Segal condition,
-- and no nerve — Thm 7.42 is cited by the plan, not approached here. There is
-- no decidable equality of graphs; that is Cor 6.62's module and it needs the
-- map layer first. `Cell` below is the C1 shape class as a predicate on
-- objects; it is not yet an object of a category, because this module builds
-- no morphisms. And the finiteness theorem itself (Thm 5.9) is cited as the
-- REASON for the shape class, not proved: nothing here computes `Γ(G)`.
------------------------------------------------------------------------------

module Gandr.Shape.Graph where

-- Arithmetic is repackaged under one qualified name so that `zero` and `suc`
-- read unambiguously as the `Fin` constructors throughout.
private
  module ℕ where
    open import Data.Nat public
      hiding (module ℕ)
    open import Data.Nat.Properties public
open ℕ
  using (ℕ)
  using (_+_)
  using (_<_)

open import Data.Empty
  using (⊥)
  using (⊥-elim)
open import Data.Fin.Base
  using (Fin)
  using (zero)
  using (suc)
  using (_↑ˡ_)
  using (_↑ʳ_)
open import Data.Fin.Properties
  using (↑ˡ-injective)
  using (↑ʳ-injective)
  using (¬Fin0)
open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (tabulate)
open import Data.List.Membership.Propositional
  using (_∈_)
open import Data.List.Membership.Propositional.Properties
  using (∈-tabulate⁺)
  using (∈-tabulate⁻)
open import Data.List.Relation.Unary.All
  using ([])
  using (_∷_)
open import Data.List.Relation.Unary.AllPairs
  using ([])
  using (_∷_)
open import Data.List.Relation.Unary.Any
  using (here)
  using (there)
open import Data.List.Relation.Unary.Unique.Propositional
  using (Unique)
open import Data.List.Relation.Unary.Unique.Propositional.Properties
  using (tabulate⁺)
open import Data.Maybe.Base
  using (Maybe)
  using (just)
  using (nothing)
open import Data.Product.Base
  using (Σ-syntax)
  using (_,_)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (_≢_)
  using (refl)
  using (sym)
  using (trans)
  using (cong)
open import Relation.Nullary.Negation
  using (¬_)

-- Small `Fin` literals as pattern synonyms, so the worked examples below read
-- as incidence tables rather than as towers of `suc`.
pattern 0# = zero
pattern 1# = suc 0#
pattern 2# = suc 1#
pattern 3# = suc 2#

-- ════════════════════════════════════════════════════════════════════════════
-- THE CARRIER. Cardinalities are carried, so vertices and edges are `Fin`; the
-- incidence maps are partial, and a missing end is a port rather than an error.
-- The listings are fields, and their three witnesses say they enumerate the
-- corresponding fibre exactly once.
-- ════════════════════════════════════════════════════════════════════════════

record Graph : Set where
  field
    -- how many vertices; a vertex IS an index below this
    ∣V∣ : ℕ
    -- how many edges; an edge IS an index below this
    ∣E∣ : ℕ
    -- where an edge starts; `nothing` means it dangles, i.e. an INPUT port
    src : Fin ∣E∣ → Maybe (Fin ∣V∣)
    -- where an edge ends; `nothing` means it dangles, i.e. an OUTPUT port
    tgt : Fin ∣E∣ → Maybe (Fin ∣V∣)
    -- the chosen order on the edges entering `v`
    inp : Fin ∣V∣ → List (Fin ∣E∣)
    -- the chosen order on the edges leaving `v`
    out : Fin ∣V∣ → List (Fin ∣E∣)
    -- nothing listed as an input of `v` fails to end at `v`
    inp-sound : (v : Fin ∣V∣) (e : Fin ∣E∣) → e ∈ inp v → tgt e ≡ just v
    -- nothing ending at `v` is left out of the listing
    inp-total : (v : Fin ∣V∣) (e : Fin ∣E∣) → tgt e ≡ just v → e ∈ inp v
    -- and nothing is listed twice, so the listing is an ORDER on the fibre
    inp-uniq : (v : Fin ∣V∣) → Unique (inp v)
    -- the same three, for the edges leaving `v`
    out-sound : (v : Fin ∣V∣) (e : Fin ∣E∣) → e ∈ out v → src e ≡ just v
    out-total : (v : Fin ∣V∣) (e : Fin ∣E∣) → src e ≡ just v → e ∈ out v
    out-uniq : (v : Fin ∣V∣) → Unique (out v)

open Graph public

-- The vertices of `G`. Finiteness is structural rather than assumed: a vertex
-- is an index, so every predicate below is decided by case analysis.
Vtx : Graph → Set
Vtx G = Fin (∣V∣ G)

-- The edges of `G`, on the same footing.
Edg : Graph → Set
Edg G = Fin (∣E∣ G)

-- A dangling end is not an endpoint. Used wherever a leg has to be told apart
-- from an internal incidence.
dangling
  : ∀ {A : Set} {x : A}
  → nothing ≡ just x
  → ⊥
dangling ()

-- ════════════════════════════════════════════════════════════════════════════
-- INCIDENCE. `Arc` is the directed incidence — an edge with both ends attached
-- — and `Link` is the same edge read in either orientation. Only `Link` is a
-- datatype, because only the orientation ever needs to be case-analysed.
-- ════════════════════════════════════════════════════════════════════════════

record Arc (G : Graph) (e : Edg G) (u v : Vtx G) : Set where
  constructor arc
  field
    -- `e` leaves `u`
    from : src G e ≡ just u
    -- `e` enters `v`
    into : tgt G e ≡ just v

data Link (G : Graph) (e : Edg G) : Vtx G → Vtx G → Set where
  -- traverse `e` in its own direction
  along : ∀ {u v} → Arc G e u v → Link G e u v
  -- traverse `e` against its direction
  against : ∀ {u v} → Arc G e v u → Link G e u v

-- Links are symmetric, which is what makes undirected reachability an
-- equivalence and what the diamond's cycle is built from.
link-sym
  : ∀ {G e u v}
  → Link G e u v
  → Link G e v u
link-sym (along a) = against a
link-sym (against a) = along a

-- Adjacency: linked by SOME edge. The edge is kept explicit so that a consumer
-- can name it, but reachability below never inspects it.
record Adj (G : Graph) (u v : Vtx G) : Set where
  constructor adj
  field
    -- the edge doing the joining
    edge : Edg G
    -- and the orientation it is traversed in
    step : Link G edge u v

-- Adjacency is symmetric, by symmetry of links.
adj-sym
  : ∀ {G u v}
  → Adj G u v
  → Adj G v u
adj-sym (adj e l) = adj e (link-sym l)

-- ════════════════════════════════════════════════════════════════════════════
-- REACHABILITY. The reflexive–transitive closure of adjacency, in the snoc
-- shape `Gandr.Arity.Path` uses. It forgets which edges were traversed, which
-- is exactly why it composes and reverses freely — connectivity does not care
-- about reducedness and should not pay for it.
-- ════════════════════════════════════════════════════════════════════════════

data Reach (G : Graph) (u : Vtx G) : Vtx G → Set where
  -- the empty walk
  stop : Reach G u u
  -- extend by one adjacency on the right
  onward : ∀ {m v} → Reach G u m → Adj G m v → Reach G u v

-- Reachability composes, by recursion on the second walk.
reach-trans
  : ∀ {G u m v}
  → Reach G u m
  → Reach G m v
  → Reach G u v
reach-trans r stop = r
reach-trans r (onward s a) = onward (reach-trans r s) a

-- Reachability reverses, since every adjacency does.
reach-sym
  : ∀ {G u v}
  → Reach G u v
  → Reach G v u
reach-sym stop = stop
reach-sym (onward r a) = reach-trans (onward stop (adj-sym a)) (reach-sym r)

-- ════════════════════════════════════════════════════════════════════════════
-- WALKS. Two walk notions, doing two jobs. `Dir` chains ARCS and is what a
-- wheel is a closed instance of. `Walk` chains LINKS and is REDUCED: it is
-- indexed by the edge it last traversed and refuses to traverse it again.
-- ════════════════════════════════════════════════════════════════════════════

-- The freshness side condition, as a constructor-headed relation rather than a
-- defined function, so that it never has to reduce inside an index.
data Fresh {A : Set} (x : A) : Maybe A → Set where
  -- nothing has been traversed yet, so anything is fresh
  opening : Fresh x nothing
  -- the previous traversal used a different element
  apart : ∀ {y} → x ≢ y → Fresh x (just y)

data Walk (G : Graph) (u : Vtx G) : Vtx G → Maybe (Edg G) → Set where
  -- the empty walk, which has traversed nothing
  stay : Walk G u u nothing
  -- extend by one link, which must not be the link just traversed
  hop
    : ∀ {m v b}
    → (e : Edg G)
    → Walk G u m b
    → Link G e m v
    → Fresh e b
    → Walk G u v (just e)

data Dir (G : Graph) (u : Vtx G) : Vtx G → Maybe (Edg G) → Set where
  -- the empty directed walk
  idle : Dir G u u nothing
  -- extend by one arc; no freshness is imposed, since a directed walk cannot
  -- backtrack except through a self-loop, which `dir⇒walk` handles
  next
    : ∀ {m v b}
    → (e : Edg G)
    → Dir G u m b
    → Arc G e m v
    → Dir G u v (just e)

-- Every reduced walk is in particular a reachability witness. This is the
-- forgetful direction and it is cheap; the converse is not needed and is not
-- claimed.
walk⇒reach
  : ∀ {G u v b}
  → Walk G u v b
  → Reach G u v
walk⇒reach stay = stop
walk⇒reach (hop e w l _) = onward (walk⇒reach w) (adj e l)

-- ════════════════════════════════════════════════════════════════════════════
-- THE PREDICATES. `WheelFree` forbids feedback; `Connected` and `Acyclic` are
-- the combinatorial readings of `β₀ = 1` and `β₁ = 0` argued for in the header.
-- ════════════════════════════════════════════════════════════════════════════

-- No directed cycle: no closed directed walk that traversed at least one arc.
WheelFree : Graph → Set
WheelFree G = ∀ {v : Vtx G} {e : Edg G} → ¬ Dir G v v (just e)

-- No undirected cycle: no closed REDUCED walk that traversed at least one
-- edge. The `just e` index is what "nontrivial" means here.
Acyclic : Graph → Set
Acyclic G = ∀ {v : Vtx G} {e : Edg G} → ¬ Walk G v v (just e)

record Connected (G : Graph) : Set where
  field
    -- there is at least one vertex, so the component count is not zero
    root : Vtx G
    -- and there is at most one component
    span : (v : Vtx G) → Reach G root v

-- Connectivity does not depend on the root: any two vertices are joined. This
-- is what makes the record's `root` a witness rather than extra structure.
reach-any
  : ∀ {G}
  → Connected G
  → (u v : Vtx G)
  → Reach G u v
reach-any c u v =
  reach-trans (reach-sym (Connected.span c u)) (Connected.span c v)

record SimplyConn (G : Graph) : Set where
  field
    -- `β₀ = 1`
    connected : Connected G
    -- `β₁ = 0`
    acyclic : Acyclic G

-- ════════════════════════════════════════════════════════════════════════════
-- CERTIFICATE 1: RANK. A height that every arc strictly increases refutes every
-- wheel at once, and is what every worked example below uses.
-- ════════════════════════════════════════════════════════════════════════════

record Ranked (G : Graph) : Set where
  field
    -- the height of a vertex
    rank : Vtx G → ℕ
    -- every arc goes strictly uphill
    climbs : (e : Edg G) (u v : Vtx G) → Arc G e u v → rank u < rank v

-- A nonempty directed walk strictly increases rank. Recursion peels the last
-- arc, so the two clauses are "one arc" and "more than one".
dir-rank
  : ∀ {G u v e}
  → (R : Ranked G)
  → Dir G u v (just e)
  → Ranked.rank R u < Ranked.rank R v
dir-rank R (next e idle a) = Ranked.climbs R e _ _ a
dir-rank R (next e (next f d a′) a) =
  ℕ.<-trans (dir-rank R (next f d a′)) (Ranked.climbs R e _ _ a)

-- Hence a ranked graph is wheel-free: a closed directed walk would put a rank
-- strictly below itself.
ranked⇒wheel-free
  : ∀ {G}
  → Ranked G
  → WheelFree G
ranked⇒wheel-free R d = ℕ.n≮n _ (dir-rank R d)

-- ════════════════════════════════════════════════════════════════════════════
-- CERTIFICATE 2: MATCHING. If the internal edges form a matching — at most one
-- per vertex, none a loop — then a reduced closed walk cannot exist, because it
-- would have to leave and return through two distinct edges at some vertex.
-- The header records exactly how far this falls short of a forest certificate.
-- ════════════════════════════════════════════════════════════════════════════

record Matched (G : Graph) : Set where
  field
    -- no edge links a vertex to itself
    unlooped : (e : Edg G) (v : Vtx G) → ¬ Link G e v v
    -- no vertex carries two distinct link edges
    unbranched
      : (e f : Edg G) (u v w : Vtx G)
      → Link G e v u
      → Link G f v w
      → e ≡ f

-- The refutation is two levels deep and no more: a closed reduced walk is
-- either a single link from a vertex to itself, or has two consecutive links
-- meeting at a vertex, and both are excluded.
matched⇒acyclic
  : ∀ {G}
  → Matched G
  → Acyclic G
matched⇒acyclic M (hop e stay l opening) = Matched.unlooped M e _ l
matched⇒acyclic M (hop e (hop f w l′ _) l (apart e≢f)) =
  e≢f (Matched.unbranched M e f _ _ _ l (link-sym l′))

-- ════════════════════════════════════════════════════════════════════════════
-- THE CONTENT LEMMA — HRY Rmk 2.36's easy direction. Simple connectivity is
-- strictly stronger than wheel-freeness; `diamond` below exhibits the strictly.
-- ════════════════════════════════════════════════════════════════════════════

-- Acyclicity kills self-loops outright, since a loop at `v` is already a
-- reduced closed walk of length one.
loop-free
  : ∀ {G e v}
  → Acyclic G
  → ¬ Arc G e v v
loop-free ac a = ac (hop _ stay (along a) opening)

-- The last arc of a nonempty directed walk ends where the walk does.
dir-into
  : ∀ {G u v e}
  → Dir G u v (just e)
  → tgt G e ≡ just v
dir-into (next _ _ a) = Arc.into a

-- If a directed walk's next arc repeats its last edge then that edge is a
-- self-loop: the repeated edge must both end and start at the shared vertex.
-- This is the only obstruction to reading a directed walk as a reduced one.
arc-repeat
  : ∀ {G : Graph} {u m v : Vtx G} {e f : Edg G}
  → Dir G u m (just f)
  → Arc G e m v
  → e ≡ f
  → Arc G e m m
arc-repeat d a refl = arc (Arc.from a) (dir-into d)

-- So, given no self-loops, every directed walk IS a reduced undirected walk,
-- with the same last-edge index.
dir⇒walk
  : ∀ {G : Graph} {u v : Vtx G} {b}
  → ((e : Edg G) (w : Vtx G) → ¬ Arc G e w w)
  → Dir G u v b
  → Walk G u v b
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
  : ∀ {G}
  → Acyclic G
  → WheelFree G
acyclic⇒wheel-free ac d = ac (dir⇒walk (λ _ _ → loop-free ac) d)

-- HRY Rmk 2.36, easy direction.
simply-conn⇒wheel-free
  : ∀ {G}
  → SimplyConn G
  → WheelFree G
simply-conn⇒wheel-free sc = acyclic⇒wheel-free (SimplyConn.acyclic sc)

-- ════════════════════════════════════════════════════════════════════════════
-- THE CELL SHAPE (commitment C1). The sketch this module was built from listed
-- wheel-freeness AND simple connectivity as a pair of fields; the pair is
-- redundant by the lemma just proved, so wheel-freeness is DERIVED here. That
-- divergence is the lemma's whole point, and stating it as a field would have
-- let an inconsistent `Cell` be written.
-- ════════════════════════════════════════════════════════════════════════════

record Cell : Set where
  field
    -- the underlying shape
    shape : Graph
    -- connected, and free of undirected cycles
    simply : SimplyConn shape

  -- and therefore free of directed cycles, at no extra cost
  wheel-free : WheelFree shape
  wheel-free = simply-conn⇒wheel-free simply

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLE 1: THE EMPTY GRAPH. Wheel-free and acyclic, but NOT connected
-- — `β₀ = 0`, not `1`. This is why `Connected` carries a root: without the
-- existence half, the empty graph would satisfy every predicate vacuously.
-- ════════════════════════════════════════════════════════════════════════════

empty : Graph
∣V∣ empty = 0
∣E∣ empty = 0
src empty ()
tgt empty ()
inp empty ()
out empty ()
inp-sound empty ()
inp-total empty ()
inp-uniq empty ()
out-sound empty ()
out-total empty ()
out-uniq empty ()

-- No edges, so no links, so the matching conditions hold vacuously.
empty-matched : Matched empty
Matched.unlooped empty-matched ()
Matched.unbranched empty-matched ()

empty-acyclic : Acyclic empty
empty-acyclic = matched⇒acyclic empty-matched

-- No vertices, so ranking is vacuous too.
empty-ranked : Ranked empty
Ranked.rank empty-ranked ()
Ranked.climbs empty-ranked ()

empty-wheel-free : WheelFree empty
empty-wheel-free = ranked⇒wheel-free empty-ranked

-- The refutation: connectivity demands a vertex and there is none.
empty-unconnected : ¬ Connected empty
empty-unconnected c = ¬Fin0 (Connected.root c)

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLE 2: THE GENERATING COROLLA `C(n;m)`. One vertex, `n` input legs
-- and `m` output legs, no internal edge. This is the generic cell of the shape
-- class and it is built for ALL `n` and `m`, so the family is not a finite
-- sample. The incidence maps recurse on `n` rather than going through
-- `splitAt`, which keeps every proof below a structural induction.
-- ════════════════════════════════════════════════════════════════════════════

-- The target of the corolla's `e`-th edge: the first `n` edges land on the
-- vertex, the last `m` dangle.
corolla-tgt : (n m : ℕ) → Fin (n + m) → Maybe (Fin 1)
corolla-tgt ℕ.zero m e = nothing
corolla-tgt (ℕ.suc n) m 0# = just 0#
corolla-tgt (ℕ.suc n) m (suc e) = corolla-tgt n m e

-- The source, dually: the first `n` edges dangle, the last `m` leave the
-- vertex.
corolla-src : (n m : ℕ) → Fin (n + m) → Maybe (Fin 1)
corolla-src ℕ.zero m e = just 0#
corolla-src (ℕ.suc n) m 0# = nothing
corolla-src (ℕ.suc n) m (suc e) = corolla-src n m e

-- Every input leg lands on the vertex.
corolla-tgt-leg : (n m : ℕ) (i : Fin n) → corolla-tgt n m (i ↑ˡ m) ≡ just 0#
corolla-tgt-leg ℕ.zero m ()
corolla-tgt-leg (ℕ.suc n) m 0# = refl
corolla-tgt-leg (ℕ.suc n) m (suc i) = corolla-tgt-leg n m i

-- …and only the input legs do, which is the completeness half of the listing.
corolla-tgt-inv
  : (n m : ℕ) (e : Fin (n + m))
  → corolla-tgt n m e ≡ just 0#
  → Σ[ i ∈ Fin n ] e ≡ i ↑ˡ m
corolla-tgt-inv ℕ.zero m e ()
corolla-tgt-inv (ℕ.suc n) m 0# _ = 0# , refl
corolla-tgt-inv (ℕ.suc n) m (suc e) h with corolla-tgt-inv n m e h
... | i , eq = suc i , cong suc eq

-- Every output leg leaves the vertex.
corolla-src-leg : (n m : ℕ) (j : Fin m) → corolla-src n m (n ↑ʳ j) ≡ just 0#
corolla-src-leg ℕ.zero m j = refl
corolla-src-leg (ℕ.suc n) m j = corolla-src-leg n m j

-- …and only the output legs do.
corolla-src-inv
  : (n m : ℕ) (e : Fin (n + m))
  → corolla-src n m e ≡ just 0#
  → Σ[ j ∈ Fin m ] e ≡ n ↑ʳ j
corolla-src-inv ℕ.zero m e _ = e , refl
corolla-src-inv (ℕ.suc n) m 0# ()
corolla-src-inv (ℕ.suc n) m (suc e) h with corolla-src-inv n m e h
... | j , eq = j , cong suc eq

-- No corolla edge has both ends attached: every edge is a leg. This is what
-- makes the corolla acyclic, and it is a statement about the INTERFACE, which
-- is the sense in which a corolla is all interface and no topology.
corolla-leg
  : (n m : ℕ) (e : Fin (n + m)) {u v : Fin 1}
  → corolla-src n m e ≡ just u
  → corolla-tgt n m e ≡ just v
  → ⊥
corolla-leg ℕ.zero m e _ ()
corolla-leg (ℕ.suc n) m 0# () _
corolla-leg (ℕ.suc n) m (suc e) s t = corolla-leg n m e s t

corolla : (n m : ℕ) → Graph
∣V∣ (corolla n m) = 1
∣E∣ (corolla n m) = n + m
src (corolla n m) = corolla-src n m
tgt (corolla n m) = corolla-tgt n m
inp (corolla n m) _ = tabulate {n = n} (_↑ˡ m)
out (corolla n m) _ = tabulate {n = m} (n ↑ʳ_)
inp-sound (corolla n m) 0# e mem with ∈-tabulate⁻ mem
... | i , refl = corolla-tgt-leg n m i
inp-total (corolla n m) 0# e h with corolla-tgt-inv n m e h
... | i , refl = ∈-tabulate⁺ i
inp-uniq (corolla n m) _ = tabulate⁺ (λ {i} {j} → ↑ˡ-injective m i j)
out-sound (corolla n m) 0# e mem with ∈-tabulate⁻ mem
... | j , refl = corolla-src-leg n m j
out-total (corolla n m) 0# e h with corolla-src-inv n m e h
... | j , refl = ∈-tabulate⁺ j
out-uniq (corolla n m) _ = tabulate⁺ (λ {i} {j} → ↑ʳ-injective n i j)

-- The corolla has no links at all, in either orientation.
corolla-unlinked
  : (n m : ℕ) (e : Edg (corolla n m)) (u v : Vtx (corolla n m))
  → ¬ Link (corolla n m) e u v
corolla-unlinked n m e u v (along a) =
  corolla-leg n m e (Arc.from a) (Arc.into a)
corolla-unlinked n m e u v (against a) =
  corolla-leg n m e (Arc.from a) (Arc.into a)

corolla-matched : (n m : ℕ) → Matched (corolla n m)
Matched.unlooped (corolla-matched n m) e v = corolla-unlinked n m e v v
Matched.unbranched (corolla-matched n m) e f u v w l l′ =
  ⊥-elim (corolla-unlinked n m e v u l)

-- Ranking is vacuous for the same reason: there is no arc to climb.
corolla-ranked : (n m : ℕ) → Ranked (corolla n m)
Ranked.rank (corolla-ranked n m) _ = 0
Ranked.climbs (corolla-ranked n m) e u v a =
  ⊥-elim (corolla-leg n m e (Arc.from a) (Arc.into a))

corolla-connected : (n m : ℕ) → Connected (corolla n m)
Connected.root (corolla-connected n m) = 0#
Connected.span (corolla-connected n m) 0# = stop

corolla-simply : (n m : ℕ) → SimplyConn (corolla n m)
SimplyConn.connected (corolla-simply n m) = corolla-connected n m
SimplyConn.acyclic (corolla-simply n m) = matched⇒acyclic (corolla-matched n m)

-- The generating cell of the shape class, for every arity.
corolla-cell : (n m : ℕ) → Cell
Cell.shape (corolla-cell n m) = corolla n m
Cell.simply (corolla-cell n m) = corolla-simply n m

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLE 3: A TWO-VERTEX COMPOSITE. `C(2;1)` feeding `C(1;1)`. Edges
-- `0` and `1` are input legs of vertex `0`; edge `2` is the internal edge
-- `0 → 1`; edge `3` is the output leg of vertex `1`. A genuine composite, and a
-- genuine cell.
-- ════════════════════════════════════════════════════════════════════════════

chain : Graph
∣V∣ chain = 2
∣E∣ chain = 4
src chain 0# = nothing
src chain 1# = nothing
src chain 2# = just 0#
src chain 3# = just 1#
tgt chain 0# = just 0#
tgt chain 1# = just 0#
tgt chain 2# = just 1#
tgt chain 3# = nothing
inp chain 0# = 0# ∷ 1# ∷ []
inp chain 1# = 2# ∷ []
out chain 0# = 2# ∷ []
out chain 1# = 3# ∷ []
inp-sound chain 0# _ (here refl) = refl
inp-sound chain 0# _ (there (here refl)) = refl
inp-sound chain 1# _ (here refl) = refl
inp-total chain _ 0# refl = here refl
inp-total chain _ 1# refl = there (here refl)
inp-total chain _ 2# refl = here refl
inp-total chain _ 3# ()
inp-uniq chain 0# = ((λ ()) ∷ []) ∷ [] ∷ []
inp-uniq chain 1# = [] ∷ []
out-sound chain 0# _ (here refl) = refl
out-sound chain 1# _ (here refl) = refl
out-total chain _ 0# ()
out-total chain _ 1# ()
out-total chain _ 2# refl = here refl
out-total chain _ 3# refl = here refl
out-uniq chain 0# = [] ∷ []
out-uniq chain 1# = [] ∷ []

chain-ranked : Ranked chain
Ranked.rank chain-ranked 0# = 0
Ranked.rank chain-ranked 1# = 1
Ranked.climbs chain-ranked 0# u v a = ⊥-elim (dangling (Arc.from a))
Ranked.climbs chain-ranked 1# u v a = ⊥-elim (dangling (Arc.from a))
Ranked.climbs chain-ranked 2# _ _ (arc refl refl) = ℕ.n<1+n 0
Ranked.climbs chain-ranked 3# u v a = ⊥-elim (dangling (Arc.into a))

chain-wheel-free : WheelFree chain
chain-wheel-free = ranked⇒wheel-free chain-ranked

-- The only linkable edge is the internal one: the other three each dangle at
-- one end. This single fact discharges both matching conditions.
chain-link
  : ∀ {e u v}
  → Link chain e u v
  → e ≡ 2#
chain-link {e = 0#} (along a) = ⊥-elim (dangling (Arc.from a))
chain-link {e = 0#} (against a) = ⊥-elim (dangling (Arc.from a))
chain-link {e = 1#} (along a) = ⊥-elim (dangling (Arc.from a))
chain-link {e = 1#} (against a) = ⊥-elim (dangling (Arc.from a))
chain-link {e = 2#} _ = refl
chain-link {e = 3#} (along a) = ⊥-elim (dangling (Arc.into a))
chain-link {e = 3#} (against a) = ⊥-elim (dangling (Arc.into a))

-- The internal edge runs between the two DISTINCT vertices, so it is no loop.
chain-unlooped : (e : Edg chain) (v : Vtx chain) → ¬ Link chain e v v
chain-unlooped 0# v (along a) = dangling (Arc.from a)
chain-unlooped 0# v (against a) = dangling (Arc.from a)
chain-unlooped 1# v (along a) = dangling (Arc.from a)
chain-unlooped 1# v (against a) = dangling (Arc.from a)
chain-unlooped 2# _ (along (arc refl ()))
chain-unlooped 2# _ (against (arc refl ()))
chain-unlooped 3# v (along a) = dangling (Arc.into a)
chain-unlooped 3# v (against a) = dangling (Arc.into a)

chain-matched : Matched chain
Matched.unlooped chain-matched = chain-unlooped
Matched.unbranched chain-matched e f u v w l l′ =
  trans (chain-link l) (sym (chain-link l′))

chain-connected : Connected chain
Connected.root chain-connected = 0#
Connected.span chain-connected 0# = stop
Connected.span chain-connected 1# = onward stop (adj 2# (along (arc refl refl)))

chain-simply : SimplyConn chain
SimplyConn.connected chain-simply = chain-connected
SimplyConn.acyclic chain-simply = matched⇒acyclic chain-matched

chain-cell : Cell
Cell.shape chain-cell = chain
Cell.simply chain-cell = chain-simply

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLE 4: THE DIAMOND — the separation. Vertex `0` fans out to `1`
-- and `2`, which reconverge on `3`. It is wheel-free and connected, and it
-- carries the reduced undirected closed walk `0-1-3-2-0`, so it is NOT simply
-- connected. This is what makes `SimplyConn` strictly stronger than
-- `WheelFree` (HRY Rmk 2.36), and it is the shape Thm 5.9's finiteness rules
-- out — reconvergence, not branching.
-- ════════════════════════════════════════════════════════════════════════════

diamond : Graph
∣V∣ diamond = 4
∣E∣ diamond = 4
src diamond 0# = just 0#
src diamond 1# = just 0#
src diamond 2# = just 1#
src diamond 3# = just 2#
tgt diamond 0# = just 1#
tgt diamond 1# = just 2#
tgt diamond 2# = just 3#
tgt diamond 3# = just 3#
inp diamond 0# = []
inp diamond 1# = 0# ∷ []
inp diamond 2# = 1# ∷ []
inp diamond 3# = 2# ∷ 3# ∷ []
out diamond 0# = 0# ∷ 1# ∷ []
out diamond 1# = 2# ∷ []
out diamond 2# = 3# ∷ []
out diamond 3# = []
inp-sound diamond 0# _ ()
inp-sound diamond 1# _ (here refl) = refl
inp-sound diamond 2# _ (here refl) = refl
inp-sound diamond 3# _ (here refl) = refl
inp-sound diamond 3# _ (there (here refl)) = refl
inp-total diamond _ 0# refl = here refl
inp-total diamond _ 1# refl = here refl
inp-total diamond _ 2# refl = here refl
inp-total diamond _ 3# refl = there (here refl)
inp-uniq diamond 0# = []
inp-uniq diamond 1# = [] ∷ []
inp-uniq diamond 2# = [] ∷ []
inp-uniq diamond 3# = ((λ ()) ∷ []) ∷ [] ∷ []
out-sound diamond 0# _ (here refl) = refl
out-sound diamond 0# _ (there (here refl)) = refl
out-sound diamond 1# _ (here refl) = refl
out-sound diamond 2# _ (here refl) = refl
out-sound diamond 3# _ ()
out-total diamond _ 0# refl = here refl
out-total diamond _ 1# refl = there (here refl)
out-total diamond _ 2# refl = here refl
out-total diamond _ 3# refl = here refl
out-uniq diamond 0# = ((λ ()) ∷ []) ∷ [] ∷ []
out-uniq diamond 1# = [] ∷ []
out-uniq diamond 2# = [] ∷ []
out-uniq diamond 3# = []

-- Heights `0 < 1 < 2`, with the two middle vertices at the same height.
diamond-ranked : Ranked diamond
Ranked.rank diamond-ranked 0# = 0
Ranked.rank diamond-ranked 1# = 1
Ranked.rank diamond-ranked 2# = 1
Ranked.rank diamond-ranked 3# = 2
Ranked.climbs diamond-ranked 0# _ _ (arc refl refl) = ℕ.n<1+n 0
Ranked.climbs diamond-ranked 1# _ _ (arc refl refl) = ℕ.n<1+n 0
Ranked.climbs diamond-ranked 2# _ _ (arc refl refl) = ℕ.n<1+n 1
Ranked.climbs diamond-ranked 3# _ _ (arc refl refl) = ℕ.n<1+n 1

diamond-wheel-free : WheelFree diamond
diamond-wheel-free = ranked⇒wheel-free diamond-ranked

diamond-connected : Connected diamond
Connected.root diamond-connected = 0#
Connected.span diamond-connected 0# = stop
Connected.span diamond-connected 1# = onward stop (adj 0# (along (arc refl refl)))
Connected.span diamond-connected 2# = onward stop (adj 1# (along (arc refl refl)))
Connected.span diamond-connected 3# =
  onward (onward stop (adj 0# (along (arc refl refl))))
    (adj 2# (along (arc refl refl)))

-- The cycle, read out: `0` along edge `0` to `1`, along edge `2` to `3`,
-- against edge `3` to `2`, against edge `1` back to `0`. Every step uses a
-- different edge from the one before it, so the walk is reduced.
diamond-cycle : Walk diamond 0# 0# (just 1#)
diamond-cycle =
  hop 1#
    (hop 3#
      (hop 2#
        (hop 0# stay (along (arc refl refl)) opening)
        (along (arc refl refl))
        (apart (λ ())))
      (against (arc refl refl))
      (apart (λ ())))
    (against (arc refl refl))
    (apart (λ ()))

diamond-cyclic : ¬ Acyclic diamond
diamond-cyclic ac = ac diamond-cycle

-- The separation: wheel-free (above) but not simply connected. Both halves of
-- HRY Rmk 2.36 are therefore exhibited, not asserted — the implication by
-- `simply-conn⇒wheel-free`, its failure in the other direction by this.
diamond-not-simply : ¬ SimplyConn diamond
diamond-not-simply sc = SimplyConn.acyclic sc diamond-cycle

-- ════════════════════════════════════════════════════════════════════════════
-- WORKED EXAMPLE 5: THE WHEEL. One vertex, one edge from it back to itself.
-- This is what `WheelFree` forbids, and exhibiting it is what stops that
-- predicate from being vacuously true — `ranked⇒wheel-free` would prove nothing
-- if no graph could carry a closed directed walk. It is also the self-loop that
-- `dir⇒walk` had to be written around: reading this wheel as an undirected walk
-- is where reducedness could have been broken.
--
-- Its exclusion is C5's confinement of feedback to the term face, and it is
-- refuted here through the content lemma rather than directly: a wheel refutes
-- wheel-freeness, so by `simply-conn⇒wheel-free` it refutes simple
-- connectivity, which is the lemma used forwards.
-- ════════════════════════════════════════════════════════════════════════════

wheel : Graph
∣V∣ wheel = 1
∣E∣ wheel = 1
src wheel 0# = just 0#
tgt wheel 0# = just 0#
inp wheel 0# = 0# ∷ []
out wheel 0# = 0# ∷ []
inp-sound wheel 0# _ (here refl) = refl
inp-total wheel _ 0# refl = here refl
inp-uniq wheel 0# = [] ∷ []
out-sound wheel 0# _ (here refl) = refl
out-total wheel _ 0# refl = here refl
out-uniq wheel 0# = [] ∷ []

-- The closed directed walk of length one.
wheel-turn : Dir wheel 0# 0# (just 0#)
wheel-turn = next 0# idle (arc refl refl)

wheel-wheeled : ¬ WheelFree wheel
wheel-wheeled wf = wf wheel-turn

-- The content lemma, used forwards.
wheel-not-simply : ¬ SimplyConn wheel
wheel-not-simply sc = wheel-wheeled (simply-conn⇒wheel-free sc)
