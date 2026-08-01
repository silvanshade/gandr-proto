# Overview

## The system in one paragraph

gandr is a dependently typed language and shell built around a minimal certified kernel.
A polarized call-by-push-value core is checked by a bidirectional typing machine, lowered by static focusing onto a polarized System-L command IL, and executed by the L machine.
Rewriting is the computational heart: user rules and machine-derived fusion cells live in a content-addressed cell store over a **circuit-algebra-shaped** substrate — cells whose interfaces are many-in/many-out wirings, with the cut as a first-class wiring datum — and a budgeted Squier completion loop synthesizes derived cells whose certificates are replayed rather than trusted.
Identity is data: the code universe is presented by an edit polygraph, per-stratum univalence is a theorem gated on finite per-degree checks (never an axiom), and the directed statement is the primitive one, with the groupoidal form as its invertible core.
A virtual-double-category judgement layer reflects the rewriting doctrine; certificates form a tracelet algebra whose normal form, bracket oracle, and replay discipline are the engine's correctness currency.
Persistence is content-addressed and untrusted; the mechanized metatheory is Agda, built over ∞-graphs in a setoid ambient, `--safe --without-K` throughout.

## What is distinctive, in five sentences

The kernel's redexes live at the cut between a producer and a consumer, so rule overlaps are shallow, critical pairs are enumerable, and fusion is a certified derived cell rather than a compiler pass.
The cell substrate carries the full generality of circuit algebras — disconnection, wheels, and multi-output interfaces — with gandr's restrictions enforced by static analysis rather than by the carrier.
Identity and univalence are placed _in the codes_, not in an interval: `refl` is a constructor, transport is certificate replay, and cost is a first-class count.
Coherence is an economy, not a tax: decide cheaply where possible, dissolve whole families by semantic invariants, normalize residues, and only ever _generate_ witnesses off the trusted base, verified by replay.
Everything trusted is small, first-order, and decidable; everything else is a carried, replayable certificate.

## The tracks

| track                 | read it for                                                                                                                                                                                                                                                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [[metatheory]]        | the mathematics of gandr's model: the circuit-algebra substrate and its carrier, the sequent kernel's metatheory, descriptions and cells, the two univalence statements (layout and pasting), the nerve warrant, the doctrine and equipment layer, the certificate algebra, the coherence economy, the ambient-primitive policy |
| [[implementation]]    | the Rust system: the crate map, the kernel IL and typing machine, the rewriting/completion/tracelet engines, the content-addressed storage stack, the surface pipeline, the gates                                                                                                                                               |
| [[proof-engineering]] | how the Agda metatheory is built: the ∞-graph substrate with laws as cells, the familial representation discipline, characterize-before-building, the coherence-cost engineering, and the pointers into the operational workflow rules                                                                                          |
| [[surface-language]]  | what gandr programs look like: the whole language sketched in one place, the grammar machinery (PBG, molder/melder, obligations), the declaration forms and reserved slots, the (co)recursion surface, the shell fragment, and the vocabulary decisions of record                                                               |

Each track links a `roadmap` sub-document with the detailed remaining work; [[metatheory/guards]] is the corpus-wide do-not-reopen ledger, and [[metatheory/citation-hazards]] the citation-trap ledger.
Conventions for reading and writing these documents (Typst math, fletcher diagrams, Hayagriva citations, named anchors) are in [[README]].
