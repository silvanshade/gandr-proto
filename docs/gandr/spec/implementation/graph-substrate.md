# The graph substrate

**Built, as `gandr-theory-graphs`, and consumed by two crates.** The substrate this document specifies is the one place in the workspace where graph structure over arena identifiers is turned into queryable relations, and it is the only crate permitted to depend on a third-party graph library.

The design has one governing rule and one consequence.
The rule: **identity lives in flat arenas that own their data, and relations live in one shared crate that owns none of it.** The consequence: no other crate implements graph search, reachability, cycle detection, or topological ordering again — including the ones that had independently begun to.

**The shape mirrors the semantics, which is why this is a design and not a convenience.** This system's data are polygraph presentations: generators and cells, which are structured graphs.
Reifying "a relation over identifiers" as one queryable artifact is the same move one level down from the thing the language is about.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The crate exists and is `gandr-theory-graphs`**, at `crates/theory-graphs`, in eight modules: the algorithm menu, partition refinement, the precedence DAG, the walk index, the private petgraph adapter, adjacency fingerprinting, the dense identifier types, and the crate root that owns the `EdgeSource` boundary.
* **Two crates consume it today**: `gandr-surface-grammar` and `gandr-theory-computads` — two of the six consumers the dependency rule below permits, with the module and incremental layers and the reflection face not yet among them.
  **The parser crate is deliberately not a third**, and the distinction between a lane and a crate is what makes this checkable: the grammar crate re-exports the precedence-DAG types, so `crates/surface-parser/Cargo.toml` carries no `gandr-theory-graphs` edge and says at the dependency why.
  The parser _lane_ consumes the substrate; the parser _crate_ consumes the grammar crate.
* **petgraph appears in exactly one manifest.** `crates/theory-graphs/Cargo.toml` is the only crate manifest naming it, and the workspace dependency table records that crate as its sole consumer.
* **The adapter is crate-private.** The `view` module is `pub(crate)`, so no petgraph type reaches the public API — which is what makes the exit path below real rather than aspirational.
* **The determinism harness is a binary plus a subprocess test**, not a convention: `gandr-theory-graphs-determinism` prints canonical row bytes for the graph foundation, the precedence DAG, and both partition-refinement results under an allocation perturbation controlled by an environment variable, and the harness compares runs across processes.
* **The precedence DAG and the walk index are public artifacts** with the shapes specified below.
* **The certificate-composition acyclicity gate is a call into this crate.** `gandr-theory-computads`'s `compose` module builds the variable-flow edge source and calls the shared cycle witness once.

**Designed, and not built.** The layout document DAG that the boundary section excludes does not exist yet, so that boundary is a standing constraint rather than an enforced one.
The module import graph and the incremental pipeline's dependency graph are named here as future clients and neither consumes the crate today.

**Two divergences from the design record, both checkable in one command, and both stated rather than smoothed.**

**The dependency closure is six crate names but seven resolved packages.** The record measured six — petgraph, fixedbitset, indexmap, equivalent, hashbrown, foldhash — at the same petgraph version the tree pins today, 0.8.3.
Resolved now, `hashbrown` appears **twice** at different versions, once beneath petgraph and once beneath indexmap.
The count of distinct crates is unchanged; the count of compiled packages is one higher.

**`fixedbitset` is a direct dependency, not only a transitive one.** The record's adoption plan named petgraph as the crate's single new dependency; the tree depends on `fixedbitset` directly as well, because partition refinement and the dense reachability closures use its bit sets without going through petgraph.

## The recurring shape

Twelve lanes arrived at the same machinery from independent directions, and the table is what turned that into a substrate rather than a coincidence.

| lane                  | graph structure                                        | operations needed                                                             | substrate                 |
| --------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------- | ------------------------- |
| parser: precedence    | small dense named DAG with adjoined bottom and top     | cycle rejection with witness; strict reachability; incomparability; extension | this crate                |
| parser: walks         | molded-terminal graph                                  | shortest-first walk enumeration; filtering; total tie-break; precomputation   | this crate                |
| parser: syntax tree   | flat arena tree with merkle identity                   | constant-time subtree identity; top-down diff over child hashes               | arena                     |
| pipeline: incremental | dynamic dependency graph over footprints               | invalidation propagation; topological order                                   | this crate, later         |
| modules               | import graph                                           | cycle detection with witness; topological order; stratification               | this crate, later         |
| kernel: hash-consing  | content DAG over arena node identifiers                | interning; erased skeleton hashing                                            | arena                     |
| polygraph: cells      | oriented cells over command patterns; overlap families | seam unification (bespoke); completion; critical-pair enumeration             | arena, plus utilities     |
| certificates          | variable-flow graph across composed seams              | acyclicity gate, with the cycle as the diagnostic                             | this crate                |
| coinductive relations | finitely generated transition graphs                   | partition refinement                                                          | this crate                |
| reflection face       | signature-morphism category; relation interfaces       | law property tests; instance-table enumeration                                | arena, plus utilities     |
| levitation            | description codes; free-monad term pairs               | decidable code equality; content addressing                                   | arena                     |
| layout                | document DAG                                           | memoized measure; Pareto-frontier resolution                                  | arena, **not** this crate |

**Every row splits the same way, and that is the whole argument.** Identity and content live in a flat arena keyed by a dense identifier; the graph is a typed edge relation over those identifiers plus a small shared algorithm menu.
**No lane needs a graph that owns its node data**, which is why one crate can serve all of them without any of them surrendering their representation.

## The two layers

**The identity layer is arenas, and it owns its code.** Flat vector-backed stores with dense identifiers, and content hashes where identity is needed across time or across space.
Nothing third-party.

**The relation layer is this crate.** It owns the graph types over arena identifiers, the algorithm menu, the determinism policy and fingerprinting, and the two domain clients that are pure graph structure — the precedence DAG and the walk index — plus the variable-flow acyclicity gate's witness machinery.

**The dependency rule is directional and is part of the design.** The kernel and syntax arenas never depend on this crate, because their arenas are the identity layer.
The grammar, parser, polygraph, and reflection lanes are the ones permitted to consume it, and the module and incremental layers will join them.
The layout lane never does.
**Permitted is not the same as consuming**, and the rule is stated over lanes rather than crates: a lane may reach the substrate through another lane's re-export, which is what the parser does today.

## The adoption, and the arena-view pattern

petgraph is adopted **behind a façade**, with default features off, as the interim algorithm engine [@petgraph].

**Node and edge data stay in gandr arenas and are never copied into a petgraph graph**, except inside the one wrapper that requires a concrete graph.
The mechanism is a lightweight view: a caller exposes an edge source over its own storage, and this crate supplies an adapter implementing the traits petgraph's algorithms are generic over.

```rust
// the public boundary — crates/theory-graphs/src/lib.rs
pub trait EdgeSource
{
    type Successors<'successors>: Iterator<Item = NodeId> + 'successors
    where
        Self: 'successors;

    /// Returns the number of dense nodes in the graph.
    #[must_use]
    fn node_count(&self) -> NodeCount;

    /// Returns the outgoing successors for `node`.
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>;
}
```

The private adapter implements seven of petgraph's visit traits over any edge source — the graph base and its data and orientation, node counting, node indexability and compact indexability, and visit-map support — which is the set the menu below needs.

**The boundary is narrower than the design record drew it, in two ways that matter.** The successor iterator is a generic associated type rather than an opaque return, so an implementer chooses its own iterator without boxing; and the node count and node identifier are newtypes rather than bare integers, so a dense identifier cannot be confused with a count, a length, or a payload at a call site.

## The exit path

**The exit is designed in rather than promised.** The façade's public signatures use gandr types only: dense identifiers, owned vectors, and this crate's own error and witness types.
The adapter module that names petgraph is crate-private, so nothing downstream can come to depend on the engine by accident.

Replacing the engine is therefore a bounded, enumerated task: reimplement the fronted menu below against the same edge-source boundary, and delete one dependency line.

**The reversal trigger is recorded so that revisiting is a decision rather than a mood.** The adoption is revisited when the algorithm menu stabilizes **and** either a supply-chain event or a measured compile-time or footprint cost names the engine.
Until then the tested implementations are worth more than the closure costs, and no open item in this corpus proposes otherwise.

## The determinism policy

**No hash-iteration order may reach any result.** This began as a parser invariant and is a crate-wide rule, because a shared substrate that leaks iteration order leaks it into every lane at once.

* Internal adjacency is vector-backed and insertion-ordered, and construction order is part of the fingerprint.
* Results that the engine would return in hash-shaped containers are re-exposed as sorted rows keyed by node identifier.
* **Every tie in every enumeration is broken by a documented total order** — walk order, component member order, cycle witnesses.
* The harness runs the public surface across processes under perturbed allocation and asserts identical output.

**The harness deliberately owns a tiny adjacency source of its own rather than reaching into crate internals**, so what it certifies is the public projection a consumer actually observes.

## The algorithm menu

**Fronted from the engine**: topological sort, strongly connected components, cycle detection **with witness extraction**, reachability and its precomputed dense closures, transitive reduction and closure, immediate dominators, shortest-path lengths, all simple paths, and condensation.

**Implemented locally, from the first landing**: bounded breadth-first walk enumeration with filtered and ordered results, partition refinement, linear extension of a partial order, and adjacency fingerprinting.
The engine has neither of the first two.

**Condensation is the one member that needs a concrete graph**, so it is wrapped: the throwaway graph is built inside the façade and never escapes it.

**Partition refinement ships two relations, not one.** Coarsest strong bisimulation partitions **and** greatest forward simulation preorders, over unlabelled finite transition systems, with refinement using bitset splitter predecessor sets and simulation using a monotone relation-elimination fixpoint.

## The precedence DAG

The named precedence DAG is this substrate's first client, and it is the artifact both the operator machinery and the parser lane consume ([[../surface-language/operators]], [[../surface-language/grammar#The parsing calculus]]).

```rust
// crates/theory-graphs/src/prec.rs — the public surface, abbreviated
pub struct Prec(PrecIndex);
pub enum Assoc { Left, Right }
pub enum Bound<T> { /* the virtual endpoints, plus a carried node */ }

pub struct PrecSpec { /* names, edges, per-node associativity */ }
pub struct PrecCycle { /* the rejection witness produced at construction */ }
pub struct PrecDag { /* names, edges, associativity, reachability closure, fingerprint */ }

impl PrecDag {
    pub fn build(spec: &PrecSpec) -> Result<Self, PrecCycle>;
    pub fn lt(&self, l: Prec, r: Prec, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn gt(&self, l: Prec, r: Prec, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn eq(&self, l: Prec, r: Prec, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn comparable(&self, l: Prec, r: Prec) -> PrecedenceComparison;
    pub fn bound_lt(&self, l: Bound<Prec>, r: Bound<Prec>, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn bound_gt(&self, l: Bound<Prec>, r: Bound<Prec>, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn bound_eq(&self, l: Bound<Prec>, r: Bound<Prec>, a: Option<Assoc>) -> PrecedenceComparison;
    pub fn bound_comparable(&self, l: Bound<Prec>, r: Bound<Prec>) -> PrecedenceComparison;
    pub fn linear_extension(&self) -> Vec<Prec>;
    pub fn fingerprint(&self) -> Fingerprint;
}
```

**Strict precedence is strict reachability**, and associativity decides the reflexive pair.
**Incomparability is a first-class answer**, which is the substantive difference from a numeric precedence level and the reason `comparable` is a query rather than a derived fact.

**The four bound-aware methods are an addition the design record did not have.** The record's surface compared nodes; the tree compares `Bound<Prec>` as well, so the adjoined virtual endpoints participate in the same comparisons as ordinary nodes rather than being special-cased at every call site.

**Integer precedence tables are the degenerate chain DAG**, so a conventional numeric surface remains expressible without a second mechanism.
**The linear extension exists solely as a tooling projection** for the external grammar export, where the loss is invisible to the grouping-insensitive parity relation it feeds.

## The walk index

The walk relation is reified as a precomputed graph artifact rather than recomputed per query.

**What the walk machinery is.** Vertices are molded terminals plus a root; edges are generated by stepping a mold's regex zipper and by expanding sorts under precedence bounds; a walk alternates _swings_, which are chains of sort entries, with _stances_, which are terminals traversed — and **the traversed material is itself the completion** used to repair input.

The index is built once, at grammar build, under a total order, and the mold table is a projection of it.
`WalkIndex::build` canonicalizes insertion order and exposes ordered direct, transitive, mold, and fingerprint observations; the index's fingerprint folds into the precedence DAG's reuse key.

**The filters are ported from the reference implementation**: validity, minimality — no tile-sorted stance strictly between the endpoints' levels — and shortest-first enumeration under the documented tie-break.

## The parser lane's relationship to this substrate

**The parsing calculus itself is specified in the surface track and is not restated here** ([[../surface-language/grammar]]): the precedence-bounded grammar, the two build-time gates, the molder and melder pipeline, the obligation taxonomy in severity order, and the adaptations registry all live there.
What belongs here is the **substrate-level** content that lane rests on, and the analysis that justified generalizing a numeric precedence order to a DAG at all.

The reference work is Moon, Blinn, Porter and Omar's account of syntactic completions with material obligations [@moon-blinn-porter-omar-2025-tylr], read directly at this pass.

### What the paper abstracts, and where totality is genuinely used

**The paper already abstracts the order, which is what makes the DAG substitution cheap.** It takes the precedence set to be the naturals together with a minimum and a maximum reserved for internal use, equipped with per-sort relations that "abstract the details of associativity for each sort", and it stipulates that every ordinary level is comparable to both reserved endpoints.
Associativity is encoded on reflexive pairs: the paper's own example is that `5 ≺ₑ 5` encodes right-associativity for infix operators at level 5 of sort `e`.

This system substitutes DAG nodes for the naturals, reads the strict relation as strict reachability, and keeps the adjoined endpoints.

**Totality of the order is genuinely used in exactly one place, and it is not the place a reader would guess.** It is used in the _weights_ presentation: the elaboration rule for infix forms concludes with bounds computed by `min`, and grout injection folds with `max`.
For incomparable elements those operations do not exist.

**The implementation never computes them.** The reference parser decides enterability per form by bound checks against the current bounds, and derives bounds from regex-context nullability — never by arithmetic on levels.
**That last claim is about the reference implementation's source code, which is not in this tree and was not read for this document**; it is carried from the design record's own source read, and a reader who needs it load-bearing should re-derive it from that source rather than from here.

**So this system generalizes the checks, not the weights**, and where a formal statement wants a bound, the DAG-correct notion is an antichain of maximal lower or minimal upper bounds.
The implementation carries the check; the antichain reading is what a metatheory note would carry.

### The re-proof obligations

Each item below is an open obligation carried from the analysis, and none is discharged.
The numbering is **stable**: retiring one leaves its number unused.

#### graph-obligation-01

**The annotation-comparison coherence theorem restates over comparable pairs.** As published it asserts two equivalences between tile comparisons and the per-sort precedence relations.
Under a DAG, for an incomparable pair the claim becomes "no derivable comparison exists" — which is testable over the generated table, and is precisely the condition the melder's deferral path receives.

#### graph-obligation-02

**The totality theorem plausibly survives, via the adjoined minimum.** The paper's parser is sound and total over all inputs; grout terminals "behave like associative operators of loosest precedence within each sort", so they live at the minimum, which remains comparable to every node in the DAG.
The deferral rule's termination argument — that the recursive deferral concludes eventually with the base shift rule, thanks to the various grout forms — therefore keeps its base case.
**Incomparable tile pairs simply have no walks**, and the repair routes through grout completion, which is exactly where this system attaches its maximum-severity ambiguity obligation ([[../surface-language/grammar#The obligation taxonomy]]).

#### graph-obligation-03

**The valid-prefixes lemma and the push-soundness-and-totality lemma need their inductions checked under partial comparisons.** Both are proved for a total order; neither proof has been re-run for a DAG.

#### graph-obligation-04

**The metatheory of the generalization remains a design with named obligations rather than a proof.** The engineering gates — a totality property test over arbitrary input, a zero-obligation corpus parse, and statement-local obligation spans — hold regardless of how the obligations above resolve, which is why the lane was not blocked on them.

### One hazard was found in the reference source, and the tree resolved it

**The reference implementation deduplicates its walk search by sort alone.** Its justification, as the design record quotes it from that implementation's source, is that differently-molded same-sort nonterminals "will only have tighter precedence bounds and cannot access any nonterminals not already reachable".
**That source is not in this tree and was not read for this document**, so the hazard is recorded as the design record found it; what _is_ verified here is the tree's response to it, below.

**"Tighter" presumes comparability.** Under a DAG two molds of one sort can carry **incomparable** bounds, so sort-level deduplication could prune a reachable region.

**The tree keys the seen-set by sort _and_ bounds**, and went further than the analysis proposed.
The analysis asked for a property test comparing the two keyings; the crate ships the comparison as a public observation — a verdict type whose two inhabitants record whether the legacy sort-only closure reaches the same canonical rows as the production key.
That converts a one-off test into a standing, queryable check.

## The certificate and relations lanes

**One implementation of "find and report the cycle" serves three clients**, and consolidating them is the reason the crate carries a witness type rather than a boolean.

**Certificate composition is acyclicity-gated, and the gate is a call into this crate.** The composing code builds the variable-flow graph across composed seams — nodes are metavariable occurrences, edges are face-internal flows, with both directions for mixed variance — exposes it as an edge source, and calls the shared cycle witness **once**.
A cycle declines the composition and **the cycle is the diagnostic** ([[template-games#Certificates as cobordisms, in a virtual variant]], [[../metatheory#The certificate algebra]]).

**The coinductive-relations engine hosts its algorithm here and keeps its semantics.** Bisimulation and simulation checks are partition refinement over finitely generated transition graphs; the engine retains ownership of duality and the coalgebra semantics, and only the graph algorithm lives in the substrate.

**Overlap enumeration stays bespoke, deliberately.** Critical-pair enumeration at cut seams is unification over content-addressed command patterns, not graph search; forcing it through a graph interface would obscure it.
It uses this crate only for the bookkeeping that genuinely is graph-shaped — the dependency order of completion tasks, and the gate above.

**The reflection face draws on the enumeration utilities** so that its law property tests do not grow a private graph toolkit, a fourth quiet reimplementation being exactly what this substrate exists to prevent.

## The boundary: what the substrate is not

**The layout engine's document DAG stays out.** Resolving a document DAG by memoized measures and Pareto frontiers is dynamic programming indexed by document node and column, not graph search.
It shares the identity layer's conventions and must not depend on this crate; conversely nothing here should grow layout-shaped features.

**The same boundary holds for two structures that already exist.** The order-maintenance index is a specialized ancestry structure and is its own crate ([[incremental-pipeline]]); the syntax-tree diff is a longest-common-subsequence over child hashes and belongs to the syntax arena.

**This boundary is currently unenforced in one direction**, because no layout crate exists to violate it.
It is recorded as a constraint on the lane that will.

## Open dispositions

**The engine adoption stands and is not under review.** No open item proposes reversing it, and the reversal trigger has not fired.

**The duplicate transitive dependency is carried as an observation, not a defect.** Two versions of one hashing container are resolved beneath the engine and its index dependency.
Nothing here depends on their unification, and the fact is recorded so a later closure measurement is not surprised by it.

**The walk-index build cost has a measured history and no current statement.** The build was once found superlinear in mold count, against a fixed budget for the largest grammar.
The corpus records the parser's cold-reparse latency ([[../surface-language/grammar#Performance and reuse discipline]]) but **states no current index-build figure**, and none was measured for this document.

**The future clients named in the recurring-shape table are named, not scheduled.** The module import graph and the incremental pipeline's dependency graph would each consume the cycle-witness and topological-order members; neither does today, and neither has a landing condition recorded here.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot graph-core design record** in full — its recurring-shape table, its two-layer discipline, its adoption and exit path, its determinism policy and algorithm menu, its precedence-DAG and walk-index designs, its paper-and-source verification section with the re-proof obligations and the deduplication hazard, its certificate and relations placements, and its layout boundary.
2. **The tree**, for every as-built claim: `gandr-theory-graphs`'s eight modules and its manifest, its determinism binary, the workspace dependency table, the resolved dependency graph, and `gandr-theory-computads`'s `compose` module.
3. The **reference work** [@moon-blinn-porter-omar-2025-tylr], read directly at this pass rather than relied on through the design record — its two assumptions, its precedence-set definition and associativity device, its coherence and totality theorems, its two prefix and push lemmas, its elaboration and grout-injection weight computations, its grout precedence remark, its deferral-termination argument, and its description of the parsing rules as walks through the precedence relation graph.
4. The **corpus documents carrying this substrate's clients** — the surface grammar's parsing calculus and obligation taxonomy, the template and cobordism apparatus's account of the acyclicity gate, and the incremental pipeline's order-maintenance structure — which this document links rather than restates.

**Confidence, by class.**

* **High** — the as-built statements, each verified against the named module, manifest, or resolved dependency graph at write time, including the two divergences from the design record.
* **High** — the claims attributed to the reference **paper**, each checked against the held copy at write time rather than transcribed from the design record.
* **Carried, and marked at the claim** — the two claims about the reference **implementation's source code**, which is not in this tree and was not read; both are the design record's own source read, and both are flagged where they appear.
* **Medium** — the recurring-shape table's future rows, which state what a lane would need rather than what it has.
* **Absent, and marked** — any current walk-index build cost.

**One correction the direct read produced, recorded because a later reader would otherwise inherit it.** The design record presents several of its quotations from the reference work as verbatim, and their **content** is accurate, but the record silently renormalizes the paper's notation — writing the precedence relations with different symbols and different sort-subscript casing than the paper uses.
This document states the paper's results in the paper's own notation, and a reader comparing the two should expect the symbols to differ.
