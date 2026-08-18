# gandr-kernel-strata

`gandr-kernel-strata` is the certified kernel's level oracle: the universe-level algebra over zero, level variables, successor, and binary maximum, held in always-canonical form, with an order oracle that returns checkable evidence in both directions.

It holds **levels only** — no terms, no types, and not even the universe rule, which is one call into the strict-order predicate and belongs to the kernel proper.
It is `no_std` over `core` and `alloc`, which is the sharpest form of the trusted base's dependency wall.

## Current provision

- The level type **is** the canonical form: a finite join of a constant part and per-variable offsets, dominated components removed and atoms keyed by variable.
  The smart constructors maintain it, so a non-canonical level is unrepresentable rather than merely rejected.
- An order oracle deciding `l ≤ m` over all valuations by domination, returning either a witness pairing each left atom with its dominating bound or a refutation carrying a concrete counter-valuation.
  Both have validators, so trust concentrates in the checkers and the decision procedure is self-incriminating under mutation.
- A landmark poset: a fixed declared set of order constraints over level variables, admitted by loop-checking as a dichotomy with evidence on each side — an admitted poset carrying an explicit homomorphism into the naturals, or a replayable pumping derivation showing none can exist.
- Entailment under an admitted poset, again with a forward-derivation witness or a countermodel, each with its validator.
  With no constraints declared, entailment agrees with the free-fragment oracle on every input, pinned by a property differential.

## Planned but absent

Declared constraints are variable-only, and query constants ride a pinned bottom generator internal to the encoding.
The crate deliberately refuses level inference and unification, generalization, displacement, constraint hypotheses beyond the declared landmark poset, `imax`, and cumulativity — these are exclusions from the stratification design rather than unbuilt steps.
A constant-time variable-plus-offset constructor remains a separate follow-up.

## Using it

`cargo nextest run -p gandr-kernel-strata --features=full` runs the crate suite, including the property differential that pins the no-constraints agreement between entailment and the free-fragment oracle.
Consumers reach it through the kernel and the core checker rather than directly.

## Theoretical ideas relied on

Universe stratification with levels as a separate certified layer; a sorted canonical form as the decision procedure for the word problem on the free fragment; loop-checking as a dichotomy with evidence on both sides; and the certificate posture at its smallest scale — an oracle that returns checkable evidence instead of a bare verdict, so the validators rather than the procedure carry the trust.

## Primary references

- Marc Bezem and Thierry Coquand, "Loop-checking and the uniform word problem for join-semilattices with an inflationary endomorphism", _Theoretical Computer Science_ 913, 2022, 1–7.
  DOI [10.1016/j.tcs.2022.01.017](https://doi.org/10.1016/j.tcs.2022.01.017).
- Per Martin-Löf, _Intuitionistic Type Theory_, Bibliopolis, 1984, ISBN 978-8870881052.
