//! Flat concrete syntax tree storage for gandr parser clients.
//!
//! This crate owns the compact arena representation shared by the parser
//! bridge, syntax-aware tests, and incremental diffing; grammar construction,
//! molding, and melding live in their owning crates. The model layer is
//! intentionally allocation-dense: a [`Cst`] stores one source buffer, one node
//! arena, and one flattened child arena. A [`NodeId`] is a dense arena
//! location, stable only inside one `Cst` — never structural identity across
//! trees.
//!
//! Stable hashes are deterministic framed FNV-1a fingerprints over significant
//! syntax only: space nodes never affect parent hashes, grout contributes its
//! mold but not bytes, and tile nodes contribute both mold and exact bytes.
//! Hash equality is a pruning hint, not identity proof; debug diffing rechecks
//! significant structure and tile text before accepting an equal-hash subtree.
//! The hash's width, frame vocabulary, byte order, and algorithm are a
//! compatibility decision, because consumers observe hashes through
//! [`NodeView::hash`].

extern crate alloc;

mod builder;
mod diff;
mod model;

#[cfg(test)]
mod tests;

pub use builder::BuildError;
pub use builder::CstBuilder;
pub use diff::Diff;
pub use diff::SubtreeMatch;
pub use diff::diff;
pub use model::ClosingClass;
pub use model::ClosingClassTag;
pub use model::Cst;
pub use model::CstEmptiness;
pub use model::DelimSpelling;
pub use model::GrammarFingerprint;
pub use model::GroutShape;
pub use model::GroutSort;
pub use model::Material;
pub use model::MoldId;
pub use model::MoldPayload;
pub use model::NodeCount;
pub use model::NodeId;
pub use model::NodeKind;
pub use model::NodeSlot;
pub use model::NodeView;
pub use model::SourceSlice;
pub use model::SourceText;
pub use model::StableHash;
pub use model::TextLen;
pub use model::TextOffset;
pub use model::TextRange;
pub use model::TextRangeEmptiness;
