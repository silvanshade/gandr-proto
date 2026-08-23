# gandr-kernel-check-memo

The dependency-free check-memo seam the certified kernel's checker consults, so a question already answered in this process is not answered twice.

## Current provision

- `CheckMemo<Support, Outcome>` is the seam: recall an answer for a support, or record one.
  Both types are the consumer's — this crate names no term, type, or identifier of its own, which is what lets the kernel depend on it without a cycle.
- `MemoActivity` is an associated constant, so a consumer's memo-handling branch is decided at compile time.
  Instantiated at `NullMemo` the branch is constant-false and monomorphization removes it, so the unmemoized path is the same code it was before the seam existed.
- `NullMemo` is the zero-sized memo that never answers.
  It is the differential's fresh side: the memoized and unmemoized paths are one function at two type parameters, not two implementations.
- `VerdictMemo` is the ordered in-memory store, plus `entry_count` and `supports` for measuring how many distinct questions a run actually answered.

This crate contains no eviction policy, no persistence, no wire encoding, and no soundness argument of its own.

## What a memo hit claims

Exactly one thing: **this process already computed this answer for this support**.
Not that the answer is right, not that the support was well formed, not that anything was validated.

A memo is sound only when its consumer's support is the whole input to the computation it indexes.
If two calls with equal supports could differ, the memo is a defect and nothing here can rescue it — the consumer owns that argument, and owes a differential that proves it.

## Using it

```toml
[dependencies]
gandr-kernel-check-memo.workspace = true
```

Define a support type carrying every input the indexed computation reads, give it `Ord`, and key on it.
Keep a memo's lifetime inside the scope its supports remain meaningful in: a support naming arena indices dies with that arena's stable prefix.

## Theoretical ideas relied on

Memoization over a support — the precise set of facts a computation reads — which is the incremental-computation contract in its simplest instance: a partition rather than an overlapping cover, so the assembly is a lookup rather than a gluing.
The static-dispatch shape follows the same null-object discipline as the sibling conversion-decision seam.

## Primary references

- Sebastian Erdweg, Oliver Bračevac, Edlira Kuci, Matthias Krebs, Mira Mezini, "A Co-contextual Formulation of Type Rules and Its Application to Incremental Type Checking", in _Proceedings of the ACM on Programming Languages_ (OOPSLA), 2015, 880–897.
  DOI [10.1145/2814270.2814277](https://doi.org/10.1145/2814270.2814277).
  The source of the finding this crate is shaped by: general-purpose incremental engines carry too much overhead at per-node granularity, so a node-grained memo is hand-rolled over the representation rather than engine-backed.
- Umut A. Acar, Guy E. Blelloch, Matthias Blume, Robert Harper, Kanat Tangwongsan, "An Experimental Analysis of Self-Adjusting Computation", in _ACM Transactions on Programming Languages and Systems_ 32(1), 2009, 3:1–3:53.
  DOI [10.1145/1596527.1596530](https://doi.org/10.1145/1596527.1596530).
