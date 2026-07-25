//! Shared vocabulary types carried across the documentation lanes.

use alloc::string::String;
use core::fmt;
use core::str::FromStr;

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
    /// Every lifecycle status, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::Built,
        Self::Partial,
        Self::AdoptedUnbuilt,
        Self::DesignPass,
        Self::Dormant,
    ];
}

impl AsRef<str> for Status
{
    #[inline]
    fn as_ref(&self) -> &'static str
    {
        match *self {
            | Self::Built => "built",
            | Self::Partial => "partial",
            | Self::AdoptedUnbuilt => "adopted-unbuilt",
            | Self::DesignPass => "design-pass",
            | Self::Dormant => "dormant",
        }
    }
}

impl fmt::Display for Status
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.as_ref())
    }
}

impl FromStr for Status
{
    type Err = UnknownStatus;

    #[inline]
    fn from_str(text: &str) -> Result<Self, Self::Err>
    {
        match text {
            | "built" => Ok(Self::Built),
            | "partial" => Ok(Self::Partial),
            | "adopted-unbuilt" => Ok(Self::AdoptedUnbuilt),
            | "design-pass" => Ok(Self::DesignPass),
            | "dormant" => Ok(Self::Dormant),
            | _ => Err(UnknownStatus),
        }
    }
}

/// Rejection returned when a lifecycle-status spelling is not canonical.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct UnknownStatus;

/// A cite-key reference resolved against the references file.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CiteKey(String);

impl AsRef<str> for CiteKey
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl From<&str> for CiteKey
{
    #[inline]
    fn from(key: &str) -> Self
    {
        Self(key.to_owned())
    }
}

impl From<String> for CiteKey
{
    #[inline]
    fn from(key: String) -> Self
    {
        Self(key)
    }
}
