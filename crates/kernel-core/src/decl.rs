//! The closed declaration vocabulary ([`Declaration`]): `Def` (a typed
//! definition) and `Axiom` (a tracked typed hole), each carrying its own
//! prenex [`LevelSignature`] (kernel-boundary.md §3).
//!
//! Growth is deliberate and closed: at S1 the vocabulary is exactly these two.
//! Datatype declarations arrive later as levitated description codes (S2),
//! never as a raw-inductive kind (K4).
//!
//! **Declarations are value-polarity at the boundary** (a design decision for
//! the S1 slice): a `Def` pairs a declared *value* type with a *value* body,
//! and an `Axiom` a declared value type. A computation definition `f : A → C`
//! enters as a thunk — declared type `U (A → C)`, body `thunk (λ. …)` — and is
//! used through `force`. This keeps the declaration vocabulary single-polarity
//! (one `add_decl` shape, no polarity-mismatch error) and matches CBPV's
//! treatment of top-level bindings as thunkable values.

use alloc::vec::Vec;

use gandr_kernel_strata::LandmarkConstraint;

use crate::levels::LevelParamCount;
use crate::term::Value;
use crate::types::ValueType;

/// A declaration's prenex level interface: its parameter count and its
/// declared landmark constraints (ADR-78 — the declaration is generalized over
/// these and checked against nothing else).
#[derive(Clone, Debug)]
pub struct LevelSignature
{
    /// The prenex level-parameter count.
    params: LevelParamCount,
    /// The declared landmark constraints over those parameters.
    constraints: Vec<LandmarkConstraint>,
}

impl LevelSignature
{
    /// A signature with no level parameters and no constraints — the common
    /// monomorphic case.
    #[inline]
    #[must_use]
    pub fn monomorphic() -> Self
    {
        Self {
            params: LevelParamCount::from(0_u32),
            constraints: Vec::new(),
        }
    }

    /// A signature over `params` prenex level parameters with `constraints`.
    #[inline]
    #[must_use]
    pub fn new(
        params: LevelParamCount,
        constraints: Vec<LandmarkConstraint>,
    ) -> Self
    {
        Self {
            params,
            constraints,
        }
    }

    /// The prenex level-parameter count.
    #[inline]
    #[must_use]
    pub const fn params(&self) -> LevelParamCount
    {
        self.params
    }

    /// The declared landmark constraints, in declaration order.
    #[inline]
    #[must_use]
    pub fn constraints(&self) -> &[LandmarkConstraint]
    {
        &self.constraints
    }
}

/// The content of a declaration: a definition or an axiom.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's declaration vocabulary is closed by design (kernel-boundary.md §3)"
)]
pub enum DeclarationContent
{
    /// A typed definition: a declared value type and a fully-elaborated value
    /// body, which the checker verifies against the type.
    Def
    {
        /// The declared value type.
        declared: ValueType,
        /// The fully-elaborated value body.
        body: Value,
    },
    /// A tracked typed hole: a declared value type with no body. Everything
    /// resting on it is flagged by the choke-point audit.
    Axiom
    {
        /// The declared value type.
        declared: ValueType,
    },
}

impl DeclarationContent
{
    /// The declared value type, common to both a definition and an axiom.
    #[inline]
    #[must_use]
    pub const fn declared_type(&self) -> &ValueType
    {
        match *self {
            | Self::Def { ref declared, .. } | Self::Axiom { ref declared } => declared,
        }
    }
}

/// A declaration entering the kernel through the choke point: a level
/// signature and a definition or axiom.
#[derive(Clone, Debug)]
pub struct Declaration
{
    /// The prenex level interface.
    levels: LevelSignature,
    /// The definition or axiom content.
    content: DeclarationContent,
}

impl Declaration
{
    /// Build a definition declaration.
    #[inline]
    #[must_use]
    pub fn def(
        levels: LevelSignature,
        declared: ValueType,
        body: Value,
    ) -> Self
    {
        Self {
            levels,
            content: DeclarationContent::Def { declared, body },
        }
    }

    /// Build an axiom declaration.
    #[inline]
    #[must_use]
    pub fn axiom(
        levels: LevelSignature,
        declared: ValueType,
    ) -> Self
    {
        Self {
            levels,
            content: DeclarationContent::Axiom { declared },
        }
    }

    /// The prenex level interface.
    #[inline]
    #[must_use]
    pub const fn levels(&self) -> &LevelSignature
    {
        &self.levels
    }

    /// The definition or axiom content.
    #[inline]
    #[must_use]
    pub const fn content(&self) -> &DeclarationContent
    {
        &self.content
    }

    /// The declared value type.
    #[inline]
    #[must_use]
    pub const fn declared_type(&self) -> &ValueType
    {
        self.content.declared_type()
    }
}
