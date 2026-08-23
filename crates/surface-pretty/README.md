# gandr-surface-pretty

Presentation documents for core state: a checked type or a first-order value becomes a layout document, and the caller's page width decides how it breaks.

Every printing face — the read-evaluate transcript, the language-server hover, the diagnostic operands — reads the same vocabulary, because they read this one producer rather than each carrying a renderer.

## What the crate provides

- `present_type` and `present_value`, each rendering at a caller-supplied page width.
- Documents built so that their **flattened image is byte-for-byte the engine's flat spelling**: every break point is a choice whose inline branch contributes exactly the byte the flat spelling carries there, so a generous page reproduces the flat rendering with no special case and a narrow page breaks with two-column continuations.
- A total posture: both walkers drain explicit task stacks rather than recursing, value rendering is depth-bounded at `DEPTH_LIMIT` with a `<deep>` leaf beyond it, and every construction or rendering failure surfaces as `PresentationError`.
- Goldens pinning representative forms at a narrow and a generous width, each generous golden asserted equal to an inline flat-spelling witness.

## What remains

- The input side moves to an arena traversal when the flat carriers land; the output side — these documents, these break points, this vocabulary — carries over unchanged.
- No face consumes this crate yet: the transcript, hover, and diagnostics print through the engine's flat renderers, and adopting the width-aware path is their own cutover.

## Using it

```rust
use gandr_surface_pretty::present_type;
use gandr_surface_layout::units::PageWidth;

let rendered = present_type(&ty, PageWidth::from(80u32))?;
```

## Theoretical ideas relied on

Pareto-optimal layout resolution over a measure set, and a document algebra with arbitrary choice and flattening — both supplied by `gandr-surface-layout`, which this crate is a client of rather than an extension to.

## Primary references

- Sorawee Porncharoenwase, Justin Pombrio and Emina Torlak, _A Pretty Expressive Printer_, Proceedings of the ACM on Programming Languages 7 (OOPSLA2), 2023, `arXiv:2310.01530`, `doi:10.1145/3622837` — the optimality theorem and the measure set the layout engine beneath this crate implements.
- Philip Wadler, _A prettier printer_, in Jeremy Gibbons and Oege de Moor (editors), _The Fun of Programming_, Palgrave Macmillan, 2003, `https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf` — the document algebra whose choice and flattening operators these documents are written in.
