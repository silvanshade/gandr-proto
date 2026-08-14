//! Semantic wrappers for primitive values crossing crate-defined signatures.
//!
//! The project-local Dylint wall forbids anonymous Rust primitives at
//! crate-defined API boundaries. These wrappers name the role carried by each
//! scalar or borrowed primitive while keeping representation transparent.

use alloc::string::String;
use alloc::vec::Vec;

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

/// Define a transparent floating-point wrapper with explicit conversions.
macro_rules! float_wrapper {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
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

copy_wrapper!(
    AppliedArgCount,
    usize,
    "Number of arguments already supplied to a primitive."
);
copy_wrapper!(
    ArenaLength,
    usize,
    "Number of allocated nodes in an arena-like collection."
);
copy_wrapper!(
    BooleanAtom,
    bool,
    "Semantic boolean value encoded in gandr's sum representation."
);
copy_wrapper!(
    CoherenceDecision,
    bool,
    "Whether the conformance coherence relation accepts a subtype pair."
);
copy_wrapper!(
    ContinuationFrameIndex,
    usize,
    "Index of a continuation frame in an evaluator stack."
);
copy_wrapper!(ListIndex, usize, "Index into a manifest list value.");
copy_wrapper!(
    ListInsertionIndex,
    usize,
    "Insertion index into a manifest list."
);
copy_wrapper!(ConstructorTag, usize, "Zero-based sum or constructor tag.");
copy_wrapper!(
    ContextLength,
    usize,
    "Number of bindings or obligations in a typing context."
);
copy_wrapper!(
    NodeIndex,
    u32,
    "Zero-based index into a typed syntax arena."
);
copy_wrapper!(
    HoleId,
    u32,
    "Opaque identity of a value or computation hole."
);
copy_wrapper!(
    EffectContains,
    bool,
    "Whether an effect row contains the queried operation."
);
copy_wrapper!(
    EffectRowEmptyStatus,
    bool,
    "Whether an effect row contains no signatures."
);
copy_wrapper!(
    EffectSubsetDecision,
    bool,
    "Whether an effect row is a subset of another effect row."
);
copy_wrapper!(
    ArenaEmptyStatus,
    bool,
    "Whether an arena-like collection contains no nodes."
);
copy_wrapper!(
    ContextEmptyStatus,
    bool,
    "Whether a typing context contains no bindings or obligations."
);
copy_wrapper!(
    ErrorStatus,
    bool,
    "Whether a marked diagnostic structure contains an error."
);
copy_wrapper!(
    F32LiteralBits,
    u32,
    "IEEE-754 bit pattern of a concrete f32 literal."
);
copy_wrapper!(
    F64LiteralBits,
    u64,
    "IEEE-754 bit pattern of a concrete f64 literal."
);
float_wrapper!(
    F32Literal,
    f32,
    "A concrete f32 literal carried by gandr syntax."
);
float_wrapper!(
    F64Literal,
    f64,
    "A concrete f64 literal carried by gandr syntax."
);
copy_wrapper!(
    GenerationDepth,
    u32,
    "Finite depth fuel for generated syntax and type strategies."
);
copy_wrapper!(
    GradeBound,
    u64,
    "Finite natural-number bound for a usage grade."
);
copy_wrapper!(
    I32Literal,
    i32,
    "A concrete i32 literal carried by gandr syntax."
);
copy_wrapper!(
    I64Literal,
    i64,
    "A concrete i64 literal carried by gandr syntax."
);
copy_wrapper!(
    InferMode,
    bool,
    "Whether a generated term is produced for inference mode."
);
copy_wrapper!(
    InternerEmptyStatus,
    bool,
    "Whether a type interner contains no canonical entries."
);
copy_wrapper!(
    MarkingEmptyStatus,
    bool,
    "Whether a marking contains no recorded facts."
);
copy_wrapper!(
    InternedTypeCount,
    usize,
    "Number of canonical types stored in an interner."
);
copy_wrapper!(
    IntegerLiteral,
    i64,
    "A concrete integer literal carried by gandr syntax."
);
copy_wrapper!(
    MachineStepCount,
    u64,
    "Number of abstract-machine steps taken."
);
copy_wrapper!(PathIndex, u32, "One component of a marked tree path.");
copy_wrapper!(PrimitiveArity, usize, "Arity of a primitive operation.");
copy_wrapper!(
    PrimitivePredicate,
    bool,
    "Truth value returned by a primitive predicate."
);
copy_wrapper!(
    ShortCircuitFlag,
    bool,
    "Whether a primitive operation is a short-circuiting control form."
);
copy_wrapper!(
    GradeOrderDecision,
    bool,
    "Whether one usage grade is below another usage grade."
);
copy_wrapper!(
    SourceIndex,
    u32,
    "Stable source-location index in a compact arena."
);
copy_wrapper!(StackDepth, usize, "Current abstract-machine stack depth.");
copy_wrapper!(
    Staticness,
    bool,
    "Whether a type is fully static rather than gradual."
);
copy_wrapper!(SubtypeDecision, bool, "Result of a subtype relation check.");
copy_wrapper!(
    TerminalStatus,
    bool,
    "Whether an evaluator computation is terminal."
);
copy_wrapper!(TypeHash, u64, "Stable hash of an interned type.");
copy_wrapper!(TypeTableIndex, usize, "Index into an interned-type table.");
copy_wrapper!(
    TypeSerial,
    u64,
    "Serial component of an interned type name."
);
copy_wrapper!(
    U32Literal,
    u32,
    "A concrete u32 literal carried by gandr syntax."
);
copy_wrapper!(
    U64Literal,
    u64,
    "A concrete u64 literal carried by gandr syntax."
);
copy_wrapper!(
    ValueEquality,
    bool,
    "Result of a structural value equality check."
);

impl PrimitivePredicate
{
    /// Return the logical negation in the same primitive-predicate domain.
    #[inline]
    #[must_use]
    pub fn negated(self) -> Self
    {
        Self(!self.0)
    }
}

impl ShortCircuitFlag
{
    /// Return the boolean identity of a quantifier with this short-circuit
    /// flag.
    #[inline]
    #[must_use]
    pub fn identity_value(self) -> PrimitivePredicate
    {
        PrimitivePredicate(!self.0)
    }
}

str_wrapper!(DataTypeName, "Borrowed declared datatype name.");
str_wrapper!(TypeAtomName, "Borrowed core type atom name.");
str_wrapper!(
    SealDeclarationName,
    "Borrowed name of the declaration whose opaque ascription minted a sealed atom."
);
str_wrapper!(
    SealComponentName,
    "Borrowed label of the abstract type component a sealed atom stands for."
);

str_wrapper!(BinderName, "Borrowed source or generated binder name.");
str_wrapper!(ContinuationName, "Borrowed continuation binder name.");
str_wrapper!(EffectSignatureName, "Borrowed effect-signature name.");
str_wrapper!(StringLiteral, "Borrowed source string literal.");
str_wrapper!(FieldName, "Borrowed record field or projection label.");
str_wrapper!(NameRef, "Borrowed source-level identifier name.");
str_wrapper!(
    NumericLiteralName,
    "Borrowed strategy name for a numeric literal family."
);
str_wrapper!(
    OperationName,
    "Borrowed effect or primitive operation name."
);
str_wrapper!(PreludeName, "Borrowed prelude binding name.");
str_wrapper!(
    RegexLiteralText,
    "Borrowed text interpreted as a regex literal."
);
str_wrapper!(ResumeName, "Borrowed deep-handler resume binder name.");
str_wrapper!(
    StringText,
    "Borrowed string literal or string-processing text."
);

/// Borrowed list of integer literals used by combinator tests.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct I64Slice<'source>(&'source [i64]);

impl<'source> From<&'source [i64]> for I64Slice<'source>
{
    #[inline]
    fn from(value: &'source [i64]) -> Self
    {
        Self(value)
    }
}

impl<'source, const N: usize> From<&'source [i64; N]> for I64Slice<'source>
{
    #[inline]
    fn from(value: &'source [i64; N]) -> Self
    {
        Self(value.as_slice())
    }
}

impl<'source> From<&'source Vec<i64>> for I64Slice<'source>
{
    #[inline]
    fn from(value: &'source Vec<i64>) -> Self
    {
        Self(value.as_slice())
    }
}

impl AsRef<[i64]> for I64Slice<'_>
{
    #[inline]
    fn as_ref(&self) -> &[i64]
    {
        self.0
    }
}

/// Borrowed path components in a marked tree.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PathSegments<'source>(&'source [u32]);

impl<'source> From<&'source [u32]> for PathSegments<'source>
{
    #[inline]
    fn from(value: &'source [u32]) -> Self
    {
        Self(value)
    }
}

impl<'source, const N: usize> From<&'source [u32; N]> for PathSegments<'source>
{
    #[inline]
    fn from(value: &'source [u32; N]) -> Self
    {
        Self(value.as_slice())
    }
}

impl<'source> From<&'source Vec<u32>> for PathSegments<'source>
{
    #[inline]
    fn from(value: &'source Vec<u32>) -> Self
    {
        Self(value.as_slice())
    }
}

impl AsRef<[u32]> for PathSegments<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u32]
    {
        self.0
    }
}

/// Mutable prefix buffer used while enumerating marked paths.
#[repr(transparent)]
#[derive(Debug)]
pub struct PathPrefixMut<'source>(&'source mut Vec<u32>);

impl<'source> From<&'source mut Vec<u32>> for PathPrefixMut<'source>
{
    #[inline]
    fn from(value: &'source mut Vec<u32>) -> Self
    {
        Self(value)
    }
}

impl AsMut<Vec<u32>> for PathPrefixMut<'_>
{
    #[inline]
    fn as_mut(&mut self) -> &mut Vec<u32>
    {
        self.0
    }
}

/// Mutable output buffer used while enumerating marked paths.
#[repr(transparent)]
#[derive(Debug)]
pub struct PathSetMut<'source>(&'source mut Vec<Vec<u32>>);

impl<'source> From<&'source mut Vec<Vec<u32>>> for PathSetMut<'source>
{
    #[inline]
    fn from(value: &'source mut Vec<Vec<u32>>) -> Self
    {
        Self(value)
    }
}

impl AsMut<Vec<Vec<u32>>> for PathSetMut<'_>
{
    #[inline]
    fn as_mut(&mut self) -> &mut Vec<Vec<u32>>
    {
        self.0
    }
}

/// Borrowed list of names shielded from quotation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct ShieldedNames<'source>(&'source [&'source str]);

impl<'source> From<&'source [&'source str]> for ShieldedNames<'source>
{
    #[inline]
    fn from(value: &'source [&'source str]) -> Self
    {
        Self(value)
    }
}

impl<'source, const N: usize> From<&'source [&'source str; N]> for ShieldedNames<'source>
{
    #[inline]
    fn from(value: &'source [&'source str; N]) -> Self
    {
        Self(value)
    }
}

impl<'source> AsRef<[&'source str]> for ShieldedNames<'source>
{
    #[inline]
    fn as_ref(&self) -> &[&'source str]
    {
        self.0
    }
}
