//! Public model for checked precedence-bounded grammars.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use core::borrow::Borrow;
use core::error::Error;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use gandr_surface_syntax::GrammarFingerprint;
use gandr_surface_syntax::GroutSort;
use gandr_theory_graphs::Bound;
use gandr_theory_graphs::Dir;
use gandr_theory_graphs::Prec;
use gandr_theory_graphs::PrecDag;
use gandr_theory_graphs::PrecSpecError;
use gandr_theory_graphs::WalkBuildError;

/// Define a transparent static-text identity wrapper with string access traits.
///
/// The generated newtype wraps a `&'static str`, derives the standard
/// value-semantics traits, and exposes `AsRef<str>`, `Borrow<str>`, and
/// `Display` so dependent crates can compare, key, and print grammar
/// identities without touching the payload.
macro_rules! static_text_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub &'static str);

        impl AsRef<str> for $name
        {
            #[inline]
            fn as_ref(&self) -> &str
            {
                self.0
            }
        }

        impl Borrow<str> for $name
        {
            #[inline]
            fn borrow(&self) -> &str
            {
                self.0
            }
        }

        impl Display for $name
        {
            #[inline]
            fn fmt(
                &self,
                f: &mut Formatter<'_>,
            ) -> FmtResult
            {
                f.write_str(self.0)
            }
        }
    };
}

static_text_wrapper!(RuleName, "Stable identity for a checked grammar rule.");
static_text_wrapper!(TileLabel, "Static label for one grammar tile occurrence.");
static_text_wrapper!(
    Provenance,
    "Tree-sitter named-kind or generated surface provenance for a grammar rule."
);
static_text_wrapper!(PrecName, "Static name of a precedence group.");
static_text_wrapper!(
    SurfaceForm,
    "Original surface form captured by an adaptation record."
);
static_text_wrapper!(
    AdaptationReason,
    "Audit reason for a grammar surface adaptation."
);
static_text_wrapper!(SortName, "Stable display name for a closed grammar sort.");

/// Count of declared mold definitions in a checked grammar.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MoldCount(pub usize);

/// Presence flag for a precedence group in a table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PrecPresence(pub bool);
/// Count of mold candidates declared for one tile label.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CandidateCount(pub usize);

use crate::check::validate_assumption_3;
use crate::check::validate_operator_form;
use crate::mold::MoldDef;
use crate::mold::MoldId;
use crate::mold::MoldTable;
use crate::mold::RCtxId;
use crate::mold::RCtxStep;

/// Closed precedence-bounded grammar sort.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Sort
{
    /// Top-level item grammar sort.
    Item,
    /// Pattern grammar sort.
    Pattern,
    /// Expression grammar sort.
    Expression,
    /// Type grammar sort.
    Type,
    /// Instantiation-slot resident grammar sort.
    Instantiation,
}

impl Sort
{
    /// Return the compact mold sort tag.
    #[inline]
    #[must_use]
    pub const fn as_u16(self) -> GroutSort
    {
        match self {
            | Self::Item => GroutSort(0),
            | Self::Pattern => GroutSort(1),
            | Self::Expression => GroutSort(2),
            | Self::Type => GroutSort(3),
            | Self::Instantiation => GroutSort(4),
        }
    }

    /// Return the stable sort name.
    #[inline]
    #[must_use]
    pub const fn name(self) -> SortName
    {
        match self {
            | Self::Item => SortName("item"),
            | Self::Pattern => SortName("pattern"),
            | Self::Expression => SortName("expression"),
            | Self::Type => SortName("type"),
            | Self::Instantiation => SortName("instantiation"),
        }
    }

    /// Decode a compact mold sort tag.
    ///
    /// # Contract
    /// - requires: `sort` is a compact grammar sort tag supplied by syntax
    ///   data.
    /// - ensures: returns the matching closed [`Sort`] for tags emitted by
    ///   [`Sort::as_u16`].
    /// - provides: the checked boundary from `gandr-surface-syntax` molds back
    ///   to PBG sorts.
    /// - fails: returns [`PbgError::InvalidSort`] for tags outside the closed
    ///   five-sort vocabulary.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::InvalidSort`] when `sort` is not a PBG sort tag.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the five accepted tags and the first
    ///   rejected tag distinguish exact closed-vocabulary decoding.
    /// - witness: `gandr_surface_grammar::contracts::sort_decode_contract`
    #[inline]
    pub const fn try_from_tag(sort: GroutSort) -> Result<Self, PbgError>
    {
        match sort.0 {
            | 0 => Ok(Self::Item),
            | 1 => Ok(Self::Pattern),
            | 2 => Ok(Self::Expression),
            | 3 => Ok(Self::Type),
            | 4 => Ok(Self::Instantiation),
            | other => Err(PbgError::InvalidSort { sort: other }),
        }
    }
}

/// Terminal tile carried by a grammar regex.
///
/// A tile is declared by label and position only; its mold identity
/// `{rctx, prec, sort}` is assigned at [`Pbg`] build from each occurrence's
/// regex-zipper context. The retired `mold: Mold` field is gone.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile
{
    /// Static surface label emitted by the tree-sitter named rule.
    pub label: &'static str,
}

impl Tile
{
    /// Construct a terminal tile from its label.
    ///
    /// # Contract
    /// - requires: `label` names a static surface token or named terminal form.
    /// - ensures: preserves `label` exactly.
    /// - provides: the tile atom used by regex leaves and mold assignment.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — a single-field constructor with no branches; label
    ///   retention is witnessed through the declared candidate inventory.
    /// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
    #[inline]
    #[must_use]
    pub const fn new(label: TileLabel) -> Self
    {
        Self { label: label.0 }
    }
}

/// Symbol leaf in a regular grammar expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sym
{
    /// Recursive use of another grammar sort.
    Sort(Sort),
    /// Terminal syntax tile.
    Tile(Tile),
}

impl Sym
{
    /// Construct a recursive sort symbol.
    #[inline]
    #[must_use]
    pub const fn sort(sort: Sort) -> Self
    {
        Self::Sort(sort)
    }

    /// Construct a terminal tile symbol from its label.
    #[inline]
    #[must_use]
    pub const fn tile(label: TileLabel) -> Self
    {
        Self::Tile(Tile::new(label))
    }
}

/// Regular expression over grammar symbols.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Regex
{
    /// Empty language and empty sequence sentinel.
    Empty,
    /// One symbol leaf.
    Sym(Sym),
    /// Concatenation of child expressions.
    Seq(Vec<Self>),
    /// Alternation of child expressions.
    Alt(Vec<Self>),
    /// Optional child expression.
    Optional(Box<Self>),
    /// Zero-or-more repetition of a child expression.
    Repeat(Box<Self>),
}

impl Regex
{
    /// Construct an empty expression.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self
    {
        Self::Empty
    }

    /// Construct an empty expression with regex-style naming.
    #[inline]
    #[must_use]
    pub const fn eps() -> Self
    {
        Self::Empty
    }

    /// Construct a symbol expression.
    #[inline]
    #[must_use]
    pub const fn sym(sym: Sym) -> Self
    {
        Self::Sym(sym)
    }

    /// Construct a recursive sort expression.
    #[inline]
    #[must_use]
    pub const fn sort(sort: Sort) -> Self
    {
        Self::Sym(Sym::Sort(sort))
    }

    /// Construct a terminal tile expression from its label.
    #[inline]
    #[must_use]
    pub const fn tile(label: TileLabel) -> Self
    {
        Self::Sym(Sym::Tile(Tile::new(label)))
    }

    /// Construct a concatenation expression.
    #[inline]
    #[must_use]
    pub fn seq<I>(items: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self::Seq(items.into_iter().collect())
    }

    /// Construct an alternation expression.
    #[inline]
    #[must_use]
    pub fn alt<I>(items: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self::Alt(items.into_iter().collect())
    }

    /// Construct an optional expression.
    #[inline]
    #[must_use]
    pub fn optional(inner: Self) -> Self
    {
        Self::Optional(Box::new(inner))
    }

    /// Construct a zero-or-more repetition expression.
    #[inline]
    #[must_use]
    pub fn repeat(inner: Self) -> Self
    {
        Self::Repeat(Box::new(inner))
    }

    /// Return rule alternatives represented by this expression.
    #[inline]
    #[must_use]
    pub fn alternatives(&self) -> Vec<Self>
    {
        match *self {
            | Self::Alt(ref items) => items.clone(),
            | ref other => vec![other.clone()],
        }
    }
}

/// Surface adaptation record retained beside checked rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Adaptation
{
    /// Rule identity the adaptation belongs to.
    pub rule: &'static str,
    /// Original surface form before adaptation.
    pub surface: &'static str,
    /// Explanation of the adaptation.
    pub reason: &'static str,
}

impl Adaptation
{
    /// Construct a surface adaptation record.
    ///
    /// # Contract
    /// - requires: `rule` names an emitted rule and `surface` is the original
    ///   surface form being documented.
    /// - ensures: preserves the rule, surface, and reason references exactly.
    /// - provides: audit data only; adaptations never relax PBG validation.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — one non-empty record observes exact field
    ///   preservation and that validation ignores adaptation presence.
    /// - witness: `gandr_surface_grammar::contracts::adaptation_contract`
    #[inline]
    #[must_use]
    pub const fn new(
        rule: RuleName,
        surface: SurfaceForm,
        reason: AdaptationReason,
    ) -> Self
    {
        Self {
            rule: rule.0,
            surface: surface.0,
            reason: reason.0,
        }
    }
}

/// One named tree-sitter-provenance grammar rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule
{
    /// Stable static rule identity.
    pub name: &'static str,
    /// Form sort produced by this rule.
    pub sort: Sort,
    /// Precedence group for this rule.
    pub prec: Prec,
    /// Rule regular expression.
    pub regex: Regex,
    /// Named tree-sitter rule or generated surface provenance.
    pub provenance: &'static str,
    /// Audit-only adaptation records.
    pub adaptations: Vec<Adaptation>,
}

impl Rule
{
    /// Construct a rule whose provenance is its name.
    ///
    /// # Contract
    /// - requires: `name` is globally unique in the PBG being built.
    /// - ensures: stores a rule with no adaptations and provenance equal to the
    ///   rule name.
    /// - provides: the common constructor for direct tree-sitter named rules.
    /// - fails: never; uniqueness is checked by [`Pbg::build`].
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the PBG duplicate-name check and a
    ///   successful build witness observe name/provenance identity and delayed
    ///   validation.
    /// - witness: `gandr_surface_grammar::contracts::pbg_duplicate_rule_contract`
    #[inline]
    #[must_use]
    pub fn new(
        name: RuleName,
        sort: Sort,
        prec: Prec,
        regex: Regex,
    ) -> Self
    {
        Self {
            provenance: name.0,
            name: name.0,
            sort,
            prec,
            regex,
            adaptations: Vec::new(),
        }
    }

    /// Construct a rule with explicit tree-sitter provenance.
    #[inline]
    #[must_use]
    pub fn with_provenance(
        name: RuleName,
        provenance: Provenance,
        sort: Sort,
        prec: Prec,
        regex: Regex,
    ) -> Self
    {
        Self {
            name: name.0,
            sort,
            prec,
            regex,
            provenance: provenance.0,
            adaptations: Vec::new(),
        }
    }

    /// Construct a rule with one adaptation record.
    #[inline]
    #[must_use]
    pub fn with_adaptation(
        name: RuleName,
        sort: Sort,
        prec: Prec,
        regex: Regex,
        adaptation: Adaptation,
    ) -> Self
    {
        let mut rule = Self::new(name, sort, prec, regex);
        rule.adaptations.push(adaptation);
        rule
    }

    /// Return this rule's stable identity.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> RuleName
    {
        RuleName(self.name)
    }

    /// Return this rule's result sort.
    #[inline]
    #[must_use]
    pub const fn sort(&self) -> Sort
    {
        self.sort
    }

    /// Return this rule's precedence group.
    #[inline]
    #[must_use]
    pub const fn prec(&self) -> Prec
    {
        self.prec
    }

    /// Return this rule's regex.
    #[inline]
    #[must_use]
    pub const fn regex(&self) -> &Regex
    {
        &self.regex
    }

    /// Return this rule's provenance string.
    #[inline]
    #[must_use]
    pub const fn provenance(&self) -> Provenance
    {
        Provenance(self.provenance)
    }

    /// Return this rule's adaptation records.
    #[inline]
    #[must_use]
    pub fn adaptations(&self) -> &[Adaptation]
    {
        &self.adaptations
    }
}

/// Checked precedence table wrapper for grammar consumers.
#[derive(Clone, Debug)]
pub struct PrecTable
{
    /// Built precedence DAG.
    dag: PrecDag,
    /// Static names mapped to dense precedence ids.
    names: BTreeMap<PrecName, Prec>,
}

impl PrecTable
{
    /// Construct a precedence table from a built DAG and static name map.
    ///
    /// # Contract
    /// - requires: every `(name, Prec)` pair refers to the supplied `dag` and
    ///   names are the constant built-in precedence names.
    /// - ensures: stores `dag` and deterministic static-name lookup rows.
    /// - provides: zero-leak named precedence lookup for surface modules.
    /// - fails: never; invalid ids are rejected by later rule validation or
    ///   [`contains`](Self::contains).
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — one present and one absent name distinguish
    ///   lookup behavior without heap-allocating keys.
    /// - witness: `gandr_surface_grammar::contracts::prec_table_lookup_contract`
    #[inline]
    #[must_use]
    pub fn new<I>(
        dag: PrecDag,
        names: I,
    ) -> Self
    where
        I: IntoIterator<Item = (PrecName, Prec)>,
    {
        Self {
            dag,
            names: names.into_iter().collect(),
        }
    }

    /// Return the wrapped precedence DAG.
    #[inline]
    #[must_use]
    pub const fn dag(&self) -> &PrecDag
    {
        &self.dag
    }

    /// Consume the table and return its DAG.
    #[inline]
    #[must_use]
    pub fn into_dag(self) -> PrecDag
    {
        self.dag
    }

    /// Return whether `prec` names a built precedence group.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        prec: Prec,
    ) -> PrecPresence
    {
        PrecPresence(self.dag.name(prec).is_some())
    }

    /// Look up a named precedence group.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        name: PrecName,
    ) -> Option<Prec>
    {
        self.names.get(name.as_ref()).copied()
    }

    /// Look up a named precedence group or return a typed PBG error.
    ///
    /// # Contract
    /// - requires: `name` is the constant surface-table name for a precedence
    ///   group.
    /// - ensures: returns the corresponding dense [`Prec`] when present.
    /// - provides: the checked named-precedence boundary for generated sibling
    ///   surface modules.
    /// - fails: returns [`PbgError::MissingPrec`] when the name is absent.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::MissingPrec`] when `name` is not in the table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — one present and one absent name distinguish
    ///   successful lookup from typed failure.
    /// - witness: `gandr_surface_grammar::contracts::prec_table_lookup_contract`
    #[inline]
    pub fn prec(
        &self,
        name: PrecName,
    ) -> Result<Prec, PbgError>
    {
        self.get(name).ok_or(PbgError::MissingPrec { name: name.0 })
    }
}

/// Typed failure while constructing a checked PBG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PbgError
{
    /// Compact mold sort tag was not one of the closed PBG sorts.
    InvalidSort
    {
        /// Unknown compact sort tag.
        sort: u16,
    },
    /// Named precedence group lookup failed.
    MissingPrec
    {
        /// Missing precedence group name.
        name: &'static str,
    },
    /// Precedence specification construction failed before DAG validation.
    PrecedenceSpec(PrecSpecError),
    /// Precedence DAG construction found a named cycle witness.
    PrecedenceCycle
    {
        /// Named cycle witness, in deterministic witness order.
        witness: Vec<&'static str>,
    },
    /// Walk-index construction failed under the PBG API.
    Walk(WalkBuildError),
    /// Rule mentions a precedence id absent from the DAG.
    InvalidPrec
    {
        /// Rule identity.
        rule: &'static str,
        /// Invalid precedence id.
        prec: Prec,
    },
    /// Two rules share the same identity.
    DuplicateRule
    {
        /// Repeated rule identity.
        name: &'static str,
    },
    /// Operator Form found adjacent generated sort symbols.
    AdjacentSorts
    {
        /// Rule identity containing the violation.
        rule: &'static str,
        /// Sort exposed on the left side.
        left: Sort,
        /// Sort exposed on the right side.
        right: Sort,
    },
    /// Unique Tiles found two occurrences interning to the same `(label,
    /// rctx)`.
    DuplicateTile
    {
        /// Duplicated tile label.
        label: &'static str,
        /// Sort of the producing form.
        sort: Sort,
        /// Precedence of the producing form.
        prec: Prec,
        /// First rule that emitted the tile occurrence.
        first_rule: &'static str,
        /// Second rule that emitted the tile occurrence.
        second_rule: &'static str,
    },
    /// Assumption 3 found a cross-sort conflict: `s ∈ FIRST(G(r, p))` and
    /// `r ∈ LAST(G(s, q))` for distinct sorts `r ≠ s`.
    Assumption3Conflict
    {
        /// The sort `r`.
        first_sort: Sort,
        /// The sort `s` beginning a form of `r`.
        second_sort: Sort,
    },
    /// The mold table exceeded the compact `u32` identity width.
    MoldOverflow,
    /// A mold id did not name a mold in this grammar's table.
    UnknownMold
    {
        /// The out-of-range mold id.
        id: MoldId,
    },
    /// A regex-context id did not name an interned context in this grammar.
    UnknownRCtx
    {
        /// The out-of-range regex-context id.
        rctx: RCtxId,
    },
}

impl Display for PbgError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        match *self {
            | Self::InvalidSort { sort } => write!(f, "invalid PBG sort tag {sort}"),
            | Self::MissingPrec { name } => write!(f, "missing precedence group {name}"),
            | Self::PrecedenceSpec(ref error) => Display::fmt(error, f),
            | Self::PrecedenceCycle { ref witness } => {
                f.write_str("precedence cycle")?;
                for name in witness {
                    write!(f, " {name}")?;
                }
                Ok(())
            },
            | Self::Walk(ref error) => Display::fmt(error, f),
            | Self::InvalidPrec { rule, prec } => {
                write!(
                    f,
                    "rule {rule} mentions invalid precedence {}",
                    prec.index()
                )
            },
            | Self::DuplicateRule { name } => write!(f, "duplicate PBG rule {name}"),
            | Self::AdjacentSorts { rule, left, right } => write!(
                f,
                "rule {rule} exposes adjacent generated sorts {} and {}",
                left.name(),
                right.name()
            ),
            | Self::DuplicateTile {
                label,
                sort,
                prec,
                first_rule,
                second_rule,
            } => write!(
                f,
                "duplicate tile {label} at sort {} prec {} interns to one context in rules {first_rule} and {second_rule}",
                sort.name(),
                prec.index()
            ),
            | Self::Assumption3Conflict {
                first_sort,
                second_sort,
            } => write!(
                f,
                "assumption 3 conflict: {} begins a form of {} while {} ends a form of {}",
                second_sort.name(),
                first_sort.name(),
                first_sort.name(),
                second_sort.name()
            ),
            | Self::MoldOverflow => f.write_str("mold table exceeds u32 identity width"),
            | Self::UnknownMold { id } => write!(f, "unknown mold id {}", u32::from(id)),
            | Self::UnknownRCtx { rctx } => {
                write!(f, "unknown regex-context id {}", u32::from(rctx))
            },
        }
    }
}

impl Error for PbgError
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match *self {
            | Self::PrecedenceSpec(ref error) => Some(error),
            | Self::Walk(ref error) => Some(error),
            | Self::InvalidSort { .. }
            | Self::MissingPrec { .. }
            | Self::PrecedenceCycle { .. }
            | Self::InvalidPrec { .. }
            | Self::DuplicateRule { .. }
            | Self::AdjacentSorts { .. }
            | Self::DuplicateTile { .. }
            | Self::Assumption3Conflict { .. }
            | Self::MoldOverflow
            | Self::UnknownMold { .. }
            | Self::UnknownRCtx { .. } => None,
        }
    }
}

impl From<PrecSpecError> for PbgError
{
    #[inline]
    fn from(value: PrecSpecError) -> Self
    {
        Self::PrecedenceSpec(value)
    }
}

impl From<WalkBuildError> for PbgError
{
    #[inline]
    fn from(value: WalkBuildError) -> Self
    {
        Self::Walk(value)
    }
}

impl PbgError
{
    /// Construct a named precedence-cycle diagnostic.
    #[inline]
    #[must_use]
    pub fn precedence_cycle(witness: Vec<PrecName>) -> Self
    {
        Self::PrecedenceCycle {
            witness: witness.into_iter().map(|name| name.0).collect(),
        }
    }
}

/// The fixity of a declared user operator, folded into an extended grammar by
/// [`Pbg::extend`].
///
/// The fixity fixes the operator form's SHAPE (an infix `E op E`, a prefix
/// `op E`, or a postfix `E op`); the associativity refinement and per-operator
/// precedence bands are a per-module operator-fixity wiring concern, not this
/// seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fixity
{
    /// A left-associative infix operator `E op E` (at `expression.add`).
    Infixl,
    /// A right-associative infix operator `E op E` (at `expression.ret`).
    Infixr,
    /// A prefix operator `op E` (at `expression.unary`).
    Prefix,
    /// A postfix operator `E op` (at `expression.postfix`).
    Postfix,
}

impl Fixity
{
    /// The precedence-group name this fixity's form lands at.
    #[inline]
    #[must_use]
    const fn band(self) -> PrecName
    {
        match self {
            | Self::Infixl => PrecName("expression.add"),
            | Self::Infixr => PrecName("expression.ret"),
            | Self::Prefix => PrecName("expression.unary"),
            | Self::Postfix => PrecName("expression.postfix"),
        }
    }

    /// The operator form regex for a tile spelling at this fixity.
    fn regex(
        self,
        spelling: TileLabel,
    ) -> Regex
    {
        let e = || Regex::sort(Sort::Expression);
        let op = Regex::tile(spelling);
        match self {
            | Self::Infixl | Self::Infixr => Regex::seq([e(), op, e()]),
            | Self::Prefix => Regex::seq([op, e()]),
            | Self::Postfix => Regex::seq([e(), op]),
        }
    }
}

/// One declared user operator, the unit of a [`Pbg::extend`] extension.
///
/// The spelling is a `'static` tile label (the labeler-wiring that maps source
/// text to this tile is per-module and OUT of this seam); the
/// fixity fixes the form shape and precedence band.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperatorDecl
{
    /// The operator's tile spelling (its label in the extended grammar).
    pub spelling: &'static str,
    /// Operator status.
    pub fixity: Fixity,
}

impl OperatorDecl
{
    #[inline]
    #[must_use]
    pub const fn new(
        spelling: TileLabel,
        fixity: Fixity,
    ) -> Self
    {
        Self {
            spelling: spelling.0,
            fixity,
        }
    }
}

/// Checked precedence-bounded grammar.
#[derive(Clone, Debug)]
pub struct Pbg
{
    /// Built precedence DAG.
    dag: PrecDag,
    /// Forms grouped by `(Sort, Prec)`.
    forms: BTreeMap<(Sort, Prec), Regex>,
    /// Rules in input order after validation.
    rules: Vec<Rule>,
    /// Rule identities in deterministic order.
    rule_names: BTreeSet<RuleName>,
    /// Adaptation records in rule/input order.
    adaptations: Vec<Adaptation>,
    /// Authoritative mold and interned regex-context tables.
    molds: MoldTable,
}

impl Pbg
{
    /// Build and validate a precedence-bounded grammar.
    ///
    /// # Contract
    /// - requires: `dag` is a built precedence DAG and `rules` are constant
    ///   surface grammar data, not user declarations.
    /// - ensures: validates precedence membership, unique rule identities,
    ///   Operator Form, and Unique Tiles before returning grouped forms.
    /// - provides: deterministic rule, name, adaptation, and `(Sort, Prec)`
    ///   form iteration for surface and walk modules.
    /// - fails: returns [`PbgError`] for the first deterministic violation.
    /// - panics: none.
    /// - intension: rules are examined in input order; grouped forms are stored
    ///   in `BTreeMap` key order; repeated form keys append alternatives in
    ///   input order.
    ///
    /// # Errors
    /// Returns [`PbgError::InvalidPrec`], [`PbgError::DuplicateRule`],
    /// [`PbgError::AdjacentSorts`], [`PbgError::Assumption3Conflict`], or
    /// [`PbgError::DuplicateTile`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise plus L1 evidence — witnesses independently
    ///   distinguish every hard-error branch and observe deterministic grouped
    ///   alternatives for multiple rules with the same `(Sort, Prec)` key.
    /// - witness: `gandr_surface_grammar::contracts::pbg_invalid_prec_contract`
    /// - witness: `gandr_surface_grammar::contracts::pbg_duplicate_rule_contract`
    /// - witness: `gandr_surface_grammar::contracts::operator_form_nullable_contract`
    /// - witness: `gandr_surface_grammar::contracts::unique_tiles_contract`
    /// - witness: `gandr_surface_grammar::contracts::assumption_3_contract`
    #[inline]
    pub fn build(
        dag: PrecDag,
        rules: Vec<Rule>,
    ) -> Result<Self, PbgError>
    {
        validate_rule_headers(&dag, &rules)?;
        for rule in &rules {
            validate_operator_form(rule)?;
        }
        // The mold-table build is the authoritative Unique Tiles gate under
        // rctx identity; `validate_unique_tiles` is its standalone wrapper.
        let molds = MoldTable::build(&rules, GrammarFingerprint(dag.fingerprint().into()))?;
        validate_assumption_3(&rules)?;
        let forms = grouped_forms(&rules);
        let rule_names = rules.iter().map(|rule| RuleName(rule.name)).collect();
        let adaptations = rules
            .iter()
            .flat_map(|rule| rule.adaptations.iter().copied())
            .collect();
        Ok(Self {
            dag,
            forms,
            rules,
            rule_names,
            adaptations,
            molds,
        })
    }

    /// Build a grammar from a precedence table wrapper.
    ///
    /// # Errors
    /// Returns any [`PbgError`] surfaced by [`Pbg::build`] for the unwrapped
    /// DAG and rules.
    #[inline]
    pub fn build_table(
        table: PrecTable,
        rules: Vec<Rule>,
    ) -> Result<Self, PbgError>
    {
        Self::build(table.into_dag(), rules)
    }

    /// Return the built precedence DAG.
    #[inline]
    #[must_use]
    pub const fn dag(&self) -> &PrecDag
    {
        &self.dag
    }

    /// Return grouped forms keyed by `(Sort, Prec)`.
    #[inline]
    #[must_use]
    pub const fn forms(&self) -> &BTreeMap<(Sort, Prec), Regex>
    {
        &self.forms
    }

    /// Iterate grouped forms in deterministic key order.
    #[inline]
    pub fn iter_forms(&self) -> impl Iterator<Item = (&(Sort, Prec), &Regex)>
    {
        self.forms.iter()
    }

    /// Return validated rules in input order.
    #[inline]
    #[must_use]
    pub fn rules(&self) -> &[Rule]
    {
        &self.rules
    }

    /// Return rule identities in deterministic order.
    #[inline]
    #[must_use]
    pub const fn rule_names(&self) -> &BTreeSet<RuleName>
    {
        &self.rule_names
    }

    /// Return adaptation records in rule/input order.
    #[inline]
    #[must_use]
    pub fn adaptations(&self) -> &[Adaptation]
    {
        &self.adaptations
    }

    /// Return the mold definition for `id`.
    ///
    /// # Contract
    /// - requires: `id` was assigned by this grammar's mold table.
    /// - ensures: returns the exact [`MoldDef`] naming the mold's zipper
    ///   identity.
    /// - provides: the authoritative mold lookup (tylr `Mold`).
    /// - fails: returns [`PbgError::UnknownMold`] when `id` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownMold`] when `id` is outside the mold table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a valid id and the first out-of-range id kill the
    ///   only branch.
    /// - witness: `gandr_surface_grammar::contracts::mold_lookup_checks_bounds`
    #[inline]
    pub fn mold(
        &self,
        id: MoldId,
    ) -> Result<&MoldDef, PbgError>
    {
        self.molds.mold(id)
    }

    /// Return the precomputed precedence bounds for `id`.
    ///
    /// # Contract
    /// - requires: `id` was assigned by this grammar's mold table.
    /// - ensures: returns the `(left, right)` bounds precomputed from the
    ///   zipper context's sort-facing nullability (tylr `Mold.bounds`).
    /// - provides: the runtime-free precedence bound surface.
    /// - fails: returns [`PbgError::UnknownMold`] when `id` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownMold`] when `id` is outside the mold table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — sort-facing and tile-facing occurrences distinguish
    ///   `Value(prec)` from `Root` per side.
    /// - witness: `gandr_surface_grammar::contracts::mold_bounds_follow_context_nullability`
    #[inline]
    pub fn bounds(
        &self,
        id: MoldId,
    ) -> Result<(Bound<Prec>, Bound<Prec>), PbgError>
    {
        self.molds.bounds(id)
    }

    /// Return the precomputed zipper steps for `rctx` in direction `dir`.
    ///
    /// # Contract
    /// - requires: `rctx` was interned by this grammar's context table.
    /// - ensures: returns the crossed-symbol steps for that side (tylr
    ///   `RZipper.step`).
    /// - provides: the runtime-free zipper stepping surface.
    /// - fails: returns [`PbgError::UnknownRCtx`] when `rctx` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownRCtx`] when `rctx` is outside the context
    /// table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — left and right steps over an infix context kill the
    ///   direction branch.
    /// - witness: `gandr_surface_grammar::contracts::rctx_steps_cross_adjacent_symbols`
    #[inline]
    pub fn step(
        &self,
        rctx: RCtxId,
        dir: Dir,
    ) -> Result<&[RCtxStep], PbgError>
    {
        self.molds.step(rctx, dir)
    }

    /// Return the molder's per-label candidate menu.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the mold ids that `label` can take, sorted by id;
    ///   empty when the label declares no tile.
    /// - provides: the per-label candidate menu (tylr `Molds`).
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a single-candidate label and a multi-candidate label
    ///   distinguish the menu contents.
    /// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
    #[inline]
    #[must_use]
    pub fn candidates(
        &self,
        label: TileLabel,
    ) -> &[MoldId]
    {
        self.molds.candidates(label)
    }

    /// Return the fresh-slot candidate menu for `label` — the subset of
    /// [`candidates`](Self::candidates) admissible with no form open (operands,
    /// operators, form-starts, and form-first tiles), so the molder skips the
    /// wide form-mid tail at a fresh operand position.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns a sorted-by-[`MoldId`] subset of `candidates(label)`;
    ///   every dropped mold has a `≐`-predecessor and is not form-first, so it
    ///   is inadmissible with no open frontier.
    /// - provides: the wide-menu-gather fast path (proposal §5.2).
    /// - fails: never; an unknown label yields an empty slice.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a bare `identifier` (wide menu, few fresh) and a
    ///   punctuation opener distinguish the fresh subset from the full menu.
    /// - witness: `gandr_surface_parser::acceptance::corpus_parses_totally`
    #[inline]
    #[must_use]
    pub fn fresh_candidates(
        &self,
        label: TileLabel,
    ) -> &[MoldId]
    {
        self.molds.fresh_candidates(label)
    }

    /// Return every candidate label with its declared candidate count.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns `(label, count)` pairs sorted by label, one per label
    ///   that declares at least one tile occurrence.
    /// - provides: the declared mold-candidate inventory; the reachable
    ///   multi-mold metric belongs to the generative walk front-end.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — a deterministic projection over the mold table;
    ///   exactness is witnessed by the pinned inventory.
    /// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
    #[inline]
    #[must_use]
    pub fn candidate_counts(&self) -> Vec<(TileLabel, CandidateCount)>
    {
        self.molds.candidate_counts()
    }

    /// Return the number of declared mold definitions.
    #[inline]
    #[must_use]
    pub fn mold_count(&self) -> MoldCount
    {
        self.molds.len()
    }

    /// Iterate every mold definition with its id, in canonical order.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: yields one `(MoldId, &MoldDef)` per tile occurrence in
    ///   canonical table order.
    /// - provides: the generative walk front-end's mold enumeration seed.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — a deterministic projection; contents are witnessed by
    ///   the walk projection test.
    /// - witness: `gandr_surface_grammar::contracts::walk_index_projects_every_mold_once`
    #[inline]
    pub fn iter_molds(&self) -> impl Iterator<Item = (MoldId, &MoldDef)>
    {
        self.molds.iter()
    }

    /// Return the consecutive same-form tile pairs (the `≐` adjacency).
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns sorted, unique `(left, right)` [`MoldId`] pairs
    ///   naming tile occurrences that are consecutive within one form
    ///   occurrence, skipping recursive-sort holes between them (tylr's
    ///   same-form `≐`).
    /// - provides: the seed for the generative walk front-end's equality rows;
    ///   `( ≐ )` and `if ≐ then ≐ else` are the model witnesses.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a bracket pair and a bare infix operator distinguish
    ///   forms that do and do not contribute a same-form adjacency.
    /// - witness: `gandr_surface_grammar::contracts::same_form_adjacency_is_the_eq_relation`
    #[inline]
    #[must_use]
    pub fn adjacencies(&self) -> &[(MoldId, MoldId)]
    {
        self.molds.adjacencies()
    }

    /// Return the molds that can be a form's first tile (its regex FIRST set,
    /// recursive-sort holes skipped), sorted and unique.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the sorted, de-duplicated form-first mold ids.
    /// - provides: the melder's form-start test — a mold that opens a form even
    ///   behind a nullable prefix, distinguishing a legitimate opener from a
    ///   genuine mid whose predecessor is required.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a form-first tile behind an optional prefix and a
    ///   required mid tile distinguish the set.
    /// - witness: `gandr_surface_parser::meld::tests::admits_a_form_first_mid_at_a_fresh_slot`
    #[inline]
    #[must_use]
    pub fn form_first(&self) -> &[MoldId]
    {
        self.molds.form_first()
    }

    /// Return the molds that can be a form's **last** tile (its regex LAST set,
    /// recursive-sort holes skipped), sorted and unique — the dual of
    /// [`form_first`](Self::form_first).
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the sorted, de-duplicated form-last mold ids.
    /// - provides: the melder's form-completable test — a form-start / mid
    ///   whose remaining tail is nullable (`?` before an optional `hole_name`)
    ///   is already a complete form at that tile, so the melder closes it
    ///   cleanly instead of force-closing with a ghost end and a spurious
    ///   missing-tile obligation.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a completable form-start (`?`) and a form-mid that
    ///   requires a closer (`{`) distinguish the set.
    /// - witness: `gandr_surface_parser::meld::tests::completable_hole_closes_without_obligation`
    #[inline]
    #[must_use]
    pub fn form_last(&self) -> &[MoldId]
    {
        self.molds.form_last()
    }

    /// Return the fingerprint of this grammar's mold and context tables.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the deterministic FNV-framed fold of the precedence
    ///   DAG fingerprint and the mold/context tables.
    /// - provides: the fingerprint a [`gandr_surface_syntax::Cst`] records so
    ///   its [`MoldId`]s never migrate silently across grammar revisions.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the built-in fingerprint is stable across processes
    ///   while a changed surface form shifts it.
    /// - witness: `gandr_surface_grammar::contracts::pbg_fingerprint_is_stable_and_folds_precdag`
    #[inline]
    #[must_use]
    pub const fn fingerprint(&self) -> GrammarFingerprint
    {
        self.molds.fingerprint()
    }

    /// Extend `base` with declared user operators, returning a fresh checked
    /// grammar; `base` is left untouched.
    ///
    /// The operator rules are **appended** after the base rules over a clone of
    /// the base precedence DAG, so mold identity is stable across the
    /// extension: mold ids are assigned in rule input order, and appending
    /// preserves every base rule's prefix, so a [`gandr_surface_syntax::Cst`]
    /// molded against `base` keeps its [`MoldId`]s — the new operator molds
    /// take fresh higher ids. The extended grammar's fingerprint
    /// necessarily differs (it folds the added molds), while the base
    /// tables are unchanged because `extend` clones rather than mutates.
    /// This is the **unwired** seam: building the extended grammar and
    /// molding a declared operator work here, but wiring the labeler to
    /// emit the operator's tile from source text is per-module and owned by
    /// the operator-fixity wiring seam.
    ///
    /// # Contract
    /// - requires: `base` is a checked PBG whose precedence DAG names the bands
    ///   the requested fixities land at (the built-in `expression.*` groups);
    ///   each `spelling` is a fresh tile label unique among the extension.
    /// - ensures: returns a checked PBG whose rules are `base`'s followed by
    ///   one operator rule per declaration, so every base `MoldId` is preserved
    ///   and `candidates(spelling)` names the new operator mold.
    /// - provides: the operator-extension seam; `base` is never mutated.
    /// - fails: returns [`PbgError`] for a missing band, a duplicate rule name
    ///   (two identical declarations), or any checked-construction violation.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::MissingPrec`] when a fixity's band is absent from
    /// `base`, or any [`PbgError`] from checked re-construction.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — an extension with a fresh operator distinguishes a
    ///   changed fingerprint, an added candidate, and preserved base molds from
    ///   the untouched base.
    /// - witness: `gandr_surface_grammar::contracts::extend_folds_a_declared_operator`
    /// - witness: `gandr_surface_grammar::contracts::extend_preserves_base_mold_ids`
    #[inline]
    pub fn extend(
        base: &Self,
        decls: &[OperatorDecl],
    ) -> Result<Self, PbgError>
    {
        let mut rules = base.rules.clone();
        for decl in decls {
            let spelling = TileLabel(decl.spelling);
            let prec = base.named_prec(decl.fixity.band())?;
            rules.push(Rule::with_provenance(
                RuleName(decl.spelling),
                Provenance("operator_declaration"),
                Sort::Expression,
                prec,
                decl.fixity.regex(spelling),
            ));
        }
        Self::build(base.dag.clone(), rules)
    }

    /// Look up a named precedence group in this grammar's DAG.
    fn named_prec(
        &self,
        name: PrecName,
    ) -> Result<Prec, PbgError>
    {
        self.dag
            .groups()
            .find_map(|(prec, group, _assoc)| (group == name.as_ref()).then_some(prec))
            .ok_or(PbgError::MissingPrec { name: name.0 })
    }
}

/// Validate rule headers before regex checks.
fn validate_rule_headers(
    dag: &PrecDag,
    rules: &[Rule],
) -> Result<(), PbgError>
{
    let mut names = BTreeSet::new();
    for rule in rules {
        if dag.name(rule.prec).is_none() {
            return Err(PbgError::InvalidPrec {
                rule: rule.name,
                prec: rule.prec,
            });
        }
        if !names.insert(rule.name) {
            return Err(PbgError::DuplicateRule { name: rule.name });
        }
    }
    Ok(())
}

/// Group alternatives by `(Sort, Prec)`.
fn grouped_forms(rules: &[Rule]) -> BTreeMap<(Sort, Prec), Regex>
{
    let mut grouped: BTreeMap<(Sort, Prec), Vec<Regex>> = BTreeMap::new();
    for rule in rules {
        grouped
            .entry((rule.sort, rule.prec))
            .or_default()
            .extend(rule.regex.alternatives());
    }
    grouped
        .into_iter()
        .map(|(key, alternatives)| (key, Regex::Alt(alternatives)))
        .collect()
}
