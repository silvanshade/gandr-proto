//! The normalizer's **syntax** interner: one deduplicating table per
//! differential face, and nothing else.
//!
//! # What this is, stated as sharply as the ruling states it
//!
//! Interning is for **static, beta-free data**, and syntax is the only such
//! data the normalizer handles. Semantic values are never interned: they live
//! in [`SemArena`] as ids, they are aliased rather than canonicalized, and
//! anything built by unfolding is scratch that the watermark drops. Putting an
//! unfolding-built value in a table would be exactly the canonicalization under
//! beta the architecture forbids.
//!
//! # One table per face — they never merge
//!
//! There are two provenance classes and each gets its **own** table:
//!
//! * [`Face::ElaborationInput`] — terms the elaborator hands the normalizer;
//! * [`Face::ReadbackNormalForm`] — normal forms readback produces.
//!
//! A hit in a face's table proves the two terms are alpha-identical **within
//! that face** and proves nothing else. No lookup may establish equality across
//! faces: that an input term and a normal form intern to the same key is not a
//! fact this module will produce, because they are never compared here.
//! Equality between an input and a normal form comes from the checker's
//! structural and semantic comparison, and from nowhere else.
//!
//! # The key
//!
//! A key is the term's **full structural content in canonical binder form** — a
//! token stream in which every binder emits one fixed token and every bound
//! occurrence emits its de Bruijn index, so two alpha-equivalent terms produce
//! the same stream and two alpha-distinct terms do not. Free occurrences emit
//! their name. A key contains no semantic-arena id, no address, no generation,
//! no session datum, and nothing about any output the normalizer produced; the
//! **syntax node ids it walks never enter it**, only the content they name.
//!
//! **Two binder namespaces, two scopes.** Terms bind value variables and a
//! package signature binds abstract type components, and the traversal keeps a
//! scope for each. Collapsing them is not a rounding error: a package binding
//! `t` would capture a *value* variable named `t` occurring in an identity
//! endpoint inside its payload, and two terms whose free value variables differ
//! would then intern to one entry — which, in a table that replaces a term with
//! its canonical representative, is a wrong answer rather than a missed hit.
//!
//! The table is a deduplicator, not an equality oracle. It is disjoint from the
//! type interner ([`gandr_core_term::intern`]) and it takes nothing into the
//! trusted base.
//!
//! [`SemArena`]: crate::sem::SemArena

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use gandr_core_term::boundary::BinderName;
use gandr_core_term::boundary::BinderScope;
use gandr_core_term::boundary::InternedSyntaxCount;
use gandr_core_term::boundary::SemanticHash;
use gandr_core_term::boundary::SemanticNodeCount;
use gandr_core_term::boundary::ValueEquality;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::CompTypeNode;
use gandr_core_term::syntax::CompTypeNodeId;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::StackNode;
use gandr_core_term::syntax::StackNodeId;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;
use gandr_core_term::syntax::ValueTypeNode;
use gandr_core_term::syntax::ValueTypeNodeId;

use crate::sem::mix_hashable;
use crate::sem::mix_word;
use crate::sem::seed;

/// One token in a canonical key's stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalToken(u64);

impl From<u64> for CanonicalToken
{
    #[inline]
    fn from(raw: u64) -> Self
    {
        Self(raw)
    }
}

impl From<CanonicalToken> for u64
{
    #[inline]
    fn from(token: CanonicalToken) -> Self
    {
        token.0
    }
}

/// A term's full structural content in canonical binder form.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalKey(Vec<CanonicalToken>);

impl CanonicalKey
{
    /// The key's folded digest, used as the table's bucket index.
    ///
    /// Named for what it is rather than `hash`, which on this type would shadow
    /// the derived hashing implementation's own method.
    #[inline]
    #[must_use]
    pub fn digest(&self) -> SemanticHash
    {
        let mut hash = seed(SemanticHash::from(0x4b45_5900));
        for token in &self.0 {
            hash = mix_word(hash, SemanticHash::from(u64::from(*token)));
        }
        hash
    }
}

/// Which differential face a term belongs to.
///
/// The faces have separate tables and are never compared with each other. The
/// distinction is provenance, not shape: the same syntax arriving from the
/// elaborator and arriving from readback is two entries, one per table.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Face
{
    /// A term the elaborator handed the normalizer.
    ElaborationInput,
    /// A normal form readback produced.
    ReadbackNormalForm,
}

/// One face's deduplicating table.
#[derive(Clone, Debug, Default)]
struct FaceTable
{
    /// Canonical entries, bucketed by key digest and disambiguated by the key.
    buckets: BTreeMap<u64, Vec<(CanonicalKey, ValueNodeId)>>,
    /// The number of canonical entries.
    count: usize,
}

impl FaceTable
{
    /// Returns the canonical representative for `term`, inserting it when this
    /// face has not seen an alpha-identical term before.
    fn intern(
        &mut self,
        arena: &FlatArena,
        term: ValueNodeId,
    ) -> ValueNodeId
    {
        let key = canonical_key(arena, term);
        let bucket = self.buckets.entry(u64::from(key.digest())).or_default();
        for entry in &*bucket {
            if entry.0 == key {
                return entry.1;
            }
        }
        bucket.push((key, term));
        self.count = self.count.saturating_add(1);
        term
    }
}

/// The normalizer's syntax interner: one table per differential face.
///
/// # Contract
/// - requires: the caller names the face a term came from; a term from one face
///   is never offered to another face's table.
/// - ensures: [`Self::intern`] returns a representative that is alpha-identical
///   to its argument and identical for every alpha-identical term previously
///   interned **into the same face**.
/// - provides: sharing across repeated conversion and readback calls, with no
///   effect on any answer — dropping the interner changes nothing but
///   allocation counts.
/// - panics: none.
#[derive(Clone, Debug, Default)]
pub struct SyntaxInterner
{
    /// The elaboration-input face's table.
    input: FaceTable,
    /// The readback-normal-form face's table.
    readback: FaceTable,
}

impl SyntaxInterner
{
    /// An interner with both faces empty.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The number of canonical entries in one face's table.
    #[inline]
    #[must_use]
    pub fn len(
        &self,
        face: Face,
    ) -> InternedSyntaxCount
    {
        let count = match face {
            | Face::ElaborationInput => self.input.count,
            | Face::ReadbackNormalForm => self.readback.count,
        };
        InternedSyntaxCount::from(count)
    }

    /// Whether one face's table is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(
        &self,
        face: Face,
    ) -> gandr_core_term::boundary::InternerEmptyStatus
    {
        gandr_core_term::boundary::InternerEmptyStatus::from(usize::from(self.len(face)) == 0)
    }

    /// Interns `term` into `face`'s table and returns that face's canonical
    /// representative.
    ///
    /// # Contract
    /// - ensures: the result is alpha-identical to `term`; two alpha-identical
    ///   terms interned into the **same** face return the same node id; a term
    ///   interned into one face never returns a representative belonging to the
    ///   other face.
    /// - provides: deduplication only — the answer of every conversion,
    ///   evaluation, and readback is identical with the interner bypassed.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — three decision surfaces, separated pointwise: an
    ///   alpha-equivalent pair in one face must share a representative, an
    ///   alpha-distinct pair in one face must not, and the same term interned
    ///   into both faces must yield two representatives rather than one.
    /// - witness: `crate::tests::interning_shares_alpha_equivalent_terms_within_a_face`
    /// - witness: `crate::tests::interning_keeps_the_two_faces_disjoint`
    #[inline]
    pub fn intern(
        &mut self,
        arena: &FlatArena,
        face: Face,
        term: ValueNodeId,
    ) -> ValueNodeId
    {
        match face {
            | Face::ElaborationInput => self.input.intern(arena, term),
            | Face::ReadbackNormalForm => self.readback.intern(arena, term),
        }
    }
}

/// One pending step in the canonical-key traversal.
enum KeyTask<'term>
{
    /// Emit the tokens of a value node.
    Value(ValueNodeId),
    /// Emit the tokens of a computation node.
    Comp(CompNodeId),
    /// Emit the tokens of a stack node.
    Stack(StackNodeId),
    /// Emit the tokens of a value type node.
    ValueType(ValueTypeNodeId),
    /// Emit the tokens of a computation type node.
    CompType(CompTypeNodeId),
    /// Push one binder onto the canonical value scope.
    PushOne(&'term str),
    /// Push two binders onto the canonical value scope, the second innermost.
    PushTwo(&'term str, &'term str),
    /// Pop this many binders off the canonical value scope.
    Pop(usize),
    /// Push these type binders onto the canonical type scope, in order, the
    /// last innermost.
    PushTypes(&'term [String]),
    /// Pop this many binders off the canonical type scope.
    PopTypes(usize),
    /// Emit a name's bytes.
    Name(&'term str),
}

/// The canonical-key traversal's node tags. Distinct tags for distinct node
/// kinds is what stops two different shapes sharing a stream.
mod tag
{
    /// The tag of a bound variable occurrence; the next token is its index.
    pub const BOUND: u64 = 1;
    /// The tag of a free variable occurrence; its name bytes follow.
    pub const FREE: u64 = 2;
    /// The tag closing a name's bytes.
    pub const NAME_END: u64 = 3;
    /// The tag of a node id that did not resolve: a key still forms, and a
    /// dangling id is equal to nothing that resolves.
    pub const DANGLING: u64 = 4;
    /// The base of the value-node tag block.
    pub const VALUE: u64 = 0x1000;
    /// The base of the computation-node tag block.
    pub const COMP: u64 = 0x2000;
    /// The base of the stack-node tag block.
    pub const STACK: u64 = 0x3000;
    /// The base of the value-type tag block.
    pub const VALUE_TYPE: u64 = 0x4000;
    /// The base of the computation-type tag block.
    pub const COMP_TYPE: u64 = 0x5000;
    /// The tag of a payload word folded in verbatim.
    pub const WORD: u64 = 0x6000;
}

/// The canonical key of the term rooted at `term`: its full structural content
/// in canonical binder form.
///
/// # Contract
/// - ensures: two alpha-equivalent terms produce equal keys, and two terms that
///   differ in anything but binder names produce different keys; the key
///   mentions no node id, no semantic-arena id, no address, no generation, and
///   no session datum — only the content the ids name.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 for the two binder namespaces, separated pointwise: one
///   term renaming a package's abstract component, which must key the same, and
///   one renaming the free value variable it could capture, which must not.
/// - witness: `crate::tests::interning_shares_alpha_equivalent_terms_within_a_face`
/// - witness: `crate::tests::a_package_binder_binds_type_atoms_without_capturing_a_value_variable`
///
/// # Termination
/// - reason: the traversal drains an explicit task stack over one finite node
///   graph.
/// - measure: pending tasks on the stack.
/// - boundedness: node graphs are finite and each node queues tasks only for
///   its own children.
/// - input recursion: none.
#[inline]
#[must_use]
pub fn canonical_key(
    arena: &FlatArena,
    term: ValueNodeId,
) -> CanonicalKey
{
    key_from(arena, KeyTask::Value(term))
}

/// The canonical key of a reified stack — how two opaque stacks are compared
/// without reading either back.
#[inline]
#[must_use]
pub fn canonical_stack_key(
    arena: &FlatArena,
    stack: StackNodeId,
) -> CanonicalKey
{
    key_from(arena, KeyTask::Stack(stack))
}

/// The canonical key of a value type — how a type carried inside a term is
/// compared without a type-level equality of its own.
///
/// The package former puts types inside values in two places: the witnesses a
/// packed module carries, and the signature a package elimination ascribes.
/// Neither is evaluated, so both are compared exactly as a reified stack is —
/// alpha-identity of syntax, through this one key builder.
#[inline]
#[must_use]
pub fn canonical_value_type_key(
    arena: &FlatArena,
    ty: ValueTypeNodeId,
) -> CanonicalKey
{
    key_from(arena, KeyTask::ValueType(ty))
}

/// Whether two value types carried inside terms are the same type: identical
/// ids, or α-identical syntax through the canonical key.
///
/// This is **embedded canonical witness identity** — the one comparison every
/// engine that meets a type inside a value performs, because such a type is
/// never evaluated and the key is the only equality on offer: conversion on a
/// packed module's witnesses and on an unpack's ascribed signature
/// ([`crate::conv`]), and the unifier's pack congruence
/// (`gandr_core_unify`). Sharing the one function is what keeps the solver's
/// verdict and conversion's verdict the same relation on this input.
#[inline]
#[must_use]
pub fn canonically_equal_value_types(
    arena: &FlatArena,
    left: ValueTypeNodeId,
    right: ValueTypeNodeId,
) -> ValueEquality
{
    ValueEquality::from(
        left == right
            || canonical_value_type_key(arena, left) == canonical_value_type_key(arena, right),
    )
}

/// Drains the canonical-key traversal from one root task.
///
/// # The two scopes are disjoint, and that is a correctness property
///
/// Value variables and type binders are separate namespaces, so the traversal
/// carries a scope for each. Sharing one stack would not merely blur an index:
/// a package binding `t` would put `t` in scope for a **value** occurrence of
/// `t` in a [`ValueTypeNode::Path`] endpoint inside its payload, so two terms
/// whose free value variables genuinely differ would emit one stream — and a
/// key hit is where this table substitutes one term for the other.
fn key_from(
    arena: &FlatArena,
    root: KeyTask<'_>,
) -> CanonicalKey
{
    let mut tokens = Vec::new();
    let mut scope: Vec<&str> = Vec::new();
    let mut types: Vec<&str> = Vec::new();
    let mut work = alloc::vec![root];
    while let Some(task) = work.pop() {
        match task {
            | KeyTask::Name(name) => {
                for byte in name.as_bytes() {
                    tokens.push(CanonicalToken::from(u64::from(*byte)));
                }
                tokens.push(CanonicalToken::from(tag::NAME_END));
            },
            | KeyTask::PushOne(binder) => scope.push(binder),
            | KeyTask::PushTwo(outer, inner) => {
                scope.push(outer);
                scope.push(inner);
            },
            | KeyTask::Pop(count) => {
                for _ in 0 .. count {
                    scope.pop();
                }
            },
            | KeyTask::PushTypes(binders) => {
                types.extend(binders.iter().map(String::as_str));
            },
            | KeyTask::PopTypes(count) => {
                for _ in 0 .. count {
                    types.pop();
                }
            },
            | KeyTask::Value(id) => match arena.values.get(id) {
                | Some(node) => visit_value(
                    node,
                    BinderScope::from(scope.as_slice()),
                    &mut tokens,
                    &mut work,
                ),
                | None => tokens.push(CanonicalToken::from(tag::DANGLING)),
            },
            | KeyTask::Comp(id) => match arena.comps.get(id) {
                | Some(node) => visit_comp(node, &mut tokens, &mut work),
                | None => tokens.push(CanonicalToken::from(tag::DANGLING)),
            },
            | KeyTask::Stack(id) => match arena.stacks.get(id) {
                | Some(node) => visit_stack(node, &mut tokens, &mut work),
                | None => tokens.push(CanonicalToken::from(tag::DANGLING)),
            },
            | KeyTask::ValueType(id) => match arena.value_types.get(id) {
                | Some(node) => visit_value_type(
                    node,
                    BinderScope::from(types.as_slice()),
                    &mut tokens,
                    &mut work,
                ),
                | None => tokens.push(CanonicalToken::from(tag::DANGLING)),
            },
            | KeyTask::CompType(id) => match arena.comp_types.get(id) {
                | Some(node) => visit_comp_type(node, &mut tokens, &mut work),
                | None => tokens.push(CanonicalToken::from(tag::DANGLING)),
            },
        }
    }
    CanonicalKey(tokens)
}

/// Emits `word` as a payload token pair.
fn payload(
    tokens: &mut Vec<CanonicalToken>,
    word: CanonicalToken,
)
{
    tokens.push(CanonicalToken::from(tag::WORD));
    tokens.push(word);
}

/// Folds a hashable payload into one token, for leaves whose content is not
/// itself a term.
fn hashed<H>(
    tokens: &mut Vec<CanonicalToken>,
    value: &H,
) where
    H: core::hash::Hash,
{
    let hash = mix_hashable(seed(SemanticHash::from(0x5041_594c)), value);
    payload(tokens, CanonicalToken::from(u64::from(hash)));
}

/// Emits a child count or a de Bruijn index as a payload token.
fn length(
    tokens: &mut Vec<CanonicalToken>,
    count: SemanticNodeCount,
)
{
    payload(
        tokens,
        CanonicalToken::from(u64::try_from(usize::from(count)).unwrap_or(u64::MAX)),
    );
}

/// Emits the canonical tokens of one value node and queues its children.
fn visit_value<'term>(
    node: &'term ValueNode,
    scope: BinderScope<'_>,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *node {
        | ValueNode::Var(ref name) => {
            let bound = scope
                .as_ref()
                .iter()
                .rev()
                .position(|open| *open == name.as_str());
            match bound {
                | Some(index) => {
                    tokens.push(CanonicalToken::from(tag::BOUND));
                    length(tokens, SemanticNodeCount::from(index));
                },
                | None => {
                    tokens.push(CanonicalToken::from(tag::FREE));
                    work.push(KeyTask::Name(name.as_str()));
                },
            }
        },
        | ValueNode::Unit => tokens.push(CanonicalToken::from(tag::VALUE)),
        | ValueNode::Int(literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(1)));
            hashed(tokens, &literal);
        },
        | ValueNode::Str(ref literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(2)));
            work.push(KeyTask::Name(literal.as_str()));
        },
        | ValueNode::Num(literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(3)));
            hashed(tokens, &literal);
        },
        | ValueNode::Pair(fst, snd) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(4)));
            work.push(KeyTask::Value(snd));
            work.push(KeyTask::Value(fst));
        },
        | ValueNode::Inj(side, carried) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(5)));
            hashed(tokens, &side);
            work.push(KeyTask::Value(carried));
        },
        | ValueNode::List(ref elements) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(6)));
            length(tokens, SemanticNodeCount::from(elements.len()));
            for element in elements.iter().rev() {
                work.push(KeyTask::Value(*element));
            }
        },
        | ValueNode::Record(ref fields) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(7)));
            length(tokens, SemanticNodeCount::from(fields.len()));
            for (label, field) in fields.iter().rev() {
                work.push(KeyTask::Value(*field));
                work.push(KeyTask::Name(label.as_str()));
            }
        },
        | ValueNode::Thunk(grade, body) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(8)));
            hashed(tokens, &grade);
            work.push(KeyTask::Comp(body));
        },
        | ValueNode::Run(body) => {
            // The one value form whose only child is a computation and which
            // carries no attribute of its own: the key is the tag and the
            // computation's own key.
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(15)));
            work.push(KeyTask::Comp(body));
        },
        | ValueNode::Annot(inner, ty) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(9)));
            work.push(KeyTask::ValueType(ty));
            work.push(KeyTask::Value(inner));
        },
        | ValueNode::Hole(id) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(10)));
            hashed(tokens, &id);
        },
        | ValueNode::Stk(stack) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(11)));
            work.push(KeyTask::Stack(stack));
        },
        | ValueNode::Here(witness) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(12)));
            work.push(KeyTask::Value(witness));
        },
        | ValueNode::Ctor {
            ref id,
            tag: constructor,
            payload: carried,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(13)));
            hashed(tokens, id);
            length(tokens, SemanticNodeCount::from(constructor));
            work.push(KeyTask::Value(carried));
        },
        | ValueNode::Pack {
            ref witnesses,
            payload,
        } => {
            // The witnesses are content, not annotation: two packs at different
            // representations are different terms, so the arity and every
            // witness type enter the stream ahead of the payload.
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(14)));
            length(tokens, SemanticNodeCount::from(witnesses.len()));
            work.push(KeyTask::Value(payload));
            for witness in witnesses.iter().rev() {
                work.push(KeyTask::ValueType(*witness));
            }
        },
    }
}

/// Emits the canonical tokens of one computation node and queues its children,
/// bracketing every binder it opens.
fn visit_comp<'term>(
    node: &'term CompNode,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    /// Queues a body under one binder, bracketed by the scope push and pop.
    fn under<'term>(
        work: &mut Vec<KeyTask<'term>>,
        binder: BinderName<'term>,
        body: CompNodeId,
    )
    {
        work.push(KeyTask::Pop(1));
        work.push(KeyTask::Comp(body));
        work.push(KeyTask::PushOne(binder.into()));
    }

    match *node {
        | CompNode::Abs(ref binder, _, body) => {
            // The optional ascription is inert for the canonical key exactly as
            // it is inert for evaluation: an annotation names a type, and two
            // terms differing only in it denote one value.
            tokens.push(CanonicalToken::from(tag::COMP));
            under(work, BinderName::from(binder.as_str()), body);
        },
        | CompNode::App(head, arg) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(1)));
            work.push(KeyTask::Value(arg));
            work.push(KeyTask::Comp(head));
        },
        | CompNode::Ret(carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(2)));
            work.push(KeyTask::Value(carried));
        },
        | CompNode::Bind(bound, ref binder, cont) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(3)));
            under(work, BinderName::from(binder.as_str()), cont);
            work.push(KeyTask::Comp(bound));
        },
        | CompNode::Force(carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(4)));
            work.push(KeyTask::Value(carried));
        },
        | CompNode::Case(scrut, ref left, ref right) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(5)));
            under(work, BinderName::from(right.0.as_str()), right.1);
            under(work, BinderName::from(left.0.as_str()), left.1);
            work.push(KeyTask::Value(scrut));
        },
        | CompNode::DataCase { scrut, ref arms } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(6)));
            length(tokens, SemanticNodeCount::from(arms.len()));
            for arm in arms.iter().rev() {
                under(work, BinderName::from(arm.0.as_str()), arm.1);
            }
            work.push(KeyTask::Value(scrut));
        },
        | CompNode::ListCase {
            scrut,
            nil,
            ref head,
            ref tail,
            cons,
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(7)));
            work.push(KeyTask::Pop(2));
            work.push(KeyTask::Comp(cons));
            work.push(KeyTask::PushTwo(head.as_str(), tail.as_str()));
            work.push(KeyTask::Comp(nil));
            work.push(KeyTask::Value(scrut));
        },
        | CompNode::Split {
            scrut,
            ref fst_name,
            ref snd_name,
            body,
            ..
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(8)));
            work.push(KeyTask::Pop(2));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::PushTwo(fst_name.as_str(), snd_name.as_str()));
            work.push(KeyTask::Value(scrut));
        },
        | CompNode::Unpack {
            scrut,
            signature,
            ref atoms,
            ref binder,
            body,
        } => {
            // The atoms are nominal identities the elaborator minted, so they
            // are content and are folded in; the ascribed signature is a term
            // child like any other type child. The module variable is the one
            // binder this node opens, and it binds a **value**.
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(22)));
            length(tokens, SemanticNodeCount::from(atoms.len()));
            for atom in atoms {
                hashed(tokens, atom);
            }
            under(work, BinderName::from(binder.as_str()), body);
            work.push(KeyTask::ValueType(signature));
            work.push(KeyTask::Value(scrut));
        },
        | CompNode::Fix(ref binder, body) => {
            // One binder over one computation child, exactly as `Shift`: the
            // self-reference binds a **value** (the thunk of the knot), so the
            // key opens the same single-name scope.
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(23)));
            under(work, BinderName::from(binder.as_str()), body);
        },
        | CompNode::RecordProj { record, ref label } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(9)));
            work.push(KeyTask::Value(record));
            work.push(KeyTask::Name(label.as_str()));
        },
        | CompNode::With(fst, snd) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(10)));
            work.push(KeyTask::Comp(snd));
            work.push(KeyTask::Comp(fst));
        },
        | CompNode::Prj(side, body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(11)));
            hashed(tokens, &side);
            work.push(KeyTask::Comp(body));
        },
        | CompNode::Dup(carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(12)));
            work.push(KeyTask::Value(carried));
        },
        | CompNode::Drop(carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(13)));
            work.push(KeyTask::Value(carried));
        },
        | CompNode::Perform(ref sig, ref op, carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(14)));
            hashed(tokens, sig.as_ref());
            work.push(KeyTask::Value(carried));
            work.push(KeyTask::Name(op.as_str()));
        },
        | CompNode::Handle {
            ref sig,
            scrutinee,
            ref ret,
            ref ops,
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(15)));
            hashed(tokens, sig.as_ref());
            length(tokens, SemanticNodeCount::from(ops.len()));
            for clause in ops.iter().rev() {
                work.push(KeyTask::Pop(2));
                work.push(KeyTask::Comp(clause.body));
                work.push(KeyTask::PushTwo(
                    clause.payload.as_str(),
                    clause.resume.as_str(),
                ));
                work.push(KeyTask::Name(clause.op.as_str()));
            }
            under(work, BinderName::from(ret.0.as_str()), ret.1);
            work.push(KeyTask::Comp(scrutinee));
        },
        | CompNode::Resume(carried, body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(16)));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::Value(carried));
        },
        | CompNode::Reset(body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(17)));
            work.push(KeyTask::Comp(body));
        },
        | CompNode::Shift(ref binder, body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(18)));
            under(work, BinderName::from(binder.as_str()), body);
        },
        | CompNode::Hole(id) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(19)));
            hashed(tokens, &id);
        },
        | CompNode::Native { prim, ref args } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(20)));
            hashed(tokens, &prim);
            length(tokens, SemanticNodeCount::from(args.len()));
            for arg in args.iter().rev() {
                work.push(KeyTask::Value(*arg));
            }
        },
        | CompNode::Walk {
            scrut, ref base, ..
        } => {
            // The motive is a type ascription on the eliminator and contributes
            // nothing to the value it computes, so it is erased here for the
            // same reason an ascription is.
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(21)));
            under(work, BinderName::from(base.x.as_str()), base.body);
            work.push(KeyTask::Value(scrut));
        },
    }
}

/// Emits the canonical tokens of one stack node and queues its children.
fn visit_stack<'term>(
    node: &'term StackNode,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *node {
        | StackNode::Empty => tokens.push(CanonicalToken::from(tag::STACK)),
        | StackNode::Arg(arg, rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(1)));
            work.push(KeyTask::Stack(rest));
            work.push(KeyTask::Value(arg));
        },
        | StackNode::Bind(ref binder, body, rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(2)));
            work.push(KeyTask::Stack(rest));
            work.push(KeyTask::Pop(1));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::PushOne(binder.as_str()));
        },
        | StackNode::Prj(side, rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(3)));
            hashed(tokens, &side);
            work.push(KeyTask::Stack(rest));
        },
    }
}

/// Emits the canonical tokens of one value type node and queues its children.
///
/// `types` is the **type**-binder scope, disjoint from the value scope the
/// term walk carries: the only binder over types is a package's abstract
/// component list, and the only occurrence it binds is an
/// [`ValueTypeNode::Atom`].
fn visit_value_type<'term>(
    node: &'term ValueTypeNode,
    types: BinderScope<'_>,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *node {
        | ValueTypeNode::Atom(ref name) => {
            // An atom is a rigid type name until a package binds it, so the
            // occurrence resolves exactly as a variable does: bound occurrences
            // emit their index and free ones emit their name.
            let bound = types
                .as_ref()
                .iter()
                .rev()
                .position(|open| *open == name.as_str());
            match bound {
                | Some(index) => {
                    tokens.push(CanonicalToken::from(tag::BOUND));
                    length(tokens, SemanticNodeCount::from(index));
                },
                | None => {
                    tokens.push(CanonicalToken::from(tag::VALUE_TYPE));
                    work.push(KeyTask::Name(name.as_str()));
                },
            }
        },
        | ValueTypeNode::Unit => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(1)));
        },
        | ValueTypeNode::Prod(fst, snd) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(2)));
            work.push(KeyTask::ValueType(snd));
            work.push(KeyTask::ValueType(fst));
        },
        | ValueTypeNode::Sum(lhs, rhs) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(3)));
            work.push(KeyTask::ValueType(rhs));
            work.push(KeyTask::ValueType(lhs));
        },
        | ValueTypeNode::List(element) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(4)));
            work.push(KeyTask::ValueType(element));
        },
        | ValueTypeNode::Record(ref fields) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(5)));
            length(tokens, SemanticNodeCount::from(fields.len()));
            for (label, field) in fields.iter().rev() {
                work.push(KeyTask::ValueType(*field));
                work.push(KeyTask::Name(label.as_str()));
            }
        },
        | ValueTypeNode::Thunk(grade, body) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(6)));
            hashed(tokens, &grade);
            work.push(KeyTask::CompType(body));
        },
        | ValueTypeNode::Stk(consumes, delivers) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(7)));
            work.push(KeyTask::CompType(delivers));
            work.push(KeyTask::CompType(consumes));
        },
        | ValueTypeNode::Path {
            ty: carrier,
            lhs,
            rhs,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(8)));
            work.push(KeyTask::Value(rhs));
            work.push(KeyTask::Value(lhs));
            work.push(KeyTask::ValueType(carrier));
        },
        | ValueTypeNode::Data { ref id, ref args } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(9)));
            hashed(tokens, id);
            length(tokens, SemanticNodeCount::from(args.len()));
            for arg in args.iter().rev() {
                work.push(KeyTask::ValueType(*arg));
            }
        },
        | ValueTypeNode::Family { ref head, ref args } => {
            // A neutral type spine: head bytes, arity, then the argument
            // values, which are ordinary value keys. Emitting the arity before
            // the arguments is what stops `H(a, b)` and a differently-shaped
            // spine with the same argument stream sharing a key.
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(15)));
            length(tokens, SemanticNodeCount::from(args.len()));
            for arg in args.iter().rev() {
                work.push(KeyTask::Value(*arg));
            }
            work.push(KeyTask::Name(head.as_str()));
        },
        | ValueTypeNode::Universe {
            ref sort,
            ref level,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(10)));
            hashed(tokens, sort);
            hashed(tokens, level);
        },
        | ValueTypeNode::Sigma {
            fst,
            ref binder,
            snd,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(11)));
            work.push(KeyTask::Pop(1));
            work.push(KeyTask::ValueType(snd));
            work.push(KeyTask::PushOne(binder.as_str()));
            work.push(KeyTask::ValueType(fst));
        },
        | ValueTypeNode::Sealed(ref id) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(12)));
            hashed(tokens, id);
        },
        | ValueTypeNode::Package {
            grade,
            ref abstracts,
            payload,
        } => {
            // The abstract components are binders over the payload, discharged
            // by `gandr_core_checker::judgements::package::instantiate`, so the labels
            // never reach the stream: they open the type scope and their
            // occurrences emit an index. Two signatures that differ only in how
            // they spell a component are one key, which is the same relation
            // subtyping decides by instantiating both sides at canonical
            // binders.
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(14)));
            hashed(tokens, &grade);
            length(tokens, SemanticNodeCount::from(abstracts.len()));
            work.push(KeyTask::PopTypes(abstracts.len()));
            work.push(KeyTask::ValueType(payload));
            work.push(KeyTask::PushTypes(abstracts.as_slice()));
        },
        | ValueTypeNode::Unknown => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(13)));
        },
    }
}

/// Emits the canonical tokens of one computation type node and queues its
/// children.
fn visit_comp_type<'term>(
    node: &'term CompTypeNode,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *node {
        | CompTypeNode::F(of, ref row) => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE));
            hashed(tokens, row);
            work.push(KeyTask::ValueType(of));
        },
        | CompTypeNode::Arrow {
            ref binder,
            arg,
            res,
        } => {
            // One tag for both spellings, because `Π(x : A). B` with an unused
            // `x` and `A → B` are one type (`gandr_core_nbe::conv`'s binder
            // alignment decides that). A written binder opens the canonical
            // value scope over the codomain, exactly as a package's labels open
            // the type scope over its payload, so the binder's *name* never
            // reaches the stream and two spellings of one function type share a
            // key.
            //
            // **Incompleteness, stated rather than discovered.** A written but
            // unused binder still shifts the index of anything bound further
            // out, so `Π(x : A). B` and `A → B` can take different keys when
            // `B` mentions an enclosing binder. That costs a missed fast path
            // and never a wrong answer: the key stays a positive-only early-out
            // and full conversion decides the pair. Nothing here can merge two
            // distinct types, which is the property the key owes.
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(1)));
            match binder.as_deref() {
                | Some(bound) => {
                    work.push(KeyTask::Pop(1));
                    work.push(KeyTask::CompType(res));
                    work.push(KeyTask::PushOne(bound));
                },
                | None => work.push(KeyTask::CompType(res)),
            }
            work.push(KeyTask::ValueType(arg));
        },
        | CompTypeNode::With(fst, snd) => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(2)));
            work.push(KeyTask::CompType(snd));
            work.push(KeyTask::CompType(fst));
        },
        | CompTypeNode::Unknown => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(3)));
        },
    }
}
