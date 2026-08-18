# gandr-theory-computads

Fusion by completion over the polarized command IL: an oriented cell store, overlap enumeration at cut seams, budgeted completion, and replayable certificates.

The crate is additive.
It reads the core sequent IL vocabulary without modifying it, and it elaborates the reserved rule faces of the levitation descriptions into cells.

## What it provides

- A cell alphabet trait, with the sequent command-pattern alphabet as its first inhabitant.
- The command-pattern IL: the cut grammar with pattern metavariables, positions, and the reduction order.
- Substitution, one-sided matching, and two-sided unification.
- The cell store, with structural deduplication and insertion order.
- Overlap enumeration for confluence critical pairs and composition seams.
- Budgeted Knuth–Bendix and Squier completion that declines with a report rather than diverging.
- Replayable certificates, their causal event order, and a certified normal form whose equality is a decidable sound under-approximation of replay equality.
- Two-mode certificate composition, one unconditional and one gated on variable-flow acyclicity.
- Static pathway queries: goal-directed synthesis of the compressed derivations that can end in a target cell, computed without evaluating any state.
- Prototypes consumed by nothing: an atom-occurrence flow projection and a polarized footprint test, both beside the shift guard rather than replacing it.

## What is planned and absent

- A protocol face for pathway queries.
  The engine answers a caller holding the store; nothing carries a query in or renders a result out, so the diagnostics-side explanation query and completion-loop-driven exploration both wait on the interactive surface.
- The higher certificate integration and the staged game route.

## Using it

Build a store, insert cells, and ask the engine a question.

```rust
use gandr_theory_computads::pathway::{PathwayBudget, synthesize_pathways};

let outcome = synthesize_pathways(&store, &seed, target, &transitions, budget)?;
for pathway in outcome.pathways() {
    let compressed = pathway.normal_form.canonical_path();
}
```

A budget is a pair of ceilings.
Each ceiling is the maximum admitted rather than the point of failure.
Reaching one returns a decline carrying the pathways already found and the frontier the search stopped on, so the answer says where it stopped instead of truncating silently.
Nothing takes a frontier back yet, so continuing a declined query means re-asking with a larger budget.

## Theoretical ideas it relies on

Tracelets and tracelet composition; shift equivalence and the trace-monoid normal form; Knuth–Bendix and Squier completion; critical pairs and overlaps; double-pushout rewriting; polarized call-by-push-value and the sequent command language.

## Primary references

- Nicolas Behr, "Tracelets and Tracelet Analysis of Compositional Rewriting Systems", EPTCS 323 (2019), 44–71. doi:10.4204/EPTCS.323.4
- Nicolas Behr and Joachim Kock, "Tracelet Hopf Algebras and Decomposition Spaces (Extended Abstract)", EPTCS 372 (2022), 323–337. doi:10.4204/EPTCS.372.23
- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer, and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023. arXiv:2312.00429
