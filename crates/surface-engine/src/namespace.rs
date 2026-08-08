//! The name-modifier scope engine: hierarchical names, the modifier language,
//! and the visible/export scope split.
//!
//! This module is the elaboration layer's answer to *which declarations are in
//! scope where*. Its first pieces are the name algebra and the carrier they
//! key:
//!
//! - [`path`] — hierarchical names as ordered segment lists;
//! - [`trie`] — the ordered-map trie carrier from names to bindings.
//!
//! # What this layer is not
//!
//! It is deliberately an internal mechanism with no surface of its own:
//!
//! - **No surface syntax.** Nothing here is spelled in gandr source.
//! - **No import lowering.** `import "URI" as name ;` parses today
//!   (`surface-grammar`'s `import_declaration` rule) and is not lowered; this
//!   module does not change that.
//! - **No operator-table or attribute-registry wiring.**
//! - **Nothing kernel-side.** Paths never leave the elaboration layer.

pub mod path;
pub mod trie;

pub use crate::namespace::path::DottedName;
pub use crate::namespace::path::NamePath;
pub use crate::namespace::path::Segment;
pub use crate::namespace::trie::Binding;
pub use crate::namespace::trie::Collision;
pub use crate::namespace::trie::Trie;
