# gandr-core-checker

The core language's static semantics: the bidirectional type system for gandr's call-by-push-value core, realized twice and held to step-for-step agreement.

`judgements::checker` is the direct-style recursive judgement — the readable reference, and the crate's centre.
`discipline::mark` is a second realization that decorates every node instead of aborting, so an incomplete program still has a typing.
The third, the defunctionalized typing machine derived from the recursive judgement by the functional correspondence, is `gandr-core-machine`: it names this crate's judgement layer, and nothing here names it back.

Where the kernel is closed by discipline, this crate is deliberately open: holes and the `Unknown` type are first-class, and the representations are non-exhaustive.
That openness is the totality half of "no parse wall" — the pipeline lowers every editor state, unparseable regions included, and the checker accepts it.

The term substrate is not here: `gandr-core-term` carries the syntax, the types, the context, substitution, interning, effect rows, grades, builtins, and the shared error, outcome, and wrapper vocabulary.
The conversion engine is not here either: `gandr-core-nbe` decides definitional equality, and subsumption calls into it for its identity endpoints.
Nor is the solver: `gandr-core-unify` answers unification problems over the same terms, and its certificates are re-checked through that same conversion relation, which is what keeps one equational theory across the three crates.
Nor is the operational realization: `gandr-core-machine` sits above this crate and drives the same rules from an explicit frame stack.
The conformance suite and the free generators that drive it are not here: they are `gandr-core-checker-tools`, so nothing test-only sits in the source tree of the checking path.

## Current provision

- `judgements` — the recursive bidirectional judgement (`checker`); the direction and `Descend`/`Return` trace vocabulary both realizations speak (`control`), which is shared by construction because the recursive judgement emits the events the machine's registers are compared against; the stack-typing judgement a reified stack needs (`stack`); the discharge of a signature's abstract type components (`package`); and the nominal-atom minting opaque ascription needs with the table that makes freshness checkable (`seal`).
- `discipline` — `subtype`, the consistent subsumption relation, reflexive but deliberately not transitive once `Unknown` participates; and `mark`, the total marking traversal that converts each abort site into a localized mark plus a matched-`Unknown` recovery.
- `kernel_bridge` — the total, iterative worklist lowering from checked core forms into `gandr-kernel-core`'s closed S1 vocabulary, rejecting out-of-subset nodes structurally with a precise refusal.

## Planned but absent

- Grade _constraints_ beyond the inline `1 ⊑ r` force check; matched-`U` operations emit none.
- Unions, intersections, polymorphism, sessions, sharing, and worlds.
- Process-soup dynamics (`fork`, `acquire`, `migrate`, and async signals).
- Source-identity wiring for the marking layer: the node facts carry a dirty bit, but setting it from the edit and order-maintenance layer is pipeline-side work.

## Using it

```toml
[dependencies]
gandr-core-checker.workspace = true
```

```rust
use gandr_core_checker::judgements::checker;
use gandr_core_checker::judgements::control::Dir;
```

A crate that only names a term, a type, or an outcome wants `gandr-core-term` instead; this crate is for the crates that need a judgement decided.

## Theoretical ideas relied on

- **Bidirectional typing** — the checking/inference mode split, which is why every rule here carries a direction and why subsumption runs exactly at the mode switch.
- **The functional correspondence** — deriving an abstract machine from a direct-style evaluator by CPS transformation and defunctionalization of the continuations.
  It is why the recursive checker and the typing machine are two presentations of one judgement, and therefore why step-for-step agreement is the right property to demand rather than a coincidence to observe.
- **Normalization by evaluation** — deciding definitional equality by evaluating into a semantic domain and quoting back, rather than by rewriting syntax.
- **Total type error localization** — marking a node and recovering with the expected type instead of aborting, so a program with errors still has a typing everywhere.
- **Consistent subtyping** — the gradual-typing relation in which the unknown type relates to every type in both directions, at the known price of non-transitivity.

## Primary references

- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Springer, 1999.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17), ISBN 978-3-540-48959-7.
- Mads Sig Ager, Dariusz Biernacki, Olivier Danvy, and Jan Midtgaard, "A Functional Correspondence between Evaluators and Abstract Machines", in _Proceedings of the 5th ACM SIGPLAN International Conference on Principles and Practice of Declarative Programming_ (PPDP '03), ACM, 2003, 8–19.
  DOI [10.1145/888251.888254](https://doi.org/10.1145/888251.888254).
- Andreas Abel and Christian Sattler, "Normalization by Evaluation for Call-by-Push-Value and Polarized Lambda Calculus", in _Proceedings of the 21st International Symposium on Principles and Practice of Declarative Programming_ (PPDP '19), ACM, 2019, 1–12.
  DOI [10.1145/3354166.3354168](https://doi.org/10.1145/3354166.3354168).
- Eric Zhao, Raef Maroof, Anand Dukkipati, Andrew Blinn, Zhiyi Pan, and Cyrus Omar, "Total Type Error Localization and Recovery with Holes", _Proceedings of the ACM on Programming Languages_ 8 (POPL), 2024, 2041–2068.
  DOI [10.1145/3632910](https://doi.org/10.1145/3632910).
- Andreas Abel and Brigitte Pientka, "Higher-Order Dynamic Pattern Unification for Dependent Types and Records", in _Proceedings of the 10th International Conference on Typed Lambda Calculi and Applications_ (TLCA 2011), Springer, 2011, 10–26.
  ISBN 978-3-642-21690-9.
- Jana Dunfield and Neel Krishnaswami, "Bidirectional Typing", 2019.
  Locator unverified: this repository holds no reference register, and no stable identifier was confirmed against a publisher record at the time of writing.
