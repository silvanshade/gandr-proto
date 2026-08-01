//! The **extended alphabet** `Â = { ⟦a, a⟧, ⟦a⟧ | a ∈ 𝔸 }` of the
//! deallocation models, plus the free-use letter `a` (NDA §3, Lemma 5.7).
//!
//! The letters are the resource-lifecycle idiom made formal (NDA §1, §3):
//!
//! - `⟦a` — **open / allocate** the name `a` (the `malloc`);
//! - `a⟧` — **close / deallocate** the name `a` (the `free`);
//! - `⟦a⟧` — **allocate and immediately deallocate** `a` (the drop/cancellation
//!   case of the adjoint-logic reading, design doc §7.2);
//! - `a` — a **free use** of an already-live name, possible only while `a` is
//!   held in memory (NDA Lemma 5.7(1)).
//!
//! RNNA's allocation-only alphabet (`a` and the binding `|a`, design doc
//! §2.2) is the [`Letter::Free`] / [`Letter::Open`] fragment — the NDA
//! Proposition 5.11 translation (`a⟧ → a`, `⟦a → |a`) maps between the two
//! presentations.

use crate::Atom;
use crate::Sort;

/// A letter of the extended alphabet `Â` (NDA §3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Letter<S>
{
    /// `a` — a free use of an already-live name (NDA Lemma 5.7(1)).
    Free(Atom<S>),
    /// `⟦a` — allocate / open `a` (NDA §3).
    Open(Atom<S>),
    /// `a⟧` — deallocate / close `a` (NDA §3).
    Close(Atom<S>),
    /// `⟦a⟧` — allocate and immediately deallocate `a` (NDA §3).
    OpenClose(Atom<S>),
}

impl<S> Letter<S>
where
    S: Sort,
{
    /// The name the letter mentions.
    #[inline]
    #[must_use]
    pub fn atom(&self) -> Atom<S>
    {
        return match *self {
            | Self::Free(atom) | Self::Open(atom) | Self::Close(atom) | Self::OpenClose(atom) => {
                atom
            },
        };
    }
}
