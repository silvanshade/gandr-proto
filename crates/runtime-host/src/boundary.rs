//! Semantic wrappers for primitive values crossing shell-runtime signatures.
//!
//! The project-local Dylint wall forbids anonymous Rust primitives at
//! crate-defined API boundaries. These wrappers name each shell-specific role
//! while preserving the primitive representation.

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

/// Define a transparent borrowed-string wrapper with explicit conversions.
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

copy_wrapper!(
    ProcessExitCode,
    i64,
    "Exit status reported by a host process."
);
copy_wrapper!(
    FileSizeBytes,
    i64,
    "Filesystem object size measured in bytes."
);
copy_wrapper!(
    GlobMatch,
    bool,
    "Whether one filesystem path segment matches a glob segment."
);

str_wrapper!(
    CommandProgram,
    "Executable name in an `Exec::exec` payload."
);
str_wrapper!(
    CommandArgument,
    "One argument in an `Exec::exec` argument vector."
);
str_wrapper!(FileContents, "Text written through an `Fs::write` payload.");
str_wrapper!(FileKind, "Filesystem object kind returned by `Fs::stat`.");
str_wrapper!(
    FilePath,
    "Filesystem path crossing the shell host boundary."
);
str_wrapper!(
    GlobPattern,
    "Filesystem glob pattern accepted by `Fs::glob`."
);
str_wrapper!(
    GlobSegmentPattern,
    "Single-segment filesystem glob pattern."
);
str_wrapper!(
    PathSegmentText,
    "Filesystem path-segment text matched by a glob."
);
str_wrapper!(
    ShellErrorDetail,
    "Human-readable detail attached to a shell host error."
);
str_wrapper!(
    SpawnModeName,
    "Stdio disposition name in an `Exec::exec` payload."
);
str_wrapper!(
    StandardError,
    "Standard-error text returned by a host process."
);
str_wrapper!(
    StandardOutput,
    "Standard-output text returned by a host process."
);
