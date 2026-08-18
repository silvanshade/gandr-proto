# gandr-theory-nominal-automata

The shared fabric for machine-minted names: one allocation discipline producing sort-tagged atoms, and the automaton layer that reclaims them.

gandr mints machine-internal names in several independent places, and each place used to carry its own counter, its own format, and its own separate re-proof that the names it minted were distinct.
This crate is the substrate those name-spaces are being consolidated onto.
One allocator per sort per pass gives exactly the guarantee its consumers need: every atom one allocator mints is distinct from every other atom that allocator has minted.

The consolidation is the structural answer to a measured failure.
Keying a continuation environment by the source binder name is dynamic scoping, and it once made a well-typed term loop to a stuck step limit; the fix is to rename each capture to a fresh machine-unique name.
Folding the ad-hoc counters into one audited primitive replaces a per-site re-proving of distinctness with a single one.

The crate is generic over the sort and carries no gandr vocabulary. gandr supplies its own sort enumeration.

## Current provision

- `Gensym`, the monotone allocator.
  Identities come from a strictly increasing counter and are never recycled, so they leak harmlessly on a consumer's backtrack rather than being reissued.
  That is precisely why distinctness holds regardless of the consumer's speculative execution.
  Exhaustion mints nothing and leaves the counter unchanged, so it can never duplicate an identity.
  Cloning forks the mint sequence: two clones thereafter mint the same identities, and the backtrack safety is about rewinding one allocator rather than duplicating it.
- `Atom`, `Sort` and `Unifiability`, the sort-tagged atom space.
  `Sort::is_unifiable` splits atoms into pure names, which are only minted, compared, freshness-tested and eventually permuted, and substitutable unknowns, which enter a substitution's domain.
- The automaton layer: three model inventories with their finitary orbit-level representations, the membership decision procedure, the name-dropping construction, a deliberately boring finite-alphabet back-end for use after the nominal reduction, and a decision-procedure catalogue carrying each entry's source theorem.

**The atom-versus-variable sort boundary is load-bearing rather than cosmetic.** Keeping unification variables in a disjoint sort is what preserves unitary most-general unifiers; collapsing them into the atom pool drops the problem into equivariant unification.

## Planned but absent

**Only the allocator half is consumed.** `Gensym`, `Sort` and `Unifiability` are the three items other crates name.
The whole automaton layer has no caller in the workspace.
The ratio is stated here rather than left for a reader to discover: the two layers are kept in one crate deliberately, because the automaton half is where the resource-lifecycle work lands, and splitting it now would buy a tidier number at the price of a crate, a name, and a category argument.

The consolidation is itself partial, and the tree is the record of how far it got.
The lowerer's hoist binders and hole addresses mint here.
The typing machine's continuation keys do not; they are still a formatted string off a bare counter in `gandr-core-sequent`.
Sealing minted a third discipline deliberately, because an opaque ascription's abstract-type identity has to be a function of the sealing site so an admission point can re-mint and compare, which a monotone allocator cannot offer.
Two of the sort vocabulary's six roles are constructed today.

Reserved behind the current API until their consuming features land:

- The swapping-list permutation and its action and freshness discipline, which arrive with the equivariance metatheory.
- The finite-support skin over which scope sets and contextual substitution are hosted, which arrives with macros and widens the sort trait so an atom can key the support container.
- Nominal unification, which arrives with the real solver.
- The bounded-alphabet restriction and the classical back-end that every remaining catalogue procedure bottlenecks on, the tree-automaton top-down run, determinization, and the Kleene expression compiler.

## Using it

Mint atoms from one allocator per sort per pass.

```rust
use gandr_theory_nominal_automata::Gensym;

let mut gensym: Gensym<MySort> = Gensym::new(MySort::HoistBinder);
let atom = gensym.fresh()?;
```

Do not clone an allocator to get a second name source.
A clone continues from the same counter and mints colliding identities, which is the one way to lose the guarantee this crate exists to provide.

## Theoretical ideas relied on

Nominal sets and atom permutation; freshness and finite support; nominal unification and its unitary most-general unifiers, as against equivariant unification; nominal automata with name binding, allocation and deallocation; name dropping as a reduction to a bounded alphabet; orbit-finite state sets represented by control points over partial injective register stores.

## Primary references

- Christian Urban, Andrew M. Pitts and Murdoch J. Gabbay, _Nominal Unification_, Theoretical Computer Science, 2004, `doi:10.1016/j.tcs.2004.06.016` — the unitary most-general unifiers that the atom-versus-variable sort boundary exists to preserve.
- Lutz Schröder, Dexter Kozen, Stefan Milius and Thorsten Wißmann, _Nominal Automata with Name Binding_, Foundations of Software Science and Computation Structures (FoSSaCS), 2017, arXiv:1603.01455 — the regular nondeterministic nominal automaton model and the inclusion result the catalogue records.
- Simon Prucker and Lutz Schröder, _Nominal Tree Automata with Name Allocation_, International Conference on Concurrency Theory (CONCUR), 2024, `doi:10.4230/LIPIcs.CONCUR.2024.35` — the tree and term model, and the name-dropping construction on it.
- Simon Prucker, Stefan Milius and Lutz Schröder, _Nominal Automata with Name Deallocation_, 2026, arXiv:2603.24468 — the deallocation model this crate's automaton layer is built to, and the source of its name-erasure condition.
