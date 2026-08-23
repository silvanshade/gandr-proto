# gandr-core-incremental-assessment

**An assessment instrument, not shipping machinery.** It measures what a general on-demand query engine would cost and buy for gandr's item-granular incremental typing, so that the question can be answered with numbers rather than argument.

Nothing in the workspace depends on this crate, and nothing should while the adoption question is open.
It is also the only crate whose manifest names the engine under assessment (`salsa`), and its own test suite enforces that: if the engine ever reaches a second manifest, `confinement` fails.

## What it currently provides

- A **query-graph model** of item typing (`engine`): a database keyed by item identity, with four memoized queries — the item's dependency footprint, the program's name-resolution table, the binding each item contributes, and the item's typing against a context restricted to its footprint.
- A **generated workload** (`workload`): independent chains of ascription-linked definitions, sized so that each edit's dirty set is known before the run and can be asserted rather than observed.
  Two edits are applied — one that changes an item's value and not its type, and one that changes both.
- A **measured comparison** (`measure`, `baseline`): the model against `gandr-core-incremental`'s validated resume, on the same programs, under two demand shapes — asking for every item's typing, and asking for one item's.
- **Work counters asserted against the workload's structure** (`ledger`): query bodies executed, memos reused, footprints rescanned, bytes crossed at the ownership boundary.
  A cost comparison whose inputs never reached the code under test is green and worthless, and only a work count catches that.
- A **correctness differential**: both paths' answers against from-scratch typing, never against each other.

## What it does not do, deliberately

- **It does not adopt anything.** Adopting an engine is a separate decision; this crate exists to inform it.
- **It does not measure below the item.** A general engine's per-query bookkeeping is not amortized by a per-node judgement, so node granularity is out of scope by rule.
- **It does not model item insertion or deletion.** Both measured edits keep the item list fixed, so the comparison is about reuse rather than about alignment.
- **It does not thread definitional unfoldings.** The model's bindings carry types only, which is sound for this workload's rigid-atom ascriptions and is stated where it matters.
  A model built for adoption would split the contribution.

## Using it

```sh
# The suite, including the correctness differential and the work-count assertions.
cargo test -p gandr-core-incremental-assessment

# The measurement itself. Take timings from a release build; the counts are
# profile-independent and are the substance.
cargo test --release -p gandr-core-incremental-assessment --test assessment \
  -- --nocapture --test-threads=1 table::
```

The `table::` filter reports three things: the per-edit comparison, the engine's per-ingredient retention breakdown, and both paths' recheck cost against program size.

## The ideas it relies on

- **Dependency footprints** — an item's cached result is reusable exactly when the inputs it read are unchanged; the footprint is the read set that decides it.
  The model projects each footprint into its query's declared dependencies.
- **Demand-driven invalidation** — recomputation is driven by what is asked for rather than by a sweep over what might have changed.
- **Backdating (equality cutoff)** — a recomputed value equal to its predecessor stops the invalidation wave.
  This is the mechanism the model's binding query exists to exploit, and the one whose failure would be invisible in a cost table.
- **Co-contextual typing** — reformulating a judgement so a fragment's obligation depends on the fragment plus an explicit interface, which is what makes footprint-restricted contexts meaningful.

## Primary resources

- Sebastian Erdweg, Oliver Bračevac, Edlira Kuci, Matthias Krebs, Mira Mezini, _A Co-contextual Formulation of Type Rules and Its Application to Incremental Type Checking_, OOPSLA 2015, pages 880–897.
  DOI: [10.1145/2814270.2814277](https://doi.org/10.1145/2814270.2814277).
  The dualization recipe, the incrementalization scheme, and the finding that a general engine's overhead is not repaid at fine granularity — which is where this crate's item-and-coarser scope comes from.
- Matthew A. Hammer, Khoo Yit Phang, Michael Hicks, Jeffrey S. Foster, _Adapton: Composable, Demand-Driven Incremental Computation_, PLDI 2014, pages 156–166.
  DOI: [10.1145/2594291.2594324](https://doi.org/10.1145/2594291.2594324).
  Demand-driven invalidation, which is the answer to the recheck-traversal floor the two demand shapes here are designed to separate.
- Thomas Reps, Tim Teitelbaum, Alan Demers, _Incremental Context-Dependent Analysis for Language-Based Editors_, ACM TOPLAS 5(3), 1983, pages 449–477.
  DOI: [10.1145/2166.357218](https://doi.org/10.1145/2166.357218).
  The dependency-footprint discipline the checkpoint engine's reuse rule instantiates.
