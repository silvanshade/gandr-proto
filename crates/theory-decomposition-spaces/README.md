# gandr-theory-decomposition-spaces

The algebra of derivation certificates: composing them, decomposing them, and minting a durable identity for one step of one.

A decomposition space is the structure that answers, of an arrow, in how many ways it factors as a composite.
That is what this crate computes over the certificates `gandr-theory-coherent-resolutions` produces.
It is also the home the tracelet Hopf-algebra work lands into as that work arrives.

## Current provision

- Two-mode certificate composition: an unconditional coherence lane, and a directed lane gated by variable-flow acyclicity across the composed seam that declines with the cycle as its diagnostic.
  Composition is binary, and composing a family means folding it — a fold whose verdict does not factor through the pairwise verdicts of the certificates in it, so a chain whose every adjacent pair is admitted may still be declined.
- Static pathway queries: which compressed derivations can end in a target cell, grown backwards from the target and compressed to normal form, evaluating nothing.
  The target-occurs-only-last condition is decided as an order property, because the rearrangements of a derivation are exactly the linear extensions of its causal order.
- The canonical step encoding and the durable step identity it frames — the one boundary at which a process-local content address becomes something that may be persisted or transmitted.

The pathway engine's positive verdict is relative to what the shift guard can discharge: a refutation is sound, an acceptance over-approximates.

## Planned but absent

- A protocol face for pathway queries.
  The engine answers a caller holding the store; nothing carries a query in or renders a result out, so the diagnostics-side explanation query and completion-loop-driven exploration both wait on the interactive surface.
- The tracelet Hopf algebra itself: the comultiplication, the antipode-as-rollback direction, and a length-graded layered schedule.

## Using it

Compose two certificates, or ask what can reach a target.

```rust
use gandr_theory_decomposition_spaces::compose_directed;
use gandr_theory_decomposition_spaces::pathway::synthesize_pathways;

let composite = compose_directed(&left, &right, &store)?;
let outcome = synthesize_pathways(&store, &seed, target, &transitions, budget)?;
```

A pathway budget is a pair of ceilings, each the maximum admitted rather than the point of failure.
Reaching one returns a decline carrying the pathways already found and the frontier the search stopped on.
Nothing takes a frontier back yet, so continuing a declined query means re-asking with a larger budget.

## Theoretical ideas relied on

Decomposition spaces and the 2-Segal condition; incidence coalgebras; tracelet Hopf algebras; certificate composition and variable-flow acyclicity; content-addressed transport identity.

## Primary references

- Imma Gálvez-Carrillo, Joachim Kock and Andrew Tonks, "Decomposition Spaces, Incidence Algebras and Möbius Inversion I: Basic Theory", Advances in Mathematics 331 (2018), 952–1015.
  `doi:10.1016/j.aim.2018.03.016`, arXiv:1512.07573
- Nicolas Behr and Joachim Kock, "Tracelet Hopf Algebras and Decomposition Spaces (Extended Abstract)", Electronic Proceedings in Theoretical Computer Science 372 (2022), 323–337.
  `doi:10.4204/EPTCS.372.23`
