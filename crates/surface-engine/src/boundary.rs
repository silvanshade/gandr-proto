//! Semantic boundary types for the incremental pipeline.
//!
//! These transparent wrappers keep source coordinates, recognizer labels, and
//! predicates distinct at function boundaries while compiling to their host
//! representations.

use alloc::string::String;
use core::ops::Deref;
use core::ops::Range;

/// Define a transparent copyable boundary wrapper with bidirectional `From`
/// conversions and passthrough `Debug`.
///
/// The generated struct keeps its payload public so pipeline boundaries can
/// construct and inspect values literally while the wrapper type keeps
/// coordinates, labels, and predicates distinct in signatures.
macro_rules! semantic_copy {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name(pub $inner);

        impl From<$inner> for $name {
            #[inline]
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl core::fmt::Debug for $name {
            #[inline]
            fn fmt(
                &self,
                formatter: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}
/// Define a transparent borrowed-text boundary wrapper with string access
/// conversions.
///
/// The generated struct keeps its `&str` payload public so call sites can
/// pass text literally while the wrapper type keeps the various pipeline
/// names and source fragments distinct in signatures.
macro_rules! semantic_borrowed_str {
    ($(#[$meta:meta])* $vis:vis struct $name:ident;) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name<'source>(pub &'source str);

        impl<'source> From<&'source str> for $name<'source> {
            #[inline]
            fn from(value: &'source str) -> Self {
                Self(value)
            }
        }

        impl<'source> From<&'source String> for $name<'source> {
            #[inline]
            fn from(value: &'source String) -> Self {
                Self(value.as_str())
            }
        }

        impl AsRef<str> for $name<'_> {
            #[inline]
            fn as_ref(&self) -> &str {
                self.0
            }
        }

        impl core::ops::Deref for $name<'_> {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.0
            }
        }
    };
}

/// Optional borrowed lowered definition name used by item alignment.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptionalDefinitionName<'source>(pub Option<&'source str>);

impl<'source> From<Option<&'source str>> for OptionalDefinitionName<'source>
{
    #[inline]
    fn from(value: Option<&'source str>) -> Self
    {
        Self(value)
    }
}

/// Implement the string access traits for a boundary wrapper: `AsRef<str>`,
/// `Deref<Target = str>`, `Display`, and symmetric `PartialEq` with `&str`.
macro_rules! semantic_str {
    ($name:ident) => {
        impl AsRef<str> for $name
        {
            #[inline]
            fn as_ref(&self) -> &str
            {
                self.0
            }
        }

        impl core::ops::Deref for $name
        {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target
            {
                self.0
            }
        }

        impl core::fmt::Display for $name
        {
            #[inline]
            fn fmt(
                &self,
                formatter: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result
            {
                formatter.write_str(self.0)
            }
        }

        impl PartialEq<&str> for $name
        {
            #[inline]
            fn eq(
                &self,
                other: &&str,
            ) -> bool
            {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for &str
        {
            #[inline]
            fn eq(
                &self,
                other: &$name,
            ) -> bool
            {
                *self == other.0
            }
        }
    };
}

/// Implement `Not<Output = bool>` for each named boolean-flag wrapper.
macro_rules! semantic_bool {
    ($($name:ident),+ $(,)?) => {
        $(
            impl core::ops::Not for $name {
                type Output = bool;

                #[inline]
                fn not(self) -> Self::Output {
                    !self.0
                }
            }
        )+
    };
}

semantic_copy!(
    /// Named syntax kind consumed by the lowerer dispatch.
    pub struct SyntaxKind(&'static str);
);
semantic_copy!(
    /// Raw grammar tile spelling used by the recognizer.
    pub struct TileSpelling(&'static str);
);
semantic_copy!(
    /// Named child field requested from a syntax node.
    pub struct SyntaxField(&'static str);
);
semantic_copy!(
    /// Host byte offset into source text.
    pub struct SourceOffset(usize);
);
semantic_copy!(
    /// Index into a node's significant-child sequence.
    pub struct SignificantIndex(usize);
);
semantic_copy!(
    /// Ordinal selecting one string run.
    pub struct StringRunIndex(usize);
);
semantic_copy!(
    /// Presence of an error in a syntax node or subtree.
    pub struct ErrorPresence(bool);
);
semantic_copy!(
    /// Presence of a parser-inserted missing syntax node.
    pub struct MissingPresence(bool);
);
semantic_copy!(
    /// Whether a recursive name is hidden by a lexical shadow.
    pub struct ShadowPresence(bool);
);
semantic_copy!(
    /// Whether a successfully typed definition enters the session scope.
    pub struct DefinitionBindingFlag(bool);
);
semantic_copy!(
    /// Whether an observation name is reserved.
    pub struct ReservedObservationFlag(bool);
);
semantic_copy!(
    /// Whether a recursive definition has a copattern body.
    pub struct CopatternBodyFlag(bool);
);
semantic_copy!(
    /// Whether syntax denotes a host escape.
    pub struct HostEscapeFlag(bool);
);
semantic_copy!(
    /// Whether a CST node is significant named syntax.
    pub struct NamedNodeFlag(bool);
);
semantic_copy!(
    /// Whether a requested tile occurs at the current structural level.
    pub struct TilePresence(bool);
);
semantic_copy!(
    /// Whether a CST leaf carries grout.
    pub struct GroutLeafFlag(bool);
);
semantic_copy!(
    /// Whether a CST subtree contains grout.
    pub struct GroutPresence(bool);
);
semantic_copy!(
    /// Whether a tile is a shell redirection operator.
    pub struct ShellRedirectionFlag(bool);
);
semantic_copy!(
    /// Whether a tile label denotes a named terminal.
    pub struct NamedTerminalFlag(bool);
);
semantic_copy!(
    /// Whether source text ends in a primitive numeric suffix.
    pub struct NumericSuffixFlag(bool);
);
semantic_copy!(
    /// Whether source text is a lower-case identifier word.
    pub struct LowerIdentifierFlag(bool);
);
semantic_copy!(
    /// Whether total-mode recovery is active.
    pub struct TotalMode(bool);
);
semantic_copy!(
    /// Whether a case arm set selects list elimination.
    pub struct ListCaseFlag(bool);
);
semantic_copy!(
    /// Whether adjacent shell fragments belong to one word.
    pub struct ShellWordContinuation(bool);
);

semantic_borrowed_str!(
    /// Borrowed source program accepted by the lowering pipeline.
    pub struct PipelineSource;
);
semantic_borrowed_str!(
    /// Borrowed surface attribute name.
    pub struct AttributeName;
);
semantic_borrowed_str!(
    /// Borrowed declared type name at a lowering boundary.
    pub struct TypeName;
);
semantic_borrowed_str!(
    /// Borrowed data constructor name at a lowering boundary.
    pub struct ConstructorName;
);
semantic_borrowed_str!(
    /// Borrowed codata observation name.
    pub struct ObservationName;
);
semantic_borrowed_str!(
    /// Borrowed circuit member, declaration, or binder name.
    pub struct CircuitName;
);
semantic_copy!(
    /// Whether a codata observation is declared in the active registry.
    pub struct ObservationPresence(bool);
);
semantic_borrowed_str!(
    /// Borrowed field or projection label.
    pub struct FieldLabel;
);
semantic_borrowed_str!(
    /// Borrowed host-operation name.
    pub struct HostOperation;
);
semantic_borrowed_str!(
    /// Borrowed foreign-function operation name.
    pub struct ForeignOperation;
);
semantic_borrowed_str!(
    /// Borrowed host-module name.
    pub struct HostModuleName;
);
semantic_borrowed_str!(
    /// Borrowed prelude-module name.
    pub struct PreludeModuleName;
);
semantic_borrowed_str!(
    /// Borrowed prelude-member name.
    pub struct PreludeMemberName;
);
semantic_borrowed_str!(
    /// Borrowed lowered definition name.
    pub struct DefinitionName;
);
semantic_borrowed_str!(
    /// Borrowed one segment of an outermost-scope path: a module name, a
    /// member name, a declaration's own name, or a value binder's.
    pub struct ScopeSegment;
);
semantic_borrowed_str!(
    /// Borrowed flat `module.member` name a prelude builtin is bound under.
    pub struct QualifiedName;
);
semantic_borrowed_str!(
    /// Borrowed syntax-node text.
    pub struct NodeText;
);

impl NodeText<'_>
{
    /// Copies this syntax-node text into owned storage.
    #[inline]
    #[must_use]
    pub fn to_owned(self) -> String
    {
        self.0.to_owned()
    }
}

/// Implement `From` conversions relabeling one borrowed-text wrapper into
/// each named target wrapper (same lifetime, same payload).
macro_rules! semantic_relabel {
    ($source:ident => $($target:ident),+ $(,)?) => {
        $(
            impl<'source> From<$source<'source>> for $target<'source>
            {
                #[inline]
                fn from(value: $source<'source>) -> Self
                {
                    Self(value.0)
                }
            }
        )+
    };
}

semantic_relabel!(
    NodeText => ConstructorName,
    ForeignOperation,
    HostModuleName,
    HostOperation,
    PreludeModuleName,
    PreludeMemberName,
);
semantic_borrowed_str!(
    /// Borrowed surface operator spelling.
    pub struct OperatorText;
);
semantic_borrowed_str!(
    /// Borrowed machine-frame role name in a diagnostic context chain.
    pub struct ContextRole;
);
semantic_borrowed_str!(
    /// Borrowed prose explaining why the cell layer declined a description
    /// member.
    pub struct DeclineReason;
);

semantic_copy!(
    /// Levenshtein edit distance between two attribute spellings.
    pub struct EditDistance(usize);
);
semantic_copy!(
    /// Number of source items in a lowered program or checkpoint set.
    pub struct ItemCount(usize);
);
semantic_copy!(
    /// Index of one source item in a lowered program.
    pub struct ItemIndex(usize);
);
semantic_copy!(
    /// Number of bindings in a base typing context.
    pub struct ContextLength(usize);
);
semantic_copy!(
    /// Position of one host module in the reserved host-module table.
    pub struct HostModuleIndex(usize);
);
semantic_copy!(
    /// Position of one member in its host module's member list.
    pub struct HostMemberIndex(usize);
);
semantic_copy!(
    /// Nesting depth of one module declaration below the outermost item.
    pub struct ModuleDepth(usize);
);
semantic_copy!(
    /// Length of one source byte range.
    pub struct SourceLength(usize);
);
semantic_copy!(
    /// Depth in a finite syntax or origin tree.
    pub struct TreeDepth(usize);
);
semantic_copy!(
    /// Row-major offset into the item-alignment table.
    pub struct AlignmentOffset(usize);
);
semantic_copy!(
    /// One compatibility component of an origin path.
    pub struct OriginPathComponent(u32);
);
semantic_copy!(
    /// Arity of a declared data constructor.
    pub struct ConstructorArity(usize);
);
semantic_copy!(
    /// Number of binders required by a surface lambda role.
    pub struct LambdaArity(usize);
);
semantic_copy!(
    /// Number of value results consumed by one rebuild frame.
    pub struct RebuiltValueCount(usize);
);
semantic_copy!(
    /// Fresh pipeline hole address projected to the core representation.
    pub struct FreshHoleId(u32);
);
semantic_copy!(
    /// Deterministic preorder ordinal of an origin node.
    pub struct OriginNodeOrdinal(u64);
);
semantic_copy!(
    /// Number of recorded entries in one origin map.
    pub struct OriginEntryCount(usize);
);
semantic_copy!(
    /// Whether two pipeline structures satisfy a named matching predicate.
    pub struct MatchDecision(bool);
);
semantic_copy!(
    /// Whether a type mentions the distinguished data universe.
    pub struct DataMention(bool);
);
semantic_copy!(
    /// Whether advisory marking was skipped at excessive recursive depth.
    pub struct RecursiveMarkDepthExceeded(bool);
);
semantic_copy!(
    /// Whether a term already carries a requested type ascription.
    pub struct AscriptionPresence(bool);
);
semantic_copy!(
    /// Whether an origin map contains no provenance entries.
    pub struct OriginMapEmpty(bool);
);
semantic_copy!(
    /// Which submission of a session a source item was written in.
    ///
    /// Submissions are numbered as a session receives them, so the index is
    /// stable for the life of the session and independent of how many core
    /// items any submission lowered to.
    pub struct SubmissionIndex(usize);
);
semantic_copy!(
    /// Stable identity of one **source** item within its submission — the
    /// programmer's item, counted as lowering reads the source file.
    ///
    /// Deliberately not a core item index. One source item may lower to any
    /// number of core items, and an obligation about a pattern the programmer
    /// wrote must survive that: the identity is assigned where the item is
    /// read, and nothing downstream renumbers it.
    pub struct SourceItemId(usize);
);
semantic_copy!(
    /// Source-order index of one match expression within its source item.
    pub struct MatchIndex(usize);
);
semantic_copy!(
    /// Source-order index of one branch within its match expression.
    pub struct BranchIndex(usize);
);
semantic_copy!(
    /// Number of leading branches that already entail a later branch.
    pub struct EntailingPrefix(usize);
);
semantic_copy!(
    /// Identity of a pattern-position typed hole, derived from its source
    /// position rather than minted, so it never perturbs the expression holes
    /// around it.
    pub struct PatternHoleId(u32);
);
semantic_copy!(
    /// Whether a value is too indeterminate for a pattern to decide against.
    pub struct ScrutineeIndeterminacy(bool);
);
semantic_copy!(
    /// Whether every step that produced a coverage row was a pattern the
    /// analysis read — the only case that can establish coverage.
    pub struct RowReadability(bool);
);
semantic_copy!(
    /// Whether filling a pattern hole would settle what a coverage row leaves
    /// open. False once an unreadable region stands on the row's path, because
    /// no filling reaches that.
    pub struct RowHoleDischargeable(bool);
);
semantic_copy!(
    /// Number of pending sub-results one assembly step consumes from a working
    /// stack.
    pub struct PendingResultCount(usize);
);

semantic_str!(SyntaxKind);
semantic_str!(TileSpelling);
semantic_str!(SyntaxField);
semantic_bool!(
    ErrorPresence,
    MissingPresence,
    ReservedObservationFlag,
    CopatternBodyFlag,
    HostEscapeFlag,
    NamedNodeFlag,
    ObservationPresence,
    TilePresence,
    GroutLeafFlag,
    GroutPresence,
    ShellRedirectionFlag,
    AscriptionPresence,
    NamedTerminalFlag,
    NumericSuffixFlag,
    LowerIdentifierFlag,
    TotalMode,
    ListCaseFlag,
    ShellWordContinuation,
    MatchDecision,
    DataMention,
    RecursiveMarkDepthExceeded,
    OriginMapEmpty,
);

/// Half-open host byte range into source text.
#[repr(transparent)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct SourceRange(pub Range<usize>);

impl core::fmt::Debug for SourceRange
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.0.fmt(f)
    }
}

impl From<Range<usize>> for SourceRange
{
    #[inline]
    fn from(value: Range<usize>) -> Self
    {
        Self(value)
    }
}

impl From<SourceRange> for Range<usize>
{
    #[inline]
    fn from(value: SourceRange) -> Self
    {
        value.0
    }
}
impl Deref for SourceRange
{
    type Target = Range<usize>;

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

/// Owned diagnostic text crossing a fallible test or decoder boundary.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticText(pub String);

impl From<String> for DiagnosticText
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

impl From<DiagnosticText> for String
{
    #[inline]
    fn from(value: DiagnosticText) -> Self
    {
        value.0
    }
}
