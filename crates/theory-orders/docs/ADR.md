# Architecture Decisions

Crate-local decisions for `gandr-theory-orders` (the order structure the A2 incremental pipeline is missing).
These extend, and never override, the design corpus in `docs/gandr/` (notably `spec/incremental-pipeline.md` §7, the Porter order-maintenance disposition) and the global ADR log.

## current

### Single-level list-labeling over a fixed `[0, 2^62)` universe

Decision: maintain a total order by assigning each element an integer `label`, strictly increasing in list order, drawn from a fixed `u64` universe of `2^62` labels; comparison is one `u64` comparison.

Rationale: comparison in O(1) is the headline operation the incremental engine needs (Porter's pre/post-order interval containment, the dirty-step priority queue).
A single-level labeling scheme (Itai–Konheim–Rodeh 1981; Bender, Cole, Demaine, Farach-Colton & Zito 2002 — the corpus's reference `[6]`) delivers it with a simple, fully inspectable relabel rule.
`2^62` keeps the relabel arithmetic clear of `u64::MAX` and makes the universe `≥ n²` for every realistic length, which is what the amortized analysis needs.

Consequences:

* Insertion is O(log² n) amortized (the uniform-half-density relabel rule), O(1) comparison.
  The two-level O(1)-amortized refinement is deferred (`OPTIMIZATION.md`).
* Labels are an internal representation, never exposed; `Pos` is opaque.

### Minimal aligned-window relabel with a density cap for totality

Decision: when an insertion's neighbour gap is exhausted, relabel the smallest power-of-two-aligned label window containing the insertion point that is at most half full, redistributing its elements (and the new one) evenly; if even the whole-universe window is too full, return `OrderError::CapacityExhausted`.

Rationale: the half-full threshold guarantees the redistributed labels are distinct, strictly increasing, and strictly inside the window, preserving global order against the un-relabeled neighbours.
The whole-universe window is at most half full for any length below `2^61`, far above the `u32` handle-index ceiling that binds first — so a relabel always succeeds in practice, and the structure is **total**: capacity exhaustion is a typed error, never a panic.
This honours the workspace no-panic lint wall (`docs/workflow/rust.md`).

Consequence: all arithmetic is `checked_*` / `saturating_*` / `midpoint` / `checked_shl` / `checked_shr`; the rare defensive failure arms keep the surface total at the cost of some uncovered defensive regions (`METRICS.md`).

### Generation- and structure-checked opaque handles

Decision: a `Pos` carries `{structure_id, index, generation}`.
Freeing a slot bumps its generation; each structure draws a process-unique `structure_id` from a global atomic.
`resolve` rejects a handle whose structure, slot, or generation does not match.

Rationale: a `Pos` must stay valid as _other_ elements are inserted and removed, but a handle to a removed element must not silently alias a later element that reuses the slot, and a handle from a different structure must not misresolve.
Generation + structure id give both, memory-safely, without exposing arena indices.

Consequence: handles are `Copy` and 16 bytes.
A stale handle after exactly `2^32` reuses of the _same_ slot could alias — documented and far beyond realistic use.

### Generic payload + a thin interval layer; nothing else

Decision: `OrderMaintenance<T>` carries a caller payload; the only structure built on top in this crate is `Interval` + `interval_contains` (pre/post-order containment).

Rationale: the order structure is generic and reusable.
The interval query is the one O(1) consumer of the order that is purely a function of it.
Everything else that the Porter synthesis layers on order maintenance — binding pointers / lowest-binder lookup, per-node mark/dirty-bit layout, the tree-sitter node-identity resync — has its own invariants and belongs in its own brick.

## designed direction

### Two-level structure for O(1)-amortized insertion

The single-level scheme's O(log² n) amortized insertion can be improved to O(1) amortized with the two-level structure of Bender et al. (a top list of O(log n) sublists).
Deferred until profiling on real edit traces shows insertion cost matters; tracked in `OPTIMIZATION.md`.

### Tree-sitter node-identity resync (out of scope here)

Wiring order points to tree-sitter node identity across a reparse — when byte ranges shift but subtrees are unchanged — is a real, unsolved sync problem.
It lands where this structure meets the parser bridge, not here; this crate's handles are deliberately reparse-agnostic.

## open decision

### `#[no_panic]` on `cmp`

The grade leaf ops in `gandr-core` carry `#[no_panic]` behind a release smoke as belt-and-suspenders.
`cmp` is the analogous hot leaf here, but `#[no_panic]` does not apply to a generic method without a concrete instantiation seam; whether to add a monomorphic smoke is deferred (`OPTIMIZATION.md`).
The lint wall already forbids every panic source in production.
