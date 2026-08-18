# gandr-theory-recursion

The execution mechanism shared by gandr's defunctionalized machines: a heap-backed frame stack that runs a recursive algorithm to completion without the algorithm ever touching the native call stack.

The rule it discharges is a safety rule rather than a performance one.
Syntax, type, and proof trees arrive from the caller, so their depth is caller-controlled; a host-recursive traversal over one of them is an attacker-controlled stack overflow.
A machine written against this crate states its recursion as data — a request, a continuation frame, an output — and the driver here walks it iteratively.

The crate owns no language semantics and performs no dynamic dispatch.
Each consumer defines its own closed request, frame, and output types, so its control flow stays inspectable and exhaustively checked.

## Current provision

- `Machine`, the trait a consumer implements: `begin` turns a request into either a return or one child request plus the frame that resumes its parent, and `resume` consumes exactly one frame and one returned child value.
- `Step`, the first-order transition type, and `StepResult`, its error-carrying alias over a machine's own typed failure.
- `run`, the driver: depth-first evaluation in the same call and return order as the recursive algorithm the machine represents, with native call depth constant and the frame stack's maximum length equal to that algorithm's logical recursion depth.

The crate is `no_std` and allocates only the frame vector.

## Planned but absent

- Nothing is scheduled here.
  The crate is complete against its contract, and it grows only if a consumer needs a step protocol this one cannot express.

One consumer instantiates it today, `gandr-surface-engine`'s recursive lowering.
The other defunctionalized machines in the workspace still carry hand-rolled frame stacks and have not been migrated onto this driver.

## Using it

Define the three closed types, implement the two transitions, and call `run`.

```rust
use gandr_theory_recursion::Machine;
use gandr_theory_recursion::Step;
use gandr_theory_recursion::run;

let output = run(&mut machine, initial_request)?;
```

The driver assumes the machine terminates: `run` makes no progress argument of its own, so a `Descend` step that does not decrease the consumer's own well-founded measure loops forever rather than failing.
Stating that measure is the consumer's obligation, and it belongs in the consumer's `# Contract`.

## Theoretical ideas relied on

Defunctionalization; continuation frames as first-order data; the abstract-machine reading of a recursive evaluator, where the control stack becomes an explicit sequence of frames.

## Primary references

- John C. Reynolds, _Definitional Interpreters for Higher-Order Programming Languages_, Higher-Order and Symbolic Computation, 1998, `doi:10.1023/A:1010027404223` — the original defunctionalization construction, which is what makes a machine's continuation a closed data type rather than a closure.
- Olivier Danvy and Lasse R. Nielsen, _Defunctionalization at Work_, Principles and Practice of Declarative Programming (PPDP), 2001, `doi:10.1145/773184.773202` — the systematic derivation this crate's request/frame/output protocol follows.
