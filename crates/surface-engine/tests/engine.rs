//! Consolidated integration-test binary for `gandr-surface-engine`.

#![allow(
    clippy::missing_inline_in_public_items,
    reason = "integration-test helpers do not need cross-crate inlining hints"
)]

extern crate alloc;
extern crate proptest as proptest_crate;

use alloc::string::String;
use alloc::vec::Vec;

/// Borrowed prose, source text, or labels crossing test-helper boundaries.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TestText<'text>(pub &'text str);

impl<'text> From<&'text str> for TestText<'text>
{
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

impl<'text> From<&&'text str> for TestText<'text>
{
    fn from(value: &&'text str) -> Self
    {
        Self(value)
    }
}

impl<'text> From<&'text String> for TestText<'text>
{
    fn from(value: &'text String) -> Self
    {
        Self(value.as_str())
    }
}

/// Borrowed origin or term path crossing test-helper boundaries.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TestPath<'path>(pub &'path [u32]);

/// Owned origin or term path crossing test-helper boundaries.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TestOwnedPath(pub Vec<u32>);

impl core::ops::Deref for TestOwnedPath
{
    type Target = [u32];

    fn deref(&self) -> &Self::Target
    {
        self.0.as_slice()
    }
}

impl<'path> From<&'path TestOwnedPath> for TestPath<'path>
{
    fn from(value: &'path TestOwnedPath) -> Self
    {
        Self(value.0.as_slice())
    }
}

impl<'path> From<&'path [u32]> for TestPath<'path>
{
    fn from(value: &'path [u32]) -> Self
    {
        Self(value)
    }
}

impl<'path> From<&'path Vec<u32>> for TestPath<'path>
{
    fn from(value: &'path Vec<u32>) -> Self
    {
        Self(value.as_slice())
    }
}

impl<'path, const N: usize> From<&'path [u32; N]> for TestPath<'path>
{
    fn from(value: &'path [u32; N]) -> Self
    {
        Self(value)
    }
}

macro_rules! test_copy_boundary {
    ($name:ident, $inner:ty) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) struct $name(pub $inner);

        impl From<$inner> for $name
        {
            fn from(value: $inner) -> Self
            {
                Self(value)
            }
        }

        impl From<$name> for $inner
        {
            fn from(value: $name) -> Self
            {
                value.0
            }
        }
    };
}

test_copy_boundary!(TestCount, usize);
test_copy_boundary!(TestPathComponent, u32);
test_copy_boundary!(TestInteger, i64);
test_copy_boundary!(TestDecision, bool);

impl core::ops::Not for TestDecision
{
    type Output = bool;

    fn not(self) -> Self::Output
    {
        !self.0
    }
}

mod acceptance;
mod attributes;
mod desc_elab;
mod diag;
mod diag_attr;
mod diag_frames;
mod edit;
mod edit_extra;
mod goals_extra;
mod incremental;
mod origin_identity;
mod proptest;
mod session;
mod surface;
mod total;
mod types;
