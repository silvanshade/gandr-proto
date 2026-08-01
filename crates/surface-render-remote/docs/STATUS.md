# Status

Crate scope: `crates/surface-render-remote` (package `gandr-surface-render-remote`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the leaf wire-protocol layer of the render-bus / editor-integration surface (`docs/gandr/spec/implementation.md` §"The surface pipeline"): plain, `Send`-safe, `serde`-ready data that the pipeline projects and that renderers (a TUI, a future editor webview, an agent) consume.
  It parses, lowers, types, and marks nothing — the renderer firewall — and depends on no other workspace crate, so downstream consumers link it without a cycle.
* Naming: this crate is the _remote_ face of the render surface — the typing-machine inspection wire protocol (`present` + `wire`) that a renderer consumes over a channel or socket.
  It is named to dovetail with the planned `surface-render` presentation printer (the wyrd-era `gandr-pretty`), which will own the in-process rendering; `surface-render-remote` owns the projected/serialized wire vocabulary that crosses the process boundary.
* Ported verbatim at rung F0 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-render-proto` crate.
  The recut renames the package to `gandr-surface-render-remote` (owner rename table) and drops the retired tracker-bead comments and wyrd ADR citations (staging call O3 and the current-terms provenance rule), with no change to types or wire shape.
* `present` — the `Send`-safe presentation seam graduated from the TUI present layer: highlight spans (`HlSpan`/`HlRole`), Hazel-style mark spans (`MarkSpan`), diagnostic and goal cards (`DiagCard`/`GoalCard`), cursor-type and completion candidates (`CursorTy`/`Candidate`), the preview and transcript frames (`PreviewFrame`/`TranscriptBlock`), and the total byte-offset ↔ (row, column) projection (`pos_of_byte`/`byte_of_pos`).
* `wire` — the versioned render-bus frame + delta schema: the `RenderFrame` envelope with its per-document routing keys and validating decode (`WIRE_SCHEMA_VERSION`, lockstep `doc_uri`/`doc_version`, body-scope checks), the `FrameBody` message set (hello/frame/delta/resync/detach), the `MachineView`/`ReportView` projections a renderer paints, and the `ServerCaps` capability advertisement.
  It is shaped as the projection of a binary session so it can later be session-typed without breaking consumers.
* Feature posture carried faithfully: `default = []`, `codecs = ["dep:serde"]` (default-off serialization; a renderer needing only the in-process types pays nothing), `full = ["codecs"]`.
  The sole dependency is optional `serde` (inheriting the workspace `derive`+`std` features), with `serde_json` a dev-dependency for the `codecs` JSON round-trip tests.
* Tests: the crate carries its `present` and `wire` unit tests verbatim — constructor field-fidelity, the position round-trip and clamping suite, and the `codecs`-gated JSON-shape goldens — green under nextest.

## designed direction

* The render-bus consumers (`surface-grammar`'s mold highlighter, and later an LSP adapter / TUI) land in the F1+ rungs; this crate is only their shared wire vocabulary.
* The delta/obligation growth path (`FrameBody::Delta`, `NodeDelta`, the `ServerCaps` delta/obligation flags) and session badges are present as the transparent scaling lever but inert in the MVP posture (`ServerCaps::mvp` is whole-frame only).
* The companion `surface-render` in-process printer (wyrd-era `gandr-pretty`) is not yet ported; when it lands, the two render crates share this wire vocabulary as their one source of truth.

## open decision

* Whether the mold highlighter is ported into `surface-grammar` (keeping this leaf's feature graph intact) or feature-gated out of the minimal grammar cut is staging call O2, resolved at the F1 rung; F0 lands the leaf either way.
