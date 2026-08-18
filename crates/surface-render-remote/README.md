# gandr-surface-render-remote

The remote face of gandr's render surface: the plain, `Send`-safe, `serde`-ready vocabulary a renderer consumes, and the versioned bus frame that carries it across a process boundary.

The crate parses, lowers, types and marks nothing.
That is the renderer firewall, and here it is a dependency fact rather than a convention: this crate names no workspace crate at all, and its one optional dependency is `serde`.
A language server, a TUI, or an agent can link it without pulling in the checker and without creating a cycle.

The in-process presentation printer that will sit beside it is not written yet.
When it arrives, the two render crates share this vocabulary rather than each minting one.

## Current provision

- `present`, the `Send`-safe presentation seam: highlight and mark spans, diagnostic and goal cards, cursor types and completion candidates, the preview and transcript frames, and the total byte-offset to row-and-column projection.
  The pipeline's own semantic values are `Rc`-based and must not leave the worker thread; everything here is plain data, so it crosses a channel or a socket.
- `wire`, the render-bus schema at version 2: the `RenderFrame` envelope with its per-document routing keys and validating decode, the hello/frame/delta/resync/detach message set, the machine-state and report projections a renderer paints, the node-delta patch form, and the capability advertisement.
  Envelope fields are private and the five body-specific constructors are the only way to build one.

Serialization is opt-in.
`default = []` and `codecs = ["dep:serde"]`, so a renderer that needs only the in-process types pays nothing.

`gandr-surface-grammar`'s mold highlighter is the only in-tree producer of these spans today.

## Planned but absent

- Every consumer beyond that one producer: the bus server, every transport, the language-server adapter, the attach handshake and its token, the bounded structural decoder, and the delta engine.
  The crate carries their shapes and none of their behaviour.
- The delta path is present but inert.
  The delta body, the node patch form and the capability flags that advertise them exist; the shipped capability set is whole-frame only.
- Session-typing the bus.
  The message set is deliberately shaped as the projection of a binary session, so the endpoint can become a typed one without breaking consumers, but nothing types it today.

## Using it

Build a frame through the body-specific constructor and check the schema version before decoding.

```rust
use gandr_surface_render_remote::wire::RenderFrame;
use gandr_surface_render_remote::wire::WIRE_SCHEMA_VERSION;

let frame = RenderFrame::frame(doc_uri, doc_version, report_view, machine_view);
```

Every document-scoped message carries its document identity and the editor-protocol document version the projection describes.
A renderer paints a frame only while that version matches the buffer it is showing and asks for a resync otherwise, so a stale frame is dropped rather than painted.

## Theoretical ideas relied on

Binary session types, as the shape the message set is designed to admit later; protocol schema versioning with validating decode; the presentation projection as a firewall between semantic values and renderers.

## Primary references

- Kohei Honda, Vasco T. Vasconcelos and Makoto Kubo, _Language Primitives and Type Discipline for Structured Communication-Based Programming_, Programming Languages and Systems (ESOP 1998), Lecture Notes in Computer Science 1381, 122–138, `doi:10.1007/BFb0053567` — the binary session-type discipline the message set is shaped to accept.
- Simon J. Gay and Malcolm Hole, _Subtyping for Session Types in the Pi Calculus_, Acta Informatica 42:2–3 (2005), 191–225, `doi:10.1007/s00236-005-0177-z` — the subtyping account behind the intent that a later typed endpoint not break today's consumers.
