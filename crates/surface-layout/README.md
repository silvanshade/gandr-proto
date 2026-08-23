# gandr-surface-layout

The shared document-layout engine every gandr printing face resolves through.

A client builds an immutable document in a sealed arena, the resolver picks a Pareto-optimal layout for a page width, and a first-order machine renders the winning plan.
The presentation printer, the source formatter, and the editor faces all use this one engine, so a layout decision is made in exactly one place.

The engine is deliberately more expressive than a greedy printer: it carries arbitrary choice and unaligned concatenation.
Neither can be retrofitted onto a greedy representation without rewriting every client document.

## What the crate provides

- A sealed document arena with stable dense identities, checked client handles, and sharing by construction.
- The document algebra: empty, text, opaque verbatim text, soft and hard line breaks, unaligned concatenation, nesting, alignment, arbitrary choice, and flattening.
- A builder that is the only insertion path, with typed errors and no infallible constructor.
- Explicit build budgets and a meter that enforces them before any store grows.
- Memoized, iterative Pareto resolution with squared-overflow cost, physical ending options, exact width taint, generational plan identities, and shared render budgets.
- The defunctionalized render machine with its entry point, rendered result, and tainted fallback execution.

## What remains

- Nothing in the engine itself.
  What is not yet true of the tree around it: `gandr-surface-pretty` is its only consumer, and no printing face renders through that path yet, so the width-aware route is exercised by goldens rather than by a user.

## Using it

A caller states its budgets, builds a document, and seals it:

```rust
use gandr_surface_layout::arena::TextSource;
use gandr_surface_layout::build::DocBuilder;
use gandr_surface_layout::limits::BuildLimits;
use gandr_surface_layout::limits::BuildMeter;
use gandr_surface_layout::units::NestAmount;

let mut meter = BuildMeter::try_new(BuildLimits::default())?;
let mut builder = DocBuilder::try_new(&mut meter)?;
let head = builder.text(TextSource::from("let x ="))?;
let line = builder.line();
let body = builder.nest(NestAmount::from(2u32), line)?;
let doc = builder.concat(head, body)?;
let arena = builder.finish()?;
```

`render::render` then takes that arena, a root, and a page width, and returns the rendered text.

## Theoretical ideas relied on

Pareto-optimal layout resolution over a measure set ordered by squared overflow and line count; width taint as a marker for candidates outside the optimality theorem; defunctionalization of the fused resolve-and-render token function into a reference-counted plan arena.

## Primary references

- Sorawee Porncharoenwase, Justin Pombrio and Emina Torlak, _A Pretty Expressive Printer_, Proceedings of the ACM on Programming Languages 7 (OOPSLA2), 2023, `arXiv:2310.01530`, `doi:10.1145/3622837` — the optimality theorem, the measure set, the computation-width taint state machine, in-context memoization, and the complexity bounds this engine implements.
- Philip Wadler, _A prettier printer_, in Jeremy Gibbons and Oege de Moor (editors), _The Fun of Programming_, Palgrave Macmillan, 2003, `https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf` — the document algebra that this crate's choice and flattening operators generalize.
