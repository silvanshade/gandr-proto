//! Terms of core CBPV (`spec:implementation/type-system.md` §"Terms").
//!
//! Values and computations are distinct sorts. Children are reference-counted
//! so that terms can be cloned cheaply into machine control states, frames,
//! and traces. Structural equality (`PartialEq`) compares through the
//! reference counting.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::marker::PhantomData;

use gandr_kernel_strata::Level;

use crate::boundary::ArenaEmptyStatus;
use crate::boundary::ArenaLength;
use crate::boundary::BinderName;
use crate::boundary::ConstructorTag;
use crate::boundary::ContinuationName;
use crate::boundary::F32Literal;
use crate::boundary::F64Literal;
use crate::boundary::FieldName;
use crate::boundary::I32Literal;
use crate::boundary::I64Literal;
use crate::boundary::IntegerLiteral;
use crate::boundary::NameRef;
use crate::boundary::NodeIndex;
use crate::boundary::OperationName;
use crate::boundary::ResumeName;
use crate::boundary::SourceIndex;
use crate::boundary::StringLiteral;
use crate::boundary::StringText;
use crate::boundary::U32Literal;
use crate::boundary::U64Literal;
use crate::classifier::SortExpr;
use crate::effect::EffectRow;
use crate::effect::EffectSig;
use crate::grade::Grade;
use crate::prim::NativePrim;
use crate::types::CompType;
use crate::types::DataId;
use crate::types::SealId;
use crate::types::ValueType;

/// A stable, typed identifier for a node in an arena-backed IR carrier.
///
/// The `Kind` parameter is a zero-sized type tag: it prevents mixing value,
/// computation, stack, and transitional carriers while the stored identity
/// stays the compact `u32` ADR-50 shape. The raw index is intentionally opaque
/// to callers; arena access goes through [`NodeArena::get`].
pub struct NodeId<Kind>
{
    /// Zero-based index into the typed arena table.
    index: u32,
    /// Invariant tag tying the raw index to one node kind.
    marker: PhantomData<fn() -> Kind>,
}

impl<Kind> Copy for NodeId<Kind>
{
}

impl<Kind> Clone for NodeId<Kind>
{
    #[inline]
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<Kind> core::fmt::Debug for NodeId<Kind>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.debug_tuple("NodeId").field(&self.index).finish()
    }
}

impl<Kind> Eq for NodeId<Kind>
{
}

impl<Kind> PartialEq for NodeId<Kind>
{
    #[inline]
    fn eq(
        &self,
        other: &Self,
    ) -> bool
    {
        self.index == other.index
    }
}

impl<Kind> core::hash::Hash for NodeId<Kind>
{
    #[inline]
    fn hash<H>(
        &self,
        state: &mut H,
    ) where
        H: core::hash::Hasher,
    {
        core::hash::Hash::hash(&self.index, state);
    }
}

impl<Kind> Ord for NodeId<Kind>
{
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> core::cmp::Ordering
    {
        self.index.cmp(&other.index)
    }
}

impl<Kind> PartialOrd for NodeId<Kind>
{
    #[inline]
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<core::cmp::Ordering>
    {
        Some(self.cmp(other))
    }
}

impl<Kind> NodeId<Kind>
{
    /// Builds a node id from its arena index.
    ///
    /// # Contract
    /// - ensures: preserves `index` exactly.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<I>(index: I) -> Self
    where
        I: Into<NodeIndex>,
    {
        let index = index.into();
        Self {
            index: u32::from(index),
            marker: PhantomData,
        }
    }

    /// Returns the raw arena index.
    ///
    /// # Contract
    /// - ensures: returns the index passed to [`Self::new`] or allocated by
    ///   [`NodeArena::alloc`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn index(self) -> NodeIndex
    {
        self.index.into()
    }
}

/// A flat, append-only arena for typed node payloads.
///
/// This is the ADR-50 carrier substrate: nodes live in one table and refer to
/// children by typed [`NodeId`]s. It deliberately exposes only checked lookup;
/// callers never index the backing vector directly.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NodeArena<Node>
{
    /// Stored node payloads indexed by raw [`NodeId`] indices.
    nodes: Vec<Node>,
}

impl<Node> Default for NodeArena<Node>
{
    /// Creates an empty arena without requiring a default node payload.
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl<Node> NodeArena<Node>
{
    /// Creates an empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self { nodes: Vec::new() }
    }

    /// Allocates `node` and returns its stable id.
    ///
    /// Returns `None` only if the arena has exhausted the `u32` id space; the
    /// caller can then keep using the legacy structural carrier rather than
    /// manufacturing an invalid id.
    #[inline]
    #[must_use]
    pub fn alloc(
        &mut self,
        node: Node,
    ) -> Option<NodeId<Node>>
    {
        let index = u32::try_from(self.nodes.len()).ok()?;
        self.nodes.push(node);
        Some(NodeId::new(NodeIndex::from(index)))
    }

    /// Looks up a node by id.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: NodeId<Node>,
    ) -> Option<&Node>
    {
        let index = usize::try_from(u32::from(id.index())).ok()?;
        self.nodes.get(index)
    }

    /// Number of allocated nodes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> ArenaLength
    {
        self.nodes.len().into()
    }

    /// Whether the arena is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> ArenaEmptyStatus
    {
        self.nodes.is_empty().into()
    }
}

/// Canonical value-node ids.
pub type ValueNodeId = NodeId<ValueNode>;
/// Canonical computation-node ids.
pub type CompNodeId = NodeId<CompNode>;
/// Canonical reified-stack node ids.
pub type StackNodeId = NodeId<StackNode>;
/// Canonical value-type node ids.
pub type ValueTypeNodeId = NodeId<ValueTypeNode>;
/// Canonical computation-type node ids.
pub type CompTypeNodeId = NodeId<CompTypeNode>;
/// Canonical value arena.
pub type ValueArena = NodeArena<ValueNode>;
/// Canonical computation arena.
pub type CompArena = NodeArena<CompNode>;
/// Canonical reified-stack arena.
pub type StackArena = NodeArena<StackNode>;
/// Canonical value-type arena.
pub type ValueTypeArena = NodeArena<ValueTypeNode>;
/// Canonical computation-type arena.
pub type CompTypeArena = NodeArena<CompTypeNode>;

/// A value type node in the flat ADR-50 carrier.
///
/// Recursive type children are ids into the type arenas. Atoms, labels, grades,
/// and effect rows stay owned scalar data so readback remains independent of
/// legacy `Rc` structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueTypeNode
{
    Atom(String),
    Unit,
    Prod(ValueTypeNodeId, ValueTypeNodeId),
    Sum(ValueTypeNodeId, ValueTypeNodeId),
    List(ValueTypeNodeId),
    Record(BTreeMap<String, ValueTypeNodeId>),
    Thunk(Grade, CompTypeNodeId),
    Stk(CompTypeNodeId, CompTypeNodeId),
    /// The identity type `Path A x y` (ADR-76): carrier plus two value
    /// endpoints, the flat mirror of [`crate::types::ValueType::Path`]
    /// (the endpoints are value-arena ids — terms in a type).
    Path
    {
        /// The carrier type id `A`.
        ty: ValueTypeNodeId,
        /// The left-endpoint value id `x`.
        lhs: ValueNodeId,
        /// The right-endpoint value id `y`.
        rhs: ValueNodeId,
    },
    /// The declared-data nominal handle `Data { id, args }` (ADR-80), the flat
    /// mirror of [`crate::types::ValueType::Data`].
    Data
    {
        /// The datatype's minted nominal identity.
        id: DataId,
        /// The type-argument ids `ā`.
        args: Vec<ValueTypeNodeId>,
    },
    /// The type-family application `head(args…)`, the flat mirror of
    /// [`crate::types::ValueType::Family`]. The head is an owned attribute; the
    /// arguments are **value**-arena ids, because a family is indexed by values
    /// where [`Self::Data`] is parameterized by types.
    Family
    {
        /// The family-kinded head's name.
        head: String,
        /// The argument value ids, in application order.
        args: Vec<ValueNodeId>,
    },
    /// A universe `Type[sort, level]`, the flat mirror of
    /// [`crate::types::ValueType::Universe`].
    ///
    /// A leaf with owned attributes, like [`Self::Family`]'s head: neither the
    /// sort nor the level is a type, so neither becomes an arena id.
    Universe
    {
        /// The family this universe collects.
        sort: SortExpr,
        /// The level this universe sits at.
        level: Level,
    },
    /// A sealed abstract type, the flat mirror of
    /// [`crate::types::ValueType::Sealed`].
    ///
    /// A leaf, like its structural counterpart: the identity is the whole node,
    /// because there is no representation recorded to give it a child.
    Sealed(SealId),
    /// The dependent pair `Σ(binder : fst). snd` (ADR-81), the flat mirror of
    /// [`crate::types::ValueType::Sigma`]. The `binder` is an owned
    /// attribute (a value-variable name); the head and tail are type-arena
    /// ids.
    Sigma
    {
        /// The head type id `A`.
        fst: ValueTypeNodeId,
        /// The bound value-variable name `x`, in scope in `snd`.
        binder: String,
        /// The dependent tail type id `B`.
        snd: ValueTypeNodeId,
    },
    /// The first-class module package `Package_grade ⟨abstracts⟩ payload`, the
    /// flat mirror of [`crate::types::ValueType::Package`].
    ///
    /// The binder labels are owned attributes (type-variable names, discharged
    /// by `gandr_core_checker::judgements::package::instantiate`); the payload
    /// is a type-arena id.
    Package
    {
        /// The usage grade `r` — how many times the package may be unpacked.
        grade: Grade,
        /// The abstract type component labels, in signature order.
        abstracts: Vec<String>,
        /// The payload type id, in whose scope every label is bound.
        payload: ValueTypeNodeId,
    },
    Unknown,
}

/// A computation type node in the flat ADR-50 carrier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompTypeNode
{
    F(ValueTypeNodeId, EffectRow),
    /// The function type — `Π(binder : arg). res` when `binder` is `Some`, and
    /// the non-dependent `arg → res` when it is `None` — the flat mirror of
    /// [`crate::types::CompType::Arrow`]. The `binder` is an owned attribute (a
    /// value-variable name); the domain and codomain are type-arena ids.
    Arrow
    {
        /// The bound value-variable name `x`, in scope in `res`, or `None` for
        /// the non-dependent arrow.
        binder: Option<String>,
        /// The argument value-type id `A`.
        arg: ValueTypeNodeId,
        /// The result computation-type id `B`, which may mention `binder`.
        res: CompTypeNodeId,
    },
    With(CompTypeNodeId, CompTypeNodeId),
    Unknown,
}

/// A value node in the flat ADR-50 carrier.
///
/// Children are ids, never owned recursive nodes. This substrate is
/// intentionally parallel to [`Value`] while existing public builders continue
/// to return the legacy structural surface during migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueNode
{
    Var(String),
    Unit,
    Int(i64),
    Str(String),
    Num(NumLit),
    Pair(ValueNodeId, ValueNodeId),
    Inj(Side, ValueNodeId),
    List(Vec<ValueNodeId>),
    Record(BTreeMap<String, ValueNodeId>),
    Thunk(Grade, CompNodeId),
    Annot(ValueNodeId, ValueTypeNodeId),
    Hole(HoleId),
    Stk(StackNodeId),
    /// A reflexivity proof `here(v)` (ADR-76), the flat mirror of
    /// [`crate::syntax::Value::Here`].
    Here(ValueNodeId),
    /// A declared-data constructor value `Ctor { id, tag, payload }` (ADR-80),
    /// the flat mirror of [`crate::syntax::Value::Ctor`].
    Ctor
    {
        /// The datatype's minted nominal identity.
        id: DataId,
        /// The constructor's position in the decl-table `ctors` list.
        tag: usize,
        /// The field-tuple payload id.
        payload: ValueNodeId,
    },
    /// A packed module `pack ⟨witnesses⟩ payload`, the flat mirror of
    /// [`crate::syntax::Value::Pack`].
    ///
    /// The witnesses are type-arena ids — the abstraction's own half of the
    /// both-directions annotation — and the payload is a value-arena id.
    Pack
    {
        /// The witness type ids, positionally matching the signature's binders.
        witnesses: Vec<ValueTypeNodeId>,
        /// The packed payload value id.
        payload: ValueNodeId,
    },
    /// The pure-computation embedding `run t`, the flat mirror of
    /// [`crate::syntax::Value::Run`]: the embedded computation is a
    /// computation-arena id.
    Run(CompNodeId),
}

/// A handler operation clause in the flat computation carrier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpClauseNode
{
    /// Operation name handled by this clause.
    pub op: String,
    /// Payload binder introduced while checking/evaluating the clause.
    pub payload: String,
    /// Resume binder introduced while checking/evaluating the clause.
    pub resume: String,
    /// Clause body computation root.
    pub body: CompNodeId,
}

/// The flat mirror of [`WalkMotive`] in the ADR-50 carrier: the identity
/// eliminator's motive `(x y q). C`, with the body a computation-type-arena id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkMotiveNode
{
    /// The left-endpoint binder `x`.
    pub x: String,
    /// The right-endpoint binder `y`.
    pub y: String,
    /// The path binder `q`.
    pub q: String,
    /// The motive body id `C(x, y, q)`.
    pub body: CompTypeNodeId,
}

/// The flat mirror of [`WalkBase`] in the ADR-50 carrier: the identity
/// eliminator's diagonal base `(x). c`, with the body a computation-arena id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkBaseNode
{
    /// The diagonal binder `x`.
    pub x: String,
    /// The base body id `c(x)`.
    pub body: CompNodeId,
}

/// The flat mirror of [`SplitMotive`] in the ADR-50 carrier.
///
/// The product / dependent-pair eliminator's motive `(z). M`, with the body a
/// computation-type-arena id (the [`WalkMotiveNode`] precedent; ADR-82 D1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitMotiveNode
{
    /// The scrutinee binder `z`.
    pub binder: String,
    /// The motive body id `M(z)`.
    pub body: CompTypeNodeId,
}

/// A computation node in the flat ADR-50 carrier.
///
/// All term children are typed ids. Binder names, annotations, signatures, and
/// primitive tags remain attributes on the node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompNode
{
    Abs(String, Option<ValueTypeNodeId>, CompNodeId),
    App(CompNodeId, ValueNodeId),
    Ret(ValueNodeId),
    Bind(CompNodeId, String, CompNodeId),
    Force(ValueNodeId),
    Case(ValueNodeId, (String, CompNodeId), (String, CompNodeId)),
    ListCase
    {
        scrut: ValueNodeId,
        nil: CompNodeId,
        head: String,
        tail: String,
        cons: CompNodeId,
    },
    /// The product / dependent-pair eliminator `split v as (p, q) [z. M] in t`
    /// (ADR-82), the flat mirror of [`crate::syntax::Comp::Split`]: the
    /// optional motive `(z). M` is a computation-type child
    /// ([`SplitMotiveNode`]), not a term child.
    Split
    {
        /// The scrutinee value id.
        scrut: ValueNodeId,
        /// The binder for the first component `p`.
        fst_name: String,
        /// The binder for the second component `q`.
        snd_name: String,
        /// The optional dependent motive `(z). M` (ADR-82); `None` is the
        /// check-only motive-less split.
        motive: Option<SplitMotiveNode>,
        /// The body computation id `t`.
        body: CompNodeId,
    },
    /// The declared-data eliminator `DataCase { scrut, arms }` (ADR-80), the
    /// flat mirror of [`crate::syntax::Comp::DataCase`]: each arm is a
    /// `(binder, body)`, arm `i` handling constructor tag `i`.
    DataCase
    {
        scrut: ValueNodeId,
        arms: Vec<(String, CompNodeId)>,
    },
    RecordProj
    {
        record: ValueNodeId,
        label: String,
    },
    With(CompNodeId, CompNodeId),
    Prj(Side, CompNodeId),
    Dup(ValueNodeId),
    Drop(ValueNodeId),
    Perform(Box<EffectSig>, String, ValueNodeId),
    Handle
    {
        sig: Box<EffectSig>,
        scrutinee: CompNodeId,
        ret: (String, CompNodeId),
        ops: Vec<OpClauseNode>,
    },
    Resume(ValueNodeId, CompNodeId),
    Reset(CompNodeId),
    Shift(String, CompNodeId),
    Hole(HoleId),
    Native
    {
        prim: NativePrim,
        args: Vec<ValueNodeId>,
    },
    /// The identity eliminator `walk(p, motive, base)` (ADR-76), the flat
    /// mirror of [`crate::syntax::Comp::Walk`].
    Walk
    {
        /// The scrutinee value id `p`.
        scrut: ValueNodeId,
        /// The motive `(x y q). C`.
        motive: WalkMotiveNode,
        /// The diagonal base `(x). c`.
        base: WalkBaseNode,
    },
    /// The package eliminator `unpack v : σ as ⟨atoms⟩ binder in t`, the flat
    /// mirror of [`crate::syntax::Comp::Unpack`].
    ///
    /// The ascribed signature is a type-arena id — the elimination's own half
    /// of the both-directions annotation — and the minted atoms are owned
    /// attributes, exactly as they are in the structural form.
    Unpack
    {
        /// The package value id.
        scrut: ValueNodeId,
        /// The ascribed package type id.
        signature: ValueTypeNodeId,
        /// The atoms minted for this elimination, in signature order.
        atoms: Vec<SealId>,
        /// The module variable bound over the body.
        binder: String,
        /// The body computation id.
        body: CompNodeId,
    },
    /// The recursion former `fix x. t`, the flat mirror of
    /// [`crate::syntax::Comp::Fix`]: the self-reference binder is an owned
    /// attribute and the body is a computation-arena id.
    Fix(String, CompNodeId),
}

/// A reified-stack node in the flat ADR-50 carrier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StackNode
{
    Empty,
    Arg(ValueNodeId, StackNodeId),
    Bind(String, CompNodeId, StackNodeId),
    Prj(Side, StackNodeId),
}

/// An opaque hole identifier (A2.2 holes extension; `A2-PLAN.md` D5).
///
/// The degenerate image of Hazelnut's hole *name* `u`: typing ignores it
/// entirely (two holes with different identifiers type identically), but
/// consumers — the pipeline's goals report, and later the A2.4 agent stream —
/// key on it to address individual holes. Identifiers are minted by whoever
/// constructs the term (the pipeline's lowerer mints them sequentially) and
/// carry no uniqueness obligation inside this crate.
pub type HoleId = u32;

/// Selects a component of a binary form: an injection tag (`inj1`/`inj2`) or
/// a projection index (`prj1`/`prj2`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side
{
    /// The first component (`inj1` / `prj1`).
    Fst,
    /// The second component (`inj2` / `prj2`).
    Snd,
}

/// The payload of a typed numeric literal ([`Value::Num`]; the value-model
/// ladder's numeric primitive rung, ADR-39).
///
/// One variant per primitive numeric atom (`u32`/`u64`/`i32`/`i64`/`f32`/`f64`,
/// the Rust-spelled first-pass scalar set). The integer variants carry the
/// native Rust integer; the float variants carry the IEEE-754 **bit pattern**
/// ([`f32::to_bits`] / [`f64::to_bits`]) rather than the float itself, so the
/// enclosing [`Value`] keeps its derived [`Eq`] and structural equality stays
/// reflexive and total — `f32` / `f64` are not [`Eq`] (`NaN` breaks
/// reflexivity). Literal equality is therefore *bitwise*: reflexive including
/// `NaN`, with `+0.0` and `-0.0` distinct — the right notion for a syntactic
/// literal, and v0 performs no float arithmetic where the IEEE total-vs-numeric
/// order would otherwise matter. Build a float variant from a value with
/// [`NumLit::f32`] / [`NumLit::f64`]; read its rigid atom with
/// [`NumLit::value_type`] (the single point of truth, keeping checker / machine
/// / mark lock-step).
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum NumLit
{
    /// A `u32` literal.
    U32(u32),
    /// A `u64` literal.
    U64(u64),
    /// An `i32` literal.
    I32(i32),
    /// An `i64` literal.
    I64(i64),
    /// An `f32` literal, stored as its IEEE-754 bit pattern ([`f32::to_bits`]).
    F32(u32),
    /// An `f64` literal, stored as its IEEE-754 bit pattern ([`f64::to_bits`]).
    F64(u64),
}

impl NumLit
{
    /// Builds an `f32` literal from a value, storing its bit pattern.
    #[inline]
    #[must_use]
    pub fn f32<L>(value: L) -> Self
    where
        L: Into<F32Literal>,
    {
        let value = value.into();
        Self::F32(f32::from(value).to_bits())
    }

    /// Builds an `f64` literal from a value, storing its bit pattern.
    #[inline]
    #[must_use]
    pub fn f64<L>(value: L) -> Self
    where
        L: Into<F64Literal>,
    {
        let value = value.into();
        Self::F64(f64::from(value).to_bits())
    }

    /// The rigid atom this literal's type is (ADR-39 D1).
    ///
    /// The single point of truth mapping a numeric literal to its primitive
    /// atom; the checker, machine, and mark all type a [`Value::Num`] through
    /// it, so the six spellings stay in one place and the three passes agree.
    #[inline]
    #[must_use]
    pub fn value_type(&self) -> ValueType
    {
        match *self {
            | Self::U32(_) => ValueType::u32(),
            | Self::U64(_) => ValueType::u64(),
            | Self::I32(_) => ValueType::i32(),
            | Self::I64(_) => ValueType::i64(),
            | Self::F32(_) => ValueType::f32(),
            | Self::F64(_) => ValueType::f64(),
        }
    }
}

impl core::fmt::Debug for NumLit
{
    /// Reproduces `derive(Debug)` for the integer variants, and renders the
    /// float variants from their stored bit pattern as the float value (e.g.
    /// `F64(1.0)`, not the raw `u64`), so error and proptest output stays
    /// legible.
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::U32(v) => f.debug_tuple("U32").field(&v).finish(),
            | Self::U64(v) => f.debug_tuple("U64").field(&v).finish(),
            | Self::I32(v) => f.debug_tuple("I32").field(&v).finish(),
            | Self::I64(v) => f.debug_tuple("I64").field(&v).finish(),
            | Self::F32(bits) => f.debug_tuple("F32").field(&f32::from_bits(bits)).finish(),
            | Self::F64(bits) => f.debug_tuple("F64").field(&f64::from_bits(bits)).finish(),
        }
    }
}

/// A value `v`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value
{
    /// A variable `x`.
    Var(
        /// The variable's name.
        String,
    ),
    /// The unit value `()`.
    Unit,
    /// An integer literal `n` (A2.1 literals extension; `A2-PLAN.md` §A2.1).
    ///
    /// Literals are axioms: an integer infers the rigid atom `Integer`
    /// ([`ValueType::integer`]) and checks by subsumption, exactly like
    /// [`Value::Unit`] with [`ValueType::Unit`].
    Int(
        /// The literal's numeric value.
        i64,
    ),
    /// A string literal `s` (the value-model ladder's first scalar rung;
    /// `proposal-shell-usage-surface.md` §2, ADR-38).
    ///
    /// Like [`Value::Int`], a string is an *axiom*: it infers the rigid atom
    /// `String` ([`ValueType::string`]) and checks by subsumption, exactly like
    /// [`Value::Unit`] with [`ValueType::Unit`]. The payload is an owned
    /// UTF-8 string — the deliberately plain first-pass representation (the
    /// richer view/COW/FFI string model is the reserved revisit).
    Str(
        /// The literal's character data (owned, UTF-8).
        String,
    ),
    /// A typed numeric literal (the value-model ladder's numeric primitive
    /// rung; `proposal-shell-usage-surface.md` §2, ADR-39).
    ///
    /// Like [`Value::Int`] and [`Value::Str`], a numeric literal is an *axiom*:
    /// it infers the rigid atom of its [`NumLit`] tag
    /// ([`NumLit::value_type`] — `u32`/`u64`/`i32`/`i64`/`f32`/`f64`) and
    /// checks by subsumption. A *suffixed* surface literal (`8080u32`,
    /// `1.5f64`) is monomorphic and lowers here; the *unsuffixed* integer
    /// literal stays the frozen [`Value::Int`] (it infers `Integer` but is
    /// checking-mode polymorphic over the integer atoms, ADR-39 D4), and
    /// the unsuffixed float literal lowers to an `f64` `Num`.
    Num(
        /// The literal's typed numeric payload.
        NumLit,
    ),
    /// An eager pair `(v, v′)`.
    Pair(
        /// The first component.
        Rc<Self>,
        /// The second component.
        Rc<Self>,
    ),
    /// An injection `inj1 v` or `inj2 v` into a tagged sum.
    Inj(
        /// Which summand is injected.
        Side,
        /// The injected payload.
        Rc<Self>,
    ),
    /// A list literal `[v₀, …, vₙ]` — the value-model ladder's list rung
    /// (`proposal-shell-usage-surface.md` §2, ADR-40).
    ///
    /// The flat-vector intro of [`crate::types::ValueType::List`]: its
    /// elements are ordinary values, so substitution and structural diffing
    /// descend into them (unlike the opaque scalar leaves [`Value::Str`] /
    /// [`Value::Num`]). Typing is **check-only** against an expected `List
    /// A` (like [`Value::Inj`]): each element checks against the element
    /// type `A`, and the empty list `[]` cannot infer one — so a list in
    /// inference position is stuck, annotate it (ADR-40 D3). The eliminator
    /// is the structural [`Comp::ListCase`].
    List(
        /// The list's elements, in order.
        Vec<Rc<Self>>,
    ),
    /// A record literal `{ℓᵢ=vᵢ}` — the value-model ladder's record rung
    /// (`proposal-shell-usage-surface.md` §2, ADR-45).
    ///
    /// The labeled intro of [`crate::types::ValueType::Record`]: its
    /// field values are ordinary values, so substitution and structural
    /// diffing descend into them (the structural-child discipline of
    /// [`Value::Pair`] / [`Value::List`], unlike the opaque scalar leaves
    /// [`Value::Str`] / [`Value::Num`]). Fields are held in a [`BTreeMap`]
    /// keyed by label, so the literal is **canonical in field order**.
    /// Typing is **direction-polymorphic**, like the eager pair
    /// [`Value::Pair`] (not check-only like [`Value::List`] /
    /// [`Value::Inj`]): a record *infers* its principal type `{ℓᵢ:Aᵢ}` from
    /// its fields, and *checks* against an expected record by pushing each
    /// expected field's type into the matching field; width / depth
    /// subtyping is then the inlined Sub rule (ADR-45 D3). The eliminator
    /// is the field projection [`Comp::RecordProj`].
    Record(
        /// The fields `ℓᵢ ↦ vᵢ`, keyed by label (canonical, name-ordered).
        BTreeMap<String, Rc<Self>>,
    ),
    /// A graded thunk `thunk_r t` suspending a computation.
    Thunk(
        /// The usage grade annotation `r`.
        Grade,
        /// The suspended computation.
        Rc<Comp>,
    ),
    /// A type annotation `(v : A)` — the standard check⇒infer coercion.
    Annot(
        /// The annotated value.
        Rc<Self>,
        /// The ascribed type.
        Rc<ValueType>,
    ),
    /// A typed hole `?u` in value position (A2.2 holes extension;
    /// `spec:implementation/incremental-pipeline.md` §"Holes", `A2-PLAN.md`
    /// D5).
    ///
    /// A hole is an *axiom*, like [`Value::Unit`] and [`Value::Int`]: it
    /// infers [`ValueType::Unknown`] (the spec's "fresh α; no constraint
    /// emitted", degenerated honestly — Stage 1 has no σ, so the fresh
    /// variable collapses to the unknown type) and checks against **any**
    /// expected type (the expected type is the *goal* the agent stream
    /// serves).
    Hole(
        /// The hole's identifier (ignored by typing; see [`HoleId`]).
        HoleId,
    ),
    /// A reified stack `stk K` — a value holding an evaluation context
    /// (`effects-control-shell.md` §2.1 rule Reify; contract §6.3; A3.3
    /// `+control`).
    ///
    /// The **dual of [`Value::Thunk`]**: a thunk holds a *computation* as a
    /// value, a `stk K` holds a *stack* (Levy's third syntactic sort) as a
    /// value, crossing the value/computation boundary the other way. Typing is
    /// **check-only** against an expected
    /// [`crate::types::ValueType::Stk`] `Stk(B, C)` (like
    /// [`Value::Inj`]): the stack-typing judgment `K : B ⇒ C` runs forward
    /// from the consumed type `B`, synthesizing the delivered answer, which
    /// the inlined Sub rule fits to `C`. The `Stk` is `Rc`'d so
    /// `Value` stays the size of the other nodes (the cons cells are `Rc` too,
    /// so cloning into traces/frames stays cheap).
    Stk(
        /// The reified stack `K`.
        Rc<Stack>,
    ),
    /// A reflexivity proof `here(v) : Path A v v` — the sole introduction of
    /// the identity type [`crate::types::ValueType::Path`] (ADR-76; rule
    /// `Here`).
    ///
    /// An **introduction** form (like [`Self::Inj`]): it infers `Path A v v`
    /// from the inferred type `A` of its witness `v` (`Here⇑`), and checks
    /// against an expected `Path A x y` when `x ≡ᵥ v` and `y ≡ᵥ v`
    /// (`Here⇓`). Its payload is an ordinary value, so substitution and
    /// structural diffing descend into it (the structural-child discipline
    /// of [`Self::Inj`]). It is the only canonical inhabitant of a closed
    /// identity type: canonicity means a closed `Path`-typed value reads
    /// back as `here(v)`. The eliminator is the full Martin-Löf
    /// [`Comp::Walk`], whose β-rule fires exactly on this constructor.
    Here(
        /// The witness value `v` (the proof is that `v` equals itself).
        Rc<Self>,
    ),
    /// A **declared-data constructor value** `Ctor { id, tag, payload }` — the
    /// intro of [`crate::types::ValueType::Data`] (ADR-80 Decision 2),
    /// mirroring [`Self::Inj`] for sums.
    ///
    /// `id` is the datatype's minted nominal identity ([`DataId`]); `tag` is
    /// the constructor's position in the decl-table `ctors` list (the
    /// analogue of [`Side`] for a `k`-constructor datatype); `payload` is
    /// the constructor's **field-tuple** as an existing structural value —
    /// [`Self::Unit`] for a nullary constructor (`None`, `Red`), the single
    /// field value for a one-field constructor (`Some(x)`'s `x`), a
    /// record/product otherwise (ADR-80 Decision 2/5). The constructor
    /// identity living **in the value** is what lets the renderer print
    /// `Some(3)` rather than the carrier `Inr(#{x = 3})` with no static
    /// type at the render site.
    ///
    /// An **introduction** form, **check-only** like [`Self::Inj`]: it checks
    /// against an expected [`crate::types::ValueType::Data`] (verifying
    /// the nominal `id`), and a `Ctor` in inference position is stuck
    /// (annotate). Its payload is an ordinary value, so substitution and
    /// structural diffing descend into it. The eliminator is
    /// [`Comp::DataCase`].
    Ctor
    {
        /// The datatype's minted nominal identity.
        id: DataId,
        /// The constructor's position in the decl-table `ctors` list.
        tag: usize,
        /// The field-tuple payload (unit / value / product-or-record).
        payload: Rc<Self>,
    },
    /// A **packed module** `pack ⟨Ā⟩ v` — the introduction of
    /// [`crate::types::ValueType::Package`], and the module layer's one
    /// new value form.
    ///
    /// It carries a witness type for each abstract type component the signature
    /// declares, in signature order, together with the payload those witnesses
    /// abstract: the grade-`r` thunked module returner the package
    /// internalizes. Checking substitutes the witnesses into the
    /// signature's payload simultaneously
    /// (`gandr_core_checker::judgements::package::instantiate`) and checks
    /// `payload` against the result, so the packer's representation is
    /// checked at the representation and hidden everywhere after.
    ///
    /// **Check-only, and the witnesses are why.** A pack in inference position
    /// is stuck ([`crate::error::text::ANNOTATE_PACK`]), like
    /// [`Self::Ctor`] and [`Self::Inj`] — but for a stronger reason than
    /// either. The abstract components exist only in the signature, so
    /// inferring a package type from the payload's structure would mean
    /// *guessing* which of the payload's types were meant to be abstract,
    /// which is exactly the guess the module-and-core boundary is annotated
    /// in both directions to forbid. The witnesses are the other half of
    /// that annotation: they are in the term rather than inferred, so no
    /// rule ever recovers them from the payload.
    ///
    /// Its payload is an ordinary value, so substitution and structural diffing
    /// descend into it (the structural-child discipline of [`Self::Inj`]). Its
    /// eliminator is [`Comp::Unpack`] and nothing else: forcing it is a shape
    /// mismatch, because a package is not a thunk however alike the two look.
    Pack
    {
        /// The witness types `Ā`, positionally matching the signature's
        /// abstract type components.
        witnesses: Vec<Rc<ValueType>>,
        /// The packed payload — the thunked module returner the package
        /// internalizes, at the witnesses' types.
        payload: Rc<Self>,
    },
    /// The **pure-computation embedding** `run t` — the value a pure
    /// computation returns, and the one form that lets a term written as an
    /// application occur in a type.
    ///
    /// # Why it exists
    ///
    /// Call-by-push-value separates the sorts: `f(x)` is a *computation* of
    /// type `F A`, never a value. Types, meanwhile, are indexed by **values** —
    /// [`crate::types::ValueType::Path`] carries two of them. So a law like
    /// `Path(Hom, comp(id(a), f), f)` cannot be written at all without a way
    /// to name the value an application produces, and no relaxation of the
    /// surface reaches it, because the obstacle is the sort rather than the
    /// syntax.
    ///
    /// # The purity premise is the whole content
    ///
    /// The embedding is admitted **exactly** when the computation's effect row
    /// is empty. That is not a caveat attached to the rule; it is what makes
    /// the rule sound. A pure computation is deterministic up to the step
    /// budget, so the value it denotes is stable under substitution and under
    /// the context it appears in — which is precisely what a type occurring in
    /// many places needs. An effectful computation denotes no such thing, and
    /// its embedding is **refused by name** rather than admitted under a
    /// pure-enough reading. There is no pure-enough reading.
    ///
    /// # What it is not
    ///
    /// It is not a thunk. [`Self::Thunk`] *suspends* a computation and keeps it
    /// a computation, eliminated by [`Comp::Force`]; this *names the value that
    /// computation returns*, and is eliminated by nothing, because it is
    /// already a value. Where the two meet is telling: `run (force (thunk t))`
    /// and `run t` denote the same value, and `thunk (run t)` does not
    /// typecheck at all.
    ///
    /// # How it computes
    ///
    /// Evaluation runs the computation under the **shared** step budget, the
    /// same budget and the same policy context every other unfolding surface
    /// reads. A computation stuck on a variable leaves the embedding a
    /// **neutral value**, quoted back as itself and compared by congruence,
    /// which is how an open law-field type normalizes as far as its arguments
    /// allow and no further. Budget exhaustion in type position is a refusal
    /// carrying its evidence, never an unsound acceptance: it costs
    /// availability and nothing else.
    Run(
        /// The embedded computation, which must infer a returner at the empty
        /// effect row.
        Rc<Comp>,
    ),
}

impl Value
{
    /// Builds a variable.
    #[inline]
    #[must_use]
    pub fn var<'source, N>(name: N) -> Self
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        Self::Var(name.as_ref().to_owned())
    }

    /// Builds the pure-computation embedding `run body`.
    #[inline]
    #[must_use]
    pub fn run(body: Comp) -> Self
    {
        Self::Run(Rc::new(body))
    }

    /// Builds an integer literal.
    #[inline]
    #[must_use]
    pub fn int<L>(literal: L) -> Self
    where
        L: Into<IntegerLiteral>,
    {
        Self::Int(i64::from(literal.into()))
    }

    /// Builds a string literal from a borrowed string slice.
    ///
    /// Named `string` rather than `str` to avoid shadowing the primitive type.
    #[inline]
    #[must_use]
    pub fn string<'source, S>(literal: S) -> Self
    where
        S: Into<StringLiteral<'source>>,
    {
        let literal = literal.into();
        Self::Str(literal.as_ref().to_owned())
    }

    /// Builds a typed numeric literal from a [`NumLit`] payload (ADR-39).
    #[inline]
    #[must_use]
    pub fn num(literal: NumLit) -> Self
    {
        Self::Num(literal)
    }

    /// Builds a `u32` literal.
    #[inline]
    #[must_use]
    pub fn u32<L>(value: L) -> Self
    where
        L: Into<U32Literal>,
    {
        Self::Num(NumLit::U32(u32::from(value.into())))
    }

    /// Builds a `u64` literal.
    #[inline]
    #[must_use]
    pub fn u64<L>(value: L) -> Self
    where
        L: Into<U64Literal>,
    {
        Self::Num(NumLit::U64(u64::from(value.into())))
    }

    /// Builds an `i32` literal.
    #[inline]
    #[must_use]
    pub fn i32<L>(value: L) -> Self
    where
        L: Into<I32Literal>,
    {
        Self::Num(NumLit::I32(i32::from(value.into())))
    }

    /// Builds an `i64` literal.
    #[inline]
    #[must_use]
    pub fn i64<L>(value: L) -> Self
    where
        L: Into<I64Literal>,
    {
        Self::Num(NumLit::I64(i64::from(value.into())))
    }

    /// Builds an `f32` literal (stored as its bit pattern).
    #[inline]
    #[must_use]
    pub fn f32<L>(value: L) -> Self
    where
        L: Into<F32Literal>,
    {
        Self::Num(NumLit::f32(value))
    }

    /// Builds an `f64` literal (stored as its bit pattern).
    #[inline]
    #[must_use]
    pub fn f64<L>(value: L) -> Self
    where
        L: Into<F64Literal>,
    {
        Self::Num(NumLit::f64(value))
    }

    /// Builds an eager pair.
    #[inline]
    #[must_use]
    pub fn pair(
        fst: Self,
        snd: Self,
    ) -> Self
    {
        Self::Pair(Rc::new(fst), Rc::new(snd))
    }

    /// Builds a list literal from its elements (ADR-40).
    #[inline]
    #[must_use]
    pub fn list(elements: Vec<Self>) -> Self
    {
        Self::List(elements.into_iter().map(Rc::new).collect())
    }

    /// Builds a record literal from its labeled fields (ADR-45).
    ///
    /// Fields collect into a [`BTreeMap`], so the result is canonical in field
    /// order; a duplicated label keeps the last-supplied value.
    #[inline]
    #[must_use]
    pub fn record<I>(fields: I) -> Self
    where
        I: IntoIterator<Item = (String, Self)>,
    {
        Self::Record(
            fields
                .into_iter()
                .map(|(label, field)| (label, Rc::new(field)))
                .collect(),
        )
    }

    /// Builds a left injection `inj1 payload`.
    #[inline]
    #[must_use]
    pub fn inj1(payload: Self) -> Self
    {
        Self::Inj(Side::Fst, Rc::new(payload))
    }

    /// Builds a right injection `inj2 payload`.
    #[inline]
    #[must_use]
    pub fn inj2(payload: Self) -> Self
    {
        Self::Inj(Side::Snd, Rc::new(payload))
    }

    /// Builds a graded thunk `thunk_grade body`.
    #[inline]
    #[must_use]
    pub fn thunk(
        grade: Grade,
        body: Comp,
    ) -> Self
    {
        Self::Thunk(grade, Rc::new(body))
    }

    /// Builds a type annotation `(value : ty)`.
    #[inline]
    #[must_use]
    pub fn annot(
        value: Self,
        ty: ValueType,
    ) -> Self
    {
        Self::Annot(Rc::new(value), Rc::new(ty))
    }

    /// Builds a typed hole in value position.
    #[inline]
    #[must_use]
    pub fn hole<I>(id: I) -> Self
    where
        I: Into<SourceIndex>,
    {
        Self::Hole(u32::from(id.into()))
    }

    /// Builds a reified stack `stk stack` (rule Reify,
    /// `effects-control-shell.md` §2.1).
    #[inline]
    #[must_use]
    pub fn stk(stack: Stack) -> Self
    {
        Self::Stk(Rc::new(stack))
    }

    /// Builds a reflexivity proof `here(witness)` (ADR-76; rule `Here`).
    #[inline]
    #[must_use]
    pub fn here(witness: Self) -> Self
    {
        Self::Here(Rc::new(witness))
    }

    /// Builds a declared-data constructor value `Ctor { id, tag, payload }`
    /// (ADR-80 Decision 2).
    #[inline]
    #[must_use]
    pub fn ctor<T>(
        id: DataId,
        tag: T,
        payload: Self,
    ) -> Self
    where
        T: Into<ConstructorTag>,
    {
        let tag = tag.into();
        Self::Ctor {
            id,
            tag: usize::from(tag),
            payload: Rc::new(payload),
        }
    }

    /// Builds a packed module `pack ⟨witnesses⟩ payload` ([`Self::Pack`]).
    ///
    /// The witnesses are positional: witness `i` discharges the signature's
    /// `i`th abstract type component.
    #[inline]
    #[must_use]
    pub fn pack<I>(
        witnesses: I,
        payload: Self,
    ) -> Self
    where
        I: IntoIterator<Item = ValueType>,
    {
        Self::Pack {
            witnesses: witnesses.into_iter().map(Rc::new).collect(),
            payload: Rc::new(payload),
        }
    }

    /// Peels type-annotation layers `(v : A)` off, returning the innermost
    /// non-[`Self::Annot`] value (an ascription is operationally transparent).
    /// The `as_*` decode accessors read through it.
    #[inline]
    #[must_use]
    fn peeled(&self) -> &Self
    {
        let mut current = self;
        while let Self::Annot(ref inner, _) = *current {
            current = inner;
        }
        current
    }

    /// Reads an underlying string literal, seeing through annotations — a
    /// host-seam payload decoder for [`Self::Str`] (ADR-35 D4), letting a host
    /// callback inspect a payload without matching on [`Value`].
    ///
    /// # Contract
    /// - ensures: `Some(StringText)` iff the annotation-stripped value is a
    ///   [`Self::Str`]; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> Option<StringText<'_>>
    {
        match *self.peeled() {
            | Self::Str(ref literal) => Some(StringText::from(literal.as_str())),
            | _ => None,
        }
    }

    /// Reads an underlying integer literal, seeing through annotations — a
    /// host-seam payload decoder for [`Self::Int`] (ADR-35 D4).
    ///
    /// # Contract
    /// - ensures: `Some(IntegerLiteral)` iff the annotation-stripped value is a
    ///   [`Self::Int`]; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_int(&self) -> Option<IntegerLiteral>
    {
        match *self.peeled() {
            | Self::Int(literal) => Some(IntegerLiteral::from(literal)),
            | _ => None,
        }
    }

    /// Reads an underlying record's fields, seeing through annotations — a
    /// host-seam payload decoder for [`Self::Record`] (ADR-35 D4).
    ///
    /// # Contract
    /// - ensures: `Some(&{ℓᵢ ↦ vᵢ})` iff the annotation-stripped value is a
    ///   [`Self::Record`]; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_record(&self) -> Option<&BTreeMap<String, Rc<Self>>>
    {
        match *self.peeled() {
            | Self::Record(ref fields) => Some(fields),
            | _ => None,
        }
    }

    /// Reads an underlying list's elements, seeing through annotations — a
    /// host-seam payload decoder for [`Self::List`] (ADR-35 D4), the list
    /// analogue of [`Self::as_str`] / [`Self::as_record`] (e.g. decoding an
    /// `exec` operation's `List Str` argv).
    ///
    /// # Contract
    /// - ensures: `Some(&[vᵢ])` iff the annotation-stripped value is a
    ///   [`Self::List`]; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_list(&self) -> Option<&[Rc<Self>]>
    {
        match *self.peeled() {
            | Self::List(ref elements) => Some(elements),
            | _ => None,
        }
    }
}

/// One operation clause `op p k ⇒ t` of a [`Comp::Handle`]
/// (`effects-control-shell.md` §1.1 rule Handle).
///
/// `op` names the handled operation (which must be an operation of the
/// handler's signature `E`). `payload` binds `p` (the op's payload `A_op`) and
/// `resume` binds `k` (the captured continuation, a
/// [`crate::types::ValueType::Stk`] value `Stk(F^ε B_op, F^ε C)`); `body`
/// is the clause's computation `t`, checked against the handler's answer `F^ε
/// C`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OpClause
{
    /// The handled operation's name (an operation of the handler's signature).
    pub op: String,
    /// The payload binder `p`, bound to the operation's payload type `A_op`.
    pub payload: String,
    /// The resumption binder `k`, bound to `Stk(F^ε B_op, F^ε C)`.
    pub resume: String,
    /// The clause body `t`, checked against the handler's answer `F^ε C`.
    pub body: Rc<Comp>,
}

impl OpClause
{
    /// Builds an operation clause `op payload resume ⇒ body`.
    #[inline]
    #[must_use]
    pub fn new<'source, O, P, R>(
        op: O,
        payload: P,
        resume: R,
        body: Comp,
    ) -> Self
    where
        O: Into<OperationName<'source>>,
        P: Into<BinderName<'source>>,
        R: Into<ResumeName<'source>>,
    {
        let op = op.into();
        let payload = payload.into();
        let resume = resume.into();
        Self {
            op: op.as_ref().to_owned(),
            payload: payload.as_ref().to_owned(),
            resume: resume.as_ref().to_owned(),
            body: Rc::new(body),
        }
    }
}

/// The **motive** binder of the identity eliminator [`Comp::Walk`]: the family
/// `C(x, y, q)` over both endpoints `x y : A` and the path `q : Path A x y`
/// (ADR-76; the full Martin-Löf dinatural form).
///
/// This is part of the `Walk` **syntax form** — like a `case` arm's binder —
/// NOT a first-class function: gandr's [`crate::types::CompType::Arrow`]
/// is non-dependent, so a motive binding value endpoints inside its result type
/// is not typeable as a standalone value. The three binders `x`, `y`, `q` are
/// value variables ([`Value::Var`]) that appear inside the
/// [`crate::types::ValueType::Path`] sub-terms of `body`; the motive
/// lands in a **computation type** (`F`-wrapped value motives are the special
/// case), so transport can eliminate into arbitrary computations. Motive
/// instantiation is the value-into-type substitution of
/// [`crate::identity`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WalkMotive
{
    /// The left-endpoint binder `x`.
    pub x: String,
    /// The right-endpoint binder `y`.
    pub y: String,
    /// The path binder `q : Path A x y`.
    pub q: String,
    /// The motive body `C(x, y, q)` — a computation type over the three
    /// binders.
    pub body: Rc<CompType>,
}

/// The **base** (diagonal) binder of the identity eliminator [`Comp::Walk`]:
/// the witness `c(x) : C(x, x, here(x))` (ADR-76).
///
/// Like [`WalkMotive`], part of the `Walk` syntax form — not a function. Its
/// single value binder `x` scopes the body computation `c`, checked against the
/// motive's diagonal instance. The β-rule reduces `walk(here(v), C, (x). c)` to
/// `c[v/x]`, substituting `v` for exactly this binder.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WalkBase
{
    /// The diagonal binder `x`.
    pub x: String,
    /// The base body `c(x)`, checked against `C[x/y][here(x)/q]`.
    pub body: Rc<Comp>,
}

impl WalkMotive
{
    /// Builds a Walk motive `(x y q). body`.
    #[inline]
    #[must_use]
    pub fn new<'source, X, Y, Q>(
        x: X,
        y: Y,
        q: Q,
        body: CompType,
    ) -> Self
    where
        X: Into<NameRef<'source>>,
        Y: Into<NameRef<'source>>,
        Q: Into<NameRef<'source>>,
    {
        let x = x.into();
        let y = y.into();
        let q = q.into();
        Self {
            x: x.as_ref().to_owned(),
            y: y.as_ref().to_owned(),
            q: q.as_ref().to_owned(),
            body: Rc::new(body),
        }
    }
}

impl WalkBase
{
    /// Builds a Walk base `(x). body`.
    #[inline]
    #[must_use]
    pub fn new<'source, X>(
        x: X,
        body: Comp,
    ) -> Self
    where
        X: Into<NameRef<'source>>,
    {
        let x = x.into();
        Self {
            x: x.as_ref().to_owned(),
            body: Rc::new(body),
        }
    }
}

/// The **motive** binder of the product / dependent-pair eliminator
/// [`Comp::Split`]: the family `M(z)` over the scrutinee value `z` (ADR-82).
///
/// The dependent generalization of the frozen non-dependent `split` (over
/// `Prod` and, since ADR-81, `Σ`).
///
/// Like [`WalkMotive`], this is part of the `Split` **syntax form** — a
/// `case`-arm-style binder, NOT a first-class function: gandr's
/// [`crate::types::CompType::Arrow`] is non-dependent, so a motive
/// binding a value inside its result type is not typeable as a standalone
/// value. The single value binder `z` ([`Value::Var`]) appears inside the
/// [`crate::types::ValueType::Path`] sub-terms (or a `Σ` tail) of `body`,
/// which lands in a **computation type** so elimination targets arbitrary
/// computations. Motive instantiation is the value-into-type substitution of
/// [`crate::identity::subst_comptype`]: the body checks against `M[(p, q)/z]`
/// and the eliminator delivers `M[v/z]` (ADR-82 D2). A motive-bearing split
/// **infers** (rule `SplitMotive`⇑); the motive-less form is check-only (rule
/// Split⇓, [`Comp::split`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SplitMotive
{
    /// The scrutinee binder `z`.
    pub binder: String,
    /// The motive body `M(z)` — a computation type over the scrutinee value.
    pub body: Rc<CompType>,
}

impl SplitMotive
{
    /// Builds a split motive `(binder). body`.
    #[inline]
    #[must_use]
    pub fn new<'source, B>(
        binder: B,
        body: CompType,
    ) -> Self
    where
        B: Into<BinderName<'source>>,
    {
        let binder = binder.into();
        Self {
            binder: binder.as_ref().to_owned(),
            body: Rc::new(body),
        }
    }
}

/// A computation `t`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Comp
{
    /// An abstraction `λx. t` (unannotated) or `λx:A. t` (annotated binder).
    Abs(
        /// The bound variable.
        String,
        /// The optional binder annotation; required for inference.
        Option<Rc<ValueType>>,
        /// The body.
        Rc<Self>,
    ),
    /// An application `t v` of a computation to a value argument.
    App(
        /// The function computation (principal premise; inferred).
        Rc<Self>,
        /// The value argument (checked).
        Rc<Value>,
    ),
    /// A returner `ret v` producing a value.
    Ret(
        /// The produced value.
        Rc<Value>,
    ),
    /// Sequencing `t >>= x. u`.
    Bind(
        /// The bound computation (always inferred).
        Rc<Self>,
        /// The variable receiving the produced value.
        String,
        /// The continuation.
        Rc<Self>,
    ),
    /// Forcing a thunk: `force v`.
    Force(
        /// The thunk value.
        Rc<Value>,
    ),
    /// Sum elimination `case v of { inj1 x → t | inj2 y → u }`.
    Case(
        /// The scrutinee (inferred).
        Rc<Value>,
        /// The first arm: binder and body for `inj1`.
        (String, Rc<Self>),
        /// The second arm: binder and body for `inj2`.
        (String, Rc<Self>),
    ),
    /// **Declared-data elimination** `case v { C₀(x₀) → t₀ | … }` — the case
    /// over a constructor tag, the eliminator of
    /// [`crate::types::ValueType::Data`] (ADR-80 Decision 3), mirroring
    /// [`Self::Case`] for a `k`-constructor datatype.
    ///
    /// `arms` is one `(binder, body)` per constructor, positionally: arm `i`
    /// handles the value `Ctor { tag: i, payload }`, binding `binder` to the
    /// constructor's field-tuple `payload`. A nullary constructor's arm binds a
    /// discard binder to the unit payload; a one-field constructor's arm binds
    /// the field. **Check-only** (like [`Self::Case`]): it infers the scrutinee
    /// (which must be a [`crate::types::ValueType::Data`] or `Unknown`)
    /// and checks each arm against the expected answer, binding each arm's
    /// payload binder at `Unknown` — the frozen core carries the nominal
    /// tag but not the constructor field types (those live in the decl
    /// table the pipeline holds, ADR-80 Decision 4/5), so field typing is
    /// the pipeline seam's job. An **empty** `arms` is the absurd match
    /// `case x {}` over an uninhabited datatype. Non-recursive only (ADR-80
    /// Decision 6).
    DataCase(
        /// The scrutinee (inferred; must be a declared-data value).
        Rc<Value>,
        /// One `(binder, body)` per constructor, arm `i` handling tag `i`.
        Vec<(String, Rc<Self>)>,
    ),
    /// List elimination `case v of { Nil → t | Cons(h, t) → u }` — the
    /// value-model ladder's structural list eliminator (`ListCase`; ADR-40 D4).
    ///
    /// The non-recursive one-level destructor of
    /// [`crate::types::ValueType::List`], the list analogue of
    /// [`Self::Case`]: it infers the scrutinee `v`, checks the `nil` body
    /// against the expected answer, and checks the `cons` body under `head
    /// : A, tail : List A` (a matched-`Unknown` scrutinee binds both
    /// at `Unknown`, the `Case` discipline). **Check-only** (like `Case`):
    /// inference is stuck. It introduces no recursion — full iteration is the
    /// native-builtin layer. The `head` / `tail` binders are
    /// attributes (not children); the term children are the scrutinee (0), the
    /// `nil` body (1), and the `cons` body (2), the order `origin::resolve` and
    /// `edit` share.
    ListCase
    {
        /// The scrutinee `v` (inferred; must be a `List A`).
        scrut: Rc<Value>,
        /// The empty-list arm body `t` (checked against the answer).
        nil: Rc<Self>,
        /// The `cons` arm's head binder `h`, bound to the element type `A`.
        head: String,
        /// The `cons` arm's tail binder `t`, bound to `List A`.
        tail: String,
        /// The `cons` arm body `u` (checked against the answer, under
        /// `head`/`tail`).
        cons: Rc<Self>,
    },
    /// Product / dependent-pair elimination `split v as (p, q) [z. M] in t`
    /// (ADR-82; rule `SplitMotive`⇑ with a motive, rule Split⇓ without).
    ///
    /// gandr's **second dependent eliminator** (after [`Self::Walk`]). Given a
    /// scrutinee `v ⇑ Σ(x:A). B` (a `Prod` is the constant-tail degenerate),
    /// the two binders scope the body: `p : A` and `q : B[p/x]`. The optional
    /// motive `(z). M` ([`SplitMotive`]) binds the **scrutinee value** `z` in a
    /// computation type:
    ///
    /// * **with a motive** the split *infers* (rule `SplitMotive`⇑) — the body
    ///   checks against `M[(p, q)/z]` and the split delivers `M[v/z]`, built
    ///   from the outer-scoped `M` and the scrutinee `v`, so **no binder can
    ///   escape into it** (the [`crate::identity::subst_comptype`]
    ///   instantiation, untraced pure type computation);
    /// * **without a motive** the split is *check-only* (rule Split⇓,
    ///   [`Comp::split`]) — the expectation `C` arrives binder-free from the
    ///   outer context and is delivered verbatim; a motive-less split in
    ///   inference position is stuck
    ///   ([`crate::error::text::SPLIT_NEEDS_MOTIVE`], the check-only-eliminator
    ///   discipline of [`Self::Case`] / [`Self::ListCase`]).
    ///
    /// A matched `Unknown` scrutinee (A2.2 holes) binds both components at
    /// `Unknown`; with a motive it delivers `Unknown` (the [`Self::Walk`]
    /// precedent, motive ignored), without one it delivers the expectation
    /// (rule Split⇓). The runtime is type-erased: split-β runs a `Σ`-typed or
    /// motive-bearing split exactly as a product one (ADR-82 D4). The `p` / `q`
    /// binders and the motive are attributes; the term children are the
    /// scrutinee (0) and the body (1) — the motive is a **type** child, not a
    /// term child.
    Split
    {
        /// The scrutinee (inferred).
        scrut: Rc<Value>,
        /// The binder for the first component `p`.
        fst_name: String,
        /// The binder for the second component `q`.
        snd_name: String,
        /// The optional dependent motive `(z). M` (ADR-82); `Box`ed so `Comp`
        /// (and thus the diagnostic [`Term`] inside
        /// [`crate::error::TypeError`]) stays small when the common motive-less
        /// case is `None`, exactly as [`Self::Walk`] boxes its motive (the
        /// large-`Err` lint threshold).
        motive: Option<Box<SplitMotive>>,
        /// The body `t`.
        body: Rc<Self>,
    },
    /// Record field projection `r.ℓ` — the value-model ladder's record
    /// eliminator (ADR-45 D4).
    ///
    /// The positive-record analogue of the lazy-product projection
    /// [`Self::Prj`]: it infers the record value `record ⇑ {…ℓ:A…}`, looks up
    /// the field `label`, and delivers the returner `F A` — eliminating a
    /// positive value type is a computation, exactly as a [`Self::Split`] is.
    /// An **inference** form (like [`Self::App`] / [`Self::Resume`]): in
    /// checking mode it infers, then subsumes. A matched-`Unknown` record
    /// projects `Unknown` (the hole localizes, as an `Unknown` application
    /// head); a record lacking the field is stuck. Operationally
    /// `{ℓ=v,…}.ℓ ⟶ ret v` extracts the field. The `record` is a value (records
    /// are inert values, like the [`Self::Split`] scrutinee); single-field
    /// projection is the minimal positive eliminator — the record / table
    /// combinator library (`get` / `insert` / …) is the native-builtin layer.
    RecordProj
    {
        /// The record value being projected (inferred; must be a `{…ℓ:A…}`).
        record: Rc<Value>,
        /// The projected field label `ℓ`.
        label: String,
    },
    /// A lazy pair `⟨t, u⟩`.
    With(
        /// The first component.
        Rc<Self>,
        /// The second component.
        Rc<Self>,
    ),
    /// A projection `prj1 t` or `prj2 t` from a lazy pair.
    Prj(
        /// Which component is projected.
        Side,
        /// The projected computation (inferred).
        Rc<Self>,
    ),
    /// The grade structural op `dup v`: split a thunk's usage budget
    /// (`spec:implementation/type-system.md` §"Grades" rule Dup; contract
    /// §6.2). Checks against `F (U_r B × U_s B)`, requiring `r + s ⊑ g`
    /// where `v ⇑ U_g B`.
    Dup(
        /// The thunk value whose budget is split (inferred).
        Rc<Value>,
    ),
    /// The grade structural op `drop v`: discard a thunk's usage budget
    /// (`spec:implementation/type-system.md` §"Grades" rule Drop; contract
    /// §6.2). Infers `F 1`; the side condition `0 ⊑ r` is vacuous on the
    /// default carrier `ℕ ∪ {ω}`.
    Drop(
        /// The thunk value whose budget is discarded (inferred).
        Rc<Value>,
    ),
    /// An effect operation `perform op v` (`effects-control-shell.md` §1.1
    /// rule Op; contract §6.2; A3.2 `+effects`).
    ///
    /// The performed operation is named by [`String`] and **resolved against
    /// the inline-carried signature** [`EffectSig`] (ADR-33 D3: signatures are
    /// environment-free, riding directly on the node). Typing infers the
    /// singleton-row returner `F^⟨E⟩ B_op`: the payload `v` checks against the
    /// op's payload type `A_op`, and the op's reply type `B_op` is the produced
    /// value type. `perform` is an **inference** form (like [`Self::App`] /
    /// [`Self::Force`]); the open tail `ε` of the kernel row `⟨E|ε⟩` is
    /// reserved for `+poly` (ADR-33 D2), so v0 contributes exactly the
    /// singleton `⟨E⟩`.
    ///
    /// The signature is `Box`ed so this node stays the size of the other
    /// elimination forms (an inline [`EffectSig`] would push `Comp` — and thus
    /// the diagnostic [`Term`] inside [`crate::error::TypeError`] — past the
    /// large-`Err` lint threshold); it remains inline-carried (ADR-33 D3).
    Perform(
        /// The effect signature `E` this operation belongs to (inline-carried).
        Box<EffectSig>,
        /// The operation's name (must name an op of the signature `E`).
        String,
        /// The payload value `v` (checked against the op's payload type
        /// `A_op`).
        Rc<Value>,
    ),
    /// A deep effect handler `handle t { ret x ⇒ t_ret | opᵢ p k ⇒ tᵢ }`
    /// (`effects-control-shell.md` §1.1 rule Handle; contract §6.2; A3.2
    /// `+effects`).
    ///
    /// The handler discharges the inline-carried signature `sig` (`E`) from the
    /// handled computation `scrutinee` (`t`). Typing is **check-only** against
    /// a returner answer `F^ε C` (like [`Self::Case`]): the return clause
    /// and each operation clause check against that answer, the
    /// continuation binder `k` is a first-class
    /// [`crate::types::ValueType::Stk`] value `Stk(F^ε B_i, F^ε C)` (deep
    /// — it delivers the same answer, ADR-33 D4), and the residual row `ε_t
    /// ∖ E` of the handled computation must fit the answer's row `ε` (the
    /// soundness leg, discharged by the inlined Sub rule).
    Handle
    {
        /// The handled effect signature `E` (inline-carried, ADR-33 D3; `Box`ed
        /// to keep `Comp` small, as [`Self::Perform`]).
        sig: Box<EffectSig>,
        /// The handled computation `t` (inferred; must be a returner).
        scrutinee: Rc<Self>,
        /// The return clause `ret x ⇒ t_ret`: the bound variable `x` (receiving
        /// the handled computation's value) and the body `t_ret`.
        ret: (String, Rc<Self>),
        /// The operation clauses `opᵢ p k ⇒ tᵢ`, one per operation of `sig`
        /// (deep-handler coverage is exact, ADR-33 D4).
        ops: Vec<OpClause>,
    },
    /// Resuming a reified stack: `resume v t` (`effects-control-shell.md` §2.1
    /// rule Resume; contract §6.2; A3.3 `+control`).
    ///
    /// The elimination of [`crate::types::ValueType::Stk`], structurally
    /// an application whose "function" is the stack value `v` and whose
    /// "argument" is the computation `t`: `v` infers `Stk(B, C)`, `t`
    /// checks against the consumed `B`, and the result is the delivered
    /// answer `C`. An **inference** form (like [`Self::App`]).
    Resume(
        /// The reified stack value `v` (principal premise; inferred to
        /// `Stk(B, C)`).
        Rc<Value>,
        /// The computation `t` fed to the stack (checked against `B`).
        Rc<Self>,
    ),
    /// A delimited-control delimiter: `reset t` (`effects-control-shell.md`
    /// §2.2 rule Reset; contract §6.2; A3.3 `+control`).
    ///
    /// Establishes the **answer type** the enclosed [`Self::Shift`]s capture up
    /// to. Typing is **check-only** against the answer `C` (like
    /// [`Self::Handle`]): it sets the ambient answer to `C`, checks the body
    /// `t ⇓ C`, and is transparent on the type — `reset t` has the same type as
    /// `t`. The operational `KReset(C)` delimiter frame is A5;
    /// the answer-type bookkeeping here rides an ambient register (the v0
    /// form of the spec's "control `C` effect", whose answer-type-modifying
    /// generalization is reserved).
    Reset(
        /// The delimited computation `t` (checked against the answer `C`).
        Rc<Self>,
    ),
    /// A delimited-control capture: `shift k. t` (`effects-control-shell.md`
    /// §2.2 rule Shift; contract §6.2; A3.3 `+control`).
    ///
    /// Captures the context up to the nearest enclosing [`Self::Reset`],
    /// reified as `k : Stk(B, C)`. Typing is **check-only** against `B`: it
    /// reads the ambient answer `C` established by the nearest `reset` (a
    /// `shift` with no enclosing `reset` is stuck), binds `k : Stk(B, C)`,
    /// checks the body `t ⇓ C`, and delivers `B`. Capture is the A5 runtime
    /// operation; typing only checks the answer-type discipline.
    Shift(
        /// The continuation binder `k`, bound to `Stk(B, C)`.
        String,
        /// The shift body `t` (checked against the ambient answer `C`).
        Rc<Self>,
    ),
    /// A typed hole `?u` in computation position (A2.2 holes extension;
    /// `spec:implementation/incremental-pipeline.md` §"Holes", `A2-PLAN.md`
    /// D5).
    ///
    /// As [`Value::Hole`]: an axiom that infers
    /// [`crate::types::CompType::Unknown`] and checks against any
    /// expected type.
    Hole(
        /// The hole's identifier (ignored by typing; see [`HoleId`]).
        HoleId,
    ),
    /// A native (Rust-backed) builtin primitive — the MVP module layer's
    /// native-builtin substrate (ADR-42; contract §6.2).
    ///
    /// A computation that runs Rust code rather than reducing a closed CBPV
    /// term — the home for builtins the v0 IR cannot express as closed terms
    /// (the iteration / table / arithmetic combinators, none writable without
    /// recursion). The node carries an **opaque**
    /// [`crate::prim::NativePrim`] tag — *not* a Rust `fn` (`Comp`
    /// derives `Eq` / `Debug`, which a closure / `fn` pointer cannot honor
    /// stably) — dispatched in the [`crate::prim`] registry, mirroring how
    /// [`Self::Perform`] names an operation rather than carrying a handler.
    ///
    /// Typing is an **axiom** (like [`Self::Hole`] / a literal): the node has
    /// the primitive's declared type with `args.len()` leading arrows peeled
    /// ([`crate::prim::NativePrim::residual_type`]). A **source** native is
    /// always argument-free (`args` empty) — the node materializes only when a
    /// prelude binding is forced; the CEK machine then accumulates argument
    /// frames into `args` and, at saturation, reduces via
    /// [`crate::prim::NativePrim::apply`]. Operationally it is a function-like
    /// terminal, exactly like [`Self::Abs`].
    Native
    {
        /// The opaque primitive tag, dispatched in [`crate::prim`].
        prim: NativePrim,
        /// The arguments accumulated so far (empty in a source term; filled by
        /// the CEK machine as it consumes argument frames).
        ///
        /// Invariant: a *stored* native is never saturated — `args.len() <
        /// prim.arity()` — because the machine reduces (via
        /// [`crate::prim::NativePrim::apply`]) the instant the last argument
        /// lands rather than parking a full list; the only `args.len() ==
        /// arity` node is the transient one `apply` consumes.
        args: Vec<Rc<Value>>,
    },
    /// The **Martin-Löf identity eliminator** `walk(p, (x y q). C, (x). c)` —
    /// the full dinatural form, the sole primitive eliminator of
    /// [`crate::types::ValueType::Path`] (ADR-76; rule `Walk`).
    ///
    /// gandr's **first dependent eliminator**. Given a scrutinee `p ⇑ Path A a
    /// b`, a motive `C(x, y, q)` over both endpoints and the path
    /// ([`WalkMotive`]), and a diagonal base `c(x) ⇓ C[x/y][here(x)/q]`
    /// ([`WalkBase`]), it delivers `C[a/x][b/y][p/q]`. Unlike [`Self::Case`]
    /// it is **inference-capable** — the explicit motive is what makes the
    /// result type inferable (`Walk⇑`); a `Walk⇓` is derived by subsumption.
    /// The motive is untraced pure type computation (instantiated by
    /// [`crate::identity`]); the two traced premises are the
    /// scrutinee (value, inferred) and the base body (computation,
    /// checked), so `Walk` traces exactly like `Case`.
    ///
    /// Its sole computation rule is the definitional β on `here`:
    /// `walk(here(v), C, (x). c) ↦ c[v/x]` — realized on the L machine (and, up
    /// to B1 stage F, the retired CEK oracle). A non-`here`,
    /// non-hole scrutinee is [`crate::outcome::StuckReason::WalkOnNonHere`]; a
    /// hole scrutinee blames (the `Case` discipline). No K, no η.
    Walk
    {
        /// The scrutinee `p` (principal premise; inferred to `Path A a b`).
        scrut: Rc<Value>,
        /// The motive `C(x, y, q)` over both endpoints and the path. `Box`ed so
        /// `Comp` (and thus the diagnostic [`Term`] inside
        /// [`crate::error::TypeError`]) stays small, exactly as
        /// [`Self::Perform`] boxes its signature (the large-`Err` lint
        /// threshold).
        motive: Box<WalkMotive>,
        /// The diagonal base `c(x)`.
        base: WalkBase,
    },
    /// **Package elimination** `unpack v : σ as ⟨ā⟩ m in t` — the sole
    /// eliminator of [`crate::types::ValueType::Package`], and the place
    /// abstraction is actually bought.
    ///
    /// The scrutinee is **checked** against the ascribed signature rather than
    /// inferred, which is the decidability fence from the elimination side: a
    /// package is opaque to core-type inference, so no rule reconstructs a
    /// module type from a core term's structure. The signature's abstract type
    /// components are then discharged with the atoms in `atoms` — one fresh
    /// [`crate::types::ValueType::Sealed`] per component — and `m` is
    /// bound to the payload at the resulting type.
    ///
    /// # Minting here is the whole point
    ///
    /// The client meets the payload at **abstract** types, never at the witness
    /// types the packer supplied, because the witnesses were discharged at the
    /// introduction and the elimination substitutes atoms instead. Two unpacks
    /// mint two sets of atoms, so their abstract types do not interchange —
    /// unpacking is generative, exactly as sealing is. [`Self::Force`] mints
    /// nothing and refuses a package outright; that asymmetry is what keeps a
    /// package from being openable as a thunk.
    ///
    /// The atoms are **recorded in the term rather than invented by typing**,
    /// which is the sealing rung's own discipline
    /// (`gandr_core_checker::judgements::seal::SealTable`): the elaborator
    /// mints against a table that refuses a repeated site, records what it
    /// minted, and a reader re-derives and refutes. Typing checks what it
    /// can decide locally — one atom per component, pairwise distinct — and
    /// does not pretend to a freshness property no state-free pass can
    /// establish.
    ///
    /// # Check-only, and the avoidance fence that follows
    ///
    /// The answer type arrives from the outer context and is delivered verbatim
    /// (the [`Self::Case`] discipline; inference is stuck with
    /// [`crate::error::text::UNPACK_NEEDS_CHECK`]). That is not a limitation
    /// worked around but the **avoidance fence**: an expectation formed outside
    /// the unpack cannot mention atoms minted inside it, so no abstract type
    /// can escape its scope, and the checker never has to invent an
    /// avoiding supertype — principal avoiding signatures do not exist in
    /// general, so a checker that tried would be guessing.
    ///
    /// The grade leg is [`Self::Force`]'s: unpacking demands `1 ⊑ r`, so a
    /// `Package_0` may be passed around and never opened.
    Unpack
    {
        /// The package value (checked against `signature`).
        scrut: Rc<Value>,
        /// The ascribed package type — the elimination's own annotation.
        signature: Rc<ValueType>,
        /// The atoms this elimination binds its abstract components to, in
        /// signature order.
        atoms: Vec<SealId>,
        /// The module variable `m`, bound to the payload over `body`.
        binder: String,
        /// The body `t`, checked against the expectation.
        body: Rc<Self>,
    },
    /// The **recursion former** `fix x. t` — one computation former binding one
    /// self-reference, and the sole source of general recursion in the core.
    ///
    /// The pure call-by-push-value fragment is strongly normalizing, so
    /// recursion is an addition rather than a derivation. The self-reference
    /// `x` is bound as a **graded thunk** `U_ω B` over a body at the fixpoint's
    /// own computation type `B`, so every self-use is a
    /// [`Self::Force`] and therefore a machine step. That step is the guard:
    /// each self-use is separated from the definition by a genuine transition,
    /// so there is no infinite regress at a single point.
    ///
    /// The grade is `ω` because a recursive call forces the knot an unbounded
    /// number of times. A grade-`1` self-reference would type only tail and
    /// linear recursion; the rule generalizes to any grade above one without
    /// changing shape, so that refinement is growth rather than a second form.
    ///
    /// **Check-primary** (rule Fix⇓), like the other type-directed introducers:
    /// the self-binding needs `B` in order to state the self-reference's type,
    /// so `B` must arrive from the context rather than be synthesized from the
    /// body. Inference is available only through the ascription coercion, which
    /// is what a recursive definition's declared signature supplies in
    /// practice.
    ///
    /// Its operational rule is unfolding to a fresh thunk of the whole
    /// fixpoint, `fix x. t ⤳ t[thunk_ω (fix x. t) / x]`; the machine realizes
    /// the same rule by **re-entry** — binding the self-reference to a
    /// first-order recursive closure over the fixpoint's own node — so
    /// knot-tying never builds a heap cycle and recursion depth is bounded by
    /// the shared step budget rather than by the host stack.
    ///
    /// **Guardedness here is well-formedness, not termination.**
    /// `fix x. force x` is well-formed and diverges, halting at the budget with
    /// [`crate::outcome::StuckReason::StepLimit`]; termination evidence is a
    /// later rung and is replayed rather than decided by a syntactic pass.
    Fix(
        /// The self-reference binder `x`, bound at `U_ω B` over the body.
        String,
        /// The body `t`, checked against the fixpoint's own type `B`.
        Rc<Self>,
    ),
}

impl Comp
{
    /// Builds an unannotated abstraction `λname. body`.
    #[inline]
    #[must_use]
    pub fn lam<'source, N>(
        name: N,
        body: Self,
    ) -> Self
    where
        N: Into<BinderName<'source>>,
    {
        let name = name.into();
        Self::Abs(name.as_ref().to_owned(), None, Rc::new(body))
    }

    /// Builds an annotated abstraction `λname:ty. body`.
    #[inline]
    #[must_use]
    pub fn lam_ann<'source, N>(
        name: N,
        ty: ValueType,
        body: Self,
    ) -> Self
    where
        N: Into<BinderName<'source>>,
    {
        let name = name.into();
        Self::Abs(name.as_ref().to_owned(), Some(Rc::new(ty)), Rc::new(body))
    }

    /// Builds an application `head arg`.
    #[inline]
    #[must_use]
    pub fn app(
        head: Self,
        arg: Value,
    ) -> Self
    {
        Self::App(Rc::new(head), Rc::new(arg))
    }

    /// Builds a returner `ret value`.
    #[inline]
    #[must_use]
    pub fn ret(value: Value) -> Self
    {
        Self::Ret(Rc::new(value))
    }

    /// Builds a sequencing `bound >>= name. cont`.
    #[inline]
    #[must_use]
    pub fn bind<'source, N>(
        bound: Self,
        name: N,
        cont: Self,
    ) -> Self
    where
        N: Into<BinderName<'source>>,
    {
        let name = name.into();
        Self::Bind(Rc::new(bound), name.as_ref().to_owned(), Rc::new(cont))
    }

    /// Builds a forcing `force value`.
    #[inline]
    #[must_use]
    pub fn force(value: Value) -> Self
    {
        Self::Force(Rc::new(value))
    }

    /// Builds a case analysis over a sum scrutinee.
    #[inline]
    #[must_use]
    pub fn case<'source, F, S>(
        scrut: Value,
        fst_name: F,
        fst_body: Self,
        snd_name: S,
        snd_body: Self,
    ) -> Self
    where
        F: Into<BinderName<'source>>,
        S: Into<BinderName<'source>>,
    {
        let fst_name = fst_name.into();
        let snd_name = snd_name.into();
        Self::Case(
            Rc::new(scrut),
            (fst_name.as_ref().to_owned(), Rc::new(fst_body)),
            (snd_name.as_ref().to_owned(), Rc::new(snd_body)),
        )
    }

    /// Builds a declared-data elimination `case scrut { … }` from its scrutinee
    /// and its per-constructor arms (ADR-80 Decision 3). Arm `i` — a
    /// `(binder, body)` — handles constructor tag `i`.
    #[inline]
    #[must_use]
    pub fn data_case(
        scrut: Value,
        arms: Vec<(String, Self)>,
    ) -> Self
    {
        Self::DataCase(
            Rc::new(scrut),
            arms.into_iter()
                .map(|(binder, body)| (binder, Rc::new(body)))
                .collect(),
        )
    }

    /// Builds a list elimination `case scrut { Nil => nil | Cons(head, tail)
    /// => cons }` (rule `ListCase`, ADR-40 D4).
    #[inline]
    #[must_use]
    pub fn list_case<'source, H, T>(
        scrut: Value,
        nil: Self,
        head: H,
        tail: T,
        cons: Self,
    ) -> Self
    where
        H: Into<BinderName<'source>>,
        T: Into<BinderName<'source>>,
    {
        let head = head.into();
        let tail = tail.into();
        Self::ListCase {
            scrut: Rc::new(scrut),
            nil: Rc::new(nil),
            head: head.as_ref().to_owned(),
            tail: tail.as_ref().to_owned(),
            cons: Rc::new(cons),
        }
    }

    /// Builds a **motive-less** (check-only) product elimination `split scrut
    /// as (fst_name, snd_name) in body` (rule Split⇓, ADR-82).
    #[inline]
    #[must_use]
    pub fn split<'source, F, S>(
        scrut: Value,
        fst_name: F,
        snd_name: S,
        body: Self,
    ) -> Self
    where
        F: Into<BinderName<'source>>,
        S: Into<BinderName<'source>>,
    {
        let fst_name = fst_name.into();
        let snd_name = snd_name.into();
        Self::Split {
            scrut: Rc::new(scrut),
            fst_name: fst_name.as_ref().to_owned(),
            snd_name: snd_name.as_ref().to_owned(),
            motive: None,
            body: Rc::new(body),
        }
    }

    /// Builds a **motive-bearing** (inference-capable) product / dependent-pair
    /// elimination `split scrut as (fst_name, snd_name) [motive] in body` (rule
    /// `SplitMotive`⇑, ADR-82).
    #[inline]
    #[must_use]
    pub fn split_motive<'source, F, S>(
        scrut: Value,
        fst_name: F,
        snd_name: S,
        motive: SplitMotive,
        body: Self,
    ) -> Self
    where
        F: Into<BinderName<'source>>,
        S: Into<BinderName<'source>>,
    {
        let fst_name = fst_name.into();
        let snd_name = snd_name.into();
        Self::Split {
            scrut: Rc::new(scrut),
            fst_name: fst_name.as_ref().to_owned(),
            snd_name: snd_name.as_ref().to_owned(),
            motive: Some(Box::new(motive)),
            body: Rc::new(body),
        }
    }

    /// Builds a record field projection `record.label` (rule `RecordProj`,
    /// ADR-45 D4).
    #[inline]
    #[must_use]
    pub fn record_proj<'source, L>(
        record: Value,
        label: L,
    ) -> Self
    where
        L: Into<FieldName<'source>>,
    {
        let label = label.into();
        Self::RecordProj {
            record: Rc::new(record),
            label: label.as_ref().to_owned(),
        }
    }

    /// Builds a lazy pair `⟨fst, snd⟩`.
    #[inline]
    #[must_use]
    pub fn with(
        fst: Self,
        snd: Self,
    ) -> Self
    {
        Self::With(Rc::new(fst), Rc::new(snd))
    }

    /// Builds a first projection `prj1 target`.
    #[inline]
    #[must_use]
    pub fn prj1(target: Self) -> Self
    {
        Self::Prj(Side::Fst, Rc::new(target))
    }

    /// Builds a second projection `prj2 target`.
    #[inline]
    #[must_use]
    pub fn prj2(target: Self) -> Self
    {
        Self::Prj(Side::Snd, Rc::new(target))
    }

    /// Builds a grade split `dup value` (rule Dup,
    /// `spec:implementation/type-system.md` §"Grades").
    #[inline]
    #[must_use]
    pub fn dup(value: Value) -> Self
    {
        Self::Dup(Rc::new(value))
    }

    /// Builds a grade discard `drop value` (rule Drop,
    /// `spec:implementation/type-system.md` §"Grades").
    #[inline]
    #[must_use]
    pub fn drop(value: Value) -> Self
    {
        Self::Drop(Rc::new(value))
    }

    /// Builds an effect operation `perform op arg` over the signature `sig`
    /// (rule Op, `effects-control-shell.md` §1.1).
    #[inline]
    #[must_use]
    pub fn perform<'source, O>(
        sig: EffectSig,
        op: O,
        arg: Value,
    ) -> Self
    where
        O: Into<OperationName<'source>>,
    {
        let op = op.into();
        Self::Perform(Box::new(sig), op.as_ref().to_owned(), Rc::new(arg))
    }

    /// Builds a deep handler `handle scrutinee { ret ret_var ⇒ ret_body | …ops
    /// }` over the signature `sig` (rule Handle, `effects-control-shell.md`
    /// §1.1).
    #[inline]
    #[must_use]
    pub fn handle<'source, R>(
        sig: EffectSig,
        scrutinee: Self,
        ret_var: R,
        ret_body: Self,
        ops: Vec<OpClause>,
    ) -> Self
    where
        R: Into<BinderName<'source>>,
    {
        let ret_var = ret_var.into();
        Self::Handle {
            sig: Box::new(sig),
            scrutinee: Rc::new(scrutinee),
            ret: (ret_var.as_ref().to_owned(), Rc::new(ret_body)),
            ops,
        }
    }

    /// Builds a stack resumption `resume stack comp` (rule Resume,
    /// `effects-control-shell.md` §2.1).
    #[inline]
    #[must_use]
    pub fn resume(
        stack: Value,
        comp: Self,
    ) -> Self
    {
        Self::Resume(Rc::new(stack), Rc::new(comp))
    }

    /// Builds a delimiter `reset body` (rule Reset, `effects-control-shell.md`
    /// §2.2).
    #[inline]
    #[must_use]
    pub fn reset(body: Self) -> Self
    {
        Self::Reset(Rc::new(body))
    }

    /// Builds a capture `shift k. body` (rule Shift, `effects-control-shell.md`
    /// §2.2).
    #[inline]
    #[must_use]
    pub fn shift<'source, K>(
        k: K,
        body: Self,
    ) -> Self
    where
        K: Into<ContinuationName<'source>>,
    {
        let k = k.into();
        Self::Shift(k.as_ref().to_owned(), Rc::new(body))
    }

    /// Builds a fixpoint `fix x. body`.
    #[inline]
    #[must_use]
    pub fn fix<'source, X>(
        x: X,
        body: Self,
    ) -> Self
    where
        X: Into<BinderName<'source>>,
    {
        let x = x.into();
        Self::Fix(x.as_ref().to_owned(), Rc::new(body))
    }

    /// Builds a typed hole in computation position.
    #[inline]
    #[must_use]
    pub fn hole<I>(id: I) -> Self
    where
        I: Into<SourceIndex>,
    {
        Self::Hole(u32::from(id.into()))
    }

    /// Builds a source-level native builtin computation (argument-free — the
    /// CEK machine accumulates the arguments; see [`Self::Native`]).
    #[inline]
    #[must_use]
    pub fn native(prim: NativePrim) -> Self
    {
        Self::Native {
            prim,
            args: Vec::new(),
        }
    }

    /// Builds an identity elimination `walk(scrut, motive, base)` (ADR-76; rule
    /// `Walk`).
    #[inline]
    #[must_use]
    pub fn walk(
        scrut: Value,
        motive: WalkMotive,
        base: WalkBase,
    ) -> Self
    {
        Self::Walk {
            scrut: Rc::new(scrut),
            motive: Box::new(motive),
            base,
        }
    }

    /// Builds a package elimination `unpack scrut : signature as ⟨atoms⟩ binder
    /// in body` ([`Self::Unpack`]).
    ///
    /// The atoms are positional: atom `i` is what the signature's `i`th
    /// abstract type component is bound to inside `body`.
    #[inline]
    #[must_use]
    pub fn unpack<'source, I, B>(
        scrut: Value,
        signature: ValueType,
        atoms: I,
        binder: B,
        body: Self,
    ) -> Self
    where
        I: IntoIterator<Item = SealId>,
        B: Into<BinderName<'source>>,
    {
        Self::Unpack {
            scrut: Rc::new(scrut),
            signature: Rc::new(signature),
            atoms: atoms.into_iter().collect(),
            binder: binder.into().as_ref().to_owned(),
            body: Rc::new(body),
        }
    }
}

/// A reified stack `K` — Levy's third syntactic sort (evaluation contexts),
/// internalized as the payload of [`Value::Stk`] (`effects-control-shell.md`
/// §2.1; contract §6.3; A3.3 `+control`).
///
/// A stack `K : B ⇒ C` consumes a `B`-computation and delivers a `C`. v0 covers
/// the four **structural** CBPV stack frames — the empty stack and the argument
/// / bind / projection frames — which are exactly the domain of the
/// stack-typing judgment (Levy). The runtime delimiter / handler frames
/// `KReset(C)` / `KHandle(clauses, depth)` of contract §6.3 are A5 CEK-machine
/// artifacts (they carry a runtime `depth` and a live derivation, not a source
/// term), reserved there and **not** constructible in a source-level `stk K`
/// here; they land with the dynamics. The children are `Rc`'d, as
/// the rest of the AST, so a stack clones cheaply into the typing machine's
/// frames.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Stack
{
    /// The empty stack `ε : B ⇒ B` — the identity evaluation context.
    Empty,
    /// An argument frame `v :: K`: consumes a function `A → B` by applying it
    /// to the value `v ⇓ A`, then runs `K` from `B`
    /// (`effects-control-shell.md` §2.1).
    Arg(
        /// The applied argument value `v` (checked against the consumed
        /// function's argument type `A`).
        Rc<Value>,
        /// The rest of the stack, run from the function's result type.
        Rc<Self>,
    ),
    /// A bind frame `(x. u) :: K`: consumes a returner `F^ε A`, binds `x : A`,
    /// runs the continuation `u`, then runs `K` — the stack-resident image of
    /// `>>=`. The consumed row `ε` folds into the continuation's result exactly
    /// as at [`Comp::Bind`] (`effects-control-shell.md` §2.1).
    Bind(
        /// The binder `x`, bound to the consumed returner's payload `A`.
        String,
        /// The continuation `u` (inferred under `x : A`).
        Rc<Comp>,
        /// The rest of the stack, run from the continuation's type.
        Rc<Self>,
    ),
    /// A projection frame `prjᵢ :: K`: consumes a lazy product `B & B′` and
    /// runs `K` from the projected conjunct (`effects-control-shell.md`
    /// §2.1).
    Prj(
        /// Which conjunct is projected.
        Side,
        /// The rest of the stack, run from the projected conjunct.
        Rc<Self>,
    ),
}

impl Stack
{
    /// Builds the empty stack `ε`.
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self::Empty
    }

    /// Builds an argument frame `value :: rest`.
    #[inline]
    #[must_use]
    pub fn arg(
        value: Value,
        rest: Self,
    ) -> Self
    {
        Self::Arg(Rc::new(value), Rc::new(rest))
    }

    /// Builds a bind frame `(name. cont) :: rest`.
    #[inline]
    #[must_use]
    pub fn bind<'source, N>(
        name: N,
        cont: Comp,
        rest: Self,
    ) -> Self
    where
        N: Into<BinderName<'source>>,
    {
        let name = name.into();
        Self::Bind(name.as_ref().to_owned(), Rc::new(cont), Rc::new(rest))
    }

    /// Builds a first-projection frame `prj1 :: rest`.
    #[inline]
    #[must_use]
    pub fn prj1(rest: Self) -> Self
    {
        Self::Prj(Side::Fst, Rc::new(rest))
    }

    /// Builds a second-projection frame `prj2 :: rest`.
    #[inline]
    #[must_use]
    pub fn prj2(rest: Self) -> Self
    {
        Self::Prj(Side::Snd, Rc::new(rest))
    }
}

/// A checked legacy-structure ↔ flat-arena adapter for the ADR-50 carrier.
///
/// This is the explicit bridge while public constructors still build the
/// structural [`Value`] / [`Comp`] / [`Stack`] surface. Conversion allocates
/// fresh canonical carrier nodes for every legacy node it traverses; it never
/// aliases an `Rc` pointer as an arena id.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlatArena
{
    /// Canonical value nodes.
    pub values: ValueArena,
    /// Canonical computation nodes.
    pub comps: CompArena,
    /// Canonical reified-stack nodes.
    pub stacks: StackArena,
    /// Canonical value type nodes.
    pub value_types: ValueTypeArena,
    /// Canonical computation type nodes.
    pub comp_types: CompTypeArena,
}

/// A checked flat-arena bridge failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaBridgeError
{
    /// The target arena has exhausted the `u32` id space.
    IdSpaceExhausted,
    /// A value id did not belong to this bridge arena.
    MissingValue(ValueNodeId),
    /// A computation id did not belong to this bridge arena.
    MissingComp(CompNodeId),
    /// A reified-stack id did not belong to this bridge arena.
    MissingStack(StackNodeId),
    /// A value-type id did not belong to this bridge arena.
    MissingValueType(ValueTypeNodeId),
    /// A computation-type id did not belong to this bridge arena.
    MissingCompType(CompTypeNodeId),
    /// The iterative bridge reached an impossible mixed-sort state.
    TraversalInvariant,
}

/// A sort-tagged borrow of the legacy structural term awaiting flat
/// allocation.
///
/// The [`FlatArena::alloc_legacy`] driver carries one root per pending
/// subtree on its explicit work stack, so a single traversal interleaves
/// every sort without native recursion.
enum LegacyRoot<'legacy>
{
    /// A legacy value type awaiting conversion.
    ValueType(&'legacy ValueType),
    /// A legacy computation type awaiting conversion.
    CompType(&'legacy CompType),
    /// A legacy value awaiting conversion.
    Value(&'legacy Value),
    /// A legacy computation awaiting conversion.
    Comp(&'legacy Comp),
    /// A legacy reified stack awaiting conversion.
    Stack(&'legacy Stack),
}

/// A sort-tagged flat id produced by the iterative allocation traversal.
///
/// Every finished subtree pushes exactly one id onto the result stack; the
/// `pop_alloc_*` helpers peel off the expected sort and treat any other
/// shape as [`ArenaBridgeError::TraversalInvariant`].
enum LegacyRootId
{
    /// An id into the value-type arena.
    ValueType(ValueTypeNodeId),
    /// An id into the computation-type arena.
    CompType(CompTypeNodeId),
    /// An id into the value arena.
    Value(ValueNodeId),
    /// An id into the computation arena.
    Comp(CompNodeId),
    /// An id into the stack arena.
    Stack(StackNodeId),
}

/// One step of the iterative legacy-to-flat allocation traversal.
enum LegacyAllocFrame<'legacy>
{
    /// Descend into a legacy subtree: allocate leaf forms immediately or
    /// schedule the reassembly step plus one `Visit` frame per child.
    Visit(LegacyRoot<'legacy>),
    /// Reassemble a flat node from the converted children on the result
    /// stack.
    Finish(LegacyAllocFinish<'legacy>),
}

/// A pending flat-node assembly for the iterative allocation traversal.
///
/// Each variant records which flat constructor to apply once the children
/// scheduled by the matching `schedule_alloc_*` step have pushed their ids
/// onto the result stack. Borrowed payloads carry the non-id attribute data
/// (labels, grades, signatures, primitive tags) the constructor still needs.
#[derive(Clone, Copy)]
enum LegacyAllocFinish<'legacy>
{
    /// Reassembles a product [`ValueTypeNode::Prod`] from the two converted
    /// component ids.
    ValueTypeProd,
    /// Reassembles a sum [`ValueTypeNode::Sum`] from the two converted
    /// variant ids.
    ValueTypeSum,
    /// Reassembles a list [`ValueTypeNode::List`] from the converted element
    /// type id.
    ValueTypeList,
    /// Reassembles a record [`ValueTypeNode::Record`] from the converted
    /// field type ids, zipped back onto `fields` in label order.
    ValueTypeRecord(&'legacy BTreeMap<String, Rc<ValueType>>),
    /// Reassembles a thunk [`ValueTypeNode::Thunk`] from the grade and the
    /// converted body computation-type id.
    ValueTypeThunk(Grade),
    /// Reassembles a reified-stack [`ValueTypeNode::Stk`] from the converted
    /// consume/deliver computation-type ids.
    ValueTypeStk,
    /// Reassembles an identity [`ValueTypeNode::Path`] from the converted
    /// carrier type id and value endpoint ids.
    ValueTypePath,
    /// Reassembles a declared-data [`ValueTypeNode::Data`] from the converted
    /// type-argument ids.
    ValueTypeData
    {
        /// The datatype's nominal identity.
        id: &'legacy DataId,
        /// The legacy type arguments, mirrored one-for-one by the converted
        /// ids on the result stack.
        args: &'legacy [Rc<ValueType>],
    },
    /// Reassembles a type-family [`ValueTypeNode::Family`] from the head and
    /// the converted argument value ids.
    ValueTypeFamily
    {
        /// The family-kinded head's name.
        head: &'legacy str,
        /// The legacy arguments, mirrored one-for-one by the converted ids on
        /// the result stack.
        args: &'legacy [Rc<Value>],
    },
    /// Reassembles a dependent-pair [`ValueTypeNode::Sigma`] from the binder
    /// name and the converted head/tail type ids.
    ValueTypeSigma(&'legacy str),
    /// Reassembles a package [`ValueTypeNode::Package`] from the grade, the
    /// binder labels, and the converted payload type id.
    ValueTypePackage
    {
        /// The package's usage grade.
        grade: Grade,
        /// The abstract type component labels, in signature order.
        abstracts: &'legacy [String],
    },
    /// Reassembles a returner [`CompTypeNode::F`] from the converted value
    /// type id and the borrowed effect row.
    CompTypeF(&'legacy EffectRow),
    /// Reassembles an arrow [`CompTypeNode::Arrow`] from the borrowed binder,
    /// the converted argument value-type id, and the result
    /// computation-type id.
    CompTypeArrow(Option<&'legacy str>),
    /// Reassembles a with [`CompTypeNode::With`] from the two converted
    /// component computation-type ids.
    CompTypeWith,
    /// Reassembles a pair [`ValueNode::Pair`] from the two converted
    /// component ids.
    ValuePair,
    /// Reassembles an injection [`ValueNode::Inj`] from the side tag and the
    /// converted payload id.
    ValueInj(Side),
    /// Reassembles a list [`ValueNode::List`] from the converted element ids,
    /// zipped back onto the legacy element slice's length.
    ValueList(&'legacy [Rc<Value>]),
    /// Reassembles a record [`ValueNode::Record`] from the converted field
    /// ids, zipped back onto `fields` in label order.
    ValueRecord(&'legacy BTreeMap<String, Rc<Value>>),
    /// Reassembles a pure-computation embedding [`ValueNode::Run`] from the
    /// converted body computation id.
    ValueRun,
    /// Reassembles a thunk [`ValueNode::Thunk`] from the grade and the
    /// converted body computation id.
    ValueThunk(Grade),
    /// Reassembles an annotation [`ValueNode::Annot`] from the converted
    /// value and value-type ids.
    ValueAnnot,
    /// Reassembles a stack reification [`ValueNode::Stk`] from the converted
    /// stack id.
    ValueStk,
    /// Reassembles a path witness [`ValueNode::Here`] from the converted
    /// witness value id.
    ValueHere,
    /// Reassembles a constructor application [`ValueNode::Ctor`] from the
    /// converted payload id.
    ValueCtor
    {
        /// The datatype's nominal identity.
        id: &'legacy DataId,
        /// The constructor's tag within the datatype declaration.
        tag: usize,
    },
    /// Reassembles a packed module [`ValueNode::Pack`] from the converted
    /// witness type ids and payload value id.
    ValuePack
    {
        /// How many witness types the pack carries.
        witnesses: usize,
    },
    /// Reassembles an abstraction [`CompNode::Abs`] from the converted body
    /// id (and the converted annotation id when present).
    CompAbs
    {
        /// The parameter binder name.
        name: &'legacy str,
        /// Whether the abstraction carries a parameter type annotation to
        /// pop.
        has_ty: bool,
    },
    /// Reassembles an application [`CompNode::App`] from the converted head
    /// computation id and argument value id.
    CompApp,
    /// Reassembles a return [`CompNode::Ret`] from the converted value id.
    CompRet,
    /// Reassembles a sequencing [`CompNode::Bind`] from the converted bound
    /// and body computation ids.
    CompBind(&'legacy str),
    /// Reassembles a force [`CompNode::Force`] from the converted thunk value
    /// id.
    CompForce,
    /// Reassembles a pair elimination [`CompNode::Case`] from the converted
    /// scrutinee value id and the two branch computation ids.
    CompCase
    {
        /// The binder for the first component in its branch.
        fst_name: &'legacy str,
        /// The binder for the second component in its branch.
        snd_name: &'legacy str,
    },
    /// Reassembles a list elimination [`CompNode::ListCase`] from the
    /// converted scrutinee value id and the nil/cons branch computation ids.
    CompListCase
    {
        /// The binder for the head element in the cons branch.
        head: &'legacy str,
        /// The binder for the tail list in the cons branch.
        tail: &'legacy str,
    },
    /// Reassembles a product/dependent-pair elimination [`CompNode::Split`]
    /// from the converted scrutinee value id, motive body type id, and body
    /// computation id.
    CompSplit
    {
        /// The binder for the first component `p`.
        fst_name: &'legacy str,
        /// The binder for the second component `q`.
        snd_name: &'legacy str,
        /// The optional dependent motive `(z). M`; `None` is the check-only
        /// motive-less split.
        motive: Option<&'legacy SplitMotive>,
    },
    /// Reassembles a declared-data elimination [`CompNode::DataCase`] from
    /// the converted scrutinee value id and one converted computation id per
    /// arm.
    CompDataCase(&'legacy [(String, Rc<Comp>)]),
    /// Reassembles a record projection [`CompNode::RecordProj`] from the
    /// converted record value id.
    CompRecordProj(&'legacy str),
    /// Reassembles a with-block [`CompNode::With`] from the two converted
    /// component computation ids.
    CompWith,
    /// Reassembles a projection [`CompNode::Prj`] from the side tag and the
    /// converted target computation id.
    CompPrj(Side),
    /// Reassembles a duplication [`CompNode::Dup`] from the converted value
    /// id.
    CompDup,
    /// Reassembles a drop [`CompNode::Drop`] from the converted value id.
    CompDrop,
    /// Reassembles an effect performance [`CompNode::Perform`] from the
    /// converted argument value id.
    CompPerform
    {
        /// The effect signature the operation belongs to.
        sig: &'legacy EffectSig,
        /// The operation name.
        op: &'legacy str,
    },
    /// Reassembles an effect handler [`CompNode::Handle`] from the converted
    /// scrutinee, return-clause, and operation-clause computation ids.
    CompHandle
    {
        /// The effect signature being handled.
        sig: &'legacy EffectSig,
        /// The binder for the returned value in the return clause.
        ret_name: &'legacy str,
        /// The operation clauses, mirrored one-for-one by the converted
        /// clause-body ids on the result stack.
        ops: &'legacy [OpClause],
    },
    /// Reassembles a resumption [`CompNode::Resume`] from the converted
    /// stack value id and fed computation id.
    CompResume,
    /// Reassembles a reset [`CompNode::Reset`] from the converted body
    /// computation id.
    CompReset,
    /// Reassembles a shift [`CompNode::Shift`] from the continuation binder
    /// and the converted body computation id.
    CompShift(&'legacy str),
    /// Reassembles a fixpoint [`CompNode::Fix`] from the self-reference binder
    /// and the converted body computation id.
    CompFix(&'legacy str),
    /// Reassembles a saturated primitive application [`CompNode::Native`]
    /// from the converted argument value ids.
    CompNative
    {
        /// The primitive being applied.
        prim: NativePrim,
        /// The legacy arguments, mirrored one-for-one by the converted ids on
        /// the result stack.
        args: &'legacy [Rc<Value>],
    },
    /// Reassembles an identity elimination [`CompNode::Walk`] from the
    /// converted scrutinee value id, motive body type id, and base
    /// computation id.
    CompWalk
    {
        /// The motive `(x y q). C`.
        motive: &'legacy WalkMotive,
        /// The diagonal base `(x). c`.
        base: &'legacy WalkBase,
    },
    /// Reassembles a package elimination [`CompNode::Unpack`] from the
    /// converted scrutinee value id, signature type id, and body computation
    /// id.
    CompUnpack
    {
        /// The atoms the elimination binds its components to.
        atoms: &'legacy [SealId],
        /// The module variable bound over the body.
        binder: &'legacy str,
    },
    /// Reassembles an argument push [`StackNode::Arg`] from the converted
    /// value id and rest stack id.
    StackArg,
    /// Reassembles a binder push [`StackNode::Bind`] from the converted
    /// continuation computation id and rest stack id.
    StackBind(&'legacy str),
    /// Reassembles a projection pop [`StackNode::Prj`] from the side tag and
    /// the converted rest stack id.
    StackPrj(Side),
}

/// A sort-tagged flat id awaiting structural read-back.
///
/// The [`FlatArena::read_flat`] driver carries one root per pending node on
/// its explicit work stack, so a single traversal interleaves every sort
/// without native recursion.
enum FlatRoot
{
    /// A value-type node id awaiting read-back.
    ValueType(ValueTypeNodeId),
    /// A computation-type node id awaiting read-back.
    CompType(CompTypeNodeId),
    /// A value node id awaiting read-back.
    Value(ValueNodeId),
    /// A computation node id awaiting read-back.
    Comp(CompNodeId),
    /// A stack node id awaiting read-back.
    Stack(StackNodeId),
}

/// A sort-tagged structural term produced by the iterative read-back
/// traversal.
///
/// Every finished node pushes exactly one rebuilt term onto the result
/// stack; the `pop_read_*` helpers peel off the expected sort and treat any
/// other shape as [`ArenaBridgeError::TraversalInvariant`].
enum StructuralRoot
{
    /// A rebuilt legacy value type.
    ValueType(ValueType),
    /// A rebuilt legacy computation type.
    CompType(CompType),
    /// A rebuilt legacy value.
    Value(Value),
    /// A rebuilt legacy computation.
    Comp(Comp),
    /// A rebuilt legacy reified stack.
    Stack(Stack),
}

/// One step of the iterative flat-to-structural read-back traversal.
enum FlatReadFrame<'arena>
{
    /// Look up a flat node: rebuild leaf forms immediately or schedule the
    /// reassembly step plus one `Visit` frame per child id.
    Visit(FlatRoot),
    /// Reassemble a structural term from the read-back children on the
    /// result stack.
    Finish(FlatReadFinish<'arena>),
}

/// A pending structural-term assembly for the iterative read-back
/// traversal.
///
/// Each variant records which structural constructor to apply once the
/// children scheduled by the matching `schedule_read_*` step have pushed
/// their rebuilt terms onto the result stack. Borrowed payloads carry the
/// non-id attribute data (labels, grades, signatures, primitive tags) read
/// out of the flat node and needed by the constructor.
#[derive(Clone, Copy)]
enum FlatReadFinish<'arena>
{
    /// Reassembles a product [`ValueType::Prod`] from the two read-back
    /// component types.
    ValueTypeProd,
    /// Reassembles a sum [`ValueType::Sum`] from the two read-back variant
    /// types.
    ValueTypeSum,
    /// Reassembles a list [`ValueType::List`] from the read-back element
    /// type.
    ValueTypeList,
    /// Reassembles a record [`ValueType::Record`] from the read-back field
    /// types, zipped back onto `fields` in label order.
    ValueTypeRecord(&'arena BTreeMap<String, ValueTypeNodeId>),
    /// Reassembles a thunk [`ValueType::Thunk`] from the grade and the
    /// read-back body computation type.
    ValueTypeThunk(Grade),
    /// Reassembles a reified-stack [`ValueType::Stk`] from the read-back
    /// consume/deliver computation types.
    ValueTypeStk,
    /// Reassembles an identity [`ValueType::Path`] from the read-back carrier
    /// type and value endpoints.
    ValueTypePath,
    /// Reassembles a declared-data [`ValueType::Data`] from the read-back
    /// type arguments.
    ValueTypeData
    {
        /// The datatype's nominal identity.
        id: &'arena DataId,
        /// The flat type-argument ids, mirrored one-for-one by the read-back
        /// types on the result stack.
        args: &'arena [ValueTypeNodeId],
    },
    /// Reassembles a type-family [`ValueType::Family`] from the head and the
    /// read-back argument values.
    ValueTypeFamily
    {
        /// The family-kinded head's name.
        head: &'arena str,
        /// The flat argument ids, mirrored one-for-one by the read-back values
        /// on the result stack.
        args: &'arena [ValueNodeId],
    },
    /// Reassembles a dependent-pair [`ValueType::Sigma`] from the binder name
    /// and the read-back head/tail types.
    ValueTypeSigma(&'arena str),
    /// Reassembles a package [`ValueType::Package`] from the grade, the binder
    /// labels, and the read-back payload type.
    ValueTypePackage
    {
        /// The package's usage grade.
        grade: Grade,
        /// The abstract type component labels, in signature order.
        abstracts: &'arena [String],
    },
    /// Reassembles a returner [`CompType::F`] from the read-back value type
    /// and the borrowed effect row.
    CompTypeF(&'arena EffectRow),
    /// Reassembles an arrow [`CompType::Arrow`] from the borrowed binder, the
    /// read-back argument value type, and the result computation type.
    CompTypeArrow(Option<&'arena str>),
    /// Reassembles a with [`CompType::With`] from the two read-back component
    /// computation types.
    CompTypeWith,
    /// Reassembles a pair [`Value::Pair`] from the two read-back component
    /// values.
    ValuePair,
    /// Reassembles an injection [`Value::Inj`] from the side tag and the
    /// read-back payload value.
    ValueInj(Side),
    /// Reassembles a list [`Value::List`] from the read-back element values,
    /// zipped back onto the flat element slice's length.
    ValueList(&'arena [ValueNodeId]),
    /// Reassembles a record [`Value::Record`] from the read-back field
    /// values, zipped back onto `fields` in label order.
    ValueRecord(&'arena BTreeMap<String, ValueNodeId>),
    /// Reassembles a thunk [`Value::Thunk`] from the grade and the read-back
    /// body computation.
    ValueThunk(Grade),
    /// Reassembles a pure-computation embedding [`Value::Run`] from the
    /// read-back body computation.
    ValueRun,
    /// Reassembles an annotation [`Value::Annot`] from the read-back value
    /// and value type.
    ValueAnnot,
    /// Reassembles a stack reification [`Value::Stk`] from the read-back
    /// stack.
    ValueStk,
    /// Reassembles a path witness [`Value::Here`] from the read-back witness
    /// value.
    ValueHere,
    /// Reassembles a constructor application [`Value::Ctor`] from the
    /// read-back payload value.
    ValueCtor
    {
        /// The datatype's nominal identity.
        id: &'arena DataId,
        /// The constructor's tag within the datatype declaration.
        tag: usize,
    },
    /// Reassembles a packed module [`Value::Pack`] from the read-back witness
    /// types and payload value.
    ValuePack
    {
        /// How many witness types the pack carries.
        witnesses: usize,
    },
    /// Reassembles an abstraction [`Comp::Abs`] from the read-back body (and
    /// the read-back annotation type when present).
    CompAbs
    {
        /// The parameter binder name.
        name: &'arena str,
        /// Whether the abstraction carries a parameter type annotation to
        /// pop.
        has_ty: bool,
    },
    /// Reassembles an application [`Comp::App`] from the read-back head
    /// computation and argument value.
    CompApp,
    /// Reassembles a return [`Comp::Ret`] from the read-back value.
    CompRet,
    /// Reassembles a sequencing [`Comp::Bind`] from the read-back bound and
    /// body computations.
    CompBind(&'arena str),
    /// Reassembles a force [`Comp::Force`] from the read-back thunk value.
    CompForce,
    /// Reassembles a pair elimination [`Comp::Case`] from the read-back
    /// scrutinee value and the two branch computations.
    CompCase
    {
        /// The binder for the first component in its branch.
        fst_name: &'arena str,
        /// The binder for the second component in its branch.
        snd_name: &'arena str,
    },
    /// Reassembles a list elimination [`Comp::ListCase`] from the read-back
    /// scrutinee value and the nil/cons branch computations.
    CompListCase
    {
        /// The binder for the head element in the cons branch.
        head: &'arena str,
        /// The binder for the tail list in the cons branch.
        tail: &'arena str,
    },
    /// Reassembles a product/dependent-pair elimination [`Comp::Split`] from
    /// the read-back scrutinee value, motive body type, and body
    /// computation.
    CompSplit
    {
        /// The binder for the first component `p`.
        fst_name: &'arena str,
        /// The binder for the second component `q`.
        snd_name: &'arena str,
        /// The optional dependent motive `(z). M`; `None` is the check-only
        /// motive-less split.
        motive: Option<&'arena SplitMotiveNode>,
    },
    /// Reassembles a declared-data elimination [`Comp::DataCase`] from the
    /// read-back scrutinee value and one read-back computation per arm.
    CompDataCase(&'arena [(String, CompNodeId)]),
    /// Reassembles a record projection [`Comp::RecordProj`] from the
    /// read-back record value.
    CompRecordProj(&'arena str),
    /// Reassembles a with-block [`Comp::With`] from the two read-back
    /// component computations.
    CompWith,
    /// Reassembles a projection [`Comp::Prj`] from the side tag and the
    /// read-back target computation.
    CompPrj(Side),
    /// Reassembles a duplication [`Comp::Dup`] from the read-back value.
    CompDup,
    /// Reassembles a drop [`Comp::Drop`] from the read-back value.
    CompDrop,
    /// Reassembles an effect performance [`Comp::Perform`] from the read-back
    /// argument value.
    CompPerform
    {
        /// The effect signature the operation belongs to.
        sig: &'arena EffectSig,
        /// The operation name.
        op: &'arena str,
    },
    /// Reassembles an effect handler [`Comp::Handle`] from the read-back
    /// scrutinee, return-clause, and operation-clause computations.
    CompHandle
    {
        /// The effect signature being handled.
        sig: &'arena EffectSig,
        /// The binder for the returned value in the return clause.
        ret_name: &'arena str,
        /// The flat operation clauses, mirrored one-for-one by the read-back
        /// clause bodies on the result stack.
        ops: &'arena [OpClauseNode],
    },
    /// Reassembles a resumption [`Comp::Resume`] from the read-back stack
    /// value and fed computation.
    CompResume,
    /// Reassembles a reset [`Comp::Reset`] from the read-back body
    /// computation.
    CompReset,
    /// Reassembles a shift [`Comp::Shift`] from the continuation binder and
    /// the read-back body computation.
    CompShift(&'arena str),
    /// Reassembles a fixpoint [`Comp::Fix`] from the self-reference binder and
    /// the read-back body computation.
    CompFix(&'arena str),
    /// Reassembles a saturated primitive application [`Comp::Native`] from
    /// the read-back argument values.
    CompNative
    {
        /// The primitive being applied.
        prim: NativePrim,
        /// The flat argument ids, mirrored one-for-one by the read-back
        /// values on the result stack.
        args: &'arena [ValueNodeId],
    },
    /// Reassembles an identity elimination [`Comp::Walk`] from the read-back
    /// scrutinee value, motive body type, and base computation.
    CompWalk
    {
        /// The motive `(x y q). C`.
        motive: &'arena WalkMotiveNode,
        /// The diagonal base `(x). c`.
        base: &'arena WalkBaseNode,
    },
    /// Reassembles a package elimination [`Comp::Unpack`] from the read-back
    /// scrutinee value, signature type, and body computation.
    CompUnpack
    {
        /// The atoms the elimination binds its components to.
        atoms: &'arena [SealId],
        /// The module variable bound over the body.
        binder: &'arena str,
    },
    /// Reassembles an argument push [`Stack::Arg`] from the read-back value
    /// and rest stack.
    StackArg,
    /// Reassembles a binder push [`Stack::Bind`] from the read-back
    /// continuation computation and rest stack.
    StackBind(&'arena str),
    /// Reassembles a projection pop [`Stack::Prj`] from the side tag and the
    /// read-back rest stack.
    StackPrj(Side),
}

/// Pops a computation-type id off the allocation result stack.
///
/// # Contract
/// - ensures: returns the id pushed by the most recently finished
///   computation-type subtree.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_alloc_comp_type(results: &mut Vec<LegacyRootId>)
-> Result<CompTypeNodeId, ArenaBridgeError>
{
    match results.pop() {
        | Some(LegacyRootId::CompType(id)) => Ok(id),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops a stack id off the allocation result stack.
///
/// # Contract
/// - ensures: returns the id pushed by the most recently finished stack
///   subtree.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_alloc_stack(results: &mut Vec<LegacyRootId>) -> Result<StackNodeId, ArenaBridgeError>
{
    match results.pop() {
        | Some(LegacyRootId::Stack(id)) => Ok(id),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` value-type ids off the allocation result stack, restoring
/// source order.
///
/// # Contract
/// - ensures: returns the ids of the `count` most recently finished value-type
///   subtrees in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_alloc_value_types(
    results: &mut Vec<LegacyRootId>,
    count: ArenaLength,
) -> Result<Vec<ValueTypeNodeId>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut ids = Vec::with_capacity(count);
    for _ in 0 .. count {
        let id = pop_alloc_value_type(results)?;
        ids.push(id);
    }
    ids.reverse();
    Ok(ids)
}

/// Pops a value-type id off the allocation result stack.
///
/// # Contract
/// - ensures: returns the id pushed by the most recently finished value-type
///   subtree.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_alloc_value_type(
    results: &mut Vec<LegacyRootId>
) -> Result<ValueTypeNodeId, ArenaBridgeError>
{
    match results.pop() {
        | Some(LegacyRootId::ValueType(id)) => Ok(id),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` value ids off the allocation result stack, restoring source
/// order.
///
/// # Contract
/// - ensures: returns the ids of the `count` most recently finished value
///   subtrees in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_alloc_values(
    results: &mut Vec<LegacyRootId>,
    count: ArenaLength,
) -> Result<Vec<ValueNodeId>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut ids = Vec::with_capacity(count);
    for _ in 0 .. count {
        let id = pop_alloc_value(results)?;
        ids.push(id);
    }
    ids.reverse();
    Ok(ids)
}

/// Pops a value id off the allocation result stack.
///
/// # Contract
/// - ensures: returns the id pushed by the most recently finished value
///   subtree.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_alloc_value(results: &mut Vec<LegacyRootId>) -> Result<ValueNodeId, ArenaBridgeError>
{
    match results.pop() {
        | Some(LegacyRootId::Value(id)) => Ok(id),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` computation ids off the allocation result stack, restoring
/// source order.
///
/// # Contract
/// - ensures: returns the ids of the `count` most recently finished computation
///   subtrees in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_alloc_comps(
    results: &mut Vec<LegacyRootId>,
    count: ArenaLength,
) -> Result<Vec<CompNodeId>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut ids = Vec::with_capacity(count);
    for _ in 0 .. count {
        let id = pop_alloc_comp(results)?;
        ids.push(id);
    }
    ids.reverse();
    Ok(ids)
}

/// Pops a computation id off the allocation result stack.
///
/// # Contract
/// - ensures: returns the id pushed by the most recently finished computation
///   subtree.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_alloc_comp(results: &mut Vec<LegacyRootId>) -> Result<CompNodeId, ArenaBridgeError>
{
    match results.pop() {
        | Some(LegacyRootId::Comp(id)) => Ok(id),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops a computation type off the read-back result stack.
///
/// # Contract
/// - ensures: returns the term rebuilt by the most recently finished
///   computation-type node.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_read_comp_type(results: &mut Vec<StructuralRoot>) -> Result<CompType, ArenaBridgeError>
{
    match results.pop() {
        | Some(StructuralRoot::CompType(ty)) => Ok(ty),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops a reified stack off the read-back result stack.
///
/// # Contract
/// - ensures: returns the term rebuilt by the most recently finished stack
///   node.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_read_stack(results: &mut Vec<StructuralRoot>) -> Result<Stack, ArenaBridgeError>
{
    match results.pop() {
        | Some(StructuralRoot::Stack(stack)) => Ok(stack),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` value types off the read-back result stack, restoring source
/// order.
///
/// # Contract
/// - ensures: returns the terms rebuilt by the `count` most recently finished
///   value-type nodes in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_read_value_types(
    results: &mut Vec<StructuralRoot>,
    count: ArenaLength,
) -> Result<Vec<ValueType>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut values = Vec::with_capacity(count);
    for _ in 0 .. count {
        let value = pop_read_value_type(results)?;
        values.push(value);
    }
    values.reverse();
    Ok(values)
}

/// Pops a value type off the read-back result stack.
///
/// # Contract
/// - ensures: returns the term rebuilt by the most recently finished value-type
///   node.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_read_value_type(results: &mut Vec<StructuralRoot>) -> Result<ValueType, ArenaBridgeError>
{
    match results.pop() {
        | Some(StructuralRoot::ValueType(ty)) => Ok(ty),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` values off the read-back result stack, restoring source
/// order.
///
/// # Contract
/// - ensures: returns the terms rebuilt by the `count` most recently finished
///   value nodes in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_read_values(
    results: &mut Vec<StructuralRoot>,
    count: ArenaLength,
) -> Result<Vec<Value>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut values = Vec::with_capacity(count);
    for _ in 0 .. count {
        let value = pop_read_value(results)?;
        values.push(value);
    }
    values.reverse();
    Ok(values)
}

/// Pops a value off the read-back result stack.
///
/// # Contract
/// - ensures: returns the term rebuilt by the most recently finished value
///   node.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_read_value(results: &mut Vec<StructuralRoot>) -> Result<Value, ArenaBridgeError>
{
    match results.pop() {
        | Some(StructuralRoot::Value(value)) => Ok(value),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

/// Pops `count` computations off the read-back result stack, restoring
/// source order.
///
/// # Contract
/// - ensures: returns the terms rebuilt by the `count` most recently finished
///   computation nodes in their original child order.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when any popped entry has
///   another sort — the traversal's schedule invariant was violated.
/// - panics: none.
fn pop_read_comps(
    results: &mut Vec<StructuralRoot>,
    count: ArenaLength,
) -> Result<Vec<Comp>, ArenaBridgeError>
{
    let count = usize::from(count);
    let mut comps = Vec::with_capacity(count);
    for _ in 0 .. count {
        let comp = pop_read_comp(results)?;
        comps.push(comp);
    }
    comps.reverse();
    Ok(comps)
}

/// Pops a computation off the read-back result stack.
///
/// # Contract
/// - ensures: returns the term rebuilt by the most recently finished
///   computation node.
/// - errors: [`ArenaBridgeError::TraversalInvariant`] when the stack is empty
///   or its top has another sort — the traversal's schedule invariant was
///   violated.
/// - panics: none.
fn pop_read_comp(results: &mut Vec<StructuralRoot>) -> Result<Comp, ArenaBridgeError>
{
    match results.pop() {
        | Some(StructuralRoot::Comp(comp)) => Ok(comp),
        | _ => Err(ArenaBridgeError::TraversalInvariant),
    }
}

impl FlatArena
{
    /// Creates an empty bridge arena set.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Converts a legacy structural root into canonical flat nodes without
    /// native recursion.
    ///
    /// Drives an explicit work stack of [`LegacyAllocFrame`] steps: `Visit`
    /// descends into a subtree and schedules its children, `Finish`
    /// reassembles the flat node from the converted children on the result
    /// stack.
    ///
    /// # Contract
    /// - ensures: returns the sort-tagged id of a freshly allocated flat graph
    ///   equivalent to `root`, with every reachable child allocated in the
    ///   matching arena.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when any arena would
    ///   exceed its id space; [`ArenaBridgeError::TraversalInvariant`] when the
    ///   result-stack bookkeeping no longer matches the schedule.
    /// - panics: none.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: legacy terms and referenced maps/vectors are finite Rust
    ///   data.
    /// - input recursion: none.
    fn alloc_legacy(
        &mut self,
        root: LegacyRoot<'_>,
    ) -> Result<LegacyRootId, ArenaBridgeError>
    {
        let mut work = alloc::vec![LegacyAllocFrame::Visit(root)];
        let mut results = Vec::new();
        while let Some(frame) = work.pop() {
            match frame {
                | LegacyAllocFrame::Visit(pending) => match pending {
                    | LegacyRoot::ValueType(ty) => {
                        self.schedule_alloc_value_type(ty, &mut work, &mut results)?;
                    },
                    | LegacyRoot::CompType(ty) => {
                        self.schedule_alloc_comp_type(ty, &mut work, &mut results)?;
                    },
                    | LegacyRoot::Value(value) => {
                        self.schedule_alloc_value(value, &mut work, &mut results)?;
                    },
                    | LegacyRoot::Comp(comp) => {
                        self.schedule_alloc_comp(comp, &mut work, &mut results)?;
                    },
                    | LegacyRoot::Stack(stack) => {
                        self.schedule_alloc_stack(stack, &mut work, &mut results)?;
                    },
                },
                | LegacyAllocFrame::Finish(finish) => self.finish_alloc(&finish, &mut results)?,
            }
        }
        let root_id = results.pop().ok_or(ArenaBridgeError::TraversalInvariant)?;
        if results.is_empty() {
            Ok(root_id)
        }
        else {
            Err(ArenaBridgeError::TraversalInvariant)
        }
    }

    /// Schedules the flat allocation of one legacy value type.
    ///
    /// Leaf forms allocate immediately and push their id onto `results`;
    /// recursive forms push a [`LegacyAllocFinish`] reassembly step plus one
    /// [`LegacyAllocFrame::Visit`] frame per child (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   flat node graph the recursive conversion of `ty` would allocate.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the value-type
    ///   arena would exceed its id space.
    /// - panics: none.
    fn schedule_alloc_value_type<'legacy>(
        &mut self,
        ty: &'legacy ValueType,
        work: &mut Vec<LegacyAllocFrame<'legacy>>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *ty {
            | ValueType::Atom(ref name) => {
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Atom(name.clone()))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | ValueType::Unit => {
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Unit)
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | ValueType::Prod(ref fst, ref snd) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeProd));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(snd.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(fst.as_ref())));
            },
            | ValueType::Sum(ref lhs, ref rhs) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeSum));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(rhs.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(lhs.as_ref())));
            },
            | ValueType::List(ref element) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeList));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                    element.as_ref(),
                )));
            },
            | ValueType::Record(ref fields) => {
                work.push(LegacyAllocFrame::Finish(
                    LegacyAllocFinish::ValueTypeRecord(fields),
                ));
                for (_, field) in fields.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                        field.as_ref(),
                    )));
                }
            },
            | ValueType::Thunk(grade, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeThunk(
                    grade,
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(body.as_ref())));
            },
            | ValueType::Stk(ref consumes, ref delivers) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeStk));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(
                    delivers.as_ref(),
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(
                    consumes.as_ref(),
                )));
            },
            | ValueType::Path {
                ty: ref carrier,
                ref lhs,
                ref rhs,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypePath));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(rhs.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(lhs.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                    carrier.as_ref(),
                )));
            },
            | ValueType::Family { ref head, ref args } => {
                work.push(LegacyAllocFrame::Finish(
                    LegacyAllocFinish::ValueTypeFamily {
                        head: head.as_str(),
                        args,
                    },
                ));
                for arg in args.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(arg.as_ref())));
                }
            },
            | ValueType::Data { ref id, ref args } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeData {
                    id,
                    args,
                }));
                for arg in args.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(arg.as_ref())));
                }
            },
            | ValueType::Universe {
                ref sort,
                ref level,
            } => {
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Universe {
                        sort: sort.clone(),
                        level: level.clone(),
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | ValueType::Sealed(ref seal) => {
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Sealed(seal.clone()))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | ValueType::Sigma {
                ref fst,
                ref binder,
                ref snd,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueTypeSigma(
                    binder,
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(snd.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(fst.as_ref())));
            },
            | ValueType::Package {
                grade,
                ref abstracts,
                ref payload,
            } => {
                work.push(LegacyAllocFrame::Finish(
                    LegacyAllocFinish::ValueTypePackage { grade, abstracts },
                ));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                    payload.as_ref(),
                )));
            },
            | ValueType::Unknown => {
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Unknown)
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
        }
        Ok(())
    }

    /// Schedules the flat allocation of one legacy computation type.
    ///
    /// Leaf forms allocate immediately and push their id onto `results`;
    /// recursive forms push a [`LegacyAllocFinish`] reassembly step plus one
    /// [`LegacyAllocFrame::Visit`] frame per child (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   flat node graph the recursive conversion of `ty` would allocate.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the
    ///   computation-type arena would exceed its id space.
    /// - panics: none.
    fn schedule_alloc_comp_type<'legacy>(
        &mut self,
        ty: &'legacy CompType,
        work: &mut Vec<LegacyAllocFrame<'legacy>>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *ty {
            | CompType::F(ref of, ref row) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompTypeF(row)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(of.as_ref())));
            },
            | CompType::Arrow {
                ref binder,
                ref arg,
                ref res,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompTypeArrow(
                    binder.as_deref(),
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(res.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(arg.as_ref())));
            },
            | CompType::With(ref fst, ref snd) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompTypeWith));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(snd.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(fst.as_ref())));
            },
            | CompType::Unknown => {
                let id = self
                    .comp_types
                    .alloc(CompTypeNode::Unknown)
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::CompType(id));
            },
        }
        Ok(())
    }

    /// Schedules the flat allocation of one legacy value.
    ///
    /// Leaf forms allocate immediately and push their id onto `results`;
    /// recursive forms push a [`LegacyAllocFinish`] reassembly step plus one
    /// [`LegacyAllocFrame::Visit`] frame per child (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   flat node graph the recursive conversion of `value` would allocate.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the value arena
    ///   would exceed its id space.
    /// - panics: none.
    fn schedule_alloc_value<'legacy>(
        &mut self,
        value: &'legacy Value,
        work: &mut Vec<LegacyAllocFrame<'legacy>>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *value {
            | Value::Var(ref name) => {
                let id = self
                    .values
                    .alloc(ValueNode::Var(name.clone()))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Unit => {
                let id = self
                    .values
                    .alloc(ValueNode::Unit)
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Int(literal) => {
                let id = self
                    .values
                    .alloc(ValueNode::Int(literal))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Str(ref literal) => {
                let id = self
                    .values
                    .alloc(ValueNode::Str(literal.clone()))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Num(literal) => {
                let id = self
                    .values
                    .alloc(ValueNode::Num(literal))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Pair(ref fst, ref snd) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValuePair));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(snd.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(fst.as_ref())));
            },
            | Value::Inj(side, ref payload) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueInj(side)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(payload.as_ref())));
            },
            | Value::List(ref elements) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueList(
                    elements,
                )));
                for element in elements.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(element.as_ref())));
                }
            },
            | Value::Record(ref fields) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueRecord(
                    fields,
                )));
                for (_, field) in fields.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(field.as_ref())));
                }
            },
            | Value::Thunk(grade, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueThunk(
                    grade,
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
            },
            | Value::Run(ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueRun));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
            },
            | Value::Annot(ref inner, ref ty) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueAnnot));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(ty.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(inner.as_ref())));
            },
            | Value::Hole(id) => {
                let id = self
                    .values
                    .alloc(ValueNode::Hole(id))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | Value::Stk(ref stack) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueStk));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Stack(stack.as_ref())));
            },
            | Value::Here(ref witness) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueHere));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(witness.as_ref())));
            },
            | Value::Ctor {
                ref id,
                tag,
                ref payload,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValueCtor {
                    id,
                    tag,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(payload.as_ref())));
            },
            | Value::Pack {
                ref witnesses,
                ref payload,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::ValuePack {
                    witnesses: witnesses.len(),
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(payload.as_ref())));
                for witness in witnesses.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                        witness.as_ref(),
                    )));
                }
            },
        }
        Ok(())
    }

    /// Schedules the flat allocation of one legacy computation.
    ///
    /// Leaf forms allocate immediately and push their id onto `results`;
    /// recursive forms push a [`LegacyAllocFinish`] reassembly step plus one
    /// [`LegacyAllocFrame::Visit`] frame per child (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   flat node graph the recursive conversion of `comp` would allocate.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the computation
    ///   arena would exceed its id space.
    /// - panics: none.
    fn schedule_alloc_comp<'legacy>(
        &mut self,
        comp: &'legacy Comp,
        work: &mut Vec<LegacyAllocFrame<'legacy>>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *comp {
            | Comp::Abs(ref name, ref ty, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompAbs {
                    name,
                    has_ty: ty.is_some(),
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
                if let Some(ty) = ty.as_ref() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(ty.as_ref())));
                }
            },
            | Comp::App(ref head, ref arg) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompApp));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(arg.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(head.as_ref())));
            },
            | Comp::Ret(ref value) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompRet));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(value.as_ref())));
            },
            | Comp::Bind(ref bound, ref name, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompBind(name)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(bound.as_ref())));
            },
            | Comp::Force(ref value) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompForce));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(value.as_ref())));
            },
            | Comp::Case(ref scrut, ref fst, ref snd) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompCase {
                    fst_name: &fst.0,
                    snd_name: &snd.0,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(snd.1.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(fst.1.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
            | Comp::ListCase {
                ref scrut,
                ref nil,
                ref head,
                ref tail,
                ref cons,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompListCase {
                    head,
                    tail,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(cons.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(nil.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
            | Comp::Split {
                ref scrut,
                ref fst_name,
                ref snd_name,
                ref motive,
                ref body,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompSplit {
                    fst_name,
                    snd_name,
                    motive: motive.as_deref(),
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
                if let Some(motive) = motive.as_ref() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(
                        motive.body.as_ref(),
                    )));
                }
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
            | Comp::DataCase(ref scrut, ref arms) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompDataCase(
                    arms,
                )));
                for arm in arms.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(arm.1.as_ref())));
                }
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
            | Comp::RecordProj {
                ref record,
                ref label,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompRecordProj(
                    label,
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(record.as_ref())));
            },
            | Comp::With(ref fst, ref snd) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompWith));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(snd.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(fst.as_ref())));
            },
            | Comp::Prj(side, ref target) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompPrj(side)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(target.as_ref())));
            },
            | Comp::Dup(ref value) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompDup));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(value.as_ref())));
            },
            | Comp::Drop(ref value) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompDrop));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(value.as_ref())));
            },
            | Comp::Perform(ref sig, ref op, ref arg) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompPerform {
                    sig: sig.as_ref(),
                    op,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(arg.as_ref())));
            },
            | Comp::Handle {
                ref sig,
                ref scrutinee,
                ref ret,
                ref ops,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompHandle {
                    sig: sig.as_ref(),
                    ret_name: &ret.0,
                    ops,
                }));
                for clause in ops.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(
                        clause.body.as_ref(),
                    )));
                }
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(ret.1.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(
                    scrutinee.as_ref(),
                )));
            },
            | Comp::Resume(ref stack, ref fed) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompResume));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(fed.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(stack.as_ref())));
            },
            | Comp::Reset(ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompReset));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
            },
            | Comp::Shift(ref k, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompShift(k)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
            },
            | Comp::Fix(ref x, ref body) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompFix(x)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
            },
            | Comp::Hole(id) => {
                let id = self
                    .comps
                    .alloc(CompNode::Hole(id))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | Comp::Native { prim, ref args } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompNative {
                    prim,
                    args,
                }));
                for arg in args.iter().rev() {
                    work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(arg.as_ref())));
                }
            },
            | Comp::Walk {
                ref scrut,
                ref motive,
                ref base,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompWalk {
                    motive,
                    base,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(
                    base.body.as_ref(),
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::CompType(
                    motive.body.as_ref(),
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
            | Comp::Unpack {
                ref scrut,
                ref signature,
                ref atoms,
                ref binder,
                ref body,
            } => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::CompUnpack {
                    atoms,
                    binder,
                }));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(body.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::ValueType(
                    signature.as_ref(),
                )));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(scrut.as_ref())));
            },
        }
        Ok(())
    }

    /// Schedules the flat allocation of one legacy reified stack.
    ///
    /// Leaf forms allocate immediately and push their id onto `results`;
    /// recursive forms push a [`LegacyAllocFinish`] reassembly step plus one
    /// [`LegacyAllocFrame::Visit`] frame per child (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   flat node graph the recursive conversion of `stack` would allocate.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the stack arena
    ///   would exceed its id space.
    /// - panics: none.
    fn schedule_alloc_stack<'legacy>(
        &mut self,
        stack: &'legacy Stack,
        work: &mut Vec<LegacyAllocFrame<'legacy>>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *stack {
            | Stack::Empty => {
                let id = self
                    .stacks
                    .alloc(StackNode::Empty)
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Stack(id));
            },
            | Stack::Arg(ref value, ref rest) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::StackArg));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Stack(rest.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Value(value.as_ref())));
            },
            | Stack::Bind(ref name, ref cont, ref rest) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::StackBind(name)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Stack(rest.as_ref())));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Comp(cont.as_ref())));
            },
            | Stack::Prj(side, ref rest) => {
                work.push(LegacyAllocFrame::Finish(LegacyAllocFinish::StackPrj(side)));
                work.push(LegacyAllocFrame::Visit(LegacyRoot::Stack(rest.as_ref())));
            },
        }
        Ok(())
    }

    /// Reassembles one flat node from its converted children.
    ///
    /// Pops the child ids reserved by the matching `schedule_alloc_*` step
    /// off `results`, allocates the node recorded by `finish`, and pushes the
    /// new id back.
    ///
    /// # Contract
    /// - ensures: `results` trades the node's children for the freshly
    ///   allocated node id of the same sort.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when the target arena
    ///   would exceed its id space; [`ArenaBridgeError::TraversalInvariant`]
    ///   when a child id has the wrong sort.
    /// - panics: none.
    fn finish_alloc(
        &mut self,
        finish: &LegacyAllocFinish<'_>,
        results: &mut Vec<LegacyRootId>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *finish {
            | LegacyAllocFinish::ValueTypeProd => {
                let snd = pop_alloc_value_type(results)?;
                let fst = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Prod(fst, snd))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeSum => {
                let rhs = pop_alloc_value_type(results)?;
                let lhs = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Sum(lhs, rhs))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeList => {
                let element = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::List(element))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeRecord(fields) => {
                let field_ids = pop_alloc_value_types(results, fields.len().into())?;
                let mut flat = BTreeMap::new();
                for ((label, _), field) in fields.iter().zip(field_ids) {
                    flat.insert(label.clone(), field);
                }
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Record(flat))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeThunk(grade) => {
                let body = pop_alloc_comp_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Thunk(grade, body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeStk => {
                let delivers = pop_alloc_comp_type(results)?;
                let consumes = pop_alloc_comp_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Stk(consumes, delivers))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypePath => {
                let rhs = pop_alloc_value(results)?;
                let lhs = pop_alloc_value(results)?;
                let ty = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Path { ty, lhs, rhs })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeFamily { head, args } => {
                let arg_ids = pop_alloc_values(results, args.len().into())?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Family {
                        head: head.to_owned(),
                        args: arg_ids,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypeData { id: data_id, args } => {
                let arg_ids = pop_alloc_value_types(results, args.len().into())?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Data {
                        id: data_id.clone(),
                        args: arg_ids,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValueTypePackage { grade, abstracts } => {
                let payload = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Package {
                        grade,
                        abstracts: abstracts.to_vec(),
                        payload,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::ValuePack { witnesses } => {
                let payload = pop_alloc_value(results)?;
                let witness_ids = pop_alloc_value_types(results, witnesses.into())?;
                let id = self
                    .values
                    .alloc(ValueNode::Pack {
                        witnesses: witness_ids,
                        payload,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::CompUnpack { atoms, binder } => {
                let body = pop_alloc_comp(results)?;
                let signature = pop_alloc_value_type(results)?;
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Unpack {
                        scrut,
                        signature,
                        atoms: atoms.to_vec(),
                        binder: binder.to_owned(),
                        body,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::ValueTypeSigma(binder) => {
                let snd = pop_alloc_value_type(results)?;
                let fst = pop_alloc_value_type(results)?;
                let id = self
                    .value_types
                    .alloc(ValueTypeNode::Sigma {
                        fst,
                        binder: binder.to_owned(),
                        snd,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::ValueType(id));
            },
            | LegacyAllocFinish::CompTypeF(row) => {
                let of = pop_alloc_value_type(results)?;
                let id = self
                    .comp_types
                    .alloc(CompTypeNode::F(of, row.clone()))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::CompType(id));
            },
            | LegacyAllocFinish::CompTypeArrow(binder) => {
                let res = pop_alloc_comp_type(results)?;
                let arg = pop_alloc_value_type(results)?;
                let id = self
                    .comp_types
                    .alloc(CompTypeNode::Arrow {
                        binder: binder.map(str::to_owned),
                        arg,
                        res,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::CompType(id));
            },
            | LegacyAllocFinish::CompTypeWith => {
                let snd = pop_alloc_comp_type(results)?;
                let fst = pop_alloc_comp_type(results)?;
                let id = self
                    .comp_types
                    .alloc(CompTypeNode::With(fst, snd))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::CompType(id));
            },
            | LegacyAllocFinish::ValuePair => {
                let snd = pop_alloc_value(results)?;
                let fst = pop_alloc_value(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Pair(fst, snd))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueInj(side) => {
                let payload = pop_alloc_value(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Inj(side, payload))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueList(elements) => {
                let element_ids = pop_alloc_values(results, elements.len().into())?;
                let id = self
                    .values
                    .alloc(ValueNode::List(element_ids))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueRecord(fields) => {
                let field_ids = pop_alloc_values(results, fields.len().into())?;
                let mut flat = BTreeMap::new();
                for ((label, _), field) in fields.iter().zip(field_ids) {
                    flat.insert(label.clone(), field);
                }
                let id = self
                    .values
                    .alloc(ValueNode::Record(flat))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueThunk(grade) => {
                let body = pop_alloc_comp(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Thunk(grade, body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueRun => {
                let body = pop_alloc_comp(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Run(body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueAnnot => {
                let ty = pop_alloc_value_type(results)?;
                let inner = pop_alloc_value(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Annot(inner, ty))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueStk => {
                let stack = pop_alloc_stack(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Stk(stack))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueHere => {
                let witness = pop_alloc_value(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Here(witness))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::ValueCtor { id: data_id, tag } => {
                let payload = pop_alloc_value(results)?;
                let id = self
                    .values
                    .alloc(ValueNode::Ctor {
                        id: data_id.clone(),
                        tag,
                        payload,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Value(id));
            },
            | LegacyAllocFinish::CompAbs { name, has_ty } => {
                let body = pop_alloc_comp(results)?;
                let ty = if has_ty {
                    let ty = pop_alloc_value_type(results)?;
                    Some(ty)
                }
                else {
                    None
                };
                let id = self
                    .comps
                    .alloc(CompNode::Abs(name.to_owned(), ty, body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompApp => {
                let arg = pop_alloc_value(results)?;
                let head = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::App(head, arg))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompRet => {
                let value = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Ret(value))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompBind(name) => {
                let body = pop_alloc_comp(results)?;
                let bound = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Bind(bound, name.to_owned(), body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompForce => {
                let value = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Force(value))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompCase { fst_name, snd_name } => {
                let snd_body = pop_alloc_comp(results)?;
                let fst_body = pop_alloc_comp(results)?;
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Case(
                        scrut,
                        (fst_name.to_owned(), fst_body),
                        (snd_name.to_owned(), snd_body),
                    ))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompListCase { head, tail } => {
                let cons = pop_alloc_comp(results)?;
                let nil = pop_alloc_comp(results)?;
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::ListCase {
                        scrut,
                        nil,
                        head: head.to_owned(),
                        tail: tail.to_owned(),
                        cons,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompSplit {
                fst_name,
                snd_name,
                motive,
            } => {
                let body = pop_alloc_comp(results)?;
                let motive_node = if let Some(motive) = motive {
                    let motive_body = pop_alloc_comp_type(results)?;
                    Some(SplitMotiveNode {
                        binder: motive.binder.clone(),
                        body: motive_body,
                    })
                }
                else {
                    None
                };
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Split {
                        scrut,
                        fst_name: fst_name.to_owned(),
                        snd_name: snd_name.to_owned(),
                        motive: motive_node,
                        body,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompDataCase(arms) => {
                let arm_ids = pop_alloc_comps(results, arms.len().into())?;
                let mut flat_arms = Vec::with_capacity(arms.len());
                for (arm, body) in arms.iter().zip(arm_ids) {
                    flat_arms.push((arm.0.clone(), body));
                }
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::DataCase {
                        scrut,
                        arms: flat_arms,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompRecordProj(label) => {
                let record = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::RecordProj {
                        record,
                        label: label.to_owned(),
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompWith => {
                let snd = pop_alloc_comp(results)?;
                let fst = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::With(fst, snd))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompPrj(side) => {
                let target = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Prj(side, target))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompDup => {
                let value = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Dup(value))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompDrop => {
                let value = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Drop(value))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompPerform { sig, op } => {
                let arg = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Perform(Box::new(sig.clone()), op.to_owned(), arg))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompHandle { sig, ret_name, ops } => {
                let op_bodies = pop_alloc_comps(results, ops.len().into())?;
                let ret_body = pop_alloc_comp(results)?;
                let scrutinee = pop_alloc_comp(results)?;
                let mut flat_ops = Vec::with_capacity(ops.len());
                for (clause, body) in ops.iter().zip(op_bodies) {
                    flat_ops.push(OpClauseNode {
                        op: clause.op.clone(),
                        payload: clause.payload.clone(),
                        resume: clause.resume.clone(),
                        body,
                    });
                }
                let id = self
                    .comps
                    .alloc(CompNode::Handle {
                        sig: Box::new(sig.clone()),
                        scrutinee,
                        ret: (ret_name.to_owned(), ret_body),
                        ops: flat_ops,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompResume => {
                let fed = pop_alloc_comp(results)?;
                let stack = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Resume(stack, fed))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompReset => {
                let body = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Reset(body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompShift(k) => {
                let body = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Shift(k.to_owned(), body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompFix(x) => {
                let body = pop_alloc_comp(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Fix(x.to_owned(), body))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompNative { prim, args } => {
                let arg_ids = pop_alloc_values(results, args.len().into())?;
                let id = self
                    .comps
                    .alloc(CompNode::Native {
                        prim,
                        args: arg_ids,
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::CompWalk { motive, base } => {
                let base_body = pop_alloc_comp(results)?;
                let motive_body = pop_alloc_comp_type(results)?;
                let scrut = pop_alloc_value(results)?;
                let id = self
                    .comps
                    .alloc(CompNode::Walk {
                        scrut,
                        motive: WalkMotiveNode {
                            x: motive.x.clone(),
                            y: motive.y.clone(),
                            q: motive.q.clone(),
                            body: motive_body,
                        },
                        base: WalkBaseNode {
                            x: base.x.clone(),
                            body: base_body,
                        },
                    })
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Comp(id));
            },
            | LegacyAllocFinish::StackArg => {
                let rest = pop_alloc_stack(results)?;
                let value = pop_alloc_value(results)?;
                let id = self
                    .stacks
                    .alloc(StackNode::Arg(value, rest))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Stack(id));
            },
            | LegacyAllocFinish::StackBind(name) => {
                let rest = pop_alloc_stack(results)?;
                let cont = pop_alloc_comp(results)?;
                let id = self
                    .stacks
                    .alloc(StackNode::Bind(name.to_owned(), cont, rest))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Stack(id));
            },
            | LegacyAllocFinish::StackPrj(side) => {
                let rest = pop_alloc_stack(results)?;
                let id = self
                    .stacks
                    .alloc(StackNode::Prj(side, rest))
                    .ok_or(ArenaBridgeError::IdSpaceExhausted)?;
                results.push(LegacyRootId::Stack(id));
            },
        }
        Ok(())
    }

    /// Reads a flat root id back to the legacy structural surface without
    /// native recursion.
    ///
    /// Drives an explicit work stack of [`FlatReadFrame`] steps: `Visit`
    /// looks up a node and schedules its children, `Finish` reassembles the
    /// structural term from the read-back children on the result stack.
    ///
    /// # Contract
    /// - ensures: returns the sort-tagged structural term represented by
    ///   `root`.
    /// - errors: a missing-child [`ArenaBridgeError`] when an id does not
    ///   belong to this arena set; [`ArenaBridgeError::TraversalInvariant`]
    ///   when the result-stack bookkeeping no longer matches the schedule.
    /// - panics: none.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite flat nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: arena nodes and referenced maps/vectors are finite Rust
    ///   data.
    /// - input recursion: none.
    fn read_flat(
        &self,
        root: FlatRoot,
    ) -> Result<StructuralRoot, ArenaBridgeError>
    {
        let mut work = alloc::vec![FlatReadFrame::Visit(root)];
        let mut results = Vec::new();
        while let Some(frame) = work.pop() {
            match frame {
                | FlatReadFrame::Visit(pending) => match pending {
                    | FlatRoot::ValueType(id) => {
                        self.schedule_read_value_type(id, &mut work, &mut results)?;
                    },
                    | FlatRoot::CompType(id) => {
                        self.schedule_read_comp_type(id, &mut work, &mut results)?;
                    },
                    | FlatRoot::Value(id) => {
                        self.schedule_read_value(id, &mut work, &mut results)?;
                    },
                    | FlatRoot::Comp(id) => self.schedule_read_comp(id, &mut work, &mut results)?,
                    | FlatRoot::Stack(id) => {
                        self.schedule_read_stack(id, &mut work, &mut results)?;
                    },
                },
                | FlatReadFrame::Finish(finish) => Self::finish_read(&finish, &mut results)?,
            }
        }
        let rebuilt = results.pop().ok_or(ArenaBridgeError::TraversalInvariant)?;
        if results.is_empty() {
            Ok(rebuilt)
        }
        else {
            Err(ArenaBridgeError::TraversalInvariant)
        }
    }

    /// Schedules the structural read-back of one value-type node.
    ///
    /// Leaf nodes rebuild immediately and push the term onto `results`;
    /// recursive nodes push a [`FlatReadFinish`] reassembly step plus one
    /// [`FlatReadFrame::Visit`] frame per child id (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   structural term the recursive read-back of `id` would rebuild.
    /// - errors: [`ArenaBridgeError::MissingValueType`] when `id` does not
    ///   belong to this arena set.
    /// - panics: none.
    fn schedule_read_value_type<'arena>(
        &'arena self,
        id: ValueTypeNodeId,
        work: &mut Vec<FlatReadFrame<'arena>>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        let node = self
            .value_types
            .get(id)
            .ok_or(ArenaBridgeError::MissingValueType(id))?;
        match *node {
            | ValueTypeNode::Atom(ref name) => {
                results.push(StructuralRoot::ValueType(ValueType::Atom(name.clone())));
            },
            | ValueTypeNode::Unit => {
                results.push(StructuralRoot::ValueType(ValueType::Unit));
            },
            | ValueTypeNode::Prod(fst, snd) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeProd));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(snd)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(fst)));
            },
            | ValueTypeNode::Sum(lhs, rhs) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeSum));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(rhs)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(lhs)));
            },
            | ValueTypeNode::List(element) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeList));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(element)));
            },
            | ValueTypeNode::Record(ref fields) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeRecord(
                    fields,
                )));
                for (_, field) in fields.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::ValueType(*field)));
                }
            },
            | ValueTypeNode::Thunk(grade, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeThunk(grade)));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(body)));
            },
            | ValueTypeNode::Stk(consumes, delivers) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeStk));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(delivers)));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(consumes)));
            },
            | ValueTypeNode::Path { ty, lhs, rhs } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypePath));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(rhs)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(lhs)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(ty)));
            },
            | ValueTypeNode::Family { ref head, ref args } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeFamily {
                    head: head.as_str(),
                    args,
                }));
                for arg in args.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Value(*arg)));
                }
            },
            | ValueTypeNode::Data {
                id: ref data_id,
                ref args,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeData {
                    id: data_id,
                    args,
                }));
                for arg in args.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::ValueType(*arg)));
                }
            },
            | ValueTypeNode::Universe {
                ref sort,
                ref level,
            } => {
                results.push(StructuralRoot::ValueType(ValueType::universe(
                    sort.clone(),
                    level.clone(),
                )));
            },
            | ValueTypeNode::Sealed(ref seal) => {
                results.push(StructuralRoot::ValueType(ValueType::Sealed(seal.clone())));
            },
            | ValueTypeNode::Sigma {
                fst,
                ref binder,
                snd,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypeSigma(
                    binder,
                )));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(snd)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(fst)));
            },
            | ValueTypeNode::Package {
                grade,
                ref abstracts,
                payload,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueTypePackage {
                    grade,
                    abstracts,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(payload)));
            },
            | ValueTypeNode::Unknown => {
                results.push(StructuralRoot::ValueType(ValueType::Unknown));
            },
        }
        Ok(())
    }

    /// Schedules the structural read-back of one computation-type node.
    ///
    /// Leaf nodes rebuild immediately and push the term onto `results`;
    /// recursive nodes push a [`FlatReadFinish`] reassembly step plus one
    /// [`FlatReadFrame::Visit`] frame per child id (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   structural term the recursive read-back of `id` would rebuild.
    /// - errors: [`ArenaBridgeError::MissingCompType`] when `id` does not
    ///   belong to this arena set.
    /// - panics: none.
    fn schedule_read_comp_type<'arena>(
        &'arena self,
        id: CompTypeNodeId,
        work: &mut Vec<FlatReadFrame<'arena>>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        let node = self
            .comp_types
            .get(id)
            .ok_or(ArenaBridgeError::MissingCompType(id))?;
        match *node {
            | CompTypeNode::F(of, ref row) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompTypeF(row)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(of)));
            },
            | CompTypeNode::Arrow {
                ref binder,
                arg,
                res,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompTypeArrow(
                    binder.as_deref(),
                )));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(res)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(arg)));
            },
            | CompTypeNode::With(fst, snd) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompTypeWith));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(snd)));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(fst)));
            },
            | CompTypeNode::Unknown => {
                results.push(StructuralRoot::CompType(CompType::Unknown));
            },
        }
        Ok(())
    }

    /// Schedules the structural read-back of one value node.
    ///
    /// Leaf nodes rebuild immediately and push the term onto `results`;
    /// recursive nodes push a [`FlatReadFinish`] reassembly step plus one
    /// [`FlatReadFrame::Visit`] frame per child id (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   structural term the recursive read-back of `id` would rebuild.
    /// - errors: [`ArenaBridgeError::MissingValue`] when `id` does not belong
    ///   to this arena set.
    /// - panics: none.
    fn schedule_read_value<'arena>(
        &'arena self,
        id: ValueNodeId,
        work: &mut Vec<FlatReadFrame<'arena>>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        let node = self
            .values
            .get(id)
            .ok_or(ArenaBridgeError::MissingValue(id))?;
        match *node {
            | ValueNode::Var(ref name) => {
                results.push(StructuralRoot::Value(Value::Var(name.clone())));
            },
            | ValueNode::Unit => {
                results.push(StructuralRoot::Value(Value::Unit));
            },
            | ValueNode::Int(literal) => {
                results.push(StructuralRoot::Value(Value::Int(literal)));
            },
            | ValueNode::Str(ref literal) => {
                results.push(StructuralRoot::Value(Value::Str(literal.clone())));
            },
            | ValueNode::Num(literal) => {
                results.push(StructuralRoot::Value(Value::Num(literal)));
            },
            | ValueNode::Pair(fst, snd) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValuePair));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(snd)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(fst)));
            },
            | ValueNode::Inj(side, payload) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueInj(side)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(payload)));
            },
            | ValueNode::List(ref elements) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueList(elements)));
                for element in elements.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Value(*element)));
                }
            },
            | ValueNode::Record(ref fields) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueRecord(fields)));
                for (_, field) in fields.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Value(*field)));
                }
            },
            | ValueNode::Thunk(grade, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueThunk(grade)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
            },
            | ValueNode::Run(body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueRun));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
            },
            | ValueNode::Annot(inner, ty) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueAnnot));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(ty)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(inner)));
            },
            | ValueNode::Hole(hole_id) => {
                results.push(StructuralRoot::Value(Value::Hole(hole_id)));
            },
            | ValueNode::Stk(stack) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueStk));
                work.push(FlatReadFrame::Visit(FlatRoot::Stack(stack)));
            },
            | ValueNode::Here(witness) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueHere));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(witness)));
            },
            | ValueNode::Ctor {
                id: ref data_id,
                tag,
                payload,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValueCtor {
                    id: data_id,
                    tag,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(payload)));
            },
            | ValueNode::Pack {
                ref witnesses,
                payload,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::ValuePack {
                    witnesses: witnesses.len(),
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(payload)));
                for witness in witnesses.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::ValueType(*witness)));
                }
            },
        }
        Ok(())
    }

    /// Schedules the structural read-back of one computation node.
    ///
    /// Leaf nodes rebuild immediately and push the term onto `results`;
    /// recursive nodes push a [`FlatReadFinish`] reassembly step plus one
    /// [`FlatReadFrame::Visit`] frame per child id (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   structural term the recursive read-back of `id` would rebuild.
    /// - errors: [`ArenaBridgeError::MissingComp`] when `id` does not belong to
    ///   this arena set.
    /// - panics: none.
    fn schedule_read_comp<'arena>(
        &'arena self,
        id: CompNodeId,
        work: &mut Vec<FlatReadFrame<'arena>>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        let node = self
            .comps
            .get(id)
            .ok_or(ArenaBridgeError::MissingComp(id))?;
        match *node {
            | CompNode::Abs(ref name, ty, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompAbs {
                    name,
                    has_ty: ty.is_some(),
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
                if let Some(ty) = ty {
                    work.push(FlatReadFrame::Visit(FlatRoot::ValueType(ty)));
                }
            },
            | CompNode::App(head, arg) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompApp));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(arg)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(head)));
            },
            | CompNode::Ret(value) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompRet));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(value)));
            },
            | CompNode::Bind(bound, ref name, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompBind(name)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(bound)));
            },
            | CompNode::Force(value) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompForce));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(value)));
            },
            | CompNode::Case(scrut, ref fst, ref snd) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompCase {
                    fst_name: &fst.0,
                    snd_name: &snd.0,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(snd.1)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(fst.1)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
            | CompNode::ListCase {
                scrut,
                nil,
                ref head,
                ref tail,
                cons,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompListCase {
                    head,
                    tail,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(cons)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(nil)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
            | CompNode::Split {
                scrut,
                ref fst_name,
                ref snd_name,
                ref motive,
                body,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompSplit {
                    fst_name,
                    snd_name,
                    motive: motive.as_ref(),
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
                if let Some(motive) = motive.as_ref() {
                    work.push(FlatReadFrame::Visit(FlatRoot::CompType(motive.body)));
                }
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
            | CompNode::DataCase { scrut, ref arms } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompDataCase(arms)));
                for arm in arms.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Comp(arm.1)));
                }
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
            | CompNode::RecordProj { record, ref label } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompRecordProj(label)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(record)));
            },
            | CompNode::With(fst, snd) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompWith));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(snd)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(fst)));
            },
            | CompNode::Prj(side, target) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompPrj(side)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(target)));
            },
            | CompNode::Dup(value) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompDup));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(value)));
            },
            | CompNode::Drop(value) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompDrop));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(value)));
            },
            | CompNode::Perform(ref sig, ref op, arg) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompPerform {
                    sig: sig.as_ref(),
                    op,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(arg)));
            },
            | CompNode::Handle {
                ref sig,
                scrutinee,
                ref ret,
                ref ops,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompHandle {
                    sig: sig.as_ref(),
                    ret_name: &ret.0,
                    ops,
                }));
                for clause in ops.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Comp(clause.body)));
                }
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(ret.1)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(scrutinee)));
            },
            | CompNode::Resume(stack, fed) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompResume));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(fed)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(stack)));
            },
            | CompNode::Reset(body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompReset));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
            },
            | CompNode::Shift(ref k, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompShift(k)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
            },
            | CompNode::Fix(ref x, body) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompFix(x)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
            },
            | CompNode::Hole(hole_id) => {
                results.push(StructuralRoot::Comp(Comp::Hole(hole_id)));
            },
            | CompNode::Native { prim, ref args } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompNative {
                    prim,
                    args,
                }));
                for arg in args.iter().rev() {
                    work.push(FlatReadFrame::Visit(FlatRoot::Value(*arg)));
                }
            },
            | CompNode::Walk {
                scrut,
                ref motive,
                ref base,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompWalk {
                    motive,
                    base,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(base.body)));
                work.push(FlatReadFrame::Visit(FlatRoot::CompType(motive.body)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
            | CompNode::Unpack {
                scrut,
                signature,
                ref atoms,
                ref binder,
                body,
            } => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::CompUnpack {
                    atoms,
                    binder,
                }));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(body)));
                work.push(FlatReadFrame::Visit(FlatRoot::ValueType(signature)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(scrut)));
            },
        }
        Ok(())
    }

    /// Schedules the structural read-back of one stack node.
    ///
    /// Leaf nodes rebuild immediately and push the term onto `results`;
    /// recursive nodes push a [`FlatReadFinish`] reassembly step plus one
    /// [`FlatReadFrame::Visit`] frame per child id (in reverse, so children
    /// finish in source order).
    ///
    /// # Contract
    /// - ensures: on return, the work and result stacks produce exactly the
    ///   structural term the recursive read-back of `id` would rebuild.
    /// - errors: [`ArenaBridgeError::MissingStack`] when `id` does not belong
    ///   to this arena set.
    /// - panics: none.
    fn schedule_read_stack<'arena>(
        &'arena self,
        id: StackNodeId,
        work: &mut Vec<FlatReadFrame<'arena>>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        let node = self
            .stacks
            .get(id)
            .ok_or(ArenaBridgeError::MissingStack(id))?;
        match *node {
            | StackNode::Empty => {
                results.push(StructuralRoot::Stack(Stack::Empty));
            },
            | StackNode::Arg(value, rest) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::StackArg));
                work.push(FlatReadFrame::Visit(FlatRoot::Stack(rest)));
                work.push(FlatReadFrame::Visit(FlatRoot::Value(value)));
            },
            | StackNode::Bind(ref name, cont, rest) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::StackBind(name)));
                work.push(FlatReadFrame::Visit(FlatRoot::Stack(rest)));
                work.push(FlatReadFrame::Visit(FlatRoot::Comp(cont)));
            },
            | StackNode::Prj(side, rest) => {
                work.push(FlatReadFrame::Finish(FlatReadFinish::StackPrj(side)));
                work.push(FlatReadFrame::Visit(FlatRoot::Stack(rest)));
            },
        }
        Ok(())
    }

    /// Reassembles one structural term from its read-back children.
    ///
    /// Pops the child terms reserved by the matching `schedule_read_*` step
    /// off `results`, applies the constructor recorded by `finish`, and
    /// pushes the rebuilt term back.
    ///
    /// # Contract
    /// - ensures: `results` trades the term's children for the rebuilt term of
    ///   the same sort.
    /// - errors: [`ArenaBridgeError::TraversalInvariant`] when a child term has
    ///   the wrong sort.
    /// - panics: none.
    fn finish_read(
        finish: &FlatReadFinish<'_>,
        results: &mut Vec<StructuralRoot>,
    ) -> Result<(), ArenaBridgeError>
    {
        match *finish {
            | FlatReadFinish::ValueTypeProd => {
                let snd = pop_read_value_type(results)?;
                let fst = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Prod(
                    Rc::new(fst),
                    Rc::new(snd),
                )));
            },
            | FlatReadFinish::ValueTypeSum => {
                let rhs = pop_read_value_type(results)?;
                let lhs = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Sum(
                    Rc::new(lhs),
                    Rc::new(rhs),
                )));
            },
            | FlatReadFinish::ValueTypeList => {
                let element = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::List(Rc::new(element))));
            },
            | FlatReadFinish::ValueTypeRecord(fields) => {
                let values = pop_read_value_types(results, fields.len().into())?;
                let mut structural = BTreeMap::new();
                for ((label, _), value) in fields.iter().zip(values) {
                    structural.insert(label.clone(), Rc::new(value));
                }
                results.push(StructuralRoot::ValueType(ValueType::Record(structural)));
            },
            | FlatReadFinish::ValueTypeThunk(grade) => {
                let body = pop_read_comp_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Thunk(
                    grade,
                    Rc::new(body),
                )));
            },
            | FlatReadFinish::ValueTypeStk => {
                let delivers = pop_read_comp_type(results)?;
                let consumes = pop_read_comp_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Stk(
                    Rc::new(consumes),
                    Rc::new(delivers),
                )));
            },
            | FlatReadFinish::ValueTypePath => {
                let rhs = pop_read_value(results)?;
                let lhs = pop_read_value(results)?;
                let ty = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Path {
                    ty: Rc::new(ty),
                    lhs: Rc::new(lhs),
                    rhs: Rc::new(rhs),
                }));
            },
            | FlatReadFinish::ValueTypeFamily { head, args } => {
                let values = pop_read_values(results, args.len().into())?;
                results.push(StructuralRoot::ValueType(ValueType::Family {
                    head: head.to_owned(),
                    args: values.into_iter().map(Rc::new).collect(),
                }));
            },
            | FlatReadFinish::ValueTypeData { id, args } => {
                let values = pop_read_value_types(results, args.len().into())?;
                results.push(StructuralRoot::ValueType(ValueType::Data {
                    id: id.clone(),
                    args: values.into_iter().map(Rc::new).collect(),
                }));
            },
            | FlatReadFinish::ValueTypeSigma(binder) => {
                let snd = pop_read_value_type(results)?;
                let fst = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Sigma {
                    fst: Rc::new(fst),
                    binder: binder.to_owned(),
                    snd: Rc::new(snd),
                }));
            },
            | FlatReadFinish::ValueTypePackage { grade, abstracts } => {
                let payload = pop_read_value_type(results)?;
                results.push(StructuralRoot::ValueType(ValueType::Package {
                    grade,
                    abstracts: abstracts.to_vec(),
                    payload: Rc::new(payload),
                }));
            },
            | FlatReadFinish::ValuePack { witnesses } => {
                let payload = pop_read_value(results)?;
                let witness_types = pop_read_value_types(results, witnesses.into())?;
                results.push(StructuralRoot::Value(Value::Pack {
                    witnesses: witness_types.into_iter().map(Rc::new).collect(),
                    payload: Rc::new(payload),
                }));
            },
            | FlatReadFinish::CompUnpack { atoms, binder } => {
                let body = pop_read_comp(results)?;
                let signature = pop_read_value_type(results)?;
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Unpack {
                    scrut: Rc::new(scrut),
                    signature: Rc::new(signature),
                    atoms: atoms.to_vec(),
                    binder: binder.to_owned(),
                    body: Rc::new(body),
                }));
            },
            | FlatReadFinish::CompTypeF(row) => {
                let of = pop_read_value_type(results)?;
                results.push(StructuralRoot::CompType(CompType::F(
                    Rc::new(of),
                    row.clone(),
                )));
            },
            | FlatReadFinish::CompTypeArrow(binder) => {
                let res = pop_read_comp_type(results)?;
                let arg = pop_read_value_type(results)?;
                results.push(StructuralRoot::CompType(CompType::Arrow {
                    binder: binder.map(str::to_owned),
                    arg: Rc::new(arg),
                    res: Rc::new(res),
                }));
            },
            | FlatReadFinish::CompTypeWith => {
                let snd = pop_read_comp_type(results)?;
                let fst = pop_read_comp_type(results)?;
                results.push(StructuralRoot::CompType(CompType::With(
                    Rc::new(fst),
                    Rc::new(snd),
                )));
            },
            | FlatReadFinish::ValuePair => {
                let snd = pop_read_value(results)?;
                let fst = pop_read_value(results)?;
                results.push(StructuralRoot::Value(Value::Pair(
                    Rc::new(fst),
                    Rc::new(snd),
                )));
            },
            | FlatReadFinish::ValueInj(side) => {
                let payload = pop_read_value(results)?;
                results.push(StructuralRoot::Value(Value::Inj(side, Rc::new(payload))));
            },
            | FlatReadFinish::ValueList(elements) => {
                let values = pop_read_values(results, elements.len().into())?;
                results.push(StructuralRoot::Value(Value::List(
                    values.into_iter().map(Rc::new).collect(),
                )));
            },
            | FlatReadFinish::ValueRecord(fields) => {
                let values = pop_read_values(results, fields.len().into())?;
                let mut structural = BTreeMap::new();
                for ((label, _), value) in fields.iter().zip(values) {
                    structural.insert(label.clone(), Rc::new(value));
                }
                results.push(StructuralRoot::Value(Value::Record(structural)));
            },
            | FlatReadFinish::ValueThunk(grade) => {
                let body = pop_read_comp(results)?;
                results.push(StructuralRoot::Value(Value::Thunk(grade, Rc::new(body))));
            },
            | FlatReadFinish::ValueRun => {
                let body = pop_read_comp(results)?;
                results.push(StructuralRoot::Value(Value::Run(Rc::new(body))));
            },
            | FlatReadFinish::ValueAnnot => {
                let ty = pop_read_value_type(results)?;
                let inner = pop_read_value(results)?;
                results.push(StructuralRoot::Value(Value::Annot(
                    Rc::new(inner),
                    Rc::new(ty),
                )));
            },
            | FlatReadFinish::ValueStk => {
                let stack = pop_read_stack(results)?;
                results.push(StructuralRoot::Value(Value::Stk(Rc::new(stack))));
            },
            | FlatReadFinish::ValueHere => {
                let witness = pop_read_value(results)?;
                results.push(StructuralRoot::Value(Value::Here(Rc::new(witness))));
            },
            | FlatReadFinish::ValueCtor { id, tag } => {
                let payload = pop_read_value(results)?;
                results.push(StructuralRoot::Value(Value::Ctor {
                    id: id.clone(),
                    tag,
                    payload: Rc::new(payload),
                }));
            },
            | FlatReadFinish::CompAbs { name, has_ty } => {
                let body = pop_read_comp(results)?;
                let ty = if has_ty {
                    let ty = pop_read_value_type(results)?;
                    Some(Rc::new(ty))
                }
                else {
                    None
                };
                results.push(StructuralRoot::Comp(Comp::Abs(
                    name.to_owned(),
                    ty,
                    Rc::new(body),
                )));
            },
            | FlatReadFinish::CompApp => {
                let arg = pop_read_value(results)?;
                let head = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::App(Rc::new(head), Rc::new(arg))));
            },
            | FlatReadFinish::CompRet => {
                let value = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Ret(Rc::new(value))));
            },
            | FlatReadFinish::CompBind(name) => {
                let body = pop_read_comp(results)?;
                let bound = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::Bind(
                    Rc::new(bound),
                    name.to_owned(),
                    Rc::new(body),
                )));
            },
            | FlatReadFinish::CompForce => {
                let value = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Force(Rc::new(value))));
            },
            | FlatReadFinish::CompCase { fst_name, snd_name } => {
                let snd_body = pop_read_comp(results)?;
                let fst_body = pop_read_comp(results)?;
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Case(
                    Rc::new(scrut),
                    (fst_name.to_owned(), Rc::new(fst_body)),
                    (snd_name.to_owned(), Rc::new(snd_body)),
                )));
            },
            | FlatReadFinish::CompListCase { head, tail } => {
                let cons = pop_read_comp(results)?;
                let nil = pop_read_comp(results)?;
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::ListCase {
                    scrut: Rc::new(scrut),
                    nil: Rc::new(nil),
                    head: head.to_owned(),
                    tail: tail.to_owned(),
                    cons: Rc::new(cons),
                }));
            },
            | FlatReadFinish::CompSplit {
                fst_name,
                snd_name,
                motive,
            } => {
                let body = pop_read_comp(results)?;
                let motive = if let Some(motive) = motive {
                    let motive_body = pop_read_comp_type(results)?;
                    Some(Box::new(SplitMotive {
                        binder: motive.binder.clone(),
                        body: Rc::new(motive_body),
                    }))
                }
                else {
                    None
                };
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Split {
                    scrut: Rc::new(scrut),
                    fst_name: fst_name.to_owned(),
                    snd_name: snd_name.to_owned(),
                    motive,
                    body: Rc::new(body),
                }));
            },
            | FlatReadFinish::CompDataCase(arms) => {
                let bodies = pop_read_comps(results, arms.len().into())?;
                let mut structural = Vec::with_capacity(arms.len());
                for (arm, body) in arms.iter().zip(bodies) {
                    structural.push((arm.0.clone(), Rc::new(body)));
                }
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::DataCase(
                    Rc::new(scrut),
                    structural,
                )));
            },
            | FlatReadFinish::CompRecordProj(label) => {
                let record = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::RecordProj {
                    record: Rc::new(record),
                    label: label.to_owned(),
                }));
            },
            | FlatReadFinish::CompWith => {
                let snd = pop_read_comp(results)?;
                let fst = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::With(Rc::new(fst), Rc::new(snd))));
            },
            | FlatReadFinish::CompPrj(side) => {
                let target = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::Prj(side, Rc::new(target))));
            },
            | FlatReadFinish::CompDup => {
                let value = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Dup(Rc::new(value))));
            },
            | FlatReadFinish::CompDrop => {
                let value = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Drop(Rc::new(value))));
            },
            | FlatReadFinish::CompPerform { sig, op } => {
                let arg = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Perform(
                    Box::new(sig.clone()),
                    op.to_owned(),
                    Rc::new(arg),
                )));
            },
            | FlatReadFinish::CompHandle { sig, ret_name, ops } => {
                let bodies = pop_read_comps(results, ops.len().into())?;
                let ret_body = pop_read_comp(results)?;
                let scrutinee = pop_read_comp(results)?;
                let mut structural_ops = Vec::with_capacity(ops.len());
                for (clause, body) in ops.iter().zip(bodies) {
                    structural_ops.push(OpClause {
                        op: clause.op.clone(),
                        payload: clause.payload.clone(),
                        resume: clause.resume.clone(),
                        body: Rc::new(body),
                    });
                }
                results.push(StructuralRoot::Comp(Comp::Handle {
                    sig: Box::new(sig.clone()),
                    scrutinee: Rc::new(scrutinee),
                    ret: (ret_name.to_owned(), Rc::new(ret_body)),
                    ops: structural_ops,
                }));
            },
            | FlatReadFinish::CompResume => {
                let fed = pop_read_comp(results)?;
                let stack = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Resume(
                    Rc::new(stack),
                    Rc::new(fed),
                )));
            },
            | FlatReadFinish::CompReset => {
                let body = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::Reset(Rc::new(body))));
            },
            | FlatReadFinish::CompShift(k) => {
                let body = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::Shift(
                    k.to_owned(),
                    Rc::new(body),
                )));
            },
            | FlatReadFinish::CompFix(x) => {
                let body = pop_read_comp(results)?;
                results.push(StructuralRoot::Comp(Comp::Fix(x.to_owned(), Rc::new(body))));
            },
            | FlatReadFinish::CompNative { prim, args } => {
                let values = pop_read_values(results, args.len().into())?;
                results.push(StructuralRoot::Comp(Comp::Native {
                    prim,
                    args: values.into_iter().map(Rc::new).collect(),
                }));
            },
            | FlatReadFinish::CompWalk { motive, base } => {
                let base_body = pop_read_comp(results)?;
                let motive_body = pop_read_comp_type(results)?;
                let scrut = pop_read_value(results)?;
                results.push(StructuralRoot::Comp(Comp::Walk {
                    scrut: Rc::new(scrut),
                    motive: Box::new(WalkMotive {
                        x: motive.x.clone(),
                        y: motive.y.clone(),
                        q: motive.q.clone(),
                        body: Rc::new(motive_body),
                    }),
                    base: WalkBase {
                        x: base.x.clone(),
                        body: Rc::new(base_body),
                    },
                }));
            },
            | FlatReadFinish::StackArg => {
                let rest = pop_read_stack(results)?;
                let value = pop_read_value(results)?;
                results.push(StructuralRoot::Stack(Stack::Arg(
                    Rc::new(value),
                    Rc::new(rest),
                )));
            },
            | FlatReadFinish::StackBind(name) => {
                let rest = pop_read_stack(results)?;
                let cont = pop_read_comp(results)?;
                results.push(StructuralRoot::Stack(Stack::Bind(
                    name.to_owned(),
                    Rc::new(cont),
                    Rc::new(rest),
                )));
            },
            | FlatReadFinish::StackPrj(side) => {
                let rest = pop_read_stack(results)?;
                results.push(StructuralRoot::Stack(Stack::Prj(side, Rc::new(rest))));
            },
        }
        Ok(())
    }

    /// Converts a legacy value type into canonical flat type nodes.
    ///
    /// # Contract
    /// - ensures: returns an id for a freshly allocated [`ValueTypeNode`]
    ///   graph.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::IdSpaceExhausted`] when the value-type arena
    /// would exceed its id space.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: value types, term endpoints, and referenced maps/vectors
    ///   are finite Rust data.
    /// - input recursion: none.
    #[inline]
    pub fn alloc_value_type(
        &mut self,
        ty: &ValueType,
    ) -> Result<ValueTypeNodeId, ArenaBridgeError>
    {
        match self.alloc_legacy(LegacyRoot::ValueType(ty))? {
            | LegacyRootId::ValueType(id) => Ok(id),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Converts a legacy computation type into canonical flat type nodes.
    ///
    /// # Contract
    /// - ensures: returns an id for a freshly allocated [`CompTypeNode`] graph.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::IdSpaceExhausted`] when the computation-type
    /// arena would exceed its id space.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: computation types and referenced maps/vectors are finite
    ///   Rust data.
    /// - input recursion: none.
    #[inline]
    pub fn alloc_comp_type(
        &mut self,
        ty: &CompType,
    ) -> Result<CompTypeNodeId, ArenaBridgeError>
    {
        match self.alloc_legacy(LegacyRoot::CompType(ty))? {
            | LegacyRootId::CompType(id) => Ok(id),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Converts a legacy computation into canonical flat carrier nodes.
    ///
    /// # Contract
    /// - ensures: returns an id for a freshly allocated [`CompNode`] graph
    ///   equivalent to `comp`, with every child represented by a typed arena
    ///   id.
    /// - errors: [`ArenaBridgeError::IdSpaceExhausted`] when any arena would
    ///   exceed its id space.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::IdSpaceExhausted`] when any carrier arena
    /// would exceed its id space.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: computations, cross-sort children, clauses, arms, and
    ///   primitive argument vectors are finite Rust data.
    /// - input recursion: none.
    #[inline]
    pub fn alloc_comp(
        &mut self,
        comp: &Comp,
    ) -> Result<CompNodeId, ArenaBridgeError>
    {
        match self.alloc_legacy(LegacyRoot::Comp(comp))? {
            | LegacyRootId::Comp(id) => Ok(id),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Converts a legacy value into canonical flat carrier nodes.
    ///
    /// # Contract
    /// - ensures: returns an id for a freshly allocated [`ValueNode`] graph.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::IdSpaceExhausted`] when any carrier arena
    /// would exceed its id space.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite children not yet visited.
    /// - boundedness: values, cross-sort children, fields, and lists are finite
    ///   Rust data.
    /// - input recursion: none.
    #[inline]
    pub fn alloc_value(
        &mut self,
        value: &Value,
    ) -> Result<ValueNodeId, ArenaBridgeError>
    {
        match self.alloc_legacy(LegacyRoot::Value(value))? {
            | LegacyRootId::Value(id) => Ok(id),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Converts a legacy reified stack into canonical flat carrier nodes.
    ///
    /// # Contract
    /// - ensures: returns an id for a freshly allocated [`StackNode`] spine.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::IdSpaceExhausted`] when the stack arena
    /// would exceed its id space.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite legacy syntax; no
    ///   Rust call-stack descent.
    /// - measure: pending worklist frames plus finite stack frames not yet
    ///   visited.
    /// - boundedness: stacks and their value/computation children are finite
    ///   Rust data.
    /// - input recursion: none.
    #[inline]
    pub fn alloc_stack(
        &mut self,
        stack: &Stack,
    ) -> Result<StackNodeId, ArenaBridgeError>
    {
        match self.alloc_legacy(LegacyRoot::Stack(stack))? {
            | LegacyRootId::Stack(id) => Ok(id),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Reads a canonical value-type id back to the legacy structural surface.
    ///
    /// # Contract
    /// - ensures: returns the structural value type represented by `id`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::MissingValueType`] or another missing-child
    /// bridge error when `id` or a reachable child id is absent.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite arena nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus reachable child ids not yet
    ///   read.
    /// - boundedness: node arenas are finite append-only tables.
    /// - input recursion: none.
    #[inline]
    pub fn value_type(
        &self,
        id: ValueTypeNodeId,
    ) -> Result<ValueType, ArenaBridgeError>
    {
        match self.read_flat(FlatRoot::ValueType(id))? {
            | StructuralRoot::ValueType(ty) => Ok(ty),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Reads a canonical computation-type id back to the legacy structural
    /// surface.
    ///
    /// # Contract
    /// - ensures: returns the structural computation type represented by `id`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::MissingCompType`] or another missing-child
    /// bridge error when `id` or a reachable child id is absent.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite arena nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus reachable child ids not yet
    ///   read.
    /// - boundedness: node arenas are finite append-only tables.
    /// - input recursion: none.
    #[inline]
    pub fn comp_type(
        &self,
        id: CompTypeNodeId,
    ) -> Result<CompType, ArenaBridgeError>
    {
        match self.read_flat(FlatRoot::CompType(id))? {
            | StructuralRoot::CompType(ty) => Ok(ty),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Reads a canonical computation id back to the legacy structural surface.
    ///
    /// # Contract
    /// - ensures: returns the structural computation represented by `id`.
    /// - errors: a [`ArenaBridgeError::MissingComp`] / missing child when an id
    ///   does not belong to this arena set.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::MissingComp`] or another missing-child
    /// bridge error when `id` or a reachable child id is absent.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite arena nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus reachable child ids not yet
    ///   read.
    /// - boundedness: node arenas are finite append-only tables.
    /// - input recursion: none.
    #[inline]
    pub fn comp(
        &self,
        id: CompNodeId,
    ) -> Result<Comp, ArenaBridgeError>
    {
        match self.read_flat(FlatRoot::Comp(id))? {
            | StructuralRoot::Comp(comp) => Ok(comp),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Reads a canonical value id back to the legacy structural surface.
    ///
    /// # Contract
    /// - ensures: returns the structural value represented by `id`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::MissingValue`] or another missing-child
    /// bridge error when `id` or a reachable child id is absent.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite arena nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus reachable child ids not yet
    ///   read.
    /// - boundedness: node arenas are finite append-only tables.
    /// - input recursion: none.
    #[inline]
    pub fn value(
        &self,
        id: ValueNodeId,
    ) -> Result<Value, ArenaBridgeError>
    {
        match self.read_flat(FlatRoot::Value(id))? {
            | StructuralRoot::Value(value) => Ok(value),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }

    /// Reads a canonical stack id back to the legacy structural surface.
    ///
    /// # Contract
    /// - ensures: returns the structural stack represented by `id`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ArenaBridgeError::MissingStack`] or another missing-child
    /// bridge error when `id` or a reachable child id is absent.
    ///
    /// # Termination
    /// - reason: explicit post-order worklist over finite arena nodes; no Rust
    ///   call-stack descent.
    /// - measure: pending worklist frames plus reachable stack ids not yet
    ///   read.
    /// - boundedness: node arenas are finite append-only tables.
    /// - input recursion: none.
    #[inline]
    pub fn stack(
        &self,
        id: StackNodeId,
    ) -> Result<Stack, ArenaBridgeError>
    {
        match self.read_flat(FlatRoot::Stack(id))? {
            | StructuralRoot::Stack(stack) => Ok(stack),
            | _ => Err(ArenaBridgeError::TraversalInvariant),
        }
    }
}

/// A term of either sort, as carried by errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Term
{
    /// A value term.
    Value(
        /// The value.
        Value,
    ),
    /// A computation term.
    Comp(
        /// The computation.
        Comp,
    ),
}

/// Tests for the host-seam value decoders (ADR-35 D4).
#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_kernel_strata::Level;

    use super::Comp;
    use super::FlatArena;
    use super::NodeArena;
    use super::Stack;
    use super::Value;
    use super::ValueNode;
    use crate::boundary::IntegerLiteral;
    use crate::boundary::NodeIndex;
    use crate::classifier::SortExpr;
    use crate::grade::Grade;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// The ADR-50 arena substrate allocates stable typed ids and exposes only
    /// checked lookup.
    #[test]
    fn node_arena_allocates_checked_typed_ids()
    {
        let mut arena = NodeArena::new();
        let first = arena.alloc(ValueNode::Unit).expect("fresh arena has id 0");
        let second = arena
            .alloc(ValueNode::Int(7))
            .expect("fresh arena has id 1");
        assert_eq!(NodeIndex::from(0), first.index());
        assert_eq!(NodeIndex::from(1), second.index());
        assert_eq!(Some(&ValueNode::Unit), arena.get(first));
        assert_eq!(Some(&ValueNode::Int(7)), arena.get(second));
        assert_eq!(None, arena.get(super::NodeId::new(99)));
    }

    /// The explicit legacy bridge allocates canonical value / computation /
    /// stack carrier nodes and reads them back without aliasing `Rc` terms as
    /// ids.
    #[test]
    fn flat_arena_round_trips_legacy_terms()
    {
        let comp = Comp::bind(
            Comp::ret(Value::int(1)),
            "x",
            Comp::force(Value::thunk(Grade::ONE, Comp::ret(Value::var("x")))),
        );
        let stack = Stack::bind(
            "x",
            Comp::ret(Value::var("x")),
            Stack::arg(Value::int(7), Stack::empty()),
        );
        let value = Value::stk(stack.clone());

        let mut arena = FlatArena::new();
        let comp_id = arena
            .alloc_comp(&comp)
            .expect("small computation fits in the canonical arena");
        let value_id = arena
            .alloc_value(&value)
            .expect("small value fits in the canonical arena");
        let stack_id = arena
            .alloc_stack(&stack)
            .expect("small stack fits in the canonical arena");

        assert_eq!(arena.comp(comp_id), Ok(comp));
        assert_eq!(arena.value(value_id), Ok(value));
        assert_eq!(arena.stack(stack_id), Ok(stack));
    }

    /// The two ground classifier families remain distinct at the same level:
    /// `Type[+, 0]` and `Type[-, 0]` are different arena nodes.
    #[test]
    fn flat_arena_distinguishes_type_plus_zero_and_type_minus_zero()
    {
        let level = Level::zero();
        let value_universe = ValueType::universe(SortExpr::value(), level.clone());
        let computation_universe = ValueType::universe(SortExpr::computation(), level);
        assert_ne!(value_universe, computation_universe);

        let mut arena = FlatArena::new();
        let value_id = arena
            .alloc_value_type(&value_universe)
            .expect("value universe fits in the type arena");
        let computation_id = arena
            .alloc_value_type(&computation_universe)
            .expect("computation universe fits in the type arena");
        assert_ne!(value_id, computation_id);
    }

    /// The arena read-back preserves both classifier sort and shared level
    /// fields for the two ground universe families.
    #[test]
    fn flat_arena_round_trips_universe_classifier_and_level()
    {
        let level = Level::zero();
        let value_universe = ValueType::universe(SortExpr::value(), level.clone());
        let computation_universe = ValueType::universe(SortExpr::computation(), level);
        let mut arena = FlatArena::new();
        let value_id = arena
            .alloc_value_type(&value_universe)
            .expect("value universe fits in the type arena");
        let computation_id = arena
            .alloc_value_type(&computation_universe)
            .expect("computation universe fits in the type arena");

        assert_eq!(Ok(value_universe), arena.value_type(value_id));
        assert_eq!(Ok(computation_universe), arena.value_type(computation_id));
    }

    /// Lambda and thunk bodies cross the checked bridge as computation-root
    /// ids, not as retained `Rc<Comp>` bodies.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        expect(
            non_local_effect_before_unhandled_error,
            reason = "the flagged allocations write only this test's own arena, built at the top of the body and dropped at scope exit, so a partial allocation is unreachable from anything outliving the call; witnessed by this test's own round-trip assertions"
        )
    )]
    #[test]
    fn flat_arena_round_trips_lambda_and_thunk_roots()
    {
        let body = Comp::ret(Value::var("x"));
        let lambda = Comp::Abs("x".to_owned(), None, Rc::new(body.clone()));
        let thunk = Value::thunk(Grade::ONE, body);
        let mut arena = FlatArena::new();

        let lambda_round_trip = arena.alloc_comp(&lambda).and_then(|root| arena.comp(root));
        let thunk_round_trip = arena.alloc_value(&thunk).and_then(|root| arena.value(root));

        assert_eq!(lambda_round_trip, Ok(lambda));
        assert_eq!(thunk_round_trip, Ok(thunk));
    }

    /// Canonical type roots allocate and read back nested function and thunk
    /// types without retaining legacy recursive children in term nodes.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        expect(
            non_local_effect_before_unhandled_error,
            reason = "the flagged allocations write only this test's own arena, built at the top of the body and dropped at scope exit, so a partial allocation is unreachable from anything outliving the call; witnessed by this test's own round-trip assertions"
        )
    )]
    #[test]
    fn flat_arena_round_trips_type_roots()
    {
        let function_ty = CompType::arrow(
            ValueType::integer(),
            CompType::returner(ValueType::string()),
        );
        let thunk_ty = ValueType::thunk(Grade::ONE, CompType::returner(ValueType::integer()));
        let mut arena = FlatArena::new();

        let function_round_trip = arena
            .alloc_comp_type(&function_ty)
            .and_then(|root| arena.comp_type(root));
        let thunk_round_trip = arena
            .alloc_value_type(&thunk_ty)
            .and_then(|root| arena.value_type(root));

        assert_eq!(function_round_trip, Ok(function_ty));
        assert_eq!(thunk_round_trip, Ok(thunk_ty));
    }

    /// Value annotations and lambda parameter annotations bridge through typed
    /// value-type roots while preserving the legacy public readback surface.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        expect(
            non_local_effect_before_unhandled_error,
            reason = "the flagged allocations write only this test's own arena, built at the top of the body and dropped at scope exit, so a partial allocation is unreachable from anything outliving the call; witnessed by this test's own round-trip assertions"
        )
    )]
    #[test]
    fn flat_arena_round_trips_annotated_value_and_lambda_parameter()
    {
        let annotated_value = Value::annot(Value::int(7), ValueType::integer());
        let lambda = Comp::Abs(
            "x".to_owned(),
            Some(Rc::new(ValueType::integer())),
            Rc::new(Comp::ret(Value::var("x"))),
        );
        let mut arena = FlatArena::new();

        let value_round_trip = arena
            .alloc_value(&annotated_value)
            .and_then(|root| arena.value(root));
        let lambda_round_trip = arena.alloc_comp(&lambda).and_then(|root| arena.comp(root));

        assert_eq!(value_round_trip, Ok(annotated_value));
        assert_eq!(lambda_round_trip, Ok(lambda));
    }

    /// Deep same-sort type chains use the bridge worklist rather than the Rust
    /// call stack.
    #[test]
    fn flat_arena_round_trips_deep_value_type_worklist()
    {
        let depth = 2_048_i32;
        let mut value_type = ValueType::integer();
        for _ in 0_i32 .. depth {
            value_type = ValueType::list(value_type);
        }
        let mut arena = FlatArena::new();
        let root = arena
            .alloc_value_type(&value_type)
            .expect("deep value type still fits in the canonical arena");
        let readback = arena
            .value_type(root)
            .expect("deep value type reads back from the canonical arena");

        let mut cursor = &readback;
        let mut seen = 0_i32;
        while let ValueType::List(ref inner) = *cursor {
            seen += 1_i32;
            cursor = inner.as_ref();
        }
        assert_eq!(depth, seen);
        assert_eq!(&ValueType::integer(), cursor);
    }

    /// Left-nested computation spines preserve order through the iterative
    /// bridge without the old recursive bind-spine special case.
    #[test]
    fn flat_arena_round_trips_deep_bind_worklist()
    {
        let depth = 512_i32;
        let mut comp = Comp::ret(Value::int(0));
        for _ in 0_i32 .. depth {
            comp = Comp::bind(comp, "x", Comp::ret(Value::var("x")));
        }
        let mut arena = FlatArena::new();
        let root = arena
            .alloc_comp(&comp)
            .expect("deep computation still fits in the canonical arena");
        let readback = arena
            .comp(root)
            .expect("deep computation reads back from the canonical arena");

        let mut cursor = &readback;
        let mut seen = 0_i32;
        while let Comp::Bind(ref bound, ..) = *cursor {
            seen += 1_i32;
            cursor = bound.as_ref();
        }
        assert_eq!(depth, seen);
        assert!(matches!(cursor, Comp::Ret(_)));
    }

    /// [`Value::as_list`] peels type annotations to the underlying list (the
    /// "seeing through annotations" behavior every host-seam decoder shares via
    /// [`Value::peeled`]), and reads `None` for a non-list.
    #[test]
    fn as_list_sees_through_annotations()
    {
        let list = Value::list(alloc::vec![Value::int(1), Value::int(2)]);
        let annotated = Value::annot(list, ValueType::list(ValueType::integer()));
        let elements = annotated
            .as_list()
            .expect("as_list peels the annotation to the underlying list");
        assert_eq!(2, elements.len());
        assert_eq!(
            Some(IntegerLiteral::from(1)),
            elements.first().and_then(|value| value.as_int())
        );
        assert_eq!(
            None,
            Value::int(3).as_list(),
            "a non-list value has no list view"
        );
    }
}
