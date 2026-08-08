//! Semantic wrappers for `gandr-sequent` scalar boundaries.
//!
//! The command IL is deliberately nominal: an arena count, a frame mark, a
//! render depth, and a semantic decision are different values even when their
//! carriers are the same Rust primitive.  These wrappers keep those boundaries
//! explicit while remaining representation-transparent.

/// Generates a `#[repr(transparent)]` `Copy` newtype over a scalar carrier,
/// with both `From` conversions and a `Display` pass-through.
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

        impl core::fmt::Display for $name
        {
            #[inline]
            fn fmt(
                &self,
                f: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result
            {
                <$inner as core::fmt::Display>::fmt(&self.0, f)
            }
        }
    };
}

/// Generates a `#[repr(transparent)]` boolean-decision newtype, with both
/// `From` conversions.
macro_rules! bool_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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
    CellCount,
    usize,
    "Number of allocated L-machine heap cells."
);
copy_wrapper!(
    CorpusItemCount,
    usize,
    "Number of top-level corpus items observed by a sequent sweep."
);
copy_wrapper!(
    CorpusFileCount,
    usize,
    "Number of corpus files observed by a sequent sweep."
);
copy_wrapper!(CellIndex, usize, "Index of one L-machine heap cell.");
copy_wrapper!(
    FrameIndex,
    usize,
    "Index of a captured L-machine continuation frame."
);
copy_wrapper!(
    CheckDepth,
    u32,
    "Depth of the typed-IL well-formedness walk."
);
copy_wrapper!(
    ConsumerArity,
    usize,
    "Number of consumer children `c̄` an IL tag declares, and the number a node \
     headed by that tag carries. Distinct from a producer arity so the two \
     cannot be exchanged."
);
copy_wrapper!(FrameHeight, usize, "Current L-machine frame-region height.");
copy_wrapper!(FrameMark, usize, "Saved L-machine frame-region height.");
copy_wrapper!(
    FocusResultCount,
    usize,
    "Number of focused result-stack entries to drain."
);
copy_wrapper!(
    OriginCount,
    usize,
    "Number of commands produced from one focus origin."
);
copy_wrapper!(RenderDepth, u32, "Depth of the command-IL renderer.");
copy_wrapper!(SequentNodeCount, usize, "Number of command-IL arena nodes.");

bool_wrapper!(
    DifferentialAgreement,
    "Whether two evaluation outcomes agree (the L machine's against a recorded \
     snapshot, or the retired CEK oracle)."
);
bool_wrapper!(
    CommandHoleStatus,
    "Whether a command is a focused computation hole."
);
bool_wrapper!(
    CommandUnsupportedStatus,
    "Whether a command is an unsupported-former decline."
);
bool_wrapper!(
    OperationConstructorStatus,
    "Whether an L value is an operation constructor."
);
bool_wrapper!(
    RenderLimitReached,
    "Whether bounded rendering has reached its depth limit."
);
bool_wrapper!(ReturnerStatus, "Whether an L value is a returner.");
bool_wrapper!(
    UnsupportedFormerStatus,
    "Whether a core term contains an unsupported sequent former."
);
bool_wrapper!(
    UnfocusRequirement,
    "Whether a native needs residual un-focusing support."
);
bool_wrapper!(
    WellformedDecision,
    "Whether a command tree is typed-IL well-formed."
);

/// Borrowed token in the command-IL renderer or inspection labels.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderToken<'source>(&'source str);

impl<'source> From<&'source str> for RenderToken<'source>
{
    #[inline]
    fn from(value: &'source str) -> Self
    {
        Self(value)
    }
}

impl<'source> From<RenderToken<'source>> for &'source str
{
    #[inline]
    fn from(value: RenderToken<'source>) -> Self
    {
        value.0
    }
}

impl AsRef<str> for RenderToken<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

impl core::fmt::Display for RenderToken<'_>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0)
    }
}

impl CheckDepth
{
    /// The root of a well-formedness walk.
    pub const ROOT: Self = Self(0);
}

impl RenderDepth
{
    /// The root rendering depth.
    pub const ROOT: Self = Self(0);

    /// The next rendering depth below the current node.
    #[inline]
    #[must_use]
    pub fn below(self) -> Self
    {
        Self(self.0.wrapping_add(1))
    }

    /// Whether this depth has reached the renderer limit.
    #[inline]
    #[must_use]
    pub fn reached(
        self,
        limit: Self,
    ) -> RenderLimitReached
    {
        (self.0 >= limit.0).into()
    }
}
