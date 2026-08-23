# gandr-theory-coherent-resolutions

Coherent resolution of a cell rewriting system: firing a cell, finding the critical pairs, joining them with a replayable certificate, and completing the system until the pairs are exhausted or the budget declines.

The modules are one construction read in order, and the crate is generic over the cell alphabet.
Overlap enumeration and the certificates are mutually dependent — a certificate carries the branching it joins, and the support index memoizes independence over certificates as well as cells — so the crate boundary encloses both rather than cutting between them.

## Current provision

- One cell applied at one position, and budgeted normalization, both under the alphabet's own firing discipline.
- The multi-sum overlap enumerator: confluence critical pairs and composition overlaps, with the seam data each carries.
- The memoized support relation the layers above query for independence.
- Replayable coherence certificates and the derived fused cell, with replay-equivalence as the identity criterion and observable step evidence.
- Budgeted Knuth–Bendix and Squier completion that declines with a report rather than diverging.
- An in-process replay memo: per-step outcome reuse keyed by the step's full support — the resolved cell's content, the position, and the input term — behind an opt-in replay path that leaves the default replay unchanged.

## Planned but absent

- Enumeration of match-and-pre-critical pairs for the citable convex confluence route.
  The completion loop cites the result; the enumerator that would make it a decision procedure here is not built.

## Using it

Complete a store within a budget, and read the outcome rather than assuming one.
Completion consumes the store and hands it back inside the outcome.

```rust
use gandr_theory_coherent_resolutions::{CompletionBudget, complete};

let outcome = complete(
    store,
    CompletionBudget::new(max_steps, max_cells, norm_budget),
);
```

A budget is three ceilings — critical pairs processed, cells the store may reach, and steps per normalization — each the maximum admitted rather than the point of failure.
Reaching one returns a decline carrying its report and the pending work, so the answer says where it stopped instead of truncating silently.

Replay a certificate through a memo when the same steps recur — across repeated replays, across append-only store growth, or across certificates that share steps.
The memo is in-process only and carries no authority: it is an optimization, and the plain `replay` remains the answer of record.

```rust
use gandr_theory_coherent_resolutions::ReplayMemo;

let mut memo = ReplayMemo::new();
let verdict = tracelet.replay_memoized(&store, &mut memo);
```

## Theoretical ideas relied on

Knuth–Bendix completion; Squier's coherence theorem and coherent presentations; critical branchings and their joins; tracelets as replayable certificates; double-pushout rewriting; support-keyed memoization, where a step's outcome is reused exactly when everything it depends on recurs.

## Primary references

- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023.
  `doi:10.1017/9781009498968`, arXiv:2312.00429
- Nicolas Behr, "Tracelets and Tracelet Analysis of Compositional Rewriting Systems", Electronic Proceedings in Theoretical Computer Science 323 (2019), 44–71.
  `doi:10.4204/EPTCS.323.4`
