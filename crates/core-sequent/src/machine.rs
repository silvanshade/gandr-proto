//! The L machine — the iterative, arena-native operational dynamics of the
//! polarized command IL (the sequent-machines design's §4, decision K3, phase
//! **L1**).
//!
//! The machine is the sequent-shaped **re-presentation** of the machine that
//! was gandr's operational oracle through the L1 migration: differentially
//! anchored to it — `L-run ∘ 𝓕 ≡ run` — until it retired, and now the live
//! driver, anchored by the frozen outcome snapshots (the
//! [`crate::differential`] canonicalization drives both the corpus and the
//! hand-built regression sweeps).
//! It is a flat loop over commands (ADR-47 — no recursion in the step),
//! environment-extension only (never textual substitution), and every
//! transition is one budgeted step against the single shared
//! [`gandr_core_checker::outcome::STEP_BUDGET`].
//!
//! # Configuration (§4.1)
//!
//! The state is `{ command, env (x ↦ 𝕍 and α ↦ 𝕊), store, steps }`. Following
//! the CEK split, the environment is realized as two persistent maps — the
//! value environment [`LEnv`] (`x ↦` [`LValue`]) and the covalue environment
//! [`LContEnv`] (`α ↦` a captured [`Continuation`]) — and the continuation the
//! command runs against is the reified frame stack [`LFrame`], the L-machine
//! image of the CEK's `Cont` stack. A frame is a first-class, inspectable
//! reification of a consumer, so the sequent ethos ("consumers are first-class
//! data") holds: the whole configuration — command, frames, store — is IR data
//! ([`crate::inspect`]).
//!
//! # Why a reified frame stack, not purely env-threaded covalues
//!
//! Effect propagation (a `perform` searching the delimited context for its
//! handler, §6) and delimited control (`shift` searching for its prompt)
//! require *walking* the continuation. The focusing translation threads a
//! command's tail continuation through covariables, which are not walkable in
//! isolation; the CEK's flat `Cont` stack is. So the L machine keeps the same
//! flat, walkable stack — built by *loading* a consumer's frame spine
//! ([`LMachine::load_consumer`]) — and covariables capture *slices* of it
//! ([`LContEnv`], the image of the CEK's `ContEnv`). This is the faithful,
//! effect-correct choice; the frames are drawn from the consumer IL, so the
//! reification stays inspectable.
//!
//! # Build status (phase L1, differential-first)
//!
//! The pure CBPV spine — `ret` / `bind` (μ̃) / `force` (with the call-by-need
//! memo cells) / the positive eliminations (`case` / `split` / `listcase` /
//! `recordproj`) / the negative intros and eliminations (`cocase` for `λ` /
//! lazy pairs, `ap` / `prj`) / the grade structural ops (`dup` / `drop`) — is
//! live and differentially anchored, as is the **native registry** (`prim`): a
//! curried native accumulates its arguments from the `ap` frames and dispatches
//! to `gandr_core_checker::prim` on saturation, reading its (first-order)
//! arguments back to source values. The **higher-order combinators** (`each` /
//! `where` / `reduce` / `any` / `all` / `update_where`) force and apply a thunk
//! argument's body, which the focused IL no longer retains; they now dispatch
//! through the **un-focusing readback** `𝓕⁻¹` ([`crate::unfocus`]) —
//! un-focusing each closure argument to a source value, invoking the builtin,
//! and re-focusing the unrolled result against the ambient continuation
//! ([`LMachine::dispatch_native_higher_order`]), exactly as the CEK does.
//!
//! The **effect and control** surface is live: `perform` propagates its
//! operation up the reified frame stack to its handler
//! ([`LMachine::drive_perform`], the CEK's `capture_to_handler` over
//! [`LFrame`]s); `handle` / `reset` install their delimiter frame eagerly (the
//! [`ProducerNode::Mu`] delimiter entry); `resume` splices a captured
//! resumption ([`LMachine::meet_resume`]); the reified-stack value (`box` /
//! `stk`) and the deep re-entry discipline carry over. An unhandled `perform`
//! is a defined [`Blame::PerformNoHandler`], never a panic. **`shift`** is the
//! faithful delimited capture ([`LMachine::drive_shift`]): it loads the
//! local continuation, walks the reified stack
//! to the enclosing prompt ([`capture_to_reset`]), binds the captured
//! continuation (prompt included) to `k`, and runs the body below the
//! delimiter — an undelimited `shift` is a defined [`Blame::ShiftNoReset`],
//! never a panic. The prelude resolution (a free name in force position) is
//! realized ([`run_comp_with_prelude`], mirroring the CEK); the higher-order
//! combinators are realized through the un-focusing readback
//! ([`LMachine::dispatch_native_higher_order`]), agreeing with the CEK on pure,
//! effectful, and blaming closures alike.
//!
//! The **host-effect seam** (ADR-35 D4) is presented at the same point: an
//! operation no source-level handler claims — where [`LMachine::run`] blames
//! [`Blame::PerformNoHandler`] — is instead offered to an ambient host by
//! [`LMachine::run_with_host`] over the public [`Value`] / signature surface
//! ([`gandr_core_checker::effect::host`]), the *identical* seam the CEK
//! presents. On a resume the reply flows into the perform's continuation
//! (deep); a decline is the same `PerformNoHandler` blame. The seam is realized
//! at the driver — the L machine's configuration is the run loop's locals, not
//! a reified state — so the offered signature carries its name only and the
//! payload is read back over the public surface through the un-focusing
//! readback `𝓕⁻¹` ([`crate::unfocus`], §7a) — exact on the first-order fragment
//! AND on higher-order payloads (a thunk closure closes under its captured
//! environment).

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::ContinuationName;
use gandr_core_checker::boundary::EffectSignatureName;
use gandr_core_checker::boundary::FieldName;
use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::boundary::OperationName;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::effect::host::HostHandler;
use gandr_core_checker::effect::host::HostOp;
use gandr_core_checker::effect::host::HostReply;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::outcome::Blame;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::outcome::STEP_BUDGET;
use gandr_core_checker::outcome::StuckReason;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::DataId;

use crate::boundary::CommandHoleStatus;
use crate::boundary::CommandUnsupportedStatus;
use crate::boundary::FrameIndex;
use crate::boundary::OperationConstructorStatus;
use crate::boundary::ReturnerStatus;
use crate::boundary::UnfocusRequirement;
use crate::focus::FocusOrigin;
use crate::focus::FreshCoVarCounter;
use crate::il::Arm;
use crate::il::CoArm;
use crate::il::CoName;
use crate::il::CommandArena;
use crate::il::CommandId;
use crate::il::CommandNode;
use crate::il::ConsumerId;
use crate::il::ConsumerNode;
use crate::il::CtorTag;
use crate::il::DtorTag;
use crate::il::HandlerConsumer;
use crate::il::Lit;
use crate::il::Name;
use crate::il::PrimOp;
use crate::il::ProducerId;
use crate::il::ProducerNode;
use crate::store::Cell;
use crate::store::CellId;
use crate::store::MemoState;
use crate::store::Store;

/// A captured continuation — a slice of reified [`LFrame`]s a covariable binds
/// to (the L-machine image of the CEK's `Rc<[Cont]>` captured continuation).
pub type Continuation = Rc<[LFrame]>;

/// A machine value `𝕍` — the result of evaluating a substitutable producer
/// `𝔭` (the sequent-machines design's §4.1), the L-machine image of the
/// retired evaluator's runtime value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LValue
{
    /// A free / neutral name (an unbound producer variable — a prelude binding
    /// consulted at a `force` miss, or a readback residual).
    Var(Name),
    /// A positive scalar leaf.
    Lit(Lit),
    /// A positive constructor `K(𝕍̄)` — its producer arguments already
    /// evaluated (every frozen-core constructor carries no consumer argument).
    Ctor
    {
        /// The constructor tag.
        tag: CtorTag,
        /// The evaluated producer arguments.
        args: Vec<Rc<Self>>,
    },
    /// A codata object (a `λ` or a lazy pair) — a negative intro closed over
    /// its environment (`proposal-sequent-kernel.md` §4.1).
    Cocase
    {
        /// The copattern arms.
        arms: Rc<[CoArm]>,
        /// The captured value environment.
        env: LEnv,
        /// The captured covalue environment.
        coenv: LContEnv,
    },
    /// A U-shift thunk closure — a suspended body command with its environment
    /// and a call-by-need [`Cell`] (`proposal-sequent-kernel.md` §4.2; ADR-50
    /// call-by-need).
    Thunk
    {
        /// The usage grade, carried from the IL `Thunk` producer (the sealed
        /// grade surface, ADR-28). The memo policy is uniform across
        /// grades (§4.2), so the grade rides the value for diagnostics /
        /// readback but does not steer forcing.
        grade: Grade,
        /// The return-continuation binder the forced body computes against.
        cobinder: CoName,
        /// The suspended body command.
        body: CommandId,
        /// The captured value environment.
        env: LEnv,
        /// The captured covalue environment.
        coenv: LContEnv,
        /// The nominal call-by-need cell address (for inspection).
        cell_id: CellId,
        /// The shared call-by-need cell handle.
        cell: Cell,
    },
    /// A partially-applied native — a curried function value accumulating
    /// arguments until it saturates its [`NativePrim::arity`] and dispatches
    /// (`proposal-sequent-kernel.md` §4.1; the opaque host seam, §7.4).
    Native
    {
        /// The primitive being applied.
        prim: NativePrim,
        /// The arguments accumulated so far.
        args: Vec<Rc<Self>>,
    },
    /// A reified consumer as a value `box(c)` — the `stk K` intro (a covalue
    /// crossing into value position).
    Boxed(Continuation),
}

impl LValue
{
    /// The unit value (a convenience for tests / readback).
    #[inline]
    #[must_use]
    pub fn unit() -> Self
    {
        Self::Lit(Lit::Unit)
    }
}

/// A reified continuation frame `LFrame` — the L-machine image of the
/// retired evaluator's continuation stack, drawn from a consumer IL node (the
/// sequent-machines design's §4.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LFrame
{
    /// An argument frame `ap(𝕍; ·)` — apply the codata object in focus to the
    /// already-evaluated argument (ADR-50 Decision C — eager argument).
    Arg(Rc<LValue>),
    /// A projection frame `prjᵢ(·)` — project the lazy pair in focus.
    Prj(Side),
    /// A record-projection frame `.ℓ(·)` — select the field and return it.
    RecordProj(String),
    /// A stack-resumption frame `resume(𝕍; ·)` — feed the reified computation
    /// to the reified stack in focus.
    Resume(Rc<LValue>),
    /// The U-shift `force(·)` — force the thunk in focus (call-by-need).
    Force,
    /// A binder frame `μ̃x. s` — receive a value, bind `x`, run `s`.
    Bind
    {
        /// The binder receiving the value.
        binder: Name,
        /// The body command run after the bind.
        body: CommandId,
        /// The captured value environment the body runs under.
        env: LEnv,
        /// The captured covalue environment the body runs under.
        coenv: LContEnv,
    },
    /// A pattern-match frame `case { … }` — match the constructor in focus.
    Case
    {
        /// The match arms.
        arms: Rc<[Arm]>,
        /// The captured value environment.
        env: LEnv,
        /// The captured covalue environment.
        coenv: LContEnv,
    },
    /// A delimiter frame (a `reset` prompt) — transparent to a returning value
    /// (`proposal-sequent-kernel.md` §6).
    Reset,
    /// A deep handler frame `handler { … }` — the handler a `perform` searches
    /// for; runs the return clause on a returning value (§6).
    Handle
    {
        /// The handler consumer.
        handler: Rc<HandlerConsumer>,
        /// The captured value environment.
        env: LEnv,
        /// The captured covalue environment.
        coenv: LContEnv,
    },
}

/// A value environment `x ↦ 𝕍` (the sequent-machines design's §4.1) — a
/// persistent, innermost-first association, the L-machine image of the retired
/// evaluator's environment.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct LEnv(Option<Rc<EnvNode>>);

/// One cons cell of an [`LEnv`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct EnvNode
{
    /// The bound producer-variable name.
    name: Name,
    /// The machine value the name denotes.
    value: Rc<LValue>,
    /// The enclosing environment.
    rest: Option<Rc<Self>>,
}

impl LEnv
{
    /// The empty environment.
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self(None)
    }

    /// Extends with `name ↦ value` bound innermost (shadowing an outer
    /// binding).
    #[inline]
    #[must_use]
    pub fn extend(
        &self,
        name: Name,
        value: Rc<LValue>,
    ) -> Self
    {
        Self(Some(Rc::new(EnvNode {
            name,
            value,
            rest: self.0.clone(),
        })))
    }

    /// Looks `name` up, innermost-first.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        name: NameRef<'_>,
    ) -> Option<Rc<LValue>>
    {
        let mut cursor = self.0.as_deref();
        while let Some(node) = cursor {
            if node.name == name.as_ref() {
                return Some(Rc::clone(&node.value));
            }
            cursor = node.rest.as_deref();
        }
        None
    }

    /// Collects the bindings innermost-first — the order the un-focusing
    /// readback closes a thunk body under, mirroring the CEK's environment
    /// walk (a later binding of a name shadows an earlier one, so an
    /// innermost-first fold of the closing substitution reproduces it).
    ///
    /// # Contract
    /// - ensures: each `name ↦ value` pair in innermost-first order (most
    ///   recently bound first).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn bindings(&self) -> Vec<(Name, Rc<LValue>)>
    {
        let mut out = Vec::new();
        let mut cursor = self.0.as_deref();
        while let Some(node) = cursor {
            out.push((node.name.clone(), Rc::clone(&node.value)));
            cursor = node.rest.as_deref();
        }
        out
    }
}

impl Default for LEnv
{
    #[inline]
    fn default() -> Self
    {
        Self::empty()
    }
}

/// A covalue environment `α ↦ 𝕊` (the sequent-machines design's §4.1) — a
/// persistent, innermost-first association of covariables to captured
/// [`Continuation`]s.
///
/// The L-machine image of the retired evaluator's continuation environment.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct LContEnv(Option<Rc<CoEnvNode>>);

/// One cons cell of an [`LContEnv`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct CoEnvNode
{
    /// The bound covariable name.
    name: CoName,
    /// The captured continuation the covariable denotes.
    cont: Continuation,
    /// The enclosing covalue environment.
    rest: Option<Rc<Self>>,
}

impl LContEnv
{
    /// The empty covalue environment.
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self(None)
    }

    /// Extends with `name ↦ cont` bound innermost.
    #[inline]
    #[must_use]
    pub fn extend(
        &self,
        name: CoName,
        cont: Continuation,
    ) -> Self
    {
        Self(Some(Rc::new(CoEnvNode {
            name,
            cont,
            rest: self.0.clone(),
        })))
    }

    /// Looks `name` up, innermost-first.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        name: ContinuationName<'_>,
    ) -> Option<Continuation>
    {
        let mut cursor = self.0.as_deref();
        while let Some(node) = cursor {
            if node.name == name.as_ref() {
                return Some(Rc::clone(&node.cont));
            }
            cursor = node.rest.as_deref();
        }
        None
    }
}

impl Default for LContEnv
{
    #[inline]
    fn default() -> Self
    {
        Self::empty()
    }
}

/// The transition of one machine cycle: run a command next, or halt.
enum Flow
{
    /// Continue with a command under its environments (the continuation is the
    /// shared, mutated frame stack).
    Focus
    {
        /// The command to run next.
        command: CommandId,
        /// The value environment it runs under.
        env: LEnv,
        /// The covalue environment it runs under.
        coenv: LContEnv,
    },
    /// Continue by feeding a value to the current continuation frames.
    Meet
    {
        /// The value about to meet the current continuation.
        value: Rc<LValue>,
    },
    /// Offer a host-interceptable operation — a `perform` that no source-level
    /// handler claims across the continuation, exactly the point the CEK's
    /// `pending_host_op` intercepts. The host-run driver
    /// ([`LMachine::run_with_host`]) consults its handler here; the plain
    /// [`LMachine::run`] maps it straight to [`Blame::PerformNoHandler`], so
    /// the bare machine's behavior is unchanged.
    Host
    {
        /// The signature (effect) name the operation belongs to (the focused IL
        /// retains the name only; `𝓕` erases the operation list).
        sig: String,
        /// The operation's name.
        op: String,
        /// The already-evaluated payload, read back to the public [`Value`]
        /// surface only when the host is actually offered the operation.
        payload: Rc<LValue>,
    },
    /// Halt with this outcome.
    Final(Eval),
}

/// The L machine over an owned command arena (`proposal-sequent-kernel.md`
/// §4.1).
///
/// Owns the [`CommandArena`] so a saturated native's re-focused result term can
/// be lowered back into it (the native re-focusing path); owns the two-region
/// [`Store`] so call-by-need cells persist across a run (including nested force
/// probes, which share the heap).
pub struct LMachine
{
    /// The command arena (owned so native results can re-focus into it).
    arena: CommandArena,
    /// The focusing provenance, consulted to distinguish the two holes the
    /// focused IL conflates (a `Comp::Hole` cut — [`FocusOrigin::CompHole`] —
    /// blames; a returned `Value::Hole` is a value).
    origins: BTreeMap<CommandId, FocusOrigin>,
    /// The two-region store (heap cells + frame region).
    store: Store,
    /// The read-only prelude resolution table (ADR-42), consulted on a
    /// force-position free name (`⟨x | force(…)⟩` with `x` unbound): a name
    /// resolves to the fresh cobinder its thunk body computes against and the
    /// arena-resident body command, mirroring the CEK's `Force(Var …)` prelude
    /// lookup. Empty for [`Self::new`] (the no-prelude default the corpus and
    /// the plain entries drive), so a force miss stays `ForcedNonThunk`.
    prelude: BTreeMap<String, (CoName, CommandId)>,
    /// The fresh-covariable counter for re-focusing a higher-order native's
    /// result term into this arena ([`Self::refocus`]), threaded across
    /// dispatches so freshly minted covariables stay distinct.
    next_fresh: FreshCoVarCounter,
}

impl LMachine
{
    /// Builds a machine over `arena` with its focusing provenance.
    #[inline]
    #[must_use]
    pub fn new(
        arena: CommandArena,
        origins: BTreeMap<CommandId, FocusOrigin>,
    ) -> Self
    {
        Self {
            arena,
            origins,
            store: Store::new(),
            prelude: BTreeMap::new(),
            next_fresh: FreshCoVarCounter::default(),
        }
    }

    /// Builds a machine over `arena` with its focusing provenance and a prelude
    /// resolution table (ADR-42) — the [`Self::new`] companion the
    /// prelude'd entry ([`run_comp_with_prelude`]) drives. A force-position
    /// free name in `prelude` resolves to its thunk body (both arena-resident,
    /// so the table's command ids are valid on this machine); a name absent
    /// from the table stays a `ForcedNonThunk` force miss, exactly the empty
    /// prelude of [`Self::new`].
    #[inline]
    #[must_use]
    pub fn new_with_prelude(
        arena: CommandArena,
        origins: BTreeMap<CommandId, FocusOrigin>,
        prelude: BTreeMap<String, (CoName, CommandId)>,
    ) -> Self
    {
        Self {
            arena,
            origins,
            store: Store::new(),
            prelude,
            next_fresh: FreshCoVarCounter::default(),
        }
    }

    /// The store (for inspection).
    #[inline]
    #[must_use]
    pub fn store(&self) -> &Store
    {
        &self.store
    }

    /// Runs the machine from `root` to a terminal [`Eval`].
    ///
    /// # Contract
    /// - ensures: drives the flat command loop from `root` to a final [`Eval`]
    ///   — a terminal value, a defined blame, or an undefined stuck — under the
    ///   shared [`STEP_BUDGET`] net.
    /// - panics: none (the arena, environments, and frame stack live on the
    ///   heap; a runtime halt surfaces as an [`Eval`], never a panic).
    #[inline]
    #[must_use]
    pub fn run(
        &mut self,
        root: CommandId,
    ) -> Eval
    {
        let mut command = root;
        let mut env = LEnv::empty();
        let mut coenv = LContEnv::empty();
        let mut cont: Vec<LFrame> = Vec::new();
        let mut steps: u64 = 0;
        loop {
            if steps >= STEP_BUDGET {
                return Eval::Stuck(StuckReason::StepLimit);
            }
            steps = steps.saturating_add(1);
            let mut flow = self.drive(command, &env, &coenv, &mut cont);
            loop {
                match flow {
                    | Flow::Meet { value } => flow = self.meet(value, &mut cont),
                    | Flow::Focus {
                        command: next,
                        env: next_env,
                        coenv: next_coenv,
                    } => {
                        command = next;
                        env = next_env;
                        coenv = next_coenv;
                        break;
                    },
                    // With no host, an unclaimed `perform` takes its ordinary
                    // step: the defined `PerformNoHandler` blame (identical to
                    // the pre-seam behavior — the offer is simply declined).
                    | Flow::Host { .. } => return Eval::Blame(Blame::PerformNoHandler),
                    | Flow::Final(eval) => return eval,
                }
            }
        }
    }

    /// Runs the machine from `root` to a terminal [`Eval`], offering each
    /// host-interceptable operation to `handler` first — the host-effect seam
    /// (ADR-35 D4).
    ///
    /// This is [`Self::run`] wired to the host seam, mirroring the retired CEK
    /// oracle's host-seam driver: at the point an
    /// unclaimed `perform` would blame [`Blame::PerformNoHandler`]
    /// ([`Flow::Host`]), the operation is offered to `handler` over the public
    /// [`Value`] / signature surface. On [`HostReply::Resume`] the reply is
    /// delivered to the perform's continuation (deep discipline — the
    /// un-truncated `cont` means a later `perform` in it is offered again); on
    /// [`HostReply::Unhandled`] the machine takes its ordinary step, the same
    /// `PerformNoHandler` blame (returned directly rather than re-driving the
    /// perform — the CEK's documented perf residual). A perform a source-level
    /// handler claims never reaches the host.
    ///
    /// # Public-surface shape (driver-level seam)
    ///
    /// The seam is realized at the driver, not as stepwise public machine
    /// state: the L machine's configuration (`command` / `env` / `coenv` /
    /// `cont`) is the run loop's locals rather than a reified value, so a
    /// stepwise `pending_host_op` / `resume_host` pair over public state
    /// (the CEK's shape) would first have to reify that loop. Interception
    /// inside the loop keeps the seam faithful without that refactor; the
    /// runtime-effects port binds this driver entry.
    ///
    /// # Residues (ADR-35 D4 seam over the L representation)
    ///
    /// - **Signature.** The focused IL retains only the signature *name* (`𝓕`
    ///   erases the operation list), so the offered [`EffectSig`] carries the
    ///   name and an empty operation list. The observable seam contract is
    ///   `(name, operation, payload)`, which both machines present identically.
    /// - **Payload.** The payload is read back over the public [`Value`]
    ///   surface through the un-focusing readback `𝓕⁻¹`
    ///   ([`crate::unfocus::unfocus_value`], §7a): exact on the first-order
    ///   fragment AND on higher-order payloads (a thunk closure closes under
    ///   its captured environment, mirroring the CEK's `quote_value`), so the
    ///   host differential exercises higher-order payloads too.
    ///
    /// # Contract
    /// - ensures: the terminal [`Eval`] of driving `root`, with every
    ///   handler-less `perform` routed to `handler` — a resumed one continues,
    ///   a declined one blames as in [`Self::run`]; [`StuckReason::StepLimit`]
    ///   guards a pathological input exactly as [`Self::run`].
    /// - panics: none (barring a panic thrown by `handler` itself).
    #[inline]
    #[must_use]
    pub fn run_with_host<H>(
        &mut self,
        root: CommandId,
        handler: &mut H,
    ) -> Eval
    where
        H: HostHandler,
    {
        let mut command = root;
        let mut env = LEnv::empty();
        let mut coenv = LContEnv::empty();
        let mut cont: Vec<LFrame> = Vec::new();
        let mut steps: u64 = 0;
        loop {
            if steps >= STEP_BUDGET {
                return Eval::Stuck(StuckReason::StepLimit);
            }
            steps = steps.saturating_add(1);
            let mut flow = self.drive(command, &env, &coenv, &mut cont);
            loop {
                match flow {
                    | Flow::Meet { value } => flow = self.meet(value, &mut cont),
                    | Flow::Focus {
                        command: next,
                        env: next_env,
                        coenv: next_coenv,
                    } => {
                        command = next;
                        env = next_env;
                        coenv = next_coenv;
                        break;
                    },
                    | Flow::Host { sig, op, payload } => {
                        // Read the performed payload back to the public `Value`
                        // surface through the un-focusing readback `𝓕⁻¹`
                        // ([`crate::unfocus::unfocus_value`], mirroring the CEK's
                        // `quote_value`): exact on the first-order fragment AND on
                        // higher-order payloads (a thunk closure closes under its
                        // captured environment), so the host sees the identical
                        // value both machines present.
                        let payload =
                            crate::unfocus::unfocus_value(&payload, &self.arena, &self.origins)
                                .unwrap_or_else(|| Value::hole(0));
                        let host_op = HostOp::new(
                            EffectSig::new(EffectSignatureName::from(sig.as_str()), Vec::new()),
                            OperationName::from(op.as_str()),
                            payload,
                        );
                        match handler.handle(&host_op.sig, &host_op.op, &host_op.payload) {
                            | HostReply::Resume(reply) => {
                                // Deliver the reply to the perform's continuation
                                // (deep discipline: `cont` is un-truncated). The
                                // host reply is a closed value from the host.
                                match value_to_lvalue(&reply, &[]) {
                                    | Some(lvalue) => flow = self.meet(lvalue, &mut cont),
                                    | None => {
                                        return Eval::Stuck(StuckReason::UnsupportedByReference);
                                    },
                                }
                            },
                            // Decline: the ordinary step of an unclaimed `perform`
                            // is exactly `PerformNoHandler`. Return it directly
                            // rather than re-driving the perform (the CEK's
                            // documented perf residual).
                            | HostReply::Unhandled => {
                                return Eval::Blame(Blame::PerformNoHandler);
                            },
                        }
                    },
                    | Flow::Final(eval) => return eval,
                }
            }
        }
    }

    /// Drives one command `⟨p | c⟩` (or `prim` / `jump`): load its consumer's
    /// frame spine onto `cont`, evaluate its producer, and meet the two.
    fn drive(
        &mut self,
        command: CommandId,
        env: &LEnv,
        coenv: &LContEnv,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        // A computation hole `Comp::Hole` focuses to a `⟨Lit(Hole) | cont⟩` cut
        // indistinguishable from a returned value hole except by its provenance:
        // it is a computation with no value, so it blames regardless of the
        // continuation (matching the CEK's `Comp::Hole ⟶ Blame::Hole`).
        if bool::from(self.command_is_comp_hole(command)) {
            return Flow::Final(Eval::Blame(Blame::Hole));
        }
        // A not-yet-built former's decline cut ([`FocusOrigin::Unsupported`] —
        // today the ADR-76 identity formers, whole-program-declined by `𝓕`):
        // the defined sentinel, never a hole blame — a hole here would
        // *disagree* with the CEK oracle instead of declining (the corpus
        // differential's realized/declined split).
        if bool::from(self.command_is_unsupported(command)) {
            return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference));
        }
        // Clone the command node to release the arena borrow before the
        // subsequent `&mut self` machine steps (native re-focusing may extend
        // the arena). Command nodes are small (ids plus id lists).
        let Some(node) = self.arena.command(command).cloned()
        else {
            return Flow::Final(Eval::Stuck(StuckReason::InvalidClosureBody));
        };
        match node {
            | CommandNode::Cut {
                producer, consumer, ..
            } => {
                match self.arena.producer(producer).cloned() {
                    // A `shift` producer captures the delimited continuation
                    // (A-SHIFT, §6): load the local continuation `consumer`'s
                    // frames, capture the machine stack up to the enclosing prompt
                    // as `k`, and run the body below it — [`Self::drive_shift`].
                    | Some(ProducerNode::Shift { binder, body }) => {
                        return self.drive_shift(binder, body, consumer, env, coenv, cont);
                    },
                    // A `μ` producer marks a **delimiter entry** (`reset` /
                    // `handle`, A-RESET / A-HANDLE, §6): load the delimiter
                    // consumer's frames eagerly — installing the `Reset` /
                    // `Handle` frame the way the CEK pushes `Cont::Reset` /
                    // `Cont::Handle` *before* the body — so a dynamically-nested
                    // `perform` / `shift` finds the delimiter by walking the
                    // stack. The body was focused against `★`, so its `μ` binder
                    // is unused (bound to the empty continuation for hygiene).
                    | Some(ProducerNode::Mu(binder, body)) => {
                        self.load_consumer(consumer, env, coenv, cont);
                        let coenv = coenv.extend(binder, empty_cont());
                        return Flow::Focus {
                            command: body,
                            env: env.clone(),
                            coenv,
                        };
                    },
                    | _ => {},
                }
                // Load the consumer's frame spine, then evaluate the producer
                // (always a value producer, by the focusing invariant) and meet.
                self.load_consumer(consumer, env, coenv, cont);
                let value = self.eval_producer(producer, env, coenv);
                if bool::from(is_operation(&value)) {
                    // A `perform`: the operation propagates up the reified frame
                    // stack to its handler (§6); an unhandled operation is a
                    // defined `PerformNoHandler` blame.
                    Self::drive_perform(&value, cont)
                }
                else {
                    Flow::Meet { value }
                }
            },
            // `prim` and `jump` grow in later checkpoints (the native registry
            // and top-level jumps); the focusing translation emits no `jump`.
            | CommandNode::Prim { op, ps, cs } => self.drive_prim(op, &ps, &cs, env, coenv, cont),
            | CommandNode::Jump { .. } => {
                Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference))
            },
        }
    }

    /// Propagates a `perform` operation up the reified frame stack to its
    /// handler (§6; the CEK's `drive_perform` / `capture_to_handler`, over the
    /// reified [`LFrame`] stack rather than the `Cont` stack).
    ///
    /// The operation `Op[sig.op](payload)` searches the continuation for the
    /// nearest [`LFrame::Handle`] declaring `op` across a structural prefix. On
    /// a hit it runs that operation's clause *below* the (removed) handler,
    /// binding the payload to the clause's payload binder and the resumption
    /// `k` to the captured delimited continuation **with the handler
    /// reinstalled** ([`LValue::Boxed`], so `resume k …` re-enters the
    /// handler — deep, ADR-33 D4). A miss — no such handler, or the search
    /// crosses a [`LFrame::Reset`] delimiter or an intervening handler not
    /// declaring `op` (the v0 single-handler scope) — is the host-effect seam
    /// offer ([`Flow::Host`], ADR-35 D4): exactly the point the CEK's
    /// `pending_host_op` intercepts, which the bare [`LMachine::run`] resolves
    /// to a defined [`Blame::PerformNoHandler`] and [`LMachine::run_with_host`]
    /// hands to its handler.
    ///
    /// # Contract
    /// - requires: `op_value` is an operation constructor ([`is_operation`]).
    /// - ensures: the clause focus on a hit (with `cont` truncated to below the
    ///   handler), else a [`Flow::Host`] offer carrying the signature name, the
    ///   operation, and the un-truncated payload.
    /// - panics: none.
    fn drive_perform(
        op_value: &LValue,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        let LValue::Ctor {
            tag: CtorTag::Op { ref sig, ref op },
            ref args,
        } = *op_value
        else {
            // `is_operation` guaranteed an `Op` constructor.
            return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference));
        };
        let payload = args
            .first()
            .map_or_else(|| Rc::new(LValue::unit()), Rc::clone);
        let Some(index) = capture_to_handler(cont, op.into())
        else {
            // No source-level handler claims the operation across the
            // continuation: offer it to the host seam. `cont` is NOT truncated —
            // the host sits below the whole continuation (deep), so a resumed
            // reply flows into the perform's continuation and a later `perform`
            // there is offered again.
            return Flow::Host {
                sig: sig.clone(),
                op: op.clone(),
                payload,
            };
        };
        // The captured delimited continuation includes the handler frame (deep):
        // a later `resume k …` re-installs it. Extract the handler-site data,
        // then truncate to below the handler, where the clause body runs.
        let captured: Continuation = match cont.get(usize::from(index) ..) {
            | Some(slice) => Rc::from(slice.to_vec().into_boxed_slice()),
            | None => return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
        };
        let (handler, handler_env, handler_coenv) = match cont.get(usize::from(index)) {
            | Some(&LFrame::Handle {
                ref handler,
                ref env,
                ref coenv,
            }) => (Rc::clone(handler), env.clone(), coenv.clone()),
            // `capture_to_handler` returned the index of a `Handle` frame.
            | _ => return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
        };
        cont.truncate(usize::from(index));
        let Some(clause) = handler.ops.iter().find(|clause| clause.op == *op)
        else {
            // `capture_to_handler` already confirmed a clause for `op` exists.
            return Flow::Final(Eval::Blame(Blame::PerformNoHandler));
        };
        // Bind the payload, then the resumption innermost — so `k` wins if the
        // clause reuses one name for both (the CEK's `payload == resume` rule,
        // where the resumption shadows the skipped payload binding).
        let clause_env = handler_env
            .extend(clause.payload.clone(), payload)
            .extend(clause.resume.clone(), Rc::new(LValue::Boxed(captured)));
        Flow::Focus {
            command: clause.body,
            env: clause_env,
            coenv: handler_coenv,
        }
    }

    /// Drives a `shift k. s` capture (A-SHIFT, §6; the CEK's `drive_shift`,
    /// over the reified [`LFrame`] stack rather than the `Cont` stack — the
    /// faithful delimited capture).
    ///
    /// The local continuation `consumer` (the shift cut's consumer — the
    /// delimited-continuation frames from the shift site up to, but not
    /// including, the enclosing prompt) is loaded onto `cont` first, exactly as
    /// a cut loads its consumer and as the CEK pushes the enclosing `let` /
    /// `app` frames before the shift. The machine then walks `cont` to the
    /// nearest [`LFrame::Reset`] prompt across a structural prefix
    /// ([`capture_to_reset`]), binds `k` to the captured delimited
    /// continuation **including the prompt** ([`LValue::Boxed`], so a later
    /// `resume k …` re-establishes the delimiter), truncates below the
    /// (removed) prompt, and runs the body — which was focused against `★`,
    /// so its returning tail falls through to the truncated stack (the
    /// continuation below the prompt). A capture that reaches no prompt, or
    /// would cross a [`LFrame::Handle`], is a defined
    /// [`Blame::ShiftNoReset`] (ADR-34 D5) — never a panic.
    ///
    /// # Contract
    /// - ensures: on a reachable prompt, binds `k` to the captured continuation
    ///   (prompt included) and focuses the body over the continuation below the
    ///   prompt; otherwise the `ShiftNoReset` blame; `cont` is truncated to
    ///   below the prompt on a hit.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: `L-run(𝓕(shift k. s))` denotes the CEK's `drive_shift`
    ///   outcome — capture up to the nearest prompt, `k` a boxed continuation,
    ///   the body below the delimiter.
    /// - witness: the `differential` gate's `shift` property (`reset`-delimited
    ///   captures, resuming and discarding clauses, and the undelimited
    ///   `ShiftNoReset` agreement) over 4096 generated cases plus the
    ///   hand-built `hand_built_shift_cases_agree`, against the external CEK
    ///   oracle.
    fn drive_shift(
        &mut self,
        binder: Name,
        body: CommandId,
        consumer: ConsumerId,
        env: &LEnv,
        coenv: &LContEnv,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        // Load the local delimited-continuation prefix onto the stack, so the
        // capture spans the same frames the CEK's does (its enclosing `let` /
        // `app` frames are pushed before the shift runs).
        self.load_consumer(consumer, env, coenv, cont);
        let Some(index) = capture_to_reset(cont)
        else {
            return Flow::Final(Eval::Blame(Blame::ShiftNoReset));
        };
        // The captured delimited continuation includes the prompt frame (standard
        // `shift`: a later `resume k …` re-establishes the delimiter around the
        // resumption). Truncate to below the (removed) prompt, where the body
        // runs.
        let captured: Continuation = match cont.get(usize::from(index) ..) {
            | Some(slice) => Rc::from(slice.to_vec().into_boxed_slice()),
            | None => return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
        };
        cont.truncate(usize::from(index));
        // Bind `k` to the boxed captured continuation in the value environment
        // (the innermost binding shadows any outer `k`, so distinct captures never
        // collide — the L-machine analogue of the CEK's α-rename to a fresh key).
        let env = env.extend(binder, Rc::new(LValue::Boxed(captured)));
        Flow::Focus {
            command: body,
            env,
            coenv: coenv.clone(),
        }
    }

    /// Whether a command is a focused computation hole (a `Comp::Hole` cut,
    /// [`FocusOrigin::CompHole`]) — a computation that blames rather than a
    /// value leaf.
    #[inline]
    #[must_use]
    fn command_is_comp_hole(
        &self,
        command: CommandId,
    ) -> CommandHoleStatus
    {
        matches!(self.origins.get(&command), Some(&FocusOrigin::CompHole)).into()
    }

    /// Whether `command` is a not-yet-built former's decline cut
    /// ([`FocusOrigin::Unsupported`]) — run to the defined
    /// [`StuckReason::UnsupportedByReference`] sentinel, never to a hole
    /// blame (see the [`Self::drive`] decline check).
    fn command_is_unsupported(
        &self,
        command: CommandId,
    ) -> CommandUnsupportedStatus
    {
        matches!(self.origins.get(&command), Some(&FocusOrigin::Unsupported)).into()
    }

    /// Evaluates a substitutable producer `𝔭` to a machine value under the
    /// environments (`𝓥`; environment-extension only, ADR-50 Decision C).
    fn eval_producer(
        &mut self,
        producer: ProducerId,
        env: &LEnv,
        coenv: &LContEnv,
    ) -> Rc<LValue>
    {
        enum Task
        {
            Node(ProducerId),
            BuildCtor
            {
                tag: CtorTag,
                len: usize,
            },
        }

        let mut work = alloc::vec![Task::Node(producer)];
        let mut values: Vec<Rc<LValue>> = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                | Task::Node(id) => {
                    let Some(node) = self.arena.producer(id).cloned()
                    else {
                        values.push(Rc::new(LValue::Lit(Lit::Hole(0))));
                        continue;
                    };
                    match node {
                        | ProducerNode::Var(name) => {
                            values.push(
                                env.lookup((&name).into())
                                    .unwrap_or_else(|| Rc::new(LValue::Var(name))),
                            );
                        },
                        | ProducerNode::Lit(lit) => values.push(Rc::new(LValue::Lit(lit))),
                        | ProducerNode::Ctor { tag, ps, .. } => {
                            work.push(Task::BuildCtor { tag, len: ps.len() });
                            for &arg in ps.iter().rev() {
                                work.push(Task::Node(arg));
                            }
                        },
                        | ProducerNode::Cocase { arms } => values.push(Rc::new(LValue::Cocase {
                            arms: Rc::from(arms),
                            env: env.clone(),
                            coenv: coenv.clone(),
                        })),
                        | ProducerNode::Thunk {
                            grade,
                            cobinder,
                            body,
                        } => {
                            let (cell_id, cell) = self.store.alloc_cell();
                            values.push(Rc::new(LValue::Thunk {
                                grade,
                                cobinder,
                                body,
                                env: env.clone(),
                                coenv: coenv.clone(),
                                cell_id,
                                cell,
                            }));
                        },
                        | ProducerNode::Boxed(consumer) => {
                            let mut frames: Vec<LFrame> = Vec::new();
                            self.load_consumer(consumer, env, coenv, &mut frames);
                            values
                                .push(Rc::new(LValue::Boxed(Rc::from(frames.into_boxed_slice()))));
                        },
                        | ProducerNode::Mu(..) | ProducerNode::Shift { .. } => {
                            values.push(Rc::new(LValue::Lit(Lit::Hole(0))));
                        },
                    }
                },
                | Task::BuildCtor { tag, len } => {
                    debug_assert!(
                        len <= values.len(),
                        "constructor frame arity exceeds the value stack"
                    );
                    let start = values.len().saturating_sub(len);
                    let args = values.split_off(start);
                    values.push(Rc::new(LValue::Ctor { tag, args }));
                },
            }
        }
        let result = values.pop();
        debug_assert!(
            result.is_some(),
            "producer evaluation yields one machine value"
        );
        result.unwrap_or_else(|| Rc::new(LValue::Lit(Lit::Hole(0))))
    }

    /// Loads a consumer's frame spine onto `cont` (innermost frame on top),
    /// splicing a covariable's captured continuation at the leaf. Iterative
    /// (ADR-47): the destructor / prompt spine is walked without recursion.
    fn load_consumer(
        &mut self,
        consumer: ConsumerId,
        env: &LEnv,
        coenv: &LContEnv,
        cont: &mut Vec<LFrame>,
    )
    {
        // Descend the destructor / prompt spine collecting outer-first frames;
        // then handle the leaf; then push the collected frames in reverse so the
        // innermost (nearest the value) ends on top.
        let mut collected: Vec<LFrame> = Vec::new();
        let mut current = consumer;
        // Clone each consumer node to release the arena borrow before the
        // `&mut self` argument evaluation in `dtor_frame`; a dangling id (the
        // `while let` exit) leaves the frames collected so far, and the meet
        // halts on the resulting shape.
        while let Some(node) = self.arena.consumer(current).cloned() {
            match node {
                | ConsumerNode::CoVar(name) => {
                    if let Some(captured) = coenv.lookup((&name).into()) {
                        cont.extend_from_slice(captured.as_ref());
                    }
                    break;
                },
                | ConsumerNode::Top => break,
                | ConsumerNode::MuTilde(binder, body) => {
                    cont.push(LFrame::Bind {
                        binder,
                        body,
                        env: env.clone(),
                        coenv: coenv.clone(),
                    });
                    break;
                },
                | ConsumerNode::Case { arms } => {
                    cont.push(LFrame::Case {
                        arms: Rc::from(arms),
                        env: env.clone(),
                        coenv: coenv.clone(),
                    });
                    break;
                },
                | ConsumerNode::Handler(handler) => {
                    let continuation = handler.continuation;
                    collected.push(LFrame::Handle {
                        handler: Rc::new(*handler),
                        env: env.clone(),
                        coenv: coenv.clone(),
                    });
                    current = continuation;
                },
                | ConsumerNode::Dtor { tag, ps, cs } => {
                    let frame = self.dtor_frame(&tag, &ps, env, coenv);
                    collected.push(frame);
                    match cs.first() {
                        | Some(&next) => current = next,
                        | None => break,
                    }
                },
                | ConsumerNode::Prompt(inner) => {
                    collected.push(LFrame::Reset);
                    current = inner;
                },
            }
        }
        for frame in collected.into_iter().rev() {
            cont.push(frame);
        }
    }

    /// Builds the reified frame for a destructor tag, evaluating its (eager)
    /// producer arguments under the environments.
    fn dtor_frame(
        &mut self,
        tag: &DtorTag,
        ps: &[ProducerId],
        env: &LEnv,
        coenv: &LContEnv,
    ) -> LFrame
    {
        match *tag {
            | DtorTag::Ap => {
                let arg = ps.first().map_or_else(
                    || Rc::new(LValue::unit()),
                    |&p| self.eval_producer(p, env, coenv),
                );
                LFrame::Arg(arg)
            },
            | DtorTag::Force => LFrame::Force,
            | DtorTag::Prj(side) => LFrame::Prj(side),
            | DtorTag::RecordProj(ref label) => LFrame::RecordProj(label.clone()),
            | DtorTag::Resume => {
                let arg = ps.first().map_or_else(
                    || Rc::new(LValue::unit()),
                    |&p| self.eval_producer(p, env, coenv),
                );
                LFrame::Resume(arg)
            },
        }
    }

    /// Meets a machine value with the top of the continuation, consuming frames
    /// until the value is bound, matched, or reaches the terminal.
    fn meet(
        &mut self,
        mut value: Rc<LValue>,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        // A `while let` (not recursion) so a `Reset` delimiter — transparent to a
        // returning value — is consumed iteratively (ADR-47); the empty
        // continuation (the loop exit) reads the terminal back.
        while let Some(frame) = cont.pop() {
            match frame {
                | LFrame::Bind {
                    binder,
                    body,
                    env,
                    coenv,
                } => {
                    // A `μ̃` binds the value of a RETURNER (`ret v`): a
                    // non-returner terminal (a `λ` / lazy pair, or a partial
                    // native) meeting it is an ill-typed sequencing, exactly the
                    // CEK's `SequencedNonReturner` (a `Cont::Bind` accepts only
                    // `Comp::Ret`).
                    if !bool::from(is_returner_value(&value)) {
                        return Flow::Final(Eval::Stuck(StuckReason::SequencedNonReturner));
                    }
                    let env = env.extend(binder, value);
                    return Flow::Focus {
                        command: body,
                        env,
                        coenv,
                    };
                },
                | LFrame::Case { arms, env, coenv } => {
                    return Self::meet_case(&value, &arms, &env, &coenv);
                },
                | LFrame::Arg(arg) => return self.meet_arg(&value, &arg),
                | LFrame::Prj(side) => return Self::meet_prj(&value, side),
                | LFrame::RecordProj(label) => {
                    // Record projection scrutinizes a VALUE (the record), so a
                    // hole record value blames (the CEK's `RtValue::Hole` arm).
                    if matches!(*value, LValue::Lit(Lit::Hole(_))) {
                        return Flow::Final(Eval::Blame(Blame::Hole));
                    }
                    match Self::record_field(&value, (&label).into()) {
                        | Ok(field) => {
                            value = field;
                        },
                        | Err(reason) => return Flow::Final(Eval::Stuck(reason)),
                    }
                },
                | LFrame::Force => return self.meet_force(&value, cont),
                | LFrame::Reset => {
                    // Transparent to a returning value: pop and re-meet.
                },
                | LFrame::Handle {
                    handler,
                    env,
                    coenv,
                } => {
                    // A returning value runs the handler's return clause under
                    // the handler-site environment extended with the bound value;
                    // the handler is discharged (§6). A non-returner terminal is
                    // the CEK's `SequencedNonReturner` (the return clause binds a
                    // returner, like a `μ̃`).
                    if !bool::from(is_returner_value(&value)) {
                        return Flow::Final(Eval::Stuck(StuckReason::SequencedNonReturner));
                    }
                    let ret_env = env.extend(handler.ret_binder.clone(), value);
                    return Flow::Focus {
                        command: handler.ret_body,
                        env: ret_env,
                        coenv,
                    };
                },
                | LFrame::Resume(reified) => {
                    return Self::meet_resume(&value, &reified, cont);
                },
            }
        }
        // The empty continuation: read the terminal value back.
        Flow::Final(self.read_terminal(&value))
    }

    /// Feeds a reified computation to a reified stack (`resume`, §6; the CEK's
    /// `Comp::Resume`).
    ///
    /// The stack `𝕊` is a boxed continuation ([`LValue::Boxed`]) — a captured
    /// resumption (a handler clause's `k`, bound by [`Self::drive_perform`]) or
    /// a source `stk K`; both inhabit the same value. The captured frames
    /// are spliced on top of the current continuation (their head ends on
    /// top — the continuation reading) and the reified computation runs
    /// against the splice, so a `perform` inside it re-enters the
    /// reinstalled handler (deep). A hole blames; any other value is the
    /// CEK's `ResumedNonStack`.
    ///
    /// # Contract
    /// - ensures: on a boxed stack, splices the captured frames and focuses the
    ///   reified body against them (its return binder falling through); a hole
    ///   is [`Blame::Hole`], any other value [`StuckReason::ResumedNonStack`].
    /// - panics: none.
    fn meet_resume(
        stack: &LValue,
        reified: &LValue,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        let LValue::Boxed(ref captured) = *stack
        else {
            return match *stack {
                | LValue::Lit(Lit::Hole(_)) => Flow::Final(Eval::Blame(Blame::Hole)),
                | _ => Flow::Final(Eval::Stuck(StuckReason::ResumedNonStack)),
            };
        };
        // Splice the captured continuation on top of the current one, then run
        // the reified computation against it — the splice stays LIVE on the
        // stack (not drained into the body's binder), so a control operation in
        // the fed computation walks the reinstalled frames.
        cont.extend_from_slice(captured.as_ref());
        let LValue::Thunk {
            ref cobinder,
            body,
            ref env,
            ref coenv,
            ..
        } = *reified
        else {
            // The reified computation is always a `Thunk` (A-RESUME wraps it).
            return Flow::Final(Eval::Stuck(StuckReason::ResumedNonStack));
        };
        let coenv = coenv.extend(String::from(cobinder.as_str()), empty_cont());
        Flow::Focus {
            command: body,
            env: env.clone(),
            coenv,
        }
    }

    /// Meets a constructor with a `case` frame — the positive elimination.
    fn meet_case(
        value: &LValue,
        arms: &[Arm],
        env: &LEnv,
        coenv: &LContEnv,
    ) -> Flow
    {
        let LValue::Ctor { ref tag, ref args } = *value
        else {
            if let LValue::Lit(Lit::Hole(_)) = *value {
                return Flow::Final(Eval::Blame(Blame::Hole));
            }
            return Flow::Final(Eval::Stuck(non_ctor_stuck(arms)));
        };
        for arm in arms {
            if arm.ctor == *tag {
                let mut arm_env = env.clone();
                for (binder, arg) in arm.binders.iter().zip(args.iter()) {
                    arm_env = arm_env.extend(binder.clone(), Rc::clone(arg));
                }
                return Flow::Focus {
                    command: arm.body,
                    env: arm_env,
                    coenv: coenv.clone(),
                };
            }
        }
        Flow::Final(Eval::Stuck(non_ctor_stuck(arms)))
    }

    /// Meets a value with an argument frame — codata application (`ap`) or
    /// native-argument accumulation.
    fn meet_arg(
        &mut self,
        value: &LValue,
        arg: &Rc<LValue>,
    ) -> Flow
    {
        match *value {
            | LValue::Cocase {
                ref arms,
                ref env,
                ref coenv,
            } => Self::apply_cocase(arms, env, coenv, &DtorTag::Ap, Some(arg)),
            // A native accumulates the argument; once it saturates its arity it
            // dispatches, otherwise it continues as a partial application meeting
            // the rest of the continuation (the CEK's currying-native discipline,
            // dispatched only on meeting an argument frame). A **higher-order**
            // combinator (one taking a thunk closure) un-focuses its arguments
            // to source values and dispatches through the un-focusing readback
            // ([`Self::dispatch_native_higher_order`]); every other native reads
            // its first-order arguments back structurally
            // ([`Self::dispatch_native`]).
            | LValue::Native { prim, ref args } => {
                let mut next = args.clone();
                next.push(Rc::clone(arg));
                if next.len() >= usize::from(prim.arity()) {
                    if bool::from(native_needs_unfocus(prim)) {
                        self.dispatch_native_higher_order(prim, &next)
                    }
                    else {
                        Self::dispatch_native(prim, &next)
                    }
                }
                else {
                    Flow::Meet {
                        value: Rc::new(LValue::Native { prim, args: next }),
                    }
                }
            },
            // A returned hole value (`ret ?u`) reaching an argument frame is an
            // ill-typed application of a returner — the CEK's
            // `AppliedNonFunction`, not a value elimination / hole blame.
            | _ => Flow::Final(Eval::Stuck(StuckReason::AppliedNonFunction)),
        }
    }

    /// Dispatches a saturated **higher-order** native combinator (`each` /
    /// `where` / `reduce` / `any` / `all` / `update_where`) through the
    /// un-focusing readback `𝓕⁻¹` — exactly the CEK's `Comp::Native`
    /// saturation.
    ///
    /// The accumulated arguments (including the thunk-closure arguments the
    /// combinator forces and applies) are un-focused to source [`Value`]s
    /// ([`crate::unfocus::unfocus_value`], mirroring the CEK's `quote_value`),
    /// the builtin is invoked over the shared `gandr_core_checker::prim`
    /// registry, and its result term — an **unrolled** closed CBPV computation
    /// over the manifest list — is re-focused into the live arena and run
    /// against the ambient continuation ([`crate::focus::focus_comp_into`]).
    /// This mirrors the CEK's `Transition::Focus(prim.apply(&args),
    /// Env::empty())`: the result runs under the empty environment against the
    /// current continuation, so an effectful / blaming closure body reaches the
    /// same enclosing handler / blame the CEK's does.
    ///
    /// # Contract
    /// - ensures: the [`Flow`] of running the native's result against the
    ///   ambient continuation — a returned value meets the continuation, a
    ///   gradual-hole result blames [`Blame::Hole`], an argument the readback
    ///   cannot reconstruct is a defined
    ///   [`StuckReason::UnsupportedByReference`].
    /// - panics: none.
    fn dispatch_native_higher_order(
        &mut self,
        prim: NativePrim,
        args: &[Rc<LValue>],
    ) -> Flow
    {
        let mut values: Vec<Rc<Value>> = Vec::with_capacity(args.len());
        for arg in args {
            match crate::unfocus::unfocus_value(arg, &self.arena, &self.origins) {
                | Some(value) => values.push(Rc::new(value)),
                | None => return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
            }
        }
        // The builtin steps to a closed result computation (an unrolled term for
        // a well-shaped list, or a gradual hole for a bad shape). Re-focus it and
        // run it against the ambient continuation — a `Comp::Hole` re-focuses to
        // the defined `Blame::Hole` cut, exactly as the CEK drives it.
        let result = prim.apply(&values);
        match self.refocus(&result) {
            | Some((root, cobinder)) => Flow::Focus {
                command: root,
                env: LEnv::empty(),
                coenv: LContEnv::empty().extend(cobinder, empty_cont()),
            },
            | None => Flow::Final(Eval::Stuck(StuckReason::InvalidClosureBody)),
        }
    }

    /// Re-focuses a source computation into the live arena against a fresh
    /// fall-through covariable (the native re-focusing path;
    /// [`crate::focus::focus_comp_into`]).
    fn refocus(
        &mut self,
        comp: &Comp,
    ) -> Option<(CommandId, CoName)>
    {
        crate::focus::focus_comp_into(
            &mut self.arena,
            &mut self.origins,
            &mut self.next_fresh,
            comp,
        )
        .ok()
    }

    /// Dispatches a saturated native to the Rust registry: read its first-order
    /// arguments back to source values, run the builtin (`prim.apply`), and
    /// feed the result to the continuation. The host seam is unchanged —
    /// the native registry is the CEK's own (`gandr_core_checker::prim`).
    ///
    /// This is the **first-order** dispatch path: a scalar / structural prim
    /// (`id` / `const`, arithmetic, list and record rearrangement) that never
    /// inspects a thunk body. A **higher-order** combinator (one forcing a
    /// thunk argument — `each` / `where` / `reduce` / `any` / `all` /
    /// `update_where`) is routed to [`Self::dispatch_native_higher_order`]
    /// by the caller; it reaches this path only from the pure-whnf force
    /// probe ([`Self::force_probe`]), where it declines with a defined
    /// [`StuckReason::UnsupportedByReference`] so the probe falls back to the
    /// real (higher-order-dispatching) inline run.
    fn dispatch_native(
        prim: NativePrim,
        args: &[Rc<LValue>],
    ) -> Flow
    {
        // A higher-order combinator only reaches this first-order path from the
        // force probe; decline so the probe runs the body inline (where the
        // caller routes it to the un-focusing dispatch). A scalar / first-order
        // prim never inspects a thunk body: it either
        // rejects the shape (a gradual hole, identically on both machines) or
        // passes the thunk through structurally (`id` / `const` / record and
        // list rearrangement). So argument thunks cross the host seam as
        // indexed readback markers and any thunk in the result is resolved
        // back to the ORIGINAL machine thunk — identity, the real body, and
        // the shared call-by-need cell survive the round trip (SEQUENT-001;
        // a first-order prim can never construct a fresh thunk).
        if bool::from(native_needs_unfocus(prim)) {
            return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference));
        }
        let mut marks: Vec<Rc<LValue>> = Vec::new();
        let mut values: Vec<Rc<Value>> = Vec::with_capacity(args.len());
        for arg in args {
            match read_value(arg, &mut marks) {
                | Some(value) => values.push(Rc::new(value)),
                | None => return Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
            }
        }
        // The builtin steps to a result computation; a first-order-argument prim
        // returns `ret v` (a returner) or `?u` (a bad-shape gradual hole).
        match prim.apply(&values) {
            | Comp::Ret(ref value) => match value_to_lvalue(value, &marks) {
                | Some(machine_value) => Flow::Meet {
                    value: machine_value,
                },
                | None => Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
            },
            | Comp::Hole(_) => Flow::Final(Eval::Blame(Blame::Hole)),
            // A non-returner native result (a higher-order combinator's unrolled
            // term) needs the un-focusing residual — decline.
            | _ => Flow::Final(Eval::Stuck(StuckReason::UnsupportedByReference)),
        }
    }

    /// Meets a lazy pair with a projection frame — the negative product
    /// elimination.
    fn meet_prj(
        value: &LValue,
        side: Side,
    ) -> Flow
    {
        match *value {
            | LValue::Cocase {
                ref arms,
                ref env,
                ref coenv,
            } => Self::apply_cocase(arms, env, coenv, &DtorTag::Prj(side), None),
            // A returned hole value reaching a projection frame is an ill-typed
            // projection of a returner, not a value elimination — the CEK's
            // `ProjectedNonPair`, not a hole blame.
            | _ => Flow::Final(Eval::Stuck(StuckReason::ProjectedNonPair)),
        }
    }

    /// Applies a codata object against a destructor: match the copattern arm,
    /// bind its binder (if any) to the frame argument, and focus the arm body
    /// against the continuation kept LIVE on the stack.
    ///
    /// The arm's cobinder is bound to the *empty* continuation, so the arm
    /// body's returning tail falls through to the frames already on the
    /// stack. This mirrors the CEK, which re-focuses a function / lazy-pair
    /// body against the current continuation (it does not drain it):
    /// keeping the frames live is what lets an effect / control operation
    /// inside the arm body walk an enclosing delimiter. Draining the
    /// continuation into the cobinder (the former L0 behaviour) buries that
    /// delimiter under the arm body; for a pure arm body the two are
    /// observationally identical.
    fn apply_cocase(
        arms: &[CoArm],
        env: &LEnv,
        coenv: &LContEnv,
        dtor: &DtorTag,
        arg: Option<&Rc<LValue>>,
    ) -> Flow
    {
        for arm in arms {
            if arm.dtor == *dtor {
                let mut arm_env = env.clone();
                if let (Some(binder), Some(value)) = (arm.binders.first(), arg) {
                    arm_env = arm_env.extend(binder.clone(), Rc::clone(value));
                }
                let mut arm_coenv = coenv.clone();
                if let Some(cobinder) = arm.cobinders.first() {
                    arm_coenv = arm_coenv.extend(cobinder.clone(), empty_cont());
                }
                return Flow::Focus {
                    command: arm.body,
                    env: arm_env,
                    coenv: arm_coenv,
                };
            }
        }
        Flow::Final(Eval::Stuck(match *dtor {
            | DtorTag::Prj(_) => StuckReason::ProjectedNonPair,
            | _ => StuckReason::AppliedNonFunction,
        }))
    }

    /// Selects a record field, or classifies the ill-typed shape.
    fn record_field(
        value: &LValue,
        label: FieldName<'_>,
    ) -> Result<Rc<LValue>, StuckReason>
    {
        match *value {
            | LValue::Ctor {
                tag: CtorTag::Record(ref labels),
                ref args,
            } => labels
                .iter()
                .position(|candidate| candidate == label.as_ref())
                .and_then(|index| args.get(index).cloned())
                .ok_or(StuckReason::RecordProjMissingField),
            | _ => Err(StuckReason::RecordProjNonRecord),
        }
    }

    /// Meets a value with a `force` frame — call-by-need thunk forcing, or a
    /// prelude resolution at a free name (the prelude path grows in a later
    /// checkpoint).
    fn meet_force(
        &mut self,
        value: &LValue,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        match *value {
            | LValue::Thunk {
                ref cobinder,
                body,
                ref env,
                ref coenv,
                ref cell,
                ..
            } => self.force(cobinder.into(), body, env, coenv, cell, cont),
            | LValue::Lit(Lit::Hole(_)) => Flow::Final(Eval::Blame(Blame::Hole)),
            // A free name in force position resolves against the prelude
            // (ADR-42), mirroring the CEK's `Force(Var …)`: a hit is a thunk
            // body focused under the EMPTY environment (the builtin is closed),
            // run inline against the live continuation (each occurrence
            // re-focuses the body — no cross-occurrence memo, exactly the CEK,
            // which re-runs the body on every force of the name); a miss (name
            // absent, or its winning binding not a thunk) stays `ForcedNonThunk`.
            | LValue::Var(ref name) => match self.prelude.get(name.as_str()) {
                | Some(&(ref cobinder, body)) => {
                    Self::force_inline(cobinder.into(), body, &LEnv::empty(), &LContEnv::empty())
                },
                | None => Flow::Final(Eval::Stuck(StuckReason::ForcedNonThunk)),
            },
            | _ => Flow::Final(Eval::Stuck(StuckReason::ForcedNonThunk)),
        }
    }

    /// Forces a memoized thunk (ADR-50 call-by-need; carried over from the
    /// CEK's `force_thunk` / `force_probe` EXACTLY): reuse the cached
    /// weak-head form, or probe the body on a nested machine at the empty
    /// continuation — cache a pure, continuation-free terminal (the
    /// pure-spine-only write-back), else clear the black hole and run the
    /// body inline against the real continuation.
    fn force(
        &mut self,
        cobinder: ContinuationName<'_>,
        body: CommandId,
        env: &LEnv,
        coenv: &LContEnv,
        cell: &Cell,
        _cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        match cell.state() {
            | MemoState::Forced(cached) => return Flow::Meet { value: cached },
            | MemoState::InProgress => return Self::force_inline(cobinder, body, env, coenv),
            | MemoState::Unforced => {},
        }
        cell.mark_in_progress();
        match self.force_probe(body, env, coenv) {
            | Some(whnf) => {
                cell.set_forced(Rc::clone(&whnf));
                Flow::Meet { value: whnf }
            },
            | None => {
                // Not purely reducible: clear the black hole and run inline.
                cell.clear();
                Self::force_inline(cobinder, body, env, coenv)
            },
        }
    }

    /// Runs a thunk body inline against the real continuation: focus the body
    /// with the continuation kept LIVE on the stack (the fast → slow fallback).
    ///
    /// The cobinder is bound to the *empty* continuation so the body's
    /// returning tail falls through to the frames already on the stack —
    /// mirroring the CEK's inline force, which re-focuses the body against
    /// the current continuation rather than draining it. Keeping the frames
    /// live is what lets an effect / control operation in an effectful
    /// thunk body walk an enclosing delimiter (which draining into the
    /// cobinder would bury).
    fn force_inline(
        cobinder: ContinuationName<'_>,
        body: CommandId,
        env: &LEnv,
        coenv: &LContEnv,
    ) -> Flow
    {
        let coenv = coenv.extend(String::from(cobinder.as_ref()), empty_cont());
        Flow::Focus {
            command: body,
            env: env.clone(),
            coenv,
        }
    }

    /// Probes a thunk body to its weak-head normal form on a nested machine at
    /// the empty continuation (ADR-50; the CEK's `force_probe`): returns the
    /// cached machine value when the body reduces PURELY to a terminal with no
    /// captured continuation, and `None` otherwise (it blames / gets stuck /
    /// captures a continuation / exhausts the budget). The nested probe shares
    /// the heap (nested thunks cache in their own cells) but takes a fresh
    /// frame stack and covalue environment.
    fn force_probe(
        &mut self,
        body: CommandId,
        env: &LEnv,
        coenv: &LContEnv,
    ) -> Option<Rc<LValue>>
    {
        // The cobinder binds to `★` (the empty continuation): the probe caches
        // only a continuation-independent terminal, so the top of the body is
        // the terminal `★` — no covalue binding is needed, and if the body tries
        // to escape through a captured continuation the probe declines (`None`).
        let mut command = body;
        let mut env = env.clone();
        let mut coenv = coenv.clone();
        let mut cont: Vec<LFrame> = Vec::new();
        let mut steps: u64 = 0;
        loop {
            if steps >= STEP_BUDGET {
                return None;
            }
            steps = steps.saturating_add(1);
            // A computation hole blames — not a pure whnf, so the probe declines
            // and the caller runs the body inline (which blames). `shift` and a
            // delimiter entry (`reset` / `handle`, a `μ` cut) likewise need the
            // real continuation, so they decline the probe and run inline.
            if bool::from(self.command_is_comp_hole(command))
                || matches!(self.origins.get(&command), Some(&FocusOrigin::Shift))
            {
                return None;
            }
            // A dangling command id declines the probe (the `?` returns `None`).
            let node = self.arena.command(command).cloned()?;
            // Only pure spine reduces in a probe: a `perform` / `prim` / `jump`
            // is not continuation-independent, so decline and run inline.
            let CommandNode::Cut {
                producer, consumer, ..
            } = node
            else {
                return None;
            };
            if matches!(self.arena.producer(producer), Some(&ProducerNode::Mu(..))) {
                return None;
            }
            self.load_consumer(consumer, &env, &coenv, &mut cont);
            let value = self.eval_producer(producer, &env, &coenv);
            if bool::from(is_operation(&value)) {
                return None;
            }
            let mut probe_flow = Flow::Meet { value };
            loop {
                match probe_flow {
                    | Flow::Meet { mut value } => loop {
                        let Some(frame) = cont.pop()
                        else {
                            return Some(value);
                        };
                        match frame {
                            | LFrame::Bind {
                                binder,
                                body,
                                env,
                                coenv,
                            } => {
                                if !bool::from(is_returner_value(&value)) {
                                    return None;
                                }
                                let env = env.extend(binder, value);
                                probe_flow = Flow::Focus {
                                    command: body,
                                    env,
                                    coenv,
                                };
                                break;
                            },
                            | LFrame::Case { arms, env, coenv } => {
                                probe_flow = Self::meet_case(&value, &arms, &env, &coenv);
                                break;
                            },
                            | LFrame::Arg(arg) => {
                                probe_flow = match *value {
                                    | LValue::Cocase {
                                        ref arms,
                                        ref env,
                                        ref coenv,
                                    } => Self::apply_cocase(
                                        arms,
                                        env,
                                        coenv,
                                        &DtorTag::Ap,
                                        Some(&arg),
                                    ),
                                    | LValue::Native { prim, ref args } => {
                                        let mut next = args.clone();
                                        next.push(Rc::clone(&arg));
                                        if next.len() >= usize::from(prim.arity()) {
                                            Self::dispatch_native(prim, &next)
                                        }
                                        else {
                                            Flow::Meet {
                                                value: Rc::new(LValue::Native { prim, args: next }),
                                            }
                                        }
                                    },
                                    | _ => {
                                        Flow::Final(Eval::Stuck(StuckReason::AppliedNonFunction))
                                    },
                                };
                                break;
                            },
                            | LFrame::Prj(side) => {
                                probe_flow = Self::meet_prj(&value, side);
                                break;
                            },
                            | LFrame::RecordProj(label) => {
                                match Self::record_field(&value, (&label).into()) {
                                    | Ok(field) => value = field,
                                    | Err(_) => return None,
                                }
                            },
                            | LFrame::Force | LFrame::Resume(_) => return None,
                            | LFrame::Reset => {},
                            | LFrame::Handle {
                                handler,
                                env,
                                coenv,
                            } => {
                                if !bool::from(is_returner_value(&value)) {
                                    return None;
                                }
                                let ret_env = env.extend(handler.ret_binder.clone(), value);
                                probe_flow = Flow::Focus {
                                    command: handler.ret_body,
                                    env: ret_env,
                                    coenv,
                                };
                                break;
                            },
                        }
                    },
                    | Flow::Focus {
                        command: next,
                        env: next_env,
                        coenv: next_coenv,
                    } => {
                        command = next;
                        env = next_env;
                        coenv = next_coenv;
                        break;
                    },
                    // A host-interceptable `perform` is not pure spine (it is
                    // declined above at `is_operation`), so it never reaches this
                    // probe; decline defensively, exactly as the probe declines
                    // any non-whnf outcome.
                    | Flow::Host { .. } | Flow::Final(_) => return None,
                }
            }
        }
    }

    /// Reads a terminal machine value back to an [`Eval`] (the readback /
    /// quote, ADR-50 Decision D) through the un-focusing readback `𝓕⁻¹`
    /// ([`crate::unfocus::unfocus_terminal`]): a positive value is a returner
    /// (`ret v`); a codata object reads back to its exact `λ` / lazy-pair
    /// intro (closed under its captured environment); a native to a partial
    /// application with its exact accumulated arguments. This is the exact
    /// readback the differential compares structurally against the CEK's
    /// `read_terminal`.
    ///
    /// A declined readback (a shape `𝓕⁻¹` cannot reconstruct — e.g. a reified
    /// stack's divergent representation) falls back to the prior kind-granular
    /// placeholder so totality never regresses.
    fn read_terminal(
        &self,
        value: &Rc<LValue>,
    ) -> Eval
    {
        if let Some(comp) = crate::unfocus::unfocus_terminal(value, &self.arena, &self.origins) {
            return Eval::Value(comp);
        }
        // Defensive fallback: the prior kind-granular placeholder readback,
        // reached only if `𝓕⁻¹` declines (never for a well-formed terminal).
        match **value {
            | LValue::Cocase { ref arms, .. } => {
                if matches!(arms.first().map(|arm| &arm.dtor), Some(&DtorTag::Prj(_))) {
                    Eval::Value(Comp::with(Comp::hole(0), Comp::hole(0)))
                }
                else {
                    Eval::Value(Comp::lam("_", Comp::hole(0)))
                }
            },
            | LValue::Native { prim, ref args } => {
                let placeholder = core::iter::repeat_with(|| Rc::new(Value::Unit))
                    .take(args.len())
                    .collect();
                Eval::Value(Comp::Native {
                    prim,
                    args: placeholder,
                })
            },
            | _ => match read_value(value, &mut Vec::new()) {
                | Some(v) => Eval::Value(Comp::ret(v)),
                | None => Eval::Value(Comp::ret(Value::hole(0))),
            },
        }
    }

    /// Dispatches a native `prim` command (the grade structural ops reduce in
    /// place; the native registry grows in the natives checkpoint).
    fn drive_prim(
        &mut self,
        op: PrimOp,
        ps: &[ProducerId],
        cs: &[ConsumerId],
        env: &LEnv,
        coenv: &LContEnv,
        cont: &mut Vec<LFrame>,
    ) -> Flow
    {
        // The return continuation is the single trailing consumer.
        if let Some(&ret) = cs.first() {
            self.load_consumer(ret, env, coenv, cont);
        }
        match op {
            | PrimOp::Dup => {
                // `dup v ⟶ ret (v, v)` (ADR-34 D2): the grade structural op is
                // erased operationally.
                let value = ps.first().map_or_else(
                    || Rc::new(LValue::unit()),
                    |&p| self.eval_producer(p, env, coenv),
                );
                let pair = Rc::new(LValue::Ctor {
                    tag: CtorTag::Pair,
                    args: Vec::from([Rc::clone(&value), value]),
                });
                self.meet(pair, cont)
            },
            | PrimOp::Drop => {
                // `drop v ⟶ ret ()` (ADR-34 D2).
                self.meet(Rc::new(LValue::unit()), cont)
            },
            // A native `prim` command builds the curried native value and meets
            // the continuation, which accumulates its arguments from the `ap`
            // frames the focusing threads (the lowered `Comp::native` carries no
            // baked-in argument, so `ps` is empty in practice).
            | PrimOp::Native(prim) => {
                let mut args = Vec::with_capacity(ps.len());
                for &p in ps {
                    args.push(self.eval_producer(p, env, coenv));
                }
                self.meet(Rc::new(LValue::Native { prim, args }), cont)
            },
        }
    }
}

/// Focuses a checked-core computation and runs it on a fresh L machine to a
/// terminal.
///
/// The empty-continuation, empty-prelude convenience the differential harness
/// drives (`proposal-sequent-kernel.md` §9, phase L1).
///
/// # Contract
/// - ensures: `L-run(𝓕(comp))` as an [`Eval`]; a focusing failure (arena
///   exhaustion, unreachable for a realistic term) surfaces as a defined stuck
///   rather than a panic.
/// - panics: none.
#[inline]
#[must_use]
pub fn run_comp(comp: &Comp) -> Eval
{
    match crate::focus::focus_comp(comp) {
        | Ok(focused) => {
            let (arena, root, origins) = focused.into_parts();
            LMachine::new(arena, origins).run(root)
        },
        | Err(_) => Eval::Stuck(StuckReason::InvalidClosureBody),
    }
}

/// Focuses a checked-core computation and runs it on a fresh L machine to a
/// terminal, offering each host-interceptable operation to `handler` first —
/// the host-effect seam (ADR-35 D4).
///
/// The [`run_comp`] companion wired to the host seam
/// ([`LMachine::run_with_host`]), the empty-continuation entry the host
/// differential drives; it mirrors the retired CEK oracle's host-seam driver.
///
/// # Contract
/// - ensures: `L-run(𝓕(comp))` as an [`Eval`], with every handler-less
///   `perform` routed to `handler` — a resumed one continues (deep discipline),
///   a declined one blames [`Blame::PerformNoHandler`]; a focusing failure
///   surfaces as a defined stuck rather than a panic.
/// - panics: none (barring a panic thrown by `handler`).
#[inline]
#[must_use]
pub fn run_comp_with_host<H>(
    comp: &Comp,
    handler: &mut H,
) -> Eval
where
    H: HostHandler,
{
    match crate::focus::focus_comp(comp) {
        | Ok(focused) => {
            let (arena, root, origins) = focused.into_parts();
            LMachine::new(arena, origins).run_with_host(root, handler)
        },
        | Err(_) => Eval::Stuck(StuckReason::InvalidClosureBody),
    }
}

/// Focuses a checked-core computation under a prelude binding-environment
/// (ADR-42) and runs it on a fresh L machine to a terminal.
///
/// The `run_comp` companion mirroring the retired CEK oracle's prelude entry.
///
/// `bindings` is the prelude as name → value pairs (later bindings shadow
/// earlier — the CEK's reverse lookup); a force-position free name resolves
/// against the thunk-valued bindings, a miss stays `ForcedNonThunk`. The empty
/// prelude reproduces [`run_comp`] exactly.
///
/// # Contract
/// - ensures: `L-run(𝓕(comp))` under `bindings` as an [`Eval`]; a focusing
///   failure surfaces as a defined stuck rather than a panic.
/// - panics: none.
#[inline]
#[must_use]
pub fn run_comp_with_prelude(
    comp: &Comp,
    bindings: &[(String, Value)],
) -> Eval
{
    match crate::focus::focus_comp_with_prelude(comp, bindings) {
        | Ok(prepared) => {
            let (focused, prelude) = prepared.into_parts();
            let (arena, root, origins) = focused.into_parts();
            LMachine::new_with_prelude(arena, origins, prelude).run(root)
        },
        | Err(_) => Eval::Stuck(StuckReason::InvalidClosureBody),
    }
}

/// Focuses a checked-core computation under a prelude and runs it while
/// offering host-interceptable operations to `handler`.
///
/// This composes [`run_comp_with_prelude`] and [`run_comp_with_host`] without
/// dropping either capability; source-level host programs need both operator
/// resolution and the ADR-35 D4 host seam.
///
/// # Contract
/// - ensures: `L-run(𝓕(comp))` under `bindings`, with every handler-less
///   `perform` offered to `handler`;
/// - ensures: a focusing failure surfaces as
///   [`StuckReason::InvalidClosureBody`];
/// - panics: none (barring a panic thrown by `handler`).
#[inline]
#[must_use]
pub fn run_comp_with_prelude_and_host<H>(
    comp: &Comp,
    bindings: &[(String, Value)],
    handler: &mut H,
) -> Eval
where
    H: HostHandler,
{
    match crate::focus::focus_comp_with_prelude(comp, bindings) {
        | Ok(prepared) => {
            let (focused, prelude) = prepared.into_parts();
            let (arena, root, origins) = focused.into_parts();
            LMachine::new_with_prelude(arena, origins, prelude).run_with_host(root, handler)
        },
        | Err(_) => Eval::Stuck(StuckReason::InvalidClosureBody),
    }
}

/// Focuses a lowered top-level term (a value or a computation) and runs it on a
/// fresh L machine to a terminal — the entry the corpus differential drives.
///
/// A [`Comp`] term runs as `L-run(𝓕(t))`; a value term `v` runs as
/// `L-run(⟨𝓥⟦v⟧ | ★⟩)`, whose terminal reads back as `ret v` (the value at the
/// top continuation), so the outcome shape matches how the session evaluates a
/// bare value item.
///
/// # Contract
/// - ensures: the terminal [`Eval`] of the focused term.
/// - panics: none.
#[inline]
#[must_use]
pub fn run_term(term: &Term) -> Eval
{
    match crate::focus::focus_term(term) {
        | Ok(focused) => {
            let (arena, root, origins) = focused.into_parts();
            LMachine::new(arena, origins).run(root)
        },
        | Err(_) => Eval::Stuck(StuckReason::InvalidClosureBody),
    }
}

/// Whether a value is a **returner** value — the value of a `ret v`, so it may
/// be bound by a `μ̃` or a handler return clause. A codata object (`λ` / lazy
/// pair) and a partial native are non-returner terminals, which meeting a
/// binder is the CEK's `SequencedNonReturner`.
#[inline]
#[must_use]
fn is_returner_value(value: &LValue) -> ReturnerStatus
{
    (!matches!(*value, LValue::Cocase { .. } | LValue::Native { .. })).into()
}

/// The empty captured continuation — a zero-length reified frame slice.
///
/// The covalue a delimiter-entry `μ` binder and a fall-through return binder
/// bind to: both are transparent, and the machine reads the ambient frame stack
/// directly, so returning through such a covariable adds no frames.
#[inline]
#[must_use]
fn empty_cont() -> Continuation
{
    Rc::from(Vec::new().into_boxed_slice())
}

/// Finds the nearest enclosing [`LFrame::Handle`] declaring `op`, reachable
/// across a **structural** prefix from the top of `cont` (the CEK's
/// `capture_to_handler`, over the reified frame stack).
///
/// # Contract
/// - ensures: `Some(index)` of the nearest [`LFrame::Handle`] whose handler
///   declares a clause for `op` when every frame above it is structural; `None`
///   when the search reaches the bottom, crosses a [`LFrame::Reset`] delimiter,
///   or the nearest handler does not declare `op` (the v0 single-handler scope
///   — routing is by operation name, ADR-33 D3).
/// - panics: none.
#[must_use]
fn capture_to_handler(
    cont: &[LFrame],
    op: OperationName<'_>,
) -> Option<FrameIndex>
{
    for (index, frame) in cont.iter().enumerate().rev() {
        match *frame {
            | LFrame::Reset => return None,
            | LFrame::Handle { ref handler, .. } => {
                // The nearest handler is decisive: it declares `op` (handle) or it
                // does not (an intervening handler for another effect — the v0
                // structural model cannot reify a capture across it, matching the
                // CEK bailing to `PerformNoHandler`).
                return handler
                    .ops
                    .iter()
                    .any(|clause| clause.op == op.as_ref())
                    .then_some(index.into());
            },
            // Every other frame is a structural prefix the operation crosses.
            | _ => {},
        }
    }
    None
}

/// Finds the nearest enclosing [`LFrame::Reset`] prompt, reachable across a
/// **structural** prefix from the top of `cont` (the CEK's `capture_to_reset`,
/// over the reified frame stack — the delimiter a `shift` captures up to).
///
/// # Contract
/// - ensures: `Some(index)` of the nearest [`LFrame::Reset`] when every frame
///   above it is structural; `None` when the search reaches the bottom with no
///   prompt, or would cross a [`LFrame::Handle`] (the unreifiable case, blamed
///   `ShiftNoReset` by the caller — ADR-34 D5, matching the CEK).
/// - panics: none.
#[must_use]
fn capture_to_reset(cont: &[LFrame]) -> Option<FrameIndex>
{
    for (index, frame) in cont.iter().enumerate().rev() {
        match *frame {
            | LFrame::Reset => return Some(index.into()),
            // A handler frame blocks the capture: the v0 structural model cannot
            // reify a delimited continuation across a handler (the CEK bails to
            // `ShiftNoReset`).
            | LFrame::Handle { .. } => return None,
            // Every other frame is a structural prefix the capture crosses.
            | _ => {},
        }
    }
    None
}

/// Whether a value is a `perform` operation constructor (the only source of an
/// `Op` tag is the focusing of `Comp::Perform`), so it propagates to its
/// handler rather than being consumed locally (§6).
#[inline]
#[must_use]
fn is_operation(value: &LValue) -> OperationConstructorStatus
{
    matches!(*value, LValue::Ctor {
        tag: CtorTag::Op { .. },
        ..
    })
    .into()
}

/// The stuck reason for a non-constructor (or non-matching) value meeting a
/// `case` frame, classified by the arms' constructor family so it matches the
/// CEK's distinct `Case` / `Split` / `ListCase` outcomes.
fn non_ctor_stuck(arms: &[Arm]) -> StuckReason
{
    match arms.first().map(|arm| &arm.ctor) {
        | Some(&CtorTag::Pair) => StuckReason::SplitNonProduct,
        | Some(&CtorTag::Nil | &CtorTag::Cons) => StuckReason::ListCasedNonList,
        // A single-`Here`-arm case is a focused `Walk`: a non-`here` scrutinee
        // is the CEK's `WalkOnNonHere` (ADR-76).
        | Some(&CtorTag::Here) => StuckReason::WalkOnNonHere,
        // A `Data`-arm case is a focused `DataCase`: a non-`Ctor` scrutinee (or
        // an out-of-range tag, which the arm loop misses) is the CEK's
        // `DataCasedNonCtor` (ADR-80). An EMPTY case is the absurd `DataCase`
        // (its arms are the only focused case with no arm — every other has a
        // fixed non-zero arity), so a `None` head is likewise `DataCasedNonCtor`.
        | Some(&CtorTag::Data(_)) | None => StuckReason::DataCasedNonCtor,
        | _ => StuckReason::CasedNonSum,
    }
}

/// Whether a native is a **higher-order combinator** — one that forces and
/// applies a thunk argument (`each` / `where` / `reduce` / `any` / `all` /
/// `update_at` / `update_where`). These need the argument thunk's source body,
/// which the focused IL no longer retains, so the L machine routes them through
/// the un-focusing readback ([`LMachine::dispatch_native_higher_order`]); every
/// other native takes the first-order structural path
/// ([`LMachine::dispatch_native`]).
fn native_needs_unfocus(prim: NativePrim) -> UnfocusRequirement
{
    matches!(
        prim,
        NativePrim::Each
            | NativePrim::Where
            | NativePrim::Reduce
            | NativePrim::Any
            | NativePrim::All
            | NativePrim::UpdateAt
            | NativePrim::UpdateWhere
    )
    .into()
}

/// Converts a first-order source [`Value`] to a machine value — the inverse of
/// [`read_value`] on the positive fragment, used to feed a native's result back
/// to the continuation.
fn value_to_lvalue(
    value: &Value,
    marks: &[Rc<LValue>],
) -> Option<Rc<LValue>>
{
    enum Task<'value>
    {
        Source(&'value Value),
        BuildPair,
        BuildInj(Side),
        BuildList(usize),
        BuildRecord
        {
            labels: Vec<String>,
            len: usize,
        },
    }

    let mut work = alloc::vec![Task::Source(value)];
    let mut values: Vec<Rc<LValue>> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            | Task::Source(source) => match *source {
                | Value::Unit => values.push(Rc::new(LValue::Lit(Lit::Unit))),
                | Value::Int(literal) => values.push(Rc::new(LValue::Lit(Lit::Int(literal)))),
                | Value::Str(ref literal) => {
                    values.push(Rc::new(LValue::Lit(Lit::Str(literal.clone()))));
                },
                | Value::Num(literal) => values.push(Rc::new(LValue::Lit(Lit::Num(literal)))),
                | Value::Hole(hole) => values.push(Rc::new(LValue::Lit(Lit::Hole(hole)))),
                | Value::Var(ref name) => values.push(Rc::new(LValue::Var(name.clone()))),
                | Value::Pair(ref fst, ref snd) => {
                    work.push(Task::BuildPair);
                    work.push(Task::Source(snd));
                    work.push(Task::Source(fst));
                },
                | Value::Inj(side, ref payload) => {
                    work.push(Task::BuildInj(side));
                    work.push(Task::Source(payload));
                },
                | Value::List(ref elements) => {
                    work.push(Task::BuildList(elements.len()));
                    for element in elements.iter().rev() {
                        work.push(Task::Source(element));
                    }
                },
                | Value::Record(ref fields) => {
                    let labels = fields.keys().cloned().collect();
                    work.push(Task::BuildRecord {
                        labels,
                        len: fields.len(),
                    });
                    for field in fields.values().rev() {
                        work.push(Task::Source(field));
                    }
                },
                | Value::Thunk(grade, ref body) => {
                    let Comp::Hole(index) = **body
                    else {
                        return None;
                    };
                    let mark_index = usize::try_from(index).ok()?;
                    let original = marks.get(mark_index)?;
                    let LValue::Thunk {
                        grade: original_grade,
                        ..
                    } = **original
                    else {
                        return None;
                    };
                    if original_grade != grade {
                        return None;
                    }
                    values.push(Rc::clone(original));
                },
                | Value::Annot(ref inner, _) => work.push(Task::Source(inner)),
                | _ => return None,
            },
            | Task::BuildPair => {
                let snd = values.pop()?;
                let fst = values.pop()?;
                values.push(Rc::new(LValue::Ctor {
                    tag: CtorTag::Pair,
                    args: Vec::from([fst, snd]),
                }));
            },
            | Task::BuildInj(side) => {
                let payload = values.pop()?;
                values.push(Rc::new(LValue::Ctor {
                    tag: CtorTag::Inj(side),
                    args: Vec::from([payload]),
                }));
            },
            | Task::BuildList(len) => {
                let start = values.len().checked_sub(len)?;
                let elements = values.split_off(start);
                let mut spine = Rc::new(LValue::Ctor {
                    tag: CtorTag::Nil,
                    args: Vec::new(),
                });
                for element in elements.into_iter().rev() {
                    spine = Rc::new(LValue::Ctor {
                        tag: CtorTag::Cons,
                        args: Vec::from([element, spine]),
                    });
                }
                values.push(spine);
            },
            | Task::BuildRecord { labels, len } => {
                let start = values.len().checked_sub(len)?;
                let args = values.split_off(start);
                values.push(Rc::new(LValue::Ctor {
                    tag: CtorTag::Record(labels.into_boxed_slice()),
                    args,
                }));
            },
        }
    }
    values.pop()
}

/// Reads a first-order machine value back to a source [`Value`] (the readback /
/// quote for the positive fragment).
fn read_value(
    value: &Rc<LValue>,
    marks: &mut Vec<Rc<LValue>>,
) -> Option<Value>
{
    enum Task
    {
        Machine(Rc<LValue>),
        BuildPair,
        BuildInj(Side),
        BuildList(usize),
        BuildRecord
        {
            fields: Vec<String>,
            len: usize,
        },
    }

    let mut work = alloc::vec![Task::Machine(Rc::clone(value))];
    let mut values: Vec<Value> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            | Task::Machine(machine) => match *machine {
                | LValue::Var(ref name) => values.push(Value::var(name)),
                | LValue::Lit(ref lit) => values.push(read_lit(lit)),
                | LValue::Ctor { ref tag, ref args } => match *tag {
                    | CtorTag::Pair => {
                        let fst_arg = args.first()?;
                        let snd_arg = args.get(1)?;
                        let fst = Rc::clone(fst_arg);
                        let snd = Rc::clone(snd_arg);
                        work.push(Task::BuildPair);
                        work.push(Task::Machine(snd));
                        work.push(Task::Machine(fst));
                    },
                    | CtorTag::Inj(side) => {
                        let payload_arg = args.first()?;
                        let payload = Rc::clone(payload_arg);
                        work.push(Task::BuildInj(side));
                        work.push(Task::Machine(payload));
                    },
                    | CtorTag::Nil | CtorTag::Cons => {
                        let mut elements: Vec<Rc<LValue>> = Vec::new();
                        let mut cursor = Rc::clone(&machine);
                        loop {
                            let next = match *cursor {
                                | LValue::Ctor {
                                    tag: CtorTag::Nil, ..
                                } => break,
                                | LValue::Ctor {
                                    tag: CtorTag::Cons,
                                    args: ref cell_args,
                                } => {
                                    let head = cell_args.first()?;
                                    let tail = cell_args.get(1)?;
                                    elements.push(Rc::clone(head));
                                    Rc::clone(tail)
                                },
                                | _ => return None,
                            };
                            cursor = next;
                        }
                        work.push(Task::BuildList(elements.len()));
                        for element in elements.into_iter().rev() {
                            work.push(Task::Machine(element));
                        }
                    },
                    | CtorTag::Record(ref labels) => {
                        // A record ctor's arity equals its label count (the
                        // well-formedness check enforces it), so the fields and
                        // the argument schedule need no intermediate pairing.
                        let fields = labels.to_vec();
                        work.push(Task::BuildRecord {
                            len: args.len(),
                            fields,
                        });
                        for arg in args.iter().rev() {
                            work.push(Task::Machine(Rc::clone(arg)));
                        }
                    },
                    | CtorTag::Op { .. } => return None,
                    // An identity proof `here(w)` (ADR-76). The differential
                    // canonicalizes a returned `Value::Here` opaquely (kind
                    // granularity, like a thunk / function terminal — it does
                    // NOT descend into the witness), so the readback is a
                    // representative with a placeholder witness rather than a
                    // recursive quote that could fail on a higher-order witness.
                    | CtorTag::Here => values.push(Value::here(Value::Unit)),
                    // A declared-data constructor `Ctor { id, tag, w }` (ADR-80).
                    // Canonicalized opaquely as `Here` is — the id (erased by
                    // `𝓕`), the tag, and the payload are all invisible to the
                    // differential — so the readback is a representative with a
                    // placeholder id and payload (matching the render-only role
                    // of the nominal id).
                    | CtorTag::Data(tag) => values.push(Value::Ctor {
                        id: DataId::new(0_u64, ""),
                        tag,
                        payload: Rc::new(Value::Unit),
                    }),
                },
                | LValue::Thunk { grade, .. } => {
                    let index = u32::try_from(marks.len()).ok()?;
                    marks.push(Rc::clone(&machine));
                    values.push(Value::thunk(grade, Comp::hole(index)));
                },
                | LValue::Boxed(_) | LValue::Cocase { .. } | LValue::Native { .. } => return None,
            },
            | Task::BuildPair => {
                let snd = values.pop()?;
                let fst = values.pop()?;
                values.push(Value::pair(fst, snd));
            },
            | Task::BuildInj(side) => {
                let payload = values.pop()?;
                values.push(Value::Inj(side, Rc::new(payload)));
            },
            | Task::BuildList(len) => {
                let start = values.len().checked_sub(len)?;
                let elements = values.split_off(start);
                values.push(Value::list(elements));
            },
            | Task::BuildRecord { fields, len } => {
                let start = values.len().checked_sub(len)?;
                let values_for_fields = values.split_off(start);
                values.push(Value::record(fields.into_iter().zip(values_for_fields)));
            },
        }
    }
    values.pop()
}

/// Reads a positive scalar leaf back to a source [`Value`].
fn read_lit(lit: &Lit) -> Value
{
    match *lit {
        | Lit::Unit => Value::Unit,
        | Lit::Int(value) => Value::int(value),
        | Lit::Str(ref value) => Value::string(value),
        | Lit::Num(num) => Value::Num(num),
        | Lit::Hole(hole) => Value::hole(hole),
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;

    use crate::focus::focus_comp;
    use crate::machine::LMachine;

    /// `ret 1` terminates at the returned value.
    #[test]
    fn ret_is_a_terminal_value()
    {
        assert_eq!(
            l_run(&Comp::ret(Value::int(1))),
            Eval::Value(Comp::ret(Value::int(1))),
            "ret 1 returns 1"
        );
    }

    /// `let x <- ret 1; ret x` binds through the μ̃ consumer.
    #[test]
    fn bind_threads_a_value()
    {
        let comp = Comp::bind(Comp::ret(Value::int(7)), "x", Comp::ret(Value::var("x")));
        assert_eq!(
            l_run(&comp),
            Eval::Value(Comp::ret(Value::int(7))),
            "the bound value flows through"
        );
    }

    /// `force (thunk { ret 2 })` runs the thunk body (call-by-need).
    #[test]
    fn force_runs_a_thunk_body()
    {
        let comp = Comp::force(Value::thunk(Grade::ONE, Comp::ret(Value::int(2))));
        assert_eq!(
            l_run(&comp),
            Eval::Value(Comp::ret(Value::int(2))),
            "forcing the thunk returns 2"
        );
    }

    /// A `case` on the left injection selects the left arm.
    #[test]
    fn case_selects_the_matching_arm()
    {
        let comp = Comp::case(
            Value::inj1(Value::int(3)),
            "l",
            Comp::ret(Value::var("l")),
            "r",
            Comp::ret(Value::int(0)),
        );
        assert_eq!(
            l_run(&comp),
            Eval::Value(Comp::ret(Value::int(3))),
            "the left arm binds the payload"
        );
    }

    /// Runs a core computation on the L machine over its focused form.
    fn l_run(comp: &Comp) -> Eval
    {
        let focused = focus_comp(comp).expect("𝓕 is total");
        let (arena, root, origins) = focused.into_parts();
        LMachine::new(arena, origins).run(root)
    }
}
