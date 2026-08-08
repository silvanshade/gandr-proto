//! The command IL node families and their arena store
//! (`proposal-sequent-kernel.md` §2).
//!
//! Three syntactic categories — [`ProducerNode`], [`ConsumerNode`], and
//! [`CommandNode`] — realize the polarized System-L / λμμ̃ command IL. Every
//! node is arena-resident and refers to its children by typed
//! [`gandr_core_checker::syntax::NodeId`]s, reusing the ADR-50 carrier
//! substrate (`NodeArena`) that the frozen core's `ValueNode` / `CompNode` /
//! `StackNode` families already use, so this IL shares the workspace's arena
//! idioms rather than reintroducing `Rc`/`Box` recursion.
//!
//! # Correspondence to the spec grammar (§2.1)
//!
//! ```text
//! Producer p ::= x | lit | μα.s | K(p̄; c̄) | cocase { D(x̄; ᾱ) ⇒ s, … }
//! Consumer c ::= α | μ̃x.s | D(p̄; c̄) | case { K(x̄; ᾱ) ⇒ s, … } | ★
//! Command  s ::= ⟨p |ε c⟩ | prim(p̄; c̄) | f(p̄; c̄)
//! ```
//!
//! The nodes below track that grammar one-to-one, with the U-shift intro
//! ([`ProducerNode::Thunk`]) and the reified-consumer value
//! ([`ProducerNode::Boxed`]) split out from the general `cocase` because the
//! frozen core carries an explicit `thunk`/`force` U-shift and a reified-stack
//! value (`stk K`) that the bare §2.1 sketch folds into codata. The §6 effect
//! surface is realized by the dedicated [`ConsumerNode::Handler`] and
//! [`ConsumerNode::Prompt`] consumers, exactly as the §2.2 representation
//! sketch lists them. The `Closure { env, body }` producer of §2.2 is the
//! phase-K3 closures lane (§8) and is deliberately **not** present at
//! L0 — the focusing translation emits [`ProducerNode::Cocase`] (the
//! pre-closure-conversion form) instead, and closure conversion is a later
//! in-IL rewrite.
//!
//! # Arity is declared at the tag, never at the reader
//!
//! Three node families carry an argument pair `(p̄; c̄)` whose shape is fixed
//! by the head the node names: [`ProducerNode::Ctor`] by a [`CtorTag`],
//! [`ConsumerNode::Dtor`] by a [`DtorTag`], and [`CommandNode::Prim`] by a
//! [`PrimOp`]. Each of those three heads **declares** the counts its shape
//! admits — [`CtorTag::producer_arity`] and [`DtorTag::producer_arity`] for
//! `p̄`, and [`CtorTag::consumer_arity`], [`DtorTag::consumer_arity`] and
//! [`PrimOp::consumer_arity`] for `c̄` — and [`crate::check`] holds a node to
//! the declaration of its own head rather than to a constant of its own.
//!
//! The declarations are exhaustive matches with no wildcard arm, so a new tag
//! variant does not compile until it states its shape; and a tag emitting to
//! several destinations declares its own consumer count without the checker
//! changing. This is the intermediate-language half of the multi-output axis
//! (`docs/gandr/spec/implementation/circuit-terms.md`, the execution ladder's
//! `circuit-terms-rung-06`).
//!
//! [`CommandNode::Jump`] carries a `c̄` too, but its head is a definition
//! **name**, not a tag: its arity is fixed by the named top-level definition,
//! whose table the L0 command carrier does not retain, so no count is decidable
//! from the command alone and the checker records that rather than counting.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::PrimitiveArity;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::HoleId;
use gandr_core_checker::syntax::NodeArena;
use gandr_core_checker::syntax::NodeId;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::Side;

use crate::boundary::ConsumerArity;
use crate::boundary::SequentNodeCount;
/// A producer's stable arena id.
pub type ProducerId = NodeId<ProducerNode>;
/// A consumer's stable arena id.
pub type ConsumerId = NodeId<ConsumerNode>;
/// A command's stable arena id.
pub type CommandId = NodeId<CommandNode>;

/// A term-level variable name (a producer variable `x`).
pub type Name = String;
/// A covariable name (a consumer variable `α`).
///
/// A namespace disjoint from [`Name`] in practice: the frozen core has no
/// covariables, so every covariable is minted by the focusing translation with
/// a reserved `%`-prefixed spelling.
pub type CoName = String;

/// The evaluation-strategy orientation a cut carries
/// (`proposal-sequent-kernel.md` §5, decision K2).
///
/// The type of the meeting `A` fixes `ε`: a positive cut fires μ first
/// (call-by-value; data flows), a negative cut fires μ̃ first (call-by-name;
/// demand flows). L0 does not run the machine, so the tag is recorded (from the
/// producer's polarity) and checked for consistency, not yet acted on.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Polarity
{
    /// A positive cut `⟨p |+ c⟩`: the meeting type is a value type; μ fires
    /// first.
    Positive,
    /// A negative cut `⟨p |− c⟩`: the meeting type is a computation type; μ̃
    /// fires first.
    Negative,
}

/// A positive scalar leaf — the `lit` production of §2.1, plus the opaque
/// unit / hole axioms the frozen core treats as literal-like values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Lit
{
    /// The unit value `()`.
    Unit,
    /// An integer literal `n` (the frozen `Value::Int`).
    Int(i64),
    /// A string literal `"s"`.
    Str(String),
    /// A typed numeric literal (`u32` … `f64`; the `Value::Num` payload).
    Num(NumLit),
    /// A typed hole `?u`: an opaque axiom of unknown type, carried through the
    /// IL as a literal-like leaf so diagnostics can address it by
    /// [`HoleId`].
    Hole(HoleId),
}

/// A positive constructor tag `K` (`proposal-sequent-kernel.md` §2.1, positive
/// intro).
///
/// Enumerates the frozen core's positive data formers. Lists appear as the
/// inductive `Nil` / `Cons` view (Adaptation A-LIST) rather than the core's
/// flat vector, so the list eliminator's `case` arms match the intro directly;
/// records carry their label vector so a projecting consumer can name the
/// field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CtorTag
{
    /// An eager pair `Pair(a; b)` (the `Value::Pair` intro).
    Pair,
    /// A sum injection `Inl(v)` / `Inr(v)` (the `Value::Inj` intro).
    Inj(Side),
    /// The empty list `Nil` (Adaptation A-LIST: the cons-cell view of
    /// `Value::List`).
    Nil,
    /// A list cons cell `Cons(head; tail)` (Adaptation A-LIST).
    Cons,
    /// A record `Record{ℓ̄}(v̄)` with the labels in canonical order (Adaptation
    /// A-RECORD: the labeled positive intro of `Value::Record`).
    Record(Box<[String]>),
    /// An effect operation viewed as a positive constructor `Op[sig.op](v)`
    /// (Adaptation A-PERFORM; §6 — the operation flows to its handler as data).
    Op
    {
        /// The signature (effect) name the operation belongs to.
        sig: String,
        /// The operation's own name.
        op: String,
    },
    /// The identity-proof constructor `Here(w)` — the reflexivity witness of
    /// the identity type (ADR-76; the `Value::Here` intro). One producer
    /// argument (the witness `w`); the eliminator [`crate::machine`] fires
    /// Walk-β on it.
    Here,
    /// A **declared-data constructor** `Data[i](w)` (ADR-80; the `Value::Ctor`
    /// intro), keyed by the constructor's position `i` in the decl-table
    /// `ctors` list (the [`Side`] analogue for a `k`-constructor datatype). One
    /// producer argument (the field-tuple payload `w`). The nominal
    /// [`gandr_core_checker::types::DataId`] is erased by `𝓕`: the L machine
    /// selects a `DataCase` arm by position, exactly as the CEK's
    /// `arms.nth(tag)`, so the render-only id is never matched on.
    Data(usize),
}

impl CtorTag
{
    /// The number of producer arguments this constructor takes (`p̄`).
    ///
    /// # Contract
    /// - ensures: the arity is total and matches the well-formed shape the
    ///   focusing translation builds; the typed-IL checker asserts a `Ctor`'s
    ///   `ps.len()` equals this.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn producer_arity(&self) -> PrimitiveArity
    {
        match *self {
            | Self::Nil => 0_usize.into(),
            | Self::Inj(_) | Self::Op { .. } | Self::Here | Self::Data(_) => 1_usize.into(),
            | Self::Pair | Self::Cons => 2_usize.into(),
            | Self::Record(ref labels) => labels.len().into(),
        }
    }

    /// The number of consumer children this constructor takes (`c̄`).
    ///
    /// The §2.1 positive intro is written `K(p̄; c̄)`, so the consumer list is
    /// part of a constructor's declared shape rather than an afterthought.
    /// Every frozen-core constructor is data-only and declares **zero**
    /// consumers; the declaration lives here so that a tag emitting to several
    /// destinations states its own count and the checker needs no change.
    ///
    /// # Contract
    /// - ensures: total, and equal to the `cs` length the focusing translation
    ///   builds for this tag; the typed-IL checker holds a
    ///   [`ProducerNode::Ctor`]'s `cs.len()` to it exactly.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L0 plus L3 — the match is exhaustive with no wildcard, so
    ///   a new tag cannot compile without declaring a count; the pointwise
    ///   residue is the declared value itself, enumerated over the finite tag
    ///   set and asserted exactly, and separated from the checker's other
    ///   arities by the distinct [`ConsumerArity`] carrier.
    /// - witness: `il::tests::tag_consumer_arities_are_declared`
    /// - witness: `check::tests::constructor_consumer_arity_is_checked`
    #[inline]
    #[must_use]
    pub fn consumer_arity(&self) -> ConsumerArity
    {
        match *self {
            | Self::Pair
            | Self::Inj(_)
            | Self::Nil
            | Self::Cons
            | Self::Record(_)
            | Self::Op { .. }
            | Self::Here
            | Self::Data(_) => 0_usize.into(),
        }
    }
}

/// A destructor / copattern-head tag `D` (`proposal-sequent-kernel.md` §2.1,
/// negative elim and codata copatterns).
///
/// A destructor frame's trailing consumer is its return continuation (§2.1);
/// the count is declared by [`Self::consumer_arity`] rather than assumed by
/// the reader, and every frozen-core destructor declares one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DtorTag
{
    /// Function application `ap(v; α)` — the arrow destructor (§3, the `App`
    /// image); one producer argument.
    Ap,
    /// The U-shift `force(α)` — forcing a thunk (§3); no producer argument.
    Force,
    /// Lazy-product projection `prj1(α)` / `prj2(α)` (the `Comp::Prj` image);
    /// no producer argument.
    Prj(Side),
    /// Record field projection `.ℓ(α)` (Adaptation A-RECORDPROJ: the
    /// single-field `Comp::RecordProj` as a destructor frame); no producer
    /// argument.
    RecordProj(String),
    /// Stack resumption `resume(t; α)` (Adaptation A-RESUME: `Comp::Resume`
    /// feeds a reified computation to a reified stack); one producer
    /// argument (the reified computation).
    Resume,
}

impl DtorTag
{
    /// The number of producer arguments this destructor frame takes (`p̄`, not
    /// counting the trailing return continuation).
    ///
    /// # Contract
    /// - ensures: total; the typed-IL checker asserts a `Dtor`'s `ps.len()`
    ///   equals this, and holds its `cs.len()` to [`Self::consumer_arity`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn producer_arity(&self) -> PrimitiveArity
    {
        match *self {
            | Self::Force | Self::Prj(_) | Self::RecordProj(_) => 0_usize.into(),
            | Self::Ap | Self::Resume => 1_usize.into(),
        }
    }

    /// The number of consumer children this destructor frame takes (`c̄`).
    ///
    /// The trailing consumer of a destructor frame is its return continuation
    /// (§2.1), so every frozen-core destructor declares exactly one. The
    /// declaration replaces the checker-side constant the typed-IL walk used to
    /// carry, so a frame returning to several continuations states that at its
    /// own tag.
    ///
    /// # Contract
    /// - ensures: total, and equal to the `cs` length the focusing translation
    ///   builds for this tag; the typed-IL checker holds a
    ///   [`ConsumerNode::Dtor`]'s `cs.len()` to it exactly.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L0 plus L3 — the match is exhaustive with no wildcard, so
    ///   a new tag cannot compile without declaring a count; the pointwise
    ///   residue is the declared value itself, enumerated over the finite tag
    ///   set and asserted exactly.
    /// - witness: `il::tests::tag_consumer_arities_are_declared`
    /// - witness: `check::tests::destructor_consumer_arity_is_checked`
    #[inline]
    #[must_use]
    pub fn consumer_arity(&self) -> ConsumerArity
    {
        match *self {
            | Self::Ap | Self::Force | Self::Prj(_) | Self::RecordProj(_) | Self::Resume => {
                1_usize.into()
            },
        }
    }
}

/// A native / structural primitive named by a `prim` command
/// (`proposal-sequent-kernel.md` §2.1, §7.4 — natives are opaque at the cut
/// seam).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimOp
{
    /// A Rust-backed builtin (`Comp::Native`), dispatched in the core's `prim`
    /// registry once saturated.
    Native(NativePrim),
    /// The grade structural op `dup` (Adaptation A-DUP: `Comp::Dup`, a
    /// structural op with no reducible cut seam).
    Dup,
    /// The grade structural op `drop` (Adaptation A-DROP: `Comp::Drop`).
    Drop,
}

impl PrimOp
{
    /// The number of consumer children a `prim` command headed by this
    /// primitive takes (`c̄`, the return continuations).
    ///
    /// A `prim(p̄; c̄)` command carries its return continuations explicitly
    /// (§4.1, §7.4), and every primitive the focusing translation emits returns
    /// to exactly one — the continuation [`crate::machine`] `drive_prim` loads
    /// before dispatching. The count is declared here, so a primitive returning
    /// to several continuations states that at its own head.
    ///
    /// There is deliberately no companion producer-arity declaration: a
    /// [`Self::Native`] command's `p̄` is the argument prefix the source
    /// `Comp::Native` carried, which varies with partial application, so the
    /// producer count is **not** fixed by the head the way the consumer count
    /// is. [`gandr_core_checker::prim::NativePrim::arity`] is the builtin's
    /// saturation arity as a function, not this command's argument count.
    ///
    /// # Contract
    /// - ensures: total, and equal to the `cs` length the focusing translation
    ///   builds for this head; the typed-IL checker holds a
    ///   [`CommandNode::Prim`]'s `cs.len()` to it exactly.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L0 plus L3 — the match is exhaustive with no wildcard, so
    ///   a new primitive cannot compile without declaring a count; the
    ///   pointwise residue is the declared value itself, asserted exactly over
    ///   the finite head set and cross-checked against what `𝓕` builds.
    /// - witness: `il::tests::tag_consumer_arities_are_declared`
    /// - witness: `check::tests::prim_consumer_arity_is_checked`
    /// - witness: `check::tests::focused_prim_commands_meet_the_declaration`
    #[inline]
    #[must_use]
    pub fn consumer_arity(self) -> ConsumerArity
    {
        match self {
            | Self::Native(_) | Self::Dup | Self::Drop => 1_usize.into(),
        }
    }
}

/// One copattern arm `D(x̄; ᾱ) ⇒ s` of a [`ProducerNode::Cocase`] (negative
/// intro).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoArm
{
    /// The destructor this arm answers.
    pub dtor: DtorTag,
    /// The producer binders `x̄` the copattern introduces.
    pub binders: Box<[Name]>,
    /// The covariable binders `ᾱ` the copattern introduces (the return
    /// continuations the arm computes against).
    pub cobinders: Box<[CoName]>,
    /// The arm body command.
    pub body: CommandId,
}

/// One pattern-match arm `K(x̄; ᾱ) ⇒ s` of a [`ConsumerNode::Case`] (positive
/// elim).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arm
{
    /// The constructor this arm matches.
    pub ctor: CtorTag,
    /// The producer binders `x̄` the pattern introduces.
    pub binders: Box<[Name]>,
    /// The covariable binders `ᾱ` the pattern introduces.
    pub cobinders: Box<[CoName]>,
    /// The arm body command.
    pub body: CommandId,
}

/// One operation clause of a [`HandlerConsumer`] (`proposal-sequent-kernel.md`
/// §6): `op(p; k) ⇒ s`, binding the payload as a producer variable and the
/// resumption as a covalue-carrying value variable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerOp
{
    /// The handled operation's name.
    pub op: String,
    /// The payload binder `p` (a producer variable).
    pub payload: Name,
    /// The resumption binder `k` (the captured continuation, a reified-stack
    /// value bound as a producer variable).
    pub resume: Name,
    /// The clause body command.
    pub body: CommandId,
}

/// A deep effect handler as a consumer (`proposal-sequent-kernel.md` §6,
/// decision K4): a case analysis on operation constructors with a distinguished
/// return clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerConsumer
{
    /// The handled signature (effect) name `E`.
    pub sig: String,
    /// The return-clause binder `x` (bound to the handled computation's value).
    pub ret_binder: Name,
    /// The return-clause body command.
    pub ret_body: CommandId,
    /// The operation clauses, one per operation of the signature.
    pub ops: Vec<HandlerOp>,
    /// The ambient continuation below the handler. Return and operation clauses
    /// fall through to this consumer after the handler is discharged.
    pub continuation: ConsumerId,
}

/// A producer `p` (`proposal-sequent-kernel.md` §2.1 / §2.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProducerNode
{
    /// A variable `x`.
    Var(Name),
    /// A positive scalar leaf `lit`.
    Lit(Lit),
    /// A context capture `μα. s` binding the covariable `α` (present for
    /// grammar completeness and the §5 μ/μ̃ critical pair; the
    /// administrative-redex-avoiding focusing translation emits `μ̃`/direct
    /// cuts and never this node).
    Mu(CoName, CommandId),
    /// A positive constructor `K(p̄; c̄)`.
    Ctor
    {
        /// The constructor tag `K`.
        tag: CtorTag,
        /// The producer arguments `p̄`, counted by
        /// [`CtorTag::producer_arity`].
        ps: Box<[ProducerId]>,
        /// The consumer arguments `c̄`, counted by
        /// [`CtorTag::consumer_arity`] (zero for every frozen-core
        /// constructor).
        cs: Box<[ConsumerId]>,
    },
    /// A codata object `cocase { D(x̄; ᾱ) ⇒ s, … }` — negative intro (the `λ`
    /// and lazy-pair images).
    Cocase
    {
        /// The copattern arms.
        arms: Box<[CoArm]>,
    },
    /// A thunk `{ force(α) ⇒ s }` — the U-shift positive intro
    /// (`Value::Thunk`).
    ///
    /// The thunk carries the core's usage grade `U_r B` from
    /// [`gandr_core_checker::grade::Grade`] (retiring the former
    /// A-THUNK-GRADE adaptation): the grade rides the binder, exactly as the
    /// core's `Value::Thunk(Grade, _)` does. The grade→store-region *placement*
    /// remains the constant-`Heap` L1 default (`store::placement`; the
    /// placement refinement is a gated follow-up), so carrying the grade here
    /// freezes no placement — it only preserves the source annotation for
    /// the machine and diagnostics.
    Thunk
    {
        /// The usage grade `r` (the sealed grade surface, ADR-28).
        grade: Grade,
        /// The return-continuation binder `α` the forced body computes against.
        cobinder: CoName,
        /// The suspended body command.
        body: CommandId,
    },
    /// A reified consumer as a value `box(c)` — the `stk K` intro (Adaptation
    /// A-STK: a covalue crossing into value position).
    Boxed(ConsumerId),
    /// A delimited-control capture `shift k. s` — the μ-capture of the
    /// polarized reading (`proposal-sequent-kernel.md` §6: "shift captures
    /// the consumer up to it (μ-capture)"; Adaptation A-SHIFT).
    ///
    /// Distinct from the general context capture [`Self::Mu`]: a `shift`'s
    /// binder `k` is a *value* variable a later `resume k` consumes (the
    /// machine binds it a boxed captured continuation in the value
    /// environment), and the body `s` is focused against `★` so its
    /// returning tail falls through to the continuation below the removed
    /// prompt. The machine captures the continuation up to the enclosing
    /// prompt ([`ConsumerNode::Prompt`]) frame and runs `s` below the
    /// delimiter — the faithful delimited capture, see [`crate::machine`]
    /// `drive_shift`.
    Shift
    {
        /// The captured-continuation binder `k` (a value variable).
        binder: Name,
        /// The body command `s`, focused against `★`.
        body: CommandId,
    },
}

/// A consumer `c` (`proposal-sequent-kernel.md` §2.1 / §2.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsumerNode
{
    /// A covariable `α`.
    CoVar(CoName),
    /// A binder consumer `μ̃x. s` (the `Bind` / stack `Bind`-frame image).
    MuTilde(Name, CommandId),
    /// A destructor frame `D(p̄; c̄)`.
    Dtor
    {
        /// The destructor tag `D`.
        tag: DtorTag,
        /// The producer arguments `p̄`, counted by
        /// [`DtorTag::producer_arity`].
        ps: Box<[ProducerId]>,
        /// The consumer arguments `c̄` — the trailing return continuations,
        /// counted by [`DtorTag::consumer_arity`] (exactly one for every
        /// frozen-core destructor).
        cs: Box<[ConsumerId]>,
    },
    /// A pattern match `case { K(x̄; ᾱ) ⇒ s, … }` — positive elim.
    Case
    {
        /// The match arms.
        arms: Box<[Arm]>,
    },
    /// The terminal consumer `★` (top level).
    Top,
    /// A deep effect handler (§6) wrapping its ambient continuation.
    Handler(Box<HandlerConsumer>),
    /// A delimited-control prompt `prompt(c)` wrapping the delimited
    /// continuation (Adaptation A-RESET: `reset` installs a prompt; §6).
    Prompt(ConsumerId),
}

/// A command `s` (`proposal-sequent-kernel.md` §2.1 / §2.2) — a machine state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandNode
{
    /// A cut `⟨p |ε c⟩` carrying its polarity orientation `ε`.
    Cut
    {
        /// The cut's evaluation-strategy orientation.
        pol: Polarity,
        /// The producer half.
        producer: ProducerId,
        /// The consumer half.
        consumer: ConsumerId,
    },
    /// A native operation `prim(p̄; c̄)` with explicit return continuations
    /// (the opaque host seam; §4.1, §7.4).
    Prim
    {
        /// The primitive being invoked.
        op: PrimOp,
        /// The producer arguments `p̄`. Not fixed by `op`: a native's argument
        /// prefix varies with partial application ([`PrimOp::consumer_arity`]).
        ps: Box<[ProducerId]>,
        /// The consumer arguments `c̄` — the return continuations, counted by
        /// [`PrimOp::consumer_arity`].
        cs: Box<[ConsumerId]>,
    },
    /// A top-level jump `f(p̄; c̄)` — no call stack (ADR-47).
    ///
    /// Present for grammar completeness (§2.2); the L0 focusing translation
    /// targets a single top-level computation and emits no jumps, so this
    /// is the seam the L1 machine's top-level-definition jumps land on.
    ///
    /// Its head is a definition **name**, not a tag, so neither its `p̄` nor
    /// its `c̄` is tag-declared: both counts belong to the named definition's
    /// signature, and the L0 command carrier retains no definition table to
    /// resolve it against. The typed-IL checker therefore walks a jump's
    /// arguments without counting them.
    Jump
    {
        /// The jumped-to definition's name.
        def: Name,
        /// The producer arguments `p̄`, counted by the named definition rather
        /// than by a tag.
        ps: Box<[ProducerId]>,
        /// The consumer arguments `c̄`, counted by the named definition rather
        /// than by a tag.
        cs: Box<[ConsumerId]>,
    },
}

/// The arena store for the three command-IL node families
/// (`proposal-sequent-kernel.md` §2.2): one typed [`NodeArena`] per category,
/// the same flat carrier the frozen core's `FlatArena` uses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandArena
{
    /// The producer nodes.
    producers: NodeArena<ProducerNode>,
    /// The consumer nodes.
    consumers: NodeArena<ConsumerNode>,
    /// The command nodes.
    commands: NodeArena<CommandNode>,
}

impl CommandArena
{
    /// Creates an empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Allocates a producer node, returning its stable id.
    ///
    /// # Contract
    /// - ensures: `Some(id)` with `producer(id) == &node`; `None` only if the
    ///   producer arena has exhausted the `u32` id space.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn alloc_producer(
        &mut self,
        node: ProducerNode,
    ) -> Option<ProducerId>
    {
        self.producers.alloc(node)
    }

    /// Allocates a consumer node, returning its stable id.
    ///
    /// # Contract
    /// - ensures: `Some(id)` with `consumer(id) == &node`; `None` only on `u32`
    ///   exhaustion.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn alloc_consumer(
        &mut self,
        node: ConsumerNode,
    ) -> Option<ConsumerId>
    {
        self.consumers.alloc(node)
    }

    /// Allocates a command node, returning its stable id.
    ///
    /// # Contract
    /// - ensures: `Some(id)` with `command(id) == &node`; `None` only on `u32`
    ///   exhaustion.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn alloc_command(
        &mut self,
        node: CommandNode,
    ) -> Option<CommandId>
    {
        self.commands.alloc(node)
    }

    /// Looks a producer node up by id.
    #[inline]
    #[must_use]
    pub fn producer(
        &self,
        id: ProducerId,
    ) -> Option<&ProducerNode>
    {
        self.producers.get(id)
    }

    /// Looks a consumer node up by id.
    #[inline]
    #[must_use]
    pub fn consumer(
        &self,
        id: ConsumerId,
    ) -> Option<&ConsumerNode>
    {
        self.consumers.get(id)
    }

    /// Looks a command node up by id.
    #[inline]
    #[must_use]
    pub fn command(
        &self,
        id: CommandId,
    ) -> Option<&CommandNode>
    {
        self.commands.get(id)
    }

    /// The number of producer nodes allocated.
    #[inline]
    #[must_use]
    pub fn producer_count(&self) -> SequentNodeCount
    {
        usize::from(self.producers.len()).into()
    }

    /// The number of consumer nodes allocated.
    #[inline]
    #[must_use]
    pub fn consumer_count(&self) -> SequentNodeCount
    {
        usize::from(self.consumers.len()).into()
    }

    /// The number of command nodes allocated.
    #[inline]
    #[must_use]
    pub fn command_count(&self) -> SequentNodeCount
    {
        usize::from(self.commands.len()).into()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The constructor and destructor arities match the shapes the focusing
    /// translation and typed-IL checker rely on.
    #[test]
    fn tag_arities_are_stable()
    {
        assert_eq!(
            0,
            usize::from(CtorTag::Nil.producer_arity()),
            "Nil is nullary"
        );
        assert_eq!(
            1,
            usize::from(CtorTag::Inj(Side::Fst).producer_arity()),
            "an injection takes one payload"
        );
        assert_eq!(
            2,
            usize::from(CtorTag::Pair.producer_arity()),
            "a pair takes two"
        );
        assert_eq!(
            2,
            usize::from(CtorTag::Cons.producer_arity()),
            "a cons cell takes two"
        );
        assert_eq!(
            2,
            usize::from(
                CtorTag::Record(Box::from([String::from("a"), String::from("b")])).producer_arity()
            ),
            "a record's arity is its label count"
        );
        assert_eq!(
            0,
            usize::from(DtorTag::Force.producer_arity()),
            "force takes no argument"
        );
        assert_eq!(
            1,
            usize::from(DtorTag::Ap.producer_arity()),
            "ap takes one argument"
        );
        assert_eq!(
            1,
            usize::from(DtorTag::Resume.producer_arity()),
            "resume takes the reified computation"
        );
    }

    /// Every tag that heads a consumer-carrying node declares its consumer
    /// arity, and the finite tag set is enumerated exhaustively so the declared
    /// value is asserted per variant rather than sampled.
    #[test]
    fn tag_consumer_arities_are_declared()
    {
        let ctors = [
            CtorTag::Pair,
            CtorTag::Inj(Side::Fst),
            CtorTag::Inj(Side::Snd),
            CtorTag::Nil,
            CtorTag::Cons,
            CtorTag::Record(Box::from([String::from("a"), String::from("b")])),
            CtorTag::Op {
                sig: String::from("State"),
                op: String::from("get"),
            },
            CtorTag::Here,
            CtorTag::Data(0),
        ];
        for tag in &ctors {
            assert_eq!(
                0,
                usize::from(tag.consumer_arity()),
                "a frozen-core constructor is data-only: {tag:?}"
            );
        }

        let dtors = [
            DtorTag::Ap,
            DtorTag::Force,
            DtorTag::Prj(Side::Fst),
            DtorTag::Prj(Side::Snd),
            DtorTag::RecordProj(String::from("field")),
            DtorTag::Resume,
        ];
        for tag in &dtors {
            assert_eq!(
                1,
                usize::from(tag.consumer_arity()),
                "a destructor frame returns to one continuation: {tag:?}"
            );
        }

        let prims = [PrimOp::Dup, PrimOp::Drop, PrimOp::Native(NativePrim::Add)];
        for &op in &prims {
            assert_eq!(
                1,
                usize::from(op.consumer_arity()),
                "a prim command returns to one continuation: {op:?}"
            );
        }
    }

    /// The arena allocates each family independently and reads nodes back by
    /// id.
    #[test]
    fn arena_allocates_and_reads_back()
    {
        let mut arena = CommandArena::new();
        let producer = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("fresh arena has room");
        let consumer = arena
            .alloc_consumer(ConsumerNode::Top)
            .expect("fresh arena has room");
        let command = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer,
                consumer,
            })
            .expect("fresh arena has room");
        assert_eq!(
            Some(&ProducerNode::Lit(Lit::Unit)),
            arena.producer(producer),
            "the producer reads back"
        );
        assert_eq!(
            Some(&ConsumerNode::Top),
            arena.consumer(consumer),
            "the consumer reads back"
        );
        assert!(
            matches!(arena.command(command), Some(&CommandNode::Cut { .. })),
            "the command reads back"
        );
        assert_eq!(1, usize::from(arena.producer_count()), "one producer");
        assert_eq!(1, usize::from(arena.consumer_count()), "one consumer");
        assert_eq!(1, usize::from(arena.command_count()), "one command");
    }
}
