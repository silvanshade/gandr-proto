//! The typing-machine inspection **wire protocol** — the leaf crate of the
//! render-bus / editor-integration surface (the inspection-protocol design).
//!
//! This crate holds only *wire types*: plain, `Send`-safe, `serde`-ready data
//! that the pipeline projects and that renderers (a TUI, an editor webview, an
//! agent) consume. It **parses, lowers, types, and marks nothing** — the
//! renderer firewall. It is a true leaf: it depends on no other workspace
//! crate, so a language server or a render bus can consume it without a cycle,
//! and a minimal renderer can link it without pulling in the checker.
//!
//! Two layers live here:
//!
//! - [`diagnostic`] — the stable diagnostic-code registry and localizable
//!   message arguments shared by every renderer.
//! - [`present`] — the `Send`-safe presentation seam: highlight/mark spans,
//!   diagnostic and goal cards, the preview and transcript frames, and the byte
//!   ↔ position projection. In-process renderers and the serialized bus carry
//!   one vocabulary.
//! - [`wire`] — the versioned render-bus frame + delta schema: the frame
//!   envelope, the machine-state projection, the incremental delta form, and
//!   the scalar machine summary. Shaped as the projection of a binary session
//!   so it can later be session-typed without breaking consumers.
//!
//! # `codecs`
//!
//! Serialization rides the default-off `codecs` feature. A renderer that only
//! needs the in-process types pays nothing by default; the bus and
//! language-server adapters opt in when they need the wire image.

pub mod diagnostic;
pub mod present;
pub mod wire;
