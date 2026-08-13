//! Semantic wrappers for primitive values crossing crate-defined signatures.
//!
//! The project-local Dylint wall forbids anonymous Rust primitives at
//! crate-defined API boundaries. These wrappers name the role carried by each
//! scalar or borrowed primitive while keeping representation transparent.

use alloc::string::String;

/// Define a transparent copy wrapper with explicit conversions.
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
    };
}

/// Define a transparent borrowed string wrapper with explicit conversions.
macro_rules! str_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name<'source>(&'source str);

        impl<'source> From<&'source str> for $name<'source>
        {
            #[inline]
            fn from(value: &'source str) -> Self
            {
                Self(value)
            }
        }

        impl<'source> From<&'source String> for $name<'source>
        {
            #[inline]
            fn from(value: &'source String) -> Self
            {
                Self(value.as_str())
            }
        }

        impl<'source> From<$name<'source>> for &'source str
        {
            #[inline]
            fn from(value: $name<'source>) -> Self
            {
                value.0
            }
        }

        impl<'source> AsRef<str> for $name<'source>
        {
            #[inline]
            fn as_ref(&self) -> &str
            {
                self.0
            }
        }
    };
}

str_wrapper!(
    DefinitionName,
    "Borrowed lowered top-level definition name."
);
copy_wrapper!(
    MatchDecision,
    bool,
    "Whether two checkpoint structures satisfy a named matching predicate."
);
copy_wrapper!(
    HolePresence,
    bool,
    "Whether a lowered item carries an unresolved hole."
);
copy_wrapper!(
    AdoptionDecision,
    bool,
    "Whether one edited item adopted its base checkpoint."
);
copy_wrapper!(
    AdoptedItemCount,
    usize,
    "Number of item checkpoints adopted by an incremental resume."
);
