# core-sequent

`gandr-core-sequent` provides the focused sequent and kernel boundary used by the executable corpus gates.

## What it currently provides

- Focused sequent checking and live L-machine execution for lowered corpus terms.
- Differential outcome snapshots, totality floors, kernel partition checks, and kernel export checks.
- Deterministic corpus provenance from repository-relative `.gandr` paths and exact source bytes.

## Planned but not implemented

- No additional corpus anchor format is planned; executable terms come from live total lowering.

## Using it

`cargo test -p gandr-core-sequent --test sequent` runs the complete sequent corpus suite.

## Theoretical ideas

Focused sequent calculus, machine readback, and deterministic golden-file verification.

- Jean-Yves Girard, Yves Lafont, and Paul Taylor, _Proofs and Types_, Cambridge University Press, 1989, ISBN 0-521-34641-1.
- Per Martin-Löf, _Intuitionistic Type Theory_, Bibliopolis, 1984, ISBN 978-8870881052.
