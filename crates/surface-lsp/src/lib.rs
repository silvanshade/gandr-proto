//! Language-server face over the reified checker state.
//!
//! Every request is a projection of the document's parse tree, lowered form,
//! and report through the span lens. This crate parses, lowers, types, and
//! marks nothing of its own: it calls the pipeline's public passes and
//! re-encodes their byte-span output into LSP positions.
//!
//! The layering keeps IO out of the protocol logic:
//!
//! - [`framing`] — Content-Length byte framing over any `BufRead`/`Write`
//! - [`rpc`] — the JSON-RPC 2.0 envelope
//! - [`protocol`] — serde types for the LSP subset served
//! - [`position`] — UTF-8 byte offset to negotiated LSP position
//! - [`analysis`] — one whole-file recheck: parse, highlight, lower, report
//! - [`tokens`] — highlight spans re-encoded as `semanticTokens` data
//! - [`server`] — the state machine: one incoming payload to outgoing messages
//!
//! The shipped driver is a synchronous stdio loop on `gandr lsp`. The
//! observable smoke path is `gandr lsp --capabilities`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

extern crate alloc;

pub mod analysis;
pub mod boundary;
pub mod framing;
pub mod position;
pub mod protocol;
pub mod rpc;
pub mod server;
pub mod tokens;
pub use crate::server::advertised_capabilities;
pub use crate::server::advertised_capabilities_text;
pub use crate::server::run_stdio;
pub use crate::tokens::TOKEN_MODIFIERS;
pub use crate::tokens::TOKEN_TYPES;
pub use crate::tokens::encode;
