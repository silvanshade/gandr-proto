//! Shared vocabulary types carried across the documentation lanes.

use alloc::string::String;

/// Lifecycle status carried by every component (and optionally by a section).
///
/// The five values are the single document class of decision `gandr-fcw.8`
/// (component-owned status); a missing status fails validation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Status
{
    /// Implemented and matches the specification.
    Built,
    /// Partially implemented.
    Partial,
    /// Decided and specified, not yet implemented.
    AdoptedUnbuilt,
    /// A design pass exists; not yet adopted.
    DesignPass,
    /// Retired or parked; retained for provenance.
    Dormant,
}

impl Status
{
    /// Parse a status from its canonical attribute spelling.
    #[inline]
    #[must_use]
    pub fn parse(text: &str) -> Option<Self>
    {
        match text {
            | "built" => Some(Self::Built),
            | "partial" => Some(Self::Partial),
            | "adopted-unbuilt" => Some(Self::AdoptedUnbuilt),
            | "design-pass" => Some(Self::DesignPass),
            | "dormant" => Some(Self::Dormant),
            | _ => None,
        }
    }

    /// Return the canonical attribute spelling of the status.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str
    {
        match self {
            | Self::Built => "built",
            | Self::Partial => "partial",
            | Self::AdoptedUnbuilt => "adopted-unbuilt",
            | Self::DesignPass => "design-pass",
            | Self::Dormant => "dormant",
        }
    }

    /// Every canonical spelling, in declaration order.
    #[inline]
    #[must_use]
    pub const fn spellings() -> &'static [&'static str]
    {
        &[
            "built",
            "partial",
            "adopted-unbuilt",
            "design-pass",
            "dormant",
        ]
    }
}

/// A cite-key reference resolved against the references file.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CiteKey
{
    /// Bibliography key, resolved against the hayagriva references file.
    pub key: String,
}

impl CiteKey
{
    /// Build a cite key from its stable corpus spelling.
    #[inline]
    #[must_use]
    pub fn new(key: &str) -> Self
    {
        Self {
            key: key.to_owned(),
        }
    }
}
