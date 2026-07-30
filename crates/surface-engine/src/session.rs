//! The REPL session engine (proposal §4, `incremental-pipeline.md` §10).
//!
//! The smallest end-to-end interactive slice: [`Session::submit`] takes one
//! line of source, lowers it totally ([`crate::lower::lower_source_total`]),
//! reports diagnostics and hole goals ([`crate::diag::report`]), and — for each
//! hole-free, well-typed item — types it ([`item_type`]) on the heap-stacked
//! typing machine and, for an expression item, evaluates it on the
//! [`gandr_core_sequent::machine`] L machine, returning the structured
//! [`Submission`] a front-end renders. The engine itself is presentation-free
//! (no `Display`, decision D3) so every frontend reuses the same session core.
//!
//! # Cross-line definitions — the interim "session-prelude" (strategy A)
//!
//! Top-level items lower *independently* (the lowerer threads no context
//! between them). Cross-line *definitions* are bridged below *without any core
//! change*; a separate, orthogonal bridge carries the **prelude**. The L
//! machine resolves module-qualified native builtins through [`prelude_env`],
//! which [`Session::submit`] threads into every run. The session bridges
//! both ways:
//!
//! - **typing** carries an accumulating [`Ctx`]: a processed `def name = …`
//!   binds `name : A` into [`Session::ctx`], so a later line's expression types
//!   against it;
//! - **evaluation** runs each definition to a *value* once, at definition time
//!   (eager, but v0-safe: the pure spine is terminating and effect-free), and
//!   stores that value; a later expression folds the stored values into a
//!   `ret`-`Bind` chain ([`eval_chain`]) — the same sequencing shape the
//!   lowerer gives a block's `val` / `run` statements — so the L machine
//!   extends its value environment with each stored definition before running
//!   the expression.
//!
//! Storing the *evaluated value* (not the definition term) keeps the chain a
//! sequence of `ret v >>= …`, which never re-runs a computation: an unevaluable
//! definition — for example, one that reaches defined blame instead of `ret` —
//! is bound for typing but carries **no** stored value, so it can never make an
//! *unrelated* later expression stuck (no poisoning), and an evaluable
//! definition runs exactly once (REPL memoization).
//!
//! This is the interim simplification of `incremental-pipeline.md` §10's
//! persisted `Γ`/`Θ`; the persistent typed context (with checkpoints) is A2.3.
//! Only definitions with a **value type** are bindable as variables (CBPV: a
//! variable is a value); a definition of bare computation type — e.g. an
//! un-thunked `λ` — is reported with `bound = false` and left out of scope
//! (thunk it to name it), matching the lowerer's own `let` discipline.
//!
//! # What evaluates in v0
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

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::ConstructorTag;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::machine;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::DataId;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
use gandr_core_sequent::machine::run_comp_with_prelude;

use crate::boundary::ConstructorName;
use crate::boundary::DefinitionName;
use crate::boundary::HolePresence;
use crate::boundary::PipelineSource;
use crate::diag;
use crate::ffi::ForeignModule;
use crate::goals::initial_state;
use crate::lower::LowerError;
use crate::lower::LoweredItem;
use crate::lower::lower_source_total_seeded;
use crate::prelude::Prelude;
use crate::prelude_ctx;
use crate::prelude_env;

/// The outcome of one lowered item in a [`Submission`], in source order.
///
/// Typing failures and hole goals are carried by the submission's
/// [`diag::Report`] (the A2.4 surface); the variants here add the *success*
/// information a front-end needs — a definition's type and whether it entered
/// scope, and an expression's type and evaluation outcome — plus markers for
/// the non-success items so a consumer can iterate items and the report in
/// lock-step.
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
        /// `false` for a bare-computation-typed one (reported but not bound —
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
    /// An item carrying one or more holes: evaluation is declined (the bead's
    /// parse-completeness validator) and the holes are listed as goals in the
    /// submission's report.
    Holey,
    /// An item of unknown sort (`Term` is `non_exhaustive` upstream); no
    /// success information is derivable.
    Unknown,
}

/// The structured result of one [`Session::submit`].
///
/// Pairs the A2.4 [`diag::Report`] (diagnostics and hole goals, rendered by the
/// front-end) with one [`ItemOutcome`] per lowered item, in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submission
{
    /// The diagnostics-and-goals report for the submitted source, typed against
    /// the session context as it stood at submission start.
    pub report: diag::Report,
    /// One outcome per lowered item, in source order (aligned with
    /// `report` by item index).
    pub outcomes: Vec<ItemOutcome>,
}

/// A REPL session: the accumulating typing context and definition prelude that
/// carry definitions across lines (see the module doc).
#[derive(Clone, Debug)]
pub struct Session
{
    /// The typing context: the prelude operators plus every value-typed
    /// definition bound so far (innermost last, so later definitions shadow).
    ctx: Ctx,
    /// The accumulated definition *values*, in definition order — each the
    /// once-evaluated result of a value-typed `def` ([`Session::define`]). A
    /// later expression folds these into a `ret`-`Bind` chain ([`eval_chain`]);
    /// a definition that did not reduce to a value contributes none.
    defs: Vec<(String, Value)>,
    /// The eval-side prelude binding environment: module-qualified native
    /// builtins consumed by the L machine's prelude focus. Built once
    /// ([`prelude_env`]); shared (`Rc`) into every run.
    prelude: Prelude,
    /// The `extern`-declared foreign modules accumulated across submissions,
    /// keyed by namespace (proposal-ffi.md §2). Bridged across lines exactly as
    /// definitions are: an `extern` block submitted on one line makes a later
    /// line's call `m.op(args)` elaborate to a `perform` (§3.1). A foreign call
    /// with no handler installed blames `PerformNoHandler` — the
    /// capability-denied outcome (§3.2), least authority made visible.
    foreign: BTreeMap<String, ForeignModule>,
    /// The `codata`-declared observation shapes accumulated across submissions,
    /// keyed by codata type name (codata design §2). Bridged across lines
    /// exactly as `extern` modules and definitions are: a `codata C` block
    /// submitted on one line makes a later line's `def rec f() -> C`
    /// coverage-check and a later `s.π` observe against it (`codata MVP`).
    codata: BTreeMap<String, crate::lower::codata::CodataDecl>,
    /// The `data`-declared datatype shapes accumulated across submissions,
    /// keyed by datatype name (declared-data design Decision 4). Bridged across
    /// lines like `codata` and `extern`: the decl table (the constructor
    /// enumeration and the minted `DataId`) that a front-end renderer
    /// resolves a declared-constructor value's `tag` against to print its
    /// constructor name (`Some(3)`, `Red`) rather than the structural
    /// carrier.
    data: BTreeMap<String, crate::lower::data::DataDecl>,
}

impl Session
{
    /// Opens a fresh session: the prelude context ([`prelude_ctx`]) and no
    /// definitions.
    ///
    /// # Contract
    /// - ensures: the returned session types against the prelude operators and
    ///   carries no definitions.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            ctx: prelude_ctx(),
            defs: Vec::new(),
            prelude: prelude_env(),
            foreign: BTreeMap::new(),
            codata: BTreeMap::new(),
            data: BTreeMap::new(),
        }
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

    /// Lowers, reports, types, and (for hole-free expressions) evaluates one
    /// line of source, advancing the session with any new value-typed
    /// definitions.
    ///
    /// # Contract
    /// - ensures: on success returns a [`Submission`] with one [`ItemOutcome`]
    ///   per lowered item (in source order) and the A2.4 [`diag::Report`];
    ///   every value-typed `def` item is bound into the session (`ctx` and the
    ///   eval prelude) before later items in the same submission are processed.
    /// - provides: the smallest end-to-end interactive slice (read → lower →
    ///   report → type → eval → result).
    /// - fails: only the infrastructure lowering failures
    ///   ([`LowerError::ParserUnavailable`] / [`LowerError::ParseFailed`]) —
    ///   every parseable input lowers totally, with out-of-fragment regions
    ///   becoming holes.
    /// - panics: none; evaluation runs on the environment-backed L machine, and
    ///   a hole, blame, or ill-typed redex surfaces as a non-terminal [`Eval`].
    ///
    /// # Errors
    ///
    /// Returns the lowering [`LowerError`] for an infrastructure failure.
    #[inline]
    pub fn submit<'source, S>(
        &mut self,
        source: S,
    ) -> Result<Submission, LowerError>
    where
        S: Into<PipelineSource<'source>>,
    {
        let source = source.into();
        let lowered = lower_source_total_seeded(source, &self.foreign, &self.codata, &self.data)?;
        let report = diag::report(&lowered, &self.ctx);
        let mut outcomes = Vec::with_capacity(lowered.items.len());
        for (index, item) in lowered.items.iter().enumerate() {
            let holey = report.goals.iter().any(|goal| goal.item == index);
            outcomes.push(self.process_item(item, holey));
        }
        // Persist this submission's `codata` blocks so a later line's copattern
        // definition or observation sees the declaration (codata design §2 bridge).
        // Before the `foreign` move below, which partially moves `lowered`.
        for (name, decl) in lowered.codata() {
            self.codata.insert(name.clone(), decl.clone());
        }
        // Persist this submission's `data` blocks so a later line's renderer
        // resolves a returned constructor value's name (declared-data design stage d
        // bridge).
        for (name, decl) in lowered.data() {
            self.data.insert(name.clone(), decl.clone());
        }
        // Persist this submission's `extern` declarations so a later line's
        // foreign call sees the module — the FFI analogue of the definition
        // bridge above (an `extern` block itself yields no runnable item).
        for module in lowered.foreign {
            self.foreign.insert(module.name.clone(), module);
        }
        Ok(Submission { report, outcomes })
    }

    /// Processes one lowered item against the current session, advancing the
    /// session when the item is a value-typed definition.
    fn process_item(
        &mut self,
        item: &LoweredItem,
        holey: impl Into<HolePresence>,
    ) -> ItemOutcome
    {
        if holey.into().0 {
            // The parse-completeness validator: an item with holes is not
            // evaluated (and a holey definition is not bound); its goals are in
            // the report.
            return ItemOutcome::Holey;
        }
        match item_type(item, &self.ctx) {
            | None => ItemOutcome::Unknown,
            | Some(Err(error)) => ItemOutcome::TypeError { error },
            | Some(Ok(ty)) => match item.name {
                | Some(ref name) => self.define(name, item, ty),
                | None => ItemOutcome::Expression {
                    value: run_comp_with_prelude(
                        &eval_chain(&self.defs, &item.term),
                        self.prelude.as_bindings(),
                    ),
                    ty,
                },
            },
        }
    }

    /// Records a successfully-typed `def name = …` item, binding it into scope
    /// when it has a value type.
    fn define<'name>(
        &mut self,
        name: impl Into<DefinitionName<'name>>,
        item: &LoweredItem,
        ty: Ty,
    ) -> ItemOutcome
    {
        let name = name.into();
        match bound_value_type(&ty) {
            | Some(value_type) => {
                // Evaluate the definition once, now, under the definitions
                // already in scope; store its value only when it reduces to
                // one. A definition that reaches defined blame or another
                // non-value outcome is type-only — it stores nothing, so it
                // can never make an unrelated later expression stuck.
                if let Eval::Value(Comp::Ret(value)) = run_comp_with_prelude(
                    &eval_chain(&self.defs, &item.term),
                    self.prelude.as_bindings(),
                ) {
                    self.defs.push((name.0.to_owned(), (*value).clone()));
                }
                self.ctx.bind(name.0.to_owned(), value_type.clone());
                ItemOutcome::Definition {
                    name: name.0.to_owned(),
                    ty: Ty::Value(value_type),
                    bound: true,
                }
            },
            | None => ItemOutcome::Definition {
                name: name.0.to_owned(),
                ty,
                bound: false,
            },
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

/// Types one lowered item against `base`, returning its result type.
///
/// The "terminal computation type of a `Done` expression item" the REPL shows
/// before the value (the `REPL-session work` pipeline-result-type addition).
/// Uses [`crate::goals::initial_state`]'s sort-and-ascription dispatch, then
/// drives the heap-stacked typing machine to completion: an item is checked
/// against its recorded ascription when the sorts match, inferred otherwise.
///
/// # Contract
/// - ensures: returns `Some(Ok(ty))` with the item's value/computation type on
///   success, `Some(Err(error))` with the first [`TypeError`] on a typing
///   failure, and `None` only for an item of unknown sort (`Term` is
///   `non_exhaustive` upstream).
/// - panics: none; typing is driven by [`gandr_core_checker::machine`], whose
///   continuation stack is heap-allocated, so input-scaled nesting in the term
///   does not consume the host call stack during item type determination.
#[inline]
#[must_use]
pub fn item_type(
    item: &LoweredItem,
    base: &Ctx,
) -> Option<Result<Ty, TypeError>>
{
    let state = initial_state(item, base)?;
    Some(machine::run(state).0)
}

/// The value type a definition binds into scope, or [`None`] when it has no
/// value type (a bare computation type — `Arrow` / `With` / `Unknown` — cannot
/// be a variable in CBPV).
///
/// A returner `F A` binds its payload `A` (the value the definition produces);
/// a value-typed definition binds its own type.
fn bound_value_type(ty: &Ty) -> Option<ValueType>
{
    match *ty {
        | Ty::Value(ref value_type) => Some(value_type.clone()),
        | Ty::Comp(CompType::F(ref payload, _)) => Some((**payload).clone()),
        | _ => None,
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
        // `Term` is non_exhaustive upstream; a future sort is driven as `ret
        // ()` so the chain stays total (it never arises in v0).
        | _ => Comp::ret(Value::Unit),
    }
}
