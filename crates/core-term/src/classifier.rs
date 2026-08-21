//! The classifier vocabulary: the `(sort, level)` pair a type is formed at.
//!
//! gandr classifies with **one** pair everywhere — surface syntax, formation,
//! normalization, kernel export, and diagnostics all read the same
//! [`Classifier`]. There is no second kind language and no separate level
//! algebra: the sort is the term category a type classifies, and the level is
//! `gandr_kernel_strata::Level`, imported rather than copied so that the rule
//! deciding a level and the oracle ordering one are the same code.
//!
//! # The two families
//!
//! A type classifies either values or computations, and the universe families
//! follow that split: `Type[+, l]` collects the value types at level `l` and
//! `Type[-, l]` collects the computation types at the same `l`. The two share
//! the level algebra and nothing else — they are distinct types at every
//! level, which is the whole point of carrying the sort.
//!
//! A universe **object** is itself a value type whichever family it collects,
//! because its inhabitants are static type descriptions. `Type[-, l]` is a
//! value type whose inhabitants classify computations; it does not say that
//! computations are values.
//!
//! # Ground and abstract
//!
//! [`GroundSort`] is the closed two-element set a checked declaration is
//! finally read at. [`SortExpr`] is what a classifier stores, because a
//! declaration may abstract over its sort; an abstract sort is discharged by
//! ground specialization before anything crosses the certified kernel
//! boundary, so the kernel never sees a [`SortExpr::Param`].

use alloc::string::String;
use core::fmt;

use gandr_kernel_strata::Level;

use crate::boundary::SortLiteral;
use crate::boundary::SortParamName;

/// One of the two ground term categories a type classifies.
///
/// The set is closed and always will be: it is the call-by-push-value polarity
/// split, not an extensible tag space.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GroundSort
{
    /// The positive category. A type at this sort classifies values.
    Value,
    /// The negative category. A type at this sort classifies computations.
    Computation,
}

impl GroundSort
{
    /// Every ground sort, in the order the surface literals are ruled.
    ///
    /// This is the finite instantiation set a sort-polymorphic declaration is
    /// checked at, and the census a total judgement over sorts iterates.
    pub const ALL: [Self; 2] = [Self::Value, Self::Computation];

    /// The ruled surface literal for this sort: `+` or `-`.
    ///
    /// The spelling lives here once, so the parser, the canonical printer, and
    /// a diagnostic never disagree about it.
    ///
    /// # Contract
    /// - ensures: returns `+` for [`Self::Value`] and `-` for
    ///   [`Self::Computation`].
    /// - provides: the one spelling of a sort in surface and printed form.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn literal(self) -> SortLiteral<'static>
    {
        match self {
            | Self::Value => SortLiteral::VALUE,
            | Self::Computation => SortLiteral::COMPUTATION,
        }
    }
}

impl fmt::Display for GroundSort
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.literal().as_ref())
    }
}

/// A sort **parameter** bound by a declaration's prenex telescope.
///
/// A parameter is a declaration input, never a runtime one: it is erased
/// before evaluation and discharged by ground specialization before kernel
/// admission. Two parameters are the same exactly when their names are.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SortParam
{
    /// The parameter's declared name.
    name: String,
}

impl SortParam
{
    /// The sort parameter declared under `name`.
    ///
    /// # Contract
    /// - ensures: preserves `name` exactly; two parameters are equal iff their
    ///   names are.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<'source, N>(name: N) -> Self
    where
        N: Into<SortParamName<'source>>,
    {
        Self {
            name: name.into().as_ref().to_owned(),
        }
    }

    /// The parameter's declared name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> SortParamName<'_>
    {
        SortParamName::from(self.name.as_str())
    }
}

/// The sort a classifier records: ground, or abstract in a declaration
/// parameter.
///
/// This enum is deliberately **not** `non_exhaustive`. It is a closed two-case
/// algebra, and a judgement over it is meant to be provably total.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SortExpr
{
    /// A ground sort, readable directly.
    Ground(
        /// The ground sort.
        GroundSort,
    ),
    /// A sort abstracted by a declaration's prenex telescope.
    ///
    /// Nothing produces this before prenex sort polymorphism lands; formation
    /// answers an abstract sort with a named error until then, because a rule
    /// that guessed a ground reading would be silently wrong at one of the two
    /// instantiations.
    Param(
        /// The bound parameter.
        SortParam,
    ),
}

impl SortExpr
{
    /// The value sort, ground.
    #[inline]
    #[must_use]
    pub const fn value() -> Self
    {
        Self::Ground(GroundSort::Value)
    }

    /// The computation sort, ground.
    #[inline]
    #[must_use]
    pub const fn computation() -> Self
    {
        Self::Ground(GroundSort::Computation)
    }

    /// The ground sort this expression reads at, when it is ground.
    ///
    /// # Contract
    /// - ensures: `Some(sort)` exactly for [`Self::Ground`]; `None` for an
    ///   abstract sort, which has no ground reading until it is specialized.
    /// - provides: the ground discharge every kernel-facing consumer needs.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn ground(&self) -> Option<GroundSort>
    {
        match *self {
            | Self::Ground(sort) => Some(sort),
            | Self::Param(_) => None,
        }
    }
}

impl From<GroundSort> for SortExpr
{
    #[inline]
    fn from(sort: GroundSort) -> Self
    {
        Self::Ground(sort)
    }
}

/// What a type is formed at: a sort and a level.
///
/// The level is not cached in every type node — formation derives it — so a
/// `Classifier` is an *answer*, produced by the formation judgement, rather
/// than an annotation carried along beside a type. The one exception is the
/// universe former itself, which stores the family it collects because that
/// pair is the universe's identity rather than a derived property of it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Classifier
{
    /// The sort of the classified type.
    sort: SortExpr,
    /// The level of the classified type.
    level: Level,
}

impl Classifier
{
    /// The classifier at `sort` and `level`.
    ///
    /// # Contract
    /// - ensures: preserves both components exactly; two classifiers are equal
    ///   iff both components are, level equality being canonical-form identity.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<S>(
        sort: S,
        level: Level,
    ) -> Self
    where
        S: Into<SortExpr>,
    {
        Self {
            sort: sort.into(),
            level,
        }
    }

    /// The sort component.
    #[inline]
    #[must_use]
    pub const fn sort(&self) -> &SortExpr
    {
        &self.sort
    }

    /// The level component.
    #[inline]
    #[must_use]
    pub const fn level(&self) -> &Level
    {
        &self.level
    }

    /// The ground sort this classifier reads at, when its sort is ground.
    #[inline]
    #[must_use]
    pub const fn ground_sort(&self) -> Option<GroundSort>
    {
        self.sort.ground()
    }
}
