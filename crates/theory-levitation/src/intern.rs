//! **Content-addressed interning** of codes, keyed on decidable code equality
//! (proposal §3, §7; ADR-50's "third identity discipline").
//!
//! Interning is the concrete consumer that makes the decidable equality of
//! [`Code`] load-bearing: two structurally identical codes intern to the *same*
//! [`CodeId`], and distinct codes to distinct ids. This is exactly what the
//! matcher and the arena's content addressing rely on.

use std::collections::HashMap;

use crate::boundary::InternedCodeCount;
use crate::boundary::InternerEmptiness;
use crate::code::Code;

/// A stable **identity** for an interned [`Code`] — a dense index into the
/// interner's table.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CodeId(u32);

impl From<u32> for CodeId
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self(index)
    }
}

impl From<CodeId> for u32
{
    #[inline]
    fn from(id: CodeId) -> Self
    {
        id.0
    }
}

/// A **content-addressed store** of codes: interning a code returns its
/// [`CodeId`], allocating a fresh id only for a code not structurally equal to
/// any already interned.
///
/// # Adequacy
/// - hypothesis: L3 — interning two structurally identical codes yields one id
///   (dedup), interning a distinct code yields a new id, and every id resolves
///   back to its code.
/// - witness: `intern::tests::interning_is_by_decidable_code_equality`.
#[derive(Clone, Debug, Default)]
pub struct CodeInterner
{
    /// The interned codes, indexed by [`CodeId`].
    table: Vec<Code>,
    /// The reverse index from a code to its assigned id (the dedup key — this
    /// is where decidable code equality is load-bearing).
    index: HashMap<Code, CodeId>,
}

impl CodeInterner
{
    /// A fresh, empty interner.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Intern `code`, returning its [`CodeId`].
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the id already assigned to a structurally equal code,
    ///   or a fresh dense id (the current table length) otherwise; the returned
    ///   id resolves to `code` under [`Self::get`].
    /// - fails: never.
    #[inline]
    pub fn intern(
        &mut self,
        code: Code,
    ) -> CodeId
    {
        if let Some(existing) = self.index.get(&code) {
            return *existing;
        }
        let id = CodeId::from(u32::try_from(self.table.len()).unwrap_or(u32::MAX));
        self.table.push(code.clone());
        self.index.insert(code, id);
        id
    }

    /// The code an id resolves to, or [`None`] for an id this interner never
    /// minted.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: CodeId,
    ) -> Option<&Code>
    {
        self.table
            .get(usize::try_from(u32::from(id)).unwrap_or(usize::MAX))
    }

    /// The number of distinct codes interned.
    #[inline]
    #[must_use]
    pub fn len(&self) -> InternedCodeCount
    {
        self.table.len().into()
    }

    /// Whether no code has been interned.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> InternerEmptiness
    {
        self.table.is_empty().into()
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::grade::Grade;

    use super::*;
    use crate::code::Attrs;
    use crate::code::ValueTypeRef;

    #[test]
    fn interning_is_by_decidable_code_equality()
    {
        let mut interner = CodeInterner::new();
        let field = || Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty());
        // Two structurally identical codes intern to one id (dedup).
        let first = interner.intern(field());
        let second = interner.intern(field());
        assert_eq!(first, second, "identical codes share an id");
        assert_eq!(
            InternedCodeCount::from(1),
            interner.len(),
            "no fresh id was minted for the duplicate"
        );

        // A distinct code gets a fresh id.
        let other = interner.intern(Code::var("Nat"));
        assert_ne!(first, other, "a distinct code gets a distinct id");
        assert_eq!(
            InternedCodeCount::from(2),
            interner.len(),
            "the distinct code was minted"
        );

        // Every id resolves back to its code.
        assert_eq!(
            interner.get(first),
            Some(&field()),
            "id resolves to its code"
        );
        assert_eq!(
            Some(&Code::var("Nat")),
            interner.get(other),
            "id resolves to var"
        );
        assert_eq!(
            None,
            interner.get(CodeId::from(99)),
            "an unminted id resolves to nothing"
        );
    }
}
