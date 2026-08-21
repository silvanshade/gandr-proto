//! Nominal scalar types for the layout engine.
//!
//! Every quantity the engine carries has a name. A width is not an index, an
//! indentation is not a column, and a byte budget is not a node budget, so none
//! of them is spelled as a bare integer anywhere a caller can see it. The
//! wrappers are the API boundary rather than a convenience alias: each is
//! `#[repr(transparent)]`, each has a checked constructor or an exact external
//! conversion, and none exposes an inherent accessor that hands the primitive
//! back.
//!
//! The pretty-printing design authority writes these quantities as bare
//! primitives. That spelling is representation; the operation set, the
//! fallibility, the arena and identity model, and the algebra are what bind.
//! Naming them here preserves all of that and satisfies the workspace rule that
//! no crate-defined signature exposes a primitive.
//!
//! # Conversions slice one owns
//!
//! Each wrapper below states the exact conversions it must gain. The rule
//! throughout: a widening conversion is a `From`, a narrowing one is a
//! `TryFrom` whose error is the owning crate error, and neither is an inherent
//! method.
//!
//! ```text
//! impl From<u32> for NestAmount
//! impl From<u32> for ScalarWidth
//! impl TryFrom<usize> for ScalarWidth
//! impl From<u32> for MaxDocNodes
//! impl From<usize> for MaxTextBytes
//! impl From<u32> for MaxVerbatimLines
//! impl From<u64> for MaxBuildSteps
//! impl<'source> From<&'source str> for TextSource<'source>
//! impl From<String> for TextOwned
//! impl<'source> From<&'source str> for VerbatimSource<'source>
//! impl From<String> for VerbatimOwned
//! impl From<ScalarWidth> for LimitBound
//! impl From<NestAmount> for ScalarWidth
//! ```

/// A count of Unicode scalar values occupying one line of output.
///
/// Width in this engine is scalar count rather than display cell count. A
/// client that owns its tabs expands them before construction; a tab preserved
/// inside verbatim text counts as one scalar and is never rewritten. Moving to
/// display cells later is one change here rather than two estimators.
///
/// # Contract
/// - requires: the value is a scalar count already checked against overflow.
/// - ensures: ordering agrees with the ordering of the underlying counts.
/// - provides: the one width currency the measure, cost, and taint rules read.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ScalarWidth
{
    /// The scalar count.
    width: u32,
}

/// The additional indentation a `Nest` node applies to its child.
///
/// # Contract
/// - requires: the value is the amount written at the construction site.
/// - ensures: addition against a current indentation is checked by the caller.
/// - provides: the argument type of the builder's nesting constructor.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NestAmount
{
    /// The indentation increment.
    amount: u32,
}

/// Borrowed newline-free text destined for a `Text` node.
///
/// # Contract
/// - requires: nothing at the type level; the builder rejects a carriage
///   return, a line feed, or a tab when the node is constructed.
/// - ensures: the borrow outlives the construction call and nothing else.
/// - provides: the borrowed argument type of the builder's text constructor.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextSource<'source>
{
    /// The borrowed or owned text.
    text: &'source str,
}

/// Owned newline-free text destined for a `Text` node.
///
/// # Contract
/// - requires: nothing at the type level; the builder applies the same
///   rejection rule as for borrowed text.
/// - ensures: the string is moved into the text arena exactly once.
/// - provides: the owned argument type of the builder's text constructor.
/// - panics: none.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextOwned
{
    /// The borrowed or owned text.
    text: String,
}

/// Borrowed opaque multiline text destined for a `Verbatim` node.
///
/// Verbatim text is the carrier for content that must survive byte-identical —
/// a comment above all. It admits line feeds and carriage-return line feeds in
/// any mixture and rejects a bare carriage return.
///
/// # Contract
/// - requires: nothing at the type level; the builder scans and rejects a bare
///   carriage return when the node is constructed.
/// - ensures: the borrow outlives the construction call and nothing else.
/// - provides: the borrowed argument type of the builder's verbatim
///   constructor.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimSource<'source>
{
    /// The borrowed or owned text.
    text: &'source str,
}

/// Owned opaque multiline text destined for a `Verbatim` node.
///
/// # Contract
/// - requires: nothing at the type level; the builder applies the same scan and
///   the same rejection as for borrowed verbatim text.
/// - ensures: the string is moved into the verbatim arena exactly once.
/// - provides: the owned argument type of the builder's verbatim constructor.
/// - panics: none.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimOwned
{
    /// The borrowed or owned text.
    text: String,
}

/// The ceiling on stored document nodes, flatten images included.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses to store a node once the count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxDocNodes
{
    /// The node count.
    nodes: u32,
}

/// The ceiling on uniquely stored text and verbatim bytes.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses to store text once the byte count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxTextBytes
{
    /// The byte count.
    bytes: usize,
}

/// The ceiling on stored verbatim physical fragments.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses a verbatim node whose scan would cross it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxVerbatimLines
{
    /// The fragment count.
    lines: u32,
}

/// The ceiling on constructor and finalization steps.
///
/// A step is one checked input edge, one interner probe, one visit, or one
/// flatten edge. It is the budget that bounds work rather than storage.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: construction and finalization refuse once the count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxBuildSteps
{
    /// The step count.
    steps: u64,
}

/// Document nodes stored so far, flatten images included.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DocNodesUsed
{
    /// The node count.
    nodes: u64,
}

/// Uniquely stored text and verbatim bytes so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: a second edge to an existing identity adds nothing to it.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextBytesUsed
{
    /// The byte count.
    bytes: u64,
}

/// Stored verbatim physical fragments so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimLinesUsed
{
    /// The fragment count.
    lines: u64,
}

/// Constructor and finalization steps consumed so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BuildStepsUsed
{
    /// The step count.
    steps: u64,
}

/// The numeric ceiling reported beside an exceeded limit.
///
/// One widened currency keeps the error's shape independent of which limit was
/// crossed, so a caller reads the kind for meaning and this for the number.
///
/// # Contract
/// - requires: the value is the limit that was crossed, widened without loss.
/// - ensures: the widening is exact for every limit currency in the crate.
/// - provides: the numeric payload of a limit-exceeded error.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LimitBound
{
    /// The widened ceiling.
    bound: u64,
}
