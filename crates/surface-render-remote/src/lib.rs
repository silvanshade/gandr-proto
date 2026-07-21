//! The typing-machine inspection **wire protocol** — the leaf crate of the
//! render-bus / editor-integration surface
//! (`docs/gandr/spec/proposal-inspection-protocol.md`).
//!
//! This crate holds only *wire types*: plain, `Send`-safe, `serde`-ready data
//! that the pipeline projects and that renderers (the `gandr-tui`, a future
//! VS Code webview, an agent) consume. It **parses, lowers, types, and marks
//! nothing** — the renderer firewall. It is a true
//! leaf: it depends on no other workspace crate, so `gandr-lsp` and
//! `gandr-render-bus` can consume it without a cycle, and a minimal renderer
//! can link it without pulling in the checker.
//!
//! Two layers live here:
//!
//! - [`present`] — the `Send`-safe presentation seam graduated from
//!   `gandr-tui::present`: highlight/mark spans, diagnostic and goal cards, the
//!   preview and transcript frames, and the byte ↔ position projection.
//!   `gandr-tui::present` re-exports these types, so the in-process
//!   worker→present seam and the serialized bus carry one vocabulary.
//! - [`wire`] — the versioned render-bus frame + delta schema: the frame
//!   envelope, the machine-state projection, the incremental delta form, and
//!   the scalar machine summary. Shaped as the projection of a binary session
//!   so it can later be session-typed without breaking consumers (proposal §4,
//!   Axis E).
//!
//! # `codecs`
//!
//! Serialization rides the default-off `codecs` feature. A renderer that only
//! needs the in-process types (today, `gandr-tui`) pays nothing by default; the
//! bus and LSP adapters opt in when they need the wire image.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps unit and property tests readable (docs/workflow/rust.md)"
    )
)]

pub mod present;
pub mod wire;
