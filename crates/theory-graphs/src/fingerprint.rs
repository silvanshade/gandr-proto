//! Canonical adjacency fingerprints for dense graph sources.
//!
//! # Contract
//! - requires: `graph` satisfies [`EdgeSource`]'s dense node contract;
//!   duplicate and non-canonical successor order are valid inputs, but
//!   successful edges target nodes in `0..node_count()`.
//! - ensures: equal dense adjacency relations return equal fingerprints, and an
//!   out-of-bounds successor returns [`GraphValidationError::EdgeOutOfBounds`].
//! - provides: a fixed FNV-1a [`Fingerprint`] over the dense adjacency
//!   relation.
//! - fails: returns [`GraphValidationError`] for invalid successor targets or
//!   impossible adjacency row-length conversion overflow.
//! - panics: none.
//! - intension: the observable fingerprint is computed from a canonical byte
//!   stream containing node count, source nodes in ascending order, and
//!   deduplicated successors in ascending order; it is independent of
//!   `DefaultHasher`, compiler versions, processes, and Rust hasher seeds.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise — the fingerprint witness separates duplicate
//!   successors, non-canonical successor order, equal dense adjacency
//!   relations, and out-of-bounds targets by observing exact fingerprint
//!   equality and the exact `EdgeOutOfBounds` variant.
//! - witness: `gandr_theory_graphs::algorithms::contracts::fingerprint_contract`

use alloc::vec::Vec;
use core::convert::TryFrom as _;

use crate::EdgeSource;
use crate::Fingerprint;
use crate::FingerprintByte;
use crate::FingerprintBytes;
use crate::FingerprintWord16;
use crate::FingerprintWord32;
use crate::FingerprintWord64;
use crate::GraphValidationError;

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Return the fixed canonical adjacency fingerprint for `graph`.
///
/// # Contract
/// - requires: `graph` satisfies [`EdgeSource`]'s dense node contract;
///   duplicate and non-canonical successor order are valid inputs, but
///   successful edges target nodes in `0..node_count()`.
/// - ensures: returns the same fingerprint for equal dense adjacency relations
///   and returns [`GraphValidationError::EdgeOutOfBounds`] for an out-of-bounds
///   successor.
/// - provides: a fixed canonical adjacency fingerprint for `graph`.
/// - fails: returns [`GraphValidationError`] for invalid successor targets or
///   impossible adjacency row-length conversion overflow.
/// - panics: none.
/// - intension: the hash stream visits source nodes in ascending order, sorts
///   and deduplicates each successor row, writes all integers little-endian,
///   and avoids `DefaultHasher` so the result is stable across runs, platforms,
///   and Rust hasher seeds.
///
/// # Errors
/// Returns [`GraphValidationError::EdgeOutOfBounds`] when a successor target is
/// outside `0..node_count()`, or [`GraphValidationError::ArithmeticOverflow`]
/// when canonical row length cannot fit `u32`.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the fingerprint witness separates duplicate
///   successors, non-canonical successor order, equal dense adjacency
///   relations, and out-of-bounds targets by observing exact fingerprint
///   equality and the exact `EdgeOutOfBounds` variant.
/// - witness: `gandr_theory_graphs::algorithms::contracts::fingerprint_contract`
#[inline]
pub fn adjacency_fingerprint<G>(graph: &G) -> Result<Fingerprint, GraphValidationError>
where
    G: EdgeSource,
{
    let node_count = graph.node_count();
    let mut state = Fnv64::new();
    state.write_u32(FingerprintWord32::from(u32::from(node_count)));
    for source in node_count.ids() {
        state.write_u32(FingerprintWord32::from(u32::from(source)));
        let mut row = Vec::new();
        for target in graph.successors(source) {
            if u32::from(target) >= u32::from(node_count) {
                return Err(GraphValidationError::EdgeOutOfBounds {
                    source,
                    target,
                    node_count,
                });
            }
            row.push(target);
        }
        row.sort_unstable();
        row.dedup();
        let row_len = u32::try_from(row.len()).map_err(|conversion_error| {
            let _ = conversion_error;
            GraphValidationError::ArithmeticOverflow
        })?;
        state.write_u32(FingerprintWord32::from(row_len));
        for target in row {
            state.write_u32(FingerprintWord32::from(u32::from(target)));
        }
    }
    Ok(state.finish())
}

/// Minimal fixed FNV-1a accumulator.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Fnv64
{
    /// Current hash state.
    state: u64,
}

impl Default for Fnv64
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl Fnv64
{
    /// Construct a new accumulator.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        Self { state: FNV_OFFSET }
    }

    /// Absorb one byte.
    #[inline]
    pub fn write_byte<B>(
        &mut self,
        byte: B,
    ) where
        B: Into<FingerprintByte>,
    {
        self.state ^= u64::from(u8::from(byte.into()));
        self.state = self.state.wrapping_mul(FNV_PRIME);
    }

    /// Absorb a `u16` in little-endian order.
    #[inline]
    pub fn write_u16<W>(
        &mut self,
        value: W,
    ) where
        W: Into<FingerprintWord16>,
    {
        for byte in u16::from(value.into()).to_le_bytes() {
            self.write_byte(FingerprintByte::from(byte));
        }
    }

    /// Absorb a `u32` in little-endian order.
    #[inline]
    pub fn write_u32<W>(
        &mut self,
        value: W,
    ) where
        W: Into<FingerprintWord32>,
    {
        for byte in u32::from(value.into()).to_le_bytes() {
            self.write_byte(FingerprintByte::from(byte));
        }
    }

    /// Absorb a `u64` in little-endian order.
    #[inline]
    pub fn write_u64<W>(
        &mut self,
        value: W,
    ) where
        W: Into<FingerprintWord64>,
    {
        for byte in u64::from(value.into()).to_le_bytes() {
            self.write_byte(FingerprintByte::from(byte));
        }
    }

    /// Absorb a raw byte slice in order.
    #[inline]
    pub fn write_bytes<'bytes, B>(
        &mut self,
        bytes: B,
    ) where
        B: Into<FingerprintBytes<'bytes>>,
    {
        let bytes = bytes.into();
        for byte in bytes.as_ref().iter().copied() {
            self.write_byte(FingerprintByte::from(byte));
        }
    }

    /// Return the accumulated hash.
    #[inline]
    #[must_use]
    pub fn finish(self) -> Fingerprint
    {
        Fingerprint::from(self.state)
    }
}
