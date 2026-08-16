//! The semantic value domain: the glued values normalization by evaluation
//! produces, their arena, and the guard word conversion reads before it walks.
//!
//! # The two faces, named together
//!
//! "Glued" names two mechanisms, and building one while believing the other
//! exists is the design's first named hazard. This module answers it the way
//! the design requires — **the domain names both faces in its type before
//! either is exercised**:
//!
//! * the **term face** ([`TermFace`]) — a semantic value caches the source term
//!   it came from when nothing inside it reduced, so readback returns that term
//!   verbatim instead of rebuilding one;
//! * the **unfolding face** ([`ValueUnfold`] and [`CompUnfold`]) — a neutral
//!   whose head has an unfolding rule keeps the neutral form and a
//!   lazily-forced unfolded form together, built incrementally as the spine
//!   grows.
//!
//! Readback chooses a face and conversion forces a face, and both consult the
//! same table — the unfolding face together with the definitional environment's
//! transparency — which is what keeps the two policies from drifting apart.
//!
//! # The sharing boundary
//!
//! Semantic values live in a **per-run arena** ([`SemArena`]) and name each
//! other by `u32` ids. That is the whole of the sharing discipline:
//!
//! * minting is **constructor-only** — every node enters through a `mint_*`
//!   method, and there is no content-keyed table anywhere in this module, so
//!   nothing built by unfolding can be canonicalized;
//! * evaluation is free to **alias** one id, which preserves the sharing the
//!   decoder handed in without ever creating sharing that was not there;
//! * the run's values are dropped **wholesale** at its watermark
//!   ([`SemArena::truncate_to`]), so they never outlive the verdict;
//! * **id equality is positive-only evidence** — two equal ids in one arena are
//!   the same value, so conversion may answer *convertible* from an id check
//!   alone, while two unequal ids imply **nothing** and fall through to the
//!   structural walk.
//!
//! Interning belongs to syntax and to syntax only ([`intern`]).
//!
//! # Totality, and why teardown is flat in every drop order
//!
//! Every child of every node here is a `Copy` id — semantic children into this
//! arena, and **syntax children into the flat node carrier** ([`FlatArena`]).
//! No record in this module owns a reference-counted term, so the derived
//! `Clone` and `Drop` are shallow, arena teardown is a flat vector drop, and no
//! traversal recurses on term depth.
//!
//! That is a **structural** property rather than a maintained one, and the
//! distinction is the point. An earlier shape had the term face hold an
//! `Rc<Value>` and a closure hold an `Rc<Comp>`. Teardown was then flat only
//! while somebody *else* still owned the term: the syntax tree's derived `Drop`
//! recurses one call per link, so an arena holding the **last** reference to a
//! deep term freed it recursively. Releasing the normalizer before the term
//! made the symptom disappear and left the ownership defect exactly where it
//! was. A handle cannot be held that way — dropping the caller's syntax first
//! changes nothing here, because nothing here ever owned it.
//!
//! Two witnesses hold that claim, one per direction, and neither reads source
//! text: `nbe::tests::a_deep_bind_chain_teardown_is_order_independent` and
//! `nbe::tests::a_deep_pair_chain_teardown_is_order_independent` each build a
//! ten-thousand-link term, run it through every public face, release the
//! caller's term **first**, and then observe through a weak handle that the
//! release actually freed the chain — which is what distinguishes a flat arena
//! from one whose symptom was merely ordered away. Both then drop the
//! normalizer with its run still live, and both repeat the opposite order.
//!
//! [`intern`]: crate::nbe::intern
//! [`FlatArena`]: crate::syntax::FlatArena

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::boundary::ApproximateDepth;
use crate::boundary::ConstructorTag;
use crate::boundary::DefinitionHeightLevel;
use crate::boundary::GuardDecision;
use crate::boundary::HoleOccurrence;
use crate::boundary::NameRef;
use crate::boundary::NodeIndex;
use crate::boundary::RigidStatus;
use crate::boundary::SemanticHash;
use crate::boundary::SemanticNodeCount;
use crate::boundary::SemanticNodeIndex;
use crate::boundary::SpineLength;
use crate::boundary::VariableLevel;
use crate::grade::Grade;
use crate::prim::NativePrim;
use crate::syntax::CompNodeId;
use crate::syntax::HoleId;
use crate::syntax::NumLit;
use crate::syntax::Side;
use crate::syntax::StackNodeId;
use crate::syntax::ValueNodeId;
use crate::types::DataId;

/// Defines a transparent arena id with its explicit conversions and its
/// checked index projection.
macro_rules! sem_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl From<u32> for $name
        {
            #[inline]
            fn from(raw: u32) -> Self
            {
                Self(raw)
            }
        }

        impl From<$name> for u32
        {
            #[inline]
            fn from(id: $name) -> Self
            {
                id.0
            }
        }

        impl $name
        {
            /// This id's position in its node family, when the host pointer
            /// width can name it.
            #[inline]
            #[must_use]
            fn index(self) -> Option<SemanticNodeIndex>
            {
                usize::try_from(self.0).ok().map(SemanticNodeIndex::from)
            }
        }
    };
}

sem_id!(
    SemValueId,
    "Identity of a semantic value in one run's arena."
);
sem_id!(
    SemCompId,
    "Identity of a weak-head-normal semantic computation in one run's arena."
);
sem_id!(
    ClosureId,
    "Identity of a first-order closure in one run's arena."
);
sem_id!(
    EnvId,
    "Identity of an environment frame in one run's arena."
);
sem_id!(
    NeutralId,
    "Identity of a neutral computation in one run's arena."
);

/// Something the normalizer could not do because its arena said so.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SemError
{
    /// The id space of one node family is exhausted.
    IdSpaceExhausted,
    /// A semantic value id did not resolve in this arena.
    MissingValue(SemValueId),
    /// A semantic computation id did not resolve in this arena.
    MissingComp(SemCompId),
    /// A closure id did not resolve in this arena.
    MissingClosure(ClosureId),
    /// An environment id did not resolve in this arena.
    MissingEnv(EnvId),
    /// A neutral id did not resolve in this arena.
    MissingNeutral(NeutralId),
    /// A closure was applied to a different number of arguments than it binds.
    ClosureArity,
    /// A syntax value node id did not resolve in the syntax store.
    MissingSyntaxValue(ValueNodeId),
    /// A syntax computation node id did not resolve in the syntax store.
    MissingSyntaxComp(CompNodeId),
    /// Lowering a caller's term into the syntax store failed.
    SyntaxStore,
}

/// The **term face**: the source term a semantic value came from, kept exactly
/// while nothing inside the value has reduced.
///
/// Readback returns the retained term verbatim rather than rebuilding one, so
/// quoting an unreduced subterm costs a copied id instead of a traversal. The
/// face is dropped the moment any reduction fires beneath the value, which is
/// what keeps it honest: a retained term is always a term the value is still
/// equal to.
///
/// It is a **non-owning handle** into the syntax store, never a pointer that
/// owns the term. The store owns the canonical syntax; this names a node in it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TermFace
{
    /// Nothing is retained — readback rebuilds the term from the value.
    #[default]
    Rebuilt,
    /// The unreduced source node this semantic value came from.
    Retained(ValueNodeId),
}

impl TermFace
{
    /// The retained source node, when the face carries one.
    #[inline]
    #[must_use]
    pub fn retained(self) -> Option<ValueNodeId>
    {
        match self {
            | Self::Rebuilt => None,
            | Self::Retained(term) => Some(term),
        }
    }
}

/// Defines one polarity's unfolding face — the same mechanism at the value and
/// the computation polarity, differing only in what a forced face holds.
macro_rules! unfold_face {
    ($name:ident, $forced:ty, $doc:literal, $detail:literal) => {
        #[doc = $doc]
        ///
        #[doc = $detail]
        /// A neutral never loses its neutral face, so forcing chooses which
        /// face to *look at* rather than destroying one. A forced face is
        /// written back onto the node that owns it — cell-local memoization,
        /// never a content-keyed table.
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub enum $name
        {
            /// The head has no unfolding rule; this neutral is rigid.
            #[default]
            Rigid,
            /// The head has an unfolding rule at this height, not yet forced.
            Pending(DefinitionHeightLevel),
            /// The unfolded face, forced once and retained.
            Forced($forced),
            /// The head has an unfolding rule that made no case-tree progress,
            /// so the smart-unfolding verdict declines to spend it again.
            Blocked(DefinitionHeightLevel),
        }

        impl $name
        {
            /// The definitional height of the head's unfolding rule, when it
            /// has one that has not already been forced.
            #[inline]
            #[must_use]
            pub fn height(self) -> Option<DefinitionHeightLevel>
            {
                match self {
                    | Self::Rigid | Self::Forced(_) => None,
                    | Self::Pending(height) | Self::Blocked(height) => Some(height),
                }
            }

            /// Whether this face can still deliver an unfolding.
            #[inline]
            #[must_use]
            pub fn unfoldable(self) -> RigidStatus
            {
                RigidStatus::from(matches!(self, Self::Pending(_) | Self::Forced(_)))
            }
        }
    };
}

unfold_face!(
    ValueUnfold,
    SemValueId,
    "The **unfolding face** of a neutral value.",
    "It is the lazily-forced value the head's definition delivers, kept beside the neutral form rather than replacing it."
);
unfold_face!(
    CompUnfold,
    SemCompId,
    "The **unfolding face** of a neutral computation.",
    "It is the lazily-forced computation obtained by unfolding the head's definition and re-applying the recorded spine."
);

/// The intrusive **guard word** every semantic node carries: the constant-time
/// answer to "can I skip this comparison?".
///
/// This is the definitional-equality pipeline's second step, kept to what this
/// rung can honestly fill — a content hash folded from the children's cached
/// hashes at mint time, so it costs constant work per node and never walks; a
/// rigidity bit; a hole bit; and a saturating depth approximation.
///
/// # The word is only ever read negatively
///
/// Equal hashes prove nothing, because conversion is equality *up to
/// unfolding*: two values with one hash may still need the walk, and two values
/// with different hashes may still converge. The word therefore decides only
/// the **distinct** direction, and only when both sides are rigid and hole-free
/// — there is then no unfolding rule anywhere inside either value and no
/// gradual wildcard, so conversion on that pair *is* structural equality on
/// exactly the content the hash covers, and differing hashes settle it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Guard
{
    /// The folded content hash.
    hash: SemanticHash,
    /// Whether no unfolding rule is reachable anywhere inside this node.
    rigid: RigidStatus,
    /// Whether a hole occurs anywhere inside this node.
    holes: HoleOccurrence,
    /// A saturating approximation of this node's depth.
    depth: ApproximateDepth,
}

impl Guard
{
    /// The guard word of a childless node hashing `seed`.
    #[inline]
    #[must_use]
    pub fn leaf(seed: SemanticHash) -> Self
    {
        Self {
            hash: seed,
            rigid: RigidStatus::from(true),
            holes: HoleOccurrence::from(false),
            depth: ApproximateDepth::from(1),
        }
    }

    /// The folded content hash.
    #[inline]
    #[must_use]
    pub fn hash(self) -> SemanticHash
    {
        self.hash
    }

    /// Whether no unfolding rule is reachable inside the guarded node.
    #[inline]
    #[must_use]
    pub fn rigid(self) -> RigidStatus
    {
        self.rigid
    }

    /// Whether a hole occurs inside the guarded node.
    #[inline]
    #[must_use]
    pub fn holes(self) -> HoleOccurrence
    {
        self.holes
    }

    /// A saturating approximation of the guarded node's depth.
    #[inline]
    #[must_use]
    pub fn depth(self) -> ApproximateDepth
    {
        self.depth
    }

    /// Marks the guarded node as carrying a hole.
    #[inline]
    #[must_use]
    pub fn with_hole(mut self) -> Self
    {
        self.holes = HoleOccurrence::from(true);
        self
    }

    /// Marks the guarded node as carrying an unfolding rule.
    #[inline]
    #[must_use]
    pub fn with_unfolding(mut self) -> Self
    {
        self.rigid = RigidStatus::from(false);
        self
    }

    /// Folds `child` into this guard: mixes the hashes, propagates the
    /// rigidity and hole bits, and saturates the depth one level up.
    ///
    /// # Contract
    /// - ensures: the result carries a hole when either operand does, is rigid
    ///   only when both operands are, and has a depth one greater than the
    ///   deeper operand, saturating at the wrapper's maximum.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn fold(
        self,
        child: Self,
    ) -> Self
    {
        let hash = mix(self.hash, child.hash);
        let depth = u32::from(self.depth).max(u32::from(child.depth).saturating_add(1));
        Self {
            hash,
            rigid: RigidStatus::from(bool::from(self.rigid) && bool::from(child.rigid)),
            holes: HoleOccurrence::from(bool::from(self.holes) || bool::from(child.holes)),
            depth: ApproximateDepth::from(depth),
        }
    }

    /// Whether this pair of guards settles the comparison as **distinct**
    /// without any structural work.
    ///
    /// # Contract
    /// - ensures: `true` only when both guards are rigid, neither carries a
    ///   hole, and the hashes differ — the exact condition under which
    ///   conversion coincides with structural equality on hashed content.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — three decision surfaces (both-rigid, both
    ///   hole-free, hashes differ) each flip on one boundary pair, so a rigid
    ///   hole-free pair with equal hashes, a rigid hole-free pair with unequal
    ///   hashes, and an unfoldable pair with unequal hashes separate every
    ///   mutant.
    /// - witness: `nbe::tests::guard_settles_distinct_only_for_rigid_hole_free_pairs`
    #[inline]
    #[must_use]
    pub fn settles_distinct(
        self,
        other: Self,
    ) -> GuardDecision
    {
        let comparable = bool::from(self.rigid)
            && bool::from(other.rigid)
            && !bool::from(self.holes)
            && !bool::from(other.holes);
        GuardDecision::from(comparable && self.hash != other.hash)
    }
}

/// Folds one word into a running hash (FNV-1a; wrapping arithmetic is the
/// algorithm, not an overflow).
#[inline]
#[must_use]
fn mix(
    seed: SemanticHash,
    word: SemanticHash,
) -> SemanticHash
{
    let mut hash = u64::from(seed);
    for byte in u64::from(word).to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    SemanticHash::from(hash)
}

/// The hash seed for a node kind: the kind tag folded into the FNV offset
/// basis, so two different kinds with identical children cannot collide.
#[inline]
#[must_use]
pub fn seed(kind: SemanticHash) -> SemanticHash
{
    mix(SemanticHash::from(0xcbf2_9ce4_8422_2325), kind)
}

/// Folds a borrowed label into a hash, byte by byte.
#[inline]
#[must_use]
pub fn mix_str(
    accumulator: SemanticHash,
    text: NameRef<'_>,
) -> SemanticHash
{
    let mut hash = accumulator;
    for byte in text.as_ref().as_bytes() {
        hash = mix(hash, SemanticHash::from(u64::from(*byte)));
    }
    hash
}

/// Folds one word into a hash.
#[inline]
#[must_use]
pub fn mix_word(
    accumulator: SemanticHash,
    word: SemanticHash,
) -> SemanticHash
{
    mix(accumulator, word)
}

/// A byte-oriented hasher over the same fold, so a payload that is not itself a
/// term — a grade, a literal, an effect signature — folds into a hash without a
/// bespoke traversal.
#[repr(transparent)]
struct PayloadHasher
{
    /// The running hash state.
    state: u64,
}

impl core::hash::Hasher for PayloadHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        self.state
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

/// Folds a hashable payload into a hash.
///
/// # Contract
/// - ensures: the result is a function of the payload's `Hash` image alone, so
///   two payloads that hash alike fold alike on every run and on every host.
/// - panics: none.
#[inline]
#[must_use]
pub fn mix_hashable<H>(
    accumulator: SemanticHash,
    payload: &H,
) -> SemanticHash
where
    H: core::hash::Hash,
{
    let mut hasher = PayloadHasher {
        state: u64::from(accumulator),
    };
    payload.hash(&mut hasher);
    SemanticHash::from(core::hash::Hasher::finish(&hasher))
}

/// A **rigid head**: what a neutral is stuck on, named rather than merely
/// reported.
///
/// The design asks stuck values to carry *blocker identity* — which variable or
/// hole a weak-head value is stuck on — so a later solver worklist has exact
/// wake-up conditions instead of a stuck *reason*.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rigid
{
    /// A variable the normalizer generated on going under a binder, named by
    /// its de Bruijn level so alpha-equivalence is identity.
    Level(VariableLevel),
    /// A source variable with no binding in either environment.
    Free(String),
    /// A hole the elaborator has not filled.
    Hole(HoleId),
}

/// A semantic **value**: the positive fragment of the value domain, in
/// weak-head-normal form.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SemValueNode
{
    /// The unit value.
    Unit,
    /// An integer literal.
    Int(i64),
    /// A string literal.
    Str(String),
    /// A typed numeric literal.
    Num(NumLit),
    /// An eager pair.
    Pair(SemValueId, SemValueId),
    /// A sum injection.
    Inj(Side, SemValueId),
    /// A list literal.
    List(Vec<SemValueId>),
    /// A record literal — also the shape a record module's structure takes.
    Record(BTreeMap<String, SemValueId>),
    /// A graded thunk: a computation suspended over its environment.
    Thunk(Grade, ClosureId),
    /// A reflexivity witness.
    Here(SemValueId),
    /// A declared-data constructor value.
    Ctor
    {
        /// The datatype's minted nominal identity.
        id: DataId,
        /// The constructor's position in its declaration's constructor list.
        tag: ConstructorTag,
        /// The constructor's field-tuple payload.
        payload: SemValueId,
    },
    /// A reified stack: opaque to conversion by construction, named by its
    /// syntax node and compared as syntax.
    Reified(StackNodeId),
    /// A neutral value: a rigid head with no eliminator that could fire.
    ///
    /// Value neutrals carry no spine, and that is a fact about the calculus
    /// rather than a simplification: every eliminator of a positive type is a
    /// *computation*, so a frustrated elimination heads a neutral computation
    /// and never a neutral value.
    Rigid(Rigid, ValueUnfold),
}

/// A semantic value together with its two faces and its guard word.
#[derive(Clone, Debug)]
pub struct SemValue
{
    /// The unfolding face's weak-head form.
    node: SemValueNode,
    /// The term face.
    face: TermFace,
    /// The cached guard word.
    guard: Guard,
}

impl SemValue
{
    /// Pairs a node with its guard word, retaining no source term.
    #[inline]
    #[must_use]
    pub fn new(
        node: SemValueNode,
        guard: Guard,
    ) -> Self
    {
        Self {
            node,
            face: TermFace::Rebuilt,
            guard,
        }
    }

    /// Retains `term` as this value's term face.
    #[inline]
    #[must_use]
    pub fn retaining(
        mut self,
        term: ValueNodeId,
    ) -> Self
    {
        self.face = TermFace::Retained(term);
        self
    }

    /// The weak-head form.
    #[inline]
    #[must_use]
    pub fn node(&self) -> &SemValueNode
    {
        &self.node
    }

    /// The term face.
    #[inline]
    #[must_use]
    pub fn face(&self) -> TermFace
    {
        self.face
    }

    /// The guard word.
    #[inline]
    #[must_use]
    pub fn guard(&self) -> Guard
    {
        self.guard
    }
}

/// A semantic **computation** in weak-head-normal form: the negative fragment.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SemCompNode
{
    /// A function, closed over its environment.
    Lambda(ClosureId),
    /// A returner carrying its value.
    Return(SemValueId),
    /// A lazy pair, each component closed over the environment.
    LazyPair(ClosureId, ClosureId),
    /// A neutral computation: a stuck head under a spine that could not fire.
    Neutral(NeutralId),
}

/// A semantic computation together with its guard word.
#[derive(Clone, Debug)]
pub struct SemComp
{
    /// The weak-head form.
    node: SemCompNode,
    /// The cached guard word.
    guard: Guard,
}

impl SemComp
{
    /// Pairs a computation node with its guard word.
    #[inline]
    #[must_use]
    pub fn new(
        node: SemCompNode,
        guard: Guard,
    ) -> Self
    {
        Self { node, guard }
    }

    /// The weak-head form.
    #[inline]
    #[must_use]
    pub fn node(&self) -> &SemCompNode
    {
        &self.node
    }

    /// The guard word.
    #[inline]
    #[must_use]
    pub fn guard(&self) -> Guard
    {
        self.guard
    }
}

/// The head a neutral computation is stuck on.
///
/// Two families sit here and they are stuck for different reasons. A
/// **frustrated elimination** is stuck on its scrutinee and resumes the moment
/// that scrutinee becomes canonical. A **quarantined** former is stuck by
/// policy: conversion never runs an effect, a handler, or a control operator,
/// so those heads stay neutral whatever their operands do, and congruence over
/// their operands is the only equality this engine offers on them.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum NeutralHead
{
    /// Forcing a neutral value.
    Force(SemValueId),
    /// A sum elimination frustrated on its scrutinee.
    Case
    {
        /// The neutral scrutinee.
        scrutinee: SemValueId,
        /// The left branch.
        on_left: ClosureId,
        /// The right branch.
        on_right: ClosureId,
    },
    /// A declared-data elimination frustrated on its scrutinee.
    DataCase
    {
        /// The neutral scrutinee.
        scrutinee: SemValueId,
        /// The arms, in source order, each named by its constructor.
        arms: Vec<(String, ClosureId)>,
    },
    /// A list elimination frustrated on its scrutinee.
    ListCase
    {
        /// The neutral scrutinee.
        scrutinee: SemValueId,
        /// The empty-list branch.
        nil: ClosureId,
        /// The cons branch, binding head and tail.
        cons: ClosureId,
    },
    /// A pair elimination frustrated on its scrutinee.
    Split
    {
        /// The neutral scrutinee.
        scrutinee: SemValueId,
        /// The body, binding both components.
        body: ClosureId,
    },
    /// A record projection frustrated on its record — the structure-projection
    /// hole, neutral exactly while its head is not a structure.
    Project
    {
        /// The neutral record.
        record: SemValueId,
        /// The projected label.
        label: String,
    },
    /// An identity elimination frustrated on its path.
    Walk
    {
        /// The neutral path.
        scrutinee: SemValueId,
        /// The diagonal base.
        base: ClosureId,
    },
    /// A native primitive: pure, but never fired inside conversion, so it is a
    /// rigid head whose arguments are compared by congruence.
    Native
    {
        /// The primitive.
        prim: NativePrim,
        /// The evaluated arguments, in order.
        args: Vec<SemValueId>,
    },
    /// A grade duplication — quarantined.
    Dup(SemValueId),
    /// A grade discard — quarantined.
    Drop(SemValueId),
    /// An effect performance — quarantined.
    ///
    /// The signature and the operation name stay on the **syntax node**, which
    /// this head names rather than copies: a semantic record carries no owned
    /// syntax at all, and reading the sig back is one arena lookup.
    Perform
    {
        /// The syntax node this performance came from.
        source: CompNodeId,
        /// The evaluated payload.
        payload: SemValueId,
    },
    /// An effect handler — quarantined.
    ///
    /// As with a performance, the signature and the clause labels stay on the
    /// syntax node this head names.
    Handle
    {
        /// The syntax node this handler came from.
        source: CompNodeId,
        /// The handled computation, delayed.
        scrutinee: ClosureId,
        /// The return clause, binding the returned value.
        ret: ClosureId,
        /// The operation clauses, in the source node's order.
        ops: Vec<ClosureId>,
    },
    /// A resumption — quarantined.
    Resume
    {
        /// The evaluated resumption value.
        value: SemValueId,
        /// The resumed body, delayed.
        body: ClosureId,
    },
    /// A delimiter — quarantined.
    Reset(ClosureId),
    /// A capture — quarantined.
    Shift(ClosureId),
    /// A computation hole.
    Hole(HoleId),
    /// A canonical computation that met an eliminator its polarity cannot
    /// accept — a lambda under a sequencing continuation, say.
    ///
    /// The checker rejects such a term, so this head is unreachable from
    /// well-typed input; it exists because the normalizer is **total** on every
    /// term it is handed, and discarding the frustrated eliminator instead
    /// would equate terms that are not equal.
    Mismatch(SemCompId),
}

/// One frustrated eliminator in a neutral computation's spine.
#[derive(Clone, Copy, Debug)]
pub enum Elim
{
    /// Application to an argument.
    Apply(SemValueId),
    /// Projection from a lazy pair.
    Project(Side),
    /// A sequencing continuation, binding the returned value.
    Sequence(ClosureId),
}

/// A neutral computation: its head, the spine that could not fire, and the
/// unfolding face grown alongside them.
#[derive(Clone, Debug)]
pub struct Neutral
{
    /// The stuck head.
    head: NeutralHead,
    /// The frustrated eliminators, outermost last.
    spine: Vec<Elim>,
    /// The unfolding face, built incrementally as the spine grows.
    unfold: CompUnfold,
}

impl Neutral
{
    /// A neutral with an empty spine and the given unfolding face.
    #[inline]
    #[must_use]
    pub fn new(
        head: NeutralHead,
        unfold: CompUnfold,
    ) -> Self
    {
        Self {
            head,
            spine: Vec::new(),
            unfold,
        }
    }

    /// The stuck head.
    #[inline]
    #[must_use]
    pub fn head(&self) -> &NeutralHead
    {
        &self.head
    }

    /// The frustrated eliminators, outermost last.
    #[inline]
    #[must_use]
    pub fn spine(&self) -> &[Elim]
    {
        &self.spine
    }

    /// The number of frustrated eliminators.
    #[inline]
    #[must_use]
    pub fn spine_len(&self) -> SpineLength
    {
        SpineLength::from(self.spine.len())
    }

    /// The unfolding face.
    #[inline]
    #[must_use]
    pub fn unfold(&self) -> CompUnfold
    {
        self.unfold
    }

    /// Extends the spine by one eliminator, carrying the head's unfolding rule
    /// along so both faces grow together.
    ///
    /// # Contract
    /// - ensures: the result appends `elim` to the spine and keeps the same
    ///   head; a face already forced or blocked at the shorter spine is reset
    ///   to pending, because a longer spine is a different application and the
    ///   shorter one's forced value does not answer it.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the spine append and the face reset are
    ///   separated by one input each: extending a rigid neutral must leave the
    ///   face rigid, and extending a forced neutral must leave it pending at
    ///   the original height.
    /// - witness: `nbe::tests::extending_a_glued_spine_reopens_the_unfolding_face`
    #[inline]
    #[must_use]
    pub fn extended(
        &self,
        elim: Elim,
        height: Option<DefinitionHeightLevel>,
    ) -> Self
    {
        let mut spine = self.spine.clone();
        spine.push(elim);
        let unfold = match height {
            | None => CompUnfold::Rigid,
            | Some(height) => CompUnfold::Pending(height),
        };
        Self {
            head: self.head.clone(),
            spine,
            unfold,
        }
    }
}

/// A **first-order closure**: an environment and a syntax body, never a host
/// closure.
///
/// Keeping closures first-order is what leaves the machine serializable and
/// checkpointable, and it is also the forward fence a later closure-conversion
/// lane depends on: an abstract closure keeps the function's type and consumes
/// the same contexts, so beta and eta survive it, where a pair encoding changes
/// the type and stops commuting with substitution.
#[derive(Clone, Debug)]
pub struct Closure
{
    /// The captured environment.
    env: EnvId,
    /// The binders this closure abstracts, in order.
    binders: Vec<String>,
    /// The syntax body, named in the store rather than owned.
    body: CompNodeId,
    /// The memoized result of a nullary closure — the explicit thunk cell.
    ///
    /// Rust has no host laziness, and the fastest precedent's call-by-need
    /// rides exactly that, so an un-memoized closure is a measured cliff. The
    /// cell is written in place on this node and is keyed by nothing: it is the
    /// node's own result, never a content-addressed lookup.
    memo: Option<SemCompId>,
}

impl Closure
{
    /// A closure over `env` abstracting `binders` in `body`.
    #[inline]
    #[must_use]
    pub fn new(
        env: EnvId,
        binders: Vec<String>,
        body: CompNodeId,
    ) -> Self
    {
        Self {
            env,
            binders,
            body,
            memo: None,
        }
    }

    /// The captured environment.
    #[inline]
    #[must_use]
    pub fn env(&self) -> EnvId
    {
        self.env
    }

    /// The abstracted binders, in order.
    #[inline]
    #[must_use]
    pub fn binders(&self) -> &[String]
    {
        &self.binders
    }

    /// The syntax body.
    #[inline]
    #[must_use]
    pub fn body(&self) -> CompNodeId
    {
        self.body
    }

    /// The memoized nullary result, once one has been forced.
    #[inline]
    #[must_use]
    pub fn memo(&self) -> Option<SemCompId>
    {
        self.memo
    }
}

/// One environment frame: a name bound to a semantic value, over the rest.
#[derive(Clone, Debug)]
struct EnvFrame
{
    /// The bound name.
    name: String,
    /// The value bound to it.
    value: SemValueId,
    /// The enclosing environment.
    rest: EnvId,
}

/// The arena population at one instant — the watermark a run truncates back to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Watermark
{
    /// The value-node population.
    values: SemanticNodeCount,
    /// The computation-node population.
    comps: SemanticNodeCount,
    /// The closure population.
    closures: SemanticNodeCount,
    /// The environment-frame population.
    envs: SemanticNodeCount,
    /// The neutral population.
    neutrals: SemanticNodeCount,
}

/// The per-run arena holding every semantic node the normalizer mints.
///
/// # Contract
/// - requires: every id handed to a resolver was minted by *this* arena and has
///   not been truncated away.
/// - ensures: minting appends and never moves an existing node, so an id stays
///   valid for the arena's lifetime up to the next truncation.
/// - provides: the whole of a run's semantic storage, dropped in one flat
///   vector teardown.
/// - fails: a resolver returns a missing-node error rather than panicking, and
///   a minter returns [`SemError::IdSpaceExhausted`] at the id-space boundary.
/// - panics: none.
#[derive(Clone, Debug)]
pub struct SemArena
{
    /// The value nodes.
    values: Vec<SemValue>,
    /// The weak-head-normal computation nodes.
    comps: Vec<SemComp>,
    /// The closure nodes.
    closures: Vec<Closure>,
    /// The environment frames; the empty environment is the zero id and owns no
    /// frame, so a frame at vector position `n` answers to id `n + 1`.
    envs: Vec<EnvFrame>,
    /// The neutral nodes.
    neutrals: Vec<Neutral>,
}

impl Default for SemArena
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl SemArena
{
    /// The empty environment: every run's root scope.
    pub const EMPTY_ENV: EnvId = EnvId(0);

    /// A fresh, empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            values: Vec::new(),
            comps: Vec::new(),
            closures: Vec::new(),
            envs: Vec::new(),
            neutrals: Vec::new(),
        }
    }

    /// The current population, for a later [`Self::truncate_to`].
    #[inline]
    #[must_use]
    pub fn watermark(&self) -> Watermark
    {
        Watermark {
            values: SemanticNodeCount::from(self.values.len()),
            comps: SemanticNodeCount::from(self.comps.len()),
            closures: SemanticNodeCount::from(self.closures.len()),
            envs: SemanticNodeCount::from(self.envs.len()),
            neutrals: SemanticNodeCount::from(self.neutrals.len()),
        }
    }

    /// Drops every node minted after `mark`, wholesale.
    ///
    /// This is the truncation that keeps the unfolding face's built-up values
    /// out of anything durable: they allocate past the watermark and are gone
    /// the moment the verdict is in.
    ///
    /// # Contract
    /// - requires: `mark` came from [`Self::watermark`] on this arena and no
    ///   earlier truncation has already dropped below it.
    /// - ensures: every node family is cut back to its recorded population, and
    ///   ids at or below the mark keep resolving to the same nodes.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the five family truncations are separated by
    ///   minting one node in each family past a mark and observing every
    ///   population return to it exactly.
    /// - witness: `nbe::tests::truncating_to_a_watermark_drops_every_family`
    #[inline]
    pub fn truncate_to(
        &mut self,
        mark: Watermark,
    )
    {
        self.values.truncate(usize::from(mark.values));
        self.comps.truncate(usize::from(mark.comps));
        self.closures.truncate(usize::from(mark.closures));
        self.envs.truncate(usize::from(mark.envs));
        self.neutrals.truncate(usize::from(mark.neutrals));
    }

    /// The number of value nodes currently minted.
    #[inline]
    #[must_use]
    pub fn value_count(&self) -> SemanticNodeCount
    {
        SemanticNodeCount::from(self.values.len())
    }

    /// Mints a value node.
    ///
    /// # Contract
    /// - ensures: appends `value` and returns its fresh id.
    /// - fails: [`SemError::IdSpaceExhausted`] past the id-space boundary.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::IdSpaceExhausted`] when the value id space is full.
    #[inline]
    pub fn mint_value(
        &mut self,
        value: SemValue,
    ) -> Result<SemValueId, SemError>
    {
        let index = next_index(SemanticNodeCount::from(self.values.len()))?;
        self.values.push(value);
        Ok(SemValueId(u32::from(index)))
    }

    /// Mints a computation node.
    ///
    /// # Contract
    /// - ensures: appends `comp` and returns its fresh id.
    /// - fails: [`SemError::IdSpaceExhausted`] past the id-space boundary.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::IdSpaceExhausted`] when the id space is full.
    #[inline]
    pub fn mint_comp(
        &mut self,
        comp: SemComp,
    ) -> Result<SemCompId, SemError>
    {
        let index = next_index(SemanticNodeCount::from(self.comps.len()))?;
        self.comps.push(comp);
        Ok(SemCompId(u32::from(index)))
    }

    /// Mints a closure node.
    ///
    /// # Contract
    /// - ensures: appends `closure` and returns its fresh id.
    /// - fails: [`SemError::IdSpaceExhausted`] past the id-space boundary.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::IdSpaceExhausted`] when the id space is full.
    #[inline]
    pub fn mint_closure(
        &mut self,
        closure: Closure,
    ) -> Result<ClosureId, SemError>
    {
        let index = next_index(SemanticNodeCount::from(self.closures.len()))?;
        self.closures.push(closure);
        Ok(ClosureId(u32::from(index)))
    }

    /// Mints a neutral node.
    ///
    /// # Contract
    /// - ensures: appends `neutral` and returns its fresh id.
    /// - fails: [`SemError::IdSpaceExhausted`] past the id-space boundary.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::IdSpaceExhausted`] when the id space is full.
    #[inline]
    pub fn mint_neutral(
        &mut self,
        neutral: Neutral,
    ) -> Result<NeutralId, SemError>
    {
        let index = next_index(SemanticNodeCount::from(self.neutrals.len()))?;
        self.neutrals.push(neutral);
        Ok(NeutralId(u32::from(index)))
    }

    /// Extends `env` with `name` bound to `value`.
    ///
    /// # Contract
    /// - ensures: returns an environment in which `name` resolves to `value`
    ///   and every other name resolves as it did in `env`.
    /// - fails: [`SemError::IdSpaceExhausted`] past the id-space boundary.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::IdSpaceExhausted`] when the id space is full.
    #[inline]
    pub fn bind(
        &mut self,
        env: EnvId,
        name: String,
        value: SemValueId,
    ) -> Result<EnvId, SemError>
    {
        let index = next_index(SemanticNodeCount::from(self.envs.len().saturating_add(1)))?;
        self.envs.push(EnvFrame {
            name,
            value,
            rest: env,
        });
        Ok(EnvId(u32::from(index)))
    }

    /// Resolves `name` in `env`, innermost binding first.
    ///
    /// # Contract
    /// - ensures: returns the value of the innermost frame binding `name`, and
    ///   `None` when no frame binds it.
    /// - fails: [`SemError::MissingEnv`] on an id this arena did not mint.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingEnv`] when a frame id does not resolve.
    #[inline]
    pub fn lookup(
        &self,
        env: EnvId,
        name: NameRef<'_>,
    ) -> Result<Option<SemValueId>, SemError>
    {
        let mut current = env;
        while current != Self::EMPTY_ENV {
            let frame = current
                .index()
                .and_then(|index| usize::from(index).checked_sub(1))
                .and_then(|index| self.envs.get(index));
            let Some(frame) = frame
            else {
                return Err(SemError::MissingEnv(current));
            };
            if frame.name == name.as_ref() {
                return Ok(Some(frame.value));
            }
            current = frame.rest;
        }
        Ok(None)
    }

    /// Resolves a value id.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingValue`] when the id does not resolve.
    #[inline]
    pub fn value(
        &self,
        id: SemValueId,
    ) -> Result<&SemValue, SemError>
    {
        id.index()
            .and_then(|index| self.values.get(usize::from(index)))
            .ok_or(SemError::MissingValue(id))
    }

    /// Resolves a computation id.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingComp`] when the id does not resolve.
    #[inline]
    pub fn comp(
        &self,
        id: SemCompId,
    ) -> Result<&SemComp, SemError>
    {
        id.index()
            .and_then(|index| self.comps.get(usize::from(index)))
            .ok_or(SemError::MissingComp(id))
    }

    /// Resolves a closure id.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingClosure`] when the id does not resolve.
    #[inline]
    pub fn closure(
        &self,
        id: ClosureId,
    ) -> Result<&Closure, SemError>
    {
        id.index()
            .and_then(|index| self.closures.get(usize::from(index)))
            .ok_or(SemError::MissingClosure(id))
    }

    /// Resolves a neutral id.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingNeutral`] when the id does not resolve.
    #[inline]
    pub fn neutral(
        &self,
        id: NeutralId,
    ) -> Result<&Neutral, SemError>
    {
        id.index()
            .and_then(|index| self.neutrals.get(usize::from(index)))
            .ok_or(SemError::MissingNeutral(id))
    }

    /// Writes a forced unfolding face back onto a neutral — cell-local
    /// memoization on the node that owns the face.
    ///
    /// # Contract
    /// - ensures: the named neutral's unfolding face becomes `face`, and
    ///   nothing else about the node changes.
    /// - fails: [`SemError::MissingNeutral`] on an unresolvable id.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingNeutral`] when the id does not resolve.
    #[inline]
    pub fn set_unfold_face(
        &mut self,
        id: NeutralId,
        face: CompUnfold,
    ) -> Result<(), SemError>
    {
        let slot = id
            .index()
            .and_then(|index| self.neutrals.get_mut(usize::from(index)));
        let Some(neutral) = slot
        else {
            return Err(SemError::MissingNeutral(id));
        };
        neutral.unfold = face;
        Ok(())
    }

    /// Writes a forced unfolding face back onto a neutral **value** — the
    /// value-polarity twin of [`Self::set_unfold_face`].
    ///
    /// # Contract
    /// - ensures: the named value's unfolding face becomes `face` when the node
    ///   is a rigid one, and nothing changes when it is canonical (a canonical
    ///   value has no unfolding rule to record).
    /// - fails: [`SemError::MissingValue`] on an unresolvable id.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingValue`] when the id does not resolve.
    #[inline]
    pub fn set_value_unfold(
        &mut self,
        id: SemValueId,
        face: ValueUnfold,
    ) -> Result<(), SemError>
    {
        let slot = id
            .index()
            .and_then(|index| self.values.get_mut(usize::from(index)));
        let Some(value) = slot
        else {
            return Err(SemError::MissingValue(id));
        };
        if let SemValueNode::Rigid(_, ref mut current) = value.node {
            *current = face;
        }
        Ok(())
    }

    /// Writes a forced result into a nullary closure's thunk cell.
    ///
    /// # Contract
    /// - ensures: the named closure memoizes `result`, and nothing else
    ///   changes.
    /// - fails: [`SemError::MissingClosure`] on an unresolvable id.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::MissingClosure`] when the id does not resolve.
    #[inline]
    pub fn set_closure_memo(
        &mut self,
        id: ClosureId,
        result: SemCompId,
    ) -> Result<(), SemError>
    {
        let slot = id
            .index()
            .and_then(|index| self.closures.get_mut(usize::from(index)));
        let Some(closure) = slot
        else {
            return Err(SemError::MissingClosure(id));
        };
        closure.memo = Some(result);
        Ok(())
    }
}

/// The next arena id for a family of the given current population.
#[inline]
fn next_index(len: SemanticNodeCount) -> Result<NodeIndex, SemError>
{
    u32::try_from(usize::from(len))
        .map(NodeIndex::from)
        .map_err(|_error| SemError::IdSpaceExhausted)
}
