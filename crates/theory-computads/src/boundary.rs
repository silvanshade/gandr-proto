//! Semantic wrappers for primitive values crossing the polygraph crate
//! boundary.
//!
//! The project-local Dylint wall forbids anonymous Rust primitives in
//! crate-defined signatures. These wrappers name the domain role for scalar and
//! borrowed primitive values while keeping representation transparent.

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
        #[derive(Clone, Copy, Debug)]
        pub struct $name<'source>(&'source str);

        impl<'source> From<&'source str> for $name<'source>
        {
            #[inline]
            fn from(value: &'source str) -> Self
            {
                Self(value)
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
    CellCount,
    usize,
    "Number of cells held by a polygraph cell store."
);
copy_wrapper!(
    CellLinearity,
    bool,
    "Whether a cell metavariable occurs linearly in the redex side."
);
copy_wrapper!(
    CellInvertibility,
    bool,
    "Whether a cell is an invertible joinability certificate."
);
copy_wrapper!(
    CellStoreEmptyStatus,
    bool,
    "Whether a polygraph cell store contains no cells."
);
copy_wrapper!(
    CompletionStepBudget,
    usize,
    "Maximum number of completion worklist steps."
);
copy_wrapper!(
    CompletionCellBudget,
    usize,
    "Maximum number of cells allowed during completion."
);
copy_wrapper!(
    CompletionStatus,
    bool,
    "Whether completion reached convergence without declining."
);
copy_wrapper!(
    FiringPermission,
    bool,
    "Whether a cell's provenance permits it to fire at a target term."
);
copy_wrapper!(
    DeclinedFaceIndex,
    usize,
    "Index of a surface 2-cell face declined during elaboration."
);
copy_wrapper!(
    DeclinedOpIndex,
    usize,
    "Index of a declared operation declined during elaboration."
);
copy_wrapper!(
    OperationInputCount,
    usize,
    "Number of input ports a declared operation reads."
);
copy_wrapper!(
    GroundPatternStatus,
    bool,
    "Whether a command pattern contains no metavariables."
);
copy_wrapper!(
    NormalizationBudget,
    usize,
    "Maximum number of rewrite steps for normalization."
);
copy_wrapper!(
    PatternSize,
    usize,
    "Node count of a command-pattern subtree."
);
copy_wrapper!(
    PositionRootStatus,
    bool,
    "Whether a command-pattern position is the root."
);
copy_wrapper!(
    PositionStep,
    usize,
    "One child-index step in a command-pattern position path."
);
copy_wrapper!(
    SubstitutionBindingCount,
    usize,
    "Number of producer and consumer metavariable bindings."
);
copy_wrapper!(
    SubstitutionEmptyStatus,
    bool,
    "Whether a substitution has no bindings."
);
copy_wrapper!(
    SubstitutionDecision,
    bool,
    "Boolean decision in substitution matching or unification."
);
copy_wrapper!(
    TraceletEquivalence,
    bool,
    "Whether two tracelets denote the same replayed certificate."
);
copy_wrapper!(
    TraceletReplay,
    bool,
    "Whether a tracelet successfully replays against a cell store."
);
copy_wrapper!(
    VarianceFlowRole,
    bool,
    "Whether a cell variance contributes a flow role."
);

str_wrapper!(
    PrimeNameRef,
    "Borrowed name receiving a deterministic freshening prime."
);
