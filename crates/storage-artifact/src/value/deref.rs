//! `cam_deref` — fetch, verify, decode.
//!
//! The three words are the whole contract and their order is load-bearing.
//! **Fetch** asks the store for the chunk the pointer names. **Verify**
//! recomputes the digest and re-reads the frame, so a store that returned the
//! wrong bytes is caught here rather than inside a decoder. **Decode** runs
//! the value's own codec over the verified body, splicing child chunks through
//! the same three steps as it crosses each seam.
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
use crate::value::ptr::ContentPtr;
use crate::value::tokens::CanonicalValue;

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
#[expect(
    clippy::todo,
    reason = "gandr-8tou.4 scaffold: the deref traversal is the implementor deliverable"
)]
pub fn cam_deref<Store, Value>(
    store: &Store,
    pointer: ContentPtr,
) -> Result<Value, ValueError>
where
    Store: ChunkStore + ?Sized,
    Value: CanonicalValue,
{
    todo!("fetch {pointer:?} from {store:p}, verify its frame, then decode through a TokenReader");
}
