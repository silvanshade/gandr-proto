# gandr-core-unify

Predictable-fragment unification for the gandr core: a solver whose failures are named rather than silent.

A problem is a list of equations between core terms, some of whose holes the caller has nominated as metavariables.
The answer is a certificate: a most general substitution, a residual of equations the solver declined to decide, and a claim the caller re-checks by substituting and asking the ordinary conversion engine.

Two commitments shape everything here.
**Most general or nothing** — a solution less general than the problem admits is user-visible unreliability even when every answer is correct, because the difference surfaces later and somewhere else.
**Postpone, never guess** — every place the solver stops has its own reason, so a caller reading one knows what would have to change and a test reading one pins the boundary rather than the absence of an answer.

**Refutation and refusal stay distinct** — `Verdict::Refuted` carries evidence that no substitution can satisfy the constraint, while `Verdict::Refused` records only that the current conversion relation declined a metavariable-free constraint.
A fuller environment or budget may decide a refusal.

The equational theory is not this crate's.
Every rule the solver applies is one `gandr-core-nbe` already decides, which is what makes the substitute-and-re-check evidence meaningful and what stops a second definitional equality growing here.

## Current provision

- The fragment: Miller patterns, function eta and lazy-pair eta, meta splitting on a projection-led spine, `Return` and same-constructor congruence over the positive structure, rigid-rigid decomposition over a shared head, Miller's intersection rule, and flex-flex where one spine covers the other.
- `frag` — spine classification: whether an occurrence is a pattern, and if not, which named reason blocks it.
- `meta` — the metavariable store and the substitution being built.
- `solve` — the solver machine itself.
- `certify` — certificate construction and the re-check that substitutes a solution and asks conversion.
- `Verdict::Refuted` and `Verdict::Refused` are separate outcomes, so consumers that require evidence-stable refutations cannot accept a conversion refusal.
- `scan` — occurrence scanning, private to the crate.

## Planned but absent

- No crate depends on this one yet: the elaborator seam that will drive it is unbuilt, so the solver is exercised by its own tests alone.
- Outside the fragment, each with a named reason: a projection after an application, a sequenced spine, a constructor in a spine, a non-pattern spine, and the rules the conversion relation itself does not decide.

## Using it

```toml
[dependencies]
gandr-core-unify.workspace = true
```

```rust
use gandr_core_unify::Certificate;
```

A caller nominates metavariables among existing holes rather than introducing a syntactic former, so the terms it hands in are ordinary core terms and the answer is checked with the ordinary conversion relation.

## Theoretical ideas relied on

- **Pattern unification** — the decidable fragment in which a metavariable applied to a spine of distinct bound variables has a most general solution, and the intersection rule for two occurrences of one metavariable.
- **Dynamic pattern unification** — deciding pattern-hood at solving time rather than by a syntactic restriction, which is what lets an equation leave the fragment and be postponed rather than rejected.
- **Eta laws as conversion, not as solver rules** — function eta and lazy-pair eta are decided by applying or projecting both sides, so the solver inherits them instead of restating them.
- **Meta splitting** — replacing a metavariable whose spine leads with a projection by a pair of fresh ones, which lazy-pair eta is what makes most general rather than a choice.
- **Certificates re-checked by replay** — the answer carries evidence a consumer verifies for itself, so the solver is not in the trusted position.

## Primary references

- Andreas Abel and Brigitte Pientka, "Higher-Order Dynamic Pattern Unification for Dependent Types and Records", in _Proceedings of the 10th International Conference on Typed Lambda Calculi and Applications_ (TLCA 2011), Springer, 2011, 10–26.
  ISBN 978-3-642-21690-9.
- Ambroise Lafont and Neel Krishnaswami, "Semantics of Pattern Unification", _Journal of Functional Programming_ 35, 2026, e26.
  DOI [10.1017/S0956796825100130](https://doi.org/10.1017/S0956796825100130).
- Adam Gundry and Conor McBride, "A Tutorial Implementation of Dynamic Pattern Unification", unpublished draft, 2013.
  Locator unverified: this repository holds no reference register, and no stable identifier was confirmed against a publisher record at the time of writing.
- Christian Urban, Andrew M. Pitts, and Murdoch J. Gabbay, "Nominal Unification", _Theoretical Computer Science_ 323, 2004, 473–497.
  DOI [10.1016/j.tcs.2004.06.016](https://doi.org/10.1016/j.tcs.2004.06.016).
