//! The document builder: the only insertion path into an arena.
//!
//! Construction is explicit. There is no infallible constructor and no
//! `Default`, because both would have to conceal either a key-minting failure
//! or a capacity failure, and concealing those is how a resource bound stops
//! being a bound. Every constructor takes handles, checks them, charges the
//! meter, and returns a handle or a typed error.
//!
//! Handles are shareable: naming the same handle twice builds a shared
//! subdocument rather than a copy, and the second edge is charged as an edge
//! rather than as a new node.
//!
//! # The construction surface
//!
//! These are the operations slice one implements. The names, the order, the
//! arity, and the fallibility are binding; the argument types are this crate's
//! nominal scalars rather than bare primitives.
//!
//! ```text
//! impl<'meter> DocBuilder<'meter> {
//!     pub fn try_new(meter: &'meter mut BuildMeter) -> Result<Self, BuildError>;
//!     pub fn empty(&self) -> DocId;
//!     pub fn text(&mut self, text: TextSource<'_>) -> Result<DocId, BuildError>;
//!     pub fn text_owned(&mut self, text: TextOwned) -> Result<DocId, BuildError>;
//!     pub fn verbatim(&mut self, text: VerbatimSource<'_>) -> Result<DocId, BuildError>;
//!     pub fn verbatim_owned(&mut self, text: VerbatimOwned) -> Result<DocId, BuildError>;
//!     pub fn line(&self) -> DocId;
//!     pub fn hard_line(&self) -> DocId;
//!     pub fn concat(&mut self, left: DocId, right: DocId) -> Result<DocId, BuildError>;
//!     pub fn concat_all<Docs>(&mut self, docs: Docs) -> Result<DocId, BuildError>
//!     where
//!         Docs: IntoIterator<Item = DocId>;
//!     pub fn nest(&mut self, amount: NestAmount, doc: DocId) -> Result<DocId, BuildError>;
//!     pub fn align(&mut self, doc: DocId) -> Result<DocId, BuildError>;
//!     pub fn choice(&mut self, left: DocId, right: DocId) -> Result<DocId, BuildError>;
//!     pub fn flatten(&mut self, doc: DocId) -> Result<DocId, BuildError>;
//!     pub fn group(&mut self, doc: DocId) -> Result<DocId, BuildError>;
//!     pub fn finish(self) -> Result<DocArena, BuildError>;
//! }
//! ```
//!
//! `try_new` mints the arena key and inserts the singleton empty, line, and
//! hard-line nodes. Those three count against the node ceiling, so a ceiling
//! below three refuses immediately rather than pretending to succeed.
//!
//! `concat_all` builds a balanced concatenation and preserves input order.
//!
//! `group(d)` is exactly `choice(d, flatten(d))`, in that order. The
//! unflattened form is deliberately on the left: when both branches end up
//! width-tainted the merge keeps the left promise unforced, so the vertical
//! form is what a document too wide to lay out falls back to. At ordinary
//! widths the cost order still selects the flattened form when it is better,
//! so the left bias costs nothing where it does not matter.
//!
//! # Finalization
//!
//! `finish` runs one iterative, memoized flatten pass under the same build
//! limits. It visits each node once, consults a deterministic structural
//! interner, appends at most one distinct flattened image per node, keeps the
//! original identity when flattening changes nothing, and records the result.
//! It receives no render options: flattening is a property of the document, not
//! of a page width. Existing identities never move, and a limit or allocation
//! failure returns without producing a partial arena.
//!
//! Finalization moves its fallibly grown stores straight into the arena and
//! keeps their capacity. It does not shrink them or convert them through a
//! boxed slice, because that step can allocate and an allocation on the way out
//! of a fallible builder is exactly the failure the fallible growth was for.
//!
//! # Accounting
//!
//! - One build step per checked input edge and per interner probe.
//! - A newly stored node, text identity, verbatim identity, byte, or physical
//!   fragment is charged exactly once to its own ceiling.
//! - A second edge to an existing handle is charged as an edge, never as a new
//!   node.
//! - Each visit, flatten edge, and interner probe during finalization charges a
//!   build step, and each distinct flattened image charges a node before it is
//!   inserted.
//! - Exceeding a build ceiling leaves the builder unfinalized, and a build
//!   failure can never consume a render counter.
//!
//! Finalization needs a deterministic structural interner. Its representation
//! is slice one's choice — a dense per-node table or an insertion-ordered map
//! both qualify — under one binding requirement: no iteration order that
//! depends on a hash seed may reach the result, because the same document must
//! finalize to the same arena on every run. Whatever state that needs becomes a
//! private field of the builder.
//!
//! # Recursion
//!
//! Nothing here recurses over caller-supplied structure. Finalization and every
//! traversal use an explicit heap work stack charged to the finalize-stack
//! allocation site, because a deep document is ordinary input and a native
//! stack is not a resource this crate is allowed to exhaust.

use crate::arena::ArenaKey;
use crate::arena::DocNode;
use crate::arena::VerbatimText;
use crate::limits::BuildMeter;

/// The mutable side of a document under construction.
///
/// The builder holds the arena key it will seal into the finished arena and
/// exclusively borrows the meter every charge is recorded against, so a
/// document and its accounting cannot come apart.
///
/// # Contract
/// - requires: the builder exclusively borrows one build meter for its life.
/// - ensures: every stored edge names an earlier node in this builder.
/// - provides: the only insertion path into a document arena.
/// - fails: every constructor returns a build error rather than panicking.
/// - panics: none.
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "slice one reads these; the expectation fails as soon as it does"
)]
pub struct DocBuilder<'meter>
{
    /// The key sealed into the finished arena.
    arena: ArenaKey,
    /// The exclusively borrowed build meter every charge lands on.
    meter: &'meter mut BuildMeter,
    /// The grow-only document node store.
    nodes: Vec<DocNode>,
    /// The grow-only text store.
    texts: Vec<String>,
    /// The grow-only verbatim store.
    verbatim: Vec<VerbatimText>,
}
