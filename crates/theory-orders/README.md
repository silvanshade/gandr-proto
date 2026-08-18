# gandr-theory-orders

An order-maintenance structure: a collection held in a total order, where an element can be inserted beside another or removed, and any two elements can be compared in constant time.

Constant-time comparison is the operation the incremental pipeline needs and the reason this crate exists.
Pre- and post-order timestamps over such a structure decide in O(1) whether one term encloses another, which is what drives the dirty-step priority queue an incremental checker schedules from.

The crate is the order structure and the containment query built directly on it, and nothing else.
The lowest-enclosing-binder lookup, the per-node mark and dirty-bit layout, and any binding of order points to the concrete-syntax tree's reproducible identity are separate pieces that _consume_ this one.

## Current provision

- `OrderMaintenance<T>`, a payload-carrying total order over opaque handles.
  Comparison is one integer comparison.
  Insertion at either end or beside an existing element, removal, navigation, and the ordinary queries complete the surface.
- `Interval` and `interval_contains`, the pre/post-order containment test in constant time.
- `Pos`, the handle: opaque, generation-checked and structure-checked.
  A handle to a removed element or a handle from a different structure is detected rather than silently aliasing an unrelated element, and a slot whose generation counter is exhausted is retired instead of wrapping.
- Totality.
  Capacity exhaustion surfaces as a typed error rather than a panic, and construction is itself fallible rather than wrapping the process-wide structure-id counter.

The implementation is single-level list-labeling over a fixed label universe.
Insertion takes the midpoint label of the gap between its neighbours when one exists; when the gap is exhausted it relabels the smallest power-of-two-aligned window around the insertion point that is at most half full and redistributes evenly.
The density cap keeps that window always sparse enough for the relabel to succeed.
Insertion is therefore O(log² n) amortized.

## Planned but absent

- The two-level refinement that would make insertion O(1) amortized.
  The single-level scheme is deliberate: comparison is the operation the consumer needs, and the simple relabel rule is fully inspectable.
- The byte-range resync against the syntax tree.
  Whether an adapter from this structure to concrete-syntax identity pays for itself waits on a dirty-frontier consumer.

`gandr-core-incremental` and `gandr-surface-engine`'s edit path are the consumers today.

## Using it

Build the order, insert relative to what is already there, and compare.

```rust
use gandr_theory_orders::OrderMaintenance;

let mut order: OrderMaintenance<()> = OrderMaintenance::new()?;
let first = order.push_back(())?;
let second = order.insert_after(first, ())?;
let ordering = order.cmp(first, second);
```

Labels are internal and never exposed, so a caller cannot depend on the numeric encoding a relabel is free to change.
`Pos` is the only identity, and it stays valid across relabels.

## Theoretical ideas relied on

The order-maintenance problem; list-labeling over a sparse integer universe; pre- and post-order timestamp intervals as a constant-time containment test; incremental bidirectional typing driven by a dirty-step priority queue.

## Primary references

- Dietz and Sleator, _Two Algorithms for Maintaining Order in a List_, Symposium on Theory of Computing (STOC), 1987, `doi:10.1145/28395.28434` — the problem statement this crate solves.
- Itai, Konheim and Rodeh, _A Sparse Table Implementation of Priority Queues_, Automata, Languages and Programming (ICALP), 1981, `doi:10.1007/3-540-10843-2_34` — the list-labeling scheme the relabel rule here refines.
- Bender, Cole, Demaine, Farach-Colton and Zito, _Two Simplified Algorithms for Maintaining Order in a List_, European Symposium on Algorithms (ESA), 2002, `doi:10.1007/3-540-45749-6_17` — the simplified algorithms and the amortized analysis the density cap is taken from.
- Porter, Kirisame, Wei, Panchekha and Omar, _Incremental Bidirectional Typing via Order Maintenance_, 2025, arXiv:2504.08946 — the consumer this crate was commissioned for, and the source of the pre/post-order interval test.
