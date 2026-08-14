//! The REPL session engine
//! (`spec:implementation/incremental-pipeline.md` §"The
//! read-evaluate loop").
//!
//! The smallest end-to-end interactive slice: [`Session::submit`] takes one
//! line of source, lowers it totally ([`crate::lower::lower_source_total`]),
//! reports diagnostics and hole goals ([`crate::diag::report`]), and resumes
//! [`gandr_core_incremental`] at the submitted items' boundaries. The validated
//! resume reuses prior checkpoints where their terms and dependency footprints
//! remain valid and re-types the dirty frontier. Hole-free expressions then
//! evaluate on the [`gandr_core_sequent::machine`] L machine, returning the
//! structured [`Submission`] a front-end renders. The engine is
//! presentation-free (no `Display`; the core stays free of presentation
//! dependencies) so every frontend reuses the same session core.
//!
//! # Cross-line definitions
//!
//! Top-level items lower independently. [`Session`] retains their ordered
//! [`Program`] and the matching [`Checkpoints`], then appends each new
//! submission and calls [`resume_with`] against the surface prelude context.
//! The checkpoint engine is therefore the typing authority across lines:
//!
//! - **typing** comes from the resumed [`ItemTyping`] sequence. A bound
//!   definition's validated typing contributes its name and value type to the
//!   session context used by diagnostics for the next submission;
//! - **evaluation** runs each definition to a value once, at definition time
//!   (eager, but safe here: the pure spine is terminating and effect-free), and
//!   stores that value. A later expression folds the stored values into a
//!   `ret`-`Bind` chain ([`eval_chain`]), the same sequencing shape the lowerer
//!   gives a block's `val` / `run` statements, so the L machine extends its
//!   value environment with each stored definition before running the
//!   expression.
//!
//! Storing the evaluated value rather than the definition term keeps the chain
//! a sequence of `ret v >>= …`, which never re-runs a computation. An
//! unevaluable definition, for example one that reaches defined blame instead
//! of `ret`, is bound for typing but carries no stored value, so it cannot make
//! an unrelated later expression stuck. An evaluable definition runs exactly
//! once (REPL memoization).
//!
//! The item checkpoints are the persisted `Γ`-side state of
//! `spec:implementation/incremental-pipeline.md` §"The read-evaluate loop".
//! Only definitions with a **value type** are bindable as variables (CBPV: a
//! variable is a value). A definition of bare computation type, such as an
//! un-thunked `λ`, is reported with `bound = false` and left out of scope
//! (thunk it to name it), matching the lowerer's own `let` discipline.
//!
//! # What evaluates
//!
//! The pure CBPV spine (literals, pairs, lists, injections, `let`/bind,
//! forcing a thunk, `case`/`split` over closed scrutinees, lambdas), **plus**
//! the fixed-table operator builtins and module-qualified native builtins
//! resolved through the eval prelude ([`prelude_env`], module/prelude design):
//! `1 + 2` evaluates through the same native-builtin seam as `force(prim.id) 5`
//! and `list.push([1, 2], 3)`. The session surfaces a definition that does not
//! reduce to `ret` of a value — for example, a handler-less effect that reaches
//! defined blame or another non-value outcome — honestly as a type-only
//! definition, never as a stored value.
//!
//! # The kernel crossing
//!
//! A session also carries a [`KernelAdmissions`] ledger and offers every typed
//! definition to it in source order, so the surface language's own `def` items
//! are what drives [`gandr_core_checker::kernel_bridge`] and the kernel's
//! `add_decl` choke point. The ledger is where the definitions that lower into
//! the closed S1 vocabulary accumulate as a kernel environment, and each
//! submission reports one [`KernelVerdict`] per item beside its
//! [`ItemOutcome`].
//!
//! **The crossing is observation, never authority.** Typing comes from the
//! resumed checkpoints and evaluation from the L machine, exactly as before: no
//! verdict feeds back into either, and a definition the kernel declines is
//! bound, evaluated, and reported precisely as it was. The kernel sits
//! downstream of the session, so what the ledger makes of a submission cannot
//! change that submission.
//!
//! [`KernelAdmissions`]: crate::kernel::KernelAdmissions
//! [`KernelVerdict`]: crate::kernel::KernelVerdict

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::ConstructorTag;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::DataId;
use gandr_core_checker::types::Ty;
use gandr_core_incremental::checkpoint::Checkpoints;
use gandr_core_incremental::checkpoint::ItemTyping;
use gandr_core_incremental::checkpoint::resume_with;
use gandr_core_incremental::region::Item;
use gandr_core_incremental::region::Program;
use gandr_core_sequent::machine::run_comp_with_prelude;

use crate::boundary::ConstructorName;
use crate::boundary::DefinitionBindingFlag;
use crate::boundary::DefinitionName;
use crate::boundary::PipelineSource;
use crate::boundary::SourceRange;
use crate::diag;
use crate::ffi::ForeignModule;
use crate::kernel::DefinitionOffer;
use crate::kernel::KernelAdmissions;
use crate::kernel::KernelVerdict;
use crate::kernel::WithheldReason;
use crate::lower::ImportDeclaration;
use crate::lower::ImportIndex;
use crate::lower::LowerError;
use crate::lower::lower_source_total_seeded;
use crate::namespace::NamePath;
use crate::namespace::Scope;
use crate::prelude::Prelude;
use crate::prelude_ctx;
use crate::prelude_env;

/// The outcome of one lowered item in a [`Submission`], in source order.
///
/// Typing failures and hole goals are carried by the submission's
/// [`diag::Report`]; the variants here add the success information a front-end
/// needs: a definition's type and whether it entered scope, or an expression's
/// type and evaluation outcome. Markers for non-success items let a consumer
/// iterate items and the report in lock-step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemOutcome
{
    /// A `def name = …` item that typed successfully.
    Definition
    {
        /// The defined name.
        name: String,
        /// The definition's type.
        ty: Ty,
        /// Whether the definition entered scope: `true` for a value-typed
        /// definition (bound into [`Session::ctx`] and the eval prelude),
        /// `false` for a bare-computation-typed one (reported but not bound;
        /// thunk it to name it).
        bound: bool,
    },
    /// An expression item that typed successfully, with its evaluation result.
    Expression
    {
        /// The expression's type (its terminal computation type, or the value
        /// type for a value item).
        ty: Ty,
        /// The L-machine outcome of evaluating the expression under the
        /// accumulated definitions (a terminal value, blame, or stuck result).
        value: Eval,
    },
    /// An item whose typing failed; the matching [`diag::Diagnostic`] in the
    /// submission's report carries the source-ranged detail.
    TypeError
    {
        /// The first typing error, as raised by the checker.
        error: TypeError,
    },
    /// An item carrying one or more holes: evaluation is declined (the
    /// parse-completeness validator) and the holes are listed as goals in the
    /// submission's report.
    Holey,
}

/// The structured result of one [`Session::submit`].
///
/// Pairs the [`diag::Report`] (diagnostics and hole goals, rendered by the
/// front-end) with one [`ItemOutcome`] per lowered item, in source order, and
/// one [`KernelVerdict`] per item beside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submission
{
    /// The diagnostics-and-goals report for the submitted source, typed against
    /// the session context as it stood at submission start.
    pub report: diag::Report,
    /// One outcome per lowered item, in source order (aligned with
    /// `report` by item index).
    pub outcomes: Vec<ItemOutcome>,
    /// What the kernel made of each lowered item, in the same source order and
    /// index-aligned with `outcomes`. Every item has an entry: one the kernel
    /// never saw carries [`KernelVerdict::Withheld`] naming why.
    pub kernel: Vec<KernelVerdict>,
}

/// A REPL session: item-granular typing checkpoints and the definition values
/// that carry definitions across lines (see the module doc).
#[derive(Clone, Debug)]
pub struct Session
{
    /// The surface prelude context against which every checkpoint set starts.
    base_ctx: Ctx,
    /// The typing context after the current checkpoint set, used to report the
    /// next submission's diagnostics.
    ctx: Ctx,
    /// The complete ordered item program whose typing state is checkpointed.
    program: Program,
    /// One validated typing checkpoint per item in [`Self::program`].
    checkpoints: Checkpoints,
    /// The accumulated definition values, in definition order, each the
    /// once-evaluated result of a value-typed `def` ([`Session::define`]).
    defs: Vec<(String, Value)>,
    /// The eval-side prelude binding environment: module-qualified native
    /// builtins consumed by the L machine's prelude focus.
    prelude: Prelude,
    /// The `extern`-declared foreign modules accumulated across submissions,
    /// keyed by namespace (proposal-ffi.md §2).
    foreign: BTreeMap<String, ForeignModule>,
    /// The `codata`-declared observation shapes accumulated across submissions,
    /// keyed by codata type name (codata design §2).
    codata: BTreeMap<String, crate::lower::codata::CodataDecl>,
    /// The `data`-declared datatype shapes accumulated across submissions,
    /// keyed by datatype name (declared-data design Decision 4).
    data: BTreeMap<String, crate::lower::data::DataDecl>,
    /// Import declarations accumulated across submissions, in the same global
    /// source order named by [`ImportIndex`].
    imports: Vec<ImportDeclaration>,
    /// The persistent visible import namespace. Its bindings index
    /// [`Self::imports`].
    import_scope: Scope<ImportIndex, SourceRange>,
    /// The kernel environment the session's S1-eligible definitions accumulate
    /// in, and the naming environment that resolves one to another.
    kernel: KernelAdmissions,
}

impl Session
{
    /// Opens a fresh session: the surface prelude and an empty checkpoint set.
    ///
    /// # Contract
    /// - ensures: the returned session types against the prelude operators and
    ///   carries no source items, checkpoints, or definitions.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        let base_ctx = prelude_ctx();
        Self {
            ctx: base_ctx.clone(),
            base_ctx,
            program: Program::default(),
            checkpoints: Checkpoints::default(),
            defs: Vec::new(),
            prelude: prelude_env(),
            foreign: BTreeMap::new(),
            codata: BTreeMap::new(),
            data: BTreeMap::new(),
            imports: Vec::new(),
            import_scope: Scope::new(),
            kernel: KernelAdmissions::new(),
        }
    }

    /// The kernel admissions this session has accumulated.
    ///
    /// # Contract
    /// - ensures: returns the ledger holding the kernel environment every
    ///   [`KernelVerdict::Admitted`] definition entered, in admission order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn kernel(&self) -> &KernelAdmissions
    {
        &self.kernel
    }

    /// The declared constructor name for a value `Ctor { id, tag, … }` — the
    /// decl-table lookup a front-end renderer uses to print `Some(3)` / `Red`
    /// rather than the structural carrier (declared-data design stage d). The
    /// datatype is keyed by the `id`'s name; the constructor is its
    /// `tag`-th.
    ///
    /// # Contract
    /// - ensures: `Some(name)` when the datatype is registered and `tag` is in
    ///   range; `None` otherwise (an unknown / stale id — the renderer then
    ///   falls back to a structural rendering).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn constructor_name<T>(
        &self,
        id: &DataId,
        tag: T,
    ) -> Option<ConstructorName<'_>>
    where
        T: Into<ConstructorTag>,
    {
        let tag = usize::from(tag.into());
        self.data
            .get(id.name().as_ref())
            .filter(|decl| decl.id == *id)
            .and_then(|decl| decl.ctors.get(tag))
            .map(ConstructorName::from)
    }

    /// The current typing context (prelude plus bound definitions).
    ///
    /// # Contract
    /// - ensures: returns the context an expression submitted now would type
    ///   against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn ctx(&self) -> &Ctx
    {
        &self.ctx
    }

    /// Resolves a persistent import path to the declaration that introduced
    /// its binding.
    ///
    /// # Contract
    /// - ensures: returns the declaration indexed by the visible namespace
    ///   binding at `path`.
    /// - ensures: returns `None` when `path` is unbound.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resolve_import(
        &self,
        path: &NamePath,
    ) -> Option<&ImportDeclaration>
    {
        let index = self.import_scope.resolve(path)?.data.0;
        self.imports.get(index)
    }

    /// Lowers, reports, incrementally types, and evaluates one line of source.
    ///
    /// # Contract
    /// - ensures: on success returns one [`ItemOutcome`] and one
    ///   [`KernelVerdict`] per newly lowered item, in source order, and retains
    ///   one validated checkpoint per item seen across the whole session;
    ///   value-typed definitions enter the diagnostic and evaluation
    ///   environments before later items are processed, and every typed
    ///   definition is offered to the session's kernel ledger after its own
    ///   outcome is produced.
    /// - provides: the smallest end-to-end interactive slice (read → lower →
    ///   report → validated resume → eval → result), and the engine's crossing
    ///   from checked core into the certified kernel.
    /// - fails: only the infrastructure lowering failures
    ///   ([`LowerError::ParserUnavailable`] / [`LowerError::ParseFailed`]);
    ///   every parseable input lowers totally, with out-of-fragment regions
    ///   becoming holes.
    /// - panics: none; evaluation runs on the environment-backed L machine, and
    ///   a hole, blame, or ill-typed redex surfaces as a non-terminal [`Eval`].
    ///
    /// # Errors
    ///
    /// Returns the lowering [`LowerError`] for an infrastructure failure.
    ///
    /// # Adequacy
    /// - hypothesis: the session's incremental typing sequence must equal a
    ///   from-scratch typing of the same accumulated program while preserving
    ///   cross-line evaluation. The session differential and the core
    ///   checkpoint differential distinguish stale adoption, missing
    ///   checkpoints, and a return to detached per-item typing.
    /// - witness: `tests::session::checkpointed_session_matches_from_scratch`
    /// - witness: `gandr_core_incremental::tests::incremental`
    /// - witness: `tests::kernel::every_item_carries_exactly_one_verdict`
    /// - witness: `tests::kernel::a_later_definition_refers_to_an_earlier_one`
    #[inline]
    pub fn submit<'source, S>(
        &mut self,
        source: S,
    ) -> Result<Submission, LowerError>
    where
        S: Into<PipelineSource<'source>>,
    {
        let lowered = lower_source_total_seeded(
            source.into(),
            &self.foreign,
            &self.codata,
            &self.data,
            &self.import_scope,
            ImportIndex(self.imports.len()),
        )?;
        let report = diag::report(&lowered, &self.ctx);

        let prior_item_count = self.program.items.len();
        let mut edited = self.program.clone();
        edited.items.extend(lowered.items.iter().cloned());
        let resumed = resume_with(&self.checkpoints, &edited, &self.base_ctx);

        let mut outcomes = Vec::with_capacity(lowered.items.len());
        let mut verdicts = Vec::with_capacity(lowered.items.len());
        for (item, typing) in lowered
            .items
            .iter()
            .zip(resumed.typings().skip(prior_item_count))
        {
            let (outcome, verdict) = self.process_item(item, typing);
            outcomes.push(outcome);
            verdicts.push(verdict);
        }
        debug_assert_eq!(
            outcomes.len(),
            lowered.items.len(),
            "resume yields one typing and outcome per appended item"
        );
        debug_assert_eq!(
            verdicts.len(),
            outcomes.len(),
            "every item's outcome is paired with exactly one kernel verdict"
        );
        self.program = edited;
        self.checkpoints = resumed.into_checkpoints();

        // Declaration tables become visible only to later submissions, after
        // this submission's outcomes have consumed the prior tables.
        for (name, decl) in lowered.codata() {
            self.codata.insert(name.clone(), decl.clone());
        }
        for (name, decl) in lowered.data() {
            self.data.insert(name.clone(), decl.clone());
        }
        self.imports.extend(lowered.imports.iter().cloned());
        self.import_scope = lowered.import_scope().clone();
        for module in lowered.foreign {
            self.foreign.insert(module.name.clone(), module);
        }
        Ok(Submission {
            report,
            outcomes,
            kernel: verdicts,
        })
    }

    /// Projects one engine-owned typing into the session's evaluation outcome
    /// and the kernel's verdict on the same item.
    ///
    /// The two are independent: the outcome is produced first and the kernel is
    /// offered afterwards, so nothing the kernel decides can reach the typing,
    /// the binding, or the evaluation of the item it decided about.
    fn process_item(
        &mut self,
        item: &Item,
        typing: &ItemTyping,
    ) -> (ItemOutcome, KernelVerdict)
    {
        match *typing {
            | ItemTyping::Definition {
                ref name,
                ref ty,
                bound,
            } => {
                let outcome = self.define(name, item, ty.clone(), bound.into());
                let verdict = self.kernel.offer(DefinitionOffer {
                    name: DefinitionName::from(name),
                    term: &item.term,
                    ty,
                });
                (outcome, verdict)
            },
            | ItemTyping::Expression { ref ty } => (
                ItemOutcome::Expression {
                    value: run_comp_with_prelude(
                        &eval_chain(&self.defs, &item.term),
                        self.prelude.as_bindings(),
                    ),
                    ty: ty.clone(),
                },
                KernelVerdict::Withheld {
                    reason: WithheldReason::Expression,
                },
            ),
            | ItemTyping::TypeError { ref error } => (
                ItemOutcome::TypeError {
                    error: error.clone(),
                },
                KernelVerdict::Withheld {
                    reason: WithheldReason::Untyped,
                },
            ),
            | ItemTyping::Holey => (ItemOutcome::Holey, KernelVerdict::Withheld {
                reason: WithheldReason::Holey,
            }),
        }
    }

    /// Records a successfully-typed definition and, when bound, its value.
    fn define<'name>(
        &mut self,
        name: impl Into<DefinitionName<'name>>,
        item: &Item,
        ty: Ty,
        bound: DefinitionBindingFlag,
    ) -> ItemOutcome
    {
        let name = name.into();
        if bool::from(bound)
            && let Ty::Value(ref value_type) = ty
        {
            // Evaluate the definition once under the definitions already in
            // scope. A non-value terminal contributes no eval binding and
            // cannot poison an unrelated later expression.
            if let Eval::Value(Comp::Ret(value)) = run_comp_with_prelude(
                &eval_chain(&self.defs, &item.term),
                self.prelude.as_bindings(),
            ) {
                self.defs.push((name.0.to_owned(), (*value).clone()));
            }
            self.ctx.bind(name.0.to_owned(), value_type.clone());
        }
        ItemOutcome::Definition {
            name: name.0.to_owned(),
            ty,
            bound: bound.into(),
        }
    }
}

impl Default for Session
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

/// Folds the stored definition values into a `ret`-`Bind` chain around `expr`,
/// so the L machine extends its value environment with each stored definition
/// before running the expression.
///
/// Each definition `name = v` folds as `ret v >>= name. rest`, the earliest
/// outermost — so a later definition's value may mention an earlier name. Every
/// bound computation is a `ret v` (never a redex), so the chain re-runs no
/// definition and an unrelated stuck definition can never appear in it.
fn eval_chain(
    defs: &[(String, Value)],
    expr: &Term,
) -> Comp
{
    let mut body = term_into_comp(expr);
    for entry in defs.iter().rev() {
        body = Comp::bind(Comp::ret(entry.1.clone()), &entry.0, body);
    }
    body
}

/// Coerces a [`Term`] into the computation the L machine drives: a value `v`
/// becomes `ret v`, while a computation passes through.
fn term_into_comp(term: &Term) -> Comp
{
    match *term {
        | Term::Value(ref value) => Comp::ret(value.clone()),
        | Term::Comp(ref comp) => comp.clone(),
    }
}
