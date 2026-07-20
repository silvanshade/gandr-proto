//! Pre/post-order intervals over an [`crate::order::OrderMaintenance`].
//!
//! A tree node maps to the pair of order points `[lo, hi]` straddling its
//! subtree — `lo` minted in pre-order (on entry) and `hi` in post-order (on
//! exit). One node's subtree contains another's iff its interval contains the
//! other's, which the order makes an O(1) test
//! ([`crate::order::OrderMaintenance::interval_contains`]). This is the
//! containment query Porter's incremental engine uses to decide term
//! enclosure; this module is only the interval datum, the order structure does
//! the comparing.

use crate::order::Pos;

/// A closed interval of order points: the `[lo, hi]` straddling a subtree.
///
/// `lo` and `hi` are handles into the *same* [`crate::order::OrderMaintenance`]
/// with `lo` no later than `hi` in the order; containment is tested via
/// [`crate::order::OrderMaintenance::interval_contains`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct Interval
{
    /// The interval's lower (pre-order) endpoint.
    pub lo: Pos,
    /// The interval's upper (post-order) endpoint.
    pub hi: Pos,
}

impl Interval
{
    /// Builds the interval `[lo, hi]`.
    ///
    /// # Contract
    /// - requires: `lo` and `hi` are handles into the same structure with `lo`
    ///   no later than `hi` in the order (the caller establishes this when it
    ///   mints the endpoints in pre/post-order).
    /// - ensures: returns the interval carrying the two endpoints.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        lo: Pos,
        hi: Pos,
    ) -> Self
    {
        return Self { lo, hi };
    }
}
