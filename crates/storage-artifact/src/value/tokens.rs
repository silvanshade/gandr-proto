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
//!
//! # The token body layout, which is a format rather than an implementation detail
//!
//! A chunk body is a sequence of **token records**, each opening with a
//! one-byte kind. The token index a [`crate::value::ptr::ContentPtr`] carries
//! counts records, one per record, from zero at the start of the body.
//!
//! ```text
//! record := TOKEN_OPEN  0x01 || u8   tag
//!         | TOKEN_WORD  0x02 || u64be value
//!         | TOKEN_BYTES 0x03 || u64be length || length bytes
//!         | TOKEN_CHILD 0x04 || 32-byte digest || u32be token offset
//!         | TOKEN_CLOSE 0x05
//! ```
//!
//! Every integer is big-endian and every width is fixed. No kind byte outside
//! `0x01..=0x05` is admitted, and a reader meeting one refuses
//! [`ValueError::UnexpectedToken`] rather than skipping it — an unknown token
//! kind in a content-addressed body is a corrupted or foreign body, never a
//! forward-compatible extension, because the digest already committed to every
//! byte.
//!
//! **Why the kind byte is separate from the tag byte.** Folding them — letting
//! the export tag byte itself be the record kind — would save one byte per
//! constructor and make a word payload whose leading byte is a valid tag
//! indistinguishable from a constructor. That is precisely the wrong-kind
//! inhabitant this plane is written to refuse, and one byte per constructor is
//! not worth being unable to state the refusal.
//!
//! **`TOKEN_CLOSE` carries no tag.** The nesting is a balanced sequence and
//! the reader tracks its own depth, so repeating the tag at close would be a
//! second copy of a fact already in the body — and two copies of one fact are
//! two facts that can disagree.
//!
//! # The tag vocabulary is not this crate's
//!
//! `gandr_kernel_core::export::NODE_TAG_TABLE` fixes each export tag's child
//! arity, its own token contribution, and its boundary-versus-alias verdict.
//! This module carries the *transport* of those tags and takes no position on
//! their meaning; a value emitting tags outside that vocabulary is a caller
//! error the codec commitment records rather than one the framing detects.

use alloc::vec::Vec;

use crate::error::ValueError;
use crate::transport::CanonicalU64;
use crate::value::chunk::ChunkStore;
use crate::value::chunk::chunk_body;
use crate::value::ptr::ChunkDigest;
use crate::value::ptr::ContentPtr;
use crate::value::ptr::TokenOffset;
use crate::value::units::ChunkBody;
use crate::value::units::SeamDepth;
use crate::value::units::TokenBytes;

/// The byte length of one open record: the kind byte and the tag.
const TAG_RECORD_LEN: ByteLen = ByteLen(0x02_usize);
/// The byte length of one word record: the kind byte and a big-endian `u64`.
const WORD_RECORD_LEN: ByteLen = ByteLen(0x09_usize);
/// The byte length of a bytes record's header: the kind byte and the length.
const BYTES_HEADER_LEN: ByteLen = ByteLen(0x09_usize);
/// The byte length of one child record: the kind byte, a digest, an offset.
const CHILD_RECORD_LEN: ByteLen = ByteLen(0x25_usize);
/// The byte offset just past a child record's digest.
const DIGEST_END: usize = 0x21_usize;
/// The byte length of one close record: the kind byte alone.
const CLOSE_RECORD_LEN: ByteLen = ByteLen(0x01_usize);
/// The canonical integer width every framed count is checked against.
const CANONICAL_WIDTH_BITS: u32 = 0x40_u32;

/// A count of bytes inside one record.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ByteLen(usize);

/// A borrowed run of record bytes taken off the front of a chunk body.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RecordSlice<'bytes>(&'bytes [u8]);

/// A number of whole records to advance past.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RecordCount(u32);

/// The leading byte of a record, before it is known to name a kind.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct KindByte(u8);

/// The name a refusal uses for a record kind.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct KindLabel(&'static str);

/// One record kind, decoded from its leading byte.
///
/// An enum rather than a byte so the reader matches on the vocabulary rather
/// than on a number, and so an unassigned byte is a distinguishable absence
/// instead of a default arm.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RecordKind
{
    /// A constructor opens.
    Open,
    /// A canonical big-endian payload word.
    Word,
    /// A length-prefixed inline byte payload.
    Bytes,
    /// A reference to an already-committed child chunk.
    Child,
    /// The innermost open constructor closes.
    Close,
}

impl RecordKind
{
    /// Decodes a kind byte, refusing anything unassigned.
    #[inline]
    const fn decode(byte: KindByte) -> Option<Self>
    {
        return match byte.0 {
            | TOKEN_OPEN => Some(Self::Open),
            | TOKEN_WORD => Some(Self::Word),
            | TOKEN_BYTES => Some(Self::Bytes),
            | TOKEN_CHILD => Some(Self::Child),
            | TOKEN_CLOSE => Some(Self::Close),
            | _ => None,
        };
    }

    /// The name a refusal uses for this kind.
    #[inline]
    const fn label(self) -> KindLabel
    {
        return KindLabel(match self {
            | Self::Open => "an open",
            | Self::Word => "a word",
            | Self::Bytes => "a bytes",
            | Self::Child => "a child",
            | Self::Close => "a close",
        });
    }

    /// The fixed byte length of this kind's record, where it has one.
    #[inline]
    const fn fixed_len(self) -> Option<ByteLen>
    {
        return match self {
            | Self::Open => Some(TAG_RECORD_LEN),
            | Self::Word => Some(WORD_RECORD_LEN),
            | Self::Child => Some(CHILD_RECORD_LEN),
            | Self::Close => Some(CLOSE_RECORD_LEN),
            | Self::Bytes => None,
        };
    }
}

/// Reads a big-endian `u64` from exactly eight bytes.
#[inline]
fn read_u64(bytes: RecordSlice<'_>) -> Option<CanonicalU64>
{
    let image: [u8; 8] = bytes.0.try_into().ok()?;
    return Some(CanonicalU64::from(u64::from_be_bytes(image)));
}

/// Reads a big-endian `u32` from exactly four bytes.
#[inline]
fn read_u32(bytes: RecordSlice<'_>) -> Option<TokenOffset>
{
    let image: [u8; 4] = bytes.0.try_into().ok()?;
    return Some(TokenOffset::from(u32::from_be_bytes(image)));
}

/// Record kind: a constructor opens.
pub const TOKEN_OPEN: u8 = 0x01;
/// Record kind: a canonical big-endian 64-bit payload word.
pub const TOKEN_WORD: u8 = 0x02;
/// Record kind: a length-prefixed inline byte payload.
pub const TOKEN_BYTES: u8 = 0x03;
/// Record kind: a reference to an already-committed child chunk.
pub const TOKEN_CHILD: u8 = 0x04;
/// Record kind: the innermost open constructor closes.
pub const TOKEN_CLOSE: u8 = 0x05;

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

/// A cursor over a value's token stream that splices child chunks in place.
///
/// The reader is the reason [`CanonicalValue::decode_tokens`] never mentions a
/// store: crossing a chunk seam is the reader's business, and a decoder that
/// handled seams itself would be deciding storage policy from inside a value's
/// own codec.
///
/// # Why it carries the store
///
/// A [`ContentPtr`] child record is a *hole* in the token stream, and filling
/// it needs a fetch. Handing the decoder a bare byte slice would have made the
/// seam visible in the codec's signature — the decoder would have to return
/// "I reached a pointer" and be re-entered — which is exactly the leak this
/// type exists to prevent. So the reader holds the store, descends on a child
/// record, and pops when the child's stream is exhausted; a decoder sees one
/// continuous stream and cannot tell where the seams were.
///
/// The store is `&dyn` rather than a type parameter deliberately.
/// Monomorphising the reader over the store would push the store type into
/// [`CanonicalValue::decode_tokens`]'s signature and therefore into every
/// value's codec, making a value's encoding depend on where it happens to be
/// stored.
pub struct TokenReader<'stream>
{
    /// Where child chunks are fetched and re-verified.
    store: &'stream dyn ChunkStore,
    /// The remaining tokens of the chunk currently being read.
    remaining: ChunkBody<'stream>,
    /// The token index of `remaining`'s first token within its chunk.
    position: TokenOffset,
    /// Suspended positions of the chunks this reader descended out of,
    /// outermost first.
    ///
    /// A descent is not a recursion the call stack can hold: a child record can
    /// appear at any depth in any chunk, and the decoder driving the reader has
    /// its own recursion already. Keeping the seam stack here means the depth
    /// of the chunk DAG is bounded by the heap rather than by the host stack.
    suspended: Vec<(ChunkBody<'stream>, TokenOffset)>,
}

impl core::fmt::Debug for TokenReader<'_>
{
    /// Prints the reader's position without the store.
    ///
    /// A store is not a value and printing one would print a heap; what a
    /// reader's reader wants is where it is, which is the position and the
    /// seam depth.
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return f
            .debug_struct("TokenReader")
            .field("position", &self.position)
            .field("seam_depth", &self.suspended.len())
            .field("remaining_bytes", &self.remaining.as_ref().len())
            .finish();
    }
}

impl<'stream> TokenReader<'stream>
{
    /// Opens a reader over a chunk body at a token offset.
    ///
    /// # Contract
    /// - requires: `body` is the verified token body of a chunk `store` holds.
    /// - ensures: the reader starts at token index `position` with no suspended
    ///   chunks.
    /// - provides: the entry point [`super::cam_deref`] builds on.
    /// - fails: never; a malformed body is refused when read, not when opened.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        store: &'stream dyn ChunkStore,
        body: ChunkBody<'stream>,
        position: TokenOffset,
    ) -> Self
    {
        return Self {
            store,
            remaining: body,
            position,
            suspended: Vec::new(),
        };
    }

    /// Returns the token index the reader is positioned at within its chunk.
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

    /// Returns how many chunk seams the reader is currently inside.
    ///
    /// Exposed so a test can assert that a value which *should* have crossed a
    /// seam actually did. A deref that silently read everything from one chunk
    /// passes a round-trip test and proves nothing about chunking.
    #[inline]
    #[must_use]
    pub fn seam_depth(&self) -> SeamDepth
    {
        return SeamDepth::from(self.suspended.len());
    }

    /// Returns the store child chunks are fetched from.
    #[inline]
    #[must_use]
    pub const fn store(&self) -> &'stream dyn ChunkStore
    {
        return self.store;
    }

    /// Positions the reader at the next readable record.
    ///
    /// Two things can stand between the cursor and a record: an exhausted
    /// chunk, which is popped, and a child pointer, which is descended into.
    /// Both are seams, and both are invisible to a caller by design.
    #[inline]
    fn settle(&mut self) -> Result<(), ValueError>
    {
        loop {
            if <&[u8]>::from(self.remaining).is_empty() {
                let Some((body, position)) = self.suspended.pop()
                else {
                    return Ok(());
                };
                self.remaining = body;
                self.position = position;
                continue;
            }
            if self.peek_kind() != Some(RecordKind::Child) {
                return Ok(());
            }
            let pointer = self.take_child()?;
            let chunk = self.store.load(pointer.digest())?;
            let body = chunk_body(chunk)?;
            self.suspended.push((self.remaining, self.position));
            self.remaining = body;
            self.position = TokenOffset::from(0_u32);
            self.skip_records(RecordCount(u32::from(pointer.offset())))?;
        }
    }

    /// The kind byte of the record under the cursor, if any.
    #[inline]
    fn peek_kind(&self) -> Option<RecordKind>
    {
        return <&[u8]>::from(self.remaining)
            .first()
            .copied()
            .map(KindByte)
            .and_then(RecordKind::decode);
    }

    /// Consumes `len` bytes from the front of the current body.
    #[inline]
    fn take(
        &mut self,
        len: ByteLen,
    ) -> Result<RecordSlice<'stream>, ValueError>
    {
        let bytes = <&[u8]>::from(self.remaining);
        let Some(head) = bytes.get(.. len.0)
        else {
            return Err(ValueError::TruncatedChunk {
                position: u32::from(self.position),
            });
        };
        let Some(tail) = bytes.get(len.0 ..)
        else {
            return Err(ValueError::TruncatedChunk {
                position: u32::from(self.position),
            });
        };
        self.remaining = ChunkBody::from(tail);
        return Ok(RecordSlice(head));
    }

    /// Advances the record cursor by one.
    #[inline]
    fn step(&mut self)
    {
        self.position = TokenOffset::from(u32::from(self.position).saturating_add(1_u32));
    }

    /// Reads one child-pointer record, which the caller has already peeked.
    #[inline]
    fn take_child(&mut self) -> Result<ContentPtr, ValueError>
    {
        let record = self.take(CHILD_RECORD_LEN)?;
        let Some(digest_bytes) = record.0.get(1 .. DIGEST_END)
        else {
            return Err(ValueError::TruncatedChunk {
                position: u32::from(self.position),
            });
        };
        let Some(offset_bytes) = record.0.get(DIGEST_END .. CHILD_RECORD_LEN.0)
        else {
            return Err(ValueError::TruncatedChunk {
                position: u32::from(self.position),
            });
        };
        let digest = ChunkDigest::try_from(digest_bytes)?;
        let offset =
            read_u32(RecordSlice(offset_bytes)).ok_or_else(|| ValueError::TruncatedChunk {
                position: u32::from(self.position),
            })?;
        self.step();
        return Ok(ContentPtr::new(digest, offset));
    }

    /// Skips forward over `count` whole records in the current body.
    ///
    /// Used on descent, where a child pointer's offset names a position inside
    /// the chunk rather than its start.
    #[inline]
    fn skip_records(
        &mut self,
        count: RecordCount,
    ) -> Result<(), ValueError>
    {
        for _ in 0_u32 .. count.0 {
            let Some(kind) = self.peek_kind()
            else {
                return Err(ValueError::TruncatedChunk {
                    position: u32::from(self.position),
                });
            };
            let Some(len) = kind.fixed_len()
            else {
                let payload_len = self.take_payload_len()?;
                let _skipped = self.take(payload_len)?;
                self.step();
                continue;
            };
            let _skipped = self.take(len)?;
            self.step();
        }
        return Ok(());
    }

    /// Refuses when the record under the cursor is not of the wanted kind.
    #[inline]
    fn require(
        &mut self,
        kind: RecordKind,
    ) -> Result<(), ValueError>
    {
        self.settle()?;
        let Some(found) = self.peek_kind()
        else {
            return Err(ValueError::TruncatedChunk {
                position: u32::from(self.position),
            });
        };
        if found == kind {
            return Ok(());
        }
        return Err(ValueError::UnexpectedToken {
            expected: kind.label().0,
            found: found.label().0,
            position: u32::from(self.position),
        });
    }

    /// Reads a bytes record's declared payload length off the front.
    #[inline]
    fn take_payload_len(&mut self) -> Result<ByteLen, ValueError>
    {
        let header = self.take(BYTES_HEADER_LEN)?;
        let declared = header
            .0
            .get(1 .. BYTES_HEADER_LEN.0)
            .map(RecordSlice)
            .and_then(read_u64)
            .ok_or_else(|| ValueError::TruncatedChunk {
                position: u32::from(self.position),
            })?;
        let declared = u64::from(declared);
        let payload = usize::try_from(declared).map_err(|_width| ValueError::WidthOverflow {
            found: declared,
            width: CANONICAL_WIDTH_BITS,
        })?;
        return Ok(ByteLen(payload));
    }

    /// Advances the cursor to a token offset within the opening chunk.
    ///
    /// # Contract
    /// - requires: `offset` names a record position in the chunk the reader was
    ///   opened over.
    /// - ensures: `Ok` leaves the cursor at that record.
    /// - provides: the entry step for a pointer that addresses the interior of
    ///   a chunk rather than its root.
    /// - fails: [`ValueError::TruncatedChunk`] when the chunk holds fewer
    ///   records, or [`ValueError::UnexpectedToken`] on an unassigned kind.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    pub fn seek(
        &mut self,
        offset: TokenOffset,
    ) -> Result<(), ValueError>
    {
        return self.skip_records(RecordCount(u32::from(offset)));
    }

    /// Reads the next constructor tag, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one open record, descending
    ///   through any child records reached on the way and popping any chunks
    ///   whose streams are exhausted, so the caller never sees a seam.
    /// - provides: the decoder's primitive step.
    /// - fails: [`ValueError::UnexpectedToken`],
    ///   [`ValueError::TruncatedChunk`], or a store rejection while crossing a
    ///   seam.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    pub fn read_tag(&mut self) -> Result<ConstructorTag, ValueError>
    {
        self.require(RecordKind::Open)?;
        let record = self.take(TAG_RECORD_LEN)?;
        let tag = record
            .0
            .get(1)
            .copied()
            .ok_or_else(|| ValueError::TruncatedChunk {
                position: u32::from(self.position),
            })?;
        self.step();
        return Ok(ConstructorTag::from(tag));
    }

    /// Reads the next canonical word, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one word record, crossing seams as
    ///   [`TokenReader::read_tag`] does.
    /// - provides: the decoder's scalar step.
    /// - fails: [`ValueError::UnexpectedToken`],
    ///   [`ValueError::TruncatedChunk`], or a store rejection while crossing a
    ///   seam.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    pub fn read_word(&mut self) -> Result<CanonicalU64, ValueError>
    {
        self.require(RecordKind::Word)?;
        let record = self.take(WORD_RECORD_LEN)?;
        let value = record
            .0
            .get(1 .. WORD_RECORD_LEN.0)
            .map(RecordSlice)
            .and_then(read_u64)
            .ok_or_else(|| ValueError::TruncatedChunk {
                position: u32::from(self.position),
            })?;
        self.step();
        return Ok(value);
    }

    /// Reads an inline byte payload, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one bytes record and borrows its
    ///   payload out of the chunk body that carries it.
    /// - provides: the decoder's payload step.
    /// - fails: [`ValueError::UnexpectedToken`],
    ///   [`ValueError::TruncatedChunk`], or a store rejection while crossing a
    ///   seam.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    pub fn read_bytes(&mut self) -> Result<TokenBytes<'stream>, ValueError>
    {
        self.require(RecordKind::Bytes)?;
        let payload_len = self.take_payload_len()?;
        let payload = self.take(payload_len)?;
        self.step();
        return Ok(TokenBytes::from(payload.0));
    }

    /// Reads the closing record of the innermost open constructor.
    ///
    /// # Contract
    /// - requires: nothing; a wrong token kind is the case this refuses.
    /// - ensures: `Ok` advances past exactly one close record, crossing seams
    ///   as [`TokenReader::read_tag`] does.
    /// - provides: the decoder's nesting step.
    /// - fails: [`ValueError::UnexpectedToken`],
    ///   [`ValueError::TruncatedChunk`], or a store rejection while crossing a
    ///   seam.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    #[inline]
    pub fn read_close(&mut self) -> Result<(), ValueError>
    {
        self.require(RecordKind::Close)?;
        let _record = self.take(CLOSE_RECORD_LEN)?;
        self.step();
        return Ok(());
    }
}
