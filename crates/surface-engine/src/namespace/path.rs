//! Hierarchical names as paths of segments — the carrier's key space.
//!
//! A hierarchical name is an ordered list of [`Segment`]s; a [`NamePath`] is
//! that list, and it is the *only* thing the modifier layer ever manipulates.
//! A path is **reachability**, not identity: it is written by a user, it is
//! inert data, and nothing here mints, compares, or perturbs a machine-minted
//! identity (see the [`crate::namespace`] module documentation for the full
//! division of labour against the nominal machinery).
//!
//! # Ordering is load-bearing
//!
//! [`NamePath`]'s [`Ord`] is the derived lexicographic order on its segment
//! list, and the carrier depends on one consequence of it: **the extensions of
//! a path are order-convex**. If `a < b < c` and both `a` and `c` extend `p`,
//! then `b` extends `p` too — so in any ordered map keyed by [`NamePath`] a
//! subtree occupies one contiguous run, which is what makes a flat ordered map
//! a faithful trie (see [`crate::namespace::trie`]).

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use alloc::vec::Vec;

/// One segment of a hierarchical name — `nat`, `plus`, or `assoc` in
/// `nat.plus.assoc`.
///
/// A segment is opaque text. The carrier never interprets it, so a segment
/// containing the separator character is representable and simply does not
/// round-trip through [`DottedName`].
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Segment(String);

impl From<&str> for Segment
{
    #[inline]
    fn from(text: &str) -> Self
    {
        Self(text.to_owned())
    }
}

impl From<String> for Segment
{
    #[inline]
    fn from(text: String) -> Self
    {
        Self(text)
    }
}

impl AsRef<str> for Segment
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_str()
    }
}

impl core::fmt::Display for Segment
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0.as_str())
    }
}

/// The dotted rendering of a hierarchical name at a text boundary —
/// `"nat.plus"`, with the empty string naming the root.
///
/// The design record writes the root as a bare period; that spelling belongs
/// to a surface syntax this layer deliberately does not have, so the boundary
/// type takes the empty string and [`NamePath`]'s [`core::fmt::Display`]
/// renders the root as `.` for human-readable diagnostics.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DottedName<'source>(&'source str);

impl<'source> From<&'source str> for DottedName<'source>
{
    #[inline]
    fn from(text: &'source str) -> Self
    {
        Self(text)
    }
}

impl AsRef<str> for DottedName<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// The number of segments in a hierarchical name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentCount(pub usize);

/// A hierarchical name: the ordered segments that reach a binding.
///
/// The empty path is the **root**, and it prefixes every path — which is why
/// `renaming . p` (relocate the root) is the qualifying modifier and
/// `renaming p .` the unqualifying one.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NamePath(Vec<Segment>);

impl NamePath
{
    /// The root path — the empty segment list.
    #[inline]
    #[must_use]
    pub const fn root() -> Self
    {
        Self(Vec::new())
    }

    /// Builds a path from its segments, in order.
    #[inline]
    #[must_use]
    pub const fn from_segments(segments: Vec<Segment>) -> Self
    {
        Self(segments)
    }

    /// The path's segments, in order.
    #[inline]
    #[must_use]
    pub fn segments(&self) -> &[Segment]
    {
        self.0.as_slice()
    }

    /// The number of segments in this path.
    #[inline]
    #[must_use]
    pub fn depth(&self) -> SegmentCount
    {
        SegmentCount(self.0.len())
    }

    /// This path with `prefix` prepended.
    ///
    /// # Contract
    /// - ensures: the result's segments are `prefix`'s followed by this path's,
    ///   in that order; prepending the root is the identity.
    /// - provides: the path a binding acquires when its namespace is grafted
    ///   under `prefix`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — concatenation has one decision surface (operand
    ///   order), separated by the boundary pair `(root, p)` / `(p, root)` plus
    ///   one two-sided case asserted exactly.
    /// - witness: `path::tests::prefixing_by_root_is_the_identity`
    /// - witness: `path::tests::prefixing_prepends_in_order`
    #[inline]
    #[must_use]
    pub fn prefixed_by(
        &self,
        prefix: &Self,
    ) -> Self
    {
        let mut segments = Vec::with_capacity(prefix.0.len().saturating_add(self.0.len()));
        segments.extend_from_slice(prefix.0.as_slice());
        segments.extend_from_slice(self.0.as_slice());
        Self(segments)
    }

    /// This path with `suffix` appended.
    ///
    /// # Contract
    /// - ensures: equal to `suffix.prefixed_by(self)`.
    /// - provides: the accumulated prefix an `in p m` run reports its events
    ///   under.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the extension is the mirror of
    ///   [`Self::prefixed_by`] and shares its witnesses through one exact
    ///   agreement assertion.
    /// - witness: `path::tests::extending_mirrors_prefixing`
    #[inline]
    #[must_use]
    pub fn extended(
        &self,
        suffix: &Self,
    ) -> Self
    {
        suffix.prefixed_by(self)
    }

    /// The remainder of this path after `prefix`, when `prefix` is a prefix.
    ///
    /// # Contract
    /// - ensures: `Some(suffix)` exactly when `suffix.prefixed_by(prefix)`
    ///   equals this path; `None` otherwise.
    /// - ensures: stripping the root returns this path unchanged, so the root
    ///   prefixes everything.
    /// - provides: the subtree-relative key of a binding, and the prefix test
    ///   the carrier's subtree operations are defined by.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (the prefix test),
    ///   separated by the boundary inputs `root`, an exact match, a proper
    ///   extension, and a segment-wise near miss, each asserted as an exact
    ///   `Option` value.
    /// - witness: `path::tests::stripping_the_root_is_the_identity`
    /// - witness: `path::tests::stripping_an_exact_match_yields_the_root`
    /// - witness: `path::tests::stripping_a_proper_prefix_yields_the_remainder`
    /// - witness: `path::tests::a_near_miss_segment_is_not_a_prefix`
    #[inline]
    #[must_use]
    pub fn strip_prefix(
        &self,
        prefix: &Self,
    ) -> Option<Self>
    {
        let suffix = self.0.strip_prefix(prefix.0.as_slice())?;
        Some(Self(suffix.to_vec()))
    }
}

impl From<DottedName<'_>> for NamePath
{
    /// Splits a dotted rendering into segments; the empty string is the root.
    ///
    /// # Contract
    /// - ensures: the empty rendering is the root — the zero-segment path — and
    ///   not a one-segment path whose segment is empty.
    /// - ensures: any other rendering splits on the separator and on nothing
    ///   else, preserving order.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (the empty-rendering case),
    ///   separated by the boundary pair empty / non-empty with the resulting
    ///   path asserted as an exact value on each.
    /// - witness: `path::tests::the_empty_dotted_rendering_is_the_root`
    /// - witness: `path::tests::segments_round_trip_through_the_dotted_boundary`
    #[inline]
    fn from(text: DottedName<'_>) -> Self
    {
        if text.0.is_empty() {
            return Self::root();
        }
        Self(text.0.split('.').map(Segment::from).collect())
    }
}

impl From<Vec<Segment>> for NamePath
{
    #[inline]
    fn from(segments: Vec<Segment>) -> Self
    {
        Self(segments)
    }
}

impl core::fmt::Display for NamePath
{
    /// Renders the root as `.` and any other path as its dot-joined segments.
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        let Some((first, rest)) = self.0.split_first()
        else {
            return f.write_str(".");
        };
        f.write_str(first.as_ref())?;
        for segment in rest {
            f.write_str(".")?;
            f.write_str(segment.as_ref())?;
        }
        Ok(())
    }
}

/// Path-algebra unit tests.
#[cfg(test)]
mod tests
{
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::DottedName;
    use super::NamePath;
    use super::Segment;
    use super::SegmentCount;

    /// Builds a path from its dotted rendering.
    fn path<'text>(text: impl Into<DottedName<'text>>) -> NamePath
    {
        NamePath::from(text.into())
    }

    #[test]
    fn prefixing_by_root_is_the_identity()
    {
        let subject = path("nat.plus");
        assert_eq!(
            subject.prefixed_by(&NamePath::root()),
            subject,
            "the root prefixes every path without changing it"
        );
    }

    #[test]
    fn prefixing_prepends_in_order()
    {
        assert_eq!(
            path("plus.assoc").prefixed_by(&path("nat")),
            path("nat.plus.assoc"),
            "the prefix's segments precede the subject's"
        );
        assert_eq!(
            path("nat").prefixed_by(&path("plus.assoc")),
            path("plus.assoc.nat"),
            "the operands are not interchangeable"
        );
    }

    #[test]
    fn extending_mirrors_prefixing()
    {
        assert_eq!(
            path("nat").extended(&path("plus")),
            path("plus").prefixed_by(&path("nat")),
            "extension is prefixing with the operands exchanged"
        );
    }

    #[test]
    fn stripping_the_root_is_the_identity()
    {
        let subject = path("nat.plus");
        assert_eq!(
            subject.strip_prefix(&NamePath::root()),
            Some(subject),
            "the root prefixes everything, so stripping it changes nothing"
        );
    }

    #[test]
    fn stripping_an_exact_match_yields_the_root()
    {
        assert_eq!(
            path("nat.plus").strip_prefix(&path("nat.plus")),
            Some(NamePath::root()),
            "a path is its own prefix, with the root as remainder"
        );
    }

    #[test]
    fn stripping_a_proper_prefix_yields_the_remainder()
    {
        assert_eq!(
            path("nat.plus.assoc").strip_prefix(&path("nat")),
            Some(path("plus.assoc")),
            "the remainder is the subtree-relative key"
        );
    }

    #[test]
    fn a_near_miss_segment_is_not_a_prefix()
    {
        assert_eq!(
            path("nat.plus").strip_prefix(&path("na")),
            None,
            "prefixes are segment-wise, never character-wise"
        );
        assert_eq!(
            path("nat").strip_prefix(&path("nat.plus")),
            None,
            "a longer candidate cannot prefix a shorter path"
        );
    }

    #[test]
    fn extensions_of_a_path_are_order_convex()
    {
        let mut sorted = Vec::from([
            path("nat"),
            path("nat.plus"),
            path("nat.times.assoc"),
            path("natural"),
            path("zero"),
        ]);
        sorted.sort();
        let prefix = path("nat");
        let flags: Vec<bool> = sorted
            .iter()
            .map(|candidate| candidate.strip_prefix(&prefix).is_some())
            .collect();
        assert_eq!(
            flags,
            Vec::from([true, true, true, false, false]),
            "the extensions of `nat` occupy one contiguous run of the sorted keys"
        );
    }

    #[test]
    fn the_root_renders_as_a_bare_period()
    {
        assert_eq!(
            format!("{}", NamePath::root()),
            ".",
            "the design record's spelling of the root survives into diagnostics"
        );
    }

    #[test]
    fn a_path_renders_dot_joined()
    {
        assert_eq!(
            format!("{}", path("nat.plus.assoc")),
            "nat.plus.assoc",
            "segments are joined by the separator"
        );
    }

    #[test]
    fn depth_counts_segments()
    {
        assert_eq!(
            path("nat.plus").depth(),
            SegmentCount(2),
            "depth is the segment count, not the character count"
        );
        assert_eq!(
            NamePath::root().depth(),
            SegmentCount(0),
            "the root has no segments"
        );
    }

    #[test]
    fn segments_round_trip_through_the_dotted_boundary()
    {
        assert_eq!(
            path("nat.plus").segments(),
            Vec::from([Segment::from("nat"), Segment::from("plus")]).as_slice(),
            "the dotted boundary splits on the separator and nothing else"
        );
    }

    #[test]
    fn the_empty_dotted_rendering_is_the_root()
    {
        assert_eq!(
            path(""),
            NamePath::root(),
            "the boundary type spells the root as the empty string, so it must not split into one \
             segment whose text happens to be empty"
        );
        assert_eq!(
            path("").depth(),
            SegmentCount(0),
            "the root carries no segments, which is what makes it prefix every path"
        );
    }

    #[test]
    fn a_segment_renders_as_its_own_text()
    {
        assert_eq!(
            format!("{}", Segment::from("plus")),
            "plus",
            "a segment renders as the text it carries and contributes no separator of its own"
        );
        assert_eq!(
            format!("{}", Segment::from(String::from("assoc"))),
            "assoc",
            "and it carries that text whether it was built from borrowed or from owned text"
        );
    }

    #[test]
    fn a_segment_list_builds_the_path_in_order()
    {
        assert_eq!(
            NamePath::from(Vec::from([Segment::from("nat"), Segment::from("plus")])),
            path("nat.plus"),
            "building from segments preserves their order, so it agrees with the dotted boundary"
        );
    }
}
