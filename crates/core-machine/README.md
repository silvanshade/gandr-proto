# gandr-core-machine

The defunctionalized typing machine for the gandr core: the operational face of the same bidirectional judgement `gandr-core-checker` states recursively.

It is derived from that recursive judgement by the functional correspondence — CPS transform the checker, then defunctionalize the continuations — so each frame constructor is the image of one pending recursive call site and the stack _is_ the continuation.
Frames carry no continuation pointers, the stack lives on the heap, and nothing recurses on term depth, which is what makes an adversarially deep term a step count rather than a stack overflow.

The dependency runs one way and that is the derivation direction: this crate names the judgement layer, and nothing in the judgement layer names this crate.
The step-for-step agreement between the two realizations is not weakened by that edge — it never lived in either crate.
`gandr-core-checker-tools` owns the conformance suite that compares them, and it drives both faces from outside, exactly as it did when they shared a crate.

## Current provision

- `State` — the machine state: the term under consideration, the context, the direction, and the frame stack.
- `Frame` — the frame inventory, one constructor per pending recursive call site in the reference judgement, with the originating direction carried on the frames that complete a rule.
- `step` — the step function; `run`, `run_value`, and `run_comp` drive it to a verdict and a control trace.
- `run_report` — the same run with two projections the `(result, trace)` pair does not expose: the step count, which pins `steps == trace.len() - 1`, and the final context, which pins that a successful run matched every bind with an unbind.
- `FailureState` — the state at the failure point, with the contexts as they stood there rather than as they were unwound.

Subsumption runs at the frame pop, exactly where the recursive judgement's inlined subsumption rule runs, and the frame-pop ordering convention puts a fallible check before a context restore so the failure state is always the pre-pop context.

## Planned but absent

- Stage 1 has no constraint solver, so a subsumption obligation is decided at the frame pop rather than emitted; emission semantics wait on the solver.
- The error-path context is deliberately not restored, which is where this realization and the recursive one differ; the conformance suite compares typed errors rather than contexts because of it.

## Using it

```toml
[dependencies]
gandr-core-machine.workspace = true
```

```rust
use gandr_core_machine::Outcome;
use gandr_core_machine::State;
use gandr_core_machine::step;
```

A caller that wants a verdict uses `run_comp` or `run_value`; a caller that wants to observe the trajectory drives `step` itself and reads the control register at each step.

## Theoretical ideas relied on

- **The functional correspondence** — deriving an abstract machine from a direct-style evaluator by CPS transformation followed by defunctionalization of the continuations.
  It is why this crate and the recursive judgement are two presentations of one thing, and therefore why step-for-step agreement is the right property to demand rather than a coincidence to observe.
- **Defunctionalization** — replacing higher-order continuations by a first-order data type plus an apply function, which is what makes the machine state inspectable, serializable, and resumable.
- **Bidirectional typing** — the checking/inference mode split the frames carry and the subsumption check at the mode switch.
- **Call-by-push-value** — the polarized calculus being typed, whose value/computation split is what gives the frame inventory its two families.

## Primary references

- Mads Sig Ager, Dariusz Biernacki, Olivier Danvy, and Jan Midtgaard, "A Functional Correspondence between Evaluators and Abstract Machines", in _Proceedings of the 5th ACM SIGPLAN International Conference on Principles and Practice of Declarative Programming_ (PPDP '03), ACM, 2003, 8–19.
  DOI [10.1145/888251.888254](https://doi.org/10.1145/888251.888254).
- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Springer, 1999.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17), ISBN 978-3-540-48959-7.
- Jana Dunfield and Neel Krishnaswami, "Bidirectional Typing", 2019.
  Locator unverified: this repository holds no reference register, and no stable identifier was confirmed against a publisher record at the time of writing.
