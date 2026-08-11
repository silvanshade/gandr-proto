# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the render wire-types leaf from wyrd (F0)

- `current`: Landed `gandr-surface-render-remote`, the leaf wire-protocol crate of the typing-machine inspection render bus (rung F0 of `docs/research/front-end-port-staging.md` §9), ported verbatim from the wyrd `gandr-render-proto` crate.
- `current`: Named as the _remote_ face of the render surface — the `present` + `wire` vocabulary a renderer consumes over a channel or socket — to dovetail with the planned `surface-render` in-process printer (the wyrd-era `gandr-pretty`).
- `current`: The port renames the package to `gandr-surface-render-remote` and drops the retired tracker-bead comments and wyrd ADR citations; the `present` and `wire` types and the render-bus wire shape are byte-for-byte the wyrd surface.
- `current`: Modules — `present` (the `Send`-safe presentation seam: highlight/mark spans, diagnostic and goal cards, preview/transcript frames, and the byte ↔ position projection) and `wire` (the versioned `RenderFrame` envelope, the `FrameBody` message set, the machine-state projection, and the `ServerCaps` capability advertisement).
- `current`: Feature posture carried faithfully — `default = []`, `codecs = ["dep:serde"]`, `full = ["codecs"]`; the only dependency is optional `serde`, with `serde_json` a dev-dependency for the `codecs` JSON round-trip coverage.
- `current`: Tests carried across — the `present` and `wire` unit and `codecs` JSON-shape goldens, green under nextest.
