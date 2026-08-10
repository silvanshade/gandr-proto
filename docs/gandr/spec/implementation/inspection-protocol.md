# The typing-machine inspection protocol

The checker's state is reified as data, and the reason for reifying it is that **something can watch it while a program is being written** — goals, marks, derivations, and obligations delivered to a consumer as they change, for a human renderer and for an agent alike.

This document fixes the protocol that carries that stream, the seam that decides what rides where, and the wire contract a remote renderer is held to.

Its organizing commitment is the one the interactive-surface design takes (now in the project's research vault — the corpus README's migration banner), restated in protocol terms: **the editor view, the language-server channel, and the agent stream are projections of one reified checker state**, and no renderer parses, lowers, types, marks, deduplicates, or reconstructs a semantic fact.
The in-process worker-to-renderer firewall becomes a **wire** boundary: a proof-owning server constructs one checked neutral projection, the leaf protocol mirrors it, and a remote renderer receives a self-contained bounded message and nothing else.

What this document does **not** fix is the editor feature set — hover, completion, and format wiring — the choice of language-server framework, the incremental delta engine, or the agent-facing decode side.
It fixes the **protocol shapes and the crate and transport layout** those consume, and the sequencing that lets a language server ship before the bus does.

## What is built, and what this document describes

**Built, and verified against the tree at write time by reading definitions rather than the prose beside them.**

* **The leaf wire crate exists, and it is a true leaf.** `gandr-surface-render-remote` declares exactly one dependency — `serde`, optional, behind a default-off `codecs` feature — and no workspace crate at all.
  A renderer that needs only the in-process types pays nothing for serialization, and a remote client links no checker-side crate.
* **The frame envelope is realized at wire schema version 1.** Its `wire` module carries `WIRE_SCHEMA_VERSION`, the `RenderFrame` envelope, the five-variant `FrameBody`, the `ReportView` and `MachineView` projections, `ServerCaps`, and the `NodeDelta` patch form.
* **The envelope's invariants are enforced at decode, not merely documented.** `RenderFrame`'s fields are private; the five body-specific constructors are the only public construction path; and the deserializer routes through a validator that rejects a schema-version mismatch, a `doc_uri` and `doc_version` pair that is not present or absent in lockstep, and any body whose scope disagrees with its routing keys.
* **The presentation seam is realized beside it.** The `present` module carries the highlight and mark spans, the diagnostic and goal cards, cursor-type and completion candidates, the preview and transcript frames, and the total byte-offset to row-and-column projection.
* **One in-tree consumer exists.** `gandr-surface-grammar`'s `highlight` module produces `HlSpan` values in the crate's `HlRole` vocabulary — the only place in this tree that the wire types are produced from real input.
* **The report envelope the lightweight channel projects from is at schema version 2.** `gandr-surface-engine`'s `diag` module carries diagnostics, hole goals, typed marks, resolved attributes, and a reserved obligation slot in one versioned envelope.

**Designed, and not built.** Everything else in this document: the bus server, every transport, the language-server adapter, the attach handshake and its token, the bounded structural decoder, the delta engine, and the whole version-2 diagnostics-transport contract.
No language-server, bus, terminal-renderer, or neutral-diagnostics crate exists in this tree under any name.

**One as-built divergence worth naming, because a reader comparing the projection against the machine will otherwise look for a state that is not there.** `ControlView` carries a `Done` variant documented as the projection's idle sentinel — the machine is at rest for the frame emitted — and it is deliberately **not** one of the typing machine's own control states.
It is a property of the projection, not of the machine being projected.

## The two channels, and what decides the seam

The Language Server Protocol is the obvious editor channel, and it does not carry the interesting payload.
It models **request and response plus server notifications over one editor-owned connection**, not a **subscription to a versioned, incremental derivation stream**, and the difference is structural rather than a matter of message size.

So the inspection surface is **two channels**:

| channel            | carries                                                                             | rides                                                                                                             |
| ------------------ | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| the editor channel | lifecycle, cursor-local state, and the scalar machine digest                        | custom methods on the normal editor JSON-RPC connection, projected directly from the report and the machine state |
| the render bus     | the heavy interactive payload — derivations, diagnostics, marks, goals, obligations | a separate localhost stream, pushed against an exact document version, feeding every renderer at once             |

**The seam between them is the payload weight, not the data source.** Both channels project the _same_ reified state; neither is authoritative over the other, and a fact that one has and the other lacks is a gap in the projection rather than a reason to add a second source.

## The design space

Five choices, each stated with the alternatives it rules out.
The alternatives are recorded because each is the shape someone reaches for first.

### The channel-count choice

_Everything over the editor protocol_, as vendor-experimental payloads, keeps a single connection but forces a derivation stream into whatever the editor's protocol client happens to carry, forces inherently push-shaped data into synchronous request framing, and **strands every non-editor consumer** — a standalone terminal renderer, an agent.

_Everything over a bespoke bus_, with the editor protocol demoted to a dumb transport, throws away what that protocol gives for free: the diagnostics gutter, hover, and the client lifecycle.

**Taken: two channels.** The editor protocol does what it is good at — editor-native, cheap, pull-or-notify, cursor-local — and the bus does what it is not: subscription, push, incremental, versioned, multi-renderer.

### The payload-granularity choice

Whole-frame messages re-send a document's checked projection on every change.
They are trivially coherent and cheap at script scale.
Node-keyed **deltas** are the scaling endpoint, and they depend on the incremental machinery and on identities that are stable under editing rather than derived from source position.

**Taken: whole checked frames for the first cut, deltas as a transparent scaling lever** behind the independently versioned envelope.
The lever is transparent precisely because the envelope does not change when it is pulled.

### The versioning-and-coherence choice

Six things evolve at different rates and **are never aliases for one another**: the outer bus schema, the nested diagnostic-frame schema, the report schema that the diagnostic frame describes, the source snapshot, the semantic presentation state, and the editor's document version.

**Taken:** every document-scoped frame carries the schema version, the document URI, and the document version; the nested diagnostic frame additionally carries its own frame version, the report schema version, a source-snapshot identity, a presentation-state identity, a nullable presentation-view identity, its own document URI and version, and two digests.
A renderer accepts only an exactly supported schema and the exact current document version, validates the bounded digests and the state fences, and otherwise **drops the whole message**.
Under edit pressure the server coalesces to the latest work per document.

### The transport choice

Framing is transport-agnostic behind a trait.
A unix domain socket is the same-host default; a loopback TCP port serves consumers that cannot open one; a WebSocket serves a browser context, which cannot open a unix socket at all; and a QUIC endpoint addressed by node identity is the cross-machine growth slot, taking the same address-versus-location split as [[proposed/packages|the package design]].
The named candidate for that slot is `iroh`, at its 1.0 line.

**Excluded: a gossip mesh.** The bus is point-to-point, and `iroh-gossip` and `iroh-docs` were pre-1.0 when the editor-integration survey behind this design ran in mid-2026 — a version claim inherited from that survey and **not re-verified here**.

### The session-typeability choice

The checker-as-service is **itself a session protocol**: attach, greet, then a loop of pushes with a client-initiated resynchronization and either-side close.
"Subscribe to the derivation stream" is therefore the system's own protocol discipline applied to itself rather than a bespoke messaging layer bolted on.

**Taken: shape the wire messages now as the projection of a binary session**, so that when session types land the bus server _becomes_ a typed endpoint and today's JSON frames are that protocol's serialized messages, with no consumer break.

## The lightweight channel

Three custom methods ride the normal editor JSON-RPC connection, namespaced `gandr/`.

**`gandr/status`** is a server-to-client **notification** carrying the check lifecycle for a document version — idle, checking, ok, or a count of errors.
It drives the editor's status item, and on an editor whose extension API exposes no custom panel — Zed, as the survey behind this design found — it is the only live-state surface there is.
It is also the handshake that bootstraps a bus attach: it advertises whether a bus endpoint is available and where.

**`gandr/goalAtCursor`** is a client-to-server **request** that takes a document and a position and returns the goal at that hole — the expected type and the local context beyond the prelude.
It is projected by splicing a synthetic hole at the cursor and reading its checking-position expected type together with its context, which is the same query the interactive-surface design's goal-directed completion runs (migrated — the corpus README's migration banner).
This is the queryable payoff of the whole reified-state design, surfaced without a panel.

**`gandr/machineSummary`** is a client-to-server **request** returning a _small_ scalar digest — step count, frame-stack depth, outstanding-obligation count, solver-trail depth — cheap enough for a status hover.
The **full** state, meaning frames, derivations, and contexts, is never sent here; it is bus-only, and keeping it so is what stops the lightweight channel from growing into a second bus.

These three are the entire interactive surface an editor-protocol-only client can consume.

## The render bus

The bus is a single server, colocated with the language server, owning the one live session and pipeline and projecting its reified state to every attached renderer.

### The frame envelope

Each message is an envelope of a schema version, an optional document URI, an optional document version, and a body.
The routing keys are optional **because two of the five bodies are not about one document**: the greeting spans the tracked document set and the close is connection lifecycle.
They are `Some` for the document-scoped bodies and `None` for the connection-scoped ones, and the body-specific constructors are what maintain that invariant, so the type stays one envelope without lying about lifecycle messages.

The five bodies:

| body      | wire tag  | direction                   | payload                                                                                    |
| --------- | --------- | --------------------------- | ------------------------------------------------------------------------------------------ |
| greeting  | `hello`   | server to client, on attach | the tracked document set with their versions, and the server's capabilities                |
| frame     | `frame`   | server to client            | one complete projection for an exact document and state — a report view and a machine view |
| delta     | `delta`   | server to client            | a patch against an acknowledged base version, as node-keyed changes                        |
| resync    | `resync`  | client to server            | "I fell behind, resend a full frame" for the envelope's document version                   |
| detach    | `detach`  | either direction            | close the stream                                                                           |

The body is **adjacently tagged**: the wire spelling above rides a `kind` field and the payload rides a `data` field beside it, which is what lets a decoder dispatch on the tag before allocating the body.

**The resync body carries no version of its own.** The envelope's document version _is_ the version to resend, which is the whole reason the routing keys sit on the envelope rather than inside each body.

**The frame body's report view is a presentation projection, never the pipeline's report.** At wire schema version 1 it carries the highlight spans, the total-marking spans, the fail-fast diagnostic cards, and the goal cards — exactly the shape an in-process renderer already consumes.
The structured pipeline report stays the lightweight channel's authority and the archival agent-facing JSON surface, and **it never crosses the render socket**; a remote decoder never treats report bytes as proof.

The mark spans carry the **total-marking** discipline, in which an ill-typed program still has a typed reading and an error is localized as a mark on the offending node rather than aborting the check [@zhao-maroof-dukkipati-blinn-pan-omar-2024-total-type-error].

**The machine view is the reified typing machine, projected to plain data.** It carries the derivation forest, the control register, the frame stack summarized top-first, and the scalar digest — everything a renderer needs to paint the machine, as strings and identities rather than as the reference-counted semantic values, which must not leave the worker.

A derivation node carries its identity, the rule name, the rendered sub-expression, the direction it was typed in, the call-by-push-value layer, an optional world badge, the bindings this node added to the context, the endpoints consumed below it, its resulting type when it produced one, its sub-derivations, an optional session before-and-after badge, and an optional rendered grade.

**A node stores a context _delta_, not a snapshot**, and a renderer reconstructs any node's full context by folding deltas along the path from the root.
That is what keeps a derivation forest from being quadratic in the context.

**A node's identity is the machine's monotone step counter**, which is what makes it stable enough to key a patch against.

### The versioning and coalescing discipline

The server tags every frame and every delta with the **exact** document version it projects.
It never emits a delta whose base version the client has not acknowledged, and under edit pressure it coalesces superseded work rather than queueing it.

A renderer accepts a document-scoped message **only when its document version exactly matches the buffer it currently shows**.
An older, newer, absent, malformed, digest-failed, or state-incoherent message is dropped whole — it renders nothing and clears nothing newer — and it may trigger a resynchronization.

**This is the lesson of an existing language server rather than a precaution invented here.** The editor-integration survey that produced this design took the version-fence discipline from tinymist, the Typst language server, where mismatched-version rendering was the failure being avoided; the attribution is **inherited from that survey and not independently verified**.

### One protocol, many renderers

An editor webview, an external terminal renderer, and a streaming agent consume **the same checked frame**.

The server may reuse an in-process renderer's projection machinery, but only the server-side adapter can pair that projection with proof-checked diagnostics and with the source, state, and version fences.
**If a renderer lacks a semantic fact, the fact is added to the pipeline's report and to the neutral projection** — a remote renderer never consults checker internals and never invents a second policy.

That rule is what makes "one protocol, two renderers" a genuine deduplication rather than a slogan, and it is the same firewall rule stated in the interactive-surface design's terms (migrated — the corpus README's migration banner), moved onto the wire.

### The session-typeable shape

The message set is the projection of a binary session, written here untyped:

```text
inspect ≙ ?Attach . !Hello . μ X. (
      !Frame . X          -- server pushes a projection
    ⊕ !Delta . X          -- or an incremental patch
    & ?Resync . X         -- client may ask for a full resend
    & ?Detach . end )     -- either side closes
```

When session types land, the bus server becomes an endpoint of this session and the JSON envelope becomes its serialized messages, so consumers written against the envelope keep working.
The shape is **cheap insurance rather than a near-term feature**, and it is worth having only because committing it now costs a naming discipline and nothing else.

## Transport behind a trait

The transport seam is one trait: a server-side accept that awaits a renderer, a client-side connect, and a connection carrying framed, length-delimited messages.

| transport          | role                                                         | note                                                                                                     |
| ------------------ | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| unix domain socket | the same-host default, serving an attached terminal renderer | the first cut's only transport                                                                           |
| loopback TCP       | consumers that cannot open a unix socket                     | same framing                                                                                             |
| WebSocket          | an editor webview                                            | a browser context cannot open a unix socket, so the extension host bridges a loopback WebSocket endpoint |
| QUIC endpoint      | cross-machine attach                                         | addressed by node identity, the address-versus-location split again                                      |

The WebSocket transport is folded into the trait from the start even though the webview that needs it comes later, because a transport added after the trait has shipped is a change to the trait's users and a transport added before it is not.

### Localhost is not trust-free

A unix socket or a loopback TCP port is reachable by **any local process of the same user**, and the bus streams a program's full typed structure.

So the attach handshake carries a **per-session capability token**, minted by the language server and handed to a renderer out of band — through the status advertisement for the editor, and through a command-line flag or environment variable for an attaching terminal renderer.
An attach without the token is refused.

**This is a floor, not a grant model.** It is what the surface needs before it is reachable at all, and it graduates into [[capability-model|the capability model]] rather than substituting for it.

## The crate layout

Three crates, each renderer-firewall clean, meaning none of them parses, lowers, types, or marks.

| crate                                               | role                                                                                                                        | depends on                                                                                                           |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| the leaf wire crate — `gandr-surface-render-remote` | closed serialization mirrors, the outer envelope, the canonical diagnostic-frame mirror, and the bounded structural decoder | no workspace crate; the decoder feature owns only version-pinned serialization, JSON, and hashing dependencies       |
| the bus crate — not built                           | client transport and framing, plus an optional server projection and accept loop                                            | client build: the leaf and a transport, nothing else; server build: additionally the diagnostics and pipeline crates |
| the language-server crate — not built               | the direct report-to-protocol adapter, the custom methods, document synchronization, and the optional bus advertisement     | the pipeline and the neutral diagnostics layer; the bus and the leaf only behind a feature                           |

**Colocation is what keeps the two channels honest.** The language-server process owns the one live pipeline and embeds or supervises the bus server, so both channels project the same checked state and no second source of truth is introduced.
A build with the bus feature off is language-server-only and drops the bus, protocol, and transport dependencies entirely.

**The leaf property is a dependency fact, not a layering aspiration**, and it is the one part of this layout the tree already realizes.
The leaf never imports the neutral diagnostics crate: their independently closed shapes meet only in the server-side adapter and in the drift and golden tests that hold them together.

## The first cut and the growth path

**The first cut.**

* The **language server ships without the bus**: diagnostics, hover goal-at-cursor, and the status notification, over a whole-file recheck inside the script-scale latency budget.
* The **bus's first cut** is the unix-socket transport, whole checked frames, exact source, state, and document-version fencing, capability-token attach, and one attached terminal renderer.
* The renderers of that cut are the attached terminal renderer and the streaming agent, consuming the same checked candidates.
  An editor webview is deliberately not in it.

**The growth path**, each item additive behind the shapes already committed: node-keyed delta frames and obligation deltas once the incremental machinery lands and the reserved obligation slot populates; the WebSocket transport with the editor webview behind it; the QUIC transport for cross-machine attach; and session-typing the bus once session types exist.

**The load-bearing sequencing claim** is worth stating as a claim rather than leaving it implicit in the ordering: **the protocol shapes — the frame envelope, the versioning, the transport trait, and the firewall — are the durable deliverable, and the transports, the delta engine, and the renderers are swappable behind them.** So the first cut commits the shapes plus the cheapest transport-and-renderer pair, and everything after it is addition rather than revision.

## The version-2 diagnostics-transport contract

A whole pipeline report, a serialized proof, and a second diagnostic, goal, or mark policy are each **forbidden on the socket**, and this section is the contract that forbids them.
It governs the **next** wire schema: this tree is at version 1, and the cutover to version 2 is atomic.

### Authority, projection, and leaf ownership

The **server owns the proof**, the canonical parsed source, the same-run report, and the optional checked presentation state and view.
It validates the source bundle, constructs one neutral diagnostic bundle, applies canonical selection and truncation **once**, and projects a diagnostic frame.
The **same** proof-owned transaction projects the machine view, which is what makes the two coherent by construction rather than by comparison after the fact.

Only the server-side adapter converts those checked neutral values into leaf wire mirrors.

The document-scoped frame body at version 2 is exactly:

```text
Frame {
  report: {
    highlights: HlSpan[],
    diagnostic_frame: DiagnosticFrameWireV1
  },
  machine: {
    presentation_state_id: Hex256,
    derivation: DerivationNode[],
    control: ControlView,
    frames: FrameSummary[],
    summary: MachineSummary
  }
}
```

**There is exactly one semantic diagnostics path.** Version 2 has no mark, diagnostic, or goal vectors beside the frame: remote cards, error underlines, goal panels, and agent items are all derived from the frame's candidates and their supplemental links after a checked decode.
Syntax highlights remain a separate field because they are not diagnostics.

The nested frame is carried **byte-for-byte**, not as a separately interpreted lookalike.

### Canonical bytes and the two digests

For a stream transport, one message is a **four-byte big-endian unsigned payload length** followed by that many UTF-8 JSON bytes.
A WebSocket carries one such payload per message without the length prefix and enforces the same cap.

The payload is **canonical**, and every clause of that is a rejection rule rather than a preference: no byte-order mark and no insignificant whitespace; outer fields in schema-version, document-URI, document-version, body order; optional fields present as null rather than omitted; adjacent body fields in kind-then-data order; nested fields in declaration order; arrays in canonical projection order; shortest decimal integers; fixed lowercase hexadecimal identities; and one exact string-escaping convention.
**Unknown, duplicate, reordered, and semantically equivalent noncanonical spellings all reject.**

The decoder drives the JSON parser through private bounded seeds and visitors, charges the already-capped input and every retained field and vector **before** any reserve or copy, performs its own canonical re-encode, and exposes only a checked frame.

Two digests are required, both full 32-byte lowercase hexadecimal BLAKE3, both domain-separated and length-framed.
Writing `len_prefix(x)` for the little-endian 64-bit length of `x` followed by `x`:

```text
source_digest_v1(bytes) =
  BLAKE3(len_prefix("gandr-diagnostic-frame-source-v1") || len_prefix(bytes))

frame_digest_v1(fields) =
  BLAKE3(len_prefix("gandr-diagnostic-frame-v1") || len_prefix(fields))
```

The frame's `source_digest` field is `source_digest_v1(source_utf8_bytes)`, and its `frame_digest` field is `frame_digest_v1` applied to the canonical JSON of **every diagnostic-frame field except `frame_digest` itself**.
Both names denote exactly the domain-separated, length-framed expressions above and nothing looser.

The decoder boundedly re-encodes the parsed payload and **requires the exact received bytes before checking the digest**, which is what makes the digest a check on the payload rather than on the decoder's own output.

**These digests detect accidental corruption and bind the self-contained projection together.** They are **not signatures**: they do not replace the per-session capability token, and they do not stand in for an authenticated cross-machine transport.

### The four fences

A version-2 frame has a non-null document URI and document version on the envelope, and its nested frame requires report schema version 3, frame version 1, a non-null document URI, and a non-null document version.
Before emission and **again** before rendering, four equalities must hold:

```text
envelope.doc_uri                 == diagnostic_frame.document_uri
envelope.doc_version             == diagnostic_frame.document_version
machine.presentation_state_id    == diagnostic_frame.presentation_state_id
source_digest_v1(source_bytes)   == diagnostic_frame.source_digest
```

**URI comparison is exact byte comparison** after the document layer has chosen its canonical URI — no basename match, no display-name match, no normalization, and no equal-source shortcut.
The consequence is deliberate: **two documents with equal bytes and equal integer versions remain non-interchangeable.**

The source-snapshot identity binds the parser-owned source, grammar, and obligation-schema snapshot; the presentation-state identity binds the semantic run and store.
A remote client can recompute **neither**, and accepts them only from the capability-selected server while still validating every locally checkable shape and equality constraint.
The presentation-view identity is nullable, and frame version 1 grants no authority to dereference a presented identity.

Any unsupported schema, absent fence, URI or version mismatch, digest failure, diagnostic-versus-machine state mismatch, invalid span, stale buffer, or failure of the final pre-render recheck **drops the whole message**: it renders nothing, clears nothing newer, and may request a resynchronization.

### Decode, aggregate, and output ceilings

Every limit below is a **hard implementation ceiling**; configuration may lower one and may never raise one.

| resource                                                           | hard ceiling | required failure point                                     |
| ------------------------------------------------------------------ | -----------: | ---------------------------------------------------------- |
| one outer JSON payload                                             |       32 MiB | reject the length before allocating or reading the payload |
| bytes before the first schema-version value                        |    128 bytes | reject before body decode                                  |
| JSON nesting depth                                                 |           64 | reject while visiting                                      |
| the nested diagnostic frame                                        |       24 MiB | reject before retaining the nested body                    |
| the included canonical source                                      |       16 MiB | reject before source allocation                            |
| an outer or nested document URI                                    |       16 KiB | reject before allocation; exact equality required          |
| a source display name                                              |       16 KiB | reject before allocation                                   |
| one non-source string                                              |       64 KiB | reject before allocation                                   |
| aggregate non-source string bytes, including the nested frame      |       12 MiB | reject while visiting                                      |
| the greeting's document list                                       |        4,096 | reject before vector reserve                               |
| highlights                                                         |       65,536 | reject before vector reserve                               |
| derivation nodes, and node deltas                                  |  65,536 each | reject before vector reserve                               |
| frame summaries                                                    |        4,096 | reject before vector reserve                               |
| diagnostic candidates                                              |       16,384 | reject before vector reserve                               |
| distinct diagnostic, highlight, and machine spans                  |       65,536 | reject during checked span-table construction              |
| total nested diagnostic rows                                       |      262,144 | reject while visiting                                      |
| total outer vector elements, including children and context deltas |      262,144 | reject while visiting                                      |

The per-code, per-identity, per-label, per-binding, per-supplement, aggregate-string, and terminal-safe-text limits of the neutral diagnostics layer are **stricter**, and they remain in force.

**The publication default and the construction ceiling are different numbers and must not be confused.** The normal diagnostic publication default is 256 candidates _including_ its reserved truncation marker; the bus consumes exactly that selected list, its marker, and its omission metadata unless a caller configured a lower limit, and it **never exposes the 16,384 construction ceiling as a second selection policy**.

The decoder checks, in order, the outer byte length, the first member's version, the body tag, the routing-key shape, every advertised vector and string length, and the checked arithmetic — all before reserve or copy.
It then validates canonical bytes and digests, UTF-8 boundaries, source-relative spans, identities, uniqueness, ordering, links, omissions, and the cross-layer fences before returning a checked frame.
**No partial frame escapes**, and no renderer-facing entry point accepts the unchecked mirror.

Construction and decoding are `O(B + N log N)` time and `O(B + N)` bounded storage in the payload size `B` and element count `N`, with no per-candidate clone of the source or context and no pairwise duplicate scan.

### What the cutover requires

The implementation is not enabled until every one of the following holds.

* Server, leaf, and webview fixtures produce and re-encode **identical canonical bytes and digests** across languages.
* Every ceiling above and every inherited diagnostics ceiling passes at one below, exactly at, and one above the limit, with **allocation-counting assertions before reserve**.
* Hostile fixtures cover late, duplicate, unknown, and reordered fields; alternate JSON spellings; unsupported bus, frame, and report versions; malformed length prefixes; invalid UTF-8, hexadecimal, spans, links, and omissions; and digest corruption.
* Splice fixtures cover the same source and version under different URIs, the same URI and version under different semantic states, nested-versus-outer version mismatch, stale buffer versions, and mutation caught at the final pre-render recheck.
* Parity fixtures prove that the command-line terminal output, the editor protocol, the terminal renderer, the webview, and the agent stream all share candidate references, tagged order, severities and codes, supplemental links, the truncation marker, and omission counts.
* Version-2 schema fixtures **reject every legacy mark, diagnostic, and goal field**, and renderer tests prove cards, underlines, and goals all derive from the one checked frame.
* Dependency gates prove the leaf has no workspace-crate dependency and that its decoder feature contains only the serialization, JSON, and hashing dependencies; that the bus client and terminal-renderer graph has no diagnostics, pipeline, or checker dependency; that the server feature is the only conversion owner; and that the language server and the core and pipeline crates stay free of `miette`, the terminal diagnostic renderer, whose report types must never become a semantic authority.
* Compile-fail and API tests prove that **no** proof serialization, remote report decode, checked-frame-to-report conversion, or renderer-side semantic switch exists.

The cutover changes the outer schema, the leaf mirror, the server adapter, the renderer and agent decoders, and the fixtures **atomically**.
Afterwards production emits and accepts version 2 only: **version 1 is historical documentation, not a compatibility mode.**

## The example plan

This is a protocol _over_ the inspection of gandr programs, so its examples come in two kinds: gandr programs whose inspection is pedagogically rich, and protocol-level replay fixtures that stress the streaming path.

**Model programs**, each exercising one projection facet:

* a program with a single typed hole in checking position, whose commented expectation is the goal-at-cursor payload — the expected type and the local context;
* one fail-fast candidate with linked total-mark evidence beside an independent error mark, pinning canonical candidate and supplement projection without any legacy wire field;
* a small well-typed program — a bind chain over a pair — whose full derivation forest is itself the teaching artifact;
* a program annotated with its machine-summary scalars, so a reader learns to read the digest;
* once session types land, a protocol-advancing program whose derivation shows the session badges the bus carries.

**Pathological fixtures**, each pinning a failure mode of the streaming path:

* a term near the concrete-syntax depth ceiling, asserting the projection **degrades** — machine-driven and heap-safe — rather than aborting;
* an edit script that re-solves a variable the whole file mentions, forcing checkpoint invalidation to a full re-frame, which is the correctness anchor for the delta-versus-frame fallback;
* an edit-then-edit-before-acknowledgement sequence, asserting the version fence drops the superseded frame;
* once obligations populate, a program with many outstanding obligations, stressing obligation-delta coalescing.

The edit-script fixtures double as the bus integration tests, and the single-file examples double as goldens for both channels.

## Honest limits, and the named dead ends

**A second localhost channel is real operational surface** that an editor-protocol-only tool simply does not have: discovery, lifecycle, coalescing and backpressure, and a security floor, because any local process of the user can reach the socket.

**The streaming claim is true at the frame level and aspirational at the delta level.** Deltas depend on incremental reuse **at node granularity**, and the reuse that exists is at the granularity of a top-level item ([[incremental-pipeline#Granularity: the item and the node]]), so until that moves the bus re-sends whole frames.

**Session-typing the bus is speculative** until session types exist; the shape is cheap and the feature is not near-term.

Three dead ends are named so that they are not re-proposed as fresh ideas.

* **Tunnelling the heavy stream through vendor-experimental editor-protocol payloads.** It is the tempting shortcut and it is exactly the coupling the two-channel split refuses.
* **Building the editor webview before the terminal renderer.** The webview is a large lift — a WebSocket transport, bundling, and a content-security policy — with an unproven payoff over a terminal renderer, and building it before a renderer has validated the protocol inverts the risk.
* **A gossip mesh transport.** Point-to-point suffices, and the mesh layers were pre-1.0 at survey time.

**Net.** Build the language server without the bus and the unix-socket bus feeding an attached terminal renderer first; commit the shapes as the durable core; and treat the webview, the cross-machine transport, deltas, and session-typing as growth gated on real demand.

## Open questions

### inspection-protocol-question-01

**Bus discovery and lifecycle.** Does the language server always run the bus and advertise it through the status notification, or lazily spawn one on first attach?
Who owns the socket path or port lease, and how is a stale socket reclaimed?
**Disposition: carried.**

### inspection-protocol-question-02

**Whether one WebSocket could serve both renderers.** A WebSocket-only bus would let a terminal renderer and a webview share one transport, dropping the unix-and-TCP split, at the cost of a WebSocket dependency in the terminal renderer.
Whether the unix-socket default earns the second transport is unsettled.
**Disposition: carried.**

### inspection-protocol-question-03

**The delta-granularity threshold.** At what document size does whole-frame streaming stop being acceptable, forcing the delta engine, and is script scale always under it?
**Disposition: carried**, and it is measurable rather than arguable, so it should be answered by measurement when a bus exists.

### inspection-protocol-question-04

**Capability-token provenance.** Is the attach token minted per server session, per document, or per renderer, and how does it reach an attaching renderer — environment, flag, or handshake file — without leaking to other local users of the same machine?
**Disposition: carried.**

### inspection-protocol-question-05

**Multi-document workspaces.** One bus per server process serving many documents, or one stream per document?
Backpressure is per-document either way, so the question is about connection lifecycle rather than about flow control.
**Disposition: carried.**

### inspection-protocol-question-06

**Where the scalar machine summary belongs.** It is specified on the lightweight channel; whether it should _also_ be a bus body, for a renderer that wants the digest without a full frame, is unsettled.
**Disposition: carried.**

### inspection-protocol-question-07

**Whether "frame" can keep naming five things.** In this document alone it names a bus message body, the nested diagnostic frame, a typing-machine stack entry, the length-prefixing construction in the digests, and a version counter.
The digest helper is renamed here to a length-prefix, which removes the worst of the five, and the rest are left as the source had them because they are load-bearing wire names.
**Disposition: carried**, and it is a vocabulary question rather than a protocol one.

## Source and confidence

This document is written against the pre-reboot typing-machine inspection design record, including its as-built section and its accepted version-2 diagnostics-transport amendment; the prior programme's tracker rows for the two-channel design pass, the leaf-crate extraction, the frame-and-delta schema design, the language-server first cut, and the deferred protocol cutover that carries the amendment's implementation contract; and the tree — `gandr-surface-render-remote`'s `wire` and `present` modules, `gandr-surface-grammar`'s `highlight` module, and `gandr-surface-engine`'s `diag` module.

**The as-built account is high confidence and was read from definitions**: the crate's dependency set and feature posture, the schema-version constant, the envelope's private fields with their five constructors and the validator behind deserialization, the five body variants, the report and machine view shapes, the derivation node's twelve fields, the scalar summary, the three delta variants, the capability advertisement with its whole-frame default, and the report envelope's schema version.

**Three things are marked rather than resolved.**

The two version claims about the cross-machine transport library and its mesh layers are **inherited from a mid-2026 survey and not re-verified here**; so is the attribution of the version-fence discipline to an existing Typst language server.

The total-marking citation is registered here for the first time, transcribed from the contributor's reference register with a verified identifier — but **the correspondence between that calculus and this project's marker was not re-derived**, and the crate's own naming is the only evidence this pass has for the lineage.

The neutral diagnostics layer this contract depends on — the bundle, the canonical selection and truncation policy, the diagnostic frame's own field list and its stricter per-field limits, and the report schema version 3 the fences require — has **no corpus document yet**, so every reference to it here is a reference to a design that is named but not carried.
Its absorption is separately owned.
