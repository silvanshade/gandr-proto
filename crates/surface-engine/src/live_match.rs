//! Live pattern matching against a checkpoint scrutinee that may contain holes
//! (Yuan, Omar et al., *Live Pattern Matching with Typed Holes*, OOPSLA 2023).
//!
//! # What this module decides, and why it is three-valued
//!
//! A match in an unfinished program cannot always be told whether it fires. Its
//! scrutinee may be — or may contain — an unfilled hole, and its branches may
//! themselves be unfinished. Two-valued matching has no honest answer there:
//! calling such a branch refuted hides a match the finished program will take,
//! and calling it satisfied claims one it may not. So every verdict this module
//! produces has a third value, and the third value is the point:
//!
//! * a branch is [`BranchStatus::Satisfied`], [`BranchStatus::Possibly`], or
//!   [`BranchStatus::Refuted`] against the scrutinee;
//! * a constructor of the scrutinee's datatype is covered, possibly covered, or
//!   uncovered by the branch set ([`MatchObligation::Inexhaustive`] and
//!   [`MatchObligation::PossiblyInexhaustive`]);
//! * a branch is entailed, possibly entailed, or not entailed by the branches
//!   before it ([`MatchObligation::Redundant`] and
//!   [`MatchObligation::PossiblyRedundant`]).
//!
//! **Each verdict is proved in its own direction.** Coverage is proved with
//! every hole reading as matching *nothing* (a hole proves nothing), and
//! non-coverage is proved with every hole reading as matching *everything*
//! (still uncovered even at its most generous). Anything between the two is the
//! middle verdict. A consumer may therefore act on `Inexhaustive` and
//! `Redundant` as facts, which is what makes them worth reporting at all.
//!
//! # A hole and an unreadable region are not the same not-knowing
//!
//! Both stand where a pattern would, and both stop a proof. Everything else
//! about them differs, and the analysis keeps them apart everywhere:
//!
//! | | [`PatternShape::Hole`] | [`PatternShape::Unreadable`] |
//! | --- | --- | --- |
//! | what it is | indeterminacy the theory names: an unfinished *test* | a limit of this analyzer: a pattern it could not read |
//! | in the coverage matrix | expands to wildcards, hole-qualified | expands to wildcards, unreadable-qualified |
//! | what it can establish | a *possibly* verdict the author can discharge | nothing — no coverage, no shadowing |
//! | what it reports | [`MatchObligation::PossiblyInexhaustive`], [`MatchObligation::PossiblyRedundant`] | [`MatchObligation::AnalysisIncomplete`], and only where the readable rows leave the question open |
//! | how it is discharged | fill the hole | nothing the author can do |
//!
//! The reason to separate them is that conflating them tells the author their
//! program is unfinished when it is this analysis that is. Both still expand to
//! wildcards, because both might have been anything and neither can be ruled
//! out; what differs is what may be *concluded* from a row that reached its
//! answer through one.
//!
//! Neither qualifier ever leaves this module. Nothing here is exported to
//! solving as an equality or a disequality — the analysis imports nothing from
//! the unifier, and a *possibly* verdict is a report, never a constraint.
//!
//! # This is analysis, not enforcement
//!
//! Nothing here blocks checking, lowering, or evaluation. The obligations are
//! **data on the elaboration result**; the branch statuses are published on the
//! synthesis stream. A match whose branches are all `Possibly` still lowers,
//! still types, and its siblings still land — which is the property the whole
//! live-matching line exists to preserve. Rendering these obligations belongs
//! to the diagnostics-reporting lane and is deliberately absent here.
//!
//! # Where the inputs come from
//!
//! Pattern structure comes from the **surface** CST, because that is the only
//! place it still exists: the core calculus compiles patterns away, so a
//! lowered `case` carries tag-indexed arms and nothing of the or-patterns,
//! as-patterns, literals, and nesting the programmer wrote. The scrutinee comes
//! from the **lowered term**, which is what a checkpoint retains. The analysis
//! is a pure function of those two: it caches nothing, and re-running it on an
//! unchanged input reproduces its result exactly.
//!
//! # Why the source records are retained, and where
//!
//! A checkpoint is the wrong place to look for a pattern. The core keeps what
//! it needs to type and evaluate, and patterns are not among it — so a session
//! that had only checkpoints could say nothing about a match submitted three
//! lines ago, and a consumer reading the stream would watch earlier matches
//! quietly stop existing.
//!
//! So the **session** retains one [`SubmissionRecord`] per submission: the
//! translated pattern shapes, the scrutinee each was checked against, and the
//! declarations they are decided against. Records are keyed by the source
//! item's own identity ([`MatchOrigin`]), never by a core item position — one
//! source item may lower to any number of core items, and an obligation about a
//! pattern the programmer wrote must survive that.
//!
//! Two consequences worth stating, because both are contracts:
//!
//! * **The core is untouched.** No checkpoint gains a field and no core
//!   equality changes. Two programs with identical lowered terms still have
//!   identical checkpoints — and may still differ in liveness, because the
//!   patterns that differ never reached the core.
//! * **A session with no records says so.** Opened on a persisted core artifact
//!   there is nothing to recompute from, and the answer is the typed
//!   [`RetainedSource::SourceNotRetained`] rather than an empty result that
//!   reads as "nothing to report".
//!
//! # What this slice analyzes, and what it leaves alone
//!
//! A match is recorded for analysis when it eliminates a **declared datatype**
//! — when some branch names a constructor the declaration registry resolves.
//! That is the boundary the contract draws: exhaustiveness is decided against
//! *the owning data declaration's* constructor set, and a match that names no
//! declared constructor has no such set to be decided against. Two matches fall
//! outside it in consequence, and both are deliberate:
//!
//! * the structural eliminators — the built-in sum's `Inl` / `Inr` and the
//!   list's `Nil` / `Cons` — whose two-constructor shape the lowerer already
//!   requires complete;
//! * a match every one of whose branches is a hole or a binder, which names no
//!   constructor and so reveals no datatype to be exhaustive over.
//!
//! # Reading the recognizer in pattern context
//!
//! The `SynNode` recognizer classifies a bracketed or braced pattern by its
//! expression kind ([`node_kinds::LIST_EXPRESSION`],
//! [`node_kinds::RECORD_EXPRESSION`]) — those spellings are shared between the
//! sorts and the melder records the sort separately. Reaching them from a
//! pattern slot is what makes them list and record *patterns*, so the
//! translation below reads them that way and says so at each site.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::discipline::mark::Mark;
use gandr_core_incremental::stream::BranchStatus;
use gandr_core_incremental::stream::Liveness;
use gandr_core_incremental::stream::MatchOrigin as PublishedOrigin;
use gandr_core_term::syntax::NumLit;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;

use crate::boundary::BranchIndex;
use crate::boundary::ConstructorArity;
use crate::boundary::ConstructorName;
use crate::boundary::EntailingPrefix;
use crate::boundary::MatchIndex;
use crate::boundary::PatternHoleId;
use crate::boundary::PendingResultCount;
use crate::boundary::RowHoleDischargeable;
use crate::boundary::RowReadability;
use crate::boundary::ScrutineeIndeterminacy;
use crate::boundary::SourceItemId;
use crate::boundary::SourceRange;
use crate::boundary::SubmissionIndex;
use crate::boundary::TileSpelling;
use crate::boundary::TypeName;
use crate::lower::decode_string_escapes;
use crate::lower::node_kinds;
use crate::synnode::SynNode;

/// The tile that marks a list pattern's rest, which the melder commits flat
/// beside the elements rather than as a node of its own.
const REST_MARKER: TileSpelling = TileSpelling("..");

// --- The declaration view ----------------------------------------------------

/// Where one constructor name lives: its datatype and its position in that
/// datatype's declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructorRef
{
    /// The owning datatype's declared name.
    pub datatype: String,
    /// The constructor's position in the datatype's declaration order.
    pub tag: usize,
}

/// One constructor as its datatype declares it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredConstructor
{
    /// The constructor's name.
    pub name: String,
    /// The constructor's declared field count.
    pub arity: usize,
}

/// The declaration view the analysis reads: which datatype a constructor name
/// belongs to, and what the whole constructor set of a datatype is.
///
/// It is a snapshot rather than a borrow of the lowerer's registries because
/// the analysis runs after lowering has finished with them, and because a
/// snapshot is what makes the analysis a function of its arguments alone.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConstructorTable
{
    /// Constructor name to its datatype and tag.
    names: BTreeMap<String, ConstructorRef>,
    /// Datatype name to its constructors, in declaration order.
    datatypes: BTreeMap<String, Vec<DeclaredConstructor>>,
}

impl ConstructorTable
{
    /// An empty table — every constructor name is unresolvable.
    ///
    /// # Contract
    /// - ensures: resolves no name and knows no datatype.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — the empty table is the identity the lowerer builds
    ///   from, so its only property is that it resolves nothing.
    /// - witness: `live_match::tests::a_missing_constructor_is_reported_by_name`
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Records one datatype's constructors, in declaration order.
    ///
    /// # Contract
    /// - requires: `constructors` is in declaration order, so a constructor's
    ///   index is its tag.
    /// - ensures: every recorded name resolves to `datatype` at its index, and
    ///   the datatype's full constructor set becomes readable.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — a tag recorded at the wrong index would refute the
    ///   wrong branches, and a constructor set recorded short would report a
    ///   present constructor as uncovered.
    /// - witness: `live_match::tests::a_missing_constructor_is_reported_by_name`
    /// - witness: `live_match::tests::a_constructor_scrutinee_refutes_the_head_and_suspends_the_payload`
    #[inline]
    pub fn declare(
        &mut self,
        datatype: TypeName<'_>,
        constructors: Vec<DeclaredConstructor>,
    )
    {
        let datatype = datatype.as_ref().to_owned();
        for (tag, constructor) in constructors.iter().enumerate() {
            let _replaced = self.names.insert(constructor.name.clone(), ConstructorRef {
                datatype: datatype.clone(),
                tag,
            });
        }
        let _replaced = self.datatypes.insert(datatype, constructors);
    }

    /// The datatype and tag a constructor name resolves to.
    ///
    /// # Contract
    /// - ensures: `None` for a name no declaration in scope introduces.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — an unresolvable name must degrade a branch to
    ///   undecided rather than refute it, which is what an unknown constructor
    ///   is entitled to.
    /// - witness: `live_match::tests::a_constructor_scrutinee_refutes_the_head_and_suspends_the_payload`
    #[inline]
    #[must_use]
    pub fn resolve(
        &self,
        name: ConstructorName<'_>,
    ) -> Option<&ConstructorRef>
    {
        self.names.get(name.as_ref())
    }

    /// A datatype's full constructor set, in declaration order.
    ///
    /// # Contract
    /// - ensures: `None` for a datatype no declaration in scope introduces.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the constructor set is the domain exhaustiveness is
    ///   decided over, so its order and its completeness are both load-bearing.
    /// - witness: `live_match::tests::completing_the_match_clears_the_obligation`
    #[inline]
    #[must_use]
    pub fn constructors(
        &self,
        datatype: TypeName<'_>,
    ) -> Option<&[DeclaredConstructor]>
    {
        self.datatypes.get(datatype.as_ref()).map(Vec::as_slice)
    }
}

// --- The constraint language -------------------------------------------------

/// A literal a pattern tests the scrutinee against.
///
/// The boolean case is here rather than under [`PatternShape::Ctor`] because
/// the surface lowers `true` / `false` to injections into `1 + 1` rather than
/// to declared constructors, so the value side is an [`Value::Inj`] and the
/// pattern side must speak the same language to compare with it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiteralConstraint
{
    /// An unsuffixed integer literal.
    Int(i64),
    /// A type-suffixed numeric literal.
    Num(NumLit),
    /// A string literal, with its escapes already decoded.
    Str(String),
    /// A boolean literal.
    Bool(bool),
    /// The unit literal `()`.
    Unit,
}

/// One pattern, as the analysis reads it — the translated image of the whole
/// landed pattern grammar.
///
/// This is the constraint language the rest of the module works in. It exists
/// so the decision procedures never touch a CST: translation happens once, and
/// what follows is ordinary structural recursion over a closed vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "Pattern shapes stay inline because translation and analysis own them structurally."
)]
pub enum PatternShape
{
    /// A pattern-position typed hole `?` / `?name` — an unfinished *test*,
    /// which is neither satisfied nor refuted by any scrutinee.
    Hole
    {
        /// The pattern-position mark this hole carries.
        mark: Mark,
        /// The hole's name, when written as `?name`.
        name: Option<String>,
    },
    /// The wildcard `_`.
    Wildcard,
    /// A variable binder, which matches anything and names it.
    Bind(String),
    /// A literal test.
    Literal(LiteralConstraint),
    /// A constructor pattern `C` or `C(p̄)`.
    Ctor
    {
        /// The constructor's name.
        name: String,
        /// The argument patterns, in declaration order.
        arguments: Vec<Self>,
    },
    /// A tuple pattern `(p, q, …)`.
    Tuple(Vec<Self>),
    /// A record pattern `#{ ℓ = p, … }`, sorted by label.
    Record(Vec<(String, Self)>),
    /// A list pattern `[p, q]` or `[p, ..r]`.
    List
    {
        /// The fixed leading element patterns.
        elements: Vec<Self>,
        /// The optional rest pattern, binding the remaining elements.
        rest: Option<Box<Self>>,
    },
    /// An or-pattern `p | q`.
    Or(Box<Self>, Box<Self>),
    /// An as-pattern `p as x`.
    As
    {
        /// The tested pattern.
        pattern: Box<Self>,
        /// The name the whole match is bound to.
        binder: String,
    },
    /// A region the parse could not resolve into a pattern — a repair, or a
    /// form outside the translated grammar.
    ///
    /// **Kept apart from [`PatternShape::Hole`] because the two are different
    /// kinds of not-knowing, and conflating them misattributes one to the
    /// other.** A hole is indeterminacy the *theory* has a name for: the
    /// programmer wrote an unfinished test, the analysis reads it as standing
    /// for any pattern, and filling it discharges whatever it made possible. An
    /// unreadable region is a limit of the *analyzer*: nothing about the
    /// program is unfinished, and no author action discharges it. So an
    /// unreadable region establishes no coverage and shadows no later branch,
    /// and where it leaves a question open the answer is
    /// [`MatchObligation::AnalysisIncomplete`] rather than a *possibly*
    /// obligation the author would read as work of theirs.
    Unreadable,
}

impl PatternShape
{
    /// Collects this pattern's hole marks, in source order.
    ///
    /// The walk carries its own stack. Children are pushed in reverse so that
    /// popping visits them in source order, which is the order the marks are
    /// reported in.
    fn collect_marks(
        &self,
        into: &mut Vec<Mark>,
    )
    {
        let mut pending: Vec<&Self> = alloc::vec![self];
        while let Some(shape) = pending.pop() {
            match *shape {
                | Self::Hole { ref mark, .. } => into.push(mark.clone()),
                | Self::Ctor {
                    arguments: ref children,
                    ..
                }
                | Self::Tuple(ref children) => {
                    pending.extend(children.iter().rev());
                },
                | Self::Record(ref fields) => {
                    pending.extend(fields.iter().rev().map(|field| &field.1));
                },
                | Self::List {
                    ref elements,
                    ref rest,
                } => {
                    if let Some(ref rest) = *rest {
                        pending.push(rest);
                    }
                    pending.extend(elements.iter().rev());
                },
                | Self::Or(ref left, ref right) => {
                    pending.push(right);
                    pending.push(left);
                },
                | Self::As { ref pattern, .. } => pending.push(pattern),
                | Self::Wildcard | Self::Bind(_) | Self::Literal(_) | Self::Unreadable => {},
            }
        }
    }

    /// The pattern under any as-binders — the part that actually tests.
    ///
    /// Iterative rather than recursive, as every walk in this module is: the
    /// patterns it reads come from user source, so host stack depth would be an
    /// input-controlled resource.
    fn tested(&self) -> &Self
    {
        let mut current = self;
        while let Self::As { ref pattern, .. } = *current {
            current = pattern;
        }
        current
    }
}

// --- The analysis input and output -------------------------------------------

/// One branch of a match, as recorded for analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchPattern
{
    /// The translated pattern.
    pub shape: PatternShape,
    /// The branch's source range.
    pub byte_range: SourceRange,
}

/// One match expression, with everything the analysis needs about it.
///
/// The scrutinee is the **lowered** value, so it is exactly what a checkpoint
/// retains and what the evaluator would eliminate; the branches are the
/// **surface** patterns, because lowering has already discarded their
/// structure.
///
/// A site is identified by the source item that owns it and its position within
/// that item — never by a core item position. Lowering assigns the source
/// identity as it reads the source file, before anything downstream decides how
/// many core items that source item becomes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchSite
{
    /// The source item that owns this match, within its own submission.
    pub source_item: SourceItemId,
    /// Source-order index of this match within that source item.
    pub match_index: MatchIndex,
    /// The match expression's source range.
    pub byte_range: SourceRange,
    /// The lowered scrutinee, holes and all.
    pub scrutinee: Value,
    /// The branches, in source order.
    pub branches: Vec<BranchPattern>,
}

/// Where one analyzed match came from: the submission it was written in, the
/// source item of that submission that owns it, and its position within that
/// item.
///
/// **This is the whole addressing scheme, and no core position enters it.** A
/// source item may lower to any number of core items, so a match addressed by
/// "the item's index" is addressed by something that is not a function of the
/// match. Addressed this way it is: the triple is assigned where the source is
/// read, and it names the same match for as long as the session keeps the
/// record.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchOrigin
{
    /// Which submission the match was written in.
    pub submission: SubmissionIndex,
    /// Which source item of that submission owns it.
    pub source_item: SourceItemId,
    /// Which match of that source item this is, in source order.
    pub match_index: MatchIndex,
}

impl From<MatchOrigin> for PublishedOrigin
{
    #[inline]
    fn from(value: MatchOrigin) -> Self
    {
        Self {
            submission: usize::from(value.submission).into(),
            source_item: usize::from(value.source_item).into(),
            match_index: usize::from(value.match_index).into(),
        }
    }
}

/// One obligation a match raises — typed data, never a rendered report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchObligation
{
    /// The branch set provably fails to cover these constructors, whatever any
    /// pattern hole is filled with.
    Inexhaustive
    {
        /// The uncovered constructors, in declaration order.
        uncovered: Vec<String>,
    },
    /// The branch set covers these constructors only if a pattern hole is
    /// filled to do so — the residue is real, and a hole could absorb it.
    PossiblyInexhaustive
    {
        /// The possibly-uncovered constructors, in declaration order.
        uncovered: Vec<String>,
    },
    /// Every value this branch matches is already matched by the branches
    /// before it.
    Redundant
    {
        /// The redundant branch.
        branch: BranchIndex,
        /// How many leading branches suffice to entail it.
        entailing_prefix: EntailingPrefix,
    },
    /// The branch would be redundant if a pattern hole in the branches before
    /// it were filled one way, and live if it were filled another — hole
    /// dependence is exactly what prevents the proof.
    PossiblyRedundant
    {
        /// The possibly-redundant branch.
        branch: BranchIndex,
        /// How many leading branches suffice to entail it once the holes are
        /// read as covering.
        entailing_prefix: EntailingPrefix,
    },
    /// A pattern this analysis could not read stands where the question would
    /// have been decided, so these constructors are left undecided.
    ///
    /// **Not a *possibly* obligation, and the distinction is the point.** The
    /// two `Possibly` variants describe the *program*: something is unfinished,
    /// and filling the hole settles it. This one describes the *analyzer*:
    /// nothing about the program is unfinished, and there is nothing the author
    /// can fill. It is raised only where the readable branches leave the
    /// question open — a constructor the readable branches already cover stays
    /// covered, and one they provably leave uncovered is reported here rather
    /// than as a proved gap, because an unreadable region might have been the
    /// pattern that covered it.
    AnalysisIncomplete
    {
        /// The constructors whose coverage could not be decided, in declaration
        /// order.
        undecided: Vec<String>,
    },
}

/// What the analysis concluded about one match.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchAnalysis
{
    /// Where the match came from — submission, source item, and position.
    pub origin: MatchOrigin,
    /// The match expression's source range.
    pub byte_range: SourceRange,
    /// The datatype this match eliminates, when the branches reveal one — the
    /// declaration exhaustiveness was decided against.
    pub datatype: Option<String>,
    /// One status per branch, in source order.
    pub branches: Vec<BranchStatus>,
    /// Each branch's source range, index-aligned with `branches`, so an
    /// obligation's branch index resolves to a span.
    pub branch_ranges: Vec<SourceRange>,
    /// The obligations this match raises.
    pub obligations: Vec<MatchObligation>,
    /// The pattern-position hole marks across all branches, in source order.
    pub pattern_marks: Vec<Mark>,
}

// --- Translation from the CST ------------------------------------------------

/// One step of the iterative translation.
#[expect(
    clippy::large_enum_variant,
    reason = "The translation worklist keeps emitted shapes inline for iterative ownership."
)]
enum Step<'tree>
{
    /// Read a CST node and emit its shape, or expand it into sub-steps.
    Visit(SynNode<'tree>),
    /// Assemble a compound shape from sub-shapes already emitted.
    Build(Frame),
    /// Emit a shape that needs no CST node — a repair standing in for a
    /// sub-pattern the parse did not supply.
    Emit(PatternShape),
}

/// A compound shape waiting on its sub-shapes.
enum Frame
{
    /// A constructor pattern and its argument count.
    Ctor(String, usize),
    /// A tuple pattern of the given width.
    Tuple(usize),
    /// A record pattern's labels, already sorted; one sub-shape per label.
    Record(Vec<String>),
    /// A list pattern's element count and whether the last sub-shape is a rest.
    List(usize, bool),
    /// An or-pattern's two alternatives.
    Or,
    /// An as-pattern's binder; one sub-shape.
    As(String),
}

/// Translates one pattern node into the constraint language.
///
/// # Contract
/// - requires: `node` sits in a pattern slot — a case arm's pattern, a binding
///   statement's binder, or a sub-pattern of one.
/// - ensures: covers the whole landed pattern grammar — wildcards, binders,
///   literals, constructors and their nesting, tuples, records, lists and their
///   rest, or-patterns, as-patterns, and pattern holes — and a node the
///   recognizer does not resolve becomes [`PatternShape::Unreadable`] rather
///   than a guess, so translation is total on any parse.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — whatever does not cross this boundary is invisible to the
///   decision procedures forever after, and would degrade silently into an
///   unreadable region rather than fail — so the witness asserts every form of
///   the landed grammar in one match, not a representative sample.
/// - witness: `live_match::tests::translation_covers_the_landed_pattern_grammar`
///
/// # Intension
/// The walk carries an explicit work stack rather than descending on the host
/// stack. Patterns come from user source, so nesting depth is an input-
/// controlled resource, and a translation that recursed would make a deeply
/// nested pattern a way to end the process rather than a way to get an answer.
#[inline]
#[must_use]
pub fn translate_pattern(node: SynNode<'_>) -> PatternShape
{
    let mut work: Vec<Step<'_>> = alloc::vec![Step::Visit(node)];
    let mut built: Vec<PatternShape> = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            | Step::Visit(node) => visit_pattern(node, &mut work, &mut built),
            | Step::Build(frame) => {
                let shape = assemble(frame, &mut built);
                built.push(shape);
            },
            | Step::Emit(shape) => built.push(shape),
        }
    }
    built.pop().unwrap_or(PatternShape::Unreadable)
}

/// Reads one pattern node: emits a leaf shape, or queues the assembly of a
/// compound one followed by its children in source order.
fn visit_pattern<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
    built: &mut Vec<PatternShape>,
)
{
    if bool::from(node.is_error()) {
        built.push(PatternShape::Unreadable);
        return;
    }
    match node.kind() {
        | node_kinds::HOLE => built.push(PatternShape::Hole {
            mark: Mark::PatternHole(hole_id_of(node).into()),
            name: node
                .child_by_field_name(node_kinds::FIELD_HOLE_NAME)
                .map(|name| name.text().as_ref().to_owned()),
        }),
        // The wildcard, and a rest pattern outside a list: both bind whatever
        // they are applied to and test nothing.
        | node_kinds::WILDCARD | node_kinds::REST_PATTERN => {
            built.push(PatternShape::Wildcard);
        },
        | node_kinds::IDENTIFIER => {
            built.push(PatternShape::Bind(node.text().as_ref().to_owned()));
        },
        // A bare nullary constructor reaching a pattern slot as a token.
        | node_kinds::CONSTRUCTOR => built.push(PatternShape::Ctor {
            name: node.text().as_ref().to_owned(),
            arguments: Vec::new(),
        }),
        | node_kinds::NUMBER => built.push(translate_number(node)),
        | node_kinds::TYPED_NUMBER => built.push(translate_typed_number(node)),
        | node_kinds::BOOLEAN => built.push(translate_boolean(node)),
        | node_kinds::STRING => built.push(translate_string(node)),
        | node_kinds::UNIT => built.push(PatternShape::Literal(LiteralConstraint::Unit)),
        | node_kinds::CONSTRUCTOR_PATTERN => visit_constructor(node, work, built),
        | node_kinds::TUPLE_PATTERN | node_kinds::TUPLE_EXPRESSION => {
            let elements = node.named_children();
            queue(work, Frame::Tuple(elements.len()), elements);
        },
        // A parenthesized pattern is its interior; the parens only group, so no
        // frame is needed.
        | node_kinds::PARENTHESIZED_EXPRESSION => match node.named_children().into_iter().next() {
            | Some(inner) => work.push(Step::Visit(inner)),
            | None => built.push(PatternShape::Unreadable),
        },
        // A `[ … ]` pattern; the recognizer names the bracketed form by its
        // expression kind, and reaching it from a pattern slot is what makes it
        // a list pattern.
        | node_kinds::LIST_EXPRESSION => visit_list(node, work),
        // A `#{ … }` pattern, named by its expression kind for the same reason.
        | node_kinds::RECORD_EXPRESSION => visit_record(node, work),
        | node_kinds::OR_PATTERN => visit_or(node, work, built),
        | node_kinds::AS_PATTERN => visit_as(node, work, built),
        | _ => built.push(PatternShape::Unreadable),
    }
}

/// Queues a compound shape's assembly followed by its children, so the children
/// are visited in source order and their shapes are on the stack when the frame
/// is assembled.
fn queue<'tree>(
    work: &mut Vec<Step<'tree>>,
    frame: Frame,
    children: Vec<SynNode<'tree>>,
)
{
    work.push(Step::Build(frame));
    work.extend(children.into_iter().rev().map(Step::Visit));
}

/// Assembles one compound shape from the sub-shapes on top of `built`.
fn assemble(
    frame: Frame,
    built: &mut Vec<PatternShape>,
) -> PatternShape
{
    match frame {
        | Frame::Ctor(name, arity) => PatternShape::Ctor {
            name,
            arguments: take_built(built, PendingResultCount::from(arity)),
        },
        | Frame::Tuple(width) => {
            PatternShape::Tuple(take_built(built, PendingResultCount::from(width)))
        },
        | Frame::Record(labels) => {
            let values = take_built(built, PendingResultCount::from(labels.len()));
            PatternShape::Record(labels.into_iter().zip(values).collect())
        },
        | Frame::List(count, has_rest) => {
            let mut elements = take_built(built, PendingResultCount::from(count));
            let rest = if has_rest {
                elements.pop().map(Box::new)
            }
            else {
                None
            };
            PatternShape::List { elements, rest }
        },
        | Frame::Or => {
            let mut alternatives = take_built(built, PendingResultCount::from(2_usize));
            let right = alternatives.pop().unwrap_or(PatternShape::Unreadable);
            let left = alternatives.pop().unwrap_or(PatternShape::Unreadable);
            PatternShape::Or(Box::new(left), Box::new(right))
        },
        | Frame::As(binder) => PatternShape::As {
            pattern: Box::new(
                take_built(built, PendingResultCount::from(1_usize))
                    .pop()
                    .unwrap_or(PatternShape::Unreadable),
            ),
            binder,
        },
    }
}

/// Takes the last `count` shapes off `built`, restored to source order.
///
/// A short stack is padded with repairs rather than truncated, so a frame whose
/// children did not all arrive still assembles into a shape of the right arity.
fn take_built(
    built: &mut Vec<PatternShape>,
    count: PendingResultCount,
) -> Vec<PatternShape>
{
    let count = usize::from(count);
    let mut taken = Vec::with_capacity(count);
    for _index in 0 .. count {
        taken.push(built.pop().unwrap_or(PatternShape::Unreadable));
    }
    taken.reverse();
    taken
}

/// The hole identity a pattern hole carries **for the analysis**: its start
/// byte.
///
/// The offset is used rather than a minted serial so the analysis stays a
/// function of the source it reads, and so re-analyzing a retained submission
/// reproduces the identities it first reported.
///
/// **This is not the identity the compiled term carries.** A pattern hole that
/// stops a match reaches the core as a `Comp::Hole` whose identity comes from
/// the lowerer's own allocator, in the arm-compilation seam, so the goal stream
/// addresses it exactly as it addresses an expression hole. The two
/// identities answer different questions — which unfinished test this is, and
/// which core node stands for it — and neither is derivable from the other.
fn hole_id_of(node: SynNode<'_>) -> PatternHoleId
{
    PatternHoleId::from(u32::try_from(node.byte_range().0.start).unwrap_or(u32::MAX))
}

/// Queues a constructor pattern `C` / `C(p̄)`.
fn visit_constructor<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
    built: &mut Vec<PatternShape>,
)
{
    let name = node
        .child_by_field_name(node_kinds::FIELD_CONSTRUCTOR)
        .map_or_else(String::new, |constructor| {
            constructor.text().as_ref().to_owned()
        });
    if name.is_empty() {
        built.push(PatternShape::Unreadable);
        return;
    }
    let arguments = node.children_by_field_name(node_kinds::FIELD_ARGUMENT);
    queue(work, Frame::Ctor(name, arguments.len()), arguments);
}

/// Queues a list pattern, splitting a trailing rest off the fixed prefix.
///
/// The melder keeps a list pattern **flat**: `[h, ..t]` commits its `..` as a
/// sibling tile, so the tail is an ordinary named child and only the tile says
/// it is a rest. The grammar puts the rest last and nowhere else, so the marker
/// and the child order together determine the split.
fn visit_list<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
)
{
    let elements = node.named_children();
    let has_rest = bool::from(node.has_top_level_tile(REST_MARKER));
    queue(work, Frame::List(elements.len(), has_rest), elements);
}

/// Queues a record pattern, label-sorted so two spellings of the same record
/// pattern translate identically.
fn visit_record<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
)
{
    let mut fields: Vec<(String, Option<SynNode<'tree>>)> = Vec::new();
    for field in node.named_children() {
        let Some(label) = field.child_by_field_name(node_kinds::FIELD_NAME)
        else {
            continue;
        };
        fields.push((
            label.text().as_ref().to_owned(),
            field.child_by_field_name(node_kinds::FIELD_VALUE),
        ));
    }
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    let labels: Vec<String> = fields.iter().map(|field| field.0.clone()).collect();
    work.push(Step::Build(Frame::Record(labels)));
    work.extend(fields.into_iter().rev().map(|field| match field.1 {
        | Some(value) => Step::Visit(value),
        | None => Step::Emit(PatternShape::Unreadable),
    }));
}

/// Queues an or-pattern from its two operands.
fn visit_or<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
    built: &mut Vec<PatternShape>,
)
{
    let children = node.named_children();
    if children.len() < 2 {
        built.push(PatternShape::Unreadable);
        return;
    }
    queue(work, Frame::Or, children);
}

/// Queues an as-pattern from its tested pattern and its binder.
fn visit_as<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<Step<'tree>>,
    built: &mut Vec<PatternShape>,
)
{
    let mut children = node.named_children().into_iter();
    let (Some(pattern), Some(binder)) = (children.next(), children.next())
    else {
        built.push(PatternShape::Unreadable);
        return;
    };
    queue(
        work,
        Frame::As(binder.text().as_ref().to_owned()),
        alloc::vec![pattern],
    );
}

/// Translates an unsuffixed numeric literal pattern, mirroring how the lowerer
/// reads the same token: a float-shaped numeral is an `f64`, an integer-shaped
/// one is the frozen integer literal.
fn translate_number(node: SynNode<'_>) -> PatternShape
{
    let source = node.text();
    let text = source.as_ref();
    let float_shaped = text.contains('.') || text.contains('e') || text.contains('E');
    let literal = if float_shaped {
        text.parse::<f64>()
            .ok()
            .filter(|float| float.is_finite())
            .map(|float| LiteralConstraint::Num(NumLit::f64(float)))
    }
    else {
        text.parse::<i64>().ok().map(LiteralConstraint::Int)
    };
    literal.map_or(PatternShape::Unreadable, PatternShape::Literal)
}

/// Translates a type-suffixed numeric literal pattern, mirroring the lowerer's
/// split of the three-character suffix from the digits.
fn translate_typed_number(node: SynNode<'_>) -> PatternShape
{
    let source = node.text();
    let text = source.as_ref();
    let parsed = text
        .len()
        .checked_sub(3)
        .and_then(|cut| text.get(.. cut).zip(text.get(cut ..)))
        .and_then(|(digits, suffix)| match suffix {
            | "u32" => digits.parse::<u32>().ok().map(NumLit::U32),
            | "u64" => digits.parse::<u64>().ok().map(NumLit::U64),
            | "i32" => digits.parse::<i32>().ok().map(NumLit::I32),
            | "i64" => digits.parse::<i64>().ok().map(NumLit::I64),
            | "f32" => digits
                .parse::<f32>()
                .ok()
                .filter(|float| float.is_finite())
                .map(NumLit::f32),
            | "f64" => digits
                .parse::<f64>()
                .ok()
                .filter(|float| float.is_finite())
                .map(NumLit::f64),
            | _ => None,
        });
    parsed.map_or(PatternShape::Unreadable, |number| {
        PatternShape::Literal(LiteralConstraint::Num(number))
    })
}

/// Translates a boolean literal pattern.
fn translate_boolean(node: SynNode<'_>) -> PatternShape
{
    match node.text().as_ref() {
        | "true" => PatternShape::Literal(LiteralConstraint::Bool(true)),
        | "false" => PatternShape::Literal(LiteralConstraint::Bool(false)),
        | _ => PatternShape::Unreadable,
    }
}

/// Translates a string literal pattern, stripping the delimiting quotes and
/// decoding escapes exactly as the lowerer does for the expression form.
fn translate_string(node: SynNode<'_>) -> PatternShape
{
    let source = node.text();
    let text = source.as_ref();
    let inner = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(text);
    PatternShape::Literal(LiteralConstraint::Str(decode_string_escapes(inner.into())))
}

// --- Branch satisfaction against the scrutinee -------------------------------

/// Kleene conjunction: a refutation anywhere refutes the whole.
fn conjoin(
    left: BranchStatus,
    right: BranchStatus,
) -> BranchStatus
{
    match (left, right) {
        | (BranchStatus::Refuted, _) | (_, BranchStatus::Refuted) => BranchStatus::Refuted,
        | (BranchStatus::Possibly, _) | (_, BranchStatus::Possibly) => BranchStatus::Possibly,
        | (BranchStatus::Satisfied, BranchStatus::Satisfied) => BranchStatus::Satisfied,
    }
}

/// Kleene disjunction: a satisfied alternative satisfies the whole.
fn disjoin(
    left: BranchStatus,
    right: BranchStatus,
) -> BranchStatus
{
    match (left, right) {
        | (BranchStatus::Satisfied, _) | (_, BranchStatus::Satisfied) => BranchStatus::Satisfied,
        | (BranchStatus::Possibly, _) | (_, BranchStatus::Possibly) => BranchStatus::Possibly,
        | (BranchStatus::Refuted, BranchStatus::Refuted) => BranchStatus::Refuted,
    }
}

/// Strips type annotations, which carry no matching information.
fn strip_annotations(value: &Value) -> &Value
{
    let mut current = value;
    while let Value::Annot(ref inner, _) = *current {
        current = inner;
    }
    current
}

/// Whether a value is indeterminate — a hole, or anything not yet a canonical
/// form the eliminator could fire on.
fn is_indeterminate(value: &Value) -> ScrutineeIndeterminacy
{
    ScrutineeIndeterminacy::from(matches!(
        *strip_annotations(value),
        Value::Hole(_) | Value::Var(_) | Value::Thunk(_, _) | Value::Stk(_)
    ))
}

/// Splits a right-nested payload into `arity` components, as the lowerer nests
/// them (`[]` ↦ `()`, `[v]` ↦ `v`, `[v₀, v₁, v₂]` ↦ `v₀ × (v₁ × v₂)`).
///
/// Returns `None` when the value is not nested that deeply — an indeterminate
/// payload, or a shape the lowerer did not build.
fn split_payload(
    value: &Value,
    arity: ConstructorArity,
) -> Option<Vec<Value>>
{
    let arity = usize::from(arity);
    if arity == 0 {
        return Some(Vec::new());
    }
    let mut components = Vec::with_capacity(arity);
    let mut current = value.clone();
    let last = arity.saturating_sub(1);
    for _index in 0 .. last {
        let Value::Pair(ref head, ref tail) = *strip_annotations(&current)
        else {
            return None;
        };
        components.push((**head).clone());
        let next = (**tail).clone();
        current = next;
    }
    components.push(current);
    Some(components)
}

/// One step of the iterative classification.
enum Test<'shape>
{
    /// Decide one pattern against one value.
    Pair(&'shape PatternShape, Value),
    /// Fold the last `usize` verdicts with a conjunction — every part of a
    /// compound pattern must hold.
    All(usize),
    /// Fold the last `usize` verdicts with a disjunction — an or-pattern holds
    /// when either alternative does.
    Any(usize),
}

/// Classifies one branch against the scrutinee.
///
/// # Contract
/// - requires: `table` resolves the constructor names the pattern mentions, or
///   the verdict degrades to [`BranchStatus::Possibly`] rather than guessing.
/// - ensures: returns [`BranchStatus::Satisfied`] or [`BranchStatus::Refuted`]
///   only when the verdict holds for **every** filling of every hole; anything
///   a hole could decide is [`BranchStatus::Possibly`].
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the three verdicts are distinguished by what the
///   scrutinee settles — a hole scrutinee settles nothing, a constructor head
///   settles the branches that name another constructor, and a hole *inside* a
///   payload settles only the branches that do not look at it. A two-valued
///   implementation, or one that let an indeterminate value refute, fails at
///   least one of the two fixtures.
/// - witness: `live_match::tests::a_hole_scrutinee_leaves_every_branch_possible`
/// - witness: `live_match::tests::a_constructor_scrutinee_refutes_the_head_and_suspends_the_payload`
///
/// # Intension
/// The walk carries an explicit work stack. Both the pattern and the value come
/// from user source, so nesting depth is an input-controlled resource, and the
/// verdict folds are associative and commutative — which is what lets a stack
/// replace the descent without changing an answer.
#[inline]
#[must_use]
pub fn classify(
    shape: &PatternShape,
    scrutinee: &Value,
    table: &ConstructorTable,
) -> BranchStatus
{
    let mut work: Vec<Test<'_>> = alloc::vec![Test::Pair(shape, scrutinee.clone())];
    let mut decided: Vec<BranchStatus> = Vec::new();
    while let Some(test) = work.pop() {
        match test {
            | Test::Pair(shape, value) => decide(shape, &value, table, &mut work, &mut decided),
            | Test::All(count) => {
                let verdict = fold_verdicts(
                    &mut decided,
                    PendingResultCount::from(count),
                    BranchStatus::Satisfied,
                    conjoin,
                );
                decided.push(verdict);
            },
            | Test::Any(count) => {
                let verdict = fold_verdicts(
                    &mut decided,
                    PendingResultCount::from(count),
                    BranchStatus::Refuted,
                    disjoin,
                );
                decided.push(verdict);
            },
        }
    }
    decided.pop().unwrap_or(BranchStatus::Possibly)
}

/// Folds the last `count` verdicts with `combine`, starting from `unit`.
fn fold_verdicts(
    decided: &mut Vec<BranchStatus>,
    count: PendingResultCount,
    unit: BranchStatus,
    combine: fn(BranchStatus, BranchStatus) -> BranchStatus,
) -> BranchStatus
{
    let mut verdict = unit;
    for _index in 0 .. usize::from(count) {
        let part = decided.pop().unwrap_or(BranchStatus::Possibly);
        verdict = combine(verdict, part);
    }
    verdict
}

/// Decides one pattern against one value, queuing sub-tests for a compound
/// pattern.
fn decide<'shape>(
    shape: &'shape PatternShape,
    scrutinee: &Value,
    table: &ConstructorTable,
    work: &mut Vec<Test<'shape>>,
    decided: &mut Vec<BranchStatus>,
)
{
    let value = strip_annotations(scrutinee);
    match *shape {
        | PatternShape::Wildcard | PatternShape::Bind(_) => {
            decided.push(BranchStatus::Satisfied);
        },
        | PatternShape::As { ref pattern, .. } => {
            work.push(Test::Pair(pattern, value.clone()));
        },
        | PatternShape::Or(ref left, ref right) => {
            work.push(Test::Any(2));
            work.push(Test::Pair(right, value.clone()));
            work.push(Test::Pair(left, value.clone()));
        },
        // Undecided, for two different reasons that reach the same verdict —
        // which is why this is one arm and the coverage matrix has two. A hole
        // is an unfinished test, undecided until it is filled and decided by
        // filling it. An unreadable region is undecided because nobody read it,
        // and no author action changes that. Neither may be satisfied or
        // refuted, and `BranchStatus` has one value for undecided, so the
        // classification cannot tell them apart and does not pretend to: the
        // distinction is carried by [`RowQualifier`], where it changes an
        // answer.
        | PatternShape::Hole { .. } | PatternShape::Unreadable => {
            decided.push(BranchStatus::Possibly);
        },
        | _ if bool::from(is_indeterminate(value)) => decided.push(BranchStatus::Possibly),
        | PatternShape::Literal(ref literal) => decided.push(classify_literal(literal, value)),
        | PatternShape::Ctor {
            ref name,
            ref arguments,
        } => decide_constructor(
            ConstructorName::from(name.as_str()),
            arguments,
            value,
            table,
            work,
            decided,
        ),
        | PatternShape::Tuple(ref elements) => {
            match split_payload(value, ConstructorArity::from(elements.len())) {
                | Some(components) => queue_pairs(work, elements, components),
                | None => decided.push(BranchStatus::Possibly),
            }
        },
        | PatternShape::Record(ref fields) => decide_record(fields, value, work, decided),
        | PatternShape::List {
            ref elements,
            ref rest,
        } => decide_list(elements, rest.as_deref(), value, work, decided),
    }
}

/// Queues one sub-test per pattern-value pair, under a conjunction.
fn queue_pairs<'shape>(
    work: &mut Vec<Test<'shape>>,
    shapes: &'shape [PatternShape],
    values: Vec<Value>,
)
{
    work.push(Test::All(shapes.len().min(values.len())));
    for (shape, value) in shapes.iter().zip(values).rev() {
        work.push(Test::Pair(shape, value));
    }
}

/// Classifies a literal test against a canonical value.
fn classify_literal(
    literal: &LiteralConstraint,
    value: &Value,
) -> BranchStatus
{
    let decided = match (literal, value) {
        | (&LiteralConstraint::Int(expected), &Value::Int(actual)) => Some(expected == actual),
        | (&LiteralConstraint::Num(expected), &Value::Num(actual)) => Some(expected == actual),
        | (&LiteralConstraint::Str(ref expected), &Value::Str(ref actual)) => {
            Some(expected == actual)
        },
        | (&LiteralConstraint::Bool(expected), &Value::Inj(side, _)) => {
            Some(expected == matches!(side, Side::Fst))
        },
        | (&LiteralConstraint::Unit, &Value::Unit) => Some(true),
        | _ => None,
    };
    match decided {
        | Some(true) => BranchStatus::Satisfied,
        | Some(false) => BranchStatus::Refuted,
        // The value is canonical but of another shape: the program is
        // ill-typed elsewhere, and a refutation here would be a claim this
        // analysis has no standing to make.
        | None => BranchStatus::Possibly,
    }
}

/// Decides a constructor pattern against a canonical value, queuing its
/// arguments against the payload's components.
fn decide_constructor<'shape>(
    name: ConstructorName<'_>,
    arguments: &'shape [PatternShape],
    value: &Value,
    table: &ConstructorTable,
    work: &mut Vec<Test<'shape>>,
    decided: &mut Vec<BranchStatus>,
)
{
    let Value::Ctor {
        ref id,
        tag,
        ref payload,
    } = *value
    else {
        decided.push(BranchStatus::Possibly);
        return;
    };
    let Some(reference) = table.resolve(name)
    else {
        decided.push(BranchStatus::Possibly);
        return;
    };
    if reference.datatype != id.name().as_ref() || reference.tag != tag {
        decided.push(BranchStatus::Refuted);
        return;
    }
    let Some(components) = split_payload(payload, ConstructorArity::from(arguments.len()))
    else {
        // The payload has not been built out to the constructor's arity — an
        // indeterminate payload, so every field is undecided.
        decided.push(if arguments.is_empty() {
            BranchStatus::Satisfied
        }
        else {
            BranchStatus::Possibly
        });
        return;
    };
    queue_pairs(work, arguments, components);
}

/// Decides a record pattern against a canonical record value.
fn decide_record<'shape>(
    fields: &'shape [(String, PatternShape)],
    value: &Value,
    work: &mut Vec<Test<'shape>>,
    decided: &mut Vec<BranchStatus>,
)
{
    let Value::Record(ref actual) = *value
    else {
        decided.push(BranchStatus::Possibly);
        return;
    };
    let mut components = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(component) = actual.get(&field.0)
        else {
            // The value has no such field: no filling of any hole can give it
            // one.
            decided.push(BranchStatus::Refuted);
            return;
        };
        components.push((**component).clone());
    }
    work.push(Test::All(fields.len()));
    for (field, component) in fields.iter().zip(components).rev() {
        work.push(Test::Pair(&field.1, component));
    }
}

/// Decides a list pattern against a canonical list value.
fn decide_list<'shape>(
    elements: &'shape [PatternShape],
    rest: Option<&'shape PatternShape>,
    value: &Value,
    work: &mut Vec<Test<'shape>>,
    decided: &mut Vec<BranchStatus>,
)
{
    let Value::List(ref actual) = *value
    else {
        decided.push(BranchStatus::Possibly);
        return;
    };
    let length_fits = match rest {
        | Some(_) => actual.len() >= elements.len(),
        | None => actual.len() == elements.len(),
    };
    if !length_fits {
        decided.push(BranchStatus::Refuted);
        return;
    }
    let paired = elements.len().min(actual.len());
    work.push(Test::All(
        paired.saturating_add(usize::from(rest.is_some())),
    ));
    if let Some(rest) = rest {
        let tail = Value::List(
            actual
                .iter()
                .skip(elements.len())
                .cloned()
                .collect::<Vec<_>>(),
        );
        work.push(Test::Pair(rest, tail));
    }
    for (element, component) in elements.iter().zip(actual.iter()).rev() {
        work.push(Test::Pair(element, (**component).clone()));
    }
}

// --- Coverage: exhaustiveness and redundancy ---------------------------------

/// How completely a set of patterns covers a queried pattern.
///
/// **Three middle values, because three different things prevent a proof and
/// each licenses a different report.** A hole is the programmer's unfinished
/// work, so hole-qualified uncertainty is reported as something filling the
/// hole would settle. A column this procedure declines — a list family, a mixed
/// column — is a boundary drawn by design, and reporting it would be noise
/// about a decision already made. An unreadable region is neither: the analysis
/// simply failed to read a pattern that bears on the question, and that is
/// worth saying precisely because nobody would otherwise know the question went
/// unasked.
///
/// The order is the strength order, and `meet` is `min`: a conjunction is only
/// as covered as its weakest part, and a part *provably* uncovered settles the
/// conjunction whatever the others say.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Coverage
{
    /// Provably not covered, however any hole is filled.
    Uncovered,
    /// Undecided because a pattern the analysis could not read stands where the
    /// answer would have come from.
    AnalysisIncomplete,
    /// Undecided because this procedure declines the column by design.
    Declined,
    /// Undecided, and a pattern hole is what would decide it.
    MaybeHole,
    /// Provably covered without appealing to any hole.
    Covered,
}

impl Coverage
{
    /// The weaker of two coverage verdicts — the conjunction used when every
    /// part of a query must be covered.
    fn meet(
        self,
        other: Self,
    ) -> Self
    {
        core::cmp::min(self, other)
    }
}

/// Why a row of the coverage matrix is standing in for a pattern rather than
/// being one — nothing, a hole, an unreadable region, or both.
///
/// **The two qualifiers are stored separately because they discharge
/// differently.** A hole-qualified row records that filling a hole would settle
/// what the row contributes; an unreadable-qualified row records that nothing
/// the author does will. A row qualified both ways is settled by neither, which
/// is exactly what keeping them apart lets the base case see.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RowQualifier
{
    /// A pattern hole was read as a wildcard to reach this row.
    hole: bool,
    /// A pattern the analysis could not read was read as a wildcard to reach
    /// this row.
    unreadable: bool,
}

impl RowQualifier
{
    /// An ordinary row: every step that produced it was a pattern the analysis
    /// read.
    const READABLE: Self = Self {
        hole: false,
        unreadable: false,
    };
    /// A row reached through a pattern hole.
    const THROUGH_HOLE: Self = Self {
        hole: true,
        unreadable: false,
    };
    /// A row reached through a region the analysis could not read.
    const THROUGH_UNREADABLE: Self = Self {
        hole: false,
        unreadable: true,
    };

    /// This row's qualification together with one more step's.
    fn with(
        self,
        step: Self,
    ) -> Self
    {
        Self {
            hole: self.hole || step.hole,
            unreadable: self.unreadable || step.unreadable,
        }
    }

    /// Whether nothing stood in for a pattern on the way to this row — the only
    /// case that can establish coverage.
    fn readable(self) -> RowReadability
    {
        RowReadability::from(!self.hole && !self.unreadable)
    }

    /// Whether the author could settle what this row leaves open by filling a
    /// hole. False once an unreadable region is on the path, because no filling
    /// reaches that.
    fn author_dischargeable(self) -> RowHoleDischargeable
    {
        RowHoleDischargeable::from(!self.unreadable)
    }
}

/// One row of the coverage matrix: a pattern vector, and what the path that
/// produced it passed through.
#[derive(Clone, Debug)]
struct Row
{
    /// The row's remaining pattern columns.
    cells: Vec<PatternShape>,
    /// What stood in for a pattern on the way to this row.
    qualifier: RowQualifier,
}

/// The head constructor of a pattern — the key the coverage matrix specializes
/// on.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Head
{
    /// A declared constructor, with its argument count.
    Ctor(String, usize),
    /// A boolean literal.
    Bool(bool),
    /// An integer literal.
    Int(i64),
    /// A typed numeric literal.
    Num(NumLit),
    /// A string literal.
    Str(String),
    /// The unit literal.
    Unit,
    /// A tuple of the given width.
    Tuple(usize),
    /// A record with the given labels, sorted.
    Record(Vec<String>),
    /// A list of exactly the given length.
    ListExact(usize),
    /// A list of at least the given length, with a rest pattern.
    ListOpen(usize),
}

impl Head
{
    /// How many sub-patterns this head exposes when specialized on.
    fn arity(&self) -> ConstructorArity
    {
        ConstructorArity::from(match *self {
            | Self::Ctor(_, arity) => arity,
            | Self::Bool(_) | Self::Int(_) | Self::Num(_) | Self::Str(_) | Self::Unit => 0,
            | Self::Tuple(width) => width,
            | Self::Record(ref labels) => labels.len(),
            | Self::ListExact(length) => length,
            | Self::ListOpen(prefix) => prefix.saturating_add(1),
        })
    }
}

/// The head of a pattern, or `None` for one that matches everything (a
/// wildcard, a binder, a hole, an unreadable region) or that has several heads
/// (an or-pattern, which is expanded before this is asked).
fn head_of(shape: &PatternShape) -> Option<Head>
{
    match *shape.tested() {
        | PatternShape::Ctor {
            ref name,
            ref arguments,
        } => Some(Head::Ctor(name.clone(), arguments.len())),
        | PatternShape::Literal(LiteralConstraint::Bool(value)) => Some(Head::Bool(value)),
        | PatternShape::Literal(LiteralConstraint::Int(value)) => Some(Head::Int(value)),
        | PatternShape::Literal(LiteralConstraint::Num(value)) => Some(Head::Num(value)),
        | PatternShape::Literal(LiteralConstraint::Str(ref value)) => {
            Some(Head::Str(value.clone()))
        },
        | PatternShape::Literal(LiteralConstraint::Unit) => Some(Head::Unit),
        | PatternShape::Tuple(ref elements) => Some(Head::Tuple(elements.len())),
        | PatternShape::Record(ref fields) => Some(Head::Record(
            fields.iter().map(|field| field.0.clone()).collect(),
        )),
        | PatternShape::List {
            ref elements,
            ref rest,
        } => Some(if rest.is_some() {
            Head::ListOpen(elements.len())
        }
        else {
            Head::ListExact(elements.len())
        }),
        | _ => None,
    }
}

/// The sub-patterns a pattern exposes under its own head.
fn head_arguments(shape: &PatternShape) -> Vec<PatternShape>
{
    match *shape.tested() {
        | PatternShape::Ctor { ref arguments, .. } => arguments.clone(),
        | PatternShape::Tuple(ref elements) => elements.clone(),
        | PatternShape::Record(ref fields) => fields.iter().map(|field| field.1.clone()).collect(),
        | PatternShape::List {
            ref elements,
            ref rest,
        } => {
            let mut arguments = elements.clone();
            if let Some(ref rest) = *rest {
                arguments.push((**rest).clone());
            }
            arguments
        },
        | _ => Vec::new(),
    }
}

/// Expands every top-level or-pattern in a row's first column into separate
/// rows, so each row has at most one head.
fn expand_rows(rows: Vec<Row>) -> Vec<Row>
{
    let mut expanded: Vec<Row> = Vec::with_capacity(rows.len());
    let mut pending = rows;
    while let Some(row) = pending.pop() {
        let split = row.cells.first().and_then(|cell| match *cell.tested() {
            | PatternShape::Or(ref left, ref right) => Some(((**left).clone(), (**right).clone())),
            | _ => None,
        });
        match split {
            | Some((left, right)) => {
                push_alternative(&mut pending, &row, left);
                push_alternative(&mut pending, &row, right);
            },
            | None => expanded.push(row),
        }
    }
    expanded.reverse();
    expanded
}

/// Queues one alternative of an expanded or-pattern as a row of its own.
fn push_alternative(
    pending: &mut Vec<Row>,
    row: &Row,
    alternative: PatternShape,
)
{
    let mut cells = row.cells.clone();
    if let Some(slot) = cells.first_mut() {
        *slot = alternative;
    }
    pending.push(Row {
        cells,
        qualifier: row.qualifier,
    });
}

/// Specializes the matrix on `head`: rows whose first column can match a value
/// headed by `head`, with that column replaced by the head's sub-columns.
///
/// A hole and an unreadable region both widen to wildcards here, because both
/// stand where a pattern would have and neither can be ruled out. They part
/// company in what they *record*: the qualifier carried forward is the whole
/// difference between a residue the author can discharge and one the analysis
/// merely failed to read.
fn specialize(
    rows: &[Row],
    head: &Head,
) -> Vec<Row>
{
    let arity = usize::from(head.arity());
    let mut specialized = Vec::new();
    for row in rows {
        let Some((first, rest)) = row.cells.split_first()
        else {
            continue;
        };
        let tested = first.tested();
        let mut push_with = |prefix: Vec<PatternShape>, step: RowQualifier| {
            let mut cells = prefix;
            cells.extend_from_slice(rest);
            specialized.push(Row {
                cells,
                qualifier: row.qualifier.with(step),
            });
        };
        match *tested {
            | PatternShape::Wildcard | PatternShape::Bind(_) => {
                push_with(
                    alloc::vec![PatternShape::Wildcard; arity],
                    RowQualifier::READABLE,
                );
            },
            // An unfinished test: read as a wildcard, and recorded as a hole so
            // whatever it makes possible is reported as the author's to settle.
            | PatternShape::Hole { .. } => {
                push_with(
                    alloc::vec![PatternShape::Wildcard; arity],
                    RowQualifier::THROUGH_HOLE,
                );
            },
            // A pattern the analysis could not read: read as a wildcard for
            // soundness — it might have been anything, so it cannot be ruled
            // out — and recorded as unreadable, which is what stops it
            // establishing coverage or shadowing a later branch.
            | PatternShape::Unreadable => {
                push_with(
                    alloc::vec![PatternShape::Wildcard; arity],
                    RowQualifier::THROUGH_UNREADABLE,
                );
            },
            | _ => {
                if let Some(row_head) = head_of(first)
                    && let Some(arguments) = match_heads(&row_head, head, first)
                {
                    push_with(arguments, RowQualifier::READABLE);
                }
            },
        }
    }
    specialized
}

/// The sub-patterns a row contributes when specialized on `head`, or `None`
/// when the row's own head cannot match a value headed by `head`.
///
/// A list row with a rest pattern is the one head that matches a family of
/// other heads: `[p, ..r]` matches every exact length from one upward, so it
/// contributes its prefix padded with wildcards.
fn match_heads(
    row_head: &Head,
    head: &Head,
    shape: &PatternShape,
) -> Option<Vec<PatternShape>>
{
    if row_head == head {
        return Some(head_arguments(shape));
    }
    match (row_head, head) {
        | (&Head::ListOpen(prefix), &Head::ListExact(length)) if prefix <= length => {
            let mut arguments = head_arguments(shape);
            // The rest sub-pattern is the last argument; it binds the tail,
            // which at this exact length is the remaining elements.
            let _rest = arguments.pop();
            arguments.resize(length, PatternShape::Wildcard);
            Some(arguments)
        },
        | _ => None,
    }
}

/// The default matrix: rows whose first column matches values of any head not
/// present in the first column.
///
/// The two standing-in forms are separated here for the same reason as in
/// [`specialize`]: both reach the default matrix, and what they record about
/// how they got there is what the base case reads.
fn default_matrix(rows: &[Row]) -> Vec<Row>
{
    let mut defaulted = Vec::new();
    for row in rows {
        let Some((first, rest)) = row.cells.split_first()
        else {
            continue;
        };
        let mut defaulted_with = |step: RowQualifier| {
            defaulted.push(Row {
                cells: rest.to_vec(),
                qualifier: row.qualifier.with(step),
            });
        };
        match *first.tested() {
            | PatternShape::Wildcard | PatternShape::Bind(_) => {
                defaulted_with(RowQualifier::READABLE);
            },
            // An unfinished test reaches the default matrix qualified by the
            // hole, so a residue it absorbs is reported as hole-dependent.
            | PatternShape::Hole { .. } => defaulted_with(RowQualifier::THROUGH_HOLE),
            // An unreadable region reaches it qualified as unreadable, so a
            // residue it absorbs is reported as undecided rather than as
            // anything the author wrote.
            | PatternShape::Unreadable => defaulted_with(RowQualifier::THROUGH_UNREADABLE),
            | _ => {},
        }
    }
    defaulted
}

/// What the first column's head set says about its domain.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Signature
{
    /// The domain is finite and every one of its heads is present, so covering
    /// the column means covering each head in turn.
    Complete(Vec<Head>),
    /// The domain has a head the column does not carry — an absent constructor,
    /// or a literal domain that is simply unbounded. A value with that head
    /// reaches only the default matrix, which is therefore exact.
    Open,
    /// This procedure declines to decide the column.
    ///
    /// The declined cases are the ones where the head set is not a
    /// constructor set: **lists**, whose `[..r]` covers a family of lengths
    /// rather than one head, and a column mixing shapes that a well-typed
    /// program would not mix. Declining is not the same as `Open`: routing
    /// these through the default matrix as though it were exact would let a
    /// dropped row read as a proved gap, and a proved gap is the one verdict
    /// this analysis must never invent.
    Undecided,
}

/// Which domain a head belongs to; heads from two domains never share a column
/// in a well-typed program.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Domain
{
    /// A declared datatype.
    Ctor,
    /// The booleans.
    Bool,
    /// An unbounded literal domain.
    Literal,
    /// The unit type.
    Unit,
    /// Tuples of one width.
    Tuple,
    /// Records of one label set.
    Record,
    /// Lists.
    List,
}

/// The domain a head belongs to.
fn domain_of(head: &Head) -> Domain
{
    match *head {
        | Head::Ctor(..) => Domain::Ctor,
        | Head::Bool(_) => Domain::Bool,
        | Head::Int(_) | Head::Num(_) | Head::Str(_) => Domain::Literal,
        | Head::Unit => Domain::Unit,
        | Head::Tuple(_) => Domain::Tuple,
        | Head::Record(_) => Domain::Record,
        | Head::ListExact(_) | Head::ListOpen(_) => Domain::List,
    }
}

/// Reads the first column's head set and classifies its domain.
fn signature_of(
    rows: &[Row],
    table: &ConstructorTable,
) -> Signature
{
    let mut heads: Vec<Head> = Vec::new();
    for row in rows {
        if let Some(cell) = row.cells.first()
            && let Some(head) = head_of(cell)
            && !heads.contains(&head)
        {
            heads.push(head);
        }
    }
    let Some(first) = heads.first()
    else {
        // No row tests this column, so every value reaches the default matrix.
        return Signature::Open;
    };
    let domain = domain_of(first);
    if heads.iter().any(|head| domain_of(head) != domain) {
        return Signature::Undecided;
    }
    match domain {
        | Domain::Ctor => constructor_signature(&heads, table),
        | Domain::Bool => {
            let has = |wanted: bool| heads.contains(&Head::Bool(wanted));
            if has(true) && has(false) {
                Signature::Complete(alloc::vec![Head::Bool(true), Head::Bool(false)])
            }
            else {
                Signature::Open
            }
        },
        // One head is the whole domain — but only when the column agrees on
        // which head that is. Two tuple widths or two record label sets in one
        // column are not a signature, and specializing on either would drop the
        // rows carrying the other.
        | Domain::Unit | Domain::Tuple | Domain::Record => {
            if heads.len() == 1 {
                Signature::Complete(alloc::vec![first.clone()])
            }
            else {
                Signature::Undecided
            }
        },
        // A literal domain is unbounded: a value outside the column's literals
        // always exists, and it reaches the default matrix.
        | Domain::Literal => Signature::Open,
        // `[..r]` covers every length from its prefix upward, so the column's
        // heads are not a constructor set and the default-matrix reduction does
        // not apply.
        | Domain::List => Signature::Undecided,
    }
}

/// The signature of a declared-datatype column: complete when every declared
/// constructor appears, open when one is missing, and undecided when the
/// declaration is not in scope to say.
fn constructor_signature(
    heads: &[Head],
    table: &ConstructorTable,
) -> Signature
{
    let Some(head) = heads.first()
    else {
        return Signature::Undecided;
    };
    let Head::Ctor(ref name, _) = *head
    else {
        return Signature::Undecided;
    };
    let Some(reference) = table.resolve(ConstructorName::from(name.as_str()))
    else {
        return Signature::Undecided;
    };
    let Some(declared) = table.constructors(TypeName::from(reference.datatype.as_str()))
    else {
        return Signature::Undecided;
    };
    let present = |constructor: &DeclaredConstructor| {
        heads.iter().any(|head| match *head {
            | Head::Ctor(ref seen, _) => *seen == constructor.name,
            | _ => false,
        })
    };
    if declared.iter().all(present) {
        Signature::Complete(
            declared
                .iter()
                .map(|constructor| Head::Ctor(constructor.name.clone(), constructor.arity))
                .collect(),
        )
    }
    else {
        Signature::Open
    }
}

/// One step of the iterative coverage decision.
enum Job
{
    /// Decide whether `rows` covers `query`.
    Solve
    {
        /// The coverage matrix at this point.
        rows: Vec<Row>,
        /// The query vector at this point.
        query: Vec<PatternShape>,
    },
    /// Meet the last `usize` verdicts — every part must be covered.
    Meet(usize),
    /// Meet the last verdict with a ceiling, capping what may be concluded.
    Cap(Coverage),
}

/// Whether every value matching `query` is matched by some row of `rows`.
///
/// This is Maranget's usefulness recursion read as coverage, with holes given
/// their own verdict rather than a truth value, and driven from an explicit
/// work stack rather than the host stack: the patterns and the arities come
/// from user source, so descent depth is an input-controlled resource. Every
/// combination in the recursion is a meet, and meet is associative and
/// commutative, so the stack reproduces the descent's answers exactly.
fn coverage(
    rows: &[Row],
    query: &[PatternShape],
    table: &ConstructorTable,
) -> Coverage
{
    let mut work: Vec<Job> = alloc::vec![Job::Solve {
        rows: rows.to_vec(),
        query: query.to_vec(),
    }];
    let mut decided: Vec<Coverage> = Vec::new();
    while let Some(job) = work.pop() {
        match job {
            | Job::Solve { rows, query } => solve(rows, &query, table, &mut work, &mut decided),
            | Job::Meet(count) => {
                let mut verdict = Coverage::Covered;
                for _index in 0 .. count {
                    let part = decided.pop().unwrap_or(Coverage::Declined);
                    verdict = verdict.meet(part);
                }
                decided.push(verdict);
            },
            | Job::Cap(ceiling) => {
                let verdict = decided.pop().unwrap_or(Coverage::Declined);
                decided.push(verdict.meet(ceiling));
            },
        }
    }
    decided.pop().unwrap_or(Coverage::Declined)
}

/// Decides one coverage sub-problem, or queues the sub-problems it reduces to.
fn solve(
    rows: Vec<Row>,
    query: &[PatternShape],
    table: &ConstructorTable,
    work: &mut Vec<Job>,
    decided: &mut Vec<Coverage>,
)
{
    let rows = expand_rows(rows);
    let Some((first, rest)) = query.split_first()
    else {
        // Width zero: a row that survived to here matches the query, and what
        // it stood in for on the way decides what may be concluded from it.
        // The order is the strength order — a row that read the whole way
        // proves coverage; failing that, a hole-only row makes the question the
        // author's to settle; failing that, an unreadable region left it
        // unasked; and no row at all is the one proved gap.
        decided.push(
            if rows.iter().any(|row| bool::from(row.qualifier.readable())) {
                Coverage::Covered
            }
            else if rows
                .iter()
                .any(|row| bool::from(row.qualifier.author_dischargeable()))
            {
                Coverage::MaybeHole
            }
            else if rows.is_empty() {
                Coverage::Uncovered
            }
            else {
                Coverage::AnalysisIncomplete
            },
        );
        return;
    };
    match *first.tested() {
        | PatternShape::Or(ref left, ref right) => {
            work.push(Job::Meet(2));
            for alternative in [right, left] {
                let mut next = alloc::vec![(**alternative).clone()];
                next.extend_from_slice(rest);
                work.push(Job::Solve {
                    rows: rows.clone(),
                    query: next,
                });
            }
        },
        // A query hole is read as a wildcard: proving the widened query covered
        // proves the actual one covered, whatever the hole becomes.
        | PatternShape::Wildcard | PatternShape::Bind(_) | PatternShape::Hole { .. } => {
            solve_wildcard(&rows, rest, table, work);
        },
        // An unreadable column in the *query* is widened the same way, for the
        // same soundness reason, and then capped. Widening concludes nothing
        // false, but a verdict reached over a pattern nobody read is still a
        // verdict about that pattern — and this analysis has no standing to
        // call a branch it could not read entailed by the branches before it.
        | PatternShape::Unreadable => {
            work.push(Job::Cap(Coverage::AnalysisIncomplete));
            solve_wildcard(&rows, rest, table, work);
        },
        // A list query with a rest ranges over unboundedly many lengths; this
        // procedure declines to decide it rather than guess in either
        // direction.
        | PatternShape::List { ref rest, .. } if rest.is_some() => {
            decided.push(Coverage::Declined);
        },
        | _ => {
            let Some(head) = head_of(first)
            else {
                decided.push(Coverage::Declined);
                return;
            };
            let mut next = head_arguments(first);
            next.extend_from_slice(rest);
            work.push(Job::Solve {
                rows: specialize(&rows, &head),
                query: next,
            });
        },
    }
}

/// Queues the sub-problems a wildcard query reduces to, by what the first
/// column's signature says.
fn solve_wildcard(
    rows: &[Row],
    rest: &[PatternShape],
    table: &ConstructorTable,
    work: &mut Vec<Job>,
)
{
    match signature_of(rows, table) {
        | Signature::Complete(signature) => {
            work.push(Job::Meet(signature.len()));
            for head in &signature {
                let mut next = alloc::vec![PatternShape::Wildcard; usize::from(head.arity())];
                next.extend_from_slice(rest);
                work.push(Job::Solve {
                    rows: specialize(rows, head),
                    query: next,
                });
            }
        },
        | Signature::Open => work.push(Job::Solve {
            rows: default_matrix(rows),
            query: rest.to_vec(),
        }),
        | Signature::Undecided => {
            work.push(Job::Cap(Coverage::Declined));
            work.push(Job::Solve {
                rows: default_matrix(rows),
                query: rest.to_vec(),
            });
        },
    }
}

// --- The analysis ------------------------------------------------------------

/// One submission's retained source-side match records: the patterns as the
/// analysis reads them, the scrutinee each was checked against, and the
/// declarations they are decided against.
///
/// **This is what a session keeps so that a match analyzed once stays analyzed
/// across later submissions.** Nothing of the CST is here — the patterns are
/// already translated into the constraint language, which is what makes the
/// record small, stable, and independent of the parse that produced it. Nothing
/// of the core is here either: checkpoints are untouched by any of this, and
/// their equality is what it was.
///
/// The declaration table is retained *with* the sites, at the submission's own
/// granularity, so re-analyzing an earlier submission reproduces the verdicts
/// it originally got rather than re-deciding it against declarations that
/// arrived afterwards.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmissionRecord
{
    /// Which submission these records belong to.
    pub submission: SubmissionIndex,
    /// The match sites the submission's source contributed, in lowering order.
    pub sites: Vec<MatchSite>,
    /// The declarations the sites are decided against, as they stood when the
    /// submission was lowered.
    pub table: ConstructorTable,
}

impl SubmissionRecord
{
    /// Analyzes every site this record retains.
    ///
    /// # Contract
    /// - ensures: one [`MatchAnalysis`] per site, in the order the sites were
    ///   recorded, each stamped with this record's submission index — and the
    ///   result is a function of the record alone, so re-analyzing an unchanged
    ///   record reproduces an unchanged result.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the record is the *whole* input, which is the
    ///   property that makes a resumed session's earlier submissions
    ///   reproducible; a verdict that depended on anything outside it would
    ///   move when nothing about the source did.
    /// - witness: `live_match::tests::a_prior_submissions_matches_survive_the_next_one`
    /// - witness: `live_match::tests::a_resumed_session_matches_an_uninterrupted_one`
    #[inline]
    #[must_use]
    pub fn analyze(&self) -> Vec<MatchAnalysis>
    {
        analyze(&self.sites, &self.table, self.submission)
    }
}

/// One submission's source records, or the typed marker standing where they
/// were not retained.
///
/// A session opened on a persisted core artifact holds checkpoints and no
/// source: the patterns the programmer wrote were never in the core to begin
/// with. [`RetainedSource::SourceNotRetained`] is what that session says about
/// its earlier submissions, and saying it is the contract — the alternative is
/// a silence a consumer reads as "those matches are all decided now".
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetainedSource
{
    /// The submission's source records are in hand and its matches are
    /// recomputable.
    Records(SubmissionRecord),
    /// The submission's source records were not retained.
    SourceNotRetained
    {
        /// The submission whose records are gone.
        submission: SubmissionIndex,
    },
}

/// The whole program's match liveness, from the source records a session
/// retained.
///
/// # Contract
/// - ensures: one keyed entry per recorded match, so a source item that lowered
///   to several core items still contributes exactly one entry per match it
///   holds; and one `SourceNotRetained` marker per submission whose records
///   were dropped.
/// - ensures: the result is a function of `records` alone — recomputed from the
///   records every time rather than accumulated, so nothing can drift out of
///   step with the source that produced it.
/// - panics: none.
/// - intension: recomputing rather than caching costs one pass over every match
///   the session has ever recorded. That is the price of the ensures above, and
///   it is charged in matches rather than in items: a session whose source
///   holds no `case` does no work here at all.
///
/// # Adequacy
/// - hypothesis: L3 — the three cases a weaker implementation conflates are a
///   prior submission still recorded, a prior submission whose records are
///   gone, and a current submission being analyzed now. Positional schemes lose
///   the first, silent schemes lose the second, and both look identical on a
///   one-submission fixture.
/// - witness: `live_match::tests::a_prior_submissions_matches_survive_the_next_one`
/// - witness: `live_match::tests::a_record_stripped_resume_reports_source_not_retained`
/// - witness: `live_match::tests::one_source_item_lowering_to_several_core_items_keeps_one_entry`
#[inline]
#[must_use]
pub fn liveness(records: &[RetainedSource]) -> Liveness
{
    let mut published = Liveness::new();
    for retained in records {
        match *retained {
            | RetainedSource::Records(ref record) => {
                for analysis in record.analyze() {
                    let _displaced =
                        published.insert(PublishedOrigin::from(analysis.origin), analysis.branches);
                }
            },
            | RetainedSource::SourceNotRetained { submission } => {
                published.mark_unretained(usize::from(submission).into());
            },
        }
    }
    published
}

/// Analyzes every recorded match site of one submission.
///
/// # Contract
/// - ensures: returns one [`MatchAnalysis`] per site, in the order the sites
///   were recorded, each carrying the origin `(submission, source item, match
///   index)`; the result is a function of the sites, the table, and the
///   submission index alone — no state is kept between calls, so an unchanged
///   input reproduces an unchanged output.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the batch adds nothing to the per-site decision but the
///   order and the submission stamp, so the property worth distinguishing is
///   that two runs over the same checkpoint agree; a cache or any retained
///   state would break it silently.
/// - witness: `live_match::tests::adoption_does_not_move_a_verdict`
#[inline]
#[must_use]
pub fn analyze(
    sites: &[MatchSite],
    table: &ConstructorTable,
    submission: SubmissionIndex,
) -> Vec<MatchAnalysis>
{
    sites
        .iter()
        .map(|site| analyze_site(site, table, submission))
        .collect()
}

/// Analyzes one match site.
///
/// # Contract
/// - ensures: every branch receives a status, every uncovered constructor is
///   named, every entailed branch is reported with the length of the prefix
///   that entails it, and a match with no branches raises no obligations — the
///   absurd match over an uninhabited datatype is well-formed, and the
///   datatype's own emptiness is the checker's business, not this analysis's.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — each obligation is proved in its own direction, so the
///   fixtures must separate three things a weaker implementation conflates: a
///   residue no branch covers, the same residue with a pattern hole beside it,
///   and no residue at all. Redundancy is separated the same way, and the
///   entailing prefix is asserted rather than merely the fact of entailment.
/// - witness: `live_match::tests::a_missing_constructor_is_reported_by_name`
/// - witness: `live_match::tests::a_pattern_hole_downgrades_the_residue`
/// - witness: `live_match::tests::completing_the_match_clears_the_obligation`
/// - witness: `live_match::tests::an_or_pattern_entails_a_later_repeat_of_either_alternative`
/// - witness: `live_match::tests::an_as_pattern_entails_through_its_binder`
/// - witness: `live_match::tests::a_hole_in_the_prefix_leaves_redundancy_unproved`
/// - witness: `live_match::tests::a_live_branch_raises_no_redundancy_obligation`
/// - witness: `live_match::tests::a_pattern_hole_carries_the_pattern_position_mark`
/// - witness: `live_match::tests::an_unreadable_row_leaves_a_missing_constructor_undecided`
/// - witness: `live_match::tests::a_readable_row_after_an_unreadable_twin_is_not_redundant`
/// - witness: `live_match::tests::an_exhaustive_match_stays_exhaustive_beside_an_unreadable_row`
#[inline]
#[must_use]
pub fn analyze_site(
    site: &MatchSite,
    table: &ConstructorTable,
    submission: SubmissionIndex,
) -> MatchAnalysis
{
    let branches: Vec<BranchStatus> = site
        .branches
        .iter()
        .map(|branch| classify(&branch.shape, &site.scrutinee, table))
        .collect();
    let mut pattern_marks = Vec::new();
    for branch in &site.branches {
        branch.shape.collect_marks(&mut pattern_marks);
    }
    let datatype = owning_datatype(&site.branches, table);
    let mut obligations = Vec::new();
    if !site.branches.is_empty() {
        obligations.extend(exhaustiveness(
            site,
            datatype.as_deref().map(TypeName::from),
            table,
        ));
        obligations.extend(redundancy(site, table));
    }
    MatchAnalysis {
        origin: MatchOrigin {
            submission,
            source_item: site.source_item,
            match_index: site.match_index,
        },
        byte_range: site.byte_range.clone(),
        datatype,
        branches,
        branch_ranges: site
            .branches
            .iter()
            .map(|branch| branch.byte_range.clone())
            .collect(),
        obligations,
        pattern_marks,
    }
}

/// The rows of the coverage matrix for a prefix of a site's branches.
fn rows_of(
    site: &MatchSite,
    upto: BranchIndex,
) -> Vec<Row>
{
    site.branches
        .iter()
        .take(usize::from(upto))
        .map(|branch| Row {
            cells: alloc::vec![branch.shape.clone()],
            qualifier: RowQualifier::READABLE,
        })
        .collect()
}

/// Decides exhaustiveness against the owning data declaration's constructor
/// set, naming the constructors the branches leave uncovered.
///
/// Each constructor lands in exactly one of four places, and which one is the
/// whole content of the three-valued design. A proved gap is an
/// `Inexhaustive`; a gap a hole could absorb is a `PossiblyInexhaustive`; a
/// question an unreadable pattern left unasked is an `AnalysisIncomplete`, said
/// separately because it is not the author's to discharge; and a covered
/// constructor, or one in a column this procedure declines by design, is
/// silence.
fn exhaustiveness(
    site: &MatchSite,
    datatype: Option<TypeName<'_>>,
    table: &ConstructorTable,
) -> Vec<MatchObligation>
{
    let Some(datatype) = datatype
    else {
        return Vec::new();
    };
    let Some(declared) = table.constructors(datatype)
    else {
        return Vec::new();
    };
    let rows = rows_of(site, BranchIndex::from(site.branches.len()));
    let mut uncovered: Vec<String> = Vec::new();
    let mut possibly: Vec<String> = Vec::new();
    let mut undecided: Vec<String> = Vec::new();
    for constructor in declared {
        let query = alloc::vec![PatternShape::Ctor {
            name: constructor.name.clone(),
            arguments: alloc::vec![PatternShape::Wildcard; constructor.arity],
        }];
        match coverage(&rows, &query, table) {
            | Coverage::Uncovered => uncovered.push(constructor.name.clone()),
            | Coverage::MaybeHole => possibly.push(constructor.name.clone()),
            | Coverage::AnalysisIncomplete => undecided.push(constructor.name.clone()),
            | Coverage::Declined | Coverage::Covered => {},
        }
    }
    let mut obligations = Vec::new();
    if !uncovered.is_empty() {
        obligations.push(MatchObligation::Inexhaustive { uncovered });
    }
    if !possibly.is_empty() {
        obligations.push(MatchObligation::PossiblyInexhaustive {
            uncovered: possibly,
        });
    }
    if !undecided.is_empty() {
        obligations.push(MatchObligation::AnalysisIncomplete { undecided });
    }
    obligations
}

/// Decides redundancy of each branch against the disjunction of the branches
/// before it, naming the shortest prefix that already entails it.
///
/// An unreadable region on either side raises nothing. In the prefix it
/// contributes no shadowing, so a later branch it stands beside is live as far
/// as anyone can prove; in the branch itself it caps the verdict, so a pattern
/// nobody read is never reported as entailed. Both fall through to silence:
/// naming a branch redundant is an instruction to delete it, and this analysis
/// issues that instruction only from a proof.
fn redundancy(
    site: &MatchSite,
    table: &ConstructorTable,
) -> Vec<MatchObligation>
{
    let mut obligations = Vec::new();
    for (index, branch) in site.branches.iter().enumerate() {
        if index == 0 {
            continue;
        }
        let query = alloc::vec![branch.shape.clone()];
        let verdict = coverage(&rows_of(site, BranchIndex::from(index)), &query, table);
        let obligation = match verdict {
            | Coverage::Covered => MatchObligation::Redundant {
                branch: BranchIndex::from(index),
                entailing_prefix: shortest_prefix(
                    site,
                    BranchIndex::from(index),
                    &query,
                    table,
                    Coverage::Covered,
                ),
            },
            | Coverage::MaybeHole => MatchObligation::PossiblyRedundant {
                branch: BranchIndex::from(index),
                entailing_prefix: shortest_prefix(
                    site,
                    BranchIndex::from(index),
                    &query,
                    table,
                    Coverage::MaybeHole,
                ),
            },
            | Coverage::AnalysisIncomplete | Coverage::Declined | Coverage::Uncovered => continue,
        };
        obligations.push(obligation);
    }
    obligations
}

/// The length of the shortest leading prefix of the branches that already
/// reaches `wanted` for this query.
fn shortest_prefix(
    site: &MatchSite,
    branch: BranchIndex,
    query: &[PatternShape],
    table: &ConstructorTable,
    wanted: Coverage,
) -> EntailingPrefix
{
    let branch = usize::from(branch);
    for length in 1 ..= branch {
        if coverage(&rows_of(site, BranchIndex::from(length)), query, table) >= wanted {
            return EntailingPrefix::from(length);
        }
    }
    EntailingPrefix::from(branch)
}

/// The declaration a branch set eliminates, read off the first constructor name
/// the table resolves.
///
/// # Contract
/// - ensures: `None` when no branch names a declared constructor — a match over
///   a structural sum, a list, or a scrutinee whose datatype is not in scope.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — exhaustiveness is decided against whatever this returns,
///   so a wrong answer would name the wrong constructor set and a `None` where
///   a datatype exists would silence the obligation entirely.
/// - witness: `live_match::tests::a_missing_constructor_is_reported_by_name`
#[inline]
#[must_use]
pub fn owning_datatype(
    branches: &[BranchPattern],
    table: &ConstructorTable,
) -> Option<String>
{
    branches
        .iter()
        .find_map(|branch| first_constructor(&branch.shape, table))
}

/// The first constructor name in a pattern that the table resolves, with its
/// datatype.
///
/// The walk carries its own stack and visits or-alternatives left to right. An
/// alternative naming a constructor the table cannot resolve does not stop the
/// search: a later alternative may name one it can, and the question asked here
/// is which declaration the branch set eliminates.
fn first_constructor(
    shape: &PatternShape,
    table: &ConstructorTable,
) -> Option<String>
{
    let mut pending: Vec<&PatternShape> = alloc::vec![shape];
    while let Some(current) = pending.pop() {
        match *current.tested() {
            | PatternShape::Ctor { ref name, .. } => {
                if let Some(reference) = table.resolve(ConstructorName::from(name.as_str())) {
                    return Some(reference.datatype.clone());
                }
            },
            | PatternShape::Or(ref left, ref right) => {
                pending.push(right);
                pending.push(left);
            },
            | _ => {},
        }
    }
    None
}
