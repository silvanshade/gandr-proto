//! Semantic wrappers for primitive values at `gandr-theory-levitation` API
//! boundaries.
//!
//! The wrappers are representation-transparent, intentionally small, and carry
//! only explicit conversion traits. Primitive extraction stays visible at the
//! handful of indexing, serialization, and test-harness boundaries that truly
//! need the scalar value.

use core::fmt;

/// Define a transparent copyable wrapper over a primitive payload with
/// bidirectional `From` conversions and `Display` passthrough.
///
/// Construction and payload access stay explicit so API boundaries read as
/// domain values rather than bare scalars.
macro_rules! copy_wrapper {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($inner);

        impl From<$inner> for $name
        {
            #[inline]
            fn from(value: $inner) -> Self
            {
                Self(value)
            }
        }

        impl From<$name> for $inner
        {
            #[inline]
            fn from(value: $name) -> Self
            {
                value.0
            }
        }

        impl fmt::Display for $name
        {
            #[inline]
            fn fmt(
                &self,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result
            {
                self.0.fmt(f)
            }
        }
    };
}

/// Define a transparent copyable boolean-flag wrapper with bidirectional
/// `From` conversions.
macro_rules! bool_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(bool);

        impl From<bool> for $name
        {
            #[inline]
            fn from(value: bool) -> Self
            {
                Self(value)
            }
        }

        impl From<$name> for bool
        {
            #[inline]
            fn from(value: $name) -> Self
            {
                value.0
            }
        }
    };
}

copy_wrapper!(
    NominalSerial,
    u64,
    "Monotone serial assigned to a minted datatype description identity."
);
copy_wrapper!(
    SurfaceByteOffset,
    usize,
    "Byte offset into the source text for description provenance spans."
);
copy_wrapper!(
    MonomialCount,
    usize,
    "Number of monomials in a bridge arity's Pi-layer."
);
copy_wrapper!(
    InternedCodeCount,
    usize,
    "Number of distinct first-order description codes interned."
);
copy_wrapper!(
    DescriptorFactorCount,
    usize,
    "Number of signature factors in a VDC test object."
);
copy_wrapper!(
    DescriptorFactorIndex,
    usize,
    "Index of a signature factor in a VDC test object."
);
copy_wrapper!(
    GeneratorIndex,
    usize,
    "Index of a generating relation face in a VDC test relation."
);
copy_wrapper!(
    RewriteDepth,
    usize,
    "Maximum rewrite-path length explored by the VDC test harness."
);
copy_wrapper!(
    TermPositionIndex,
    usize,
    "Child index within a first-order free term position."
);
copy_wrapper!(
    PortArgumentCount,
    usize,
    "Number of arguments a rewrite-sorted port's instantiation involves."
);
copy_wrapper!(
    RoundTripSampleCount,
    usize,
    "Number of code-isomorphism replay samples checked in one direction."
);
copy_wrapper!(
    SymbolNameMatchCount,
    usize,
    "Number of VDC symbols matching a requested name."
);
copy_wrapper!(
    ReplayClassCount,
    usize,
    "Number of replay-inequivalent certificate representatives."
);
copy_wrapper!(
    NumeralCount,
    usize,
    "Number of `Succ` constructors in a unary VDC test numeral."
);

bool_wrapper!(
    AttributeEmptiness,
    "Whether a description attribute slot contains no markers."
);
bool_wrapper!(
    AttributePresence,
    "Whether a description attribute slot contains a named marker."
);
bool_wrapper!(
    InternerEmptiness,
    "Whether a code interner contains no interned codes."
);
bool_wrapper!(
    FirstOrderStatus,
    "Whether a description code lies in the first-order fragment."
);
bool_wrapper!(
    RecursiveStatus,
    "Whether a description or code mentions a recursive occurrence."
);
bool_wrapper!(
    CellVariableLinearity,
    "Whether a cell-pattern variable occurs exactly once in the left-hand side."
);
bool_wrapper!(
    GenericEquality,
    "Result of description-guided structural equality for generic values."
);
bool_wrapper!(
    ContextTotality,
    "Whether a typed cell context covers every declared pattern variable."
);
bool_wrapper!(
    MonomorphicStatus,
    "Whether a code-isomorphism boundary is parameter-free on both sides."
);
bool_wrapper!(
    RoundTripStatus,
    "Whether every replayed code-isomorphism round trip held."
);
bool_wrapper!(
    ReplayEquivalence,
    "Whether two certificates are replay-equivalent over a sample corpus."
);
bool_wrapper!(
    SymbolPresence,
    "Whether a named constructor or operation is present in a description."
);
bool_wrapper!(
    PatternMatch,
    "Whether first-order pattern matching succeeded."
);
bool_wrapper!(
    LooseInstanceEquality,
    "Whether two loose instances are boundary-normalized equal."
);
bool_wrapper!(
    CellEquivalence,
    "Whether two VDC test-side cells are replay-equivalent over a corpus."
);

/// Canonical serialized bytes for a generic description value.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct SerializedValueBytes(Vec<u8>);

impl From<Vec<u8>> for SerializedValueBytes
{
    #[inline]
    fn from(value: Vec<u8>) -> Self
    {
        Self(value)
    }
}

impl From<SerializedValueBytes> for Vec<u8>
{
    #[inline]
    fn from(value: SerializedValueBytes) -> Self
    {
        value.0
    }
}

impl AsRef<[u8]> for SerializedValueBytes
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        &self.0
    }
}

/// Inspection notation for a whole description.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct SerializedDescText(String);

impl From<String> for SerializedDescText
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

impl From<SerializedDescText> for String
{
    #[inline]
    fn from(value: SerializedDescText) -> Self
    {
        value.0
    }
}

impl AsRef<str> for SerializedDescText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl fmt::Display for SerializedDescText
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}

/// Human-readable diagnostic or validation message text.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct DiagnosticMessage(String);

impl From<String> for DiagnosticMessage
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

impl From<DiagnosticMessage> for String
{
    #[inline]
    fn from(value: DiagnosticMessage) -> Self
    {
        value.0
    }
}

impl AsRef<str> for DiagnosticMessage
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl fmt::Display for DiagnosticMessage
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}
