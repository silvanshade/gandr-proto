# core-checker-tools

Test-facing machinery for the core checker and its typing machine, held above both so neither crate carries test-only code in the source tree of the checking path.
No shipping crate takes a library dependency on this one; every consumer is a test target.
Since the checker split, this crate is also what carries the step-for-step invariant between the two realizations: they are two crates now, and the suite that compares them drives both from outside.

## Current provision

- `strategies` — the _free_ proptest generators over the core call-by-push-value syntax and types: grades, binder names, leaf and recursive value and computation types, and hole identifiers.
  These are grammar-directed rather than type-directed, so they produce mostly ill-typed terms, which is what the agreement properties want.
- The `conformance` test target — the checker-versus-machine conformance suite.
  It pins _step-for-step_ agreement between the two realizations of the same typing judgement that `gandr-core-checker` keeps in tree: the direct recursive bidirectional checker and the defunctionalized typing machine.
  Its evidence is of two kinds — example-based rows over the worked core-CBPV examples, including a literal machine trace, and property rows over generated terms, where the type-directed generators produce well-typed terms (agreement and success) and the free generators produce arbitrary ones (agreement on the error and on the trace prefix).
- The type-directed, well-typed generators, which stay beside the conformance suite that drives them rather than becoming library surface.

## Planned but absent

- Promotion of the type-directed generators to library surface, which waits on a second consumer; they are coupled to the conformance harness today.
- The executable observable-outcome soundness rows are **not** here: they re-homed to `gandr-core-sequent`'s own test target when the L machine became the sole operational driver, and this suite carries only the typing rows.

## Using it

A crate whose tests need the free generators takes a dev-dependency and names them directly:

```toml
[dev-dependencies]
gandr-core-checker-tools.workspace = true
```

```rust
use gandr_core_checker_tools::strategies::binder_name;
```

The generators name only `gandr-core-term`, so a consumer of them links the substrate and nothing above it; the two typing faces the conformance suite compares are this crate's own development dependencies.
`gandr-core-checker` dev-depends on this crate for its inline property tests, so the pair still carries a development cycle, which is the arrangement Cargo admits — the non-development graph stays acyclic, so nothing propagates to a consumer of either crate.

## Theoretical ideas relied on

- **The functional correspondence** — the derivation of an abstract machine from a direct-style evaluator by CPS transformation followed by defunctionalization of the continuations.
  It is why the recursive checker and the typing machine are two presentations of one judgement, and therefore why step-for-step agreement is the right property to demand rather than a coincidence to observe.
- **Property-based testing** — generated inputs plus shrinking, as against enumerated cases.
- **Differential testing**, and its limit: a differential suite refutes agreement and never establishes correctness.
  Two implementations derived from each other share their bugs, so agreement rows say nothing about a fault both faces carry.
  That limit is why the checker's directed coherence oracles exist beside this suite rather than inside it.

## Primary references

- Mads Sig Ager, Dariusz Biernacki, Olivier Danvy, and Jan Midtgaard, "A Functional Correspondence between Evaluators and Abstract Machines", in _Proceedings of the 5th ACM SIGPLAN International Conference on Principles and Practice of Declarative Programming_ (PPDP '03), ACM, 2003, 8–19.
  DOI [10.1145/888251.888254](https://doi.org/10.1145/888251.888254).
- Koen Claessen and John Hughes, "QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs", Proceedings of the 5th ACM SIGPLAN International Conference on Functional Programming (ICFP), 2000.
  Locator unverified: this repository holds no reference register, and no stable identifier was confirmed against a publisher record at the time of writing.
