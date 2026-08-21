# gandr-surface-diagnostics

The terminal diagnostic facade over the surface engine's merged verdict stream.

The crate does not parse, lower, type, or decide outcomes.
It projects report diagnostics and outcome-only type errors into Unicode source snippets through `annotate-snippets`, with deterministic plain and terminal-styled policies.

## Contract

- `render_submission` preserves merged verdict order and returns one report for each warning, report diagnostic, and outcome-only type error.
- `render_verdict` returns `None` for values, definitions, and hole goals.
- `RenderStyle::Plain` is escape-free for snapshots and non-terminals; `RenderStyle::Styled` is the explicit color-forcing boundary.
- Valid source spans become primary annotations; invalid spans degrade to a path-only origin rather than panicking.
- Type mismatch labels name both the expected and actual types.
- The facade keeps `annotate-snippets` behind its public API, so the script and REPL surfaces depend on a stable gandr report vocabulary.

## Using it

```rust
use gandr_surface_diagnostics::RenderStyle;
use gandr_surface_diagnostics::render_submission;
use gandr_surface_syntax::SourceSlice;

let reports = render_submission(
    SourceSlice::from(source),
    Some(path),
    &submission,
    RenderStyle::Plain,
);
```

The script runner prefixes the rendered refusal with `type checking failed:`.
The REPL emits the same report as a diagnostic transcript block.

## Theoretical ideas relied on

The renderer firewall: diagnostics are an encoder over reified checker state, not a second type checker.

## Primary references

- Rust compiler team, _annotate-snippets_, version 0.12.16, `https://docs.rs/annotate-snippets/0.12.16/annotate_snippets/` — source-anchored terminal report rendering.
- Cyrus Omar, Ian Voysey, Michael Hilton, Jonathan Aldrich and Matthew A. Hammer, _Hazelnut: A Bidirectionally Typed Structure Editor Calculus_, POPL 2017, `doi:10.1145/3009837.3009900` — total marking, the discipline that treats an error as a mark rather than an abort.
