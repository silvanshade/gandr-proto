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
//! their name. A key contains no arena id, no arena index, no address, no
//! generation, no session datum, and nothing about any output the normalizer
//! produced.
//!
//! The table is a deduplicator, not an equality oracle. It is disjoint from the
//! type interner ([`crate::intern`]) and it takes nothing into the trusted
//! base.
//!
//! [`SemArena`]: crate::nbe::sem::SemArena

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::boundary::BinderName;
use crate::boundary::BinderScope;
use crate::boundary::InternedSyntaxCount;
use crate::boundary::SemanticHash;
use crate::nbe::sem::mix_hashable;
use crate::nbe::sem::mix_word;
use crate::nbe::sem::seed;
use crate::syntax::Comp;
use crate::syntax::Stack;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::ValueType;

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
    /// Named for what it is rather than `hash`, which on this type would
    /// shadow the derived hashing implementation's own method.
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
    /// Canonical entries, bucketed by key hash and disambiguated by the key.
    buckets: BTreeMap<u64, Vec<(CanonicalKey, Rc<Value>)>>,
    /// The number of canonical entries.
    count: usize,
}

impl FaceTable
{
    /// Returns the canonical representative for `term`, inserting it when this
    /// face has not seen an alpha-identical term before.
    fn intern(
        &mut self,
        term: Rc<Value>,
    ) -> Rc<Value>
    {
        let key = canonical_key(&term);
        let bucket = self.buckets.entry(u64::from(key.digest())).or_default();
        for entry in &*bucket {
            if entry.0 == key {
                return Rc::clone(&entry.1);
            }
        }
        bucket.push((key, Rc::clone(&term)));
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
///   to its argument and identical (by pointer) for every alpha-identical term
///   previously interned **into the same face**.
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
    ) -> crate::boundary::InternerEmptyStatus
    {
        crate::boundary::InternerEmptyStatus::from(usize::from(self.len(face)) == 0)
    }

    /// Interns `term` into `face`'s table and returns that face's canonical
    /// representative.
    ///
    /// # Contract
    /// - ensures: the result is alpha-identical to `term`; two alpha-identical
    ///   terms interned into the **same** face return the same pointer; a term
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
    /// - witness: `nbe::tests::interning_shares_alpha_equivalent_terms_within_a_face`
    /// - witness: `nbe::tests::interning_keeps_the_two_faces_disjoint`
    #[inline]
    pub fn intern(
        &mut self,
        face: Face,
        term: Rc<Value>,
    ) -> Rc<Value>
    {
        match face {
            | Face::ElaborationInput => self.input.intern(term),
            | Face::ReadbackNormalForm => self.readback.intern(term),
        }
    }
}

/// One pending step in the canonical-key traversal.
enum KeyTask<'term>
{
    /// Emit the tokens of a value.
    Value(&'term Value),
    /// Emit the tokens of a computation.
    Comp(&'term Comp),
    /// Emit the tokens of a stack.
    Stack(&'term Stack),
    /// Emit the tokens of a value type.
    ValueType(&'term ValueType),
    /// Emit the tokens of a computation type.
    CompType(&'term CompType),
    /// Push one binder onto the canonical scope.
    PushOne(&'term str),
    /// Push two binders onto the canonical scope, the second innermost.
    PushTwo(&'term str, &'term str),
    /// Pop this many binders off the canonical scope.
    Pop(usize),
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

/// The canonical key of `term`: its full structural content in canonical binder
/// form.
///
/// # Contract
/// - ensures: two alpha-equivalent terms produce equal keys, and two terms that
///   differ in anything but binder names produce different keys; the key
///   mentions no arena id, index, address, generation, or session datum.
/// - panics: none.
///
/// # Termination
/// - reason: the traversal drains an explicit task stack over one finite term.
/// - measure: pending tasks on the stack.
/// - boundedness: terms are finite values, and each node pushes tasks only for
///   its own children.
/// - input recursion: none.
#[must_use]
#[inline]
pub fn canonical_key(term: &Value) -> CanonicalKey
{
    let mut tokens = Vec::new();
    let mut scope: Vec<&str> = Vec::new();
    let mut work = alloc::vec![KeyTask::Value(term)];
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
            | KeyTask::Value(value) => visit_value(
                value,
                BinderScope::from(scope.as_slice()),
                &mut tokens,
                &mut work,
            ),
            | KeyTask::Comp(comp) => visit_comp(comp, &mut tokens, &mut work),
            | KeyTask::Stack(stack) => visit_stack(stack, &mut tokens, &mut work),
            | KeyTask::ValueType(ty) => visit_value_type(ty, &mut tokens, &mut work),
            | KeyTask::CompType(ty) => visit_comp_type(ty, &mut tokens, &mut work),
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

/// Emits the canonical tokens of one value node and queues its children.
fn visit_value<'term>(
    value: &'term Value,
    scope: BinderScope<'_>,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *value {
        | Value::Var(ref name) => match scope
            .as_ref()
            .iter()
            .rev()
            .position(|bound| *bound == name.as_str())
        {
            | Some(index) => {
                tokens.push(CanonicalToken::from(tag::BOUND));
                payload(
                    tokens,
                    CanonicalToken::from(u64::try_from(index).unwrap_or(u64::MAX)),
                );
            },
            | None => {
                tokens.push(CanonicalToken::from(tag::FREE));
                work.push(KeyTask::Name(name.as_str()));
            },
        },
        | Value::Unit => tokens.push(CanonicalToken::from(tag::VALUE)),
        | Value::Int(literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(1)));
            hashed(tokens, &literal);
        },
        | Value::Str(ref literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(2)));
            work.push(KeyTask::Name(literal.as_str()));
        },
        | Value::Num(literal) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(3)));
            hashed(tokens, &literal);
        },
        | Value::Pair(ref fst, ref snd) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(4)));
            work.push(KeyTask::Value(snd));
            work.push(KeyTask::Value(fst));
        },
        | Value::Inj(side, ref payload_value) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(5)));
            hashed(tokens, &side);
            work.push(KeyTask::Value(payload_value));
        },
        | Value::List(ref elements) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(6)));
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(elements.len()).unwrap_or(u64::MAX)),
            );
            for element in elements.iter().rev() {
                work.push(KeyTask::Value(element));
            }
        },
        | Value::Record(ref fields) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(7)));
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(fields.len()).unwrap_or(u64::MAX)),
            );
            for (label, field) in fields.iter().rev() {
                work.push(KeyTask::Value(field));
                work.push(KeyTask::Name(label.as_str()));
            }
        },
        | Value::Thunk(grade, ref body) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(8)));
            hashed(tokens, &grade);
            work.push(KeyTask::Comp(body));
        },
        | Value::Annot(ref inner, ref ty) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(9)));
            work.push(KeyTask::ValueType(ty));
            work.push(KeyTask::Value(inner));
        },
        | Value::Hole(id) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(10)));
            hashed(tokens, &id);
        },
        | Value::Stk(ref stack) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(11)));
            work.push(KeyTask::Stack(stack));
        },
        | Value::Here(ref witness) => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(12)));
            work.push(KeyTask::Value(witness));
        },
        | Value::Ctor {
            ref id,
            tag: constructor,
            payload: ref carried,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE.saturating_add(13)));
            hashed(tokens, id);
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(constructor).unwrap_or(u64::MAX)),
            );
            work.push(KeyTask::Value(carried));
        },
    }
}

/// Emits the canonical tokens of one computation node and queues its children,
/// bracketing every binder it opens.
fn visit_comp<'term>(
    comp: &'term Comp,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    /// Queues a body under one binder, bracketed by the scope push and pop.
    fn under<'term>(
        work: &mut Vec<KeyTask<'term>>,
        binder: BinderName<'term>,
        body: &'term Comp,
    )
    {
        work.push(KeyTask::Pop(1));
        work.push(KeyTask::Comp(body));
        work.push(KeyTask::PushOne(binder.into()));
    }

    match *comp {
        | Comp::Abs(ref binder, ref ann, ref body) => {
            tokens.push(CanonicalToken::from(tag::COMP));
            // The optional ascription is inert for the canonical key exactly as
            // it is inert for evaluation: an annotation names a type, and two
            // terms differing only in it denote one value.
            let _ = ann;
            under(work, BinderName::from(binder.as_str()), body);
        },
        | Comp::App(ref head, ref arg) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(1)));
            work.push(KeyTask::Value(arg));
            work.push(KeyTask::Comp(head));
        },
        | Comp::Ret(ref value) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(2)));
            work.push(KeyTask::Value(value));
        },
        | Comp::Bind(ref bound, ref binder, ref cont) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(3)));
            under(work, BinderName::from(binder.as_str()), cont);
            work.push(KeyTask::Comp(bound));
        },
        | Comp::Force(ref value) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(4)));
            work.push(KeyTask::Value(value));
        },
        | Comp::Case(ref scrut, ref left, ref right) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(5)));
            under(work, BinderName::from(right.0.as_str()), &right.1);
            under(work, BinderName::from(left.0.as_str()), &left.1);
            work.push(KeyTask::Value(scrut));
        },
        | Comp::DataCase(ref scrut, ref arms) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(6)));
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(arms.len()).unwrap_or(u64::MAX)),
            );
            for arm in arms.iter().rev() {
                under(work, BinderName::from(arm.0.as_str()), &arm.1);
            }
            work.push(KeyTask::Value(scrut));
        },
        | Comp::ListCase {
            ref scrut,
            ref nil,
            ref head,
            ref tail,
            ref cons,
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(7)));
            work.push(KeyTask::Pop(2));
            work.push(KeyTask::Comp(cons));
            work.push(KeyTask::PushTwo(head.as_str(), tail.as_str()));
            work.push(KeyTask::Comp(nil));
            work.push(KeyTask::Value(scrut));
        },
        | Comp::Split {
            ref scrut,
            ref fst_name,
            ref snd_name,
            ref body,
            ..
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(8)));
            work.push(KeyTask::Pop(2));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::PushTwo(fst_name.as_str(), snd_name.as_str()));
            work.push(KeyTask::Value(scrut));
        },
        | Comp::RecordProj {
            ref record,
            ref label,
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(9)));
            work.push(KeyTask::Value(record));
            work.push(KeyTask::Name(label.as_str()));
        },
        | Comp::With(ref fst, ref snd) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(10)));
            work.push(KeyTask::Comp(snd));
            work.push(KeyTask::Comp(fst));
        },
        | Comp::Prj(side, ref body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(11)));
            hashed(tokens, &side);
            work.push(KeyTask::Comp(body));
        },
        | Comp::Dup(ref value) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(12)));
            work.push(KeyTask::Value(value));
        },
        | Comp::Drop(ref value) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(13)));
            work.push(KeyTask::Value(value));
        },
        | Comp::Perform(ref sig, ref op, ref carried) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(14)));
            hashed(tokens, sig.as_ref());
            work.push(KeyTask::Value(carried));
            work.push(KeyTask::Name(op.as_str()));
        },
        | Comp::Handle {
            ref sig,
            ref scrutinee,
            ref ret,
            ref ops,
        } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(15)));
            hashed(tokens, sig.as_ref());
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(ops.len()).unwrap_or(u64::MAX)),
            );
            for clause in ops.iter().rev() {
                work.push(KeyTask::Pop(2));
                work.push(KeyTask::Comp(&clause.body));
                work.push(KeyTask::PushTwo(
                    clause.payload.as_str(),
                    clause.resume.as_str(),
                ));
                work.push(KeyTask::Name(clause.op.as_str()));
            }
            under(work, BinderName::from(ret.0.as_str()), &ret.1);
            work.push(KeyTask::Comp(scrutinee));
        },
        | Comp::Resume(ref value, ref body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(16)));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::Value(value));
        },
        | Comp::Reset(ref body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(17)));
            work.push(KeyTask::Comp(body));
        },
        | Comp::Shift(ref binder, ref body) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(18)));
            under(work, BinderName::from(binder.as_str()), body);
        },
        | Comp::Hole(id) => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(19)));
            hashed(tokens, &id);
        },
        | Comp::Native { prim, ref args } => {
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(20)));
            hashed(tokens, &prim);
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(args.len()).unwrap_or(u64::MAX)),
            );
            for arg in args.iter().rev() {
                work.push(KeyTask::Value(arg));
            }
        },
        | Comp::Walk {
            ref scrut,
            ref base,
            ..
        } => {
            // The motive is a type ascription on the eliminator and contributes
            // nothing to the value it computes, so it is erased here for the
            // same reason an annotation is.
            tokens.push(CanonicalToken::from(tag::COMP.saturating_add(21)));
            under(work, BinderName::from(base.x.as_str()), &base.body);
            work.push(KeyTask::Value(scrut));
        },
    }
}

/// Emits the canonical tokens of one stack node and queues its children.
fn visit_stack<'term>(
    stack: &'term Stack,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *stack {
        | Stack::Empty => tokens.push(CanonicalToken::from(tag::STACK)),
        | Stack::Arg(ref arg, ref rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(1)));
            work.push(KeyTask::Stack(rest));
            work.push(KeyTask::Value(arg));
        },
        | Stack::Bind(ref binder, ref body, ref rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(2)));
            work.push(KeyTask::Stack(rest));
            work.push(KeyTask::Pop(1));
            work.push(KeyTask::Comp(body));
            work.push(KeyTask::PushOne(binder.as_str()));
        },
        | Stack::Prj(side, ref rest) => {
            tokens.push(CanonicalToken::from(tag::STACK.saturating_add(3)));
            hashed(tokens, &side);
            work.push(KeyTask::Stack(rest));
        },
    }
}

/// Emits the canonical tokens of one value type and queues its children.
fn visit_value_type<'term>(
    ty: &'term ValueType,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *ty {
        | ValueType::Atom(ref name) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE));
            work.push(KeyTask::Name(name.as_str()));
        },
        | ValueType::Unit => tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(1))),
        | ValueType::Prod(ref fst, ref snd) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(2)));
            work.push(KeyTask::ValueType(snd));
            work.push(KeyTask::ValueType(fst));
        },
        | ValueType::Sum(ref lhs, ref rhs) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(3)));
            work.push(KeyTask::ValueType(rhs));
            work.push(KeyTask::ValueType(lhs));
        },
        | ValueType::List(ref element) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(4)));
            work.push(KeyTask::ValueType(element));
        },
        | ValueType::Record(ref fields) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(5)));
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(fields.len()).unwrap_or(u64::MAX)),
            );
            for (label, field) in fields.iter().rev() {
                work.push(KeyTask::ValueType(field));
                work.push(KeyTask::Name(label.as_str()));
            }
        },
        | ValueType::Thunk(grade, ref body) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(6)));
            hashed(tokens, &grade);
            work.push(KeyTask::CompType(body));
        },
        | ValueType::Stk(ref consumes, ref delivers) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(7)));
            work.push(KeyTask::CompType(delivers));
            work.push(KeyTask::CompType(consumes));
        },
        | ValueType::Path {
            ty: ref carrier,
            ref lhs,
            ref rhs,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(8)));
            work.push(KeyTask::Value(rhs));
            work.push(KeyTask::Value(lhs));
            work.push(KeyTask::ValueType(carrier));
        },
        | ValueType::Data { ref id, ref args } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(9)));
            hashed(tokens, id);
            payload(
                tokens,
                CanonicalToken::from(u64::try_from(args.len()).unwrap_or(u64::MAX)),
            );
            for arg in args.iter().rev() {
                work.push(KeyTask::ValueType(arg));
            }
        },
        | ValueType::Universe => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(10)));
        },
        | ValueType::Sigma {
            ref fst,
            ref binder,
            ref snd,
        } => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(11)));
            work.push(KeyTask::Pop(1));
            work.push(KeyTask::ValueType(snd));
            work.push(KeyTask::PushOne(binder.as_str()));
            work.push(KeyTask::ValueType(fst));
        },
        | ValueType::Sealed(ref id) => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(12)));
            hashed(tokens, id);
        },
        | ValueType::Unknown => {
            tokens.push(CanonicalToken::from(tag::VALUE_TYPE.saturating_add(13)));
        },
    }
}

/// Emits the canonical tokens of one computation type and queues its children.
fn visit_comp_type<'term>(
    ty: &'term CompType,
    tokens: &mut Vec<CanonicalToken>,
    work: &mut Vec<KeyTask<'term>>,
)
{
    match *ty {
        | CompType::F(ref of, ref row) => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE));
            hashed(tokens, row);
            work.push(KeyTask::ValueType(of));
        },
        | CompType::Arrow(ref arg, ref res) => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(1)));
            work.push(KeyTask::CompType(res));
            work.push(KeyTask::ValueType(arg));
        },
        | CompType::With(ref fst, ref snd) => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(2)));
            work.push(KeyTask::CompType(snd));
            work.push(KeyTask::CompType(fst));
        },
        | CompType::Unknown => {
            tokens.push(CanonicalToken::from(tag::COMP_TYPE.saturating_add(3)));
        },
    }
}
