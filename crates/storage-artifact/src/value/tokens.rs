//! The canonical token stream: what a value emits, and what a chunk holds.
//!
//! The scanner is **event-driven rather than byte-driven**. A value walks its
//! own structure in preorder and announces constructor entry and exit; the
//! sink owns the rolling hash and the cut decision, and the chunker owns the
//! parameters those decisions are taken under. Nothing about a value's Rust
//! representation reaches the framing: only tags, canonical words, canonical
//! byte payloads, and already-committed child pointers do.
//!
//! # Why entry and exit rather than a flat token list
//!
//! A boundary event needs the rolling hash **over the subtree rooted at the
//! boundary**, so something has to know where a subtree ends. Making the value
//! announce exit puts that knowledge where it already exists — in the walk —
//! and keeps the sink able to maintain one residue per open subtree without
//! ever inspecting the value. It is also why a flat token vector is not the
//! interface: a vector would force the sink to re-derive the nesting it was
//! just told.
//!
//! # The tag vocabulary is not this crate's
//!
//! `gandr_kernel_core::export::NODE_TAG_TABLE` fixes each export tag's child
//! arity, its own token contribution, and its boundary-versus-alias verdict.
//! This module carries the *transport* of those tags and takes no position on
//! their meaning; a value emitting tags outside that vocabulary is a caller
//! error the codec commitment records rather than one the framing detects.

use crate::error::ValueError;
use crate::transport::CanonicalU64;
use crate::value::ptr::ContentPtr;
use crate::value::ptr::TokenOffset;
use crate::value::units::ChunkBody;
use crate::value::units::TokenBytes;

/// One constructor tag in a canonical token stream.
///
/// The byte is the export wire tag. This newtype exists so a tag cannot be
/// confused with a canonical word or with a raw payload byte at the framing
/// boundary, which is the only confusion the encoder cannot otherwise catch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConstructorTag(u8);

impl From<u8> for ConstructorTag
{
    #[inline]
    fn from(tag: u8) -> Self
    {
        return Self(tag);
    }
}

impl From<ConstructorTag> for u8
{
    #[inline]
    fn from(tag: ConstructorTag) -> Self
    {
        return tag.0;
    }
}

/// The receiver of one value's canonical token stream.
///
/// Implemented by the committing traversal ([`crate::value::commit`]) and by
/// whatever else needs to observe a value's canonical shape without
/// materializing it. A sink never sees the value; it sees the walk.
pub trait TokenSink
{
    /// Announces entry into a constructor, contributing its own token.
    ///
    /// # Contract
    /// - requires: every [`TokenSink::open`] is matched by a later
    ///   [`TokenSink::close`] at the same nesting depth.
    /// - ensures: the sink accounts one token for `tag` and pushes a fresh
    ///   subtree residue.
    /// - provides: the preorder skeleton the rolling hash is taken over.
    /// - fails: [`ValueError`] when the sink's own accounting overflows or its
    ///   backing store refuses a chunk committed at this position.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn open(
        &mut self,
        tag: ConstructorTag,
    ) -> Result<(), ValueError>;

    /// Contributes one canonical big-endian 64-bit payload word.
    ///
    /// # Contract
    /// - requires: a constructor is open.
    /// - ensures: the word joins the open subtree's residue and its chunk.
    /// - provides: the only integer width the framing admits.
    /// - fails: [`ValueError`], as [`TokenSink::open`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn word(
        &mut self,
        value: CanonicalU64,
    ) -> Result<(), ValueError>;

    /// Contributes an inline canonical byte payload, length-prefixed.
    ///
    /// # Contract
    /// - requires: a constructor is open; `bytes` is already canonical.
    /// - ensures: the payload joins the open subtree's residue and its chunk.
    /// - provides: the escape for payloads that are not word-shaped.
    /// - fails: [`ValueError`], as [`TokenSink::open`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn bytes(
        &mut self,
        bytes: TokenBytes<'_>,
    ) -> Result<(), ValueError>;

    /// Contributes an already-committed child by reference.
    ///
    /// This is the **chunk wrapper**: the token that replaces a subtree the
    /// traversal has already cut and stored. A value never emits one itself;
    /// the traversal splices it in when a cut fires, and a re-emitted value
    /// read back through [`TokenReader`] may carry one where the original had
    /// structure. That asymmetry is deliberate and is what structural sharing
    /// *is*.
    ///
    /// # Contract
    /// - requires: `pointer` addresses a chunk already present in the store.
    /// - ensures: the pointer joins the open subtree's residue as a fixed
    ///   canonical image rather than as the subtree it stands for.
    /// - provides: the sharing edge of the chunk DAG.
    /// - fails: [`ValueError`], as [`TokenSink::open`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn child_pointer(
        &mut self,
        pointer: ContentPtr,
    ) -> Result<(), ValueError>;

    /// Announces exit from the innermost open constructor.
    ///
    /// This is where a boundary event is raised for a boundary-classified tag
    /// and where the chunker's cut decision is taken.
    ///
    /// # Contract
    /// - requires: a constructor is open.
    /// - ensures: the subtree's residue closes into its parent, and a cut fires
    ///   when the committed profile says so.
    /// - provides: the boundary positions the whole locality result is about.
    /// - fails: [`ValueError`], as [`TokenSink::open`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn close(&mut self) -> Result<(), ValueError>;
}

/// A value that can be written to, and read back from, the canonical token
/// stream.
///
/// The two halves are one contract: `decode(emit(v)) == v` for every value the
/// codec admits. That equation is what makes a [`ContentPtr`] mean anything,
/// and it is the round trip the rung's exit gate measures rather than assumes.
pub trait CanonicalValue: Sized
{
    /// Walks this value in preorder, announcing it to `sink`.
    ///
    /// # Contract
    /// - requires: `sink` is fresh or positioned inside an open constructor.
    /// - ensures: every [`TokenSink::open`] this walk performs is closed before
    ///   it returns, so the sink is left at the depth it was entered at.
    /// - provides: the producer half of the codec.
    /// - fails: [`ValueError`] propagated from the sink.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn emit_tokens<Sink>(
        &self,
        sink: &mut Sink,
    ) -> Result<(), ValueError>
    where
        Sink: TokenSink + ?Sized;

    /// Reads one value from the canonical token stream.
    ///
    /// # Contract
    /// - requires: `reader` is positioned at this value's opening constructor.
    /// - ensures: `Ok` leaves `reader` positioned immediately after the value's
    ///   closing token; chunk wrappers are spliced transparently, so a value
    ///   read back is not told which chunks it crossed.
    /// - provides: the consumer half of the codec.
    /// - fails: [`ValueError`] on an unexpected tag, a truncated stream, or a
    ///   chunk the store cannot answer for.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn decode_tokens(reader: &mut TokenReader<'_>) -> Result<Self, ValueError>;
}

/// A cursor over one chunk's token stream that splices child chunks in place.
///
/// The reader is the reason [`CanonicalValue::decode_tokens`] never mentions a
/// store: crossing a chunk seam is the reader's business, and a decoder that
/// tried to handle seams itself would be deciding storage policy from inside a
/// value's own codec.
#[derive(Debug)]
pub struct TokenReader<'stream>
{
    /// The remaining tokens of the chunk currently being read.
    remaining: ChunkBody<'stream>,
    /// The token index of `remaining`'s first token within its chunk.
    position: TokenOffset,
}

impl<'stream> TokenReader<'stream>
{
    /// Opens a reader over a chunk body at a token offset.
    ///
    /// # Contract
    /// - requires: `body` is the verified token body of one chunk image.
    /// - ensures: the reader starts at token index `position`.
    /// - provides: the entry point [`super::cam_deref`] builds on.
    /// - fails: never; a malformed body is refused when read, not when opened.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        body: ChunkBody<'stream>,
        position: TokenOffset,
    ) -> Self
    {
        return Self {
            remaining: body,
            position,
        };
    }

    /// Returns the token index the reader is positioned at.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> TokenOffset
    {
        return self.position;
    }

    /// Returns the unread remainder of the current chunk body.
    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> ChunkBody<'stream>
    {
        return self.remaining;
    }

    /// Reads the next constructor tag, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one tag token.
    /// - provides: the decoder's primitive step.
    /// - fails: [`ValueError::UnexpectedToken`] or
    ///   [`ValueError::TruncatedChunk`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the token-body decode step is the implementor deliverable"
    )]
    pub fn read_tag(&mut self) -> Result<ConstructorTag, ValueError>
    {
        todo!("read one tag token from the chunk body, advancing self.position by one");
    }

    /// Reads the next canonical word, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one word token.
    /// - provides: the decoder's scalar step.
    /// - fails: [`ValueError::UnexpectedToken`] or
    ///   [`ValueError::TruncatedChunk`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the token-body decode step is the implementor deliverable"
    )]
    pub fn read_word(&mut self) -> Result<CanonicalU64, ValueError>
    {
        todo!("read one big-endian canonical word token from the chunk body");
    }

    /// Reads the closing token of the innermost open constructor.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one close token.
    /// - provides: the decoder's nesting step.
    /// - fails: [`ValueError::UnexpectedToken`] or
    ///   [`ValueError::TruncatedChunk`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the token-body decode step is the implementor deliverable"
    )]
    pub fn read_close(&mut self) -> Result<(), ValueError>
    {
        todo!("read one close token from the chunk body");
    }
}
