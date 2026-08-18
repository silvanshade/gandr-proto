# gandr-core-sequent

`gandr-core-sequent` is the polarized command intermediate language and the machine that runs it.
It carries the sequent kernel through its operational phase beside the frozen CBPV core, consuming the term substrate alone: the checker's own faces appear only in this crate's test targets.

Two phases live here.
The command IL reifies the polarized sequent calculus as arena-resident, inspectable nodes, and the static focusing translation bridges the checked core into it.
The L machine then runs the focused IL iteratively over a two-region store — call-by-need cells beside a frame region — executing the whole effect and control surface: `perform`, `handle`, `resume`, `reset`, and `shift`.

## Current provision

- The three node families of the command IL in one arena, with focusing from checked core terms into it and an un-focusing readback out of it.
- The iterative L machine over the two-region store, total and deterministic, with the full effect and control surface.
- Focused sequent checking, and the L machine's half of the compiled host's path.
- The differential that anchors the machine against frozen outcome snapshots, plus totality floors, kernel partition checks, and kernel export checks over the executable corpus.
- Deterministic corpus provenance from repository-relative source paths and exact source bytes.

## Planned but absent

The fusion engine over command seams is not here: it reads this crate's public IL from the theory tier.
The readback recovers everything except a reified stack, which stays opaque by construction.
No additional corpus anchor format is planned — executable terms come from live total lowering.

## Using it

`cargo nextest run -p gandr-core-sequent --features=full` runs the crate suite.
`cargo test -p gandr-core-sequent --test sequent` runs the sequent corpus suite on its own, which is the narrower gate when only corpus terms changed.

## Theoretical ideas relied on

The polarized sequent calculus and its focusing discipline; call-by-push-value polarity as the source language's shape; an abstract machine derived from the focused calculus rather than invented beside it; delimited control and algebraic effect handlers as machine transitions; and deterministic golden-file verification as the anchor for a machine whose correctness is agreement rather than proof.

## Primary references

- Jean-Yves Girard, Yves Lafont, and Paul Taylor, _Proofs and Types_, Cambridge University Press, 1989, ISBN 0-521-34641-1.
- Per Martin-Löf, _Intuitionistic Type Theory_, Bibliopolis, 1984, ISBN 978-8870881052.
- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Springer, 1999.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17), ISBN 978-3-540-48959-7.
- Mads Sig Ager, Dariusz Biernacki, Olivier Danvy, and Jan Midtgaard, "A Functional Correspondence between Evaluators and Abstract Machines", in _Proceedings of the 5th ACM SIGPLAN International Conference on Principles and Practice of Declarative Programming_ (PPDP '03), ACM, 2003, 8–19.
  DOI [10.1145/888251.888254](https://doi.org/10.1145/888251.888254).
- Matija Pretnar, "An Introduction to Algebraic Effects and Handlers.
  Invited Tutorial Paper", _Electronic Notes in Theoretical Computer Science_ 319, 2015, 19–35.
  DOI [10.1016/j.entcs.2015.12.003](https://doi.org/10.1016/j.entcs.2015.12.003).
