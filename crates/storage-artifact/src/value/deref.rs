//! `cam_deref` — fetch, verify, decode.
//!
//! The three words are the whole contract and their order is load-bearing.
//! **Fetch** asks the store for the chunk the pointer names. **Verify**
//! recomputes the digest and re-reads the frame, so a store that returned the
//! wrong bytes is caught here rather than inside a decoder. **Decode** runs
//! the value's own codec over the verified body, splicing child chunks through
//! the same three steps as it crosses each seam.
//!
//! # The store arrives erased, and that is forced rather than chosen
//!
//! [`cam_deref`] takes `&dyn ChunkStore` rather than a store type parameter,
//! because [`crate::value::tokens::TokenReader`] holds one and a generic
//! `Store: ChunkStore + ?Sized` cannot be coerced to a trait object — the
//! unsizing coercion needs a sized type, and dropping `?Sized` to get it would
//! be solving the wrong problem. Erasing here is the honest form of the reason
//! the reader states: a value's codec must not know where the value is stored,
//! and it cannot know if the store never reaches its signature.
//!
//! [`super::cam_commit`] stays generic over its store, and the asymmetry is
//! real rather than an oversight. Nothing about the value's encoding escapes
//! into the commit path — the traversal owns its own sink — so there is nothing
//! there to keep the store out of.
//!
//! # Verification is not validity, restated where it is easiest to forget
//!
//! A successful `cam_deref` proves the bytes are the bytes the pointer names.
//! It proves nothing whatever about whether the value they decode to is
//! well-typed, admissible, or safe to act on. Replay from the canonical inner
//! bytes remains the sole validity authority, and a caller that reads a
//! successful deref as a validity result has crossed the inner wall without
//! noticing.

use crate::error::ValueError;
use crate::value::chunk::ChunkStore;
use crate::value::chunk::chunk_body;
use crate::value::ptr::ContentPtr;
use crate::value::ptr::TokenOffset;
use crate::value::tokens::CanonicalValue;
use crate::value::tokens::TokenReader;

/// Fetches, verifies and decodes the value a pointer addresses.
///
/// # Contract
/// - requires: `pointer` was produced by [`super::cam_commit`] against a store
///   whose chunks `store` can still answer for, under the same committed
///   profile and index base.
/// - ensures: `Ok(value)` equal to the value originally committed — the round
///   trip is an equality, not a resemblance, and chunk seams are invisible in
///   the result.
/// - provides: the value plane's read path, and the portability claim a content
///   pointer makes.
/// - fails: [`ValueError::UnknownChunk`] when the store cannot answer,
///   [`ValueError::DigestMismatch`] when it answers wrongly,
///   [`ValueError::MalformedChunk`] on a bad frame, and the codec's own
///   rejections on a body that is not this value.
/// - panics: none.
///
/// # Errors
/// [`ValueError`].
#[inline]
pub fn cam_deref<Value>(
    store: &dyn ChunkStore,
    pointer: ContentPtr,
) -> Result<Value, ValueError>
where
    Value: CanonicalValue,
{
    let chunk = store.load(pointer.digest())?;
    let body = chunk_body(chunk)?;
    let mut reader = TokenReader::new(store, body, TokenOffset::from(0_u32));
    reader.seek(pointer.offset())?;
    return Value::decode_tokens(&mut reader);
}
