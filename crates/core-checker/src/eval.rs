#![allow(
    unknown_lints,
    non_topologically_sorted_functions,
    reason = "the evaluator and readback helpers form an explicit mutually recursive machine with no linear call order"
)]

//! The frozen-core sequential CEK evaluator.
//!
//! It is the current driver and the differential oracle for gandr's polarized L
//! machine during the L1 migration (`effects-control-shell.md` §4;
//! `core-ir-contract.md` §6.3/§6.4; ADR-34).
//!
//! This is the *second* machine of ADR-9/ADR-22 on the **evaluation** side: it
//! is derived from the recursive big-step evaluator [`eval_comp`] by the same
//! functional correspondence (CPS-transform, then defunctionalize the
//! continuations into an explicit stack of [`Cont`] frames) that produced the
//! typing machine ([`crate::machine`]) from the recursive checker
//! ([`crate::checker`]). The configuration is `⟨t | ρ | K⟩` (a computation in
//! focus, a value environment, and a continuation `K`); [`step`] is one
//! reduction; [`run`] drives to a terminal.
//!
//! # Two machines, kept differential (ADR-9, ADR-34 D1)
//!
//! [`eval_comp`] is the direct-style reference; the CEK machine is the derived
//! artifact. Both return an [`Eval`], and the conformance suite property-tests
//! them for **agreement** on the pure CBPV spine — the runtime analogue of the
//! `checker ≡ machine` differential. A direct recursive evaluator cannot
//! express `shift`/handlers (there is no reified continuation to capture),
//! which is exactly *why* the CEK machine exists; so [`eval_comp`] covers the
//! pure spine and returns a defined [`StuckReason::UnsupportedByReference`] on
//! a control or effect form (the differential never feeds it one), while the
//! CEK machine covers the full language.
//!
//! Like [`crate::checker`], [`eval_comp`] recurses on the host call stack. The
//! CEK machine keeps the value environment and continuation on the heap, while
//! substitution ([`subst_comp`]) is an explicit heap worklist ([`Subst`],
//! ADR-47) rather than host recursion, so neither deep *continuation* nesting
//! (a left-nested `bind` chain) nor a deep *substituted-into* term overflows
//! the host stack — the machine's nesting bound is memory, as the typing
//! machine's is. (The residual host-recursive walks over a deep term are the
//! derived `Drop` / `PartialEq` over the `Rc` web, whose iterative conversion
//! is tracked under the same epic, tranche T2.)
//!
//! # Runtime continuation (ADR-34 D1; contract §6.3/§6.4)
//!
//! The runtime `K` is [`Cont`]: the reifiable structural frames (argument /
//! bind / projection — the runtime image of [`crate::syntax::Stack`]) **plus**
//! the runtime-only delimiter / handler frames [`Cont::Reset`] /
//! [`Cont::Handle`] (`KReset` / `KHandle`). A captured **structural** prefix
//! reifies to a [`crate::syntax::Value::Stk`] (reify stays over `Stack` only);
//! the delimiter / handler frames are not source-constructible. The frame
//! inventory is **derived** here, not transcribed from the contract (§6.4):
//! application pushes [`Cont::Arg`], sequencing [`Cont::Bind`], projection
//! [`Cont::Prj`], `reset` [`Cont::Reset`], `handle` [`Cont::Handle`]; `force` /
//! `case` / `split` / `dup` / `drop` / `hole` reduce in place and push no
//! frame.
//!
//! # Outcomes (ADR-34 D4)
//!
//! The defined non-stuck outcomes are a **terminal** (a value-producing whnf at
//! the empty continuation) and **blame** ([`Blame`] — a typed runtime halt:
//! the gradual-hole `Canonicity` arm, plus the transitional control blames of
//! ADR-34 D5). An **undefined stuck** ([`StuckReason`]) is reachable only on an
//! ill-typed configuration; the operational soundness oracle asserts a closed
//! well-typed `F A` never reaches one.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use gandr_theory_nominal_automata::Atom;
use gandr_theory_nominal_automata::Gensym;
use gandr_theory_nominal_automata::GensymExhausted;

use crate::boundary::BinderName;
use crate::boundary::ContinuationFrameIndex;
use crate::boundary::ContinuationName;
use crate::boundary::MachineStepCount;
use crate::boundary::NameRef;
use crate::boundary::OperationName;
use crate::boundary::ShieldedNames;
use crate::boundary::StackDepth;
use crate::boundary::TerminalStatus;
use crate::effect::EffectSig;
use crate::grade::Grade;
use crate::nominal::GandrSort;
use crate::syntax::Comp;
use crate::syntax::CompNodeId;
use crate::syntax::FlatArena;
use crate::syntax::HoleId;
use crate::syntax::NumLit;
use crate::syntax::OpClause;
use crate::syntax::Side;
use crate::syntax::Stack;
use crate::syntax::Value;
use crate::syntax::WalkBase;
use crate::types::ValueType;

/// A runtime continuation frame `K` (`core-ir-contract.md` §6.3; ADR-34 D1).
///
/// The structural frames [`Self::Arg`] / [`Self::Bind`] / [`Self::Prj`] are the
/// runtime image of [`crate::syntax::Stack`] (reifiable into a `stk K`); the
/// delimiter / handler frames [`Self::Reset`] / [`Self::Handle`] are runtime
/// artifacts (`KReset` / `KHandle`), not source-constructible.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Cont
{
    /// An argument frame `v :: K`: applies the function in focus to the
    /// **already-evaluated** argument value (the runtime image of
    /// [`crate::syntax::Stack::Arg`]; ADR-50 Decision C — arguments evaluate
    /// eagerly to an [`RtValue`] when the frame is pushed).
    Arg(
        /// The applied argument runtime value.
        Rc<RtValue>,
    ),
    /// A bind frame `(x. u) :: K`: receives a returned value, binds `x`, and
    /// runs `u` under the captured value environment (the runtime image of
    /// [`crate::syntax::Stack::Bind`]; ADR-50 Decision C).
    Bind(
        /// The binder `x` receiving the returned value.
        String,
        /// The continuation `u` run after the bind.
        Rc<Comp>,
        /// The value environment `u` runs under (the bind-site environment;
        /// `x` is added innermost when the bind fires).
        Env,
    ),
    /// A projection frame `prjᵢ :: K`: selects a component of the lazy pair in
    /// focus (the runtime image of [`crate::syntax::Stack::Prj`]).
    Prj(
        /// Which component is projected.
        Side,
    ),
    /// A delimiter frame `KReset`: the boundary a `shift` captures up to
    /// (`effects-control-shell.md` §2.2; runtime-only). Transparent to a
    /// returning terminal (popped on pass-through).
    Reset,
    /// A handler frame `KHandle`: the deep effect handler a `perform` searches
    /// for (`effects-control-shell.md` §1.1; runtime-only). Deep (ADR-33 D4):
    /// the clause body runs *below* the handler, while the captured
    /// continuation `k` carries a copy of the handler (in [`ContEnv`]), so a
    /// `resume k …` re-enters it — see [`drive_perform`].
    Handle
    {
        /// The handled effect signature `E`.
        sig: EffectSig,
        /// The return clause `ret x ⇒ t_ret`.
        ret: (String, Rc<Comp>),
        /// The operation clauses `opᵢ p k ⇒ tᵢ`.
        ops: Vec<OpClause>,
        /// The value environment the return / operation clauses run under (the
        /// handler-site environment; ADR-50 Decision C).
        env: Env,
    },
}

/// A **defined** runtime halt (ADR-34 D4): a typed outcome that is *not* an
/// undefined stuck.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Blame
{
    /// A gradual hole reached an elimination, so it has no value to produce
    /// (the `Canonicity.normalize` blame arm; A2.2).
    Hole,
    /// A `shift` reached no enclosing `KReset` (or its capture would cross a
    /// handler) — the transitional outcome for the over-accepted
    /// escaping-`shift` set (ADR-34 D5); the deferred conservative restriction
    /// will reject these statically.
    ShiftNoReset,
    /// A `perform` reached no matching `KHandle` (or its capture would cross a
    /// delimiter / unrelated handler — the v0 single-handler scope; see the
    /// module's design note).
    PerformNoHandler,
}

/// An **undefined** stuck configuration: reachable only on an ill-typed input.
///
/// The operational soundness oracle pins that a closed well-typed `F A`
/// computation never reaches one (ADR-34 D4).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StuckReason
{
    /// A non-function terminal met an argument frame, or a function met a
    /// non-argument frame — an ill-typed application.
    AppliedNonFunction,
    /// A non-returner terminal met a bind frame, or `ret v` met an argument /
    /// projection frame — an ill-typed sequencing.
    SequencedNonReturner,
    /// `force` was applied to a value that is not a thunk.
    ForcedNonThunk,
    /// `case` scrutinized a value that is not an injection.
    CasedNonSum,
    /// A declared-data `case` scrutinized a value that is not a matching
    /// constructor — a non-`Ctor` value, or a `Ctor` whose `tag` is out of the
    /// arm range (an ill-typed / non-exhaustive elimination; ADR-80).
    DataCasedNonCtor,
    /// A list-case scrutinized a value that is not a list (ADR-40 D4).
    ListCasedNonList,
    /// `split` scrutinized a value that is not a pair.
    SplitNonProduct,
    /// A projection met a focus that is not a lazy pair, or a lazy pair met a
    /// non-projection frame.
    ProjectedNonPair,
    /// A record projection `r.ℓ` scrutinized a value that is not a record
    /// (ADR-45 D4).
    RecordProjNonRecord,
    /// A record projection `r.ℓ` scrutinized a record that has no field `ℓ`
    /// (ADR-45 D4).
    RecordProjMissingField,
    /// `resume` was applied to a value that is not a reified stack.
    ResumedNonStack,
    /// The identity eliminator `Walk` scrutinized a value that is not a `here`
    /// (ADR-76) — an ill-typed elimination, mirroring [`Self::CasedNonSum`]. On
    /// a well-typed closed program the scrutinee is always `here(v)`
    /// (canonicity of the identity type), so this is reachable only on
    /// ill-typed input.
    WalkOnNonHere,
    /// A control or effect form reached the recursive reference evaluator
    /// ([`eval_comp`]), which realizes the pure spine only (the CEK machine
    /// handles these); the differential never feeds the reference one.
    UnsupportedByReference,
    /// A CEK closure body id did not resolve in its arena. This indicates a
    /// corrupted transitional closure adapter, not a well-typed source term.
    InvalidClosureBody,
    /// The continuation-key allocator exhausted its atom identity space while
    /// α-renaming a captured continuation binder.
    FreshContinuationNameExhausted,
    /// The step budget was exhausted (a safety net for [`run`]; not reachable
    /// on the bounded, non-recursive v0 fragment — see [`STEP_BUDGET`]).
    StepLimit,
}

/// The final outcome of an evaluation, shared by [`eval_comp`] and [`run`] so
/// the two are directly comparable (ADR-9 differential).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Eval
{
    /// A terminal computation — a value-producing whnf (`ret v`, `λx. t`, or a
    /// lazy pair) reached at the empty continuation.
    Value(
        /// The terminal computation.
        Comp,
    ),
    /// A defined runtime halt (a gradual hole or a control blame).
    Blame(
        /// Which defined halt.
        Blame,
    ),
    /// An undefined stuck configuration (ill-typed input).
    Stuck(
        /// Why the configuration is stuck.
        StuckReason,
    ),
}

/// One step of the CEK machine: a successor state, or a final outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Outcome
{
    /// One more state.
    Step(
        /// The successor state.
        State,
    ),
    /// A terminal outcome (a value, a blame, or a stuck).
    Final(
        /// The final evaluation outcome.
        Eval,
    ),
}

/// A **continuation environment**: the captured runtime continuations bound by
/// the binders `shift` / `perform` introduce, keyed by a **fresh,
/// machine-unique name** ([`fresh_name`]) the capture α-renames the binder to.
///
/// A captured continuation is a *runtime* continuation (a slice of [`Cont`]
/// frames that may carry the reinstalled delimiter / handler — the deep
/// discipline, ADR-33 D4), not a structural [`crate::syntax::Stack`]: it cannot
/// be represented as the source-level `stk K` value, so `shift` / `perform`
/// bind it here rather than substituting it, and [`Comp::Resume`] resolves a
/// continuation *variable* against this environment (a source-level `stk K`
/// value still splices its structural stack directly). Both inhabit the type
/// `Stk(B, C)`; this is the runtime representation.
///
/// **α-renaming is load-bearing.** The binder is renamed to a fresh name unique
/// to the run *before* the body runs, so two captures can never collide even
/// when the source reuses one name (e.g. nested handler clauses both binding
/// `k`) — keying by the source name would be dynamic scoping, not
/// capture-avoiding substitution. Resumption is multi-shot in v0 (`Σ` is
/// vacuous) and a captured continuation may be resumed long after its capture,
/// so an entry is **read, never popped** (a captured continuation's own
/// `resume` references must stay resolvable); the fresh names keep the keyspace
/// collision-free.
///
/// Distinct from the value environment [`Env`] (source binders ↦ [`RtValue`]s,
/// ADR-50 Decision C): this one maps `shift` / `perform` continuation binders
/// to captured runtime continuations.
type ContEnv = Vec<(String, Rc<[Cont]>)>;

/// A read-only **prelude binding-environment**: top-level names bound to
/// values, consulted at a `Force(Var …)` miss so a name-bound builtin resolves
/// (ADR-42).
///
/// Each bound value is a thunk wrapping a [`Comp::Native`]; forcing the name
/// focuses the native computation. Distinct from [`Env`] (which keys source
/// value binders to runtime values): the prelude is fixed for a run, shared
/// (`Rc`), and only ever **read** by the machine — never written. The empty
/// prelude ([`Prelude::empty`]) is the v0 default ([`run_comp`]); the REPL
/// session supplies the operator / module prelude ([`run_comp_with_prelude`]).
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Prelude(Rc<[(String, Value)]>);

impl Prelude
{
    /// The empty prelude — no name resolves (the default for [`State::new`] /
    /// [`run_comp`], preserving the no-prelude v0 semantics).
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self::from_bindings(Vec::new())
    }

    /// Builds a prelude from `bindings` (name → value, each value a thunk
    /// wrapping a native builtin).
    #[inline]
    #[must_use]
    pub fn from_bindings(bindings: Vec<(String, Value)>) -> Self
    {
        Self(bindings.into())
    }

    /// Looks `name` up, returning the most-recently-bound value (later bindings
    /// shadow earlier ones, as [`crate::ctx::Ctx`]).
    #[inline]
    #[must_use]
    fn lookup<'source, N>(
        &self,
        name: N,
    ) -> Option<&Value>
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        self.0
            .iter()
            .rev()
            .find(|&entry| {
                let (ref entry_name, _) = *entry;
                entry_name.as_str() == name.as_ref()
            })
            .map(|entry| {
                let (_, ref entry_value) = *entry;
                entry_value
            })
    }
}

/// The complete CEK-machine state `⟨t | ρ | K⟩`: inspectable, cloneable,
/// resumable (ADR-50 Decision C — an environment `ρ` replaces eager
/// substitution on the hot path).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State
{
    /// The computation in focus.
    focus: Comp,
    /// The value environment `ρ` the focus's free variables resolve under
    /// (ADR-50 Decision C).
    env: Env,
    /// The continuation; the *last* element is the top (the frame nearest the
    /// focus).
    cont: Vec<Cont>,
    /// The captured-continuation environment (`shift` / `perform` binders).
    contenv: ContEnv,
    /// The continuation-key allocator, for α-renaming captured-continuation
    /// binders ([`fresh_name`]); a monotone [`Gensym`] over the
    /// [`GandrSort::ContKey`] atom sort (ADR-41), so every captured
    /// continuation gets a run-unique key.
    gensym: Gensym<GandrSort>,
    /// Monotone step counter.
    steps: u64,
    /// The read-only prelude binding-environment consulted on a `Force(Var …)`
    /// miss (ADR-42); empty in the v0 default.
    prelude: Prelude,
}

impl State
{
    /// The initial state driving `comp` over the empty continuation, with no
    /// prelude (the no-prelude v0 default).
    #[inline]
    #[must_use]
    pub fn new(comp: Comp) -> Self
    {
        Self::with_prelude(comp, Prelude::empty())
    }

    /// The initial state driving `comp` over the empty continuation under a
    /// prelude binding-environment (ADR-42): a `Force(Var …)` that misses the
    /// ordinary thunk path resolves the name against `prelude`.
    #[inline]
    #[must_use]
    pub fn with_prelude(
        comp: Comp,
        prelude: Prelude,
    ) -> Self
    {
        Self {
            focus: comp,
            env: Env::empty(),
            cont: Vec::new(),
            contenv: Vec::new(),
            gensym: Gensym::new(GandrSort::ContKey),
            steps: 0,
            prelude,
        }
    }

    /// The computation currently in focus.
    #[inline]
    #[must_use]
    pub fn focus(&self) -> &Comp
    {
        &self.focus
    }

    /// The value environment the focus resolves under (ADR-50 Decision C).
    #[inline]
    #[must_use]
    pub fn env(&self) -> &Env
    {
        &self.env
    }

    /// Reconstructs the configuration `⟨t | ρ | K⟩` as a **closed** computation
    /// term — the focus closed under its environment, wrapped by the
    /// readback of each continuation frame (outermost-last). The inverse of the
    /// machine's decomposition, used to type a mid-run state (the per-step
    /// subject-reduction oracle) now that the machine is environment-based:
    /// closing the focus and each frame's captured environment produces a
    /// substitution-equivalent term (ADR-50 Decision C/D; supersedes the
    /// conformance suite's old `plug`).
    ///
    /// # Contract
    /// - ensures: returns a closed computation equivalent to the configuration
    ///   under the environment semantics; each `Bind` / `Handle` clause body is
    ///   closed under its captured environment with its own binders shielded.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn reconstruct(&self) -> Comp
    {
        let mut result = close_comp(&self.focus, &self.env);
        for frame in self.cont.iter().rev() {
            result = match *frame {
                | Cont::Arg(ref value) => Comp::App(Rc::new(result), Rc::new(quote_value(value))),
                | Cont::Bind(ref binder, ref body, ref env) => Comp::Bind(
                    Rc::new(result),
                    binder.clone(),
                    Rc::new(close_comp_shielding(body, env, &[binder.as_str()])),
                ),
                | Cont::Prj(side) => match side {
                    | Side::Fst => Comp::prj1(result),
                    | Side::Snd => Comp::prj2(result),
                },
                | Cont::Reset => Comp::reset(result),
                | Cont::Handle {
                    ref sig,
                    ref ret,
                    ref ops,
                    ref env,
                } => Comp::Handle {
                    sig: Box::new(sig.clone()),
                    scrutinee: Rc::new(result),
                    ret: (
                        ret.0.clone(),
                        Rc::new(close_comp_shielding(&ret.1, env, &[ret.0.as_str()])),
                    ),
                    ops: ops
                        .iter()
                        .map(|clause| OpClause {
                            op: clause.op.clone(),
                            payload: clause.payload.clone(),
                            resume: clause.resume.clone(),
                            body: Rc::new(close_comp_shielding(&clause.body, env, &[
                                clause.payload.as_str(),
                                clause.resume.as_str(),
                            ])),
                        })
                        .collect(),
                },
            };
        }
        result
    }

    /// The continuation; the *last* element is the top.
    #[inline]
    #[must_use]
    pub fn cont(&self) -> &[Cont]
    {
        &self.cont
    }

    /// The current continuation depth.
    #[inline]
    #[must_use]
    pub fn depth(&self) -> StackDepth
    {
        self.cont.len().into()
    }

    /// The number of steps taken so far.
    #[inline]
    #[must_use]
    pub fn steps(&self) -> MachineStepCount
    {
        self.steps.into()
    }

    /// The **host-interceptable** operation the machine would next perform with
    /// no in-term handler to catch it, as an *owned* payload ([`HostOp`]) — the
    /// host-effect seam (ADR-35 D4).
    ///
    /// Returns `Some` exactly when the focus is a `perform sig op v` that no
    /// source-level handler claims across the structural prefix —
    /// `capture_to_handler` finds no enclosing [`Cont::Handle`] declaring `op`
    /// before an intervening [`Cont::Reset`] or non-matching [`Cont::Handle`],
    /// i.e. the next [`step`] would blame [`Blame::PerformNoHandler`]. The seam
    /// intercepts exactly this set (see [`HostHandler`] for the invariant).
    ///
    /// The payload is [`strip_annot`]-peeled and cloned, so the returned
    /// [`HostOp`] borrows nothing from the state — it is robust across
    /// machine/arena representation changes, where a payload may be an owned
    /// node with no `&Value` to lend. The seam is expressed over the public
    /// [`EffectSig`] surface and the operation *name* only, never a
    /// continuation frame, so it wraps [`run`] / [`step`] rather than
    /// reimplementing capture.
    ///
    /// # v0 decision (a `reset` is transparent to the host; reversible)
    ///
    /// A `perform` whose in-term capture is blocked by an intervening
    /// [`Cont::Reset`] (a source-level `reset`) still reports here:
    /// `capture_to_handler` returns `None` on a `Reset`, so the host — the
    /// ambient handler *outside* every delimiter — intercepts it (INTERCEPT
    /// semantics). A test pins this choice.
    ///
    /// # Contract
    /// - ensures: `Some(HostOp { sig, op, payload })` with an owned,
    ///   annotation-stripped `payload` iff the focus is a handler-less
    ///   `perform`; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn pending_host_op(&self) -> Option<HostOp>
    {
        match self.focus {
            | Comp::Perform(ref sig, ref op, ref arg)
                if capture_to_handler(&self.cont, op).is_none() =>
            {
                // Evaluate the payload under the focus environment and read it
                // back as an owned, annotation-stripped value — the host sees a
                // self-contained value over the public surface (ADR-50 Decision
                // C: the payload's free variables resolve against the CEK
                // environment, not a substituted term).
                let evaluated = eval_value(arg, &self.env);
                Some(HostOp {
                    sig: sig.as_ref().clone(),
                    op: op.clone(),
                    payload: quote_value(rt_peel(&evaluated)),
                })
            },
            | _ => None,
        }
    }

    /// Resumes a host-intercepted operation (see [`Self::pending_host_op`])
    /// with the host's `reply`, resuming the whole current continuation from
    /// below it (the deep discipline; see [`HostHandler`] for the invariant).
    ///
    /// The focus becomes `ret reply`; the continuation, captured-continuation
    /// environment, key allocator, and prelude are retained unchanged — no
    /// handler-truncation, since the host sits below the entire continuation,
    /// so a later `perform` in it is intercepted again (the deep
    /// discipline). The step counter advances by one.
    ///
    /// # Contract
    /// - requires: intended to be called on a state whose
    ///   [`Self::pending_host_op`] is `Some` (the focus is the intercepted
    ///   `perform`); the returned state discards the focus regardless.
    /// - ensures: returns the successor state with `focus = ret reply` (under
    ///   the empty environment — the host reply is a closed value), the same
    ///   `cont` / `contenv` / `gensym` / `prelude`, and `steps` saturating-
    ///   incremented.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resume_host(
        self,
        reply: Value,
    ) -> Self
    {
        let Self {
            cont,
            contenv,
            gensym,
            steps,
            prelude,
            ..
        } = self;
        Self {
            focus: Comp::ret(reply),
            // The host reply is a closed value, so the empty environment is
            // sufficient; the awaiting frames carry their own environments.
            env: Env::empty(),
            cont,
            contenv,
            gensym,
            steps: steps.saturating_add(1),
            prelude,
        }
    }
}

/// The maximum number of [`step`]s [`run`] takes before halting with
/// [`StuckReason::StepLimit`].
///
/// The v0 fragment has no recursion / fixpoint and substitutes only closed
/// values, so evaluation of a finite term terminates; this is a pure safety net
/// against a pathological (necessarily ill-typed) input, set far above any term
/// the test fragment produces.
///
/// **Single source of truth.** This is the one budget constant the whole
/// workspace shares: `run` / `run_state_with_host` / `force_probe` here, the
/// `gandr-shell` and `gandr-ffi` drivers (which consume it directly rather than
/// mirroring it), and the sequent L machine (`gandr-sequent`, which runs the
/// same budget so its step-count net stays parity with this machine's under the
/// shared bound). Keeping one public constant is the L1 de-duplication of the
/// two former driver mirrors (`proposal-sequent-kernel.md` §9, phase L1).
pub const STEP_BUDGET: u64 = 1_000_000;

/// Reads a terminal computation back to a closed source term at the empty
/// continuation (ADR-50 Decision C/D): a returner reads its value through the
/// value domain (`quote_value ∘ eval_value`, preserving annotations); a
/// function / lazy pair / native closes its free variables under the
/// environment (a native is already closed).
///
/// # Contract
/// - ensures: returns the terminal `comp` closed under `env`, structurally
///   identical to the substitution reference's result.
/// - panics: none.
fn read_terminal(
    comp: &Comp,
    env: &Env,
) -> Comp
{
    match *comp {
        | Comp::Ret(ref value) => Comp::ret(quote_value(&eval_value(value, env))),
        | _ => close_comp(comp, env),
    }
}

/// Meets a **terminal** computation, under its value environment, with the top
/// of the continuation: consume the matching frame (extending the environment
/// rather than substituting), or — at the empty continuation — read the
/// terminal back (ADR-50 Decision C).
///
/// # Contract
/// - requires: `comp` is a terminal whnf (`is_terminal(&comp)`).
/// - ensures: returns the next focus and its environment over the mutated
///   continuation, or a final value (empty continuation) / stuck (an ill-typed
///   terminal/frame pairing).
/// - panics: none.
fn meet(
    comp: Comp,
    env: Env,
    cont: &mut Vec<Cont>,
) -> Transition
{
    // The `Cont::Reset` delimiter is transparent to a returning terminal: it is
    // popped and the terminal re-meets the next frame. A loop (not a tail
    // self-call) so an adversarially deep nest of `reset` delimiters is consumed
    // iteratively rather than recursing the host stack (ADR-47); `comp` / `env`
    // are moved only on the exiting (returning) paths and stay live across the
    // `Reset` back-edge.
    loop {
        let Some(frame) = cont.pop()
        else {
            // The empty continuation: read the terminal back under its
            // environment.
            return Transition::Final(Eval::Value(read_terminal(&comp, &env)));
        };
        // Dispatch on the *frame*, then check the terminal's constructor, so the
        // stuck classification is the elimination's (the same view the recursive
        // [`eval_comp`] takes — keeping the two faces' outcomes equal).
        match frame {
            // A bind frame consumes a returner: `ret v` binds `v` (evaluated
            // under the terminal's environment) in the bind-site environment and
            // runs `u`.
            | Cont::Bind(var, body, body_env) => {
                return match comp {
                    | Comp::Ret(ref value) => {
                        let bound = eval_value(value, &env);
                        Transition::Focus(unrc_comp(body), body_env.extend(var, bound))
                    },
                    | _ => Transition::Final(Eval::Stuck(StuckReason::SequencedNonReturner)),
                };
            },
            // An argument frame consumes a function: `λx. t` β-reduces by binding
            // `x` in the current (function's) environment; a native accumulates
            // the read-back argument and, once saturated, reduces in Rust to a
            // closed term (ADR-42). Refocusing rather than recursing keeps the
            // host stack flat.
            | Cont::Arg(arg) => {
                return match comp {
                    | Comp::Abs(name, _, body) => match Closure::from_body(env, body.as_ref()) {
                        | Some(closure) => match closure_body(&closure) {
                            | Some(closure_comp) => {
                                Transition::Focus(closure_comp, closure.env.extend(name, arg))
                            },
                            | None => {
                                Transition::Final(Eval::Stuck(StuckReason::InvalidClosureBody))
                            },
                        },
                        | None => Transition::Final(Eval::Stuck(StuckReason::InvalidClosureBody)),
                    },
                    | Comp::Native { prim, mut args } => {
                        args.push(Rc::new(quote_value(&arg)));
                        if args.len() >= usize::from(prim.arity()) {
                            Transition::Focus(prim.apply(&args), Env::empty())
                        }
                        else {
                            Transition::Focus(Comp::Native { prim, args }, Env::empty())
                        }
                    },
                    | _ => Transition::Final(Eval::Stuck(StuckReason::AppliedNonFunction)),
                };
            },
            // A projection frame consumes a lazy pair: select the component,
            // which runs under the same (lazy pair's) environment.
            | Cont::Prj(side) => {
                return match comp {
                    | Comp::With(fst, snd) => Transition::Focus(
                        unrc_comp(match side {
                            | Side::Fst => fst,
                            | Side::Snd => snd,
                        }),
                        env,
                    ),
                    | _ => Transition::Final(Eval::Stuck(StuckReason::ProjectedNonPair)),
                };
            },
            // A handler frame consumes a returner scrutinee: `ret v` runs the
            // return clause under the handler-site environment extended with the
            // bound value (the handler is discharged).
            | Cont::Handle {
                ret,
                env: handle_env,
                ..
            } => {
                return match comp {
                    | Comp::Ret(ref value) => {
                        let (var, body) = ret;
                        let bound = eval_value(value, &env);
                        Transition::Focus(unrc_comp(body), handle_env.extend(var, bound))
                    },
                    | _ => Transition::Final(Eval::Stuck(StuckReason::SequencedNonReturner)),
                };
            },
            // The delimiter is transparent to a returning terminal: pop it and
            // re-meet the next frame on the next iteration (ADR-47).
            | Cont::Reset => {},
        }
    }
}

/// The result of a single transition: a new focus with its environment
/// (continue) or a final outcome.
enum Transition
{
    /// Continue with a new focus and the environment it runs under, over the
    /// (mutated) continuation.
    Focus(Comp, Env),
    /// Halt with this outcome.
    Final(Eval),
}

/// The result of inspecting a memoized thunk before forcing it.
enum PreparedForce
{
    /// Reuse an already-cached whnf.
    Whnf(Comp, Env),
    /// Run the body inline against the caller's continuation.
    Inline,
    /// Probe the closure body under an empty continuation before deciding.
    Probe(Closure, MemoCell),
}

/// Inspect and, for an unforced thunk, black-hole the memo cell before probing.
fn prepare_force(
    closure: &Closure,
    memo: &MemoCell,
) -> PreparedForce
{
    {
        // Scoped read: reuse a cached whnf, or fall back on the black hole.
        match *memo.0.borrow() {
            | ThunkMemo::Forced { ref comp, ref env } => {
                return PreparedForce::Whnf(comp.clone(), env.clone());
            },
            | ThunkMemo::InProgress => return PreparedForce::Inline,
            | ThunkMemo::Unforced => {},
        }
    }
    *memo.0.borrow_mut() = ThunkMemo::InProgress;
    PreparedForce::Probe(closure.clone(), memo.clone())
}

/// One self-contained thunk-probe machine.
struct ProbeMachine
{
    /// The computation currently under evaluation.
    focus: Comp,
    /// The value environment the focus evaluates under.
    env: Env,
    /// The machine's continuation stack.
    cont: Vec<Cont>,
    /// The captured-continuation bindings minted so far (probe-local keys).
    contenv: ContEnv,
    /// The fresh-name allocator for captured-continuation binders.
    gensym: Gensym<GandrSort>,
    /// The steps taken so far, checked against [`STEP_BUDGET`].
    steps: u64,
}

impl ProbeMachine
{
    /// Start probing `closure` from an empty continuation.
    #[must_use]
    fn from_closure(closure: &Closure) -> Option<Self>
    {
        Some(Self {
            focus: closure_body(closure)?,
            env: closure.env.clone(),
            cont: Vec::new(),
            contenv: Vec::new(),
            gensym: Gensym::new(GandrSort::ContKey),
            steps: 0,
        })
    }
}

/// A parent probe suspended while a nested thunk is probed.
struct SuspendedProbe
{
    /// The suspended parent probe machine.
    machine: ProbeMachine,
    /// The nested thunk closure under probe.
    closure: Closure,
    /// The nested thunk's memo cell, updated with the probe outcome.
    memo: MemoCell,
}

/// Probe-local transition; `Probe` is the trampoline edge for nested forces.
enum ProbeTransition
{
    /// Continue the probe with a new focus and environment.
    Focus(Comp, Env),
    /// Halt the probe (a blame, a stuck state, or a terminal at a non-empty
    /// continuation).
    Final,
    /// Suspend the current probe and probe a nested thunk force.
    Probe(Closure, MemoCell),
}

/// Probes a thunk closure's body to its weak-head normal form on an explicit
/// stack of self-contained machines (ADR-50): returns the terminal computation
/// and its environment when the body reduces PURELY — reaching a terminal with
/// no captured continuation left behind — and `None` otherwise (it blames /
/// gets stuck on an escaping effect, captured a continuation whose keys are
/// probe-local, or exhausted the step budget). On `None` the caller runs the
/// body inline against the real continuation, preserving effect semantics (the
/// fast → slow fallback).
///
/// A returner / λ / lazy pair / native whnf is continuation-independent for a
/// pure body, so the cached terminal is exactly the one the real continuation
/// would meet — memoization is observationally transparent.
///
/// # Contract
/// - ensures: `Some((comp, env))` for a pure, continuation-free terminal;
///   `None` otherwise.
/// - panics: none.
/// # Termination
/// - reason: evaluator drives nested thunk probes with an explicit machine
///   stack, never host recursion.
/// - measure: pending probe machines plus their finite runtime syntax and
///   continuations.
/// - boundedness: programs, continuations, and probe machines are finite Rust
///   values bounded by [`STEP_BUDGET`] per probe.
/// - input recursion: none.
#[cfg_attr(
    dylint_lib = "non_local_effect_before_unhandled_error",
    allow(
        unknown_lints,
        non_local_effect_before_unhandled_error,
        reason = "an error carries the suspended probe machine and is immediately resumed, not discarded; a systematic failure-atomic arena audit is tracked separately"
    )
)]
fn force_probe(
    closure: &Closure,
    prelude: &Prelude,
) -> Option<(Comp, Env)>
{
    let mut current = ProbeMachine::from_closure(closure)?;
    let mut suspended: Vec<SuspendedProbe> = Vec::new();
    loop {
        if current.steps >= STEP_BUDGET {
            match resume_completed_probe(&mut suspended, None) {
                | Ok(result) => return result,
                | Err(machine) => {
                    current = *machine;
                    continue;
                },
            }
        }
        if bool::from(is_terminal(&current.focus)) {
            if current.cont.is_empty() {
                let result = current
                    .contenv
                    .is_empty()
                    .then_some((current.focus, current.env));
                match resume_completed_probe(&mut suspended, result) {
                    | Ok(result) => return result,
                    | Err(machine) => current = *machine,
                }
                continue;
            }
            match meet(current.focus, current.env, &mut current.cont) {
                | Transition::Focus(next_focus, next_env) => {
                    current.focus = next_focus;
                    current.env = next_env;
                    current.steps = current.steps.saturating_add(1);
                },
                | Transition::Final(_) => match resume_completed_probe(&mut suspended, None) {
                    | Ok(result) => return result,
                    | Err(machine) => current = *machine,
                },
            }
            continue;
        }
        let ProbeMachine {
            focus,
            env,
            mut cont,
            mut contenv,
            mut gensym,
            steps,
        } = current;
        match probe_drive(focus, env, &mut cont, &mut contenv, &mut gensym, prelude) {
            | ProbeTransition::Focus(next_focus, next_env) => {
                current = ProbeMachine {
                    focus: next_focus,
                    env: next_env,
                    cont,
                    contenv,
                    gensym,
                    steps: steps.saturating_add(1),
                };
            },
            | ProbeTransition::Final => match resume_completed_probe(&mut suspended, None) {
                | Ok(result) => return result,
                | Err(machine) => current = *machine,
            },
            | ProbeTransition::Probe(probe_closure, probe_memo) => {
                let nested = ProbeMachine::from_closure(&probe_closure);
                suspended.push(SuspendedProbe {
                    machine: ProbeMachine {
                        focus: Comp::ret(Value::Unit),
                        env: Env::empty(),
                        cont,
                        contenv,
                        gensym,
                        steps: steps.saturating_add(1),
                    },
                    closure: probe_closure,
                    memo: probe_memo,
                });
                match nested {
                    | Some(machine) => current = machine,
                    | None => match resume_completed_probe(&mut suspended, None) {
                        | Ok(result) => return result,
                        | Err(machine) => current = *machine,
                    },
                }
            },
        }
    }
}

/// Resume the parent probe after a nested probe completes.
///
/// # Contract
/// - ensures: `Ok(result)` once every suspended probe is resolved (`result` is
///   the innermost probe's outcome); `Err(machine)` with the parent machine to
///   continue — re-focused on the nested probe's terminal when it was purely
///   reducible, on the inline body otherwise (the boxed payload keeps the `Err`
///   variant small). The nested thunk's memo cell records the outcome in both
///   cases.
/// - panics: none.
fn resume_completed_probe(
    suspended: &mut Vec<SuspendedProbe>,
    mut result: Option<(Comp, Env)>,
) -> Result<Option<(Comp, Env)>, Box<ProbeMachine>>
{
    loop {
        let Some(suspended_probe) = suspended.pop()
        else {
            return Ok(result);
        };
        match result {
            | Some((comp, env)) => {
                *suspended_probe.memo.0.borrow_mut() = ThunkMemo::Forced {
                    comp: comp.clone(),
                    env: env.clone(),
                };
                let mut machine = suspended_probe.machine;
                machine.focus = comp;
                machine.env = env;
                return Err(Box::new(machine));
            },
            | None => {
                *suspended_probe.memo.0.borrow_mut() = ThunkMemo::Unforced;
                if let Some(body) = closure_body(&suspended_probe.closure) {
                    let mut machine = suspended_probe.machine;
                    machine.focus = body;
                    machine.env = suspended_probe.closure.env.clone();
                    return Err(Box::new(machine));
                }
                result = None;
            },
        }
    }
}

/// Probe-local copy of [`drive`] whose force case trampolines nested probes
/// instead of calling back into [`force_thunk`] on the host stack.
fn probe_drive(
    comp: Comp,
    env: Env,
    cont: &mut Vec<Cont>,
    contenv: &mut ContEnv,
    gensym: &mut Gensym<GandrSort>,
    prelude: &Prelude,
) -> ProbeTransition
{
    match comp {
        | Comp::App(head, arg) => {
            cont.push(Cont::Arg(eval_value(&arg, &env)));
            ProbeTransition::Focus(unrc_comp(head), env)
        },
        | Comp::Bind(bound, var, body) => {
            cont.push(Cont::Bind(var, body, env.clone()));
            ProbeTransition::Focus(unrc_comp(bound), env)
        },
        | Comp::Prj(side, target) => {
            cont.push(Cont::Prj(side));
            ProbeTransition::Focus(unrc_comp(target), env)
        },
        | Comp::Force(thunked) => {
            let evaluated = eval_value(&thunked, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Thunk(_, ref closure, ref memo) => match prepare_force(closure, memo) {
                    | PreparedForce::Whnf(next_focus, next_env) => {
                        ProbeTransition::Focus(next_focus, next_env)
                    },
                    | PreparedForce::Inline => match closure_body(closure) {
                        | Some(body) => ProbeTransition::Focus(body, closure.env.clone()),
                        | None => ProbeTransition::Final,
                    },
                    | PreparedForce::Probe(probe_closure, probe_memo) => {
                        ProbeTransition::Probe(probe_closure, probe_memo)
                    },
                },
                | RtValue::Var(ref name) => match prelude.lookup(name) {
                    | Some(&Value::Thunk(_, ref body)) => {
                        ProbeTransition::Focus(body.as_ref().clone(), Env::empty())
                    },
                    | _ => ProbeTransition::Final,
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::Case(scrut, arm_fst, arm_snd) => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Inj(side, ref payload) => {
                    let (var, body) = match side {
                        | Side::Fst => arm_fst,
                        | Side::Snd => arm_snd,
                    };
                    let extended = env.extend(var, Rc::clone(payload));
                    ProbeTransition::Focus(unrc_comp(body), extended)
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::DataCase(scrut, arms) => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Ctor {
                    tag, ref payload, ..
                } => match arms.into_iter().nth(tag) {
                    | Some((var, body)) => {
                        let extended = env.extend(var, Rc::clone(payload));
                        ProbeTransition::Focus(unrc_comp(body), extended)
                    },
                    | None => ProbeTransition::Final,
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::Split {
            scrut,
            fst_name,
            snd_name,
            body,
            ..
        } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Pair(ref fst, ref snd) => {
                    let extended = env
                        .extend(fst_name, Rc::clone(fst))
                        .extend(snd_name, Rc::clone(snd));
                    ProbeTransition::Focus(unrc_comp(body), extended)
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::RecordProj { record, label } => {
            let evaluated = eval_value(&record, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Record(ref fields) => match fields.get(label.as_str()) {
                    | Some(value) => {
                        ProbeTransition::Focus(Comp::ret(quote_value(value)), Env::empty())
                    },
                    | None => ProbeTransition::Final,
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::ListCase {
            scrut,
            nil,
            head,
            tail,
            cons,
        } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::List(ref elements) => match elements.split_first() {
                    | None => ProbeTransition::Focus(unrc_comp(nil), env),
                    | Some((first, rest)) => {
                        let tail_value = Rc::new(RtValue::List(rest.to_vec()));
                        let extended = env.extend(head, Rc::clone(first)).extend(tail, tail_value);
                        ProbeTransition::Focus(unrc_comp(cons), extended)
                    },
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::Dup(value) => {
            let resolved = quote_value(&eval_value(&value, &env));
            ProbeTransition::Focus(
                Comp::ret(Value::pair(resolved.clone(), resolved)),
                Env::empty(),
            )
        },
        | Comp::Drop(_) => ProbeTransition::Focus(Comp::ret(Value::Unit), Env::empty()),
        | Comp::Hole(_) => ProbeTransition::Final,
        | Comp::Reset(body) => {
            cont.push(Cont::Reset);
            ProbeTransition::Focus(unrc_comp(body), env)
        },
        | Comp::Shift(k, body) => match drive_shift(&k, &body, env, cont, contenv, gensym) {
            | Transition::Focus(next_focus, next_env) => {
                ProbeTransition::Focus(next_focus, next_env)
            },
            | Transition::Final(_) => ProbeTransition::Final,
        },
        | Comp::Perform(_sig, op, arg) => {
            match drive_perform(&op, &arg, &env, cont, contenv, gensym) {
                | Transition::Focus(next_focus, next_env) => {
                    ProbeTransition::Focus(next_focus, next_env)
                },
                | Transition::Final(_) => ProbeTransition::Final,
            }
        },
        | Comp::Handle {
            sig,
            scrutinee,
            ret,
            ops,
        } => {
            cont.push(Cont::Handle {
                sig: *sig,
                ret,
                ops,
                env: env.clone(),
            });
            ProbeTransition::Focus(unrc_comp(scrutinee), env)
        },
        | Comp::Resume(reified, fed) => {
            let evaluated = eval_value(&reified, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Var(ref name) => match env_lookup(contenv, name) {
                    | Some(captured) => {
                        cont.extend_from_slice(captured.as_ref());
                        ProbeTransition::Focus(unrc_comp(fed), env)
                    },
                    | None => ProbeTransition::Final,
                },
                | RtValue::Stk(ref captured, ref stack_env) => {
                    splice(captured, stack_env, cont);
                    ProbeTransition::Focus(unrc_comp(fed), env)
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::Walk { scrut, base, .. } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Here(ref witness) => {
                    let extended = env.extend(base.x, Rc::clone(witness));
                    ProbeTransition::Focus(unrc_comp(base.body), extended)
                },
                | RtValue::Hole(_) | _ => ProbeTransition::Final,
            }
        },
        | Comp::Ret(_) | Comp::Abs(..) | Comp::With(..) | Comp::Native { .. } => {
            ProbeTransition::Final
        },
    }
}

/// Drives a **non-terminal** computation one step under its value environment
/// (ADR-50 Decision C): decompose it, pushing a frame and re-focusing on a
/// sub-term, or reduce a redex by extending the environment (no substitution).
///
/// # Contract
/// - requires: `comp` is not a terminal whnf (`is_terminal(&comp)` is `false`).
/// - ensures: returns the next focus and its environment over the mutated
///   continuation, or a final blame / stuck.
/// - panics: none.
/// # Termination
/// - reason: evaluator decomposes finite runtime syntax and work stacks.
/// - measure: pending work items or remaining runtime syntax nodes.
/// - boundedness: programs and work stacks are finite Rust values.
/// - input recursion: none.
fn drive(
    comp: Comp,
    env: Env,
    cont: &mut Vec<Cont>,
    contenv: &mut ContEnv,
    gensym: &mut Gensym<GandrSort>,
    prelude: &Prelude,
) -> Transition
{
    match comp {
        | Comp::App(head, arg) => {
            // Evaluate the argument eagerly under the current environment and
            // push it; the head keeps the current environment.
            cont.push(Cont::Arg(eval_value(&arg, &env)));
            Transition::Focus(unrc_comp(head), env)
        },
        | Comp::Bind(bound, var, body) => {
            // The continuation body captures the current environment; the bound
            // computation runs under the same environment.
            cont.push(Cont::Bind(var, body, env.clone()));
            Transition::Focus(unrc_comp(bound), env)
        },
        | Comp::Prj(side, target) => {
            cont.push(Cont::Prj(side));
            Transition::Focus(unrc_comp(target), env)
        },
        | Comp::Force(thunked) => {
            let evaluated = eval_value(&thunked, &env);
            match *rt_peel(&evaluated) {
                // Force a thunk closure (ADR-50 call-by-need): reuse the cached
                // weak-head form, or run the body under the closure's captured
                // environment. Memoization is transparent — an effectful body
                // falls back to running inline against the real continuation.
                | RtValue::Thunk(_, ref closure, ref memo) => {
                    match force_thunk(closure, memo, prelude) {
                        | ForceStep::Whnf(next_focus, next_env) => {
                            Transition::Focus(next_focus, next_env)
                        },
                        | ForceStep::Inline => match closure_body(closure) {
                            | Some(body) => Transition::Focus(body, closure.env.clone()),
                            | None => {
                                Transition::Final(Eval::Stuck(StuckReason::InvalidClosureBody))
                            },
                        },
                    }
                },
                // A free name in force position (ADR-42): resolve it against the
                // prelude — a hit is a thunk wrapping a native builtin, whose
                // body becomes the focus under the empty environment (the
                // builtin is closed); a miss is `ForcedNonThunk`.
                | RtValue::Var(ref name) => match prelude.lookup(name) {
                    | Some(&Value::Thunk(_, ref body)) => {
                        Transition::Focus(body.as_ref().clone(), Env::empty())
                    },
                    | _ => Transition::Final(Eval::Stuck(StuckReason::ForcedNonThunk)),
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::ForcedNonThunk)),
            }
        },
        | Comp::Case(scrut, arm_fst, arm_snd) => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Inj(side, ref payload) => {
                    let (var, body) = match side {
                        | Side::Fst => arm_fst,
                        | Side::Snd => arm_snd,
                    };
                    let extended = env.extend(var, Rc::clone(payload));
                    Transition::Focus(unrc_comp(body), extended)
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::CasedNonSum)),
            }
        },
        // A declared-data case selects the arm at the constructor's `tag` and
        // binds the arm's payload binder to the field-tuple, exactly as `case`
        // selects by `Side` (ADR-80 Decision 3). A non-`Ctor` scrutinee or an
        // out-of-range tag is `DataCasedNonCtor` (an ill-typed / non-exhaustive
        // elimination); a hole blames.
        | Comp::DataCase(scrut, arms) => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Ctor {
                    tag, ref payload, ..
                } => match arms.into_iter().nth(tag) {
                    | Some((var, body)) => {
                        let extended = env.extend(var, Rc::clone(payload));
                        Transition::Focus(unrc_comp(body), extended)
                    },
                    | None => Transition::Final(Eval::Stuck(StuckReason::DataCasedNonCtor)),
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::DataCasedNonCtor)),
            }
        },
        // Split-β is runtime type-erased (ADR-82 D4): the motive is inert, so a
        // `Σ`-typed or motive-bearing split reduces exactly as a product one.
        | Comp::Split {
            scrut,
            fst_name,
            snd_name,
            body,
            ..
        } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Pair(ref fst, ref snd) => {
                    // Bind `fst` then `snd`, so `snd` is innermost: when the two
                    // binders collide the later one wins, matching the typing — exactly as the
                    // reference substitutes `snd` first.
                    let extended = env
                        .extend(fst_name, Rc::clone(fst))
                        .extend(snd_name, Rc::clone(snd));
                    Transition::Focus(unrc_comp(body), extended)
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::SplitNonProduct)),
            }
        },
        // Record projection reduces in place (no frame pushed): `{ℓ=W, …}.ℓ ⟶
        // ret W` (ADR-45 D4). The field is read back to a source value and
        // returned (it is closed, so it re-evaluates trivially).
        | Comp::RecordProj { record, label } => {
            let evaluated = eval_value(&record, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Record(ref fields) => match fields.get(label.as_str()) {
                    | Some(value) => Transition::Focus(Comp::ret(quote_value(value)), Env::empty()),
                    | None => Transition::Final(Eval::Stuck(StuckReason::RecordProjMissingField)),
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::RecordProjNonRecord)),
            }
        },
        // The list eliminator reduces in place: select the `nil` or `cons` arm
        // on the scrutinee's shape, extending the environment (ADR-40 D4). The
        // recursive reference uses the subst-based [`reduce_list_case`]; the CEK
        // path is env-based here.
        | Comp::ListCase {
            scrut,
            nil,
            head,
            tail,
            cons,
        } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::List(ref elements) => match elements.split_first() {
                    | None => Transition::Focus(unrc_comp(nil), env),
                    | Some((first, rest)) => {
                        // Bind `head` then `tail`, so `tail` is innermost (the
                        // later binder wins on collision — the reference
                        // substitutes `tail` first).
                        let tail_value = Rc::new(RtValue::List(rest.to_vec()));
                        let extended = env.extend(head, Rc::clone(first)).extend(tail, tail_value);
                        Transition::Focus(unrc_comp(cons), extended)
                    },
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::ListCasedNonList)),
            }
        },
        // The grade structural ops are erased operationally (ADR-34 D2): `dup v`
        // returns `(v, v)` (v resolved under the environment), `drop v` returns
        // `()`.
        | Comp::Dup(value) => {
            let resolved = quote_value(&eval_value(&value, &env));
            Transition::Focus(
                Comp::ret(Value::pair(resolved.clone(), resolved)),
                Env::empty(),
            )
        },
        | Comp::Drop(_) => Transition::Focus(Comp::ret(Value::Unit), Env::empty()),
        // A computation hole has no value to produce (the gradual blame arm).
        | Comp::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
        | Comp::Reset(body) => {
            cont.push(Cont::Reset);
            Transition::Focus(unrc_comp(body), env)
        },
        | Comp::Shift(k, body) => drive_shift(&k, &body, env, cont, contenv, gensym),
        | Comp::Perform(_sig, op, arg) => drive_perform(&op, &arg, &env, cont, contenv, gensym),
        | Comp::Handle {
            sig,
            scrutinee,
            ret,
            ops,
        } => {
            cont.push(Cont::Handle {
                sig: *sig,
                ret,
                ops,
                env: env.clone(),
            });
            Transition::Focus(unrc_comp(scrutinee), env)
        },
        | Comp::Resume(reified, fed) => {
            let evaluated = eval_value(&reified, &env);
            match *rt_peel(&evaluated) {
                // A captured continuation (bound by `shift` / `perform`):
                // resolve it against the continuation environment and splice the
                // runtime continuation (which may carry the reinstalled
                // delimiter / handler — deep).
                | RtValue::Var(ref name) => match env_lookup(contenv, name) {
                    | Some(captured) => {
                        cont.extend_from_slice(captured.as_ref());
                        Transition::Focus(unrc_comp(fed), env)
                    },
                    | None => Transition::Final(Eval::Stuck(StuckReason::ResumedNonStack)),
                },
                // A source-level `stk K` value: splice its structural stack,
                // resolving its argument values under the stack's environment.
                | RtValue::Stk(ref captured, ref stack_env) => {
                    splice(captured, stack_env, cont);
                    Transition::Focus(unrc_comp(fed), env)
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::ResumedNonStack)),
            }
        },
        // The identity β-rule `walk(here(w), C, (x). c) ↦ c[w/x]` (ADR-76): the
        // motive is runtime-erased, so only the scrutinee and base matter. Bind
        // the diagonal binder `x ↦ w` and focus the base body (env-based, as the
        // `Case` arm); a hole blames; anything else is `WalkOnNonHere`.
        | Comp::Walk { scrut, base, .. } => {
            let evaluated = eval_value(&scrut, &env);
            match *rt_peel(&evaluated) {
                | RtValue::Here(ref witness) => {
                    let extended = env.extend(base.x, Rc::clone(witness));
                    Transition::Focus(unrc_comp(base.body), extended)
                },
                | RtValue::Hole(_) => Transition::Final(Eval::Blame(Blame::Hole)),
                | _ => Transition::Final(Eval::Stuck(StuckReason::WalkOnNonHere)),
            }
        },
        // Terminals are handled by `meet`; `drive` is only called on
        // non-terminals (a native builtin is a function-like terminal too).
        | Comp::Ret(_) | Comp::Abs(..) | Comp::With(..) | Comp::Native { .. } => {
            Transition::Final(Eval::Stuck(StuckReason::UnsupportedByReference))
        },
    }
}

/// Drives a `shift k. t` (rule Shift dynamics; `effects-control-shell.md` §2.2;
/// ADR-34 D2): walk the continuation to the nearest [`Cont::Reset`], capture
/// the delimited continuation (the `Reset` and the structural prefix above it)
/// as `k` in `env`, and run `t` below the delimiter.
///
/// The captured continuation **includes the `Reset`** (standard `shift`: a
/// later `resume k …` re-establishes the delimiter around the resumption), so
/// it is bound in `env` rather than reified to a `stk K` value. The binder `k`
/// is **α-renamed to a fresh name** before the body runs, so distinct captures
/// never collide in `env` even when the source reuses one name.
///
/// # Contract
/// - ensures: on a reachable delimiter across a structural prefix, binds a
///   fresh name to the captured continuation, α-renames `k` to it in the body,
///   and returns that body over the continuation below the (removed) `Reset`;
///   otherwise a [`Blame::ShiftNoReset`] — no enclosing `Reset`, or a capture
///   that would cross a handler (ADR-34 D5; the over-accepted set).
/// - panics: none.
fn drive_shift<'source, K>(
    k: K,
    body: &Comp,
    env: Env,
    cont: &mut Vec<Cont>,
    contenv: &mut ContEnv,
    gensym: &mut Gensym<GandrSort>,
) -> Transition
where
    K: Into<ContinuationName<'source>>,
{
    let k = k.into();
    let Some(reset_index) = capture_to_reset(cont)
    else {
        return Transition::Final(Eval::Blame(Blame::ShiftNoReset));
    };
    let reset_index = usize::from(reset_index);
    // The captured continuation is `[Reset, …structural prefix]`; bind it under a
    // fresh continuation key, α-rename `k` to it, and run `t` below the (removed)
    // delimiter under the shift-site value environment (its other free variables
    // resolve there).
    let captured = cont
        .get(reset_index ..)
        .map(<[Cont]>::to_vec)
        .unwrap_or_default();
    let Ok(fresh) = fresh_name(gensym)
    else {
        return Transition::Final(Eval::Stuck(StuckReason::FreshContinuationNameExhausted));
    };
    cont.truncate(reset_index);
    let renamed = subst_comp(body, NameRef::from(k.as_ref()), &Value::Var(fresh.clone()));
    contenv.push((fresh, Rc::from(captured)));
    Transition::Focus(renamed, env)
}

/// The outcome of forcing a memoized thunk (ADR-50 call-by-need).
enum ForceStep
{
    /// Continue with the thunk's weak-head form — the cached terminal, or the
    /// one just probed and cached — and its environment.
    Whnf(Comp, Env),
    /// The body is not purely reducible (it escapes an effect, or captured a
    /// continuation), so it cannot be cached: run it inline against the real
    /// continuation (the fast → slow fallback).
    Inline,
}

/// Finds the nearest enclosing [`Cont::Reset`] reachable from the top of `cont`
/// across a **structural** prefix only.
///
/// # Contract
/// - ensures: returns `Some(index)` of the nearest `Reset` when every frame
///   above it is structural (`Arg` / `Bind` / `Prj`); `None` when no `Reset`
///   exists or the capture would cross a [`Cont::Handle`] (the unreifiable
///   case, blamed by the caller).
/// - panics: none.
fn capture_to_reset(cont: &[Cont]) -> Option<ContinuationFrameIndex>
{
    for (index, frame) in cont.iter().enumerate().rev() {
        match *frame {
            | Cont::Arg(_) | Cont::Bind(..) | Cont::Prj(_) => {},
            | Cont::Reset => return Some(ContinuationFrameIndex::from(index)),
            | Cont::Handle { .. } => return None,
        }
    }
    None
}

/// Finds the nearest enclosing [`Cont::Handle`] declaring `op`, reachable
/// across a **structural** prefix only, returning its index and the resolved
/// clause.
///
/// # Contract
/// - ensures: returns `Some((index, clause, env))` for the nearest handler
///   whose signature declares `op` when every frame above it is structural —
///   its index, the resolved clause, and the handler-site value environment the
///   clause runs under; `None` when no such handler exists, or the capture
///   would cross a [`Cont::Reset`] or an intervening [`Cont::Handle`] that does
///   not declare `op` (the v0 single-handler scope, blamed by the caller).
/// - panics: none.
fn capture_to_handler<'source, O>(
    cont: &[Cont],
    op: O,
) -> Option<(ContinuationFrameIndex, OpClause, Env)>
where
    O: Into<OperationName<'source>>,
{
    let op = op.into();
    let op_name = op.as_ref();
    for (index, frame) in cont.iter().enumerate().rev() {
        match *frame {
            | Cont::Arg(_) | Cont::Bind(..) | Cont::Prj(_) => {},
            | Cont::Reset => return None,
            | Cont::Handle {
                ref sig,
                ref ops,
                ref env,
                ..
            } => {
                sig.op(op)?;
                let clause = ops.iter().find(|clause| clause.op == op_name)?;
                return Some((
                    ContinuationFrameIndex::from(index),
                    clause.clone(),
                    env.clone(),
                ));
            },
        }
    }
    None
}
/// Looks up the innermost captured continuation bound to `name` in the
/// environment (`shift` / `perform` resumption binders).
///
/// # Contract
/// - ensures: returns the innermost (most recently bound) captured continuation
///   for `name`, or `None` when `name` binds no captured continuation.
/// - panics: none.
#[inline]
#[must_use]
fn env_lookup<'source, N>(
    env: &ContEnv,
    name: N,
) -> Option<Rc<[Cont]>>
where
    N: Into<NameRef<'source>>,
{
    let name = name.into();
    env.iter()
        .rev()
        .find(|&entry| {
            let (ref entry_name, _) = *entry;
            entry_name == name.as_ref()
        })
        .map(|entry| {
            let (_, ref captured) = *entry;
            Rc::clone(captured)
        })
}
/// Runs the CEK machine on a computation from the empty continuation.
///
/// # Contract
/// - ensures: returns the final [`Eval`] of driving `comp` to a terminal (see
///   [`run`]).
/// - panics: none.
#[inline]
#[must_use]
pub fn run_comp(comp: Comp) -> Eval
{
    run(State::new(comp))
}
/// Runs the CEK machine on a computation from the empty continuation under a
/// prelude binding-environment (ADR-42).
///
/// The eval-side companion of the typing prelude: a forced prelude name (an
/// operator or a module builtin) resolves to its native computation instead of
/// halting at [`StuckReason::ForcedNonThunk`]. With [`Prelude::empty`] this is
/// exactly [`run_comp`].
///
/// # Contract
/// - ensures: returns the final [`Eval`] of driving `comp` to a terminal, with
///   `Force(Var …)` misses resolved against `prelude` (see [`run`]).
/// - panics: none.
#[inline]
#[must_use]
pub fn run_comp_with_prelude(
    comp: Comp,
    prelude: Prelude,
) -> Eval
{
    run(State::with_prelude(comp, prelude))
}
/// Runs the CEK machine to a terminal outcome.
///
/// # Contract
/// - ensures: drives [`step`] from `state` to a final [`Eval`] — a terminal
///   value, a defined blame, or an undefined stuck — returning it.
/// - provides: on the v0 fragment (no user recursion; environment-based
///   evaluation) evaluation of a finite term terminates;
///   [`StuckReason::StepLimit`] is a safety net for a pathological (necessarily
///   ill-typed) input.
/// - panics: none; the environment, continuation, and substitution worklist
///   live on the heap, so deep continuation nesting and deep substituted-into
///   terms do not overflow the host stack (see the module doc).
#[inline]
#[must_use]
pub fn run(state: State) -> Eval
{
    let mut current = state;
    loop {
        if current.steps >= STEP_BUDGET {
            return Eval::Stuck(StuckReason::StepLimit);
        }
        match step(current) {
            | Outcome::Step(next) => current = next,
            | Outcome::Final(eval) => return eval,
        }
    }
}

/// Runs the CEK machine on a computation from the empty continuation under a
/// host handler (ADR-35 D4) — the convenience over [`run_state_with_host`].
///
/// # Contract
/// - ensures: returns the final [`Eval`] of driving `comp` to a terminal with
///   handler-less `perform`s routed to `handler` (see [`run_state_with_host`]).
/// - panics: none (barring a panic thrown by `handler`).
#[inline]
#[must_use]
pub fn run_with_host<H>(
    comp: Comp,
    handler: &mut H,
) -> Eval
where
    H: HostHandler,
{
    run_state_with_host(State::new(comp), handler)
}
/// Runs the CEK machine to a terminal outcome, offering each host-interceptable
/// operation to `handler` first — the host-effect seam primitive (ADR-35 D4).
///
/// This is [`run`] wrapped: at each iteration, if the state has a pending host
/// operation ([`State::pending_host_op`]) the `handler` is consulted; on
/// [`HostReply::Resume`] the state resumes ([`State::resume_host`]) and the
/// loop continues, on [`HostReply::Unhandled`] the machine takes its ordinary
/// [`step`] (which blames [`Blame::PerformNoHandler`] for the unclaimed
/// `perform`). A state with no pending host operation always takes an ordinary
/// step. The loop is iterative (ADR-47): the host adds no host-stack recursion.
///
/// Exposed as the primitive so a prelude'd state ([`State::with_prelude`]) can
/// flow through; [`run_with_host`] is the empty-continuation convenience.
///
/// # Contract
/// - ensures: returns the final [`Eval`] of driving `state` to a terminal, with
///   every handler-less `perform` routed to `handler` — a resumed one
///   continues, a declined one blames as in [`run`]; [`StuckReason::StepLimit`]
///   guards a pathological input exactly as [`run`].
/// - panics: none (barring a panic thrown by `handler` itself).
#[inline]
#[must_use]
pub fn run_state_with_host<H>(
    mut state: State,
    handler: &mut H,
) -> Eval
where
    H: HostHandler,
{
    loop {
        if state.steps >= STEP_BUDGET {
            return Eval::Stuck(StuckReason::StepLimit);
        }
        if let Some(host_op) = state.pending_host_op() {
            match handler.handle(&host_op.sig, &host_op.op, &host_op.payload) {
                | HostReply::Resume(reply) => {
                    state = state.resume_host(reply);
                    continue;
                },
                // Decline: a pending host op is by definition a `perform` no
                // source handler claims, so its ordinary step is exactly
                // `Blame::PerformNoHandler`. Return it directly instead of
                // re-driving the `perform` (which would re-evaluate the payload
                // the CEK already read in `pending_host_op`) — the yxpx perf
                // residual (ADR-50; equivalent to the old fall-through step).
                | HostReply::Unhandled => return Eval::Blame(Blame::PerformNoHandler),
            }
        }
        match step(state) {
            | Outcome::Step(next) => state = next,
            | Outcome::Final(eval) => return eval,
        }
    }
}

/// Performs one machine step, dispatching on the focus and the top of the
/// continuation.
///
/// # Contract
/// - ensures: returns an `Outcome` for every input `State` (the machine is
///   total): `Outcome::Step(next)` for an ordinary transition (with the step
///   counter saturating-incremented), or `Outcome::Final(eval)` for a terminal
///   (a value at the empty continuation), a defined blame, or an undefined
///   stuck.
/// - provides: a runtime halt surfaces as `Outcome::Final`, never as a panic.
/// - panics: none.
#[inline]
#[must_use]
pub fn step(state: State) -> Outcome
{
    let State {
        focus,
        env,
        mut cont,
        mut contenv,
        mut gensym,
        steps,
        prelude,
    } = state;
    let stepped = if bool::from(is_terminal(&focus)) {
        meet(focus, env, &mut cont)
    }
    else {
        drive(focus, env, &mut cont, &mut contenv, &mut gensym, &prelude)
    };
    match stepped {
        | Transition::Focus(focus_next, env_next) => Outcome::Step(State {
            focus: focus_next,
            env: env_next,
            cont,
            contenv,
            gensym,
            steps: steps.saturating_add(1),
            prelude,
        }),
        | Transition::Final(eval) => Outcome::Final(eval),
    }
}

/// Whether a computation is a **terminal** whnf — a value-producing form that
/// meets the continuation rather than driving further (`ret v`, `λx. t`, or a
/// lazy pair).
#[inline]
#[must_use]
fn is_terminal(comp: &Comp) -> TerminalStatus
{
    matches!(
        *comp,
        Comp::Ret(_) | Comp::Abs(..) | Comp::With(..) | Comp::Native { .. }
    )
    .into()
}

/// The iterative big-step CBPV evaluator — the direct-style **reference** the
/// CEK machine is derived from (ADR-9; ADR-34 D1).
///
/// Realizes the **pure CBPV spine** (sequencing, application, forcing,
/// case / split / projection, the grade structural ops, and the gradual hole);
/// a control or effect form returns [`StuckReason::UnsupportedByReference`],
/// since this reference has no reified continuation to capture (the CEK machine
/// is what runs those). The conformance suite property-tests `eval_comp ≡ run`
/// on the pure spine.
///
/// # Contract
/// - ensures: returns the term's terminal whnf as `Eval::Value` (`ret v` for a
///   closed `F A`), a defined `Eval::Blame` (a hole reaching an elimination),
///   or `Eval::Stuck` (an ill-typed redex, or a control/effect form).
/// - panics: none; the evaluator uses an explicit heap continuation rather than
///   recursive host calls.
#[inline]
#[must_use]
/// # Termination
/// - reason: evaluator decomposes finite runtime syntax and an explicit finite
///   continuation.
/// - measure: pending evaluator frames plus the current finite runtime syntax
///   spine.
/// - boundedness: programs and evaluator frames are finite Rust values.
/// - input recursion: none.
pub fn eval_comp(comp: Comp) -> Eval
{
    enum Frame
    {
        App(Rc<Value>),
        Bind(String, Rc<Comp>),
        Prj(Side),
    }

    let mut current = comp;
    let mut frames = Vec::new();
    loop {
        match current {
            | Comp::Ret(_) | Comp::Abs(..) | Comp::With(..) => {
                let Some(frame) = frames.pop()
                else {
                    return Eval::Value(current);
                };
                current = match frame {
                    | Frame::App(arg) => match current {
                        | Comp::Abs(name, _, body) => subst_comp(&body, &name, arg.as_ref()),
                        | _ => return Eval::Stuck(StuckReason::AppliedNonFunction),
                    },
                    | Frame::Bind(var, body) => match current {
                        | Comp::Ret(value) => subst_comp(&body, &var, &value),
                        | _ => return Eval::Stuck(StuckReason::SequencedNonReturner),
                    },
                    | Frame::Prj(side) => match current {
                        | Comp::With(fst, snd) => {
                            let chosen = match side {
                                | Side::Fst => fst,
                                | Side::Snd => snd,
                            };
                            unrc_comp(chosen)
                        },
                        | _ => return Eval::Stuck(StuckReason::ProjectedNonPair),
                    },
                };
            },
            | Comp::App(head, arg) => {
                frames.push(Frame::App(arg));
                current = unrc_comp(head);
            },
            | Comp::Bind(bound, var, body) => {
                frames.push(Frame::Bind(var, body));
                current = unrc_comp(bound);
            },
            | Comp::Force(thunked) => match *strip_annot(&thunked) {
                | Value::Thunk(_, ref body) => current = body.as_ref().clone(),
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::ForcedNonThunk),
            },
            | Comp::Case(scrut, arm_fst, arm_snd) => match *strip_annot(&scrut) {
                | Value::Inj(side, ref payload) => {
                    let (var, body) = match side {
                        | Side::Fst => arm_fst,
                        | Side::Snd => arm_snd,
                    };
                    current = subst_comp(&body, &var, payload);
                },
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::CasedNonSum),
            },
            | Comp::DataCase(scrut, arms) => match *strip_annot(&scrut) {
                | Value::Ctor {
                    tag, ref payload, ..
                } => match arms.into_iter().nth(tag) {
                    | Some((var, body)) => current = subst_comp(&body, &var, payload),
                    | None => return Eval::Stuck(StuckReason::DataCasedNonCtor),
                },
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::DataCasedNonCtor),
            },
            | Comp::Split {
                scrut,
                fst_name,
                snd_name,
                body,
                ..
            } => match *strip_annot(&scrut) {
                | Value::Pair(ref fst, ref snd) => {
                    let once = subst_comp(&body, &snd_name, snd);
                    current = subst_comp(&once, &fst_name, fst);
                },
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::SplitNonProduct),
            },
            | Comp::RecordProj { record, label } => match *strip_annot(&record) {
                | Value::Record(ref fields) => match fields.get(label.as_str()) {
                    | Some(value) => current = Comp::ret(value.as_ref().clone()),
                    | None => return Eval::Stuck(StuckReason::RecordProjMissingField),
                },
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::RecordProjNonRecord),
            },
            | Comp::ListCase {
                scrut,
                nil,
                head,
                tail,
                cons,
            } => match reduce_list_case(&scrut, &nil, &head, &tail, &cons) {
                | ListReduce::Body(body) => current = body,
                | ListReduce::Final(eval) => return eval,
            },
            | Comp::Prj(side, target) => {
                frames.push(Frame::Prj(side));
                current = unrc_comp(target);
            },
            | Comp::Dup(value) => {
                current = Comp::ret(Value::pair(value.as_ref().clone(), value.as_ref().clone()));
            },
            | Comp::Drop(_) => current = Comp::ret(Value::Unit),
            | Comp::Walk { scrut, base, .. } => match *strip_annot(&scrut) {
                | Value::Here(ref witness) => {
                    current = subst_comp(base.body.as_ref(), &base.x, witness.as_ref());
                },
                | Value::Hole(_) => return Eval::Blame(Blame::Hole),
                | _ => return Eval::Stuck(StuckReason::WalkOnNonHere),
            },
            | Comp::Hole(_) => return Eval::Blame(Blame::Hole),
            | Comp::Perform(..)
            | Comp::Handle { .. }
            | Comp::Resume(..)
            | Comp::Reset(_)
            | Comp::Shift(..)
            | Comp::Native { .. } => return Eval::Stuck(StuckReason::UnsupportedByReference),
        }
    }
}

// The host-effect seam types (`HostOp`, `HostReply`, `HostHandler`) live in
// `crate::host` — their durable home, outliving the CEK (ADR-35 D4; coordinator
// decision D1). The CEK drivers below still offer them, so they are re-exported
// here for the CEK's public surface and its tests.
pub use crate::host::HostHandler;
pub use crate::host::HostOp;
pub use crate::host::HostReply;

/// Clones a reference-counted computation node into an owned term.
#[inline]
#[must_use]
fn unrc_comp(node: Rc<Comp>) -> Comp
{
    Rc::unwrap_or_clone(node)
}

/// Capture-avoiding substitution of `repl` for the free value variable `name`
/// inside a **value** — the value-into-value entry of the shared [`Subst`]
/// engine (the same iterative, ADR-47 traversal as [`subst_comp`], reusing its
/// binder-shadowing discipline).
///
/// This is the substitution the identity former's motive instantiation drives
/// (`crate::identity` calls it at each [`crate::types::ValueType::Path`]
/// endpoint), and the value-position analogue of [`subst_comp`]. Exposed to the
/// crate (`pub(crate)`) precisely so motive instantiation shares one proven
/// substitution rather than reimplementing capture-avoidance.
///
/// # Contract
/// - ensures: returns `value` with every free `name` replaced by `repl`,
///   leaving occurrences under a rebinding of `name` (a thunked computation's
///   binders) untouched; structurally identical to the direct recursive
///   definition.
/// - panics: none (the worklist's post-order balance keeps every result pop
///   defined; a `debug_assert` guards the invariant in test / debug builds).
#[must_use]
pub(crate) fn subst_value<'source, N>(
    value: &Value,
    name: N,
    repl: &Value,
) -> Value
where
    N: Into<NameRef<'source>>,
{
    let mut engine = Subst::new(name.into(), repl);
    engine.work.push(Task::DescendValue(value));
    engine.run();
    engine.take_value()
}

/// Peels type-ascription layers `(W : A)` off a runtime value, returning the
/// innermost non-[`RtValue::Annot`] value — the runtime companion of
/// [`strip_annot`], used when an elimination inspects a value's constructor.
///
/// # Contract
/// - ensures: returns the innermost non-annotation value.
/// - panics: none.
#[inline]
#[must_use]
fn rt_peel(value: &RtValue) -> &RtValue
{
    let mut current = value;
    while let RtValue::Annot(ref inner, _) = *current {
        current = inner;
    }
    current
}

/// Forces a memoized thunk closure (ADR-50 call-by-need): reuse the cached
/// weak-head form if present; otherwise probe the body ([`force_probe`]) — on a
/// pure, self-contained reduction cache the terminal and reuse it; on an
/// effectful / continuation-capturing body fall back to running it inline.
///
/// Black-holing: the cell is marked [`ThunkMemo::InProgress`] across the probe,
/// so a re-entrant force (a cycle) observes it and falls back — unreachable in
/// v0 (no recursion), where the deny path degrades to the [`STEP_BUDGET`] net.
///
/// # Contract
/// - ensures: [`ForceStep::Whnf`] with the cached / freshly-probed terminal for
///   a pure self-contained body; [`ForceStep::Inline`] otherwise.
/// - panics: none.
/// # Termination
/// - reason: evaluator drives thunk probes with an explicit probe-machine
///   stack.
/// - measure: pending probe machines plus their finite runtime syntax and
///   continuations.
/// - boundedness: programs, continuations, and probe machines are finite Rust
///   values bounded by [`STEP_BUDGET`] per probe.
/// - input recursion: none.
fn force_thunk(
    closure: &Closure,
    memo: &MemoCell,
    prelude: &Prelude,
) -> ForceStep
{
    match prepare_force(closure, memo) {
        | PreparedForce::Whnf(comp, env) => ForceStep::Whnf(comp, env),
        | PreparedForce::Inline => ForceStep::Inline,
        | PreparedForce::Probe(probe_closure, probe_memo) => {
            match force_probe(&probe_closure, prelude) {
                | Some((comp, env)) => {
                    *probe_memo.0.borrow_mut() = ThunkMemo::Forced {
                        comp: comp.clone(),
                        env: env.clone(),
                    };
                    ForceStep::Whnf(comp, env)
                },
                | None => {
                    // Not purely reducible: clear the black hole and run inline.
                    *probe_memo.0.borrow_mut() = ThunkMemo::Unforced;
                    ForceStep::Inline
                },
            }
        },
    }
}

/// Builds a closure arena for a structural computation boundary.
///
/// # Contract
/// - ensures: returns an arena/id pair for `body` converted to canonical
///   [`crate::syntax::CompNode`] carrier nodes, or `None` only when the checked
///   bridge cannot allocate an id.
/// - panics: none.
#[cfg_attr(
    dylint_lib = "non_local_effect_before_unhandled_error",
    allow(
        unknown_lints,
        non_local_effect_before_unhandled_error,
        reason = "allocation uses a fresh local arena that is discarded on failure; a systematic failure-atomic arena audit is tracked separately"
    )
)]
#[inline]
#[must_use]
fn closure_body_arena(body: &Comp) -> Option<(Rc<FlatArena>, ClosureBodyId)>
{
    let mut arena = FlatArena::new();
    let id = arena.alloc_comp(body).ok()?;
    Some((Rc::new(arena), id))
}

/// The result of reducing a [`Comp::ListCase`] against its scrutinee: the
/// selected arm's body to continue with, or a final runtime halt.
enum ListReduce
{
    /// Continue with the selected arm's body.
    Body(Comp),
    /// Halt (a `Hole` blame, or an ill-typed non-list scrutinee stuck).
    Final(Eval),
}
/// Evaluates a source [`Value`] under a value environment [`Env`] to a runtime
/// value [`RtValue`] (ADR-50 Decision C).
///
/// Resolves variables against the environment, snapshots a [`Value::Thunk`] /
/// [`Value::Stk`] into a [`Closure`] / stack closure, and strips
/// operationally-transparent annotations.
///
/// The traversal is an explicit heap worklist (ADR-47), so an adversarially
/// deep value evaluates on the heap and cannot overflow the host call stack.
///
/// # Contract
/// - ensures: returns the runtime image of `value` under `env`; a name not
///   bound in `env` becomes an [`RtValue::Var`] neutral.
/// - panics: none (a `debug_assert` guards the post-order balance in debug /
///   test builds).
#[inline]
#[must_use]
pub fn eval_value(
    value: &Value,
    env: &Env,
) -> Rc<RtValue>
{
    let mut work: Vec<EvalTask<'_>> = alloc::vec![EvalTask::Descend(value)];
    let mut out: Vec<Rc<RtValue>> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            | EvalTask::Descend(node) => eval_descend(node, env, &mut work, &mut out),
            | EvalTask::Combine(node) => eval_combine(node, &mut out),
        }
    }
    debug_assert!(
        out.len() == 1,
        "eval_value worklist must leave exactly one runtime value (post-order balance)"
    );
    out.pop().unwrap_or_else(|| Rc::new(RtValue::Unit))
}

/// Mints a fresh, run-unique name for α-renaming a captured-continuation binder
/// (`shift` / `perform`), by allocating the next [`GandrSort::ContKey`] atom.
///
/// The `%` prefix cannot occur in a source-level binder (binders are
/// alphanumeric identifiers), so a fresh name never shadows — or is shadowed
/// by — a program variable, and the allocator's global-distinctness invariant
/// (ADR-41) keeps every fresh name distinct. The atom's identity is rendered as
/// the existing `%k{id}` key; the continuation environment stays string-keyed,
/// so only the *minting* moves onto the substrate, not the IR's name carrier.
///
/// # Contract
/// - ensures: returns a name unique among all names this allocator has produced
///   and disjoint from every source-level identifier; advances `gensym` only on
///   successful allocation.
/// - fails: returns [`GensymExhausted`] without fabricating a continuation key
///   when the nominal allocator reports exhaustion.
/// - panics: none.
#[inline]
fn fresh_name(gensym: &mut Gensym<GandrSort>) -> Result<String, GensymExhausted<GandrSort>>
{
    gensym.fresh().map(render_fresh_atom)
}

/// Renders a freshly-minted [`GandrSort::ContKey`] atom as its `%k{id}`
/// continuation-key name.
#[inline]
fn render_fresh_atom(atom: Atom<GandrSort>) -> String
{
    let id = u32::from(atom.id());
    alloc::format!("%k{id}")
}

/// Visits a source value on the [`eval_value`] worklist: rebuilds a leaf
/// directly, or pushes a [`EvalTask::Combine`] and descends its children in
/// source order.
fn eval_descend<'src>(
    value: &'src Value,
    env: &Env,
    work: &mut Vec<EvalTask<'src>>,
    out: &mut Vec<Rc<RtValue>>,
)
{
    match *value {
        | Value::Var(ref name) => out.push(
            env.lookup(name)
                .unwrap_or_else(|| Rc::new(RtValue::Var(name.clone()))),
        ),
        | Value::Unit => out.push(Rc::new(RtValue::Unit)),
        | Value::Int(literal) => out.push(Rc::new(RtValue::Int(literal))),
        | Value::Str(ref literal) => out.push(Rc::new(RtValue::Str(literal.clone()))),
        | Value::Num(literal) => out.push(Rc::new(RtValue::Num(literal))),
        | Value::Hole(id) => out.push(Rc::new(RtValue::Hole(id))),
        | Value::Thunk(grade, ref body) => {
            let Some(closure) = Closure::from_body(env.clone(), body.as_ref())
            else {
                out.push(Rc::new(RtValue::Hole(0)));
                return;
            };
            out.push(Rc::new(RtValue::Thunk(
                grade,
                closure,
                MemoCell::unforced(),
            )));
        },
        | Value::Stk(ref stack) => {
            out.push(Rc::new(RtValue::Stk(Rc::clone(stack), env.clone())));
        },
        // An annotation is operationally transparent but PRESERVED in the
        // value domain, so a returned annotated value reads back identically to
        // the substitution reference (an elimination peels it via `rt_peel`).
        | Value::Annot(ref inner, _) => {
            work.push(EvalTask::Combine(value));
            work.push(EvalTask::Descend(inner));
        },
        | Value::Pair(ref fst, ref snd) => {
            work.push(EvalTask::Combine(value));
            work.push(EvalTask::Descend(fst));
            work.push(EvalTask::Descend(snd));
        },
        | Value::Inj(_, ref payload) => {
            work.push(EvalTask::Combine(value));
            work.push(EvalTask::Descend(payload));
        },
        | Value::List(ref elements) => {
            work.push(EvalTask::Combine(value));
            for element in elements {
                work.push(EvalTask::Descend(element));
            }
        },
        | Value::Record(ref fields) => {
            work.push(EvalTask::Combine(value));
            for field in fields.values() {
                work.push(EvalTask::Descend(field));
            }
        },
        // A reflexivity proof descends into its witness (ADR-76), as an
        // injection payload.
        | Value::Here(ref witness) => {
            work.push(EvalTask::Combine(value));
            work.push(EvalTask::Descend(witness));
        },
        // A declared-data constructor descends into its field-tuple payload
        // (ADR-80), as an injection payload.
        | Value::Ctor {
            payload: ref field, ..
        } => {
            work.push(EvalTask::Combine(value));
            work.push(EvalTask::Descend(field));
        },
    }
}

/// Reassembles a runtime value from its already-built children on the result
/// stack (children pop in source order — the two LIFO reversals cancel).
fn eval_combine(
    value: &Value,
    out: &mut Vec<Rc<RtValue>>,
)
{
    let rebuilt = match *value {
        | Value::Pair(..) => {
            let fst = pop_rt(out);
            let snd = pop_rt(out);
            RtValue::Pair(fst, snd)
        },
        | Value::Inj(side, _) => RtValue::Inj(side, pop_rt(out)),
        | Value::List(ref elements) => {
            let mut built = Vec::with_capacity(elements.len());
            for _ in elements {
                built.push(pop_rt(out));
            }
            RtValue::List(built)
        },
        | Value::Record(ref fields) => {
            let mut built = BTreeMap::new();
            for label in fields.keys() {
                built.insert(label.clone(), pop_rt(out));
            }
            RtValue::Record(built)
        },
        | Value::Annot(_, ref ty) => RtValue::Annot(pop_rt(out), Rc::clone(ty)),
        | Value::Here(_) => RtValue::Here(pop_rt(out)),
        | Value::Ctor { ref id, tag, .. } => RtValue::Ctor {
            id: id.clone(),
            tag,
            payload: pop_rt(out),
        },
        // Leaves never reach a combine (they resolve in `eval_descend`); the
        // arm is required only for exhaustiveness.
        | _ => return,
    };
    out.push(Rc::new(rebuilt));
}

/// One pending task on the substitution worklist — the defunctionalised image
/// of a recursive `subst_*` call (ADR-47 T1). A `Descend*` visits a source
/// node, rebuilding a leaf / whole-node-shadow directly or pushing a `Combine*`
/// followed by its substituted children; a `Combine*` re-reads the **same**
/// source node and reassembles it from those children, now on the result
/// stacks. Every task borrows the immutable input for the whole run, so a
/// `Descend`/`Combine` pair reads one source and recomputes the identical
/// shadowing decision — the two halves cannot desync.
enum Task<'src>
{
    /// Visit a computation.
    DescendComp(&'src Comp),
    /// Reassemble a computation from its substituted children.
    CombineComp(&'src Comp),
    /// Visit a value.
    DescendValue(&'src Value),
    /// Reassemble a value from its substituted children.
    CombineValue(&'src Value),
    /// Visit a reified stack.
    DescendStack(&'src Stack),
    /// Reassemble a reified stack from its substituted children.
    CombineStack(&'src Stack),
}

/// The iterative capture-avoiding substitution engine (ADR-47 T1): the shared
/// driver behind `subst_comp` (and the value / stack sub-substitutions it
/// inlines). It owns an explicit LIFO work stack and one result stack per
/// syntactic sort, so substitution depth follows the heap, not the host call
/// stack — the iterative shadow of the recursive specification (the Agda
/// metatheory stays the oracle, ADR-47).
///
/// # Contract
/// - ensures: after [`Self::run`] drains a work stack seeded by exactly one
///   `Descend*`, the matching result stack holds exactly one rebuilt node and
///   the other result stacks are empty (the post-order balance invariant).
struct Subst<'src>
{
    /// The variable being replaced.
    name: &'src str,
    /// The replacement value (cloned at each matching free [`Value::Var`]).
    repl: &'src Value,
    /// Pending tasks, processed last-in-first-out (post order).
    work: Vec<Task<'src>>,
    /// Rebuilt computations, most-recent last.
    comps: Vec<Comp>,
    /// Rebuilt values, most-recent last.
    values: Vec<Value>,
    /// Rebuilt reified stacks, most-recent last.
    stacks: Vec<Stack>,
}

impl<'src> Subst<'src>
{
    /// Builds an empty engine substituting `repl` for `name`.
    fn new(
        name: NameRef<'src>,
        repl: &'src Value,
    ) -> Self
    {
        Self {
            name: <&str>::from(name),
            repl,
            work: Vec::new(),
            comps: Vec::new(),
            values: Vec::new(),
            stacks: Vec::new(),
        }
    }

    /// Drains the work stack to completion (post-order rebuild).
    fn run(&mut self)
    {
        while let Some(task) = self.work.pop() {
            match task {
                | Task::DescendComp(node) => self.descend_comp(node),
                | Task::CombineComp(node) => self.combine_comp(node),
                | Task::DescendValue(node) => self.descend_value(node),
                | Task::CombineValue(node) => self.combine_value(node),
                | Task::DescendStack(node) => self.descend_stack(node),
                | Task::CombineStack(node) => self.combine_stack(node),
            }
        }
    }

    /// Pops the most-recent rebuilt computation.
    ///
    /// The post-order balance invariant guarantees a result is present at every
    /// pop; the `debug_assert` surfaces a broken invariant in test / debug
    /// builds, and the fallback keeps the pop total under the no-`unwrap` /
    /// no-`panic` lint wall — it is never reached (the `subst_comp` ≡
    /// `subst_comp_recursive` differential proptest would fail first on any
    /// desync).
    fn take_comp(&mut self) -> Comp
    {
        debug_assert!(
            !self.comps.is_empty(),
            "subst worklist underflow: a rebuilt computation must be present (ADR-47 post-order balance)"
        );
        self.comps.pop().unwrap_or_else(|| Comp::ret(Value::Unit))
    }

    /// Pops the most-recent rebuilt value (see [`Self::take_comp`]).
    fn take_value(&mut self) -> Value
    {
        debug_assert!(
            !self.values.is_empty(),
            "subst worklist underflow: a rebuilt value must be present (ADR-47 post-order balance)"
        );
        self.values.pop().unwrap_or(Value::Unit)
    }

    /// Pops the most-recent rebuilt reified stack (see [`Self::take_comp`]).
    fn take_stack(&mut self) -> Stack
    {
        debug_assert!(
            !self.stacks.is_empty(),
            "subst worklist underflow: a rebuilt stack must be present (ADR-47 post-order balance)"
        );
        self.stacks.pop().unwrap_or(Stack::Empty)
    }

    /// Visits a computation: rebuilds a leaf / whole-node-shadow directly, or
    /// pushes a [`Task::CombineComp`] and descends its substituted children in
    /// source order (mirrors the recursive `subst_comp` match arm-for-arm; a
    /// shadowed child is not descended and is shared by `combine_comp`).
    fn descend_comp(
        &mut self,
        comp: &'src Comp,
    )
    {
        match *comp {
            | Comp::Abs(ref binder, _, ref body) => {
                if binder == self.name {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::App(ref head, ref arg) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(head.as_ref()));
                self.work.push(Task::DescendValue(arg.as_ref()));
            },
            // `ret v`, `force v`, `dup v`, `drop v`, and `perform … v` each
            // descend a single value child; only the reassembly differs, so the
            // descent is shared here (`combine_comp` re-matches to rebuild each).
            | Comp::Ret(ref value)
            | Comp::Force(ref value)
            | Comp::Dup(ref value)
            | Comp::Drop(ref value)
            | Comp::Perform(_, _, ref value) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(value.as_ref()));
            },
            | Comp::Bind(ref bound, ref binder, ref body) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(bound.as_ref()));
                if binder != self.name {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::Case(ref scrut, ref arm_fst, ref arm_snd) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if arm_fst.0 != self.name {
                    self.work.push(Task::DescendComp(arm_fst.1.as_ref()));
                }
                if arm_snd.0 != self.name {
                    self.work.push(Task::DescendComp(arm_snd.1.as_ref()));
                }
            },
            | Comp::ListCase {
                ref scrut,
                ref nil,
                ref head,
                ref tail,
                ref cons,
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                self.work.push(Task::DescendComp(nil.as_ref()));
                // The `cons` body is under `head`/`tail`; descend it only when
                // neither binder rebinds `name` (the `nil` body always descends).
                if head != self.name && tail != self.name {
                    self.work.push(Task::DescendComp(cons.as_ref()));
                }
            },
            // The motive is a *type* (runtime-erased, untraced) carried verbatim
            // by the combine arm — exactly as an `Abs` binder annotation and the
            // `Walk` motive are not substituted here (ADR-82 D4); only the
            // scrutinee value and the body (under `p`/`q`) are descended.
            | Comp::Split {
                ref scrut,
                ref fst_name,
                ref snd_name,
                ref body,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if fst_name != self.name && snd_name != self.name {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            // Each arm body is under its own payload binder; descend only the
            // arms that binder does not rebind `name` (ADR-80), in source order.
            | Comp::DataCase(ref scrut, ref arms) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                for arm in arms {
                    if arm.0 != self.name {
                        self.work.push(Task::DescendComp(arm.1.as_ref()));
                    }
                }
            },
            | Comp::With(ref fst, ref snd) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(fst.as_ref()));
                self.work.push(Task::DescendComp(snd.as_ref()));
            },
            | Comp::Prj(_, ref target) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(target.as_ref()));
            },
            // The label is not a binder, so substitution descends into the
            // record value unconditionally (ADR-45 D4).
            | Comp::RecordProj { ref record, .. } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(record.as_ref()));
            },
            | Comp::Handle {
                ref scrutinee,
                ret: (ref ret_var, ref ret_body),
                ref ops,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(scrutinee.as_ref()));
                if ret_var != self.name {
                    self.work.push(Task::DescendComp(ret_body.as_ref()));
                }
                // Each clause body is under its own payload / resume binders;
                // descend only the clauses neither rebinds `name`, in order.
                for clause in ops {
                    if clause.payload != self.name && clause.resume != self.name {
                        self.work.push(Task::DescendComp(clause.body.as_ref()));
                    }
                }
            },
            | Comp::Resume(ref reified, ref fed) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(reified.as_ref()));
                self.work.push(Task::DescendComp(fed.as_ref()));
            },
            | Comp::Reset(ref body) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(body.as_ref()));
            },
            | Comp::Shift(ref k, ref body) => {
                if k == self.name {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::Hole(_) => self.comps.push(comp.clone()),
            // A native builtin carries its accumulated argument values (closed in
            // a closed program, and empty in a source term) — descend into each
            // so substitution stays total (ADR-42); the opaque `prim` is
            // unaffected.
            | Comp::Native { ref args, .. } => {
                self.work.push(Task::CombineComp(comp));
                for arg in args {
                    self.work.push(Task::DescendValue(arg.as_ref()));
                }
            },
            // The identity eliminator (ADR-76): descend into the scrutinee value
            // and the base body (under the diagonal binder `x`, so descend it
            // only when `x` does not rebind `name`). The motive is a *type*
            // (runtime-erased, untraced) and is carried verbatim, exactly as an
            // `Abs` binder annotation is not substituted here.
            | Comp::Walk {
                ref scrut,
                ref base,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if base.x != self.name {
                    self.work.push(Task::DescendComp(base.body.as_ref()));
                }
            },
        }
    }

    /// Reassembles a computation from its substituted children on the result
    /// stacks (re-reads the same source node as [`Self::descend_comp`], so the
    /// shadowing decision is recomputed identically; children pop in source
    /// order — the two LIFO reversals of work and result stacks cancel).
    fn combine_comp(
        &mut self,
        comp: &'src Comp,
    )
    {
        let rebuilt = match *comp {
            | Comp::Abs(ref binder, ref annot, _) => {
                Comp::Abs(binder.clone(), annot.clone(), Rc::new(self.take_comp()))
            },
            | Comp::App(..) => {
                let head = self.take_comp();
                let arg = self.take_value();
                Comp::App(Rc::new(head), Rc::new(arg))
            },
            | Comp::Ret(_) => Comp::Ret(Rc::new(self.take_value())),
            | Comp::Bind(_, ref binder, ref body) => {
                let bound = self.take_comp();
                let body_sub = if binder == self.name {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Bind(Rc::new(bound), binder.clone(), body_sub)
            },
            | Comp::Force(_) => Comp::Force(Rc::new(self.take_value())),
            | Comp::Case(_, ref arm_fst, ref arm_snd) => {
                let scrut = self.take_value();
                let fst_body = if arm_fst.0 == self.name {
                    Rc::clone(&arm_fst.1)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let snd_body = if arm_snd.0 == self.name {
                    Rc::clone(&arm_snd.1)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Case(
                    Rc::new(scrut),
                    (arm_fst.0.clone(), fst_body),
                    (arm_snd.0.clone(), snd_body),
                )
            },
            | Comp::ListCase {
                ref head,
                ref tail,
                ref cons,
                ..
            } => {
                let scrut = self.take_value();
                let nil = self.take_comp();
                let cons_sub = if head == self.name || tail == self.name {
                    Rc::clone(cons)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::ListCase {
                    scrut: Rc::new(scrut),
                    nil: Rc::new(nil),
                    head: head.clone(),
                    tail: tail.clone(),
                    cons: cons_sub,
                }
            },
            // The motive is carried verbatim (a runtime-erased type child, the
            // `Walk` precedent; ADR-82 D4) — only the scrutinee value and the
            // body (unless `p`/`q` shadow `name`) are substituted.
            | Comp::Split {
                ref fst_name,
                ref snd_name,
                ref motive,
                ref body,
                ..
            } => {
                let scrut = self.take_value();
                let body_sub = if fst_name == self.name || snd_name == self.name {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Split {
                    scrut: Rc::new(scrut),
                    fst_name: fst_name.clone(),
                    snd_name: snd_name.clone(),
                    motive: motive.clone(),
                    body: body_sub,
                }
            },
            | Comp::DataCase(_, ref arms) => {
                let scrut = self.take_value();
                let mut arms_sub = Vec::with_capacity(arms.len());
                for arm in arms {
                    if arm.0 == self.name {
                        arms_sub.push((arm.0.clone(), Rc::clone(&arm.1)));
                    }
                    else {
                        arms_sub.push((arm.0.clone(), Rc::new(self.take_comp())));
                    }
                }
                Comp::DataCase(Rc::new(scrut), arms_sub)
            },
            | Comp::With(..) => {
                let fst = self.take_comp();
                let snd = self.take_comp();
                Comp::With(Rc::new(fst), Rc::new(snd))
            },
            | Comp::Prj(side, _) => Comp::Prj(side, Rc::new(self.take_comp())),
            | Comp::RecordProj { ref label, .. } => Comp::RecordProj {
                record: Rc::new(self.take_value()),
                label: label.clone(),
            },
            | Comp::Dup(_) => Comp::Dup(Rc::new(self.take_value())),
            | Comp::Drop(_) => Comp::Drop(Rc::new(self.take_value())),
            | Comp::Perform(ref sig, ref op, _) => {
                Comp::Perform(sig.clone(), op.clone(), Rc::new(self.take_value()))
            },
            | Comp::Handle {
                ref sig,
                ret: (ref ret_var, ref ret_body),
                ref ops,
                ..
            } => {
                let scrutinee = self.take_comp();
                let ret_sub = if ret_var == self.name {
                    Rc::clone(ret_body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let mut ops_sub = Vec::with_capacity(ops.len());
                for clause in ops {
                    if clause.payload == self.name || clause.resume == self.name {
                        ops_sub.push(clause.clone());
                    }
                    else {
                        ops_sub.push(OpClause {
                            op: clause.op.clone(),
                            payload: clause.payload.clone(),
                            resume: clause.resume.clone(),
                            body: Rc::new(self.take_comp()),
                        });
                    }
                }
                Comp::Handle {
                    sig: sig.clone(),
                    scrutinee: Rc::new(scrutinee),
                    ret: (ret_var.clone(), ret_sub),
                    ops: ops_sub,
                }
            },
            | Comp::Resume(..) => {
                let reified = self.take_value();
                let fed = self.take_comp();
                Comp::Resume(Rc::new(reified), Rc::new(fed))
            },
            | Comp::Reset(_) => Comp::Reset(Rc::new(self.take_comp())),
            | Comp::Shift(ref k, _) => Comp::Shift(k.clone(), Rc::new(self.take_comp())),
            // A leaf / whole-node-shadow is rebuilt in `descend_comp` and never
            // reaches a combine; the arm is required only for exhaustiveness.
            | Comp::Hole(_) => comp.clone(),
            | Comp::Native { prim, ref args } => {
                let mut built = Vec::with_capacity(args.len());
                for _ in args {
                    built.push(Rc::new(self.take_value()));
                }
                Comp::Native { prim, args: built }
            },
            | Comp::Walk {
                ref motive,
                ref base,
                ..
            } => {
                let scrut = self.take_value();
                let base_body = if base.x == self.name {
                    Rc::clone(&base.body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Walk {
                    scrut: Rc::new(scrut),
                    motive: motive.clone(),
                    base: WalkBase {
                        x: base.x.clone(),
                        body: base_body,
                    },
                }
            },
        };
        self.comps.push(rebuilt);
    }

    /// Visits a value (mirrors the recursive `subst_value`).
    fn descend_value(
        &mut self,
        value: &'src Value,
    )
    {
        match *value {
            | Value::Var(ref var) => {
                let substituted = if var == self.name {
                    self.repl.clone()
                }
                else {
                    value.clone()
                };
                self.values.push(substituted);
            },
            | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) | Value::Hole(_) => {
                self.values.push(value.clone());
            },
            | Value::Pair(ref fst, ref snd) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(fst.as_ref()));
                self.work.push(Task::DescendValue(snd.as_ref()));
            },
            | Value::Inj(_, ref payload) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(payload.as_ref()));
            },
            // A list's elements are ordinary values: descend into each (ADR-40
            // D2).
            | Value::List(ref elements) => {
                self.work.push(Task::CombineValue(value));
                for element in elements {
                    self.work.push(Task::DescendValue(element.as_ref()));
                }
            },
            // A record's field values are ordinary values: descend into each,
            // preserving the labels (ADR-45 D2). Iteration is canonical key
            // order (`Value::Record` is a `BTreeMap`), matched by `combine_value`.
            | Value::Record(ref fields) => {
                self.work.push(Task::CombineValue(value));
                for field in fields.values() {
                    self.work.push(Task::DescendValue(field.as_ref()));
                }
            },
            | Value::Thunk(_, ref body) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendComp(body.as_ref()));
            },
            | Value::Annot(ref inner, _) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(inner.as_ref()));
            },
            | Value::Stk(ref stack) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendStack(stack.as_ref()));
            },
            // A reflexivity proof carries an ordinary value witness (ADR-76);
            // descend into it exactly as an injection payload.
            | Value::Here(ref witness) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(witness.as_ref()));
            },
            // A declared-data constructor carries an ordinary field-tuple
            // payload (ADR-80); descend into it exactly as an injection payload.
            | Value::Ctor {
                payload: ref field, ..
            } => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(field.as_ref()));
            },
        }
    }

    /// Reassembles a value from its substituted children (re-reads the source
    /// node; children pop in source order).
    fn combine_value(
        &mut self,
        value: &'src Value,
    )
    {
        let rebuilt = match *value {
            | Value::Pair(..) => {
                let fst = self.take_value();
                let snd = self.take_value();
                Value::Pair(Rc::new(fst), Rc::new(snd))
            },
            | Value::Inj(side, _) => Value::Inj(side, Rc::new(self.take_value())),
            | Value::Ctor { ref id, tag, .. } => Value::Ctor {
                id: id.clone(),
                tag,
                payload: Rc::new(self.take_value()),
            },
            | Value::List(ref elements) => {
                let mut built = Vec::with_capacity(elements.len());
                for _ in elements {
                    built.push(Rc::new(self.take_value()));
                }
                Value::List(built)
            },
            | Value::Record(ref fields) => {
                let mut built = BTreeMap::new();
                // `fields.values()` (descend) and this key iteration walk the
                // BTreeMap in the same canonical order, so `take_value` returns
                // each field's substituted value in the matching order.
                for label in fields.keys() {
                    let field = self.take_value();
                    built.insert(label.clone(), Rc::new(field));
                }
                Value::Record(built)
            },
            | Value::Thunk(grade, _) => Value::Thunk(grade, Rc::new(self.take_comp())),
            | Value::Annot(_, ref ty) => Value::Annot(Rc::new(self.take_value()), Rc::clone(ty)),
            | Value::Stk(_) => Value::Stk(Rc::new(self.take_stack())),
            | Value::Here(_) => Value::Here(Rc::new(self.take_value())),
            // Leaves are rebuilt in `descend_value` and never reach a combine;
            // the arm is required only for exhaustiveness.
            | Value::Var(_)
            | Value::Unit
            | Value::Int(_)
            | Value::Str(_)
            | Value::Num(_)
            | Value::Hole(_) => value.clone(),
        };
        self.values.push(rebuilt);
    }

    /// Visits a reified stack (mirrors the recursive `subst_stack`).
    fn descend_stack(
        &mut self,
        stack: &'src Stack,
    )
    {
        match *stack {
            | Stack::Empty => self.stacks.push(Stack::Empty),
            | Stack::Arg(ref value, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                self.work.push(Task::DescendValue(value.as_ref()));
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
            | Stack::Bind(ref binder, ref body, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                if binder != self.name {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
            | Stack::Prj(_, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
        }
    }

    /// Reassembles a reified stack from its substituted children (re-reads the
    /// source node; the value / bind-body and the rest live on distinct result
    /// stacks, so their pop order is independent).
    fn combine_stack(
        &mut self,
        stack: &'src Stack,
    )
    {
        let rebuilt = match *stack {
            | Stack::Arg(..) => {
                let value = self.take_value();
                let rest = self.take_stack();
                Stack::Arg(Rc::new(value), Rc::new(rest))
            },
            | Stack::Bind(ref binder, ref body, _) => {
                let body_sub = if binder == self.name {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let rest = self.take_stack();
                Stack::Bind(binder.clone(), body_sub, Rc::new(rest))
            },
            | Stack::Prj(side, _) => Stack::Prj(side, Rc::new(self.take_stack())),
            // `Stack::Empty` is rebuilt in `descend_stack` and never reaches a
            // combine; the arm is required only for exhaustiveness.
            | Stack::Empty => Stack::Empty,
        };
        self.stacks.push(rebuilt);
    }
}

// ── The runtime value domain (ADR-50 Decision C/D, Phase 1a) ─────────────────
//
// A first-order runtime value domain for the CEK environment machine: source
// values evaluate to `RtValue`s under a value `Env`, a `Value::Thunk` becomes a
// first-order `Closure` (never a host closure — the machine stays serializable
// and inspectable, ADR-9), and readback (`quote_value`) reconstructs a source
// `Value`. Purely additive here: nothing on the `run` / `step` hot path
// consumes it yet (that is Phase 1b) — it is the shared value domain the CEK
// dynamics and the (later) NbE normalizer both drive (ADR-50 Decision D).

/// A **runtime value** `W` (ADR-50 Decision C/D).
///
/// The result of evaluating a source [`Value`] under a value environment
/// ([`Env`]) on the CEK machine.
///
/// The runtime image of [`Value`]: a source variable is resolved to the value
/// it denotes (an unresolved name — a prelude binding, or an open variable from
/// readback under a binder — stays a [`Self::Var`] neutral), and a source
/// [`Value::Thunk`] / [`Value::Stk`] becomes a first-order [`Closure`] / stack
/// closure rather than a host closure, keeping the machine
/// serializable / inspectable (ADR-9). The domain is **glue-ready** but not yet
/// glued: per-node unfolded / local forms (smalltt `VUnfold`, Idris 2
/// `Core.Value`) attach in a later phase, so no global
/// term‖value pair is baked in here (ADR-50 Decision D). Kept
/// `#[non_exhaustive]` so those faces and multi-output values (ADR-49 D5) land
/// later without breaking matches.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RtValue
{
    /// A free / neutral name: a prelude binding consulted at a `force` miss, or
    /// an open variable produced by readback under a binder (Phase 3).
    Var(
        /// The unresolved name.
        String,
    ),
    /// The unit value `()`.
    Unit,
    /// An integer literal `n` (the runtime image of [`Value::Int`]).
    Int(
        /// The literal's numeric value.
        i64,
    ),
    /// A string literal `s` (the runtime image of [`Value::Str`]).
    Str(
        /// The literal's character data.
        String,
    ),
    /// A typed numeric literal (the runtime image of [`Value::Num`]).
    Num(
        /// The literal's typed numeric payload.
        NumLit,
    ),
    /// An eager pair `(W, W′)`.
    Pair(
        /// The first component.
        Rc<Self>,
        /// The second component.
        Rc<Self>,
    ),
    /// An injection `inj1 W` / `inj2 W` into a tagged sum.
    Inj(
        /// Which summand is injected.
        Side,
        /// The injected payload.
        Rc<Self>,
    ),
    /// A list `[W₀, …, Wₙ]`.
    List(
        /// The list's elements, in order.
        Vec<Rc<Self>>,
    ),
    /// A record `{ℓᵢ = Wᵢ}`, keyed by label (canonical, name-ordered).
    Record(
        /// The fields `ℓᵢ ↦ Wᵢ`.
        BTreeMap<String, Rc<Self>>,
    ),
    /// A graded thunk closure `thunk_r (Env, t)` — a suspended computation
    /// captured with its environment and a call-by-need memoization cell
    /// (ADR-50: forcing caches the weak-head form; see [`MemoCell`]).
    Thunk(
        /// The usage grade annotation `r`.
        Grade,
        /// The captured computation closure.
        Closure,
        /// The call-by-need memoization cell (shared across copies of this
        /// thunk value, so a force through any of them caches for all).
        MemoCell,
    ),
    /// A typed hole `?u` in value position (the runtime image of
    /// [`Value::Hole`]).
    Hole(
        /// The hole's identifier.
        HoleId,
    ),
    /// A reified stack closure `stk (Env, K)` — a source-level evaluation
    /// context captured with the environment resolving its argument values.
    Stk(
        /// The reified stack `K`.
        Rc<Stack>,
        /// The environment its argument values resolve under.
        Env,
    ),
    /// A type ascription `(W : A)` — operationally transparent (peeled by
    /// [`rt_peel`] at an elimination), but PRESERVED in the value domain so a
    /// returned annotated value reads back identically to the substitution
    /// reference (the runtime image of [`Value::Annot`]).
    Annot(
        /// The annotated runtime value.
        Rc<Self>,
        /// The ascribed type.
        Rc<ValueType>,
    ),
    /// A reflexivity proof `here(W)` — the runtime image of [`Value::Here`]
    /// (ADR-76). The identity eliminator `Walk` inspects this constructor to
    /// fire its β-rule; a closed `Path`-typed result is always a `Here`
    /// (canonicity).
    Here(
        /// The witness runtime value `W`.
        Rc<Self>,
    ),
    /// A declared-data constructor value `Ctor { id, tag, W }` — the runtime
    /// image of [`Value::Ctor`] (ADR-80). The eliminator [`Comp::DataCase`]
    /// inspects the `tag` to select its arm, binding the arm's payload binder
    /// to the field-tuple `W`, exactly as [`Self::Inj`] drives a `case`.
    Ctor
    {
        /// The datatype's minted nominal identity.
        id: crate::types::DataId,
        /// The constructor's tag (position in the decl-table `ctors` list).
        tag: usize,
        /// The field-tuple payload runtime value `W`.
        payload: Rc<Self>,
    },
}

/// Pops the most-recent runtime value from a worklist result stack, with a
/// balance-invariant guard (the fallback is never reached — a desync would fail
/// the round-trip proptests first).
fn pop_rt(out: &mut Vec<Rc<RtValue>>) -> Rc<RtValue>
{
    debug_assert!(
        !out.is_empty(),
        "eval_value worklist underflow (post-order balance)"
    );
    out.pop().unwrap_or_else(|| Rc::new(RtValue::Unit))
}

/// A **value environment** (ADR-50 Decision C).
///
/// A persistent, innermost-first association of source binder names to the
/// [`RtValue`]s they denote on the CEK machine. It replaces eager substitution
/// on the hot path — `β`, `bind`, `case`, `split`, `listcase`, and `recordproj`
/// extend it rather than rewriting the term (Phase 1b).
///
/// A cons list, so extension is `O(1)` and a snapshot into a frame or a
/// [`Closure`] is an `Rc` bump; lookup walks innermost-first (later bindings
/// shadow earlier ones, as [`crate::ctx::Ctx`]).
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Env(Option<Rc<EnvNode>>);

/// One cons cell of an [`Env`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct EnvNode
{
    /// The bound binder name.
    name: String,
    /// The runtime value the name denotes.
    value: Rc<RtValue>,
    /// The enclosing environment (the next-outer bindings).
    rest: Option<Rc<Self>>,
}

impl Env
{
    /// The empty environment — no name is bound.
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self(None)
    }

    /// Extends the environment with `name ↦ value` bound innermost, shadowing
    /// any outer binding of the same name.
    #[inline]
    #[must_use]
    pub fn extend(
        &self,
        name: String,
        value: Rc<RtValue>,
    ) -> Self
    {
        Self(Some(Rc::new(EnvNode {
            name,
            value,
            rest: self.0.clone(),
        })))
    }

    /// Looks `name` up, returning the innermost (most recently bound) value, or
    /// `None` when `name` is not bound (a free / neutral name).
    #[inline]
    #[must_use]
    pub fn lookup<'source, N>(
        &self,
        name: N,
    ) -> Option<Rc<RtValue>>
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        let mut cursor = self.0.as_deref();
        while let Some(node) = cursor {
            if node.name == name.as_ref() {
                return Some(Rc::clone(&node.value));
            }
            cursor = node.rest.as_deref();
        }
        None
    }
}

/// A **first-order closure** `(Env, CompNodeId)` (ADR-50 Decision C).
///
/// A captured environment paired with a suspended computation body id in the
/// canonical flat [`FlatArena`] carrier — never a host (Rust) closure, and no
/// direct `Rc<Comp>` body ownership in the closure state. It backs
/// [`RtValue::Thunk`] (a source [`Value::Thunk`]) and lambda bodies at
/// application time.
type ClosureBodyId = CompNodeId;

/// Iterative readback / closure-closing trampoline (ADR-47 / ADR-50).
///
/// A single worklist owns value quotation, computation closing, and stack
/// closing. When a thunk or stack closure is encountered, its close task is
/// pushed on the same heap stack instead of calling back into [`quote_value`],
/// which keeps closure environments depth-robust.
struct Readback
{
    /// The pending-task worklist (LIFO).
    work: Vec<ReadbackTask>,
    /// The rebuilt source-value result stack.
    values: Vec<Value>,
    /// The rebuilt computation result stack.
    comps: Vec<Comp>,
    /// The rebuilt stack result stack.
    stacks: Vec<Stack>,
}

/// One pending readback task.
enum ReadbackTask
{
    /// Visit a runtime value, pushing a combine plus its children, or a leaf.
    QuoteDescend(Rc<RtValue>),
    /// Reassemble a source value from its children on the result stack.
    QuoteCombine(Rc<RtValue>),
    /// Close a computation under the remaining environment cursor.
    CloseComp
    {
        /// The computation being closed.
        current: Comp,
        /// The remaining environment bindings to substitute, innermost first.
        cursor: Option<Rc<EnvNode>>,
        /// Binder names shadowed inside the computation — left unsubstituted.
        shielded: Vec<String>,
    },
    /// Apply one quoted environment replacement to a computation.
    ApplyCompSubst
    {
        /// The computation the substitution applies to.
        current: Comp,
        /// The environment variable being replaced.
        name: String,
        /// The remaining environment bindings after this substitution.
        rest: Option<Rc<EnvNode>>,
        /// Binder names shadowed inside the computation — left unsubstituted.
        shielded: Vec<String>,
    },
    /// Close a source-level stack under the remaining environment cursor.
    CloseStack
    {
        /// The stack being closed.
        current: Stack,
        /// The remaining environment bindings to substitute, innermost first.
        cursor: Option<Rc<EnvNode>>,
    },
    /// Apply one quoted environment replacement to a source-level stack.
    ApplyStackSubst
    {
        /// The stack the substitution applies to.
        current: Stack,
        /// The environment variable being replaced.
        name: String,
        /// The remaining environment bindings after this substitution.
        rest: Option<Rc<EnvNode>>,
    },
    /// Wrap a closed thunk body as a source value.
    WrapThunk(Grade),
    /// Wrap a closed stack as a source value.
    WrapStack,
}

impl Readback
{
    /// Create an empty readback engine.
    #[inline]
    #[must_use]
    fn new() -> Self
    {
        Self {
            work: Vec::new(),
            values: Vec::new(),
            comps: Vec::new(),
            stacks: Vec::new(),
        }
    }

    /// Quote one runtime value.
    #[must_use]
    fn quote(value: &RtValue) -> Value
    {
        let mut engine = Self::new();
        engine
            .work
            .push(ReadbackTask::QuoteDescend(Rc::new(value.clone())));
        engine.run();
        engine.take_value()
    }

    /// Close one computation under `env`, skipping `shielded` environment
    /// names.
    #[must_use]
    fn close_comp(
        comp: &Comp,
        env: &Env,
        shielded: Vec<String>,
    ) -> Comp
    {
        let mut engine = Self::new();
        engine.work.push(ReadbackTask::CloseComp {
            current: comp.clone(),
            cursor: env.0.clone(),
            shielded,
        });
        engine.run();
        engine.take_comp()
    }

    /// Drain the heap worklist.
    fn run(&mut self)
    {
        while let Some(task) = self.work.pop() {
            match task {
                | ReadbackTask::QuoteDescend(value) => self.quote_descend(&value),
                | ReadbackTask::QuoteCombine(value) => {
                    quote_combine(value.as_ref(), &mut self.values);
                },
                | ReadbackTask::CloseComp {
                    current,
                    cursor,
                    shielded,
                } => self.close_comp_step(current, cursor, shielded),
                | ReadbackTask::ApplyCompSubst {
                    current,
                    name,
                    rest,
                    shielded,
                } => {
                    let replacement = self.take_value();
                    let next = subst_comp(&current, &name, &replacement);
                    self.work.push(ReadbackTask::CloseComp {
                        current: next,
                        cursor: rest,
                        shielded,
                    });
                },
                | ReadbackTask::CloseStack { current, cursor } => {
                    self.close_stack_step(current, cursor);
                },
                | ReadbackTask::ApplyStackSubst {
                    current,
                    name,
                    rest,
                } => {
                    let replacement = self.take_value();
                    let next = subst_stack(&current, &name, &replacement);
                    self.work.push(ReadbackTask::CloseStack {
                        current: next,
                        cursor: rest,
                    });
                },
                | ReadbackTask::WrapThunk(grade) => {
                    let body = self.take_comp();
                    self.values.push(Value::Thunk(grade, Rc::new(body)));
                },
                | ReadbackTask::WrapStack => {
                    let stack = self.take_stack();
                    self.values.push(Value::Stk(Rc::new(stack)));
                },
            }
        }
    }

    /// Visit one runtime value.
    fn quote_descend(
        &mut self,
        value: &Rc<RtValue>,
    )
    {
        match *value.as_ref() {
            | RtValue::Var(ref name) => self.values.push(Value::Var(name.clone())),
            | RtValue::Unit => self.values.push(Value::Unit),
            | RtValue::Int(literal) => self.values.push(Value::Int(literal)),
            | RtValue::Str(ref literal) => self.values.push(Value::Str(literal.clone())),
            | RtValue::Num(literal) => self.values.push(Value::Num(literal)),
            | RtValue::Hole(id) => self.values.push(Value::Hole(id)),
            | RtValue::Thunk(grade, ref closure, _) => {
                let Some(body) = closure_body(closure)
                else {
                    self.values
                        .push(Value::Thunk(grade, Rc::new(Comp::Hole(0))));
                    return;
                };
                self.work.push(ReadbackTask::WrapThunk(grade));
                self.work.push(ReadbackTask::CloseComp {
                    current: body,
                    cursor: closure.env.0.clone(),
                    shielded: Vec::new(),
                });
            },
            | RtValue::Stk(ref stack, ref env) => {
                self.work.push(ReadbackTask::WrapStack);
                self.work.push(ReadbackTask::CloseStack {
                    current: stack.as_ref().clone(),
                    cursor: env.0.clone(),
                });
            },
            | RtValue::Annot(ref inner, _) => {
                self.work.push(ReadbackTask::QuoteCombine(Rc::clone(value)));
                self.work.push(ReadbackTask::QuoteDescend(Rc::clone(inner)));
            },
            | RtValue::Pair(ref fst, ref snd) => {
                self.work.push(ReadbackTask::QuoteCombine(Rc::clone(value)));
                self.work.push(ReadbackTask::QuoteDescend(Rc::clone(fst)));
                self.work.push(ReadbackTask::QuoteDescend(Rc::clone(snd)));
            },
            | RtValue::Inj(_, ref payload)
            | RtValue::Here(ref payload)
            | RtValue::Ctor { ref payload, .. } => {
                self.work.push(ReadbackTask::QuoteCombine(Rc::clone(value)));
                self.work
                    .push(ReadbackTask::QuoteDescend(Rc::clone(payload)));
            },
            | RtValue::List(ref elements) => {
                self.work.push(ReadbackTask::QuoteCombine(Rc::clone(value)));
                for element in elements {
                    self.work
                        .push(ReadbackTask::QuoteDescend(Rc::clone(element)));
                }
            },
            | RtValue::Record(ref fields) => {
                self.work.push(ReadbackTask::QuoteCombine(Rc::clone(value)));
                for field in fields.values() {
                    self.work.push(ReadbackTask::QuoteDescend(Rc::clone(field)));
                }
            },
        }
    }

    /// Advance one computation-close task.
    fn close_comp_step(
        &mut self,
        current: Comp,
        cursor: Option<Rc<EnvNode>>,
        shielded: Vec<String>,
    )
    {
        let Some(node) = cursor
        else {
            self.comps.push(current);
            return;
        };
        let rest = node.rest.clone();
        if shielded.iter().any(|name| name == &node.name) {
            self.work.push(ReadbackTask::CloseComp {
                current,
                cursor: rest,
                shielded,
            });
            return;
        }
        self.work.push(ReadbackTask::ApplyCompSubst {
            current,
            name: node.name.clone(),
            rest,
            shielded,
        });
        self.work
            .push(ReadbackTask::QuoteDescend(Rc::clone(&node.value)));
    }

    /// Advance one stack-close task.
    fn close_stack_step(
        &mut self,
        current: Stack,
        cursor: Option<Rc<EnvNode>>,
    )
    {
        let Some(node) = cursor
        else {
            self.stacks.push(current);
            return;
        };
        self.work.push(ReadbackTask::ApplyStackSubst {
            current,
            name: node.name.clone(),
            rest: node.rest.clone(),
        });
        self.work
            .push(ReadbackTask::QuoteDescend(Rc::clone(&node.value)));
    }

    /// Pop one readback value result.
    fn take_value(&mut self) -> Value
    {
        debug_assert!(
            !self.values.is_empty(),
            "readback value stack must not underflow (post-order balance)"
        );
        self.values.pop().unwrap_or(Value::Unit)
    }

    /// Pop one readback computation result.
    fn take_comp(&mut self) -> Comp
    {
        debug_assert!(
            !self.comps.is_empty(),
            "readback computation stack must not underflow (post-order balance)"
        );
        self.comps.pop().unwrap_or_else(|| Comp::ret(Value::Unit))
    }

    /// Pop one readback stack result.
    fn take_stack(&mut self) -> Stack
    {
        debug_assert!(
            !self.stacks.is_empty(),
            "readback stack result must not underflow (post-order balance)"
        );
        self.stacks.pop().unwrap_or(Stack::Empty)
    }
}

/// Reads a closure body by canonical arena id.
///
/// # Contract
/// - ensures: returns the stored body when the closure's id belongs to its
///   arena; returns `None` instead of panicking for an impossible corrupted
///   closure.
/// - panics: none.
#[inline]
#[must_use]
fn closure_body(closure: &Closure) -> Option<Comp>
{
    closure.arena.comp(closure.body).ok()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Closure
{
    /// The captured value environment.
    env: Env,
    /// The suspended computation body id.
    body: ClosureBodyId,
    /// The arena set containing the suspended computation body and its value /
    /// stack children.
    arena: Rc<FlatArena>,
}

impl Closure
{
    /// Captures `body` under `env` as a canonical arena-backed first-order
    /// closure.
    ///
    /// # Contract
    /// - ensures: returns a closure whose body resolves through [`CompNodeId`]
    ///   in a [`FlatArena`], not through structural `Rc<Comp>` ownership.
    /// - panics: none.
    #[inline]
    #[must_use]
    fn from_body(
        env: Env,
        body: &Comp,
    ) -> Option<Self>
    {
        let (arena, body) = closure_body_arena(body)?;
        Some(Self { env, body, arena })
    }
}

/// The **call-by-need memoization cell** of a [`RtValue::Thunk`] (ADR-50).
///
/// The explicit backing store the discipline requires — Rust has no free GHC
/// laziness, and Idris 2's un-memoized CBN is the named negative precedent.
///
/// Forcing a thunk once caches its weak-head form here, so a re-force through
/// any shared copy of the thunk value reuses it; the [`ThunkMemo::InProgress`]
/// state is the **black hole** marking an in-progress force (cycle detection).
///
/// Its identity is intentionally invisible to `Eq` / `Debug` (a memo is a
/// derived cache, not part of the value's meaning), so two structurally equal
/// thunks compare equal regardless of force state and [`RtValue`] keeps its
/// derived equality.
#[derive(Clone)]
#[repr(transparent)]
pub struct MemoCell(Rc<RefCell<ThunkMemo>>);

impl MemoCell
{
    /// A fresh, unforced memo cell.
    #[inline]
    #[must_use]
    fn unforced() -> Self
    {
        Self(Rc::new(RefCell::new(ThunkMemo::Unforced)))
    }
}

impl PartialEq for MemoCell
{
    /// A memo cell carries no semantic content, so all cells compare equal —
    /// keeping [`RtValue`]'s derived [`Eq`] a structural equality that ignores
    /// force state.
    #[inline]
    fn eq(
        &self,
        other: &Self,
    ) -> bool
    {
        let _ = other;
        true
    }
}

impl Eq for MemoCell
{
}

impl core::fmt::Debug for MemoCell
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str("<memo>")
    }
}

/// The state of a thunk's [`MemoCell`] (ADR-50 call-by-need).
enum ThunkMemo
{
    /// Not yet forced.
    Unforced,
    /// Currently being forced — the **black hole**; re-entry is a cycle. In v0
    /// (no recursion) a thunk cannot refer to itself, so this state is
    /// transient (set only across a force probe) and the cycle path is
    /// unreachable — a genuine cycle would be bounded by the
    /// [`STEP_BUDGET`] safety net.
    InProgress,
    /// Forced to a weak-head normal form: the reduced terminal computation and
    /// the environment it runs under (the cached value).
    Forced
    {
        /// The reduced terminal computation.
        comp: Comp,
        /// The environment the terminal runs under.
        env: Env,
    },
}

/// One task on the [`eval_value`] worklist — the defunctionalized image of a
/// recursive evaluation call (ADR-47).
enum EvalTask<'src>
{
    /// Visit a source value, pushing a combine plus its children, or a leaf.
    Descend(&'src Value),
    /// Reassemble a runtime value from its children on the result stack.
    Combine(&'src Value),
}

/// Splices a **source-level** captured stack `K` (a `stk K` value) on top of
/// the continuation `cont` (rule Resume dynamics; `effects-control-shell.md`
/// §2.1; ADR-34 D2).
///
/// A `stk K` value is structural ([`crate::syntax::Stack`]); a *runtime*
/// captured continuation (from `shift` / `perform`) instead lives in
/// [`ContEnv`] and is spliced by [`Comp::Resume`] directly. The stack's
/// argument values are evaluated under the stack closure's captured environment
/// `env`, and its bind frames carry that environment (ADR-50 Decision C). The
/// stack head ends on top (consumed first) — the continuation reading.
///
/// # Contract
/// - ensures: `cont` gains the stack's frames (arguments evaluated under `env`,
///   bind bodies carrying `env`) so the stack head ends on top.
/// - panics: none.
fn splice(
    stack: &Stack,
    env: &Env,
    cont: &mut Vec<Cont>,
)
{
    // Collect head-first, then push reversed so the head ends on top.
    let mut frames: Vec<Cont> = Vec::new();
    let mut current = stack;
    loop {
        match *current {
            | Stack::Empty => break,
            | Stack::Arg(ref value, ref rest) => {
                frames.push(Cont::Arg(eval_value(value, env)));
                current = rest;
            },
            | Stack::Bind(ref name, ref body, ref rest) => {
                frames.push(Cont::Bind(name.clone(), Rc::clone(body), env.clone()));
                current = rest;
            },
            | Stack::Prj(side, ref rest) => {
                frames.push(Cont::Prj(side));
                current = rest;
            },
        }
    }
    for frame in frames.into_iter().rev() {
        cont.push(frame);
    }
}

/// Reassembles a source value from its already-built children (children pop in
/// source order).
fn quote_combine(
    value: &RtValue,
    out: &mut Vec<Value>,
)
{
    let rebuilt = match *value {
        | RtValue::Pair(..) => {
            let fst = pop_value(out);
            let snd = pop_value(out);
            Value::Pair(Rc::new(fst), Rc::new(snd))
        },
        | RtValue::Inj(side, _) => Value::Inj(side, Rc::new(pop_value(out))),
        | RtValue::List(ref elements) => {
            let mut built = Vec::with_capacity(elements.len());
            for _ in elements {
                built.push(Rc::new(pop_value(out)));
            }
            Value::List(built)
        },
        | RtValue::Record(ref fields) => {
            let mut built = BTreeMap::new();
            for label in fields.keys() {
                built.insert(label.clone(), Rc::new(pop_value(out)));
            }
            Value::Record(built)
        },
        | RtValue::Annot(_, ref ty) => Value::Annot(Rc::new(pop_value(out)), Rc::clone(ty)),
        | RtValue::Here(_) => Value::Here(Rc::new(pop_value(out))),
        | RtValue::Ctor { ref id, tag, .. } => Value::Ctor {
            id: id.clone(),
            tag,
            payload: Rc::new(pop_value(out)),
        },
        // Leaves and closures never reach a combine; the arm is required only
        // for exhaustiveness.
        | _ => return,
    };
    out.push(rebuilt);
}

/// Pops the most-recent source value from a worklist result stack (see
/// [`pop_rt`]).
fn pop_value(out: &mut Vec<Value>) -> Value
{
    debug_assert!(
        !out.is_empty(),
        "quote_value worklist underflow (post-order balance)"
    );
    out.pop().unwrap_or(Value::Unit)
}
/// Closes a computation against a value environment (ADR-50 Decision D
/// readback): each free environment variable is replaced by the readback of the
/// value it denotes, innermost binding first. Quotation and closing share the
/// [`Readback`] trampoline, so environment values that themselves contain
/// closures do not re-enter readback on the host stack.
///
/// # Contract
/// - ensures: returns `comp` with every free environment variable substituted
///   by its readback, respecting binder shadowing within `comp`.
/// - panics: none.
#[must_use]
/// # Termination
/// - reason: evaluator drains a finite readback worklist.
/// - measure: pending readback tasks plus result-frame stacks.
/// - boundedness: runtime environments and source computations are finite Rust
///   values.
/// - input recursion: none.
fn close_comp(
    comp: &Comp,
    env: &Env,
) -> Comp
{
    Readback::close_comp(comp, env, Vec::new())
}

/// Capture-avoiding substitution of `repl` for the free occurrences of `name`
/// in a computation (`effects-control-shell.md` §4; ADR-34 D2).
///
/// Substitution descends into every sub-term, stopping at a binder that
/// **rebinds** `name` (so an inner binding shadows the substitution). `repl` is
/// a closed value in well-typed closed programs, so no variable capture can
/// occur; the shadowing discipline keeps it correct regardless.
///
/// The traversal is **iterative** — an explicit heap worklist ([`Subst`]), not
/// host recursion — so an adversarially deep term (e.g. the `O(list-length)`
/// native-combinator unroll) substitutes on the heap and cannot
/// overflow the host call stack (ADR-47; Rust has no tail-call optimisation, so
/// recursion whose depth follows unbounded input is a latent abort). The result
/// is structurally identical to the direct recursive definition, which is
/// retained as the differential reference `subst_comp_recursive` in the tests.
///
/// # Contract
/// - ensures: returns `comp` with every free `name` replaced by `repl`, leaving
///   occurrences under a rebinding of `name` untouched.
/// - panics: none (the worklist's post-order balance keeps every result pop
///   defined; a `debug_assert` guards the invariant in test / debug builds).
#[must_use]
fn subst_comp<'source, N>(
    comp: &Comp,
    name: N,
    repl: &Value,
) -> Comp
where
    N: Into<NameRef<'source>>,
{
    let mut engine = Subst::new(name.into(), repl);
    engine.work.push(Task::DescendComp(comp));
    engine.run();
    engine.take_comp()
}
/// Peels type annotations off a value: `(v : A)` is operationally `v` (the
/// ascription is typing-only), so a value is inspected for its *constructor*
/// only after stripping any [`Value::Annot`] layers.
///
/// # Contract
/// - ensures: returns the innermost non-annotation value reachable through
///   nested [`Value::Annot`] wrappers (the value itself when it is not an
///   annotation).
/// - panics: none.
#[inline]
#[must_use]
fn strip_annot(value: &Value) -> &Value
{
    let mut current = value;
    while let Value::Annot(ref inner, _) = *current {
        current = inner;
    }
    current
}

/// Reduces a list elimination `case v { Nil ⇒ nil | Cons(head, tail) ⇒ cons }`
/// against its (annotation-stripped) scrutinee (rule `ListCase` dynamics;
/// ADR-40 D4). Shared by the recursive reference [`eval_comp`] and the CEK
/// machine [`drive`] so the two faces reduce a list elimination identically.
///
/// An empty list selects the `nil` arm; a non-empty list `[h, …rest]` selects
/// the `cons` arm with `head ↦ h` and `tail ↦ [rest]`. The `tail` binder is
/// substituted **first** (when `head == tail` the later-bound `tail` wins,
/// matching the typing's `Γ` shadowing — `head` is bound before `tail`). A hole
/// scrutinee blames; any other value is an (ill-typed) undefined stuck.
///
/// # Contract
/// - ensures: returns [`ListReduce::Body`] of the selected arm with the `cons`
///   binders substituted (capture-avoiding; the scrutinee's elements are closed
///   in a well-typed closed program), or [`ListReduce::Final`] for a hole blame
///   / non-list stuck.
/// - panics: none.
fn reduce_list_case<'source, H, T>(
    scrut: &Value,
    nil: &Rc<Comp>,
    head: H,
    tail: T,
    cons: &Rc<Comp>,
) -> ListReduce
where
    H: Into<BinderName<'source>>,
    T: Into<BinderName<'source>>,
{
    let head = head.into();
    let tail = tail.into();
    match *strip_annot(scrut) {
        | Value::List(ref elements) => match elements.split_first() {
            | None => ListReduce::Body(nil.as_ref().clone()),
            | Some((first, rest)) => {
                let tail_value = Value::List(rest.to_vec());
                // `tail` first (it is bound innermost), then `head`: when the
                // two names collide the later binder wins, as the typing's `Γ`.
                let once = subst_comp(cons, NameRef::from(tail.as_ref()), &tail_value);
                ListReduce::Body(subst_comp(
                    &once,
                    NameRef::from(head.as_ref()),
                    first.as_ref(),
                ))
            },
        },
        | Value::Hole(_) => ListReduce::Final(Eval::Blame(Blame::Hole)),
        | _ => ListReduce::Final(Eval::Stuck(StuckReason::ListCasedNonList)),
    }
}
/// Closes a computation against a value environment, **shielding** a set of
/// binder names (a frame's own binders) — an environment binding whose name is
/// shielded is skipped, so an occurrence bound by the reconstructed frame stays
/// free (see [`State::reconstruct`]). Only for the subject-reduction oracle's
/// term reconstruction, never the hot path.
///
/// # Contract
/// - ensures: returns `comp` closed under `env` except for the shielded names,
///   respecting binder shadowing within `comp`.
/// - panics: none.
#[must_use]
fn close_comp_shielding<'source, S>(
    comp: &Comp,
    env: &Env,
    shielded: S,
) -> Comp
where
    S: Into<ShieldedNames<'source>>,
{
    let shielded = shielded.into();
    let owned = shielded
        .as_ref()
        .iter()
        .map(|&name| name.to_owned())
        .collect();
    Readback::close_comp(comp, env, owned)
}

/// Reads a runtime value [`RtValue`] back to a source [`Value`] (`NbE`
/// quote / readback; ADR-50 Decision D).
///
/// The inverse of [`eval_value`] on the closed fragment. A [`RtValue::Thunk`] /
/// [`RtValue::Stk`] closure is CLOSED by pushing its captured computation or
/// stack plus environment onto the [`Readback`] worklist, so the suspended term
/// is reconstructed with its free variables resolved.
///
/// The traversal is an explicit heap worklist (ADR-47), depth-robust over the
/// value spine; closure closing and environment quotation share the same
/// [`Readback`] trampoline, so nested closure environments do not re-enter
/// [`quote_value`] on the host stack.
///
/// # Contract
/// - ensures: `quote_value(eval_value(v, env))` reproduces `v` closed under
///   `env` (the identity on a closed pure value under the empty environment).
/// - panics: none (a `debug_assert` guards the post-order balance in debug /
///   test builds).
#[inline]
#[must_use]
/// # Termination
/// - reason: evaluator drains a finite readback worklist.
/// - measure: pending readback tasks plus result-frame stacks.
/// - boundedness: runtime values, environments, and source terms are finite
///   Rust values allocated before readback.
/// - input recursion: none.
pub fn quote_value(value: &RtValue) -> Value
{
    Readback::quote(value)
}

/// Drives a `perform op v` (rule Op dynamics; `effects-control-shell.md` §1.1;
/// ADR-34 D2): walk the continuation to the nearest [`Cont::Handle`] declaring
/// `op`, capture the delimited continuation **with the handler reinstalled in
/// it** (deep, ADR-33 D4) as the clause's resumption binder `k` in `env`, and
/// run that operation's clause body — *outside* the handler — with the payload
/// `v` substituted.
///
/// The clause runs over the continuation *below* the handler (so a non-resuming
/// clause's result is the handle's result, not re-handled); the captured
/// continuation carries a copy of the handler, so `resume k …` re-enters it
/// (deep). The resumption binder `k` is **α-renamed to a fresh name** (so
/// nested clauses reusing one name never collide in `env`). Routing is by op
/// name (the one-name-one-signature invariant, ADR-33 D3), so the `perform`'s
/// own inline signature is not needed at runtime.
///
/// # Contract
/// - ensures: on a reachable matching handler across a structural prefix, binds
///   a fresh name to the captured continuation (handler included), and returns
///   the clause body `tᵢ` under the handler-site environment extended with the
///   payload `v` bound to `p`, with `k` α-renamed to that fresh name, over the
///   continuation below the handler; otherwise a [`Blame::PerformNoHandler`]
///   (no matching handler, an undeclared op, or a capture that would cross a
///   delimiter / unrelated handler — the v0 single-handler scope).
/// - panics: none.
fn drive_perform<'source, O>(
    op: O,
    arg: &Value,
    env: &Env,
    cont: &mut Vec<Cont>,
    contenv: &mut ContEnv,
    gensym: &mut Gensym<GandrSort>,
) -> Transition
where
    O: Into<OperationName<'source>>,
{
    // The payload is evaluated under the perform-site environment.
    let payload = eval_value(arg, env);
    let Some((handle_index, clause, handler_env)) = capture_to_handler(cont, op)
    else {
        return Transition::Final(Eval::Blame(Blame::PerformNoHandler));
    };
    // The captured continuation is `[Handle, …structural prefix]`: it carries
    // the handler so the clause's resumption re-enters it (deep). The clause
    // body runs below the (removed) handler, under the handler-site environment.
    let handle_index = usize::from(handle_index);
    let captured = cont
        .get(handle_index ..)
        .map(<[Cont]>::to_vec)
        .unwrap_or_default();
    let Ok(fresh) = fresh_name(gensym)
    else {
        return Transition::Final(Eval::Stuck(StuckReason::FreshContinuationNameExhausted));
    };
    cont.truncate(handle_index);
    // Bind the payload in the clause environment, then α-rename the resumption
    // binder to the fresh continuation key. When `p` and `k` share a name the
    // resumption binds innermost (rule Handle binds `p` then `k`), so the
    // payload binding is skipped — `k` (the continuation) wins (its occurrences
    // are renamed to the fresh key), matching the typing.
    let clause_env = if clause.payload == clause.resume {
        handler_env
    }
    else {
        handler_env.extend(clause.payload.clone(), payload)
    };
    let renamed = subst_comp(&clause.body, &clause.resume, &Value::Var(fresh.clone()));
    contenv.push((fresh, Rc::from(captured)));
    Transition::Focus(renamed, clause_env)
}

/// Capture-avoiding substitution into a reified stack — the [`subst_comp`]
/// companion for a `stk K` value's frames, over the same iterative [`Subst`]
/// engine (ADR-47).
///
/// # Contract
/// - ensures: returns `stack` with every free `name` replaced by `repl`.
/// - panics: none.
#[must_use]
fn subst_stack<'source, N>(
    stack: &Stack,
    name: N,
    repl: &Value,
) -> Stack
where
    N: Into<NameRef<'source>>,
{
    let mut engine = Subst::new(name.into(), repl);
    engine.work.push(Task::DescendStack(stack));
    engine.run();
    engine.take_stack()
}

/// Directed reduction tests: the pure CBPV spine drives to its normal form
/// (with the recursive reference agreeing), the grade ops and holes have their
/// operational outcomes, the effect / control operators round-trip on the CEK
/// machine, and a deep continuation runs on the heap-allocated continuation
/// (the eval analogue of `machine`'s depth test).
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::boundary::I64Literal;
    use crate::boundary::NodeIndex;
    use crate::effect::EffectOp;
    use crate::effect::EffectSig;
    use crate::grade::Grade;
    use crate::types::CompType;
    use crate::types::ValueType;
    /// Walk-β on a `here` scrutinee (ADR-76): `walk(here(7), C, (x). ret
    /// here(x)) ↦ (ret here(x))[7/x] = ret here(7)`. The reference
    /// evaluator and the CEK machine agree, and the closed `Path`-typed
    /// result reads back as `here(7)` — **canonicity**, witnessed, no
    /// postulate.
    #[test]
    fn walk_beta_reduces_here_and_the_result_is_canonical()
    {
        let motive = crate::syntax::WalkMotive::new(
            "x",
            "y",
            "q",
            CompType::returner(ValueType::path(
                ValueType::integer(),
                Value::var("y"),
                Value::var("x"),
            )),
        );
        let base = WalkBase::new("x", Comp::ret(Value::here(Value::var("x"))));
        let j = Comp::walk(Value::here(Value::int(7)), motive, base);
        assert_eq!(
            agree(j),
            Eval::Value(Comp::ret(Value::here(Value::int(7)))),
            "Walk-β reduces to ret here(7), a canonical Path-typed value"
        );
    }
    /// Walk-β threads the witness (not the whole `here`) into the base binder:
    /// a projecting base `(x). ret x` yields `ret 7`, agreeing on both
    /// evaluators.
    #[test]
    fn walk_beta_binds_the_witness_not_the_proof()
    {
        let motive =
            crate::syntax::WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer()));
        let base = WalkBase::new("x", Comp::ret(Value::var("x")));
        let j = Comp::walk(Value::here(Value::int(7)), motive, base);
        assert_eq!(
            agree(j),
            Eval::Value(Comp::ret(Value::int(7))),
            "the base binder receives the witness 7, not here(7)"
        );
    }
    /// A non-`here` scrutinee is `WalkOnNonHere` on both evaluators (ill-typed
    /// input; a well-typed closed `Path` scrutinee is always a `here`).
    #[test]
    fn walk_on_a_non_here_scrutinee_is_stuck()
    {
        let motive =
            crate::syntax::WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer()));
        let base = WalkBase::new("x", Comp::ret(Value::var("x")));
        let j = Comp::walk(Value::int(7), motive, base);
        assert_eq!(Eval::Stuck(StuckReason::WalkOnNonHere), agree(j));
    }

    /// `quote_value ∘ eval_value` round-trips a closed pure source value under
    /// the empty environment — the readback of the runtime image reproduces the
    /// original value (the ADR-50 Decision D identity on the closed fragment).
    #[test]
    fn quote_round_trips_a_closed_pure_value()
    {
        let values = alloc::vec![
            Value::Unit,
            Value::int(7),
            Value::string("hi"),
            Value::u32(3),
            Value::pair(Value::int(1), Value::Unit),
            Value::inj1(Value::int(2)),
            Value::inj2(Value::pair(Value::int(1), Value::int(2))),
            Value::list(alloc::vec![Value::int(1), Value::int(2), Value::int(3)]),
            Value::record([
                ("a".to_owned(), Value::int(1)),
                ("b".to_owned(), Value::Unit),
            ]),
            // A thunk closes to its (empty-env) body on readback.
            Value::thunk(Grade::ONE, Comp::ret(Value::int(5))),
            // Nested compound with a thunk leaf.
            Value::pair(
                Value::inj1(Value::int(1)),
                Value::thunk(Grade::ONE, Comp::ret(Value::Unit)),
            ),
            // An annotation is preserved (peeled only at an elimination), so a
            // returned annotated value reads back identically to the reference.
            Value::annot(Value::int(3), ValueType::integer()),
        ];
        for value in values {
            let runtime = eval_value(&value, &Env::empty());
            let back = quote_value(&runtime);
            assert_eq!(
                back, value,
                "quote_value ∘ eval_value must round-trip the closed value"
            );
        }
    }

    /// `eval_value` resolves a bound variable against the environment, and a
    /// thunk capturing it CLOSES to the bound value on readback (the free
    /// variable is resolved into the reconstructed suspended term).
    #[test]
    fn eval_value_resolves_a_bound_variable()
    {
        let env = Env::empty().extend("x".to_owned(), Rc::new(RtValue::Int(9)));
        let resolved = eval_value(&Value::var("x"), &env);
        assert_eq!(
            quote_value(&resolved),
            Value::int(9),
            "a bound variable resolves to its environment value"
        );
        let captured = eval_value(&Value::thunk(Grade::ONE, Comp::ret(Value::var("x"))), &env);
        assert_eq!(
            quote_value(&captured),
            Value::thunk(Grade::ONE, Comp::ret(Value::int(9))),
            "a thunk capturing x closes to the bound value on readback"
        );
    }

    /// Thunk runtime closures carry a canonical computation-node body id rather
    /// than owning an `Rc<Comp>` body in the closure record.
    #[test]
    fn thunk_closure_body_is_arena_addressed()
    {
        let thunk = Value::thunk(Grade::ONE, Comp::ret(Value::int(5)));
        let runtime = eval_value(&thunk, &Env::empty());
        match runtime.as_ref() {
            | &RtValue::Thunk(_, ref closure, _) => {
                assert_eq!(NodeIndex::from(0), closure.body.index());
                assert_eq!(
                    closure_body(closure),
                    Some(Comp::ret(Value::int(5))),
                    "closure body id resolves through its arena"
                );
            },
            | other => panic!("expected thunk runtime value, got {other:?}"),
        }
    }

    // ── Phase 1a: the runtime value domain (eval_value / quote_value) ─────────

    /// A deeply nested value evaluates and quotes on the heap worklists instead
    /// of overflowing the host stack (ADR-47); reverting either walk to
    /// recursion aborts this test (the guard property).
    #[test]
    fn quote_is_depth_robust()
    {
        let worker = std::thread::Builder::new()
            .stack_size(QUOTE_STACK)
            .spawn(|| {
                let mut value = Value::int(0);
                for _ in 0 .. QUOTE_DEPTH {
                    value = Value::inj1(value);
                }
                let runtime = eval_value(&value, &Env::empty());
                let back = quote_value(&runtime);
                // Verify the round-trip by an ITERATIVE walk of the readback
                // (a recursive `assert_eq!` would itself overflow at this
                // depth): count the `inj1` layers down to the `0` leaf.
                let mut cursor = &back;
                let mut depth = 0_usize;
                while let Value::Inj(Side::Fst, ref inner) = *cursor {
                    depth += 1;
                    cursor = inner;
                }
                assert!(
                    matches!(*cursor, Value::Int(0)),
                    "the readback must bottom out at the original leaf"
                );
                assert_eq!(
                    QUOTE_DEPTH, depth,
                    "the readback must reproduce the {QUOTE_DEPTH}-deep injection chain"
                );
            })
            .expect("spawn the pinned-stack worker thread");
        worker
            .join()
            .expect("deep eval / quote must not overflow the host stack");
    }

    /// A bare returner is already terminal.
    #[test]
    fn ret_is_terminal()
    {
        assert_eq!(agree(Comp::ret(Value::int(0))), ret_int(0));
    }

    /// `ret v >>= x. u` substitutes the returned value into the continuation.
    #[test]
    fn bind_substitutes_the_returned_value()
    {
        let comp = Comp::bind(Comp::ret(Value::int(5)), "x", Comp::ret(Value::var("x")));
        assert_eq!(agree(comp), ret_int(5));
    }

    /// Depth for the `quote_value` / `eval_value` iterative-worklist regression
    /// (ADR-47). A recursive readback of a chain this deep overflows the host
    /// stack well below this depth; the worklists descend it on the heap. The
    /// depth sits above the recursive-walk ceiling and below the (still
    /// recursive) `Drop` ceiling on the pinned 4 MiB worker stack —
    /// verification is an ITERATIVE walk (no recursive `PartialEq`), so
    /// only the deep `Drop` at teardown, comfortably within the window,
    /// stays recursive.
    const QUOTE_DEPTH: usize = 8_000;

    /// The pinned worker-thread stack for the depth regression.
    const QUOTE_STACK: usize = 4 * 1024 * 1024;

    /// `force (thunk t)` runs the thunked computation.
    #[test]
    fn force_runs_the_thunk_body()
    {
        let comp = Comp::force(Value::thunk(Grade::ONE, Comp::ret(Value::int(7))));
        assert_eq!(agree(comp), ret_int(7));
    }

    /// A thunk bound to a variable and forced twice shares one memoization cell
    /// (ADR-50 call-by-need): both forces resolve to the SAME runtime thunk
    /// value (env lookup returns the shared `Rc`), so the second reuses the
    /// cached weak-head form. Memoization is transparent, so the observable
    /// result is `ret (5, 5)` and the substitution reference agrees.
    #[test]
    fn forcing_a_shared_thunk_twice_is_memoized_and_transparent()
    {
        // (λt. force t >>= x. force t >>= y. ret (x, y)) (thunk (ret 5))
        let comp = Comp::app(
            Comp::lam(
                "t",
                Comp::bind(
                    Comp::force(Value::var("t")),
                    "x",
                    Comp::bind(
                        Comp::force(Value::var("t")),
                        "y",
                        Comp::ret(Value::pair(Value::var("x"), Value::var("y"))),
                    ),
                ),
            ),
            Value::thunk(Grade::ONE, Comp::ret(Value::int(5))),
        );
        assert_eq!(
            agree(comp),
            Eval::Value(Comp::ret(Value::pair(Value::int(5), Value::int(5))))
        );
    }

    /// A thunk closure captures the environment at the suspension site, and the
    /// memoized weak-head form keeps that captured environment on re-force. The
    /// later `x = 2` binding must not retarget the shared thunk's body.
    #[test]
    fn shared_thunk_memoization_keeps_its_captured_environment()
    {
        // x = 1; t = thunk(ret x); a = force t; x = 2; b = force t; ret (a, b)
        let comp = Comp::bind(
            Comp::ret(Value::int(1)),
            "x",
            Comp::bind(
                Comp::ret(Value::thunk(Grade::ONE, Comp::ret(Value::var("x")))),
                "t",
                Comp::bind(
                    Comp::force(Value::var("t")),
                    "a",
                    Comp::bind(
                        Comp::ret(Value::int(2)),
                        "x",
                        Comp::bind(
                            Comp::force(Value::var("t")),
                            "b",
                            Comp::ret(Value::pair(Value::var("a"), Value::var("b"))),
                        ),
                    ),
                ),
            ),
        );
        assert_eq!(
            run_comp(comp),
            Eval::Value(Comp::ret(Value::pair(Value::int(1), Value::int(1))))
        );
    }

    /// Applying the result of a lambda application uses the function terminal's
    /// captured environment. The inner `λy. ret x` must retain `x = 1` after
    /// the outer application returns it as a first-order closure.
    #[test]
    fn lambda_application_uses_the_closure_environment()
    {
        let comp = Comp::app(
            Comp::app(
                Comp::lam("x", Comp::lam("y", Comp::ret(Value::var("x")))),
                Value::int(1),
            ),
            Value::int(2),
        );
        assert_eq!(run_comp(comp), ret_int(1));
    }

    /// `(λx. ret x) v` β-reduces.
    #[test]
    fn application_beta_reduces()
    {
        let comp = Comp::app(Comp::lam("x", Comp::ret(Value::var("x"))), Value::int(3));
        assert_eq!(agree(comp), ret_int(3));
    }

    /// `case (inj1 w) …` runs the first arm under `w`; `inj2` the second.
    #[test]
    fn case_selects_the_injected_arm()
    {
        let fst = Comp::case(
            Value::inj1(Value::int(1)),
            "x",
            Comp::ret(Value::var("x")),
            "y",
            Comp::ret(Value::int(0)),
        );
        assert_eq!(agree(fst), ret_int(1));
        let snd = Comp::case(
            Value::inj2(Value::int(2)),
            "x",
            Comp::ret(Value::int(0)),
            "y",
            Comp::ret(Value::var("y")),
        );
        assert_eq!(agree(snd), ret_int(2));
    }

    /// `case [] { Nil ⇒ ret 0 | Cons(h, t) ⇒ ret h }` selects the `nil` arm
    /// on the empty list (ADR-40 D4).
    #[test]
    fn list_case_selects_nil_on_empty()
    {
        let comp = Comp::list_case(
            Value::list(alloc::vec![]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        );
        assert_eq!(agree(comp), ret_int(0));
    }

    /// `case [1, 2, 3] { Nil ⇒ ret 0 | Cons(h, t) ⇒ ret h }` selects the `cons`
    /// arm, binding `head` to the first element.
    #[test]
    fn list_case_selects_cons_head_on_nonempty()
    {
        let comp = Comp::list_case(
            Value::list(alloc::vec![Value::int(1), Value::int(2), Value::int(3)]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        );
        assert_eq!(agree(comp), ret_int(1));
    }

    /// The `cons` arm's `tail` binds the rest of the list: a second `case` over
    /// `t` reads `2` from `[1, 2, 3]`'s tail `[2, 3]`.
    #[test]
    fn list_case_cons_tail_is_the_rest()
    {
        let inner = Comp::list_case(
            Value::var("t"),
            Comp::ret(Value::int(0)),
            "h2",
            "t2",
            Comp::ret(Value::var("h2")),
        );
        let comp = Comp::list_case(
            Value::list(alloc::vec![Value::int(1), Value::int(2), Value::int(3)]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            inner,
        );
        assert_eq!(agree(comp), ret_int(2));
    }

    /// A list-case on a non-list scrutinee is an undefined stuck (the oracle
    /// pins these out of the well-typed fragment).
    #[test]
    fn list_case_on_non_list_is_stuck()
    {
        let comp = Comp::list_case(
            Value::int(0),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        );
        assert_eq!(
            Eval::Stuck(StuckReason::ListCasedNonList),
            eval_comp(comp.clone())
        );
        assert_eq!(Eval::Stuck(StuckReason::ListCasedNonList), run_comp(comp));
    }

    /// `split (v1, v2) as (x, y) in t` binds both components.
    #[test]
    fn split_binds_both_components()
    {
        let comp = Comp::split(
            Value::pair(Value::int(1), Value::int(2)),
            "x",
            "y",
            Comp::ret(Value::var("y")),
        );
        assert_eq!(agree(comp), ret_int(2));
    }

    /// **Adversary regression.** A `split` with two identically
    /// named binders `(x, x)` binds `x` to the *second* component in BOTH
    /// typing and eval — the inner binder wins (`ctx.lookup` is
    /// innermost-wins, and the body is typed under that inner type). Before
    /// the fix, eval substituted the *first* binder first, so its
    /// substitution consumed every free `x` and the second was a no-op: `x`
    /// got the first component, and this closed, well-typed, `Unknown`-free
    /// term got stuck — a rigid-fragment soundness violation. `split (1,
    /// inj1 ()) as (x, x) in case x { … }` types `x` at the inner `Sum(1,
    /// 1)`, so the `case` is well-typed, and eval must select the `inj1`
    /// arm.
    #[test]
    fn split_with_colliding_binders_binds_the_inner()
    {
        let bool_ty = ValueType::sum(ValueType::Unit, ValueType::Unit);
        let comp = Comp::split(
            Value::pair(
                Value::int(1),
                Value::annot(Value::inj1(Value::Unit), bool_ty),
            ),
            "x",
            "x",
            Comp::case(
                Value::var("x"),
                "a",
                Comp::ret(Value::int(7)),
                "b",
                Comp::ret(Value::int(8)),
            ),
        );
        // Closed and well-typed against `F Integer` (the inner `x : Sum(1, 1)`
        // is what the `case` scrutinee needs)...
        let typed = crate::checker::check_comp(
            crate::ctx::Ctx::new(),
            comp.clone(),
            CompType::returner(ValueType::integer()),
        );
        assert!(
            typed.is_ok(),
            "the colliding-binder split must type-check: {typed:?}"
        );
        // ...and eval binds `x` to the inner (second) component — the injection —
        // selecting the `inj1` arm (`ret 7`), not getting stuck on `1`.
        assert_eq!(agree(comp), ret_int(7));
    }

    /// `prjᵢ ⟨t1, t2⟩` selects the projected component.
    #[test]
    fn projection_selects_the_component()
    {
        let comp = Comp::prj1(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        ));
        assert_eq!(agree(comp), ret_int(1));
        let snd = Comp::prj2(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        ));
        assert_eq!(agree(snd), ret_int(2));
    }

    /// `{ℓ = v, …}.ℓ` projects the field's value (ADR-45 D4); the field is read
    /// directly off the inert record, by label, in any field order.
    #[test]
    fn record_projection_selects_the_field()
    {
        let record = Value::record([
            ("a".to_owned(), Value::int(1)),
            ("b".to_owned(), Value::int(2)),
        ]);
        assert_eq!(agree(Comp::record_proj(record.clone(), "a")), ret_int(1));
        assert_eq!(agree(Comp::record_proj(record, "b")), ret_int(2));
    }

    /// A record projection on a non-record scrutinee is an undefined stuck (the
    /// oracle pins these out of the well-typed fragment).
    #[test]
    fn record_projection_on_non_record_is_stuck()
    {
        let comp = Comp::record_proj(Value::int(0), "a");
        assert_eq!(
            Eval::Stuck(StuckReason::RecordProjNonRecord),
            eval_comp(comp.clone())
        );
        assert_eq!(
            Eval::Stuck(StuckReason::RecordProjNonRecord),
            run_comp(comp)
        );
    }

    /// A record projection of a field the record does not carry is an undefined
    /// stuck (the type checker rules it out for a well-typed term).
    #[test]
    fn record_projection_missing_field_is_stuck()
    {
        let record = Value::record([("a".to_owned(), Value::int(1))]);
        let comp = Comp::record_proj(record, "b");
        assert_eq!(
            Eval::Stuck(StuckReason::RecordProjMissingField),
            eval_comp(comp.clone())
        );
        assert_eq!(
            Eval::Stuck(StuckReason::RecordProjMissingField),
            run_comp(comp)
        );
    }

    /// `dup v` returns the pair `(v, v)`; `drop v` returns `()` (grades
    /// erased).
    #[test]
    fn grade_ops_are_erased_operationally()
    {
        let dup = Comp::dup(Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit)));
        assert_eq!(
            agree(dup),
            Eval::Value(Comp::ret(Value::pair(
                Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit)),
                Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit)),
            )))
        );
        let drop = Comp::drop(Value::thunk(Grade::ONE, Comp::ret(Value::Unit)));
        assert_eq!(agree(drop), Eval::Value(Comp::ret(Value::Unit)));
    }

    /// A computation hole blames; a hole reaching an elimination (`force ?`)
    /// blames.
    #[test]
    fn holes_blame()
    {
        assert_eq!(Eval::Blame(Blame::Hole), agree(Comp::hole(0)));
        assert_eq!(Eval::Blame(Blame::Hole), agree(Comp::force(Value::hole(1))));
    }

    /// On the pure spine the recursive reference and the CEK machine agree;
    /// assert that, and return the shared outcome.
    fn agree(comp: Comp) -> Eval
    {
        let recursive = eval_comp(comp.clone());
        let machine = run_comp(comp);
        assert_eq!(
            recursive, machine,
            "the recursive evaluator and the CEK machine must agree on the pure spine"
        );
        machine
    }

    /// Ill-typed eliminations that reach a terminal with the wrong shape are
    /// undefined stuck, and the recursive oracle and CEK machine must classify
    /// them identically. These are observable semantic outcomes, not just
    /// coverage of the error vocabulary: changing any branch to blame, a
    /// different stuck reason, or a value result breaks the differential.
    #[test]
    fn non_matching_eliminations_are_stuck_on_both_evaluators()
    {
        let cases = alloc::vec![
            (
                Comp::force(Value::int(0)),
                StuckReason::ForcedNonThunk,
                "forcing a concrete integer",
            ),
            (
                Comp::case(
                    Value::int(0),
                    "x",
                    Comp::ret(Value::var("x")),
                    "y",
                    Comp::ret(Value::var("y")),
                ),
                StuckReason::CasedNonSum,
                "case over a non-injection",
            ),
            (
                Comp::bind(
                    Comp::with(Comp::ret(Value::int(1)), Comp::ret(Value::int(2))),
                    "x",
                    Comp::ret(Value::var("x")),
                ),
                StuckReason::SequencedNonReturner,
                "sequencing a lazy product",
            ),
            (
                Comp::prj1(Comp::ret(Value::Unit)),
                StuckReason::ProjectedNonPair,
                "projecting a returner",
            ),
        ];

        for (term, expected, context) in cases {
            assert_eq!(
                eval_comp(term.clone()),
                Eval::Stuck(expected.clone()),
                "recursive evaluator: {context}"
            );
            assert_eq!(
                run_comp(term),
                Eval::Stuck(expected),
                "CEK machine: {context}"
            );
        }
    }

    /// An ill-typed redex is an undefined stuck (the oracle pins these out of
    /// the well-typed fragment): applying a returner is
    /// `AppliedNonFunction` — and the recursive reference and the CEK
    /// machine classify it identically (the by-frame stuck reasons are
    /// aligned, so the differential is exact).
    #[test]
    fn applying_a_returner_is_stuck()
    {
        let comp = Comp::app(Comp::ret(Value::int(0)), Value::int(1));
        assert_eq!(
            Eval::Stuck(StuckReason::AppliedNonFunction),
            eval_comp(comp.clone())
        );
        assert_eq!(Eval::Stuck(StuckReason::AppliedNonFunction), run_comp(comp));
    }

    /// The gradual-downcast gap (deferred; needs runtime casts). An
    /// `Unknown`-typed value annotated to a concrete former is a downcast that
    /// v0 erases, so the dynamics is unsound over the gradual fragment without
    /// a runtime cast: here a closed, well-typed `F Integer` computation
    /// gets stuck because the value is really an injection. This pins the
    /// v0 limitation ADR-34 D4 reserves to the gradual layer (the soundness
    /// oracle is over the rigid fragment; the cast / blame-on-boundary
    /// dynamics is the residual). `split ((inj1 0 : Unknown) : Integer ×
    /// Integer) as (x, y) in ret x` type-checks (`Unknown` is consistently
    /// a product) yet splits an injection.
    #[test]
    fn gradual_downcast_without_a_cast_is_stuck()
    {
        let downcast = Value::annot(
            Value::annot(Value::inj1(Value::int(0)), ValueType::Unknown),
            ValueType::prod(ValueType::integer(), ValueType::integer()),
        );
        let comp = Comp::split(downcast, "x", "y", Comp::ret(Value::var("x")));
        // The term is closed and well-typed against `F Integer`...
        let typed = crate::checker::check_comp(
            crate::ctx::Ctx::new(),
            comp.clone(),
            CompType::returner(ValueType::integer()),
        );
        assert!(
            typed.is_ok(),
            "the downcast term must type-check: {typed:?}"
        );
        // ...yet the erased downcast makes it stuck (no v0 runtime cast).
        assert_eq!(Eval::Stuck(StuckReason::SplitNonProduct), run_comp(comp));
    }

    /// `reset (shift k. resume k (ret 0))` round-trips to `ret 0`: the captured
    /// continuation is the identity up to the reset.
    #[test]
    fn reset_shift_resume_round_trip()
    {
        let body = Comp::resume(Value::var("k"), Comp::ret(Value::int(0)));
        let comp = Comp::reset(Comp::shift("k", body));
        assert_eq!(run_comp(comp), ret_int(0));
    }

    /// Resuming a continuation captured by `shift` reinstalls the captured
    /// frames with their bind-site environments. The continuation below `k`
    /// reads `x` from before capture (`1`), not from the dynamic environment at
    /// the resume site (`2`).
    #[test]
    fn shift_resumption_reinstalls_the_captured_frame_environment()
    {
        // reset (x = 1; (shift k. x = 2; resume k (ret ())) >>= _; ret x)
        let comp = Comp::reset(Comp::bind(
            Comp::ret(Value::int(1)),
            "x",
            Comp::bind(
                Comp::shift(
                    "k",
                    Comp::bind(
                        Comp::ret(Value::int(2)),
                        "x",
                        Comp::resume(Value::var("k"), Comp::ret(Value::Unit)),
                    ),
                ),
                "_",
                Comp::ret(Value::var("x")),
            ),
        ));
        assert_eq!(run_comp(comp), ret_int(1));
    }

    /// `reset (ret 0)` is transparent: the delimiter passes a returning value
    /// through.
    #[test]
    fn reset_passes_a_value_through()
    {
        assert_eq!(run_comp(Comp::reset(Comp::ret(Value::int(0)))), ret_int(0));
    }

    /// A `shift` with no enclosing `reset` is the defined `ShiftNoReset` blame
    /// (ADR-34 D5; the transitional over-accepted outcome), not an undefined
    /// stuck.
    #[test]
    fn shift_with_no_reset_blames()
    {
        let comp = Comp::shift("k", Comp::ret(Value::int(0)));
        assert_eq!(Eval::Blame(Blame::ShiftNoReset), run_comp(comp));
    }

    /// A `shift` whose enclosing `reset` is reachable only across a
    /// *suspension* (here a `thunk` that is forced after the `reset` has
    /// returned) is the over-accepted escaping-`shift` set the oracle
    /// flags: at runtime it reaches no live `KReset`, so it is a defined
    /// `ShiftNoReset` blame.
    #[test]
    fn escaping_shift_through_a_thunk_blames()
    {
        // `reset (force (thunk (shift k. ret 0)))` — the typing over-accepts (the
        // ambient answer leaks into the thunk body), but operationally the thunk
        // is forced under the reset here, so to exhibit the *escape* the thunk
        // must outlive the reset. `(force (thunk (shift k. ret 0)))` with the
        // shift's reset *removed* models the post-return state directly.
        let escaping = Comp::force(Value::thunk(
            Grade::ONE,
            Comp::shift("k", Comp::ret(Value::int(0))),
        ));
        assert_eq!(Eval::Blame(Blame::ShiftNoReset), run_comp(escaping));
    }

    /// `handle (perform get ()) { ret r ⇒ ret r | get p k ⇒ resume k (ret 7) |
    /// put … }` resumes the operation with `7`.
    #[test]
    fn perform_handle_resumes_the_operation()
    {
        let scrutinee = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
        );
        let ops = alloc::vec![
            OpClause::new(
                "get",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(7)))
            ),
            OpClause::new("put", "p", "k", Comp::ret(Value::Unit)),
        ];
        let comp = Comp::handle(state_sig(), scrutinee, "r", Comp::ret(Value::var("r")), ops);
        assert_eq!(run_comp(comp), ret_int(7));
    }

    /// A **non-resuming** clause's result is the handle's result, *not*
    /// re-handled by the return clause — the deep discipline's clause runs
    /// below the handler. `handle (perform put 5) { ret r ⇒ ret (inj1 r) |
    /// … | put p k ⇒ ret p }` yields `ret 5`, not `ret (inj1 5)`.
    #[test]
    fn non_resuming_clause_result_is_not_re_handled()
    {
        let ops = alloc::vec![
            OpClause::new("get", "p", "k", Comp::ret(Value::Unit)),
            OpClause::new("put", "p", "k", Comp::ret(Value::var("p"))),
        ];
        let comp = Comp::handle(
            state_sig(),
            Comp::perform(state_sig(), "put", Value::int(5)),
            "r",
            Comp::ret(Value::inj1(Value::var("r"))),
            ops,
        );
        assert_eq!(run_comp(comp), ret_int(5));
    }

    /// Deep handling: an operation performed *inside a resumption* is re-caught
    /// by the same handler. Two sequenced `get`s, each resumed with `3`, both
    /// route through the handler, yielding `ret (3, 3)`.
    #[test]
    fn deep_handler_re_handles_the_resumption()
    {
        let scrutinee = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::bind(
                Comp::perform(state_sig(), "get", Value::Unit),
                "y",
                Comp::ret(Value::pair(Value::var("x"), Value::var("y"))),
            ),
        );
        let ops = alloc::vec![
            OpClause::new(
                "get",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(3)))
            ),
            OpClause::new("put", "p", "k", Comp::ret(Value::Unit)),
        ];
        let comp = Comp::handle(state_sig(), scrutinee, "r", Comp::ret(Value::var("r")), ops);
        assert_eq!(
            run_comp(comp),
            Eval::Value(Comp::ret(Value::pair(Value::int(3), Value::int(3))))
        );
    }

    /// **Adversary regression** (the A5.1 precise-analyst pass). Nested
    /// handlers that reuse the resumption binder name `k` must NOT collide:
    /// each capture α-renames `k` to a fresh `env` key, so a closed
    /// well-typed `F Integer` program evaluates to `ret 42`. Keying `env`
    /// by the source name (dynamic scoping) instead made this exact term
    /// loop to a stuck `StepLimit` — a well-typed-term-gets-stuck soundness
    /// violation. The clause body `resume k (ret z)` refers to the *outer*
    /// `o`-clause `k`, while the inner handler's `i`-clause re-binds `k`;
    /// only α-renaming keeps them distinct.
    #[test]
    fn nested_handlers_reusing_the_resume_binder_do_not_collide()
    {
        const OUTER_HANDLER_REPLY: i64 = 42;
        let outer_sig = EffectSig::new(
            crate::boundary::EffectSignatureName::from("Eo"),
            alloc::vec![EffectOp::new(
                OperationName::from("o"),
                ValueType::Unit,
                ValueType::Unit
            )],
        );
        let inner_sig = EffectSig::new(
            crate::boundary::EffectSignatureName::from("Ei"),
            alloc::vec![EffectOp::new(
                OperationName::from("i"),
                ValueType::Unit,
                ValueType::Unit
            )],
        );
        // handle_Ei (perform i () >>= z. resume k (ret z)) { ret r ⇒ ret r
        //   | i p2 k ⇒ resume k (ret ()) }   -- the `resume k (ret z)` is the OUTER k
        let inner = Comp::handle(
            inner_sig.clone(),
            Comp::bind(
                Comp::perform(inner_sig, "i", Value::Unit),
                "z",
                Comp::resume(Value::var("k"), Comp::ret(Value::var("z"))),
            ),
            "r",
            Comp::ret(Value::var("r")),
            alloc::vec![OpClause::new(
                "i",
                "p2",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::Unit))
            )],
        );
        // handle_Eo (perform o () >>= w. ret 42) { ret r ⇒ ret r | o p k ⇒ inner }
        let outer = Comp::handle(
            outer_sig.clone(),
            Comp::bind(
                Comp::perform(outer_sig, "o", Value::Unit),
                "w",
                Comp::ret(Value::int(OUTER_HANDLER_REPLY)),
            ),
            "r",
            Comp::ret(Value::var("r")),
            alloc::vec![OpClause::new("o", "p", "k", inner)],
        );
        // Closed and well-typed against `F Integer`...
        let typed = crate::checker::check_comp(
            crate::ctx::Ctx::new(),
            outer.clone(),
            CompType::returner(ValueType::integer()),
        );
        assert!(
            typed.is_ok(),
            "the nested-handler term must type-check: {typed:?}"
        );
        // ...and the α-renamed continuations resolve correctly to `ret 42`.
        assert_eq!(run_comp(outer), ret_int(42));
    }

    /// `perform` with no enclosing handler is the defined `PerformNoHandler`
    /// blame, not an undefined stuck.
    #[test]
    fn perform_with_no_handler_blames()
    {
        let comp = Comp::perform(state_sig(), "get", Value::Unit);
        assert_eq!(Eval::Blame(Blame::PerformNoHandler), run_comp(comp));
    }

    /// A source-level `stk K` value resumes by splicing its structural stack
    /// under the stack's captured environment: arguments are evaluated there,
    /// bind bodies retain the same environment, and projection frames stay in
    /// order. The observable result distinguishes all three frame classes.
    #[test]
    fn resume_splices_a_source_stack()
    {
        let stack = Stack::arg(
            Value::var("argument"),
            Stack::bind(
                "result",
                Comp::with(
                    Comp::ret(Value::var("argument")),
                    Comp::ret(Value::var("result")),
                ),
                Stack::prj2(Stack::empty()),
            ),
        );
        let comp = Comp::bind(
            Comp::ret(Value::int(9)),
            "argument",
            Comp::resume(
                Value::stk(stack.clone()),
                Comp::lam("x", Comp::ret(Value::var("x"))),
            ),
        );
        assert_eq!(
            run_comp(comp),
            ret_int(9),
            "resume must apply the stack argument, bind the lambda result, then project the second lazy component"
        );

        let env = Env::empty().extend("argument".to_owned(), Rc::new(RtValue::Int(9)));
        let closed_stack = quote_value(&eval_value(&Value::stk(stack), &env));
        assert_eq!(
            closed_stack,
            Value::stk(Stack::arg(
                Value::int(9),
                Stack::bind(
                    "result",
                    Comp::with(Comp::ret(Value::int(9)), Comp::ret(Value::var("result"))),
                    Stack::prj2(Stack::empty()),
                ),
            )),
            "readback of a captured source stack closes free values but shields stack binders"
        );
    }

    /// A deeply left-nested `bind` chain evaluates on the heap-continuation CEK
    /// machine without host-stack overflow.
    #[test]
    fn deeply_nested_bind_chain_runs_on_the_machine()
    {
        let mut comp = Comp::ret(Value::Unit);
        for _ in 0 .. DEPTH {
            comp = Comp::bind(comp, "x", Comp::ret(Value::Unit));
        }
        assert_eq!(run_comp(comp), Eval::Value(Comp::ret(Value::Unit)));
    }

    /// Forcing a thunk whose body is a deep `bind` chain drives the whole chain
    /// on the CEK force-probe's heap continuation instead of overflowing the
    /// host stack (ADR-50 Decision C; the successor to the old iterative-subst
    /// regression, now that β env-extends rather than substituting).
    #[test]
    fn deep_thunk_body_forces_on_the_heap()
    {
        // The assertion runs *inside* the worker: `Eval` holds `Rc` (`!Send`), so
        // it cannot cross the thread boundary — return `()` and let a failure
        // panic the worker (surfaced by `join`).
        let worker = std::thread::Builder::new()
            .stack_size(PROBE_STACK)
            .spawn(|| {
                // `force (thunk (bind y ← (… (bind y ← ret () in ret ()) …) in
                // ret ()))`: forcing routes the whole `PROBE_DEPTH`-deep body
                // through the call-by-need probe, which drives it to `ret ()`.
                // Left-nested, so the probe's continuation dismantles level by
                // level and its environment stays shallow, isolating the depth
                // to the continuation stack.
                let mut body = Comp::ret(Value::Unit);
                for _ in 0 .. PROBE_DEPTH {
                    body = Comp::bind(body, "y", Comp::ret(Value::Unit));
                }
                let comp = Comp::force(Value::thunk(Grade::ONE, body));
                assert_eq!(
                    run_comp(comp),
                    Eval::Value(Comp::ret(Value::Unit)),
                    "a {PROBE_DEPTH}-deep thunk body must force on the heap probe"
                );
            })
            .expect("spawn the pinned-stack worker thread");
        worker
            .join()
            .expect("the deep force-probe must not overflow the host stack");
    }

    /// Nested unforced thunk probes trampoline through an explicit
    /// probe-machine stack while preserving call-by-need readback of the
    /// final whnf.
    #[test]
    fn nested_thunk_probes_are_trampolined()
    {
        const NESTED_PROBE_DEPTH: usize = 32;

        let worker = std::thread::Builder::new()
            .stack_size(PROBE_STACK)
            .spawn(|| {
                let mut comp = Comp::ret(Value::Unit);
                for _ in 0 .. NESTED_PROBE_DEPTH {
                    comp = Comp::force(Value::thunk(Grade::ONE, comp));
                }
                assert_eq!(
                    run_comp(comp),
                    Eval::Value(Comp::ret(Value::Unit)),
                    "a {NESTED_PROBE_DEPTH}-deep chain of nested thunk probes must reduce"
                );
            })
            .expect("spawn the pinned-stack worker thread");
        worker
            .join()
            .expect("nested thunk probing must not overflow the host stack");
    }

    /// Nesting depth for the CEK continuation-depth regression.
    ///
    /// A *left-nested* `bind` chain grows the heap-allocated continuation to
    /// this depth; on the CEK machine (ADR-50 Decision C) each `bind` pushes a
    /// [`Cont::Bind`] frame and each `ret ()` extends the environment and pops
    /// one — all iterative ([`drive`] / [`meet`] loop over the heap `cont`), no
    /// substitution. The recursive reference [`eval_comp`] would recurse once
    /// per level and overflow the host stack, so this drives [`run`] only.
    const DEPTH: usize = 20_000;

    /// A deep nest of `reset` delimiters around a returner is popped
    /// iteratively by `meet` (was a tail self-call — ADR-47).
    #[test]
    fn nested_reset_delimiters_pop_iteratively()
    {
        let mut comp = Comp::ret(Value::Unit);
        for _ in 0 .. RESET_DEPTH {
            comp = Comp::reset(comp);
        }
        assert_eq!(run_comp(comp), Eval::Value(Comp::ret(Value::Unit)));
    }

    /// Thunk-body depth for the CEK force-probe regression (ADR-50 Decision C).
    ///
    /// A depth-`PROBE_DEPTH` left-nested `bind` chain suspended in a thunk,
    /// then forced. Under the CEK, forcing runs the call-by-need *probe*
    /// ([`force_probe`]) — a nested machine that drives the body to weak-head
    /// normal form — which must descend the whole chain iteratively on the heap
    /// `cont`, not the host stack. The recursive reference [`eval_comp`] would
    /// recurse one host frame per level and abort well below this depth, so
    /// this drives [`run`] only; `PROBE_DEPTH` sits in the window
    /// (recursive-walk ceiling well below, recursive-`Drop` ceiling ~12k on
    /// 2 MiB — the derived `Drop` of the suspended body is still recursive,
    /// tranche T2), so the iterative probe plus the still-recursive
    /// `Drop` at teardown stays within the pinned 2 MiB worker stack.
    /// Making the probe (or `drive` / `meet`) recursive aborts this test —
    /// the guard property.
    const PROBE_DEPTH: usize = 4_000;

    /// The pinned worker-thread stack for the force-probe depth regression,
    /// matching the 2 MiB datapoint so the window is deterministic
    /// across test runners (`std` is linkable in tests; the crate keeps the
    /// `alloc` discipline but is not `#![no_std]`).
    const PROBE_STACK: usize = 2 * 1024 * 1024;

    /// The host-effect seam resumes a handler-less `perform`: `perform
    /// State.get () >>= x. ret x` with a host that resumes `get` with `7`
    /// yields `ret 7` (ADR-35 D4).
    #[test]
    fn host_seam_resumes_a_handler_less_perform()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
        );
        let mut host = |sig: &EffectSig, op: &str, _payload: &Value| -> HostReply {
            assert_eq!(
                "State",
                sig.name().as_ref(),
                "the host must see the State signature"
            );
            assert_eq!("get", op, "the host must see the get operation");
            HostReply::Resume(Value::int(7))
        };
        assert_eq!(run_with_host(comp, &mut host), ret_int(7));
    }

    /// Nesting depth for the `meet` `Cont::Reset` deloop regression (ADR-47).
    ///
    /// `meet` popped a transparent `Cont::Reset` by *tail-recursing*
    /// (`meet(comp, cont)`); a deep nest of `reset` delimiters would overflow
    /// the host stack. It is now a loop. Evaluating this many nested
    /// `reset`s around `ret ()` descends to a run of `Cont::Reset` frames
    /// on the heap continuation, then `meet` pops them iteratively; the
    /// recursive form overflowed well below this depth.
    const RESET_DEPTH: usize = 100_000;

    /// A host that declines every operation falls through to the ordinary step,
    /// so an unclaimed `perform` blames `PerformNoHandler` — the seam does not
    /// change the no-handler outcome when the host abstains.
    #[test]
    fn host_seam_unhandled_falls_through_to_blame()
    {
        let comp = Comp::perform(state_sig(), "get", Value::Unit);
        let mut host = |_: &EffectSig, _: &str, _: &Value| -> HostReply { HostReply::Unhandled };
        assert_eq!(
            Eval::Blame(Blame::PerformNoHandler),
            run_with_host(comp, &mut host)
        );
    }
    /// An in-term handler wins over the host: the handled term of
    /// [`perform_handle_resumes_the_operation`] run through the seam never
    /// consults the host (which panics if called), still yielding `ret 7` — the
    /// seam intercepts only operations no source-level handler claims.
    #[test]
    fn host_seam_in_term_handler_wins()
    {
        let scrutinee = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
        );
        let ops = alloc::vec![
            OpClause::new(
                "get",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(7)))
            ),
            OpClause::new("put", "p", "k", Comp::ret(Value::Unit)),
        ];
        let comp = Comp::handle(state_sig(), scrutinee, "r", Comp::ret(Value::var("r")), ops);
        let mut host = |_: &EffectSig, _: &str, _: &Value| -> HostReply {
            panic!("the host must not be consulted when an in-term handler catches the operation")
        };
        assert_eq!(run_with_host(comp, &mut host), ret_int(7));
    }

    /// The host is re-entered for each sequenced operation: two `get`s, each
    /// resumed with `3`, yield `ret (3, 3)` — the seam re-catches each
    /// handler-less `perform` after resumption (the deep discipline).
    #[test]
    fn host_seam_re_enters_for_sequenced_ops()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::bind(
                Comp::perform(state_sig(), "get", Value::Unit),
                "y",
                Comp::ret(Value::pair(Value::var("x"), Value::var("y"))),
            ),
        );
        let mut host =
            |_: &EffectSig, _: &str, _: &Value| -> HostReply { HostReply::Resume(Value::int(3)) };
        assert_eq!(
            run_with_host(comp, &mut host),
            Eval::Value(Comp::ret(Value::pair(Value::int(3), Value::int(3))))
        );
    }

    /// The host callback decodes an owned payload through the public [`Value`]
    /// API: `perform Fs.read "p"` hands the host a payload whose
    /// [`Value::as_str`] is `"p"`, and its `Resume("data")` yields `ret
    /// "data"`.
    #[test]
    fn host_seam_decodes_the_payload()
    {
        let comp = Comp::perform(fs_sig(), "read", Value::string("p"));
        let mut host = |sig: &EffectSig, op: &str, payload: &Value| -> HostReply {
            assert_eq!(
                "Fs",
                sig.name().as_ref(),
                "the host must see the Fs signature"
            );
            assert_eq!("read", op, "the host must see the read operation");
            assert_eq!(
                Some(crate::boundary::StringText::from("p")),
                payload.as_str(),
                "the payload decodes to the path string"
            );
            HostReply::Resume(Value::string("data"))
        };
        assert_eq!(
            run_with_host(comp, &mut host),
            Eval::Value(Comp::ret(Value::string("data")))
        );
    }

    /// An `Fs`-like signature `{ read : String ↠ String }` for the host-seam
    /// payload-decode test.
    fn fs_sig() -> EffectSig
    {
        EffectSig::new(
            crate::boundary::EffectSignatureName::from("Fs"),
            alloc::vec![EffectOp::new(
                OperationName::from("read"),
                ValueType::string(),
                ValueType::string()
            )],
        )
    }

    /// **v0 semantics pin (reversible).** A `perform` under a source-level
    /// `reset` is host-interceptable: `reset (perform Exec.exec ())` reaches no
    /// live handler (plain `run_comp` blames `PerformNoHandler`), so the host —
    /// the ambient handler outside every delimiter — intercepts and resumes it
    /// (INTERCEPT semantics).
    #[test]
    fn host_seam_intercepts_across_a_reset()
    {
        let exec_sig = EffectSig::new(
            crate::boundary::EffectSignatureName::from("Exec"),
            alloc::vec![EffectOp::new(
                OperationName::from("exec"),
                ValueType::Unit,
                ValueType::integer()
            )],
        );
        let comp = Comp::reset(Comp::perform(exec_sig, "exec", Value::Unit));
        // Without a host, the reset-blocked perform blames.
        assert_eq!(Eval::Blame(Blame::PerformNoHandler), run_comp(comp.clone()));
        // With a host, it is intercepted and resumed (the pinned v0 choice).
        let mut host =
            |_: &EffectSig, _: &str, _: &Value| -> HostReply { HostReply::Resume(Value::int(1)) };
        assert_eq!(run_with_host(comp, &mut host), ret_int(1));
    }

    /// A mid-run state is an inspectable semantic snapshot: after decomposing a
    /// projection of a reset-delimited handler, the public accessors expose the
    /// current focus, continuation depth, step count, and host-interceptable
    /// operation, while `reconstruct` losslessly rebuilds the source-shaped
    /// computation. A frame-order, reset, handler, or environment bug changes
    /// the reconstructed term or the pending host operation.
    #[test]
    fn state_reconstructs_host_interceptable_handler_context()
    {
        let sig = state_sig();
        let scrutinee = Comp::perform(sig.clone(), "get", Value::Unit);
        let comp = Comp::prj2(Comp::reset(Comp::handle(
            sig.clone(),
            scrutinee.clone(),
            "r",
            Comp::ret(Value::var("r")),
            alloc::vec![OpClause::new("put", "p", "k", Comp::ret(Value::var("p")),)],
        )));
        let mut state = State::new(comp.clone());
        for expected_step in 1_u32 ..= 3_u32 {
            state = match step(state) {
                | Outcome::Step(next) => next,
                | other => panic!("expected decomposition step {expected_step}, got {other:?}"),
            };
        }

        assert_eq!(
            state.focus(),
            &scrutinee,
            "the handler scrutinee is in focus"
        );
        assert_eq!(
            state.env(),
            &Env::empty(),
            "decomposition has not introduced value bindings"
        );
        assert_eq!(
            StackDepth::from(3),
            state.depth(),
            "depth mirrors the public continuation slice"
        );
        assert_eq!(
            MachineStepCount::from(3),
            state.steps(),
            "one step per decomposition frame"
        );
        assert_eq!(
            state.reconstruct(),
            comp,
            "the public state snapshot reconstructs the original computation"
        );

        let host_op = state
            .pending_host_op()
            .expect("the handler has no `get` clause, so the host can intercept");
        assert_eq!(host_op.sig, sig);
        assert_eq!("get", host_op.op);
        assert_eq!(Value::Unit, host_op.payload);
    }

    /// **v0 semantics pin (reversible).** A `perform` whose in-term handler is
    /// shadowed by an intervening handler for a *different* effect is
    /// host-interceptable. In `handle E (handle F (perform E.e ()))` the v0
    /// structural capture cannot reify across the non-matching `F` handler
    /// (`capture_to_handler` bails at it), so plain `run_comp` blames
    /// `PerformNoHandler` even though a source-level `E` handler encloses the
    /// `perform`; the host intercepts and resumes it (INTERCEPT semantics).
    /// This pins the v0 intercept-set so a future CEK change to
    /// `capture_to_handler` (e.g. one that saw *through* non-matching
    /// handlers) cannot silently flip it.
    #[test]
    fn host_seam_intercepts_across_an_intervening_handler()
    {
        let e_sig = EffectSig::new(
            crate::boundary::EffectSignatureName::from("E"),
            alloc::vec![EffectOp::new(
                OperationName::from("e"),
                ValueType::Unit,
                ValueType::integer()
            )],
        );
        let f_sig = EffectSig::new(
            crate::boundary::EffectSignatureName::from("F"),
            alloc::vec![EffectOp::new(
                OperationName::from("f"),
                ValueType::Unit,
                ValueType::integer()
            )],
        );
        // handle E { e p k ⇒ resume k (ret 42) | ret r ⇒ ret r }
        //   (handle F { f p k ⇒ resume k (ret ()) | ret r ⇒ ret r }
        //     (perform E.e ()))
        let inner = Comp::handle(
            f_sig,
            Comp::perform(e_sig.clone(), "e", Value::Unit),
            "r",
            Comp::ret(Value::var("r")),
            alloc::vec![OpClause::new(
                "f",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::Unit)),
            )],
        );
        let comp = Comp::handle(e_sig, inner, "r", Comp::ret(Value::var("r")), alloc::vec![
            OpClause::new(
                "e",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(42))),
            )
        ]);
        // The intervening `F` handler shadows the outer `E` handler: the perform
        // reaches no live handler and blames.
        assert_eq!(Eval::Blame(Blame::PerformNoHandler), run_comp(comp.clone()));
        // With a host handling `E.e`, it is intercepted and resumed — the host,
        // not the (unreachable) `E` clause, services it, so the result is `1`,
        // not the clause's `42`.
        let mut host = |sig: &EffectSig, op: &str, _: &Value| -> HostReply {
            assert_eq!(
                "E",
                sig.name().as_ref(),
                "the host must see the E signature"
            );
            assert_eq!("e", op, "the host must see the e operation");
            HostReply::Resume(Value::int(1))
        };
        assert_eq!(run_with_host(comp, &mut host), ret_int(1));
    }

    /// **The CEK regression oracle (ADR-35 D4).** For a `State` term, the host
    /// seam replying *by operation name* must agree with the *top-level deep
    /// handler* whose per-op clauses resume with the matching per-op replies —
    /// the seam IS that handler.
    ///
    /// The replies are DISTINCT per operation (`get ↦ 10`, `put ↦ 20`) and the
    /// terms end asymmetrically (`ret x` after a dropped reply, or a pair
    /// `(x, y)` of two different ops), so the observable result depends on
    /// WHICH reply lands in WHICH binder. A reply spliced into the wrong
    /// bind frame, or an op-routing swap, flips a result and fails the
    /// oracle — the exact reply-splicing the seam guarantees, which a
    /// single constant reply (the oracle's earlier form) could not detect.
    /// A CEK / arena rewrite that broke the equivalence fails here.
    #[test]
    fn host_seam_equals_the_top_level_deep_handler()
    {
        // Distinct replies per operation, so the result depends on op-routing.
        const GET_REPLY_VALUE: i64 = 10;
        const PUT_REPLY_VALUE: i64 = 20;
        let get_reply = Value::int(GET_REPLY_VALUE);
        let put_reply = Value::int(PUT_REPLY_VALUE);
        // Closed `State` computations mixing `get`/`put`; each ends
        // asymmetrically, so a mis-routed reply changes the observable result.
        // Each term is paired with its GOLDEN expected `Eval` (the `↦` value
        // spelled in the comment): the seam must not only agree with the
        // in-term deep handler (the differential, which a SYMMETRIC misroute —
        // a bind-frame bug hitting both paths equally — could survive) but also
        // land on the independently-written value. This pins the absolute
        // reply-routing, so a CEK / arena rewrite that broke reply-splicing in
        // both faces at once still fails here.
        let cases: Vec<(Comp, Eval)> = alloc::vec![
            // perform get () >>= x. ret x                           ↦ 10
            (
                Comp::bind(
                    Comp::perform(state_sig(), "get", Value::Unit),
                    "x",
                    Comp::ret(Value::var("x")),
                ),
                Eval::Value(Comp::ret(get_reply.clone())),
            ),
            // perform put 5 >>= u. perform get () >>= x. ret x      ↦ 10
            // (put's reply is dropped; x must bind get's reply, not put's)
            (
                Comp::bind(
                    Comp::perform(state_sig(), "put", Value::int(5)),
                    "u",
                    Comp::bind(
                        Comp::perform(state_sig(), "get", Value::Unit),
                        "x",
                        Comp::ret(Value::var("x")),
                    ),
                ),
                Eval::Value(Comp::ret(get_reply.clone())),
            ),
            // perform get () >>= x. perform put 5 >>= y. ret (x, y) ↦ (10, 20)
            (
                Comp::bind(
                    Comp::perform(state_sig(), "get", Value::Unit),
                    "x",
                    Comp::bind(
                        Comp::perform(state_sig(), "put", Value::int(5)),
                        "y",
                        Comp::ret(Value::pair(Value::var("x"), Value::var("y"))),
                    ),
                ),
                Eval::Value(Comp::ret(Value::pair(get_reply.clone(), put_reply.clone()))),
            ),
            // perform put 5 >>= x. perform get () >>= y. ret (x, y) ↦ (20, 10)
            // (reversed order: op-routing, not position, fixes each component)
            (
                Comp::bind(
                    Comp::perform(state_sig(), "put", Value::int(5)),
                    "x",
                    Comp::bind(
                        Comp::perform(state_sig(), "get", Value::Unit),
                        "y",
                        Comp::ret(Value::pair(Value::var("x"), Value::var("y"))),
                    ),
                ),
                Eval::Value(Comp::ret(Value::pair(put_reply.clone(), get_reply.clone()))),
            ),
        ];
        // The host replies BY OPERATION NAME; the in-term mirror resumes each
        // op-clause with the matching per-op reply. A routing swap on either
        // side desynchronizes them, failing the `assert_eq!`s below.
        let mut host = |_: &EffectSig, op: &str, _: &Value| -> HostReply {
            match op {
                | "get" => HostReply::Resume(get_reply.clone()),
                | "put" => HostReply::Resume(put_reply.clone()),
                | other => panic!("unexpected State operation {other}"),
            }
        };
        for (term, expected) in cases {
            let via_host = run_with_host(term.clone(), &mut host);
            // Golden-value leg: the seam lands on the independently-written
            // expected `Eval` (catches a symmetric misroute the differential
            // alone cannot).
            assert_eq!(
                via_host, expected,
                "the host seam must land on the golden expected value for the State term"
            );
            let ops = alloc::vec![
                OpClause::new(
                    "get",
                    "p",
                    "k",
                    Comp::resume(Value::var("k"), Comp::ret(get_reply.clone())),
                ),
                OpClause::new(
                    "put",
                    "p",
                    "k",
                    Comp::resume(Value::var("k"), Comp::ret(put_reply.clone())),
                ),
            ];
            let via_handler = run_comp(Comp::handle(
                state_sig(),
                term,
                "r",
                Comp::ret(Value::var("r")),
                ops,
            ));
            // Differential leg: the seam equals the top-level deep handler
            // (catches an ASYMMETRIC misroute affecting only one face).
            assert_eq!(
                via_host, via_handler,
                "the host seam must equal the top-level deep handler on the State term"
            );
        }
    }

    /// The `State` effect signature `{ get : 1 ↠ Integer, put : Integer ↠ 1 }`.
    fn state_sig() -> EffectSig
    {
        EffectSig::new(
            crate::boundary::EffectSignatureName::from("State"),
            alloc::vec![
                EffectOp::new(
                    OperationName::from("get"),
                    ValueType::Unit,
                    ValueType::integer()
                ),
                EffectOp::new(
                    OperationName::from("put"),
                    ValueType::integer(),
                    ValueType::Unit
                ),
            ],
        )
    }

    /// A returner terminal `ret n`.
    fn ret_int<L>(literal: L) -> Eval
    where
        L: Into<I64Literal>,
    {
        Eval::Value(Comp::ret(Value::int(i64::from(literal.into()))))
    }
}
