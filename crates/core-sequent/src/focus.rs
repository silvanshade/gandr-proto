//! The static focusing translation `𝓕` (the sequent-machines design's §3).
//!
//! `𝓕` takes a checked-core CBPV computation (or value) to a **focused**
//! command of the [`crate::il`] IL, Curien–Herbelin style and in the
//! administrative-redex-avoiding formulation: a non-value argument is μ̃-lifted
//! at its `Bind` seam rather than emitting a transparent administrative redex,
//! so the output carries no `let`-noise. It is the only entry into the IL, so a
//! machine built on it (phase L1) only ever sees focused states.
//!
//! # The core rules (§3)
//!
//! ```text
//! 𝓕⟦ret v⟧ α          = ⟨𝓥⟦v⟧ | α⟩
//! 𝓕⟦let x <- t; u⟧ α  = 𝓕⟦t⟧ (μ̃x. 𝓕⟦u⟧ α)          -- Bind IS μ̃
//! 𝓕⟦t v⟧ α            = 𝓕⟦t⟧ (ap(𝓥⟦v⟧; α))          -- application = ap destructor
//! 𝓕⟦λx. t⟧            = cocase { ap(x; α) ⇒ 𝓕⟦t⟧ α }
//! 𝓕⟦force v⟧ α        = ⟨𝓥⟦v⟧ | force(α)⟩
//! 𝓕⟦case v {…}⟧ α     = ⟨𝓥⟦v⟧ | case { K(x̄) ⇒ 𝓕⟦tᵢ⟧ α, … }⟩
//! ```
//!
//! # Adaptations beyond the §3 sketch
//!
//! The §3 sketch covers only the pure CBPV spine. The frozen core carries more
//! formers; each is translated in the same focusing discipline and recorded
//! here (the letters are cross-referenced from [`crate::il`]):
//!
//! - **A-LIST / A-RECORD**: `Value::List` becomes the inductive `Nil`/`Cons`
//!   view so `ListCase` matches the intro directly; `Value::Record` becomes a
//!   labeled constructor.
//! - **A-SPLIT / A-LISTCASE**: `Comp::Split` and `Comp::ListCase` are positive
//!   eliminations — a `case` on the `Pair`, resp. `Nil`/`Cons`, constructors.
//! - **A-RECORDPROJ**: `Comp::RecordProj` is a single-field destructor frame.
//! - **A-WITH**: the lazy pair `Comp::With` is negative-product intro — a
//!   `cocase` with `prj1`/`prj2` copatterns eliminated by
//!   [`crate::il::DtorTag::Prj`].
//! - The thunk's usage grade rides its binder into
//!   [`crate::il::ProducerNode::Thunk`] (retiring the former A-THUNK-GRADE
//!   adaptation — the grade is carried, not dropped).
//! - **A-DUP / A-DROP / A-NATIVE**: the grade structural ops and the
//!   Rust-backed builtins become opaque `prim` commands (§7.4 — natives have no
//!   cut seam).
//! - **A-PERFORM / A-RESET / A-HANDLE / A-RESUME / A-STK**: the effect and
//!   control surface (§6), operationally faithful at L1. `perform` is an
//!   operation constructor cut against the ambient continuation
//!   ([`crate::il::CtorTag::Op`]); the L machine propagates the operation up
//!   the reified frame stack to its handler ([`crate::machine`]
//!   `drive_perform`). `handle` / `reset` install their delimiter frame
//!   **eagerly** via [`FocusTask::BuildDelimiter`]: the handler
//!   ([`crate::il::HandlerConsumer`]) or prompt
//!   ([`crate::il::ConsumerNode::Prompt`]) is loaded onto the stack before the
//!   scrutinee / body runs (mirroring the CEK pushing `Cont::Handle` /
//!   `Cont::Reset`), so a dynamically-nested `perform` / `shift` finds the
//!   delimiter by walking the stack (`capture_to_handler` semantics carry over,
//!   §6). `resume` feeds a reified computation to a reified stack; `stk K`
//!   boxes a consumer into value position. **A-SHIFT** is the faithful
//!   delimited capture (now resolved): `shift k. s` emits a
//!   [`crate::il::ProducerNode::Shift`] — the μ-capture of the polarized
//!   reading (§6) — whose body is focused against `★` and cut against the local
//!   continuation; the L machine ([`crate::machine`] `drive_shift`) captures
//!   the machine stack up to the enclosing prompt as `k`, truncates below it,
//!   and runs `s` below the delimiter, exactly the CEK's `drive_shift`.
//!
//! Recursion (`fix`) is not a frozen-core `Comp` former — the core expresses
//! iteration through native builtins — so `𝓕` has no `fix` rule (the §3 `fix`
//! entry is aspirational, ADR-57). Any future `Comp` former the core grows is
//! focused, forward-compatibly, as an opaque hole
//! ([`FocusOrigin::Unsupported`]), keeping `𝓕` total.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::EffectSignatureName;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::OpClause;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Stack;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;

use crate::boundary::FocusResultCount;
use crate::boundary::UnsupportedFormerStatus;
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
use crate::il::HandlerOp;
use crate::il::Lit;
use crate::il::Polarity;
use crate::il::PrimOp;
use crate::il::ProducerId;
use crate::il::ProducerNode;

/// A translation failure (`𝓕`).
///
/// The translation is total on well-formed core terms; the only failure is
/// arena exhaustion, which cannot arise for any realistic term (it needs more
/// than `u32::MAX` nodes of one family).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusError
{
    /// A node arena exhausted its `u32` id space during translation.
    ArenaFull,
}

impl core::fmt::Display for FocusError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::ArenaFull => f.write_str("the command-IL arena exhausted its node id space"),
        }
    }
}

impl core::error::Error for FocusError
{
}

/// The source construct a translated command came from — the `ElabKind`
/// provenance (§3) that lets diagnostics un-sugar through the IL
/// (the sequent-machines design's §9 inspection surface).
///
/// Recorded only for the commands `𝓕` *creates* (cuts and prim invocations);
/// the consumer-extending rules (`App` / `Bind` / `Prj` / `Reset` / `Handle`)
/// create no command and so record no origin — the returned command is the
/// sub-computation's.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FocusOrigin
{
    /// `ret v` — a cut of the value against its continuation.
    Ret,
    /// `force v` — a cut against the `force` destructor.
    Force,
    /// `λx. t` — a cut of the `ap` cocase against its continuation.
    Abs,
    /// `case v {…}` — a cut against a sum `case`.
    Case,
    /// `case v { Nil | Cons }` — a cut against a list `case` (A-LISTCASE).
    ListCase,
    /// `split v as (x, y)` — a cut against a `Pair` `case` (A-SPLIT).
    Split,
    /// `walk(p, C, (x). c)` — the identity eliminator as a cut against a
    /// single-`Here`-arm `case` (ADR-76; the motive is runtime-erased).
    Walk,
    /// `case v { C₀(x₀) ⇒ t₀ | … }` — the declared-data eliminator as a cut
    /// against a positional `Data` `case` (ADR-80).
    DataCase,
    /// `r.ℓ` — a cut against a record-projection frame (A-RECORDPROJ).
    RecordProj,
    /// `⟨t, u⟩` — a cut of the lazy-pair cocase (A-WITH).
    With,
    /// `perform op v` — a cut of the operation constructor (A-PERFORM).
    Perform,
    /// `shift k. t` — a cut reifying the continuation as `k` (A-SHIFT).
    Shift,
    /// `resume v t` — a cut feeding a reified computation to a stack
    /// (A-RESUME).
    Resume,
    /// `handle E t { … }` — the delimiter-entry cut that installs the handler
    /// frame eagerly then runs the scrutinee against it (A-HANDLE; §6).
    Handle,
    /// `reset t` — the delimiter-entry cut that installs the prompt frame
    /// eagerly then runs the body against it (A-RESET; §6).
    Reset,
    /// A computation hole `?u` — a cut of an opaque hole.
    CompHole,
    /// `dup v` — a grade structural prim (A-DUP).
    Dup,
    /// `drop v` — a grade structural prim (A-DROP).
    Drop,
    /// A native builtin `prim` invocation (A-NATIVE).
    Native,
    /// A top-level value term — a cut of the value against `★`.
    TopValue,
    /// A forward-compatibility placeholder: an unrecognized future `Comp`
    /// former, focused as an opaque hole so `𝓕` stays total as the core
    /// grows.
    Unsupported,
}

/// A completed focusing: the arena, the root command, and the provenance table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Focused
{
    /// The arena holding every translated node.
    arena: CommandArena,
    /// The root command `𝓕⟦t⟧ ★`.
    root: CommandId,
    /// The provenance of each created command.
    origins: BTreeMap<CommandId, FocusOrigin>,
}

impl Focused
{
    /// The arena holding the translated IL.
    #[inline]
    #[must_use]
    pub fn arena(&self) -> &CommandArena
    {
        &self.arena
    }

    /// The root command `𝓕⟦t⟧ ★`.
    #[inline]
    #[must_use]
    pub fn root(&self) -> CommandId
    {
        self.root
    }

    /// The full provenance table.
    #[inline]
    #[must_use]
    pub fn origins(&self) -> &BTreeMap<CommandId, FocusOrigin>
    {
        &self.origins
    }

    /// The provenance of a specific command, if it is one `𝓕` created.
    #[inline]
    #[must_use]
    pub fn origin(
        &self,
        command: CommandId,
    ) -> Option<FocusOrigin>
    {
        self.origins.get(&command).copied()
    }

    /// Consumes the focusing into its owned arena, root command, and provenance
    /// table — the entry the phase-L1 machine drives, which OWNS
    /// the arena so a saturated native's re-focused result term can lower back
    /// into it, and consults the origins to distinguish the two holes the
    /// focused IL conflates (a returned `Value::Hole` is a value; a
    /// `Comp::Hole` is a [`FocusOrigin::CompHole`] cut that blames —
    /// matching the CEK).
    ///
    /// # Contract
    /// - ensures: the returned arena, root, and origins are `𝓕`'s own; `root`
    ///   resolves in the arena and is a typed-IL well-formed command.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (CommandArena, CommandId, BTreeMap<CommandId, FocusOrigin>)
    {
        (self.arena, self.root, self.origins)
    }
}

/// Monotone serial counter for freshly minted covariable names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FreshCoVarCounter(u64);

/// The translation state: the growing arena, a monotone fresh-covariable
/// counter, and the provenance table.
struct Focuser
{
    /// The arena being built.
    arena: CommandArena,
    /// The next fresh-covariable serial number.
    fresh: FreshCoVarCounter,
    /// The provenance of each created command.
    origins: BTreeMap<CommandId, FocusOrigin>,
}

impl Focuser
{
    /// Creates an empty translation state.
    fn new() -> Self
    {
        Self {
            arena: CommandArena::new(),
            fresh: FreshCoVarCounter::default(),
            origins: BTreeMap::new(),
        }
    }

    /// Mints a fresh covariable name in the reserved `%`-prefixed namespace,
    /// disjoint from every frozen-core identifier.
    fn fresh_covar(&mut self) -> CoName
    {
        let name = format!("%k{}", self.fresh.0);
        self.fresh.0 = self.fresh.0.wrapping_add(1_u64);
        name
    }

    /// Allocates a producer, mapping arena exhaustion to
    /// [`FocusError::ArenaFull`].
    fn new_producer(
        &mut self,
        node: ProducerNode,
    ) -> Result<ProducerId, FocusError>
    {
        self.arena.alloc_producer(node).ok_or(FocusError::ArenaFull)
    }

    /// Allocates a consumer, mapping arena exhaustion to
    /// [`FocusError::ArenaFull`].
    fn new_consumer(
        &mut self,
        node: ConsumerNode,
    ) -> Result<ConsumerId, FocusError>
    {
        self.arena.alloc_consumer(node).ok_or(FocusError::ArenaFull)
    }

    /// Allocates a command, mapping arena exhaustion to
    /// [`FocusError::ArenaFull`].
    fn new_command(
        &mut self,
        node: CommandNode,
    ) -> Result<CommandId, FocusError>
    {
        self.arena.alloc_command(node).ok_or(FocusError::ArenaFull)
    }

    /// Allocates a cut command and records its provenance.
    fn cut(
        &mut self,
        pol: Polarity,
        producer: ProducerId,
        consumer: ConsumerId,
        origin: FocusOrigin,
    ) -> Result<CommandId, FocusError>
    {
        let id = self.new_command(CommandNode::Cut {
            pol,
            producer,
            consumer,
        })?;
        let _previous = self.origins.insert(id, origin);
        Ok(id)
    }

    /// Allocates a prim command and records its provenance.
    fn prim(
        &mut self,
        op: PrimOp,
        ps: Box<[ProducerId]>,
        cs: Box<[ConsumerId]>,
        origin: FocusOrigin,
    ) -> Result<CommandId, FocusError>
    {
        let id = self.new_command(CommandNode::Prim { op, ps, cs })?;
        let _previous = self.origins.insert(id, origin);
        Ok(id)
    }

    /// Translates a value to a producer (`𝓥`).
    fn focus_value(
        &mut self,
        value: &Value,
    ) -> Result<ProducerId, FocusError>
    {
        let result = self.focus(FocusTask::Value(value))?;
        debug_assert!(
            matches!(result, FocusResult::Producer(_)),
            "value focusing produced a non-producer"
        );
        match result {
            | FocusResult::Producer(id) => Ok(id),
            | _ => Ok(ProducerId::new(u32::MAX)),
        }
    }

    /// Translates a computation `t` against the continuation `cont` to a
    /// command (`𝓕⟦t⟧ cont`).
    fn focus_comp(
        &mut self,
        comp: &Comp,
        cont: ConsumerId,
    ) -> Result<CommandId, FocusError>
    {
        let result = self.focus(FocusTask::Comp { comp, cont })?;
        debug_assert!(
            matches!(result, FocusResult::Command(_)),
            "computation focusing produced a non-command"
        );
        match result {
            | FocusResult::Command(id) => Ok(id),
            | _ => Ok(CommandId::new(u32::MAX)),
        }
    }

    /// Runs the focusing translation with an explicit work stack. Build frames
    /// sit on the same stack as source nodes, so caller-provided value,
    /// computation, and reified-stack trees never consume the Rust call stack.
    fn focus(
        &mut self,
        initial: FocusTask<'_>,
    ) -> Result<FocusResult, FocusError>
    {
        let mut work = alloc::vec![initial];
        let mut results = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                | FocusTask::Value(value) => match *value {
                    | Value::Var(ref name) => {
                        let producer = self.new_producer(ProducerNode::Var(name.clone()))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Unit => {
                        let producer = self.new_producer(ProducerNode::Lit(Lit::Unit))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Int(literal) => {
                        let producer = self.new_producer(ProducerNode::Lit(Lit::Int(literal)))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Str(ref literal) => {
                        let producer =
                            self.new_producer(ProducerNode::Lit(Lit::Str(literal.clone())))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Num(literal) => {
                        let producer = self.new_producer(ProducerNode::Lit(Lit::Num(literal)))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Pair(ref fst, ref snd) => {
                        work.push(FocusTask::BuildPair);
                        work.push(FocusTask::Value(snd.as_ref()));
                        work.push(FocusTask::Value(fst.as_ref()));
                    },
                    | Value::Inj(side, ref payload) => {
                        work.push(FocusTask::BuildInj { side });
                        work.push(FocusTask::Value(payload.as_ref()));
                    },
                    | Value::List(ref elements) => {
                        let spine = self.new_producer(ProducerNode::Ctor {
                            tag: CtorTag::Nil,
                            ps: Box::from([]),
                            cs: Box::from([]),
                        })?;
                        if elements.is_empty() {
                            results.push(FocusResult::Producer(spine));
                        }
                        else {
                            let index = elements.len().saturating_sub(1);
                            work.push(FocusTask::BuildListCons {
                                elements,
                                index,
                                spine,
                            });
                            if let Some(element) = elements.last() {
                                work.push(FocusTask::Value(element.as_ref()));
                            }
                        }
                    },
                    | Value::Record(ref fields) => {
                        let labels = fields.keys().cloned().collect();
                        work.push(FocusTask::BuildRecord {
                            labels,
                            len: fields.len(),
                        });
                        for (_, field) in fields.iter().rev() {
                            work.push(FocusTask::Value(field.as_ref()));
                        }
                    },
                    | Value::Thunk(grade, ref body) => {
                        let cobinder = self.fresh_covar();
                        let cobinder_cons =
                            self.new_consumer(ConsumerNode::CoVar(cobinder.clone()))?;
                        work.push(FocusTask::BuildThunk { grade, cobinder });
                        work.push(FocusTask::Comp {
                            comp: body.as_ref(),
                            cont: cobinder_cons,
                        });
                    },
                    | Value::Annot(ref inner, _) => work.push(FocusTask::Value(inner.as_ref())),
                    // A packed module **erases to its payload**: its witnesses
                    // are types, and `𝓕` is type-erased, so a package has no
                    // runtime existence beyond the module it carries. The
                    // abstraction it buys is static — the checker refuses a
                    // client that reaches the representation, and there is
                    // nothing left at runtime for such a client to reach.
                    | Value::Pack { ref payload, .. } => {
                        work.push(FocusTask::Value(payload.as_ref()));
                    },
                    | Value::Hole(hole) => {
                        let producer = self.new_producer(ProducerNode::Lit(Lit::Hole(hole)))?;
                        results.push(FocusResult::Producer(producer));
                    },
                    | Value::Stk(ref stack) => {
                        work.push(FocusTask::BuildBoxed);
                        work.push(FocusTask::Stack(stack.as_ref()));
                    },
                    | Value::Here(ref witness) => {
                        work.push(FocusTask::BuildHere);
                        work.push(FocusTask::Value(witness.as_ref()));
                    },
                    | Value::Ctor {
                        tag, ref payload, ..
                    } => {
                        // A declared-data constructor is a unary positive intro
                        // keyed by its position (ADR-80); the nominal id is
                        // render-only, erased by `𝓕`.
                        work.push(FocusTask::BuildDataCtor { tag });
                        work.push(FocusTask::Value(payload.as_ref()));
                    },
                },
                | FocusTask::Comp { comp, cont } => match *comp {
                    | Comp::Abs(ref binder, _, ref body) => {
                        let arm_covar = self.fresh_covar();
                        let arm_cons = self.new_consumer(ConsumerNode::CoVar(arm_covar.clone()))?;
                        work.push(FocusTask::BuildAbs {
                            binder,
                            arm_covar,
                            cont,
                        });
                        work.push(FocusTask::Comp {
                            comp: body.as_ref(),
                            cont: arm_cons,
                        });
                    },
                    | Comp::App(ref head, ref arg) => {
                        work.push(FocusTask::BuildApp {
                            head: head.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(arg.as_ref()));
                    },
                    | Comp::Ret(ref value) => {
                        work.push(FocusTask::BuildRet { cont });
                        work.push(FocusTask::Value(value.as_ref()));
                    },
                    | Comp::Bind(ref bound, ref binder, ref continuation) => {
                        work.push(FocusTask::BuildBind {
                            bound: bound.as_ref(),
                            binder,
                        });
                        work.push(FocusTask::Comp {
                            comp: continuation.as_ref(),
                            cont,
                        });
                    },
                    // An unpack **erases to a bind of a return**: with the
                    // witnesses gone and the atoms a typing-time device, what
                    // is left of `unpack v as ⟨ā⟩ m in t` is `ret v >>= m. t`,
                    // and the two focus identically. The atom minting has no
                    // runtime image at all, which is the point — abstraction is
                    // a static property, so erasing it costs nothing.
                    | Comp::Unpack {
                        ref scrut,
                        ref binder,
                        ref body,
                        ..
                    } => {
                        work.push(FocusTask::BuildUnpack {
                            scrut: scrut.as_ref(),
                            binder,
                        });
                        work.push(FocusTask::Comp {
                            comp: body.as_ref(),
                            cont,
                        });
                    },
                    | Comp::Force(ref value) => {
                        work.push(FocusTask::BuildForce { cont });
                        work.push(FocusTask::Value(value.as_ref()));
                    },
                    | Comp::Case(
                        ref scrut,
                        (ref left_binder, ref left_body),
                        (ref right_binder, ref right_body),
                    ) => {
                        work.push(FocusTask::BuildCase {
                            left_binder,
                            right_binder,
                        });
                        work.push(FocusTask::Comp {
                            comp: right_body.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Comp {
                            comp: left_body.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(scrut.as_ref()));
                    },
                    | Comp::ListCase {
                        ref scrut,
                        ref nil,
                        ref head,
                        ref tail,
                        ref cons,
                    } => {
                        work.push(FocusTask::BuildListCase { head, tail });
                        work.push(FocusTask::Comp {
                            comp: cons.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Comp {
                            comp: nil.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(scrut.as_ref()));
                    },
                    | Comp::Split {
                        ref scrut,
                        fst_name: ref fst_binder,
                        snd_name: ref snd_binder,
                        ref body,
                        ..
                    } => {
                        work.push(FocusTask::BuildSplit {
                            fst_binder,
                            snd_binder,
                        });
                        work.push(FocusTask::Comp {
                            comp: body.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(scrut.as_ref()));
                    },
                    | Comp::RecordProj {
                        ref record,
                        ref label,
                    } => {
                        work.push(FocusTask::BuildRecordProj { label, cont });
                        work.push(FocusTask::Value(record.as_ref()));
                    },
                    | Comp::With(ref fst, ref snd) => {
                        let fst_covar = self.fresh_covar();
                        let fst_cons = self.new_consumer(ConsumerNode::CoVar(fst_covar.clone()))?;
                        work.push(FocusTask::BuildWithAfterFst {
                            snd: snd.as_ref(),
                            fst_covar,
                            cont,
                        });
                        work.push(FocusTask::Comp {
                            comp: fst.as_ref(),
                            cont: fst_cons,
                        });
                    },
                    | Comp::Prj(side, ref inner) => {
                        let frame = self.new_consumer(ConsumerNode::Dtor {
                            tag: DtorTag::Prj(side),
                            ps: Box::from([]),
                            cs: Box::from([cont]),
                        })?;
                        work.push(FocusTask::Comp {
                            comp: inner.as_ref(),
                            cont: frame,
                        });
                    },
                    | Comp::Dup(ref value) => {
                        work.push(FocusTask::BuildPrim {
                            op: PrimOp::Dup,
                            cont,
                            origin: FocusOrigin::Dup,
                        });
                        work.push(FocusTask::Value(value.as_ref()));
                    },
                    | Comp::Drop(ref value) => {
                        work.push(FocusTask::BuildPrim {
                            op: PrimOp::Drop,
                            cont,
                            origin: FocusOrigin::Drop,
                        });
                        work.push(FocusTask::Value(value.as_ref()));
                    },
                    | Comp::Perform(ref sig, ref op, ref payload) => {
                        work.push(FocusTask::BuildPerform { sig, op, cont });
                        work.push(FocusTask::Value(payload.as_ref()));
                    },
                    | Comp::Handle {
                        ref sig,
                        ref scrutinee,
                        ret: (ref ret_binder, ref ret_body),
                        ref ops,
                    } => {
                        let fallthrough = self.new_consumer(ConsumerNode::Top)?;
                        work.push(FocusTask::BuildHandle {
                            sig: sig.name(),
                            scrutinee: scrutinee.as_ref(),
                            ret_binder,
                            ops,
                            cont,
                        });
                        for clause in ops.iter().rev() {
                            work.push(FocusTask::Comp {
                                comp: clause.body.as_ref(),
                                cont: fallthrough,
                            });
                        }
                        work.push(FocusTask::Comp {
                            comp: ret_body.as_ref(),
                            cont: fallthrough,
                        });
                    },
                    | Comp::Resume(ref stack, ref computation) => {
                        work.push(FocusTask::BuildResumeAfterStack {
                            computation: computation.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(stack.as_ref()));
                    },
                    | Comp::Reset(ref inner) => {
                        let prompt = self.new_consumer(ConsumerNode::Prompt(cont))?;
                        work.push(FocusTask::Delimiter {
                            body: inner.as_ref(),
                            delimiter: prompt,
                            origin: FocusOrigin::Reset,
                        });
                    },
                    | Comp::Shift(ref binder, ref body) => {
                        let top = self.new_consumer(ConsumerNode::Top)?;
                        work.push(FocusTask::BuildShift { binder, cont });
                        work.push(FocusTask::Comp {
                            comp: body.as_ref(),
                            cont: top,
                        });
                    },
                    | Comp::Hole(hole) => {
                        let producer = self.new_producer(ProducerNode::Lit(Lit::Hole(hole)))?;
                        let command =
                            self.cut(Polarity::Positive, producer, cont, FocusOrigin::CompHole)?;
                        results.push(FocusResult::Command(command));
                    },
                    | Comp::Native { prim, ref args } => {
                        work.push(FocusTask::BuildNative {
                            prim,
                            len: args.len(),
                            cont,
                        });
                        for arg in args.iter().rev() {
                            work.push(FocusTask::Value(arg.as_ref()));
                        }
                    },
                    | Comp::Walk {
                        ref scrut,
                        ref base,
                        ..
                    } => {
                        // The identity eliminator is a single-`Here`-arm case:
                        // Walk-β binds the diagonal base binder to the witness and
                        // runs the base body (ADR-76). The motive is a type child,
                        // runtime-erased, so `𝓕` drops it — exactly the CEK, whose
                        // Walk reduction reads only the scrutinee and base.
                        work.push(FocusTask::BuildWalk {
                            base_binder: &base.x,
                        });
                        work.push(FocusTask::Comp {
                            comp: base.body.as_ref(),
                            cont,
                        });
                        work.push(FocusTask::Value(scrut.as_ref()));
                    },
                    | Comp::DataCase(ref scrut, ref arms) => {
                        // The declared-data eliminator is a positional `Data`
                        // case: arm `i` matches tag `i`, binding its single
                        // binder to the WHOLE field-tuple payload and running the
                        // arm body — exactly the CEK's `arms.nth(tag)` selecting
                        // by position (ADR-80). An empty `arms` is the absurd
                        // match. The scrutinee focuses first, then each arm body
                        // in order (pushed in reverse so they pop source-order).
                        #[expect(clippy::needless_borrowed_reference, reason = "false positive")]
                        let binders: Vec<&String> =
                            arms.iter().map(|&(ref binder, _)| binder).collect();
                        work.push(FocusTask::BuildDataCase { binders });
                        for arm in arms.iter().rev() {
                            work.push(FocusTask::Comp {
                                comp: arm.1.as_ref(),
                                cont,
                            });
                        }
                        work.push(FocusTask::Value(scrut.as_ref()));
                    },
                },
                | FocusTask::Delimiter {
                    body,
                    delimiter,
                    origin,
                } => {
                    let top = self.new_consumer(ConsumerNode::Top)?;
                    work.push(FocusTask::BuildDelimiter { delimiter, origin });
                    work.push(FocusTask::Comp {
                        comp: body,
                        cont: top,
                    });
                },
                | FocusTask::Stack(stack) => match *stack {
                    | Stack::Arg(ref value, ref rest) => {
                        work.push(FocusTask::BuildStackArg);
                        work.push(FocusTask::Stack(rest.as_ref()));
                        work.push(FocusTask::Value(value.as_ref()));
                    },
                    | Stack::Bind(ref binder, ref continuation, ref rest) => {
                        work.push(FocusTask::BuildStackBindAfterRest {
                            binder,
                            continuation: continuation.as_ref(),
                        });
                        work.push(FocusTask::Stack(rest.as_ref()));
                    },
                    | Stack::Prj(side, ref rest) => {
                        work.push(FocusTask::BuildStackPrj { side });
                        work.push(FocusTask::Stack(rest.as_ref()));
                    },
                    | _ => {
                        let consumer = self.new_consumer(ConsumerNode::Top)?;
                        results.push(FocusResult::Consumer(consumer));
                    },
                },
                | FocusTask::BuildPair => {
                    let snd = pop_producer(&mut results);
                    let fst = pop_producer(&mut results);
                    let producer = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Pair,
                        ps: Box::from([fst, snd]),
                        cs: Box::from([]),
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildInj { side } => {
                    let payload = pop_producer(&mut results);
                    let producer = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Inj(side),
                        ps: Box::from([payload]),
                        cs: Box::from([]),
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildListCons {
                    elements,
                    index,
                    spine,
                } => {
                    let head = pop_producer(&mut results);
                    let spine = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Cons,
                        ps: Box::from([head, spine]),
                        cs: Box::from([]),
                    })?;
                    if index == 0 {
                        results.push(FocusResult::Producer(spine));
                    }
                    else {
                        let next = index.saturating_sub(1);
                        work.push(FocusTask::BuildListCons {
                            elements,
                            index: next,
                            spine,
                        });
                        if let Some(element) = elements.get(next) {
                            work.push(FocusTask::Value(element.as_ref()));
                        }
                    }
                },
                | FocusTask::BuildRecord { labels, len } => {
                    let ps = pop_producers(&mut results, FocusResultCount::from(len));
                    let producer = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Record(labels.into_boxed_slice()),
                        ps: ps.into_boxed_slice(),
                        cs: Box::from([]),
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildThunk { grade, cobinder } => {
                    let body = pop_command(&mut results);
                    let producer = self.new_producer(ProducerNode::Thunk {
                        grade,
                        cobinder,
                        body,
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildBoxed => {
                    let consumer = pop_consumer(&mut results);
                    let producer = self.new_producer(ProducerNode::Boxed(consumer))?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildAbs {
                    binder,
                    arm_covar,
                    cont,
                } => {
                    let body = pop_command(&mut results);
                    let arm = CoArm {
                        dtor: DtorTag::Ap,
                        binders: Box::from([binder.clone()]),
                        cobinders: Box::from([arm_covar]),
                        body,
                    };
                    let cocase = self.new_producer(ProducerNode::Cocase {
                        arms: Box::from([arm]),
                    })?;
                    let command = self.cut(Polarity::Negative, cocase, cont, FocusOrigin::Abs)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildApp { head, cont } => {
                    let arg = pop_producer(&mut results);
                    let frame = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::Ap,
                        ps: Box::from([arg]),
                        cs: Box::from([cont]),
                    })?;
                    work.push(FocusTask::Comp {
                        comp: head,
                        cont: frame,
                    });
                },
                | FocusTask::BuildRet { cont } => {
                    let producer = pop_producer(&mut results);
                    let command = self.cut(Polarity::Positive, producer, cont, FocusOrigin::Ret)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildBind { bound, binder } => {
                    let continuation = pop_command(&mut results);
                    let mu =
                        self.new_consumer(ConsumerNode::MuTilde(binder.clone(), continuation))?;
                    work.push(FocusTask::Comp {
                        comp: bound,
                        cont: mu,
                    });
                },
                | FocusTask::BuildUnpack { scrut, binder } => {
                    let continuation = pop_command(&mut results);
                    let mu =
                        self.new_consumer(ConsumerNode::MuTilde(binder.clone(), continuation))?;
                    work.push(FocusTask::BuildRet { cont: mu });
                    work.push(FocusTask::Value(scrut));
                },
                | FocusTask::BuildForce { cont } => {
                    let producer = pop_producer(&mut results);
                    let frame = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::Force,
                        ps: Box::from([]),
                        cs: Box::from([cont]),
                    })?;
                    let command =
                        self.cut(Polarity::Positive, producer, frame, FocusOrigin::Force)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildCase {
                    left_binder,
                    right_binder,
                } => {
                    let right = pop_command(&mut results);
                    let left = pop_command(&mut results);
                    let producer = pop_producer(&mut results);
                    let arms = Box::from([
                        Arm {
                            ctor: CtorTag::Inj(Side::Fst),
                            binders: Box::from([left_binder.clone()]),
                            cobinders: Box::from([]),
                            body: left,
                        },
                        Arm {
                            ctor: CtorTag::Inj(Side::Snd),
                            binders: Box::from([right_binder.clone()]),
                            cobinders: Box::from([]),
                            body: right,
                        },
                    ]);
                    let case = self.new_consumer(ConsumerNode::Case { arms })?;
                    let command =
                        self.cut(Polarity::Positive, producer, case, FocusOrigin::Case)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildListCase { head, tail } => {
                    let cons = pop_command(&mut results);
                    let nil = pop_command(&mut results);
                    let producer = pop_producer(&mut results);
                    let arms = Box::from([
                        Arm {
                            ctor: CtorTag::Nil,
                            binders: Box::from([]),
                            cobinders: Box::from([]),
                            body: nil,
                        },
                        Arm {
                            ctor: CtorTag::Cons,
                            binders: Box::from([head.clone(), tail.clone()]),
                            cobinders: Box::from([]),
                            body: cons,
                        },
                    ]);
                    let case = self.new_consumer(ConsumerNode::Case { arms })?;
                    let command =
                        self.cut(Polarity::Positive, producer, case, FocusOrigin::ListCase)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildSplit {
                    fst_binder,
                    snd_binder,
                } => {
                    let body = pop_command(&mut results);
                    let producer = pop_producer(&mut results);
                    let arms = Box::from([Arm {
                        ctor: CtorTag::Pair,
                        binders: Box::from([fst_binder.clone(), snd_binder.clone()]),
                        cobinders: Box::from([]),
                        body,
                    }]);
                    let case = self.new_consumer(ConsumerNode::Case { arms })?;
                    let command =
                        self.cut(Polarity::Positive, producer, case, FocusOrigin::Split)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildHere => {
                    let witness = pop_producer(&mut results);
                    let producer = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Here,
                        ps: Box::from([witness]),
                        cs: Box::from([]),
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildWalk { base_binder } => {
                    let body = pop_command(&mut results);
                    let producer = pop_producer(&mut results);
                    let arms = Box::from([Arm {
                        ctor: CtorTag::Here,
                        binders: Box::from([base_binder.clone()]),
                        cobinders: Box::from([]),
                        body,
                    }]);
                    let case = self.new_consumer(ConsumerNode::Case { arms })?;
                    let command =
                        self.cut(Polarity::Positive, producer, case, FocusOrigin::Walk)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildDataCtor { tag } => {
                    let payload = pop_producer(&mut results);
                    let producer = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Data(tag),
                        ps: Box::from([payload]),
                        cs: Box::from([]),
                    })?;
                    results.push(FocusResult::Producer(producer));
                },
                | FocusTask::BuildDataCase { binders } => {
                    let bodies = pop_commands(&mut results, FocusResultCount::from(binders.len()));
                    let producer = pop_producer(&mut results);
                    let arms = binders
                        .into_iter()
                        .zip(bodies)
                        .enumerate()
                        .map(|(index, (binder, body))| Arm {
                            ctor: CtorTag::Data(index),
                            binders: Box::from([binder.clone()]),
                            cobinders: Box::from([]),
                            body,
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice();
                    let case = self.new_consumer(ConsumerNode::Case { arms })?;
                    let command =
                        self.cut(Polarity::Positive, producer, case, FocusOrigin::DataCase)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildRecordProj { label, cont } => {
                    let producer = pop_producer(&mut results);
                    let frame = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::RecordProj(label.clone()),
                        ps: Box::from([]),
                        cs: Box::from([cont]),
                    })?;
                    let command =
                        self.cut(Polarity::Positive, producer, frame, FocusOrigin::RecordProj)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildWithAfterFst {
                    snd,
                    fst_covar,
                    cont,
                } => {
                    let fst_cmd = pop_command(&mut results);
                    let snd_covar = self.fresh_covar();
                    let snd_cons = self.new_consumer(ConsumerNode::CoVar(snd_covar.clone()))?;
                    work.push(FocusTask::BuildWith {
                        fst_cmd,
                        fst_covar,
                        snd_covar,
                        cont,
                    });
                    work.push(FocusTask::Comp {
                        comp: snd,
                        cont: snd_cons,
                    });
                },
                | FocusTask::BuildWith {
                    fst_cmd,
                    fst_covar,
                    snd_covar,
                    cont,
                } => {
                    let snd_cmd = pop_command(&mut results);
                    let arms = Box::from([
                        CoArm {
                            dtor: DtorTag::Prj(Side::Fst),
                            binders: Box::from([]),
                            cobinders: Box::from([fst_covar]),
                            body: fst_cmd,
                        },
                        CoArm {
                            dtor: DtorTag::Prj(Side::Snd),
                            binders: Box::from([]),
                            cobinders: Box::from([snd_covar]),
                            body: snd_cmd,
                        },
                    ]);
                    let cocase = self.new_producer(ProducerNode::Cocase { arms })?;
                    let command = self.cut(Polarity::Negative, cocase, cont, FocusOrigin::With)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildPrim { op, cont, origin } => {
                    let producer = pop_producer(&mut results);
                    let command =
                        self.prim(op, Box::from([producer]), Box::from([cont]), origin)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildPerform { sig, op, cont } => {
                    let payload = pop_producer(&mut results);
                    let operation = self.new_producer(ProducerNode::Ctor {
                        tag: CtorTag::Op {
                            sig: sig.clone(),
                            op: op.clone(),
                        },
                        ps: Box::from([payload]),
                        cs: Box::from([]),
                    })?;
                    let command =
                        self.cut(Polarity::Positive, operation, cont, FocusOrigin::Perform)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildHandle {
                    sig,
                    scrutinee,
                    ret_binder,
                    ops,
                    cont,
                } => {
                    let body_cmds = pop_commands(&mut results, FocusResultCount::from(ops.len()));
                    let ret_cmd = pop_command(&mut results);
                    let handler_ops = ops
                        .iter()
                        .zip(body_cmds)
                        .map(|(clause, body)| HandlerOp {
                            op: clause.op.clone(),
                            payload: clause.payload.clone(),
                            resume: clause.resume.clone(),
                            body,
                        })
                        .collect();
                    let handler =
                        self.new_consumer(ConsumerNode::Handler(Box::new(HandlerConsumer {
                            sig: String::from(sig.as_ref()),
                            ret_binder: ret_binder.clone(),
                            ret_body: ret_cmd,
                            ops: handler_ops,
                            continuation: cont,
                        })))?;
                    work.push(FocusTask::Delimiter {
                        body: scrutinee,
                        delimiter: handler,
                        origin: FocusOrigin::Handle,
                    });
                },
                | FocusTask::BuildResumeAfterStack { computation, cont } => {
                    let stack = pop_producer(&mut results);
                    let cobinder = self.fresh_covar();
                    let cobinder_cons = self.new_consumer(ConsumerNode::CoVar(cobinder.clone()))?;
                    work.push(FocusTask::BuildResume {
                        stack,
                        cobinder,
                        cont,
                    });
                    work.push(FocusTask::Comp {
                        comp: computation,
                        cont: cobinder_cons,
                    });
                },
                | FocusTask::BuildResume {
                    stack,
                    cobinder,
                    cont,
                } => {
                    let comp_cmd = pop_command(&mut results);
                    let reified = self.new_producer(ProducerNode::Thunk {
                        grade: Grade::OMEGA,
                        cobinder,
                        body: comp_cmd,
                    })?;
                    let frame = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::Resume,
                        ps: Box::from([reified]),
                        cs: Box::from([cont]),
                    })?;
                    let command =
                        self.cut(Polarity::Positive, stack, frame, FocusOrigin::Resume)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildShift { binder, cont } => {
                    let body = pop_command(&mut results);
                    let capture = self.new_producer(ProducerNode::Shift {
                        binder: binder.clone(),
                        body,
                    })?;
                    let command =
                        self.cut(Polarity::Positive, capture, cont, FocusOrigin::Shift)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildNative { prim, len, cont } => {
                    let ps = pop_producers(&mut results, FocusResultCount::from(len));
                    let command = self.prim(
                        PrimOp::Native(prim),
                        ps.into_boxed_slice(),
                        Box::from([cont]),
                        FocusOrigin::Native,
                    )?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildDelimiter { delimiter, origin } => {
                    let body = pop_command(&mut results);
                    let binder = self.fresh_covar();
                    let mu = self.new_producer(ProducerNode::Mu(binder, body))?;
                    let command = self.cut(Polarity::Positive, mu, delimiter, origin)?;
                    results.push(FocusResult::Command(command));
                },
                | FocusTask::BuildStackArg => {
                    let rest = pop_consumer(&mut results);
                    let value = pop_producer(&mut results);
                    let consumer = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::Ap,
                        ps: Box::from([value]),
                        cs: Box::from([rest]),
                    })?;
                    results.push(FocusResult::Consumer(consumer));
                },
                | FocusTask::BuildStackBindAfterRest {
                    binder,
                    continuation,
                } => {
                    let rest = pop_consumer(&mut results);
                    work.push(FocusTask::BuildStackBind { binder });
                    work.push(FocusTask::Comp {
                        comp: continuation,
                        cont: rest,
                    });
                },
                | FocusTask::BuildStackBind { binder } => {
                    let continuation = pop_command(&mut results);
                    let consumer =
                        self.new_consumer(ConsumerNode::MuTilde(binder.clone(), continuation))?;
                    results.push(FocusResult::Consumer(consumer));
                },
                | FocusTask::BuildStackPrj { side } => {
                    let rest = pop_consumer(&mut results);
                    let consumer = self.new_consumer(ConsumerNode::Dtor {
                        tag: DtorTag::Prj(side),
                        ps: Box::from([]),
                        cs: Box::from([rest]),
                    })?;
                    results.push(FocusResult::Consumer(consumer));
                },
            }
        }
        debug_assert_eq!(1, results.len(), "focusing work stack left stray results");
        let result = results.pop();
        debug_assert!(
            result.is_some(),
            "focusing produces exactly one result for one root task"
        );
        Ok(result.unwrap_or_else(|| FocusResult::Producer(ProducerId::new(u32::MAX))))
    }

    /// Consumes the state into a [`Focused`] rooted at `root`.
    fn finish(
        self,
        root: CommandId,
    ) -> Focused
    {
        Focused {
            arena: self.arena,
            root,
            origins: self.origins,
        }
    }

    /// Rebuilds a translation state over an existing arena and provenance
    /// table, resuming the fresh-covariable counter — the seed for
    /// re-focusing a native's result term into a live machine arena
    /// ([`focus_comp_into`]).
    fn from_parts(
        arena: CommandArena,
        fresh: FreshCoVarCounter,
        origins: BTreeMap<CommandId, FocusOrigin>,
    ) -> Self
    {
        Self {
            arena,
            fresh,
            origins,
        }
    }

    /// Consumes the state into its raw arena, fresh counter, and provenance
    /// table (the companion of [`Self::from_parts`]).
    fn into_raw_parts(
        self
    ) -> (
        CommandArena,
        FreshCoVarCounter,
        BTreeMap<CommandId, FocusOrigin>,
    )
    {
        (self.arena, self.fresh, self.origins)
    }
}

/// Focuses a checked-core computation into an **existing** command arena.
///
/// The re-focusing entry the L machine drives when a saturated higher-order
/// native's result term (`gandr_core_checker::prim`) must be lowered back into
/// the live machine arena and run against the ambient continuation
/// ([`crate::machine`] `dispatch_native_higher_order`).
///
/// The result is focused against a freshly-minted covariable (not `★`), so a
/// machine binding that covariable to the empty continuation lets the result's
/// returning tail **fall through** to the ambient frame stack — exactly the
/// CEK's `Transition::Focus(prim.apply(&args), Env::empty())` continuing
/// against its live continuation.
///
/// `fresh` threads the covariable-name counter across re-focuses so freshly
/// minted covariables stay distinct within the arena.
///
/// # Contract
/// - ensures: on success, `arena` / `origins` gain the focused nodes, `fresh`
///   is advanced past the covariables minted, and the returned `(root,
///   cobinder)` is a well-formed command whose returning tail cuts against
///   `cobinder`.
/// - panics: none.
///
/// # Errors
/// [`FocusError::ArenaFull`] only on arena exhaustion.
#[inline]
pub fn focus_comp_into(
    arena: &mut CommandArena,
    origins: &mut BTreeMap<CommandId, FocusOrigin>,
    fresh: &mut FreshCoVarCounter,
    comp: &Comp,
) -> Result<(CommandId, CoName), FocusError>
{
    let mut focuser = Focuser::from_parts(core::mem::take(arena), *fresh, core::mem::take(origins));
    let cobinder = focuser.fresh_covar();
    let cobinder_cons = focuser.new_consumer(ConsumerNode::CoVar(cobinder.clone()))?;
    let root = focuser.focus_comp(comp, cobinder_cons)?;
    let (out_arena, out_fresh, out_origins) = focuser.into_raw_parts();
    *arena = out_arena;
    *fresh = out_fresh;
    *origins = out_origins;
    Ok((root, cobinder))
}

/// One focusing work-stack entry (ADR-47): either a source node to translate
/// or a build frame that assembles the focused images of its children. Build
/// frames sit on the same stack as source nodes, so caller-provided value,
/// computation, and reified-stack trees of any depth never consume the host
/// call stack.
enum FocusTask<'src>
{
    /// Translates a source value to its producer (`𝓥`).
    Value(&'src Value),
    /// Translates a source computation against a continuation (`𝓕⟦t⟧ cont`).
    Comp
    {
        /// The source computation.
        comp: &'src Comp,
        /// The continuation consumer the computation is cut against.
        cont: ConsumerId,
    },
    /// Translates a reified source stack (`stk K`) to a consumer.
    Stack(&'src Stack),
    /// Translates a delimited body against a fresh `★`, then cuts the
    /// resulting `μ` against the delimiter (a `reset` prompt or a handler).
    Delimiter
    {
        /// The delimited body.
        body: &'src Comp,
        /// The delimiter consumer (prompt / handler).
        delimiter: ConsumerId,
        /// The delimiter command's provenance.
        origin: FocusOrigin,
    },
    /// Assembles a `Pair` constructor from the top two producer results.
    BuildPair,
    /// Assembles an injection constructor from the top producer result.
    BuildInj
    {
        /// The injection side (`Inl` / `Inr`).
        side: Side,
    },
    /// Assembles one `Cons` link of the list spine (A-LIST: the flat vector is
    /// folded into a `Nil`/`Cons` spine right to left, so the intro is the
    /// inductive view the eliminator matches).
    BuildListCons
    {
        /// The full source element vector.
        elements: &'src [alloc::rc::Rc<Value>],
        /// The index of the element this link wraps.
        index: usize,
        /// The spine accumulated so far (initially `Nil`).
        spine: ProducerId,
    },
    /// Assembles a labeled record constructor from the top `len` producer
    /// results (A-RECORD: labels in canonical order).
    BuildRecord
    {
        /// The field labels, in canonical order.
        labels: Vec<String>,
        /// The number of field producers on the result stack.
        len: usize,
    },
    /// Assembles a thunk producer (the U-shift intro; the usage grade rides
    /// the binder, retiring A-THUNK-GRADE).
    BuildThunk
    {
        /// The thunk's usage grade.
        grade: Grade,
        /// The fresh covariable the body is cut against.
        cobinder: CoName,
    },
    /// Assembles a boxed consumer (`stk K` reifies the stack as a value).
    BuildBoxed,
    /// Assembles a lambda: a single-`Ap`-arm cocase cut negatively against the
    /// continuation.
    BuildAbs
    {
        /// The parameter binder.
        binder: &'src String,
        /// The fresh covariable the arm body is cut against.
        arm_covar: CoName,
        /// The continuation consumer the abstraction is cut against.
        cont: ConsumerId,
    },
    /// Continues an application: once the argument is focused (eager argument,
    /// ADR-50 Decision C), focus the head against the `ap` frame.
    BuildApp
    {
        /// The head computation.
        head: &'src Comp,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Cuts the top producer result against the continuation (`ret v`).
    BuildRet
    {
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Continues a bind: once the continuation is focused, focus the bound
    /// computation against the `μ̃binder` consumer.
    BuildBind
    {
        /// The bound computation.
        bound: &'src Comp,
        /// The binder the bound value is received under.
        binder: &'src String,
    },
    /// Continues an unpack: once the body is focused, cut the package value
    /// against the `μ̃binder` consumer — the image of [`Self::BuildBind`] whose
    /// bound computation is the erased `ret v`, which no borrow can name.
    BuildUnpack
    {
        /// The package value, erased to its payload by the value case.
        scrut: &'src Value,
        /// The module variable the payload is received under.
        binder: &'src String,
    },
    /// Cuts the top producer result against a `force` destructor frame.
    BuildForce
    {
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Assembles a two-arm injection case consumer and cuts the scrutinee
    /// against it.
    BuildCase
    {
        /// The `Inl` arm binder.
        left_binder: &'src String,
        /// The `Inr` arm binder.
        right_binder: &'src String,
    },
    /// Assembles a nil/cons list-case consumer and cuts the scrutinee against
    /// it.
    BuildListCase
    {
        /// The cons head binder.
        head: &'src String,
        /// The cons tail binder.
        tail: &'src String,
    },
    /// Assembles a single-pair-arm split consumer and cuts the scrutinee
    /// against it.
    BuildSplit
    {
        /// The first-component binder.
        fst_binder: &'src String,
        /// The second-component binder.
        snd_binder: &'src String,
    },
    /// Assembles the identity-proof constructor `Here(w)` from the top producer
    /// result (ADR-76; the `Value::Here` intro).
    BuildHere,
    /// Assembles the identity eliminator's single-`Here`-arm case and cuts the
    /// scrutinee against it (ADR-76; the motive is runtime-erased).
    BuildWalk
    {
        /// The diagonal base binder `x` the witness is bound to.
        base_binder: &'src String,
    },
    /// Assembles a declared-data constructor `Data[i](w)` from the top producer
    /// result (ADR-80; the `Value::Ctor` intro), keyed by position `i`.
    BuildDataCtor
    {
        /// The constructor's position in the decl-table `ctors` list.
        tag: usize,
    },
    /// Assembles the declared-data eliminator's positional `Data` case and cuts
    /// the scrutinee against it (ADR-80). Arm `i` matches tag `i`.
    BuildDataCase
    {
        /// One payload binder per constructor, in `ctors` order.
        binders: Vec<&'src String>,
    },
    /// Cuts the top producer result against a record-projection destructor.
    BuildRecordProj
    {
        /// The projected label.
        label: &'src String,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Continues a lazy pair (`with`): once the first component is focused
    /// against its fresh covariable, focus the second likewise.
    BuildWithAfterFst
    {
        /// The second component.
        snd: &'src Comp,
        /// The first component's covariable.
        fst_covar: CoName,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Assembles a lazy pair as a two-`Prj`-arm cocase and cuts it negatively
    /// against the continuation.
    BuildWith
    {
        /// The first component's command.
        fst_cmd: CommandId,
        /// The first component's covariable.
        fst_covar: CoName,
        /// The second component's covariable.
        snd_covar: CoName,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Emits a primitive command (`dup` / `drop`) over the top producer
    /// result, recording its provenance.
    BuildPrim
    {
        /// The primitive operation.
        op: PrimOp,
        /// The continuation consumer.
        cont: ConsumerId,
        /// The primitive command's provenance.
        origin: FocusOrigin,
    },
    /// Cuts an operation constructor (the `perform` payload) against the
    /// continuation.
    BuildPerform
    {
        /// The complete source effect signature.
        sig: &'src EffectSig,
        /// The operation name.
        op: &'src String,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Assembles a deep-handler consumer from the focused clause bodies, then
    /// delimits the scrutinee with it.
    BuildHandle
    {
        /// The effect signature name.
        sig: EffectSignatureName<'src>,
        /// The handled scrutinee.
        scrutinee: &'src Comp,
        /// The return-clause binder.
        ret_binder: &'src String,
        /// The operation clauses.
        ops: &'src [OpClause],
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Continues a `resume`: once the stack value is focused, focus the
    /// computation against a fresh covariable.
    BuildResumeAfterStack
    {
        /// The computation to reify.
        computation: &'src Comp,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Assembles a `resume` cut: the reified computation thunk feeds the
    /// stack's `resume` destructor.
    BuildResume
    {
        /// The reified stack producer.
        stack: ProducerId,
        /// The fresh covariable the computation was cut against.
        cobinder: CoName,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Cuts a `shift` capture producer against the continuation.
    BuildShift
    {
        /// The captured-continuation binder.
        binder: &'src String,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Emits a saturated native-primitive command from the top `len` producer
    /// results.
    BuildNative
    {
        /// The native primitive.
        prim: gandr_core_checker::prim::NativePrim,
        /// The number of argument producers on the result stack.
        len: usize,
        /// The continuation consumer.
        cont: ConsumerId,
    },
    /// Cuts the delimited body's `μ` against the delimiter, recording the
    /// delimiter command's provenance.
    BuildDelimiter
    {
        /// The delimiter consumer (prompt / handler).
        delimiter: ConsumerId,
        /// The delimiter command's provenance.
        origin: FocusOrigin,
    },
    /// Assembles an `ap` destructor from the top producer and consumer results
    /// (a reified `Arg` stack link).
    BuildStackArg,
    /// Continues a reified `Bind` stack link: once the rest is focused, focus
    /// the link's continuation against it.
    BuildStackBindAfterRest
    {
        /// The link's binder.
        binder: &'src String,
        /// The link's continuation.
        continuation: &'src Comp,
    },
    /// Assembles a `μ̃binder` consumer from the top command result (the head of
    /// a reified `Bind` stack link).
    BuildStackBind
    {
        /// The link's binder.
        binder: &'src String,
    },
    /// Assembles a projection destructor from the top consumer result (a
    /// reified `Prj` stack link).
    BuildStackPrj
    {
        /// The projection side.
        side: Side,
    },
}

/// The focused image of one finished task: the id of the arena node it
/// produced. The task machine's accounting nets exactly one result per task,
/// so a build frame pops exactly the images of its children.
enum FocusResult
{
    /// A focused value (the `𝓥` image).
    Producer(ProducerId),
    /// A focused consumer or reified stack (the `𝓒` image).
    Consumer(ConsumerId),
    /// A focused computation cut against its continuation (the `𝓕` image).
    Command(CommandId),
}

/// Pops a consumer id off the result stack.
///
/// # Contract
/// - requires: the task machine pushed a [`FocusResult::Consumer`] for the
///   current build frame.
/// - ensures: returns that consumer.
/// - panics: none in release builds. The machine's task/result accounting keeps
///   the stack sorted to the build frame; a `debug_assert!` guards that
///   invariant in test / debug builds, and a desync falls back to the
///   dangling-id sentinel, which surfaces downstream as a structured
///   dangling-id error (never reached — the differential oracle would fail
///   first).
fn pop_consumer(results: &mut Vec<FocusResult>) -> ConsumerId
{
    let result = results.pop();
    debug_assert!(
        matches!(result, Some(FocusResult::Consumer(_))),
        "consumer result is present and consumer-sorted"
    );
    match result {
        | Some(FocusResult::Consumer(id)) => id,
        | _ => ConsumerId::new(u32::MAX),
    }
}

/// Drains `len` producer ids off the result stack, preserving source order.
///
/// # Contract
/// - requires: the `len` most recent results are producer-sorted (see
///   [`pop_producer`]).
/// - ensures: returns them oldest-first (source order).
/// - panics: see [`pop_producer`].
fn pop_producers(
    results: &mut Vec<FocusResult>,
    len: FocusResultCount,
) -> Vec<ProducerId>
{
    let len = usize::from(len);
    let mut values = Vec::with_capacity(len);
    for _ in 0 .. len {
        values.push(pop_producer(results));
    }
    values.reverse();
    values
}

/// Pops a producer id off the result stack.
///
/// # Contract
/// - requires: the task machine pushed a [`FocusResult::Producer`] for the
///   current build frame.
/// - ensures: returns that producer.
/// - panics: none in release builds (a `debug_assert!` guards the result-stack
///   sort discipline; a desync falls back to the dangling-id sentinel — see
///   [`pop_consumer`]).
fn pop_producer(results: &mut Vec<FocusResult>) -> ProducerId
{
    let result = results.pop();
    debug_assert!(
        matches!(result, Some(FocusResult::Producer(_))),
        "producer result is present and producer-sorted"
    );
    match result {
        | Some(FocusResult::Producer(id)) => id,
        | _ => ProducerId::new(u32::MAX),
    }
}

/// Drains `len` command ids off the result stack, preserving source order.
///
/// # Contract
/// - requires: the `len` most recent results are command-sorted (see
///   [`pop_command`]).
/// - ensures: returns them oldest-first (source order).
/// - panics: see [`pop_command`].
fn pop_commands(
    results: &mut Vec<FocusResult>,
    len: FocusResultCount,
) -> Vec<CommandId>
{
    let len = usize::from(len);
    let mut values = Vec::with_capacity(len);
    for _ in 0 .. len {
        values.push(pop_command(results));
    }
    values.reverse();
    values
}

/// Pops a command id off the result stack.
///
/// # Contract
/// - requires: the task machine pushed a [`FocusResult::Command`] for the
///   current build frame.
/// - ensures: returns that command.
/// - panics: none in release builds (a `debug_assert!` guards the result-stack
///   sort discipline; a desync falls back to the dangling-id sentinel — see
///   [`pop_consumer`]).
fn pop_command(results: &mut Vec<FocusResult>) -> CommandId
{
    let result = results.pop();
    debug_assert!(
        matches!(result, Some(FocusResult::Command(_))),
        "command result is present and command-sorted"
    );
    match result {
        | Some(FocusResult::Command(id)) => id,
        | _ => CommandId::new(u32::MAX),
    }
}

/// Focuses a lowered top-level term — a value or a computation — against `★`.
///
/// This is the entry the phase-L0 totality gate drives over every corpus
/// program: a [`Term::Value`] focuses as `⟨𝓥⟦v⟧ | ★⟩`, a [`Term::Comp`] as
/// `𝓕⟦t⟧ ★`.
///
/// # Contract
/// - ensures: total on well-formed lowered terms.
///
/// # Errors
/// [`FocusError::ArenaFull`] only on arena exhaustion.
#[inline]
pub fn focus_term(term: &Term) -> Result<Focused, FocusError>
{
    match *term {
        | Term::Value(ref value) => focus_value(value),
        | Term::Comp(ref comp) => focus_comp(comp),
    }
}

/// Focuses a checked-core value against `★` (`⟨𝓥⟦v⟧ | ★⟩`).
///
/// A value mentioning a known-but-not-yet-realized former declines whole, as
/// [`focus_comp`]; currently no such former exists (the ADR-76/80 formers are
/// realized), so the scan is the forward-compat guard, not an active decline.
///
/// # Contract
/// - ensures: total on well-formed core values; the root is a positive cut of
///   the value's producer against `★` (a future unsupported former would yield
///   the Unsupported decline command).
///
/// # Errors
/// [`FocusError::ArenaFull`] only on arena exhaustion.
#[inline]
pub fn focus_value(value: &Value) -> Result<Focused, FocusError>
{
    if bool::from(value_mentions_unsupported_former(value)) {
        return unsupported_program();
    }
    let mut focuser = Focuser::new();
    let top = focuser.new_consumer(ConsumerNode::Top)?;
    let producer = focuser.focus_value(value)?;
    let root = focuser.cut(Polarity::Positive, producer, top, FocusOrigin::TopValue)?;
    Ok(finalize(focuser, root))
}

/// One node on the unsupported-former scan worklist (see
/// [`comp_mentions_unsupported_former`]).
enum ScanNode<'src>
{
    /// A computation to scan.
    Comp(&'src Comp),
    /// A value to scan.
    Value(&'src Value),
    /// A reified stack to scan.
    Stack(&'src Stack),
}

/// Whether a value mentions a known-but-not-yet-realized former (see
/// [`comp_mentions_unsupported_former`]).
fn value_mentions_unsupported_former(value: &Value) -> UnsupportedFormerStatus
{
    unsupported_former_scan(alloc::vec![ScanNode::Value(value)])
}

/// Focuses a checked-core computation against the terminal consumer `★`
/// (`𝓕⟦t⟧ ★`).
///
/// A computation containing a known-but-not-yet-realized former focuses as one
/// whole-program [`FocusOrigin::Unsupported`] decline — the cross-lane rule
/// preferring a whole-program decline over a partial per-node fallback (a hole
/// producer) that would silently disagree with the oracle on realized
/// items. The ADR-76 identity formers and ADR-80 declared data that once
/// declined here are now realized, so no former triggers the decline —
/// the scan is the forward-compat guard for a future former.
///
/// # Contract
/// - ensures: on success, `focused.root()` is a well-formed focused command
///   with no free covariables (`★` is not a covariable and every mint is
///   bound); the result is total on well-formed core computations (a future
///   unsupported former would yield the Unsupported decline command).
///
/// # Errors
/// [`FocusError::ArenaFull`] only if a node arena exhausts its `u32` id space.
#[inline]
pub fn focus_comp(comp: &Comp) -> Result<Focused, FocusError>
{
    if bool::from(comp_mentions_unsupported_former(comp)) {
        return unsupported_program();
    }
    let mut focuser = Focuser::new();
    let top = focuser.new_consumer(ConsumerNode::Top)?;
    let root = focuser.focus_comp(comp, top)?;
    Ok(finalize(focuser, root))
}

/// A completed focusing paired with its prelude resolution table (ADR-42).
///
/// The [`focus_comp_with_prelude`] result: the focused program plus a
/// `name → (cobinder, body)` table for the prelude names, each thunk body
/// focused into the **same** arena, so its command id is valid on the L
/// machine that owns the arena. A force-position free name resolves against
/// this table, mirroring the retired CEK oracle's `Force(Var …)` prelude
/// lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreludeFocused
{
    /// The focused program (arena, root, provenance).
    focused: Focused,
    /// The prelude resolution table: a name resolves to the fresh cobinder its
    /// thunk body computes against and the body command, both arena-resident.
    prelude: BTreeMap<String, (CoName, CommandId)>,
}

impl PreludeFocused
{
    /// The focused program.
    #[inline]
    #[must_use]
    pub fn focused(&self) -> &Focused
    {
        &self.focused
    }

    /// Consumes into the focused program and its prelude resolution table.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (Focused, BTreeMap<String, (CoName, CommandId)>)
    {
        (self.focused, self.prelude)
    }
}

/// Focuses `comp` together with a prelude binding-environment (ADR-42).
///
/// Each thunk-valued prelude binding's body is focused into the **same** arena,
/// so a force-position free name resolves against it exactly as the retired
/// CEK oracle's `Force(Var …)` consulted its prelude table.
///
/// Later bindings shadow earlier ones (the CEK's reverse lookup); a binding
/// whose winning value is not a thunk does not resolve (a force miss stays
/// [`gandr_core_checker::outcome::StuckReason::ForcedNonThunk`], as the CEK).
/// The empty prelude reproduces [`focus_comp`] exactly — an empty table, so
/// every force-position free name still halts at `ForcedNonThunk`.
///
/// # Errors
/// [`FocusError::ArenaFull`] only on arena exhaustion.
#[inline]
pub fn focus_comp_with_prelude(
    comp: &Comp,
    bindings: &[(String, Value)],
) -> Result<PreludeFocused, FocusError>
{
    if bool::from(comp_mentions_unsupported_former(comp)) {
        return Ok(PreludeFocused {
            focused: unsupported_program()?,
            prelude: BTreeMap::new(),
        });
    }
    let mut focuser = Focuser::new();
    let top = focuser.new_consumer(ConsumerNode::Top)?;
    let root = focuser.focus_comp(comp, top)?;
    // Resolve each name to its LAST binding (later shadows earlier — the CEK's
    // reverse lookup); only a thunk-valued winner focuses into the table, so a
    // non-thunk winner leaves a force-position miss at `ForcedNonThunk`.
    let mut winners: BTreeMap<&str, &Value> = BTreeMap::new();
    for binding in bindings {
        let (ref name, ref value) = *binding;
        let _previous = winners.insert(name.as_str(), value);
    }
    let mut prelude: BTreeMap<String, (CoName, CommandId)> = BTreeMap::new();
    for (name, value) in winners {
        if let Value::Thunk(_, ref body) = *value {
            let cobinder = focuser.fresh_covar();
            let cobinder_cons = focuser.new_consumer(ConsumerNode::CoVar(cobinder.clone()))?;
            let body_cmd = focuser.focus_comp(body.as_ref(), cobinder_cons)?;
            let _previous = prelude.insert(String::from(name), (cobinder, body_cmd));
        }
    }
    Ok(PreludeFocused {
        focused: finalize(focuser, root),
        prelude,
    })
}

/// Whether a computation mentions a known-but-not-yet-realized former anywhere
/// — the whole-program decline predicate of [`focus_comp`] / [`focus_value`].
/// The ADR-76 identity formers and ADR-80 declared data are realized, so the
/// scan traverses THROUGH them and currently declines on nothing; it is the
/// forward-compat guard for a future former. Iterative (an explicit worklist,
/// ADR-47): depth follows the heap, not the host stack.
fn comp_mentions_unsupported_former(comp: &Comp) -> UnsupportedFormerStatus
{
    unsupported_former_scan(alloc::vec![ScanNode::Comp(comp)])
}

/// Builds the whole-program decline command — an opaque hole producer cut
/// against `★` under [`FocusOrigin::Unsupported`], which the L machine runs to
/// its `UnsupportedByReference` sentinel (the not-yet-built surface).
fn unsupported_program() -> Result<Focused, FocusError>
{
    let mut focuser = Focuser::new();
    let top = focuser.new_consumer(ConsumerNode::Top)?;
    let producer = focuser.new_producer(ProducerNode::Lit(Lit::Hole(0)))?;
    let root = focuser.cut(Polarity::Positive, producer, top, FocusOrigin::Unsupported)?;
    Ok(finalize(focuser, root))
}

/// Drains the unsupported-former scan worklist: `true` iff any reachable node
/// is a known-but-not-yet-realized former. The ADR-76 identity formers and
/// ADR-80 declared data are realized, so it traverses THROUGH them and (with no
/// former currently unrealized) always returns `false`; a future former added
/// to the decline set would trip it.
fn unsupported_former_scan(mut work: Vec<ScanNode<'_>>) -> UnsupportedFormerStatus
{
    use ScanNode as Node;
    while let Some(node) = work.pop() {
        match node {
            | Node::Comp(comp_node) => match *comp_node {
                // The identity eliminator `Walk` (ADR-76) and the declared-data
                // eliminator `DataCase` (ADR-80) are both realized — scan
                // THROUGH them (scrutinee and arm/base bodies) so a not-yet-
                // realized former nested inside still triggers the whole-program
                // decline, rather than declining on the eliminator itself.
                | Comp::DataCase(ref scrut, ref arms) => {
                    work.push(Node::Value(scrut));
                    for arm in arms {
                        work.push(Node::Comp(&arm.1));
                    }
                },
                | Comp::Walk {
                    ref scrut,
                    ref base,
                    ..
                } => {
                    work.push(Node::Value(scrut));
                    work.push(Node::Comp(base.body.as_ref()));
                },
                | Comp::Abs(_, _, ref body)
                | Comp::Reset(ref body)
                | Comp::Shift(_, ref body)
                | Comp::Prj(_, ref body) => work.push(Node::Comp(body)),
                | Comp::App(ref head, ref arg) => {
                    work.push(Node::Comp(head));
                    work.push(Node::Value(arg));
                },
                | Comp::Ret(ref value)
                | Comp::Force(ref value)
                | Comp::Dup(ref value)
                | Comp::Drop(ref value)
                | Comp::Perform(_, _, ref value) => work.push(Node::Value(value)),
                | Comp::Bind(ref bound, _, ref body) => {
                    work.push(Node::Comp(bound));
                    work.push(Node::Comp(body));
                },
                | Comp::Case(ref scrut, (_, ref fst), (_, ref snd)) => {
                    work.push(Node::Value(scrut));
                    work.push(Node::Comp(fst));
                    work.push(Node::Comp(snd));
                },
                | Comp::ListCase {
                    ref scrut,
                    ref nil,
                    ref cons,
                    ..
                } => {
                    work.push(Node::Value(scrut));
                    work.push(Node::Comp(nil));
                    work.push(Node::Comp(cons));
                },
                | Comp::Split {
                    ref scrut,
                    ref body,
                    ..
                } => {
                    work.push(Node::Value(scrut));
                    work.push(Node::Comp(body));
                },
                | Comp::RecordProj { ref record, .. } => work.push(Node::Value(record)),
                | Comp::With(ref fst, ref snd) => {
                    work.push(Node::Comp(fst));
                    work.push(Node::Comp(snd));
                },
                | Comp::Handle {
                    ref scrutinee,
                    ret: (_, ref ret_body),
                    ref ops,
                    ..
                } => {
                    work.push(Node::Comp(scrutinee));
                    work.push(Node::Comp(ret_body));
                    for clause in ops {
                        work.push(Node::Comp(&clause.body));
                    }
                },
                | Comp::Resume(ref stack, ref fed) => {
                    work.push(Node::Value(stack));
                    work.push(Node::Comp(fed));
                },
                | Comp::Native { ref args, .. } => {
                    for arg in args {
                        work.push(Node::Value(arg));
                    }
                },
                // A hole is a leaf; a future former this predicate does not
                // know is NOT an unsupported former (the focusing fallback arms
                // own it — the whole-program decline is reserved for a former
                // known-but-not-yet-realized, of which there are currently none).
                | _ => {},
            },
            | Node::Value(value_node) => match *value_node {
                // The identity-proof value `Here` (ADR-76) and the declared-data
                // constructor value `Ctor` (ADR-80) are both realized — scan
                // THROUGH their witness / payload for a nested unsupported
                // former, rather than declining on the constructor itself.
                | Value::Ctor { ref payload, .. } => work.push(Node::Value(payload)),
                | Value::Here(ref witness) => work.push(Node::Value(witness)),
                | Value::Pair(ref fst, ref snd) => {
                    work.push(Node::Value(fst));
                    work.push(Node::Value(snd));
                },
                | Value::Inj(_, ref payload) | Value::Annot(ref payload, _) => {
                    work.push(Node::Value(payload));
                },
                | Value::List(ref elements) => {
                    for element in elements {
                        work.push(Node::Value(element));
                    }
                },
                | Value::Record(ref fields) => {
                    for field in fields.values() {
                        work.push(Node::Value(field));
                    }
                },
                | Value::Thunk(_, ref body) => work.push(Node::Comp(body)),
                | Value::Stk(ref stack) => work.push(Node::Stack(stack)),
                // Leaves (variables, literals, unit, holes), and any future
                // value former this predicate does not know — not identity.
                | _ => {},
            },
            | Node::Stack(stack_node) => match *stack_node {
                | Stack::Arg(ref value, ref rest) => {
                    work.push(Node::Value(value));
                    work.push(Node::Stack(rest));
                },
                | Stack::Bind(_, ref body, ref rest) => {
                    work.push(Node::Comp(body));
                    work.push(Node::Stack(rest));
                },
                | Stack::Prj(_, ref rest) => work.push(Node::Stack(rest)),
                // The empty stack, and any future frame this predicate does
                // not know — not identity.
                | _ => {},
            },
        }
    }
    false.into()
}

/// Consumes the state into a [`Focused`] and, in debug builds, asserts the
/// produced command is typed-IL well-formed — the §2.3 construction-time
/// debug-assertion face, so a `𝓕` bug that emits a dangling id, an out-of-scope
/// covariable, a non-value argument, a mis-arity frame, or a mislabeled cut
/// polarity trips at once rather than silently propagating.
fn finalize(
    focuser: Focuser,
    root: CommandId,
) -> Focused
{
    let built = focuser.finish(root);
    debug_assert!(
        bool::from(crate::check::is_wellformed(built.arena(), built.root())),
        "the focusing translation must produce a typed-IL well-formed command"
    );
    built
}
