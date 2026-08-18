# gandr-theory-computads

Fusion by completion over the polarized command IL: an oriented cell store, overlap enumeration at cut seams, budgeted completion, and replayable certificates.

The crate is additive.
It reads the core sequent IL vocabulary without modifying it, and it elaborates the reserved rule faces of the levitation descriptions into cells.

## What it provides

- A cell alphabet trait, with the sequent command-pattern alphabet as its first inhabitant.
- The command-pattern IL: the cut grammar with pattern metavariables and positions.
- The reduction order completion orients critical pairs by: a node count guarded by hole occurrence, so substitution cannot reverse it, with a lexicographic path order deciding the ties a node count leaves.
- Substitution, one-sided matching, and two-sided unification.
- The cell store, with structural deduplication and insertion order.
- Overlap enumeration for confluence critical pairs and composition seams.
- Budgeted Knuth–Bendix and Squier completion that declines with a report rather than diverging.
- Replayable certificates, their causal event order, and a certified normal form whose equality is a decidable sound under-approximation of replay equality.
- Two-mode certificate composition, one unconditional and one gated on variable-flow acyclicity, the gate reading each endpoint pair of a seam hole rather than a flag over their union.
- The η cell a declaration licenses, minted at the same admission seam every other cell passes, in the contracting direction only and gated on the cut polarity its kind requires.
- Static pathway queries: goal-directed synthesis of the compressed derivations that can end in a target cell, computed without evaluating any state.
- Prototypes consumed by nothing: an atom-occurrence flow projection and a polarized footprint test, both beside the shift guard rather than replacing it.

## What is planned and absent

- A protocol face for pathway queries.
  The engine answers a caller holding the store; nothing carries a query in or renders a result out, so the diagnostics-side explanation query and completion-loop-driven exploration both wait on the interactive surface.
- The higher certificate integration and the staged game route.
- A hole identity that survives renaming across cells.
  The composition gate keys a seam hole by its name, so two cells that spell an unrelated hole alike land in one seam bucket, and every endpoint pair of a hole is drawn because which occurrence meets which is substitution data a certificate does not record.
  Both would close together under a slot-style identity carrying an explicit renaming across the reference.

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

Tracelets and tracelet composition; shift equivalence and the trace-monoid normal form; Knuth–Bendix and Squier completion; critical pairs and overlaps; recursive and lexicographic path orders; rewrite-rule formats headed by a destructor, and the contracting direction of an η law; double-pushout rewriting; polarized call-by-push-value and the sequent command language.

## Primary references

- Nicolas Behr, "Tracelets and Tracelet Analysis of Compositional Rewriting Systems", EPTCS 323 (2019), 44–71. doi:10.4204/EPTCS.323.4
- Nicolas Behr and Joachim Kock, "Tracelet Hopf Algebras and Decomposition Spaces (Extended Abstract)", EPTCS 372 (2022), 323–337. doi:10.4204/EPTCS.372.23
- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer, and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023. arXiv:2312.00429
- Nachum Dershowitz, "Orderings for Term-Rewriting Systems", _Theoretical Computer Science_ 17:3 (1982), 279–301. doi:10.1016/0304-3975(82)90026-3
- Thiago Felicissimo, "Generic Bidirectional Typing for Dependent Type Theories", _ESOP 2024_, LNCS 14577, 143–170. doi:10.1007/978-3-031-57262-3_6
- Thiago Felicissimo and Théo Winterhalter, "Confluence Techniques for Dependent Type Theory with Typed Conversion", *Proc.
  ACM Program.
  Lang.* 10, ICFP (2026), article 293. hal-05520710
