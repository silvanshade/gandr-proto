//! The **erased static dependent core**: the type-level calculus whose normal
//! forms are ground types.
//!
//! # What this is, and what it is not
//!
//! gandr's type operators are dependent terms, not a kind grammar. A family
//! like `T : Pi (A : Type[+, 0]). Type[-, 0]` is a static Pi; a definition
//! `\ A -> -F (A × A)` is a static lambda; applying one is beta reduction.
//! There is no second syntactic layer above types and there will not be: this
//! is an implementation-phase distinction inside one dependent theory.
//!
//! **Static means erased.** Type variables, family parameters, level
//! parameters and sort parameters are declaration inputs that do not survive
//! to runtime. Nothing here becomes a [`crate::syntax::Value`] or a
//! [`crate::syntax::Comp`], and the dynamic arrow stays what it is — a
//! computation type whose body runs against a stack, never a type operator.
//!
//! # Where it meets the ground types
//!
//! A static normal form is reified into one of the two ground enums according
//! to its result classifier: a value result becomes a [`ValueType`], a
//! computation result becomes a [`CompType`]. That is the whole interface, and
//! it is why the classifier has to be decided before reification rather than
//! guessed after it.
//!
//! An application that cannot reduce — because its head is a declared family
//! rather than a lambda — is a **neutral**, and a neutral is available at
//! either ground sort. That is what lets `T A` be a computation type while
//! `Hom a b` is a value type, with one representation behind both.
//!
//! [`ValueType`]: crate::types::ValueType
//! [`CompType`]: crate::types::CompType

//! # The cutover this module is the contract for
//!
//! These declarations are deliberately **isolated**: nothing in the two ground
//! type enums names them yet, and no caller has been migrated. That is the
//! whole point — the migration and the structural semantics are one
//! indivisible change, so they land together rather than as a shape now and a
//! meaning later.
//!
//! What the cutover owes, stated here so it can be checked rather than
//! recalled:
//!
//! - `ValueType::Family` carries a [`FamilyApp`] instead of a name and a list
//!   of values, so a family argument may be a level, a sort, a type or a value
//!   index rather than a value alone.
//! - `CompType` gains the same former, so one neutral serves both ground sorts
//!   and the declared result classifier decides which enum an application lands
//!   in.
//! - **Adding that former breaks the computation census first.** `CompTypeTag`
//!   is not `non_exhaustive` and its `tag` method is an exhaustive match, so a
//!   new variant fails to compile until the tag, the `ALL` constant and the
//!   downstream formation table all account for it. That breakage is the census
//!   working, and the cutover is expected to repair it in the same change
//!   rather than route around it.
//! - The persisted checkpoint payload changes shape when a family's arguments
//!   do, so the format identity moves with it in the same commit, with a
//!   round-trip witness for the new payload and an explicit outcome for a
//!   record written under the old identity.
//! - Substitution, free variables, the flat arena and its round trip,
//!   structural equality and interning all extend to the static calculus, and a
//!   static normal form reifies into whichever ground enum its result
//!   classifier names.
//!
//! Nothing in this module carries a `todo!` body or a scaffold-only refusal.
//! A placeholder that fails the lint wall makes every gate in a dependent
//! crate unrunnable, which is what stalled the previous rung.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::Hash;
use core::hash::Hasher;

use gandr_kernel_strata::Level;

use crate::boundary::BinderName;
use crate::boundary::FamilyArity;
use crate::boundary::NameRef;
use crate::boundary::StaticTermCount;
use crate::boundary::StaticTermIndex;
use crate::boundary::TypeFamilyName;
use crate::classifier::Classifier;
use crate::classifier::GroundSort;
use crate::classifier::SortExpr;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::Ty;
use crate::types::ValueType;

/// A static variable: a type-level binding introduced by a telescope or a
/// static lambda.
///
/// Erased, so this is never a runtime name. Identity is the binder it was
/// introduced by, which is what keeps alpha-equivalent operators equal.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StaticVar
{
    /// The variable's declared name.
    name: String,
}

/// A static binder: the name and the classifier a static Pi or lambda binds
/// at.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StaticBinder
{
    /// The bound variable.
    variable: StaticVar,
    /// The classifier the variable ranges over.
    classifier: Classifier,
}

/// An argument a static application may carry.
///
/// The four cases are the four things a declaration may abstract over, and the
/// set is closed deliberately: widening it is a change to what a family may be
/// indexed by, which is a programme decision rather than an implementation
/// one.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StaticArg
{
    /// A universe level.
    Level(Level),
    /// A sort, ground or abstract.
    Sort(SortExpr),
    /// A type, given as a static term.
    Type(Rc<StaticTerm>),
    /// A value index, for a family indexed by values rather than types.
    Value(Rc<Value>),
}

/// A static application that cannot reduce, because its head is a declared
/// family rather than a lambda.
///
/// Neutrals are what make a named family a first-class type expression without
/// restricting higher-order abstraction: `T` applied to an argument is a
/// neutral until `T`'s definition unfolds, and it classifies at whichever
/// ground sort its declared result names.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StaticNeutral
{
    /// A bare variable head.
    Head(StaticVar),
    /// A neutral applied to one further argument.
    App
    {
        /// The neutral being applied.
        head: Rc<Self>,
        /// The argument.
        argument: StaticArg,
    },
}

/// A term of the erased static dependent core.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StaticTerm
{
    /// A static variable.
    Var(StaticVar),
    /// A universe, as a static term: what a telescope binds a type parameter
    /// at.
    Universe(Classifier),
    /// A ground type quoted into the static layer.
    ///
    /// This is the leaf that makes reification total in one direction: every
    /// ground type is a static term, so a telescope may mention one without a
    /// coercion.
    Quote(Rc<crate::types::Ty>),
    /// A dependent function type over static terms.
    Pi
    {
        /// The bound variable and its classifier.
        binder: StaticBinder,
        /// The codomain, which may mention the binder.
        codomain: Rc<Self>,
    },
    /// A static lambda: a type operator's definition.
    Lam
    {
        /// The bound variable and its classifier.
        binder: StaticBinder,
        /// The body, which may mention the binder.
        body: Rc<Self>,
    },
    /// A static application.
    App
    {
        /// The operator.
        function: Rc<Self>,
        /// The argument.
        argument: StaticArg,
    },
    /// A stuck application.
    Neutral(StaticNeutral),
}

/// A family application carried by a ground type: a neutral with the
/// classifier its declaration gives its result.
///
/// This is the same neutral in both ground enums. The classifier is what
/// decides which enum a given application lives in, and storing it here is
/// what lets a reader of either enum answer that without re-deriving it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FamilyApp
{
    /// The applied family and its arguments.
    neutral: StaticNeutral,
    /// The classifier the declaration gives the result.
    result: Classifier,
}

/// The parameters a declaration's telescope binds, in declaration order.
///
/// A telescope is surface sugar over nested static Pi; this is the checked
/// interface that elaborates into one.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StaticParam
{
    /// A level parameter.
    Level
    {
        /// The parameter's name.
        name: String,
    },
    /// A sort parameter.
    Sort
    {
        /// The parameter's name.
        name: String,
    },
    /// A type parameter, bound at a classifier.
    Type
    {
        /// The parameter's name.
        name: String,
        /// The classifier it ranges over.
        classifier: Classifier,
    },
    /// A value parameter, for a family indexed by values.
    Value
    {
        /// The parameter's name.
        name: String,
        /// The value type it ranges over.
        ty: Rc<crate::types::ValueType>,
    },
}

/// A type family's declared signature: its telescope and its result
/// classifier.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypeFamilySig
{
    /// The bound parameters, in declaration order.
    params: Vec<StaticParam>,
    /// The classifier the declaration gives the result.
    result: Classifier,
}

impl StaticVar
{
    /// Creates an erased static variable with the given binder name.
    ///
    /// # Contract
    /// - ensures: preserves `name` exactly.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<N>(name: N) -> Self
    where
        N: Into<String>,
    {
        Self { name: name.into() }
    }

    /// Returns the variable's binder name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> BinderName<'_>
    {
        BinderName::from(self.name.as_str())
    }
}

impl StaticBinder
{
    /// Creates a static binder.
    ///
    /// # Contract
    /// - ensures: stores `variable` and `classifier` without alteration.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        variable: StaticVar,
        classifier: Classifier,
    ) -> Self
    {
        Self {
            variable,
            classifier,
        }
    }

    /// Returns the variable bound by this binder.
    #[inline]
    #[must_use]
    pub const fn variable(&self) -> &StaticVar
    {
        &self.variable
    }

    /// Returns the classifier at which this binder ranges.
    #[inline]
    #[must_use]
    pub const fn classifier(&self) -> &Classifier
    {
        &self.classifier
    }
}

impl StaticNeutral
{
    /// Creates a neutral with a variable head.
    ///
    /// # Contract
    /// - ensures: creates the bare neutral `variable`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn head(variable: StaticVar) -> Self
    {
        Self::Head(variable)
    }

    /// Extends a neutral by one static argument.
    ///
    /// # Contract
    /// - ensures: preserves the existing spine and appends `argument`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn app(
        head: Self,
        argument: StaticArg,
    ) -> Self
    {
        Self::App {
            head: Rc::new(head),
            argument,
        }
    }

    /// Returns the variable at the head of this neutral spine.
    ///
    /// # Contract
    /// - ensures: returns the declaration head name, regardless of spine
    ///   length.
    /// - panics: none.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "The worklist follows borrowed neutral nodes without cloning their spine."
    )]
    #[inline]
    #[must_use]
    pub fn head_variable(&self) -> &StaticVar
    {
        let mut current = self;
        loop {
            match current {
                | Self::Head(variable) => return variable,
                | Self::App { head, .. } => current = head,
            }
        }
    }

    /// Returns the declaration head name of this neutral.
    #[inline]
    #[must_use]
    pub fn head_name(&self) -> TypeFamilyName<'_>
    {
        let variable = self.head_variable();
        TypeFamilyName::from(&variable.name)
    }

    /// Returns the neutral's application spine in source order.
    ///
    /// # Contract
    /// - ensures: returns every argument exactly once, from head to tail.
    /// - panics: none.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "The worklist follows borrowed neutral nodes without cloning their spine."
    )]
    #[inline]
    #[must_use]
    pub fn arguments(&self) -> Vec<StaticArg>
    {
        let mut reversed = Vec::new();
        let mut current = self;
        loop {
            match current {
                | Self::Head(_) => {
                    reversed.reverse();
                    return reversed;
                },
                | Self::App { head, argument } => {
                    reversed.push(argument.clone());
                    current = head;
                },
            }
        }
    }
}

impl FamilyApp
{
    /// Creates a classifier-bearing family application.
    ///
    /// # Contract
    /// - ensures: stores `neutral` and `result` exactly.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        neutral: StaticNeutral,
        result: Classifier,
    ) -> Self
    {
        Self { neutral, result }
    }

    /// Returns the neutral family spine.
    #[inline]
    #[must_use]
    pub const fn neutral(&self) -> &StaticNeutral
    {
        &self.neutral
    }

    /// Returns the declaration's result classifier.
    #[inline]
    #[must_use]
    pub const fn result(&self) -> &Classifier
    {
        &self.result
    }

    /// Returns a copy with every runtime value argument substituted.
    ///
    /// Static type, level, and sort arguments are already erased from runtime
    /// value substitution and therefore remain structurally unchanged.
    ///
    /// # Contract
    /// - ensures: substitutes only [`StaticArg::Value`] entries in the neutral
    ///   spine and preserves its head and result classifier.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn substitute_value(
        &self,
        name: NameRef<'_>,
        replacement: &Value,
    ) -> Self
    {
        let arguments =
            self.neutral
                .arguments()
                .into_iter()
                .map(|argument| match argument {
                    | StaticArg::Value(value) => StaticArg::Value(Rc::new(
                        crate::subst::subst_value(value.as_ref(), name, replacement),
                    )),
                    | other => other,
                });
        let neutral = arguments.fold(
            StaticNeutral::head(self.neutral.head_variable().clone()),
            StaticNeutral::app,
        );
        Self::new(neutral, self.result.clone())
    }
}

impl TypeFamilySig
{
    /// Creates a type-family signature.
    ///
    /// # Contract
    /// - ensures: preserves parameter order and the declared result classifier.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        params: Vec<StaticParam>,
        result: Classifier,
    ) -> Self
    {
        Self { params, result }
    }

    /// Returns the declaration parameters in source order.
    #[inline]
    #[must_use]
    pub fn params(&self) -> &[StaticParam]
    {
        &self.params
    }

    /// Returns the classifier declared for the family result.
    #[inline]
    #[must_use]
    pub const fn result(&self) -> &Classifier
    {
        &self.result
    }

    /// Returns the declared family arity.
    #[inline]
    #[must_use]
    pub fn arity(&self) -> FamilyArity
    {
        FamilyArity::from(self.params.len())
    }
}

/// The two ground carriers produced by static reification.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ReifiedType
{
    /// A value type.
    Value(ValueType),
    /// A computation type.
    Computation(CompType),
}

/// Why a static normal form could not be reified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReifyError
{
    /// The classifier is abstract and has no ground carrier.
    AbstractClassifier(Classifier),
    /// The static form is not a ground type former.
    UnsupportedForm,
    /// A quoted type has the opposite carrier from the requested result.
    WrongQuotedSort,
}

/// A stable content hash for a static term.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct StaticHash(u64);

/// A small deterministic FNV-1a hasher for static interning.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct StaticHasher(u64);

impl Default for StaticHasher
{
    #[inline]
    fn default() -> Self
    {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for StaticHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        self.0
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        for byte in bytes {
            self.0 = self
                .0
                .wrapping_mul(0x0100_0000_01b3)
                .wrapping_add(u64::from(*byte));
        }
    }
}

/// A content-addressed id for a static term.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct StaticTermId(u32);

impl StaticTermId
{
    /// Returns the numeric id for diagnostics and bounded storage.
    #[inline]
    #[must_use]
    pub fn index(self) -> StaticTermIndex
    {
        StaticTermIndex::from(self.0)
    }
}

/// A structural interner for erased static terms.
#[derive(Clone, Debug, Default)]
pub struct StaticInterner
{
    /// Interned static terms, retained for id resolution.
    terms: Vec<Rc<StaticTerm>>,
    /// Hash buckets used to find structurally equal interned terms.
    buckets: BTreeMap<StaticHash, Vec<StaticTermId>>,
}

impl StaticInterner
{
    /// Interns a static term by structural content.
    ///
    /// # Contract
    /// - ensures: structurally equal terms return one id, including terms with
    ///   distinct `Rc` allocations.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn intern(
        &mut self,
        term: Rc<StaticTerm>,
    ) -> StaticTermId
    {
        let mut hasher = StaticHasher::default();
        term.hash(&mut hasher);
        let hash = StaticHash(hasher.finish());
        if let Some(ids) = self.buckets.get(&hash) {
            for &id in ids {
                if self.terms.get(usize::try_from(id.0).unwrap_or(usize::MAX)) == Some(&term) {
                    return id;
                }
            }
        }
        let id = StaticTermId(u32::try_from(self.terms.len()).unwrap_or(u32::MAX));
        self.terms.push(term);
        self.buckets.entry(hash).or_default().push(id);
        id
    }

    /// Resolves an id minted by this interner.
    #[inline]
    #[must_use]
    pub fn resolve(
        &self,
        id: StaticTermId,
    ) -> Option<&Rc<StaticTerm>>
    {
        self.terms.get(usize::try_from(id.0).ok()?)
    }

    /// Returns the number of distinct static terms.
    #[inline]
    #[must_use]
    pub fn len(&self) -> StaticTermCount
    {
        StaticTermCount::from(self.terms.len())
    }
}

/// Performs capture-avoiding substitution of one static variable.
///
/// # Contract
/// - ensures: replaces free occurrences of `variable`, alpha-renaming a
///   conflicting binder before descending.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — beta substitution, binder shadowing, and capture cases
///   are separately witnessed by the static calculus tests, each paired with
///   the positive control that separates it from the neighbouring case: a
///   non-target variable, a binder on another name, and a replacement with no
///   free occurrence of the binder.
/// - witness: `static_term::tests::substitution_replaces_a_free_occurrence`
/// - witness: `static_term::tests::substitution_stops_at_a_shadowing_binder`
/// - witness: `static_term::tests::substitution_avoids_capture`
#[inline]
#[must_use]
pub fn substitute(
    term: &StaticTerm,
    variable: &StaticVar,
    replacement: &StaticTerm,
) -> StaticTerm
{
    transform_term(term, variable, replacement)
}

/// Computes the free static variables of a term.
///
/// # Contract
/// - ensures: returns exactly variables not enclosed by a same-named static
///   binder.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The free-variable worklist matches borrowed nodes by reference."
)]
#[inline]
#[must_use]
pub fn free_variables(term: &StaticTerm) -> BTreeSet<StaticVar>
{
    let mut variables = BTreeSet::new();
    let mut bound = BTreeSet::new();
    let mut tasks = alloc::vec![FreeTask::Term(Rc::new(term.clone()))];
    while let Some(task) = tasks.pop() {
        match task {
            | FreeTask::Term(term) => match term.as_ref() {
                | StaticTerm::Var(variable) => {
                    if !bound.contains(variable) {
                        variables.insert(variable.clone());
                    }
                },
                | StaticTerm::Universe(_) | StaticTerm::Quote(_) => {},
                | StaticTerm::Pi { binder, codomain }
                | StaticTerm::Lam {
                    binder,
                    body: codomain,
                } => {
                    let inserted = bound.insert(binder.variable.clone());
                    tasks.push(FreeTask::ExitBound {
                        variable: binder.variable.clone(),
                        inserted,
                    });
                    tasks.push(FreeTask::Term(Rc::clone(codomain)));
                },
                | StaticTerm::App { function, argument } => {
                    tasks.push(FreeTask::Arg(argument.clone()));
                    tasks.push(FreeTask::Term(Rc::clone(function)));
                },
                | StaticTerm::Neutral(neutral) => {
                    tasks.push(FreeTask::Neutral(Rc::new(neutral.clone())));
                },
            },
            | FreeTask::Neutral(neutral) => match neutral.as_ref() {
                | StaticNeutral::Head(variable) => {
                    if !bound.contains(variable) {
                        variables.insert(variable.clone());
                    }
                },
                | StaticNeutral::App { head, argument } => {
                    tasks.push(FreeTask::Arg(argument.clone()));
                    tasks.push(FreeTask::Neutral(Rc::clone(head)));
                },
            },
            | FreeTask::Arg(StaticArg::Type(term)) => {
                tasks.push(FreeTask::Term(term));
            },
            | FreeTask::Arg(StaticArg::Level(_) | StaticArg::Sort(_) | StaticArg::Value(_)) => {},
            | FreeTask::ExitBound { variable, inserted } => {
                if inserted {
                    bound.remove(&variable);
                }
            },
        }
    }
    variables
}

/// Normalizes static beta-redexes with an explicit heap worklist.
///
/// # Contract
/// - ensures: reduces every static lambda application and normalizes all
///   surviving children.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — beta reduction and neutral-spine preservation are
///   distinguished by one redex and one stuck application, each asserted as an
///   exact term, with the redex leg carrying a nested and an under-binder case
///   for the surviving-children clause.
/// - witness: `static_term::tests::normalizes_beta_redex`
/// - witness: `static_term::tests::normalization_preserves_a_stuck_application`
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The normalization worklist matches borrowed nodes by reference."
)]
#[inline]
#[must_use]
pub fn normalize(term: &StaticTerm) -> StaticTerm
{
    let mut tasks = alloc::vec![NormTask::Term(Rc::new(term.clone()))];
    let mut results = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | NormTask::Term(term) => match term.as_ref() {
                | StaticTerm::Var(_) | StaticTerm::Universe(_) | StaticTerm::Quote(_) => {
                    results.push(NormResult::Term((*term).clone()));
                },
                | StaticTerm::Neutral(neutral) => {
                    tasks.push(NormTask::FinishNeutralTerm);
                    tasks.push(NormTask::Neutral(Rc::new(neutral.clone())));
                },
                | StaticTerm::Pi { binder, codomain } => {
                    tasks.push(NormTask::FinishPi(binder.clone()));
                    tasks.push(NormTask::Term(Rc::clone(codomain)));
                },
                | StaticTerm::Lam { binder, body } => {
                    tasks.push(NormTask::FinishLam(binder.clone()));
                    tasks.push(NormTask::Term(Rc::clone(body)));
                },
                | StaticTerm::App { function, argument } => {
                    tasks.push(NormTask::FinishApp);
                    tasks.push(NormTask::Arg(argument.clone()));
                    tasks.push(NormTask::Term(Rc::clone(function)));
                },
            },
            | NormTask::Neutral(neutral) => match neutral.as_ref() {
                | StaticNeutral::Head(_) => {
                    results.push(NormResult::Neutral(neutral.as_ref().clone()));
                },
                | StaticNeutral::App { head, argument } => {
                    tasks.push(NormTask::FinishNeutral);
                    tasks.push(NormTask::Arg(argument.clone()));
                    tasks.push(NormTask::Neutral(Rc::clone(head)));
                },
            },
            | NormTask::Arg(argument) => match argument {
                | StaticArg::Type(term) => {
                    tasks.push(NormTask::FinishArgType);
                    tasks.push(NormTask::Term(term));
                },
                | other => results.push(NormResult::Arg(other)),
            },
            | NormTask::FinishArgType => {
                let term = pop_norm_term(&mut results);
                results.push(NormResult::Arg(StaticArg::Type(Rc::new(term))));
            },
            | NormTask::FinishNeutral => {
                let argument = pop_norm_arg(&mut results);
                let head = pop_norm_neutral(&mut results);
                results.push(NormResult::Neutral(StaticNeutral::app(head, argument)));
            },
            | NormTask::FinishNeutralTerm => {
                let neutral = pop_norm_neutral(&mut results);
                results.push(NormResult::Term(StaticTerm::Neutral(neutral)));
            },
            | NormTask::FinishPi(binder) => {
                let codomain = pop_norm_term(&mut results);
                results.push(NormResult::Term(StaticTerm::Pi {
                    binder,
                    codomain: Rc::new(codomain),
                }));
            },
            | NormTask::FinishLam(binder) => {
                let body = pop_norm_term(&mut results);
                results.push(NormResult::Term(StaticTerm::Lam {
                    binder,
                    body: Rc::new(body),
                }));
            },
            | NormTask::FinishApp => {
                let normalized_argument = pop_norm_arg(&mut results);
                let function = pop_norm_term(&mut results);
                if let StaticTerm::Lam { binder, body } = function {
                    let replacement = match normalized_argument {
                        | StaticArg::Type(term) => (*term).clone(),
                        | StaticArg::Level(level) => StaticTerm::Quote(Rc::new(Ty::Value(
                            ValueType::universe(SortExpr::value(), level),
                        ))),
                        | StaticArg::Sort(sort) => StaticTerm::Universe(Classifier::new(
                            sort,
                            gandr_kernel_strata::Level::zero(),
                        )),
                        | StaticArg::Value(value) => {
                            StaticTerm::Quote(Rc::new(Ty::Value(ValueType::Path {
                                ty: Rc::new(ValueType::Unit),
                                lhs: Rc::clone(&value),
                                rhs: value,
                            })))
                        },
                    };
                    tasks.push(NormTask::Term(Rc::new(substitute(
                        body.as_ref(),
                        binder.variable(),
                        &replacement,
                    ))));
                }
                else {
                    results.push(NormResult::Term(StaticTerm::App {
                        function: Rc::new(function),
                        argument: normalized_argument,
                    }));
                }
            },
        }
    }
    pop_norm_term(&mut results)
}

/// Reifies a static normal form according to its result classifier.
///
/// # Contract
/// - ensures: selects `ValueType` for a value classifier and `CompType` for a
///   computation classifier.
/// - fails: abstract classifiers and non-ground static forms return a named
///   error.
/// - panics: none.
/// # Errors
/// Returns [`ReifyError`] when the classifier is abstract, the form is not
/// a ground type, or the quoted type has the opposite sort.
#[inline]
#[must_use = "the reified result must be handled"]
pub fn reify(
    term: &StaticTerm,
    classifier: &Classifier,
) -> Result<ReifiedType, ReifyError>
{
    let normal = normalize(term);
    let Some(sort) = classifier.ground_sort()
    else {
        return Err(ReifyError::AbstractClassifier(classifier.clone()));
    };
    match sort {
        | GroundSort::Value => reify_value(&normal, classifier).map(ReifiedType::Value),
        | GroundSort::Computation => reify_comp(&normal, classifier).map(ReifiedType::Computation),
    }
}

#[expect(
    clippy::pattern_type_mismatch,
    reason = "Reification matches borrowed static nodes without taking ownership."
)]
/// Reifies the value carrier of one normal form.
fn reify_value(
    term: &StaticTerm,
    classifier: &Classifier,
) -> Result<ValueType, ReifyError>
{
    match term {
        | StaticTerm::Quote(quoted) => match quoted.as_ref() {
            | Ty::Value(value) => Ok(value.clone()),
            | Ty::Comp(_) => Err(ReifyError::WrongQuotedSort),
        },
        | StaticTerm::Universe(universe) => Ok(ValueType::universe(
            universe.sort().clone(),
            universe.level().clone(),
        )),
        | StaticTerm::Var(variable) => Ok(ValueType::family(FamilyApp::new(
            StaticNeutral::head(variable.clone()),
            classifier.clone(),
        ))),
        | StaticTerm::Neutral(neutral) => Ok(ValueType::family(FamilyApp::new(
            neutral.clone(),
            classifier.clone(),
        ))),
        | StaticTerm::App { .. } | StaticTerm::Pi { .. } | StaticTerm::Lam { .. } => {
            Err(ReifyError::UnsupportedForm)
        },
    }
}

/// Reifies the computation carrier of one normal form.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Reification matches borrowed static nodes without taking ownership."
)]
fn reify_comp(
    term: &StaticTerm,
    classifier: &Classifier,
) -> Result<CompType, ReifyError>
{
    match term {
        | StaticTerm::Quote(quoted) => match quoted.as_ref() {
            | Ty::Comp(comp) => Ok(comp.clone()),
            | Ty::Value(_) => Err(ReifyError::WrongQuotedSort),
        },
        | StaticTerm::Var(variable) => Ok(CompType::family(FamilyApp::new(
            StaticNeutral::head(variable.clone()),
            classifier.clone(),
        ))),
        | StaticTerm::Neutral(neutral) => Ok(CompType::family(FamilyApp::new(
            neutral.clone(),
            classifier.clone(),
        ))),
        | StaticTerm::Universe(_)
        | StaticTerm::App { .. }
        | StaticTerm::Pi { .. }
        | StaticTerm::Lam { .. } => Err(ReifyError::UnsupportedForm),
    }
}

/// A pending free-variable walk operation.
enum FreeTask
{
    /// Visit a static term.
    Term(Rc<StaticTerm>),
    /// Visit a neutral spine.
    Neutral(Rc<StaticNeutral>),
    /// Visit one static argument.
    Arg(StaticArg),
    /// Leave a binder scope.
    ExitBound
    {
        /// The binder being left.
        variable: StaticVar,
        /// Whether this walk inserted the binder.
        inserted: bool,
    },
}

/// A pending normalization operation.
enum NormTask
{
    /// Visit a static term.
    Term(Rc<StaticTerm>),
    /// Visit a neutral spine.
    Neutral(Rc<StaticNeutral>),
    /// Visit a static argument.
    Arg(StaticArg),
    /// Finish a type-level argument.
    FinishArgType,
    /// Finish a neutral application.
    FinishNeutral,
    /// Turn a neutral into a static term.
    FinishNeutralTerm,
    /// Finish a Pi.
    FinishPi(StaticBinder),
    /// Finish a lambda.
    FinishLam(StaticBinder),
    /// Finish an application.
    FinishApp,
}

/// One normalized result on the explicit result stack.
enum NormResult
{
    /// A normalized term.
    Term(StaticTerm),
    /// A normalized neutral.
    Neutral(StaticNeutral),
    /// A normalized argument.
    Arg(StaticArg),
}

/// Pops one normalized term, preserving the totality boundary.
fn pop_norm_term(results: &mut Vec<NormResult>) -> StaticTerm
{
    match results.pop() {
        | Some(NormResult::Term(term)) => term,
        | _ => StaticTerm::Neutral(StaticNeutral::head(StaticVar::new(
            "<normalization-invariant>",
        ))),
    }
}

/// Pops one normalized neutral, preserving the totality boundary.
fn pop_norm_neutral(results: &mut Vec<NormResult>) -> StaticNeutral
{
    match results.pop() {
        | Some(NormResult::Neutral(neutral)) => neutral,
        | _ => StaticNeutral::head(StaticVar::new("<normalization-invariant>")),
    }
}

/// Pops one normalized argument, preserving the totality boundary.
fn pop_norm_arg(results: &mut Vec<NormResult>) -> StaticArg
{
    match results.pop() {
        | Some(NormResult::Arg(argument)) => argument,
        | _ => StaticArg::Sort(SortExpr::value()),
    }
}

/// One substitution or alpha-renaming operation.
enum TransformTask
{
    /// Visit a term.
    Term(Rc<StaticTerm>),
    /// Visit a neutral.
    Neutral(Rc<StaticNeutral>),
    /// Visit an argument.
    Arg(StaticArg),
    /// Finish a Pi.
    FinishPi(StaticBinder),
    /// Finish a lambda.
    FinishLam(StaticBinder),
    /// Finish an application.
    FinishApp,
    /// Finish a neutral application.
    FinishNeutral,
    /// Turn a neutral into a term.
    FinishNeutralTerm,
    /// Finish a type argument.
    FinishArgType,
}

/// One transformed result on the explicit result stack.
enum TransformResult
{
    /// A transformed term.
    Term(StaticTerm),
    /// A transformed neutral.
    Neutral(StaticNeutral),
    /// A transformed argument.
    Arg(StaticArg),
}

/// Performs one capture-avoiding substitution using a heap worklist.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The substitution worklist matches borrowed nodes by reference."
)]
fn transform_term(
    term: &StaticTerm,
    variable: &StaticVar,
    replacement: &StaticTerm,
) -> StaticTerm
{
    let replacement_free = free_variables(replacement);
    let mut tasks = alloc::vec![TransformTask::Term(Rc::new(term.clone()))];
    let mut results = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | TransformTask::Term(term) => match term.as_ref() {
                | StaticTerm::Var(current) => {
                    let result = if current == variable {
                        replacement.clone()
                    }
                    else {
                        StaticTerm::Var(current.clone())
                    };
                    results.push(TransformResult::Term(result));
                },
                | StaticTerm::Universe(_) | StaticTerm::Quote(_) => {
                    results.push(TransformResult::Term(term.as_ref().clone()));
                },
                | StaticTerm::Neutral(neutral) => {
                    tasks.push(TransformTask::FinishNeutralTerm);
                    tasks.push(TransformTask::Neutral(Rc::new(neutral.clone())));
                },
                | StaticTerm::App { function, argument } => {
                    tasks.push(TransformTask::FinishApp);
                    tasks.push(TransformTask::Arg(argument.clone()));
                    tasks.push(TransformTask::Term(Rc::clone(function)));
                },
                | StaticTerm::Pi { binder, codomain } => {
                    if binder.variable() == variable {
                        results.push(TransformResult::Term(term.as_ref().clone()));
                    }
                    else if replacement_free.contains(binder.variable()) {
                        let fresh = fresh_variable(
                            binder.variable(),
                            codomain.as_ref(),
                            replacement,
                            variable,
                        );
                        let renamed = rename_bound(codomain.as_ref(), binder.variable(), &fresh);
                        tasks.push(TransformTask::FinishPi(StaticBinder::new(
                            fresh,
                            binder.classifier().clone(),
                        )));
                        tasks.push(TransformTask::Term(Rc::new(renamed)));
                    }
                    else {
                        tasks.push(TransformTask::FinishPi(binder.clone()));
                        tasks.push(TransformTask::Term(Rc::clone(codomain)));
                    }
                },
                | StaticTerm::Lam { binder, body } => {
                    if binder.variable() == variable {
                        results.push(TransformResult::Term(term.as_ref().clone()));
                    }
                    else if replacement_free.contains(binder.variable()) {
                        let fresh =
                            fresh_variable(binder.variable(), body.as_ref(), replacement, variable);
                        let renamed = rename_bound(body.as_ref(), binder.variable(), &fresh);
                        tasks.push(TransformTask::FinishLam(StaticBinder::new(
                            fresh,
                            binder.classifier().clone(),
                        )));
                        tasks.push(TransformTask::Term(Rc::new(renamed)));
                    }
                    else {
                        tasks.push(TransformTask::FinishLam(binder.clone()));
                        tasks.push(TransformTask::Term(Rc::clone(body)));
                    }
                },
            },
            | TransformTask::Neutral(neutral) => match neutral.as_ref() {
                | StaticNeutral::Head(variable) => {
                    results.push(TransformResult::Neutral(StaticNeutral::Head(
                        variable.clone(),
                    )));
                },
                | StaticNeutral::App { head, argument } => {
                    tasks.push(TransformTask::FinishNeutral);
                    tasks.push(TransformTask::Arg(argument.clone()));
                    tasks.push(TransformTask::Neutral(Rc::clone(head)));
                },
            },
            | TransformTask::Arg(argument) => match argument {
                | StaticArg::Type(term) => {
                    tasks.push(TransformTask::FinishArgType);
                    tasks.push(TransformTask::Term(term));
                },
                | other => {
                    results.push(TransformResult::Arg(other));
                },
            },
            | TransformTask::FinishPi(binder) => {
                let codomain = pop_transform_term(&mut results);
                results.push(TransformResult::Term(StaticTerm::Pi {
                    binder,
                    codomain: Rc::new(codomain),
                }));
            },
            | TransformTask::FinishLam(binder) => {
                let body = pop_transform_term(&mut results);
                results.push(TransformResult::Term(StaticTerm::Lam {
                    binder,
                    body: Rc::new(body),
                }));
            },
            | TransformTask::FinishApp => {
                let argument = pop_transform_arg(&mut results);
                let function = pop_transform_term(&mut results);
                results.push(TransformResult::Term(StaticTerm::App {
                    function: Rc::new(function),
                    argument,
                }));
            },
            | TransformTask::FinishNeutral => {
                let argument = pop_transform_arg(&mut results);
                let head = pop_transform_neutral(&mut results);
                results.push(TransformResult::Neutral(StaticNeutral::app(head, argument)));
            },
            | TransformTask::FinishNeutralTerm => {
                let neutral = pop_transform_neutral(&mut results);
                results.push(TransformResult::Term(StaticTerm::Neutral(neutral)));
            },
            | TransformTask::FinishArgType => {
                let term = pop_transform_term(&mut results);
                results.push(TransformResult::Arg(StaticArg::Type(Rc::new(term))));
            },
        }
    }
    pop_transform_term(&mut results)
}

/// Renames occurrences bound by the outer binder.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The renaming worklist matches borrowed nodes by reference."
)]
fn rename_bound(
    term: &StaticTerm,
    old: &StaticVar,
    new: &StaticVar,
) -> StaticTerm
{
    let mut tasks = alloc::vec![RenameTask::Term(Rc::new(term.clone()), false)];
    let mut results = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | RenameTask::Term(term, shadowed) => match term.as_ref() {
                | StaticTerm::Var(variable) => {
                    let variable = if !shadowed && variable == old {
                        new.clone()
                    }
                    else {
                        variable.clone()
                    };
                    results.push(RenameResult::Term(StaticTerm::Var(variable)));
                },
                | StaticTerm::Universe(_) | StaticTerm::Quote(_) => {
                    results.push(RenameResult::Term(term.as_ref().clone()));
                },
                | StaticTerm::Neutral(neutral) => {
                    tasks.push(RenameTask::FinishNeutralTerm);
                    tasks.push(RenameTask::Neutral(Rc::new(neutral.clone()), shadowed));
                },
                | StaticTerm::App { function, argument } => {
                    tasks.push(RenameTask::FinishApp);
                    tasks.push(RenameTask::Arg(argument.clone(), shadowed));
                    tasks.push(RenameTask::Term(Rc::clone(function), shadowed));
                },
                | StaticTerm::Pi { binder, codomain } => {
                    if binder.variable() == old {
                        results.push(RenameResult::Term(term.as_ref().clone()));
                    }
                    else {
                        tasks.push(RenameTask::FinishPi(binder.clone()));
                        tasks.push(RenameTask::Term(Rc::clone(codomain), shadowed));
                    }
                },
                | StaticTerm::Lam { binder, body } => {
                    if binder.variable() == old {
                        results.push(RenameResult::Term(term.as_ref().clone()));
                    }
                    else {
                        tasks.push(RenameTask::FinishLam(binder.clone()));
                        tasks.push(RenameTask::Term(Rc::clone(body), shadowed));
                    }
                },
            },
            | RenameTask::Neutral(neutral, shadowed) => match neutral.as_ref() {
                | StaticNeutral::Head(variable) => {
                    let variable = if !shadowed && variable == old {
                        new.clone()
                    }
                    else {
                        variable.clone()
                    };
                    results.push(RenameResult::Neutral(StaticNeutral::Head(variable)));
                },
                | StaticNeutral::App { head, argument } => {
                    tasks.push(RenameTask::FinishNeutral);
                    tasks.push(RenameTask::Arg(argument.clone(), shadowed));
                    tasks.push(RenameTask::Neutral(Rc::clone(head), shadowed));
                },
            },
            | RenameTask::Arg(argument, shadowed) => match argument {
                | StaticArg::Type(term) => {
                    tasks.push(RenameTask::FinishArgType);
                    tasks.push(RenameTask::Term(term, shadowed));
                },
                | other => results.push(RenameResult::Arg(other)),
            },
            | RenameTask::FinishPi(binder) => {
                let codomain = pop_rename_term(&mut results);
                results.push(RenameResult::Term(StaticTerm::Pi {
                    binder,
                    codomain: Rc::new(codomain),
                }));
            },
            | RenameTask::FinishLam(binder) => {
                let body = pop_rename_term(&mut results);
                results.push(RenameResult::Term(StaticTerm::Lam {
                    binder,
                    body: Rc::new(body),
                }));
            },
            | RenameTask::FinishApp => {
                let argument = pop_rename_arg(&mut results);
                let function = pop_rename_term(&mut results);
                results.push(RenameResult::Term(StaticTerm::App {
                    function: Rc::new(function),
                    argument,
                }));
            },
            | RenameTask::FinishNeutral => {
                let argument = pop_rename_arg(&mut results);
                let head = pop_rename_neutral(&mut results);
                results.push(RenameResult::Neutral(StaticNeutral::app(head, argument)));
            },
            | RenameTask::FinishNeutralTerm => {
                let neutral = pop_rename_neutral(&mut results);
                results.push(RenameResult::Term(StaticTerm::Neutral(neutral)));
            },
            | RenameTask::FinishArgType => {
                let term = pop_rename_term(&mut results);
                results.push(RenameResult::Arg(StaticArg::Type(Rc::new(term))));
            },
        }
    }
    pop_rename_term(&mut results)
}

/// One alpha-renaming worklist task.
enum RenameTask
{
    /// Visit a term and carry whether an outer binder shadows the target.
    Term(Rc<StaticTerm>, bool),
    /// Visit a neutral with the same shadowing state.
    Neutral(Rc<StaticNeutral>, bool),
    /// Visit an argument with the same shadowing state.
    Arg(StaticArg, bool),
    /// Finish a Pi.
    FinishPi(StaticBinder),
    /// Finish a lambda.
    FinishLam(StaticBinder),
    /// Finish an application.
    FinishApp,
    /// Finish a neutral application.
    FinishNeutral,
    /// Turn a neutral into a term.
    FinishNeutralTerm,
    /// Finish a type argument.
    FinishArgType,
}

/// One alpha-renamed result.
enum RenameResult
{
    /// A term.
    Term(StaticTerm),
    /// A neutral.
    Neutral(StaticNeutral),
    /// An argument.
    Arg(StaticArg),
}

/// Pops one renamed term.
fn pop_rename_term(results: &mut Vec<RenameResult>) -> StaticTerm
{
    match results.pop() {
        | Some(RenameResult::Term(term)) => term,
        | _ => StaticTerm::Neutral(StaticNeutral::head(StaticVar::new("<rename-invariant>"))),
    }
}

/// Pops one renamed neutral.
fn pop_rename_neutral(results: &mut Vec<RenameResult>) -> StaticNeutral
{
    match results.pop() {
        | Some(RenameResult::Neutral(neutral)) => neutral,
        | _ => StaticNeutral::head(StaticVar::new("<rename-invariant>")),
    }
}

/// Pops one renamed argument.
fn pop_rename_arg(results: &mut Vec<RenameResult>) -> StaticArg
{
    match results.pop() {
        | Some(RenameResult::Arg(argument)) => argument,
        | _ => StaticArg::Sort(SortExpr::value()),
    }
}

/// Chooses a binder name absent from the source and replacement.
fn fresh_variable(
    binder: &StaticVar,
    body: &StaticTerm,
    replacement: &StaticTerm,
    target: &StaticVar,
) -> StaticVar
{
    let mut names = all_names(body);
    names.extend(all_names(replacement));
    names.insert(target.name().as_ref().to_owned());
    let base = binder.name().as_ref().to_owned();
    let mut candidate = base.clone();
    let mut serial = 0_u32;
    while names.contains(&candidate) {
        serial = serial.saturating_add(1);
        candidate = alloc::format!("{base}${serial}");
    }
    StaticVar::new(candidate)
}

/// Collects bound and free names for fresh-name generation.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The name-collection worklist matches borrowed nodes by reference."
)]
fn all_names(term: &StaticTerm) -> BTreeSet<String>
{
    let mut names = BTreeSet::new();
    let mut tasks = alloc::vec![NameTask::Term(Rc::new(term.clone()))];
    while let Some(task) = tasks.pop() {
        match task {
            | NameTask::Term(term) => match term.as_ref() {
                | StaticTerm::Var(variable) => {
                    names.insert(variable.name().as_ref().to_owned());
                },
                | StaticTerm::Universe(_) | StaticTerm::Quote(_) => {},
                | StaticTerm::Pi { binder, codomain }
                | StaticTerm::Lam {
                    binder,
                    body: codomain,
                } => {
                    names.insert(binder.variable().name().as_ref().to_owned());
                    tasks.push(NameTask::Term(Rc::clone(codomain)));
                },
                | StaticTerm::App { function, argument } => {
                    tasks.push(NameTask::Arg(argument.clone()));
                    tasks.push(NameTask::Term(Rc::clone(function)));
                },
                | StaticTerm::Neutral(neutral) => {
                    tasks.push(NameTask::Neutral(Rc::new(neutral.clone())));
                },
            },
            | NameTask::Neutral(neutral) => match neutral.as_ref() {
                | StaticNeutral::Head(variable) => {
                    names.insert(variable.name().as_ref().to_owned());
                },
                | StaticNeutral::App { head, argument } => {
                    tasks.push(NameTask::Arg(argument.clone()));
                    tasks.push(NameTask::Neutral(Rc::clone(head)));
                },
            },
            | NameTask::Arg(StaticArg::Type(term)) => tasks.push(NameTask::Term(term)),
            | NameTask::Arg(StaticArg::Level(_) | StaticArg::Sort(_) | StaticArg::Value(_)) => {},
        }
    }
    names
}

/// One fresh-name collection task.
enum NameTask
{
    /// Visit a term.
    Term(Rc<StaticTerm>),
    /// Visit a neutral.
    Neutral(Rc<StaticNeutral>),
    /// Visit an argument.
    Arg(StaticArg),
}

/// Pops one transformed term.
fn pop_transform_term(results: &mut Vec<TransformResult>) -> StaticTerm
{
    match results.pop() {
        | Some(TransformResult::Term(term)) => term,
        | _ => StaticTerm::Neutral(StaticNeutral::head(StaticVar::new(
            "<substitution-invariant>",
        ))),
    }
}

/// Pops one transformed neutral.
fn pop_transform_neutral(results: &mut Vec<TransformResult>) -> StaticNeutral
{
    match results.pop() {
        | Some(TransformResult::Neutral(neutral)) => neutral,
        | _ => StaticNeutral::head(StaticVar::new("<substitution-invariant>")),
    }
}

/// Pops one transformed argument.
fn pop_transform_arg(results: &mut Vec<TransformResult>) -> StaticArg
{
    match results.pop() {
        | Some(TransformResult::Arg(argument)) => argument,
        | _ => StaticArg::Sort(SortExpr::value()),
    }
}

/// Calculus tests for the static core's two rewriting faces: [`substitute`]'s
/// three cases — replacement, shadowing, and capture — and [`normalize`]'s
/// separation of a beta redex from a stuck application.
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::boundary::BinderName;

    /// The static variable named `name`.
    fn var<'source, N>(name: N) -> StaticVar
    where
        N: Into<BinderName<'source>>,
    {
        StaticVar::new(<&str>::from(name.into()))
    }

    /// The term that is just the variable named `name`.
    fn term<'source, N>(name: N) -> StaticTerm
    where
        N: Into<BinderName<'source>>,
    {
        StaticTerm::Var(var(name))
    }

    /// A binder on `name`, at the ground value classifier every fixture here
    /// shares — the classifier is carried through untouched, so it is held
    /// fixed rather than varied.
    fn binder<'source, N>(name: N) -> StaticBinder
    where
        N: Into<BinderName<'source>>,
    {
        StaticBinder::new(var(name), Classifier::new(SortExpr::value(), Level::zero()))
    }

    /// The static lambda `\name. body`.
    fn lam<'source, N>(
        name: N,
        body: StaticTerm,
    ) -> StaticTerm
    where
        N: Into<BinderName<'source>>,
    {
        StaticTerm::Lam {
            binder: binder(name),
            body: Rc::new(body),
        }
    }

    /// The static product `(name : _) -> codomain`.
    fn pi<'source, N>(
        name: N,
        codomain: StaticTerm,
    ) -> StaticTerm
    where
        N: Into<BinderName<'source>>,
    {
        StaticTerm::Pi {
            binder: binder(name),
            codomain: Rc::new(codomain),
        }
    }

    /// The application `function argument`, at a type argument.
    fn app(
        function: StaticTerm,
        argument: StaticTerm,
    ) -> StaticTerm
    {
        StaticTerm::App {
            function: Rc::new(function),
            argument: StaticArg::Type(Rc::new(argument)),
        }
    }

    #[test]
    fn substitution_replaces_a_free_occurrence()
    {
        // The plain case, at the leaf: the target variable becomes the
        // replacement and nothing else does. The non-target leaf is the
        // positive control, so a mutant that replaced unconditionally is
        // separated from one that replaced correctly.
        assert_eq!(
            term("y"),
            substitute(&term("x"), &var("x"), &term("y")),
            "a free occurrence of the target becomes the replacement"
        );
        assert_eq!(
            term("w"),
            substitute(&term("w"), &var("x"), &term("y")),
            "and a variable that is not the target is left alone"
        );

        // And under a spine, so the walk is exercised rather than only the
        // leaf: both positions are the target, and both are replaced.
        assert_eq!(
            app(term("y"), term("y")),
            substitute(&app(term("x"), term("x")), &var("x"), &term("y")),
            "every free occurrence along an application spine is replaced"
        );
    }

    #[test]
    fn substitution_stops_at_a_shadowing_binder()
    {
        // A binder on the substituted name rebinds it, so nothing beneath it is
        // free and the term comes back untouched.
        let shadowed = lam("x", term("x"));
        assert_eq!(
            shadowed,
            substitute(&shadowed, &var("x"), &term("y")),
            "a binder on the target shadows every occurrence beneath it"
        );

        // The positive control: the same shape with a binder on some other
        // name, where the descent does happen. A mutant that stopped at every
        // binder passes the assertion above and fails this one.
        assert_eq!(
            lam("z", term("y")),
            substitute(&lam("z", term("x")), &var("x"), &term("y")),
            "a binder on another name does not stop the descent"
        );

        // The same pair at the product former, which carries its own copy of
        // the shadowing test.
        let shadowed_pi = pi("x", term("x"));
        assert_eq!(
            shadowed_pi,
            substitute(&shadowed_pi, &var("x"), &term("y")),
            "the product former shadows on its own binder too"
        );
        assert_eq!(
            pi("z", term("y")),
            substitute(&pi("z", term("x")), &var("x"), &term("y")),
            "and descends under a binder on another name"
        );
    }

    #[test]
    fn substitution_avoids_capture()
    {
        // The capture case: the binder is spelled like a free variable of the
        // replacement, so substituting naively would bind the replacement's own
        // `y`. The binder is renamed apart first, to the first serial spelling
        // that no name in the body, the replacement, or the target occupies.
        assert_eq!(
            lam("y$1", term("y")),
            substitute(&lam("y", term("x")), &var("x"), &term("y")),
            "a binder that would capture the replacement is renamed apart"
        );

        // The positive control for the rename: with the replacement carrying no
        // free `y`, the same binder survives verbatim. Its own bound occurrence
        // is in the body, so a mutant that renamed unconditionally would move
        // the binder to a fresh serial and is separated here rather than
        // producing the same term by luck.
        assert_eq!(
            lam("y", app(term("y"), term("w"))),
            substitute(&lam("y", app(term("y"), term("x"))), &var("x"), &term("w")),
            "a binder that would capture nothing keeps its own name"
        );

        // The rename walks the body rather than only the binder: the bound
        // occurrence moves to the fresh name while the substituted occurrence
        // becomes the replacement, so the two cannot be confused.
        assert_eq!(
            lam("y$1", app(term("y$1"), term("y"))),
            substitute(&lam("y", app(term("y"), term("x"))), &var("x"), &term("y")),
            "the bound occurrence follows the rename and the free one does not"
        );

        // And the product former avoids capture on its own binder by the same
        // rule, which is a separate arm of the same match.
        assert_eq!(
            pi("y$1", term("y")),
            substitute(&pi("y", term("x")), &var("x"), &term("y")),
            "the product former renames its capturing binder apart too"
        );
        assert_eq!(
            pi("y", app(term("y"), term("w"))),
            substitute(&pi("y", app(term("y"), term("x"))), &var("x"), &term("w")),
            "and leaves a binder alone when there is nothing to capture"
        );
    }

    #[test]
    fn normalizes_beta_redex()
    {
        // The redex: an application whose function is a lambda reduces by
        // substituting the argument into the body, and the result is the exact
        // term rather than something merely equivalent to it.
        assert_eq!(
            term("y"),
            normalize(&app(lam("x", term("x")), term("y"))),
            "a static lambda applied to an argument reduces"
        );

        // The argument goes where the binder was, rather than becoming the
        // whole contractum. Every fixture above is an identity lambda, where
        // substituting the argument into the body and returning the argument
        // agree — so the separating case is a body that is not the bound
        // variable, and it is what refutes a substitution whose term and
        // replacement are exchanged.
        assert_eq!(
            app(term("f"), term("y")),
            normalize(&app(lam("x", app(term("f"), term("x"))), term("y"))),
            "the argument lands at the binder's occurrence, under the spine"
        );
        assert_eq!(
            app(term("y"), term("y")),
            normalize(&app(lam("x", app(term("x"), term("x"))), term("y"))),
            "and at every occurrence, so a single-occurrence rule is separated"
        );
        assert_eq!(
            term("w"),
            normalize(&app(lam("x", term("w")), term("y"))),
            "a body that ignores its binder discards the argument entirely"
        );

        // Reduction is iterated rather than performed once: the substituted
        // body is itself a redex here, and the normal form is what survives
        // both steps.
        assert_eq!(
            term("y"),
            normalize(&app(
                app(lam("f", term("f")), lam("x", term("x"))),
                term("y")
            )),
            "and the contractum is normalized in turn, so nested redexes go"
        );

        // A redex under a binder is reduced too, which is the surviving-children
        // half of the contract: normalization is not restricted to the root.
        assert_eq!(
            lam("z", term("z")),
            normalize(&lam("z", app(lam("x", term("x")), term("z")))),
            "a redex beneath a binder is reduced where it stands"
        );
    }

    #[test]
    fn normalization_preserves_a_stuck_application()
    {
        // The stuck case: a function position that is not a lambda cannot
        // reduce, so the application is rebuilt exactly as it was. This is the
        // positive control the redex assertions need — a mutant that reduced
        // unconditionally would have to invent a body.
        let stuck = app(term("f"), term("y"));
        assert_eq!(
            stuck,
            normalize(&stuck),
            "an application with a non-lambda function stays an application"
        );

        // Stuck at the root does not mean inert underneath: the argument is
        // still normalized, so a redex inside it goes while the spine survives.
        assert_eq!(
            app(term("f"), term("y")),
            normalize(&app(term("f"), app(lam("x", term("x")), term("y")))),
            "and its argument is normalized while the spine is preserved"
        );

        // The same at the neutral former, whose spine is the shape a declared
        // family's application takes and which no reduction rule may collapse.
        let neutral = StaticTerm::Neutral(StaticNeutral::app(
            StaticNeutral::head(var("f")),
            StaticArg::Type(Rc::new(app(lam("x", term("x")), term("y")))),
        ));
        assert_eq!(
            StaticTerm::Neutral(StaticNeutral::app(
                StaticNeutral::head(var("f")),
                StaticArg::Type(Rc::new(term("y"))),
            )),
            normalize(&neutral),
            "a neutral spine survives with its arguments normalized"
        );
    }
}
