# gandr-surface-lsp

The language-server face: a hand-rolled JSON-RPC encoder over the pipeline report and the shared highlight spans.

The crate parses, lowers, types, and marks nothing.
It calls the public parse, highlight, lower, and report passes and re-encodes their byte spans into LSP positions.

## Current provision

- Semantic tokens from `HlRole` / `HlSpan`, using the standard LSP token-type registry agreed with the TUI highlighter, for the whole document and for a requested range.
  A range restricts which tokens are sent and never the coordinate system they are sent in: deltas still chain from the document origin, and a token overlapping either edge is returned whole rather than clipped.
- Diagnostics with stable `code` values, hover, and completion over the same whole-file report envelope.
- Content-Length JSON-RPC on stdio.
- `gandr lsp --capabilities`, the advertised initialize result.

## Planned but absent

- Incremental recheck.
  Every request is a whole-file recheck today, and it cannot be otherwise from this side.
  `Session::submit` is append-only, while an editor's `didChange` replaces the document, so a fresh session per recheck is the only correct use of the engine's interactive API.
  The grammar is cached per process; what remains is whole-file typing, which is engine-side work.
- The render-bus attach advertisement and custom `gandr/` methods.
- Delegated formatting.

## Using it

```text
gandr lsp --capabilities
gandr lsp
```

`gandr lsp` speaks the language server protocol on standard input and output.
`gandr lsp --capabilities` prints the initialize result and leaves.

## Theoretical ideas relied on

The renderer firewall: every surface is an encoder over reified checker state.
Semantic highlighting is a projection of the grammar's highlight roles, not a second parser.

## Primary references

- Microsoft, _Language Server Protocol Specification_, 3.17, 2022, `https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/` — the protocol subset this face serves.
- Cyrus Omar, Ian Voysey, Michael Hilton, Jonathan Aldrich and Matthew A. Hammer, _Hazelnut: A Bidirectionally Typed Structure Editor Calculus_, POPL 2017, `doi:10.1145/3009837.3009900` — total marking, the discipline that treats an error as a mark rather than an abort.
