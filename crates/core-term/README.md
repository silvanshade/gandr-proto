# gandr-core-term

The core call-by-push-value term substrate: the vocabulary every gandr core judgement is stated over.

Nothing here decides anything, and nothing here names a crate that does: this crate has no upward dependency, which is what makes it the common substrate.
It carries the terms, the types that classify them, the services a judgement spends while walking them, and the shared wrapper and error vocabulary the answers come back in, so every crate that _does_ decide is stated over one vocabulary: `gandr-core-checker`'s recursive bidirectional judgement, `gandr-core-machine`'s defunctionalized realization of that same judgement, `gandr-core-nbe`'s conversion engine, `gandr-core-unify`'s solver, and `gandr-core-sequent`'s L machine.

Those crates are not independent of each other, and `ARCHITECTURE.md`'s tier map is where their edges are stated: both the checker and the solver depend on the conversion engine, because a definitional equality is decided in exactly one place.
What this crate buys is that none of them has to depend on another merely to name a term.

## Current provision

- `syntax` — the values and computations of core CBPV as two distinct sorts over reference-counted children, plus the flat arena a total traversal interns into.
- `types` — the value and computation types that classify them, split by the same polarity, each with the variant census that lets a judgement in another crate be checked for totality despite the non-exhaustive enums.
- `classifier` — the `(sort, level)` pair a type is formed at: the two ground sorts, the sort expression a declaration may abstract over, and the classifier itself.
- `ctx` — the two-zone typing context `Γ; Σ`.
- `subst` — the iterative capture-avoiding substitution engine over terms, and the hole substitution a solver's certificate is re-checked through.
- `identity` — the value-into-type substitution that instantiates a `Walk` motive; types carry no binders, so it is capture-free structural recursion delegating its one binder-bearing case to `subst`.
- `intern` — content-addressed identity for a type, giving O(1) equality.
- `effect` — effect-graded returners `F^ε`, the sealed name-ordered effect row, and the operation signatures, with `effect::host` carrying the representation-independent host seam the native runtime and the surface lowerer share.
- `grade` — the preordered semiring over `ℕ ∪ {ω}` the thunk discipline counts usage in, sealed behind a module-private representation.
- `boundary` — the semantic wrappers every crate-defined signature in this tier crosses, which is what keeps anonymous primitives out of the substrate's public surface.
- `prim` — the native builtin registry behind `syntax::Comp::Native`, carrying an opaque tag rather than a Rust function so the IR stays structurally comparable.
- `error` — the structured typing error every total judgement returns instead of diverging.
- `outcome` — the evaluation vocabulary and the one step budget the whole workspace shares.
- `nominal` — gandr's sort tags over the shared atom substrate.

Every walk here is total and iterative: substitution, interning, and arena construction run over explicit worklists rather than the host call stack, because a term's depth is caller-controlled.

## The classifier model

gandr classifies with one pair, `(sort, level)`, everywhere: surface syntax, formation, normalization, kernel export, and diagnostics all read the same classifier.
There are two universe families, `Type[+, l]` over value types and `Type[-, l]` over computation types, sharing the one level algebra that `gandr-kernel-strata` owns and nothing else; there is no kind layer above them and no second level algebra beside them.
The `+U` and `-F` bridges are the only crossings between the two term categories, and an elaborator writes one only at a designated checked site, as a recorded node — never through unification, conversion, or a coercion search.
There is no dependent-elimination keyword: an effect-sequencing binder is opaque to the type layer.
Only ground sorts cross into the certified kernel; a declaration abstract in its sort is specialized to ground before admission.

## Planned but absent

- The linear zone `Σ` is the committed shape but vacuous: no obligation source exists yet, so the discipline is exercised over the zone directly.
- Grade _constraints_ beyond the inline `1 ⊑ r` force check; matched-`U` operations emit none.
- Unions, intersections, polymorphism, and the row-polymorphic open tail `ρ`.
- No shipping crate keys anything on an interned `TypeId` yet; the facility is exercised by tests alone.

## Using it

A crate that names a gandr core term, its type, or the vocabulary an answer comes back in depends on this crate and nothing above it:

```toml
[dependencies]
gandr-core-term.workspace = true
```

```rust
use gandr_core_term::syntax::Comp;
use gandr_core_term::types::ValueType;
```

The `gandr_feat_regex` feature (through `full`) enables the regex-backed builtins `regex.extract` and `string.escape`; it is off by default.

## Theoretical ideas relied on

- **Call-by-push-value** — the polarized split of terms into values and computations, with the thunk and returner shifts between them, which is what makes the two sorts here two sorts rather than one.
- **Quantitative type theory** — usage tracked in a preordered semiring, which is what `grade` is and why `0`, `1`, and `ω` are the elements the thunk rules read.
- **Algebraic effects and handlers** — operations declared in a signature and interpreted by a handler, which is what `effect` gives the returner its row from.
- **Gradual typing's unknown type** — the hole type that relates to every type in both directions, which is why the representations are non-exhaustive and why nothing here refuses to represent an incomplete program.
- **Nominal sets** — atoms with sorts, and the atom-role versus variable-role boundary that keeps unification unitary, which is what `nominal` tags.
- **Hash consing** — content-addressed identity giving constant-time structural equality, which is what `intern` is.

## Primary references

- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Springer, 1999.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17), ISBN 978-3-540-48959-7.
- Robert Atkey, "Syntax and Semantics of Quantitative Type Theory", in _Proceedings of the 33rd Annual ACM/IEEE Symposium on Logic in Computer Science_ (LICS '18), ACM, 2018, 56–65.
  DOI [10.1145/3209108.3209189](https://doi.org/10.1145/3209108.3209189).
- Matija Pretnar, "An Introduction to Algebraic Effects and Handlers.
  Invited Tutorial Paper", _Electronic Notes in Theoretical Computer Science_ 319, 2015, 19–35.
  DOI [10.1016/j.entcs.2015.12.003](https://doi.org/10.1016/j.entcs.2015.12.003).
- Christian Urban, Andrew M. Pitts, and Murdoch J. Gabbay, "Nominal Unification", _Theoretical Computer Science_ 323, 2004, 473–497.
  DOI [10.1016/j.tcs.2004.06.016](https://doi.org/10.1016/j.tcs.2004.06.016).
- Jeremy G. Siek and Walid Taha, "Gradual Typing for Functional Languages", _Scheme and Functional Programming Workshop_, 2006.
  Locator unverified: this repository holds no reference register, and no stable identifier was confirmed against a publisher record at the time of writing.
