# gandr-theory-graphs

Two libraries under one name: a general dense-graph algorithm library, and the grammar structures gandr's surface tier is actually built on.

They share a crate because they share a substrate.
Both are stated over the same dense node vocabulary, and both fingerprint with the same fixed FNV accumulator rather than a process-random hasher, which is what makes their outputs reproducible across runs.

The crate exposes a small `EdgeSource` boundary and keeps its petgraph view adapter private.
No petgraph type appears in the public API, so a consumer of this crate does not inherit a dependency on that library's types.

## Current provision

**The algorithm half**, a general library over `EdgeSource`: topological sort, strongly connected components, condensation, immediate dominators, reachability, shortest path lengths, all simple paths, transitive reduction and closure, a cycle witness, the bisimulation and simulation refinements, and an adjacency fingerprint.
Every one of them is total: an invalid dense boundary surfaces as a typed validation error rather than a panic, and results contain only dense node identifiers.

**The grammar half**, parser semantics expressed over that library:

- The named precedence DAG.
  Precedence groups by name, associativity-controlled reflexive comparison, virtual bottom and root bounds, a deterministic topological order, and stable metadata-sensitive fingerprints.
  Building one rejects a cyclic tighter-to-looser relation with a closed deterministic witness.
- The declarative walk index, which computes the reachable mold set from exact ends and direct steps.

Determinism is a tested property rather than an intention.
The crate ships a probe binary whose standard output is the declared byte-level projection of the public results, and a harness perturbs scratch allocation to confirm the projection does not move.

## Planned but absent

Nothing is scheduled.
What is worth stating instead is the consumption gap, because it is the crate's most important as-built fact:

- The grammar half carries the surface tier.
  `gandr-surface-grammar` and `gandr-surface-parser` depend on the precedence DAG and the walk index throughout, and `gandr-surface-grammar` re-exports several of its types.
- The algorithm half has two external callers in the whole workspace: `condensation`, and the cycle witness that `gandr-theory-decomposition-spaces` uses to decline a directed composition.
  The remaining algorithms are exercised by this crate's own suites and by nothing else.

That is not an argument for deleting them, and it is not an argument for keeping them either.
It is the measurement a decision about this crate's shape would have to start from.

## Using it

Implement `EdgeSource` over your own dense adjacency, or build a precedence DAG from a specification.

```rust
use gandr_theory_graphs::PrecDag;
use gandr_theory_graphs::PrecSpec;
use gandr_theory_graphs::cycle_witness;

let mut spec = PrecSpec::default();
let additive = spec.insert("additive", None)?;
let multiplicative = spec.insert("multiplicative", None)?;
spec.add_edge(multiplicative, additive)?;
let dag = PrecDag::build(&spec)?;

let witness: Option<_> = cycle_witness(&graph)?;
```

Ties are broken by the smallest dense identifier everywhere a choice exists, including the topological sort's ready set.
That is what makes two runs over the same input produce byte-identical rows.

## Theoretical ideas relied on

Precedence graphs for mixfix parsing; Kahn's topological sort; Kosaraju's strongly-connected-component decomposition; partition refinement for strong bisimulation, and the greatest forward simulation preorder; transitive reduction and closure; dominator trees.

## Primary references

- Nils Anders Danielsson and Ulf Norell, _Parsing Mixfix Operators_, Implementation and Application of Functional Languages (IFL 2008), Lecture Notes in Computer Science 5836, 80–99, 2011, `doi:10.1007/978-3-642-24452-0_5` — the precedence-graph presentation the precedence DAG follows.
- A. B. Kahn, _Topological Sorting of Large Networks_, Communications of the ACM 5:11 (1962), 558–562, `doi:10.1145/368996.369025` — the topological sort, whose ready-set tie-break is where this crate's determinism is pinned.
- Robert Paige and Robert E. Tarjan, _Three Partition Refinement Algorithms_, SIAM Journal on Computing 16:6 (1987), 973–989, `doi:10.1137/0216062` — the refinement discipline behind the coarsest strong bisimulation partition.
