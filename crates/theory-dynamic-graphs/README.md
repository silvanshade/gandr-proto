# gandr-theory-dynamic-graphs

Graph invariants kept current as edges arrive, instead of recomputed from scratch after each one.

Two structures live here and they answer the same question at two strengths.
`AcyclicityMaintenance` keeps a topological order of the edges it has admitted, so it can say whether a new edge closes a cycle without traversing the graph.
`PotentialMaintenance` keeps a valuation satisfying offset-carrying constraints of the form `value(target) >= value(source) + offset`, which is the same question once the edges carry arithmetic.

The crate exists because the second question is the one a universe-level constraint solver asks, and the first is the cheap approximation of it.
Knowing exactly where the approximation stops holding is the point, and the crate's tests measure that rather than assert it.

## Current provision

- `AcyclicityMaintenance`, a directed graph whose topological order is maintained under edge insertion.
  `insert_edge` answers `Admitted` when the standing order already places the source before the target, `AdmittedAfterRepair` when the order had to change, and `Refused` with the cycle the edge would have closed.
  A refused edge is not recorded and leaves the structure untouched.
- `PotentialMaintenance`, a constraint system whose valuation is maintained under constraint insertion.
  `insert_constraint` answers `Satisfied`, `SatisfiedAfterRaise`, or `Refuted` with the positive-weight cycle that makes the system unsatisfiable.
  A refuted constraint is rolled back exactly, so the valuation is left byte-for-byte as it was found.
- `order_is_topological` and `valuation_is_feasible`, each structure's own invariant offered as a query rather than a promise.
  The differential and the probe both check them at every step, and each has a seeded corruption proving the query notices.
- Totality.
  Every failure surfaces as a typed error; no path panics, and no search recurses.

Refusal carries `gandr_theory_graphs::CycleWitness` — the same type the batch `cycle_witness` returns.
That is deliberate: it makes an incremental verdict and a batch verdict the same kind of thing, so the differential compares them directly instead of translating between two vocabularies.

## Planned but absent

- A consumer.
  Nothing in the tree yet feeds these structures an edge stream; the acyclicity gate that motivated the work builds and discards its graph inside a single call and has no insertion stream to offer.
  The intended consumer is a level-constraint solver, whose constraints genuinely accumulate.
- Edge deletion.
  Both structures are insertion-only, which is what their consumers need and what keeps the repair bound meaningful.
  Deletion needs a different analysis and is not attempted.
- A two-level order-maintenance refinement.
  Insertion into the underlying order is `O(log² n)` amortized; the repair bound is stated against the affected region and inherits that factor.

## Using it

Offer edges one at a time and read the verdict.

```rust
use gandr_theory_dynamic_graphs::AcyclicityMaintenance;
use gandr_theory_dynamic_graphs::EdgeVerdict;
use gandr_theory_graphs::EdgeId;
use gandr_theory_graphs::NodeId;

let mut graph = AcyclicityMaintenance::new()?;
graph.insert_edge(EdgeId::new(NodeId::from(0), NodeId::from(1)))?;
graph.insert_edge(EdgeId::new(NodeId::from(1), NodeId::from(2)))?;
let verdict = graph.insert_edge(EdgeId::new(NodeId::from(2), NodeId::from(0)))?;
assert!(matches!(verdict, EdgeVerdict::Refused(_)));
```

Nodes are created on demand, so no capacity has to be declared up front.
`telemetry` reports the work done, which is what prices the structure against a batch recheck.

## Theoretical ideas relied on

Dynamic topological sort under edge insertion; the affected-region bound that makes the repair local; order maintenance as the position structure the order is read from; difference constraints and their feasible potentials; positive-weight cycles as the obstruction to a feasible valuation.

## Primary references

- David J. Pearce and Paul H. J. Kelly, _A Dynamic Topological Sort Algorithm for Directed Acyclic Graphs_, ACM Journal of Experimental Algorithmics, volume 11, 2007, `doi:10.1145/1187436.1210590` — the insertion algorithm, its two bounded searches, and the affected-region cost analysis this crate implements.
- Bowen Alpern, Roger Hoover, Barry K. Rosen, Peter F. Sweeney and F. Kenneth Zadeck, _Incremental Evaluation of Computational Circuits_, Symposium on Discrete Algorithms (SODA), 1990, ISBN 0-89871-251-3 — the priority-bounded incremental reordering the above refines.
- Michael L. Fredman and Robert Endre Tarjan, _Fibonacci Heaps and Their Uses in Improved Network Optimization Algorithms_, Journal of the ACM, volume 34, number 3, 1987, `doi:10.1145/28869.28874` — the feasible-potential reformulation of difference constraints the valuation structure maintains.
- Thomas H. Cormen, Charles E. Leiserson, Ronald L. Rivest and Clifford Stein, _Introduction to Algorithms_, fourth edition, MIT Press, 2022, ISBN 978-0-262-04630-5, chapter 22 — difference constraints, feasible valuations, and the positive-weight-cycle obstruction, in the textbook statement.
- Paul F. Dietz and Daniel D. Sleator, _Two Algorithms for Maintaining Order in a List_, Symposium on Theory of Computing (STOC), 1987, `doi:10.1145/28395.28434` — the order-maintenance problem `gandr-theory-orders` solves and this crate reads positions from.
