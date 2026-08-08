//! The name-modifier scope engine: hierarchical names, the modifier language,
//! and the visible/export scope split.
//!
//! This module is the elaboration layer's answer to *which declarations are in
//! scope where*. Its first pieces are the name algebra and the carrier they
//! key:
//!
//! - [`path`] — hierarchical names as ordered segment lists;
//! - [`trie`] — the ordered-map trie carrier from names to bindings;
//! - [`modifier`] — the six-constructor modifier language and its iterative
//!   interpreter;
//! - [`event`] — the handler seam for the three elaboration-time events;
//! - [`scope`] — the scope value carrying a visible and an export namespace,
//!   with sections.
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

pub mod event;
pub mod modifier;
pub mod path;
pub mod scope;
pub mod trie;

pub use crate::namespace::event::EventKind;
pub use crate::namespace::event::EventRejection;
pub use crate::namespace::event::NamespaceEvent;
pub use crate::namespace::event::NamespaceEventHandler;
pub use crate::namespace::event::PermissiveHandler;
pub use crate::namespace::event::RejectionReason;
pub use crate::namespace::modifier::Modifier;
pub use crate::namespace::path::DottedName;
pub use crate::namespace::path::NamePath;
pub use crate::namespace::path::Segment;
pub use crate::namespace::scope::Scope;
pub use crate::namespace::scope::ScopeError;
pub use crate::namespace::scope::ScopeResult;
pub use crate::namespace::trie::Binding;
pub use crate::namespace::trie::Collision;
pub use crate::namespace::trie::Trie;
