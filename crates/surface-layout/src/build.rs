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

use core::num::NonZeroU32;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use crate::arena::ArenaKey;
use crate::arena::CheckedText;
use crate::arena::DocArena;
use crate::arena::DocId;
use crate::arena::DocNode;
use crate::arena::NodeId;
use crate::arena::TextId;
use crate::arena::TextOwned;
use crate::arena::TextSource;
use crate::arena::VerbatimId;
use crate::arena::VerbatimOwned;
use crate::arena::VerbatimSource;
use crate::arena::VerbatimText;
use crate::error::BuildAllocationSite;
use crate::error::BuildArithmetic;
use crate::error::BuildError;
use crate::limits::BuildMeter;
use crate::units::NestAmount;

/// The next process-local arena key, with zero reserved as the exhausted state.
static NEXT_ARENA_KEY: AtomicU32 = AtomicU32::new(1u32);

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
pub struct DocBuilder<'meter>
{
    /// The key sealed into the finished arena.
    arena: ArenaKey,
    /// The exclusively borrowed build meter every charge lands on.
    meter: &'meter mut BuildMeter,
    /// The grow-only document node store.
    nodes: Vec<DocNode>,
    /// The grow-only text store.
    texts: Vec<CheckedText>,
    /// The grow-only verbatim store.
    verbatim: Vec<VerbatimText>,
    /// The singleton empty node identity.
    empty: NodeId,
    /// The singleton soft-line node identity.
    line: NodeId,
    /// The singleton hard-line node identity.
    hard_line: NodeId,
    /// Flattened images for original nodes in insertion order.
    flattened: Vec<NodeId>,
    /// Deterministic structural interner for flattened images.
    flatten_memo: std::collections::HashMap<DocNode, NodeId>,
    /// The shared text identity used by flattened soft lines.
    space_text: Option<TextId>,
}

impl<'meter> DocBuilder<'meter>
{
    /// Creates a builder and inserts the three algebraic singleton nodes.
    ///
    /// # Contract
    /// - requires: `meter` is unused by another builder for this lifetime.
    /// - ensures: the builder owns distinct `Empty`, `Line`, and `HardLine`
    ///   identities before any client node is accepted.
    /// - provides: the only constructor for a mutable document build.
    /// - fails: reports arena-key exhaustion, allocation failure, or a node
    ///   ceiling below three without returning a partial builder.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the exact build failure that prevents key minting or singleton
    /// insertion.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the three singleton inserts and the lower node
    ///   boundary are separated by the exact `2`/`3` limit pair.
    /// - witness: `algebra::a_builder_with_a_node_ceiling_below_three_refuses_immediately`.
    #[inline]
    #[must_use = "the builder owns the document under construction"]
    pub fn try_new(meter: &'meter mut BuildMeter) -> Result<Self, BuildError>
    {
        let arena = mint_arena_key()?;
        let mut builder = Self {
            arena,
            meter,
            nodes: Vec::new(),
            texts: Vec::new(),
            verbatim: Vec::new(),
            empty: NodeId::from(0u32),
            line: NodeId::from(0u32),
            hard_line: NodeId::from(0u32),
            flattened: Vec::new(),
            flatten_memo: std::collections::HashMap::new(),
            space_text: None,
        };
        let empty = builder.insert_node(DocNode::Empty)?;
        let line = builder.insert_node(DocNode::Line)?;
        let hard_line = builder.insert_node(DocNode::HardLine)?;
        builder.empty = empty.node_id();
        builder.line = line.node_id();
        builder.hard_line = hard_line.node_id();
        Ok(builder)
    }

    /// Returns the shared empty document handle.
    #[inline]
    #[must_use]
    pub fn empty(&self) -> DocId
    {
        self.handle(self.empty)
    }

    /// Stores newline-free borrowed text as a document node.
    ///
    /// # Contract
    /// - requires: `text` contains no carriage return, line feed, or tab.
    /// - ensures: one text identity and one document node are stored on
    ///   success.
    /// - provides: a checked text leaf for the document algebra.
    /// - fails: rejects invalid text, allocation failure, or a build ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `InvalidText`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — each forbidden scalar is distinguished from ordinary
    ///   newline-free text and the exact byte/node boundaries are checked.
    /// - witness: `algebra::text_rejects_a_carriage_return_a_line_feed_and_a_tab`.
    #[inline]
    #[must_use = "the text handle is the stored document leaf"]
    pub fn text(
        &mut self,
        text: TextSource<'_>,
    ) -> Result<DocId, BuildError>
    {
        self.store_text(CheckedText::try_from(text)?)
    }

    /// Stores newline-free owned text as a document node.
    ///
    /// # Contract
    /// - requires: `text` contains no carriage return, line feed, or tab.
    /// - ensures: the supplied allocation is moved into one text identity.
    /// - provides: an owned checked text leaf for the document algebra.
    /// - fails: rejects invalid text, allocation failure, or a build ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `InvalidText`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — owned text preserves bytes and checked width while
    ///   rejecting each forbidden scalar through the borrowed validation path.
    /// - witness: `algebra::owned_text_preserves_bytes_width_and_rejects_forbidden_scalars`.
    #[inline]
    #[must_use = "the text handle is the stored document leaf"]
    pub fn text_owned(
        &mut self,
        text: TextOwned,
    ) -> Result<DocId, BuildError>
    {
        self.store_text(CheckedText::try_from(text)?)
    }

    /// Stores borrowed verbatim text with its physical fragment metrics.
    ///
    /// # Contract
    /// - requires: `text` is UTF-8 and uses only LF or CRLF endings.
    /// - ensures: bytes and one record per physical fragment are stored
    ///   together.
    /// - provides: the opaque byte-identical document leaf.
    /// - fails: rejects bare carriage returns, allocation failure, or a build
    ///   ceiling before the node becomes reachable.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `InvalidVerbatimLineEnding`, `AllocationFailed`,
    /// `ArithmeticOverflow`, or `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — no-ending, trailing, middle, and mixed-ending inputs
    ///   distinguish bytes, scalar widths, endings, and fragment counts.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
    #[inline]
    #[must_use = "the verbatim handle is the stored document leaf"]
    pub fn verbatim(
        &mut self,
        text: VerbatimSource<'_>,
    ) -> Result<DocId, BuildError>
    {
        self.store_verbatim(VerbatimText::try_from(text)?)
    }

    /// Stores owned verbatim text with its physical fragment metrics.
    ///
    /// # Contract
    /// - requires: `text` is UTF-8 and uses only LF or CRLF endings.
    /// - ensures: the supplied bytes and their scan records move into one node.
    /// - provides: the owned opaque byte-identical document leaf.
    /// - fails: rejects bare carriage returns, allocation failure, or a build
    ///   ceiling before the node becomes reachable.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `InvalidVerbatimLineEnding`, `AllocationFailed`,
    /// `ArithmeticOverflow`, or `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — owned verbatim preserves an ending shape and rejects
    ///   a bare carriage return through the shared scanner.
    /// - witness: `algebra::owned_verbatim_preserves_an_ending_and_rejects_a_bare_carriage_return`.
    #[inline]
    #[must_use = "the verbatim handle is the stored document leaf"]
    pub fn verbatim_owned(
        &mut self,
        text: VerbatimOwned,
    ) -> Result<DocId, BuildError>
    {
        self.store_verbatim(VerbatimText::try_from(text)?)
    }

    /// Returns the shared soft-line document handle.
    #[inline]
    #[must_use]
    pub fn line(&self) -> DocId
    {
        self.handle(self.line)
    }

    /// Returns the shared hard-line document handle.
    #[inline]
    #[must_use]
    pub fn hard_line(&self) -> DocId
    {
        self.handle(self.hard_line)
    }

    /// Stores an unaligned concatenation of two existing documents.
    ///
    /// # Contract
    /// - requires: both handles belong to this builder's arena.
    /// - ensures: the left edge is visited before the right edge and both point
    ///   at earlier identities.
    /// - provides: one concatenation node preserving source order.
    /// - fails: rejects foreign handles, allocation failure, or a build
    ///   ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — swapping the edge order changes the rendered node
    ///   sequence and the exact two-edge boundary rejects at the ceiling.
    /// - witness: `algebra::concat_resolves_the_right_at_the_left_ending_column`.
    #[inline]
    #[must_use = "the concatenation handle is the new document node"]
    pub fn concat(
        &mut self,
        left: DocId,
        right: DocId,
    ) -> Result<DocId, BuildError>
    {
        let left = self.checked_edge(left)?;
        let right = self.checked_edge(right)?;
        self.meter.check_doc_node()?;
        self.nodes
            .try_reserve(1usize)
            .map_err(|_error| BuildError::AllocationFailed {
                site: BuildAllocationSite::NodeArena,
            })?;
        self.meter.charge_doc_node()?;
        self.nodes.push(DocNode::Concat { left, right });
        let Ok(index) = u32::try_from(self.nodes.len().saturating_sub(1usize))
        else {
            return Err(BuildError::NodeIdExhausted);
        };
        Ok(self.handle(NodeId::from(index)))
    }

    /// Builds a balanced concatenation while preserving iterator order.
    ///
    /// # Contract
    /// - requires: every iterator item is a handle from this builder.
    /// - ensures: the empty input is `empty`, one item is returned unchanged,
    ///   and larger inputs form a balanced left-to-right tree.
    /// - provides: bounded-depth concatenation construction for long inputs.
    /// - fails: propagates handle, allocation, arithmetic, and build-limit
    ///   errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the first typed failure raised while validating or combining the
    /// supplied handles.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — empty, singleton, and multi-item order distinguish
    ///   the unit and associativity claims without relying on implementation
    ///   ids.
    /// - witness: `algebra::concatenation_is_associative_up_to_the_rendered_node_sequence`.
    #[inline]
    #[must_use = "the balanced concatenation handle is the new document"]
    pub fn concat_all<Docs>(
        &mut self,
        docs: Docs,
    ) -> Result<DocId, BuildError>
    where
        Docs: IntoIterator<Item = DocId>,
    {
        let mut current = Vec::new();
        for doc in docs {
            self.validate_doc(doc)?;
            if current.try_reserve(1usize).is_err() {
                return Err(BuildError::AllocationFailed {
                    site: BuildAllocationSite::FinalizeStack,
                });
            }
            current.push(doc);
        }
        if current.is_empty() {
            return Ok(self.empty());
        }
        loop {
            if current.len() <= 1usize {
                break;
            }
            let mut next = Vec::new();
            let mut iter = current.into_iter();
            while let Some(left) = iter.next() {
                let Some(right) = iter.next()
                else {
                    if next.try_reserve(1usize).is_err() {
                        return Err(BuildError::AllocationFailed {
                            site: BuildAllocationSite::FinalizeStack,
                        });
                    }
                    next.push(left);
                    break;
                };
                let pair = self.concat(left, right)?;
                if next.try_reserve(1usize).is_err() {
                    return Err(BuildError::AllocationFailed {
                        site: BuildAllocationSite::FinalizeStack,
                    });
                }
                next.push(pair);
            }
            current = next;
        }
        current
            .into_iter()
            .next()
            .map_or_else(|| Ok(self.empty()), Ok)
    }

    /// Stores a checked nesting node.
    ///
    /// # Contract
    /// - requires: `doc` belongs to this builder and `amount` is the caller's
    ///   desired indentation increment.
    /// - ensures: the amount and child identity are retained without wrapping.
    /// - provides: a nesting node for later checked indentation resolution.
    /// - fails: rejects foreign handles, allocation failure, or a build
    ///   ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — distinct amounts remain distinct and no insertion
    ///   path converts them through a wrapping cast.
    /// - witness: `algebra::nest_raises_indentation_by_a_checked_amount`.
    #[inline]
    #[must_use = "the nesting handle is the new document node"]
    pub fn nest(
        &mut self,
        amount: NestAmount,
        doc: DocId,
    ) -> Result<DocId, BuildError>
    {
        let doc = self.checked_edge(doc)?;
        self.insert_node(DocNode::Nest {
            amount: u32::from(amount),
            doc,
        })
    }

    /// Stores an alignment node.
    ///
    /// # Contract
    /// - requires: `doc` belongs to this builder.
    /// - ensures: the child is retained under an alignment boundary.
    /// - provides: an alignment node for later resolution.
    /// - fails: rejects foreign handles, allocation failure, or a build
    ///   ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the child edge is checked exactly once and the node
    ///   identity is retained unchanged.
    /// - witness: `algebra::align_sets_indentation_to_the_current_column`.
    #[inline]
    #[must_use = "the alignment handle is the new document node"]
    pub fn align(
        &mut self,
        doc: DocId,
    ) -> Result<DocId, BuildError>
    {
        let doc = self.checked_edge(doc)?;
        self.insert_node(DocNode::Align { doc })
    }

    /// Stores an arbitrary choice between two existing documents.
    ///
    /// # Contract
    /// - requires: both handles belong to this builder.
    /// - ensures: the left branch precedes the right branch and ties retain
    ///   that order for later resolution.
    /// - provides: a choice node with both alternatives intact.
    /// - fails: rejects foreign handles, allocation failure, or a build
    ///   ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — exchanging alternatives changes the declared tie
    ///   projection while preserving the two-edge accounting boundary.
    /// - witness: `algebra::group_is_choice_of_the_unflattened_form_then_the_flattened_form`.
    #[inline]
    #[must_use = "the choice handle is the new document node"]
    pub fn choice(
        &mut self,
        left: DocId,
        right: DocId,
    ) -> Result<DocId, BuildError>
    {
        let left = self.checked_edge(left)?;
        let right = self.checked_edge(right)?;
        self.insert_node(DocNode::Choice { left, right })
    }

    /// Stores a flatten node over an existing document.
    ///
    /// # Contract
    /// - requires: `doc` belongs to this builder.
    /// - ensures: the flatten request is retained until finalization.
    /// - provides: a node whose finalized image softens layout-owned lines.
    /// - fails: rejects foreign handles, allocation failure, or a build
    ///   ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a soft line changes image identity while hard lines
    ///   and verbatim nodes retain their identity.
    /// - witness: `algebra::flatten_turns_a_line_into_one_space`.
    #[inline]
    #[must_use = "the flatten handle is the new document node"]
    pub fn flatten(
        &mut self,
        doc: DocId,
    ) -> Result<DocId, BuildError>
    {
        let doc = self.checked_edge(doc)?;
        self.insert_node(DocNode::Flatten { doc })
    }

    /// Stores `choice(doc, flatten(doc))` in that order.
    ///
    /// # Contract
    /// - requires: `doc` belongs to this builder.
    /// - ensures: the unflattened branch is the left alternative and the
    ///   flattened branch is the right alternative.
    /// - provides: the standard source-preserving grouping operation.
    /// - fails: propagates the typed failures of flattening and choice.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded` as applicable.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — branch order is observable independently from later
    ///   cost selection.
    /// - witness: `algebra::group_is_choice_of_the_unflattened_form_then_the_flattened_form`.
    #[inline]
    #[must_use = "the grouped handle is the new document node"]
    pub fn group(
        &mut self,
        doc: DocId,
    ) -> Result<DocId, BuildError>
    {
        let flattened = self.flatten(doc)?;
        self.choice(doc, flattened)
    }

    /// Seals the document and computes all flattened images iteratively.
    ///
    /// # Contract
    /// - requires: every builder edge already names an earlier node.
    /// - ensures: the returned arena is immutable, deterministic, and has one
    ///   flattened-image entry for every stored node.
    /// - provides: the complete slice-one document input for later phases.
    /// - fails: returns a typed limit, allocation, arithmetic, or identity
    ///   error without returning a partial arena.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns a build error when finalization cannot complete its bounded
    /// iterative pass.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — repeated finalization, structural reuse, and a limit
    ///   boundary distinguish idempotence, linear growth, and no partial
    ///   output.
    /// - witness: `algebra::flattening_is_idempotent`.
    #[inline]
    #[must_use = "the sealed arena is the document's immutable result"]
    pub fn finish(mut self) -> Result<DocArena, BuildError>
    {
        let original_count = self.nodes.len();
        if self.flattened.try_reserve(original_count).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::FlattenMemo,
            });
        }
        if self.flatten_memo.try_reserve(original_count).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::FlattenMemo,
            });
        }
        let mut index = 0usize;
        while index < original_count {
            self.meter.charge_step()?;
            let Some(node) = self.nodes.get(index).copied()
            else {
                return Err(BuildError::NodeIdExhausted);
            };
            let current = match u32::try_from(index) {
                | Ok(index) => NodeId::from(index),
                | Err(_) => return Err(BuildError::NodeIdExhausted),
            };
            let candidate = match node {
                | DocNode::Empty => DocNode::Empty,
                | DocNode::Text(text) => DocNode::Text(text),
                | DocNode::Verbatim(verbatim) => DocNode::Verbatim(verbatim),
                | DocNode::Line => {
                    let text = self.space_text_id()?;
                    DocNode::Text(text)
                },
                | DocNode::HardLine => DocNode::HardLine,
                | DocNode::Concat { left, right } => DocNode::Concat {
                    left: self.flattened_edge(left)?,
                    right: self.flattened_edge(right)?,
                },
                | DocNode::Nest { amount, doc } => DocNode::Nest {
                    amount,
                    doc: self.flattened_edge(doc)?,
                },
                | DocNode::Align { doc } => DocNode::Align {
                    doc: self.flattened_edge(doc)?,
                },
                | DocNode::Choice { left, right } => DocNode::Choice {
                    left: self.flattened_edge(left)?,
                    right: self.flattened_edge(right)?,
                },
                | DocNode::Flatten { doc } => {
                    let doc = self.flattened_edge(doc)?;
                    let Ok(index) = usize::try_from(u32::from(doc))
                    else {
                        return Err(BuildError::NodeIdExhausted);
                    };
                    let Some(image) = self.nodes.get(index).copied()
                    else {
                        return Err(BuildError::UnknownDoc);
                    };
                    image
                },
            };
            let image = if candidate == node {
                current
            }
            else {
                match self.find_flattened(candidate)? {
                    | Some(image) => image,
                    | None => self.insert_flattened(candidate)?,
                }
            };
            if self.flattened.try_reserve(1usize).is_err() {
                return Err(BuildError::AllocationFailed {
                    site: BuildAllocationSite::FlattenMemo,
                });
            }
            self.flattened.push(image);
            index = index.saturating_add(1usize);
        }
        while self.flattened.len() < self.nodes.len() {
            let index = self.flattened.len();
            let Ok(index) = u32::try_from(index)
            else {
                return Err(BuildError::NodeIdExhausted);
            };
            let node = NodeId::from(index);
            if self.flattened.try_reserve(1usize).is_err() {
                return Err(BuildError::AllocationFailed {
                    site: BuildAllocationSite::FlattenMemo,
                });
            }
            self.flattened.push(node);
        }
        Ok(DocArena::from_parts(
            self.arena,
            self.nodes,
            self.texts,
            self.verbatim,
            self.flattened,
        ))
    }

    /// Returns a client handle for an internal node identity.
    #[inline]
    fn handle(
        &self,
        node: NodeId,
    ) -> DocId
    {
        DocId::from_parts(self.arena, node)
    }

    /// Validates a client handle before any store lookup.
    ///
    /// # Contract
    /// - requires: `doc` may be foreign or out of range.
    /// - ensures: success returns only a node identity present in this builder.
    /// - provides: the identity guard shared by every edge constructor.
    /// - fails: returns `UnknownDoc` before any meter charge or lookup.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc` for a foreign or out-of-range handle.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — foreign and out-of-range handles are distinguished
    ///   from valid handles before mutation.
    /// - witness: `algebra::a_handle_from_another_arena_is_refused_before_lookup`.
    #[inline]
    fn validate_doc(
        &self,
        doc: DocId,
    ) -> Result<NodeId, BuildError>
    {
        if self.arena != doc.arena_key() {
            return Err(BuildError::UnknownDoc);
        }
        let node = doc.node_id();
        let Ok(index) = usize::try_from(u32::from(node))
        else {
            return Err(BuildError::UnknownDoc);
        };
        if self.nodes.get(index).is_none() {
            return Err(BuildError::UnknownDoc);
        }
        Ok(node)
    }

    /// Validates one edge and charges its checked traversal step.
    ///
    /// # Contract
    /// - requires: `doc` is a candidate edge handle.
    /// - ensures: the edge is validated before its step is charged.
    /// - provides: the common constructor edge path.
    /// - fails: returns `UnknownDoc` or a step-limit error.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `ArithmeticOverflow`, or `LimitExceeded`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — invalid handles do not consume a build step.
    /// - witness: `algebra::a_handle_from_another_arena_is_refused_before_lookup`.
    #[inline]
    fn checked_edge(
        &mut self,
        doc: DocId,
    ) -> Result<NodeId, BuildError>
    {
        let doc = self.validate_doc(doc)?;
        self.meter.check_step()?;
        self.meter.charge_step()?;
        Ok(doc)
    }

    /// Inserts one node after preflighting its node ceiling and capacity.
    ///
    /// # Contract
    /// - requires: every identity in `node` names an earlier stored node.
    /// - ensures: the node is appended exactly once and receives the next id.
    /// - provides: the common single-node insertion path.
    /// - fails: reports identity, allocation, arithmetic, or limit failure.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `NodeIdExhausted`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — one node insertion separates storage and node-limit
    ///   boundaries without exposing partial state.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    #[inline]
    fn insert_node(
        &mut self,
        node: DocNode,
    ) -> Result<DocId, BuildError>
    {
        let Ok(index) = u32::try_from(self.nodes.len())
        else {
            return Err(BuildError::NodeIdExhausted);
        };
        self.meter.check_doc_node()?;
        if self.nodes.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::NodeArena,
            });
        }
        self.meter.charge_doc_node()?;
        self.nodes.push(node);
        Ok(self.handle(NodeId::from(index)))
    }

    /// Stores one text identity and its document node atomically.
    ///
    /// # Contract
    /// - requires: `text` has already passed the newline-free validation.
    /// - ensures: bytes and node usage are charged before either store grows.
    /// - provides: the shared text insertion path.
    /// - fails: returns typed identity, allocation, arithmetic, or limit
    ///   errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the first typed failure found during preflight or insertion.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — byte and node boundaries reject without mutating the
    ///   corresponding usage counter.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    fn store_text(
        &mut self,
        text: CheckedText,
    ) -> Result<DocId, BuildError>
    {
        let Ok(text_index) = u32::try_from(self.texts.len())
        else {
            return Err(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::IdConversion,
            });
        };
        let amount = text.bytes_used()?;
        self.meter.check_text_bytes(amount)?;
        self.meter.check_doc_node()?;
        if self.texts.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::TextArena,
            });
        }
        if self.nodes.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::NodeArena,
            });
        }
        self.meter.charge_text_bytes(amount)?;
        self.meter.charge_doc_node()?;
        self.texts.push(text);
        self.nodes.push(DocNode::Text(TextId::from(text_index)));
        let Ok(node_index) = u32::try_from(self.nodes.len().saturating_sub(1usize))
        else {
            return Err(BuildError::NodeIdExhausted);
        };
        Ok(self.handle(NodeId::from(node_index)))
    }

    /// Stores one verbatim identity and its document node atomically.
    ///
    /// # Contract
    /// - requires: `lines` is the scan of `bytes` and has a final fragment.
    /// - ensures: bytes, fragments, and node usage are charged before stores
    ///   grow.
    /// - provides: the shared verbatim insertion path.
    /// - fails: returns typed identity, allocation, arithmetic, or limit
    ///   errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the first typed failure found during preflight or insertion.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — every physical fragment, including an empty final
    ///   fragment, is counted exactly once.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
    #[inline]
    fn store_verbatim(
        &mut self,
        text: VerbatimText,
    ) -> Result<DocId, BuildError>
    {
        let Ok(verbatim_index) = u32::try_from(self.verbatim.len())
        else {
            return Err(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::IdConversion,
            });
        };
        let bytes_used = text.bytes_used()?;
        let lines_used = text.lines_used()?;
        self.meter.check_text_bytes(bytes_used)?;
        self.meter.check_verbatim_lines(lines_used)?;
        self.meter.check_doc_node()?;
        if self.verbatim.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::VerbatimArena,
            });
        }
        if self.nodes.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::NodeArena,
            });
        }
        self.meter.charge_text_bytes(bytes_used)?;
        self.meter.charge_verbatim_lines(lines_used)?;
        self.meter.charge_doc_node()?;
        self.verbatim.push(text);
        self.nodes
            .push(DocNode::Verbatim(VerbatimId::from(verbatim_index)));
        let Ok(node_index) = u32::try_from(self.nodes.len().saturating_sub(1usize))
        else {
            return Err(BuildError::NodeIdExhausted);
        };
        Ok(self.handle(NodeId::from(node_index)))
    }
    /// # Contract
    /// - requires: `doc` is an earlier original node already visited by finish.
    /// - ensures: one edge charge precedes its flattened-image lookup.
    /// - provides: the child image used to build a parent image.
    /// - fails: reports edge-limit or identity errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `UnknownDoc`, `ArithmeticOverflow`, or `LimitExceeded`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — every flatten edge contributes exactly one build
    ///   step.
    /// - witness: `algebra::every_finalization_visit_edge_and_probe_charges_a_build_step`.
    #[inline]
    fn flattened_edge(
        &mut self,
        doc: NodeId,
    ) -> Result<NodeId, BuildError>
    {
        self.meter.check_step()?;
        let Ok(index) = usize::try_from(u32::from(doc))
        else {
            return Err(BuildError::UnknownDoc);
        };
        let Some(image) = self.flattened.get(index).copied()
        else {
            return Err(BuildError::UnknownDoc);
        };
        self.meter.charge_step()?;
        Ok(image)
    }

    /// Finds an existing structurally equal flattened image deterministically.
    ///
    /// # Contract
    /// - requires: `candidate` is a fully mapped flattened node.
    /// - ensures: memo entries are compared in insertion order only.
    /// - provides: deterministic image reuse without a hash-seed dependency.
    /// - fails: reports a build-step limit or arithmetic overflow.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` or `LimitExceeded` when a probe cannot be
    /// charged.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — reordering structural probes cannot change the chosen
    ///   image identity or final arena shape.
    /// - witness: `algebra::finalization_is_deterministic_across_runs`.
    #[inline]
    fn find_flattened(
        &mut self,
        candidate: DocNode,
    ) -> Result<Option<NodeId>, BuildError>
    {
        self.meter.check_step()?;
        let image = self.flatten_memo.get(&candidate).copied();
        self.meter.charge_step()?;
        Ok(image)
    }

    /// Inserts one distinct flattened image into the node and memo stores.
    ///
    /// # Contract
    /// - requires: `candidate` is not already in the deterministic interner.
    /// - ensures: the image is appended once and its identity is stable.
    /// - provides: a newly stored flattened node identity.
    /// - fails: reports identity, allocation, arithmetic, or node-limit errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `NodeIdExhausted`, `AllocationFailed`, `ArithmeticOverflow`, or
    /// `LimitExceeded`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — one distinct candidate creates at most one image and
    ///   consumes exactly one node charge.
    /// - witness: `algebra::finalization_appends_at_most_one_image_per_node`.
    #[inline]
    fn insert_flattened(
        &mut self,
        candidate: DocNode,
    ) -> Result<NodeId, BuildError>
    {
        let Ok(index) = u32::try_from(self.nodes.len())
        else {
            return Err(BuildError::NodeIdExhausted);
        };
        self.meter.check_doc_node()?;
        if self.flatten_memo.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::FlattenMemo,
            });
        }
        if self.nodes.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::NodeArena,
            });
        }
        self.meter.charge_doc_node()?;
        let node = NodeId::from(index);
        self.nodes.push(candidate);
        self.flatten_memo.insert(candidate, node);
        Ok(node)
    }

    /// Returns or creates the text identity used to flatten a soft line.
    ///
    /// # Contract
    /// - requires: finalization needs a flattened soft-line image.
    /// - ensures: one shared single-space text identity exists.
    /// - provides: the flattened representation of `Line`.
    /// - fails: reports allocation, arithmetic, or text-limit failure.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `AllocationFailed`, `ArithmeticOverflow`, or `LimitExceeded`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — repeated soft lines share one text identity and one
    ///   byte charge.
    /// - witness: `algebra::flatten_turns_a_line_into_one_space`.
    #[inline]
    fn space_text_id(&mut self) -> Result<TextId, BuildError>
    {
        if let Some(text) = self.space_text {
            return Ok(text);
        }
        let Ok(index) = u32::try_from(self.texts.len())
        else {
            return Err(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::IdConversion,
            });
        };
        let text = CheckedText::try_from(TextSource::from(" "))?;
        let amount = text.bytes_used()?;
        self.meter.check_text_bytes(amount)?;
        if self.texts.try_reserve(1usize).is_err() {
            return Err(BuildError::AllocationFailed {
                site: BuildAllocationSite::TextArena,
            });
        }
        self.meter.charge_text_bytes(amount)?;
        self.texts.push(text);
        let id = TextId::from(index);
        self.space_text = Some(id);
        Ok(id)
    }
}

/// Mints one non-zero process-local arena key.
///
/// # Contract
/// - requires: the process-local counter has not been exhausted.
/// - ensures: every successful call returns a distinct non-zero token.
/// - provides: the arena identity used to reject foreign handles.
/// - fails: returns `ArenaKeyExhausted` after the counter's final value.
/// - panics: none.
///
/// # Errors
/// Returns `ArenaKeyExhausted` when no non-zero token remains.
///
/// # Adequacy
/// - hypothesis: L3 — the exhausted sentinel is never returned as a usable
///   arena key and no successful call reuses a token.
/// - witness: `algebra::an_exhausted_arena_key_counter_is_reported_rather_than_reused`.
#[inline]
fn mint_arena_key() -> Result<ArenaKey, BuildError>
{
    let token = NEXT_ARENA_KEY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        if current == 0u32 {
            None
        }
        else if current == u32::MAX {
            Some(0u32)
        }
        else {
            current.checked_add(1u32)
        }
    });
    let Ok(token) = token
    else {
        return Err(BuildError::ArenaKeyExhausted);
    };
    let Some(token) = NonZeroU32::new(token)
    else {
        return Err(BuildError::ArenaKeyExhausted);
    };
    Ok(ArenaKey::from(token))
}
