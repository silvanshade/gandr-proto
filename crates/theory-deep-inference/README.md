# gandr-theory-deep-inference

The identity relations on derivations: when two derivations that fire the same cells in different orders are the same derivation, and what structure decides it.

Deep inference is where the relations come from.
Adjacent independent steps permute; the permutations quotient a derivation to a canonical form; and the atom-occurrence flow of a derivation is a projection through that quotient rather than a finer invariant.
None of them is the semantic oracle — replay, in `gandr-theory-coherent-resolutions`, is.

The causal order and the normal form are mutually dependent, and the crate boundary encloses both: an event's canonical key digests its causal past, and the causal order is read back to schedule the normal form.

## What it provides

- The earned shift-equivalence witness: two adjacent applications at disjoint positions with trivial overlap are one composite transformation, granted per pair against the decided guard and carrying the convexity conjunct's discharge as a certificate rather than a recomputed sweep.
  This is the crate's single independence relation.
- The finite event partial order of a recorded derivation: its events, the dependence edges the guard decides, the causal precedence order, the layering, and the exchange witness carrying one sequentialization to another as licensed adjacent transpositions.
- The certificate normal form: unique primitive factorization by content address, integer-graded multiplicities, and a causal canonical schedule, whose equality is a decidable sound under-approximation of replay-equality.
  Normal-form-equal implies replay-equal; the converse is never claimed.
- The atom-occurrence flow projection over certificate legs, which witnesses the shift quotient rather than certificate identity.
- A polarized footprint prototype beside the shift guard, measuring where a polarized reading would license commutations the guard refuses.

The flow projection and the footprint test are prototypes and neither has a consumer.
Both say so at their own module heads, and neither replaces the guard.

## What is planned and absent

- The trace seam and spinal duplication from the sharing programme.
- Any consumer for the flow projection or the footprint test.

## Using it

Ask whether two adjacent applications commute, and read the obstruction when they do not.

```rust
use gandr_theory_deep_inference::derive_shift_equivalence;

let witness = derive_shift_equivalence(&store, first, second, convexity)?;
```

The content addresses this crate computes are process-local.
Nothing may persist or transmit one; the durable identity is minted at the transport boundary in `gandr-theory-decomposition-spaces`.

## Theoretical ideas it relies on

Deep inference and atomic flows; permutation of inference steps; trace monoids over an independence relation; causal orders and their linear extensions; content-addressed canonical keys.

## Primary references

- Alessio Guglielmi and Tom Gundersen, "Normalisation Control in Deep Inference via Atomic Flows", Logical Methods in Computer Science, Volume 4, Issue 1 (2008).
  `doi:10.2168/LMCS-4(1:9)2008`
- Nicolas Behr, "Tracelets and Tracelet Analysis of Compositional Rewriting Systems", Electronic Proceedings in Theoretical Computer Science 323 (2019), 44–71.
  `doi:10.4204/EPTCS.323.4`
