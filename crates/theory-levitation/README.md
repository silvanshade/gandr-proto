# gandr-theory-levitation

Datatypes as descriptions: a datatype's description is a first-class value, so a generic operation over datatypes is an ordinary program over descriptions.

That is what buys derive-style code generation, wire serialization, structural equality, content-addressed interning, and the uniform handling the polygraph layer needs, without any of them being special-cased per datatype.
It is also the recursable universe the identity programme depends on: identity cannot be computed by recursion over an open universe, and a levitated universe is an inductive object one can recurse over.

The crate is the theory tier's hub by consumption.
Four workspace crates depend on it, and its reach is broad rather than narrow.

Two rungs of the ladder are here.
At the first, descriptions are Rust values and the generic functions are Rust functions; the meta-theory functions stay host-side.
At the second, the first of those moves into a written host function and the typed cell face arrives.
Nothing in either needs a dependent type.

## Current provision

- **The code universe.** The canonical declaration-table shape is a tagged description: an enumeration of constructors, each carrying a first-order code.
  The grammar is the finitary first-order fragment of unit, variable, product and sum, plus a graded and attributed field leaf over a core value type and an atom-abstraction leaf.
  **Higher-order codes are excluded from the fragment rather than deferred**, because the fragment is first-order precisely so that code equality stays decidable.

  Decidable equality is load-bearing rather than cosmetic.
  It is what content-addressed interning keys on and what the matching-modulo engine compares, so a code derives total structural equality and hashing.
- **Two-cell faces.** A rewrite over a signature is a pair of open terms in the free structure over that signature.
  They are stored untyped-but-host-checked as term pairs, and each face carries derived per-variable metadata whose variance is constant at this rung, so the later refinement is an update rather than a migration.
- **Circuit rules.** A circuit rule's boundary pair is derived from a wiring rather than written: the source boundary is the diagram with every redex replaced by its source, the target with every redex replaced by its target.
  The declared sphere stays the declaration's, and the derived pair is checked against it, so a mis-glued boundary fails at the declaration table rather than downstream.
- **Circuit-block elaboration.** A rewrite-sorted port binds the interface pair a hole carries.
  Instantiating one at a redex line unifies the source against the line's input wiring and binds the target; a block then elaborates to the boundary language's whiskered composite of its redex inside its frames.
- **Multi-output arities**, presented as a bridge diagram of four finite sets and three maps.
  That separates one operation's named result tuple from destination aggregation, which independently requires a commutative monoid.
  The encoding is finite sets and maps, so it stays content-addressable and first-order.
- **The description table**, which is the declaration table: the minted identity, the graded and attributed parameters and constructors, the reserved operations and two-cell faces, and the declaration polarity.
- **Real consumers**, which is the point.
  A description table nobody decodes is the named dead end this rung was built to refute, so the description drives structural equality of two values guided by a code, a canonical deterministic wire encoding, the inspectable rendering of a signature's normal form, and the content-addressed interner.
  Primitive formers are retrofitted as descriptions, so the same generic programs cover builtins and declared data uniformly.
- **The decoder**, a large elimination from descriptions into the core value-type universe over the decidable first-order fragment, and the typed two-cell face, which refines a face with a decoded pattern context and reuses the untyped face whole rather than rewriting it.

## Planned but absent

- The codata decoder.
  The declaration polarity tag ships; only the inductive decoder is written.
- A dependent sum code.
  The current fragment is non-dependent, so the decoder cannot yet target the core's dependent pair.
- The remaining meta-theory functions.
  Moving them from host-side Rust into the checker one at a time is what the later rungs of the ladder are.

## Using it

Build a description, then run a generic program over it.

```rust
use gandr_theory_levitation::CodeInterner;
use gandr_theory_levitation::generic_eq;
use gandr_theory_levitation::serialize_value;

let equal = generic_eq(&code, &left, &right);
let bytes = serialize_value(&code, &value);
```

Code equality is structural and total, so two descriptions that intern to the same address are the same description.
Any change to the code grammar that admits a higher-order code breaks that, which is why the exclusion is a fragment boundary rather than a to-do.

## Theoretical ideas relied on

Levitation, and the universe of datatype descriptions as an inductive object; generic programming driven by a description rather than by a type; large elimination; the free monad over a signature, whose elements are the open terms a rewrite face pairs; bridge diagrams for multi-output arities; content-addressed interning on a decidable equality; the data and codata polarity split between inductive and coinductive decoding.

## Primary references

- James Chapman, Pierre-Évariste Dagand, Conor McBride and Peter Morris, _The Gentle Art of Levitation_, International Conference on Functional Programming (ICFP), 2010, `doi:10.1145/1863543.1863547` — the levitated universe of descriptions this crate realizes, and the source of the staging discipline that keeps the meta-theory functions host-side until each is moved deliberately.
