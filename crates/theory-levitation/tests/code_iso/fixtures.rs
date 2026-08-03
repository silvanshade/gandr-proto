//! The monomorphic descriptions, named certificates, and finite value samples
//! the U3.0 suite exercises.
//!
//! All descriptions are **parameter-free** (monomorphic — the U3.0 scope
//! constraint) and **finite** (no recursive
//! [`gandr_theory_levitation::Code::Var`]), so the whole value space of each is
//! a short enumerable list — the sample sets below. The flagship inter-code
//! instance is the declared two-constructor [`bool_two_ctor`] (`Boolean`, from
//! the landed retrofit) against the inline `1 + 1` sum form [`bool_sum`]
//! (design note §4.2, §4.1's cardinality witness).

use alloc::rc::Rc;

use gandr_core_checker::grade::Grade;
use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::DescValue;
use gandr_theory_levitation::NameRef;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::Payload;
use gandr_theory_levitation::PrimTy;
use gandr_theory_levitation::Side;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::ValueTypeRef;
use gandr_theory_levitation::builtin::bool_desc;

use super::harness::CodeIso;
use super::harness::Translate;

/// The **rotation** auto-iso on [`rgb`]: `R → G → B → R`. Non-involutive (its
/// inverse is the reverse rotation), so `rotate ⨟ rotate ⨟ rotate` is the
/// identity up to replay — the groupoid content for the associativity and
/// order-three tests.
pub fn rgb_rotate() -> CodeIso
{
    let forward: Translate = Rc::new(|value: &DescValue| {
        let next = match usize::from(value.ctor) {
            | 0 => 1_usize,
            | 1 => 2_usize,
            | _ => 0_usize,
        };
        DescValue::new(next.into(), Payload::Unit)
    });
    let backward: Translate = Rc::new(|value: &DescValue| {
        let prev = match usize::from(value.ctor) {
            | 0 => 2_usize,
            | 1 => 0_usize,
            | _ => 1_usize,
        };
        DescValue::new(prev.into(), Payload::Unit)
    });
    CodeIso::new("rgb-rotate", rgb(), rgb(), forward, backward)
}
/// A three-constructor monomorphic enum (`RGB { R = 1, G = 1, B = 1 }`) — the
/// carrier for a non-involutive auto-iso ([`rgb_rotate`]), so composition and
/// inverse have real content beyond the self-inverse Boolean cases.
pub fn rgb() -> SignDesc
{
    SignDesc::new(
        NominalId::new(2.into(), "RGB"),
        Vec::new(),
        [
            nullary("R".into(), "RGB".into()),
            nullary("G".into(), "RGB".into()),
            nullary("B".into(), "RGB".into()),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// A nullary constructor of the given name (payload code `1`), targeting the
/// result sort `of`.
fn nullary(
    name: NameRef<'_>,
    of: NameRef<'_>,
) -> CtorDesc
{
    CtorDesc::new(name, Code::Unit, of, Attrs::empty())
}

/// The two values of [`bool_sum`]: `MkBool(Inl ())` and `MkBool(Inr ())`.
pub fn bool_sum_values() -> Vec<DescValue>
{
    vec![
        DescValue::new(0.into(), inl(Payload::Unit)),
        DescValue::new(0.into(), inr(Payload::Unit)),
    ]
}
/// A left injection into an inline sum, wrapping `payload`.
fn inl(payload: Payload) -> Payload
{
    Payload::Inj(Side::Left, Box::new(payload))
}

/// A right injection into an inline sum, wrapping `payload`.
fn inr(payload: Payload) -> Payload
{
    Payload::Inj(Side::Right, Box::new(payload))
}

// ======================================================================
// Monomorphic, finite descriptions
// ======================================================================

/// The **identity** auto-iso on [`bool_two_ctor`] — the groupoid unit at
/// `(Boolean, Boolean)`.
pub fn identity_bool() -> CodeIso
{
    CodeIso::identity("id[Boolean]", bool_two_ctor())
}
/// The **negation** auto-iso on [`bool_two_ctor`] — swaps `False` and `True`;
/// its own inverse. The nontrivial member of `CodeIso(Boolean, Boolean)` that
/// the U3.0c guard pins as replay-distinct from [`identity_bool`].
pub fn negation_bool() -> CodeIso
{
    let flip: Translate = Rc::new(|value: &DescValue| match usize::from(value.ctor) {
        | 0 => DescValue::new(1.into(), Payload::Unit),
        | _ => DescValue::new(0.into(), Payload::Unit),
    });
    CodeIso::new(
        "negation",
        bool_two_ctor(),
        bool_two_ctor(),
        Rc::clone(&flip),
        flip,
    )
}
/// The flagship **cross-code bridge** `Boolean → BoolSum`: `False ↦ MkBool(Inl
/// ())`, `True ↦ MkBool(Inr ())`. A genuine iso between two *structurally
/// distinct* codes — the value-level identification that ua-base's `⤳` must
/// carry and that decidable code equality cannot (design note §4.1).
pub fn bool_bridge() -> CodeIso
{
    let forward: Translate = Rc::new(|value: &DescValue| match usize::from(value.ctor) {
        | 0 => DescValue::new(0.into(), inl(Payload::Unit)),
        | _ => DescValue::new(0.into(), inr(Payload::Unit)),
    });
    let backward: Translate = Rc::new(|value: &DescValue| {
        if matches!(value.payload, Payload::Inj(Side::Right, _)) {
            DescValue::new(1.into(), Payload::Unit)
        }
        else {
            DescValue::new(0.into(), Payload::Unit)
        }
    });
    CodeIso::new(
        "Boolean ⨟ BoolSum",
        bool_two_ctor(),
        bool_sum(),
        forward,
        backward,
    )
}
/// The declared **two-constructor `Boolean`** (`False = 1`, `True = 1`) — the
/// landed retrofit, reused verbatim as one side of the flagship instance.
pub fn bool_two_ctor() -> SignDesc
{
    bool_desc()
}

/// The **inline `1 + 1` Boolean** (`BoolSum { MkBool = (1 + 1) }`) — one
/// constructor whose payload is the inline sum. The other side of the flagship
/// inter-code instance: structurally distinct codes describing the same
/// two-element value space (design note §4.2).
pub fn bool_sum() -> SignDesc
{
    SignDesc::new(
        NominalId::new(1.into(), "BoolSum"),
        Vec::new(),
        [CtorDesc::new(
            "MkBool",
            Code::sum(Code::Unit, Code::Unit),
            "BoolSum",
            Attrs::empty(),
        )],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}

// ======================================================================
// Finite value samples (the whole value space of each description)
// ======================================================================

/// The two values of [`bool_two_ctor`]: `False` (index 0) and `True` (index 1).
pub fn bool_two_ctor_values() -> Vec<DescValue>
{
    vec![
        DescValue::new(0.into(), Payload::Unit),
        DescValue::new(1.into(), Payload::Unit),
    ]
}

/// The three values of [`rgb`]: `R`, `G`, `B`.
pub fn rgb_values() -> Vec<DescValue>
{
    vec![
        DescValue::new(0.into(), Payload::Unit),
        DescValue::new(1.into(), Payload::Unit),
        DescValue::new(2.into(), Payload::Unit),
    ]
}

// ======================================================================
// Named certificates
// ======================================================================

// ======================================================================
// The infinite-leaf boundary (U3.0d — the leaf-shift guard)
// ======================================================================

/// The single-constructor **`IntBox`** description (`Box = Integer`) — one
/// field over the unbounded primitive leaf [`PrimTy::Integer`]. A monomorphic
/// endo-boundary whose only degree of freedom is *leaf content*: with exactly
/// one constructor, the structural (constructor-permuting) auto-iso group is
/// trivial, so any nontrivial auto-iso must move the leaf. The carrier for the
/// [`leaf_shift`] witness.
pub fn int_box() -> SignDesc
{
    SignDesc::new(
        NominalId::new(3.into(), "IntBox"),
        Vec::new(),
        [CtorDesc::new(
            "Box",
            Code::field(
                ValueTypeRef::Prim(PrimTy::Integer),
                Grade::ONE,
                Attrs::empty(),
            ),
            "IntBox",
            Attrs::empty(),
        )],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}

/// Finite integer sample carried by the `IntBox` fixture.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct IntBoxLeaf(i64);

impl IntBoxLeaf
{
    /// Zero, the first replay sample.
    pub const ZERO: Self = Self(0);

    /// One, the successor image of zero.
    pub const ONE: Self = Self(1);

    /// Two, the second positive replay sample.
    const TWO: Self = Self(2);

    /// Negative one, the first negative replay sample.
    const NEGATIVE_ONE: Self = Self(-1);

    /// Negative two, the second negative replay sample.
    const NEGATIVE_TWO: Self = Self(-2);

    /// Return the representable fixture successor.
    fn successor(self) -> Self
    {
        Self(
            self.0
                .checked_add(1)
                .expect("the sampled IntBox successor is representable"),
        )
    }

    /// Return the representable fixture predecessor.
    fn predecessor(self) -> Self
    {
        Self(
            self.0
                .checked_sub(1)
                .expect("the sampled IntBox predecessor is representable"),
        )
    }
}

/// An [`int_box`] value carrying the integer `n` — constructor `Box` (index 0)
/// over the canonical decimal-ASCII leaf encoding of `n`. The decimal spelling
/// is unbounded-length, faithful to `Integer`'s unbounded leaf (the finite
/// `i64` here bounds only the *sampled* range, never the leaf type).
pub fn int_leaf(n: IntBoxLeaf) -> DescValue
{
    DescValue::new(0.into(), Payload::Leaf(n.0.to_string().into_bytes().into()))
}

/// A finite sample of [`int_box`] leaf values, `0` first (so the earliest
/// replay disagreement with the identity is inspectable at `0`), staying well
/// inside the `i64` sample bound.
pub fn int_box_values() -> Vec<DescValue>
{
    [
        IntBoxLeaf::ZERO,
        IntBoxLeaf::ONE,
        IntBoxLeaf::TWO,
        IntBoxLeaf::NEGATIVE_ONE,
        IntBoxLeaf::NEGATIVE_TWO,
    ]
    .into_iter()
    .map(int_leaf)
    .collect()
}

/// The **identity** auto-iso on [`int_box`] — the sole leaf-natural member of
/// the endo-boundary (the structural auto-iso group is trivial), against which
/// [`leaf_shift`] is pinned replay-distinct.
pub fn int_box_identity() -> CodeIso
{
    CodeIso::identity("id[IntBox]", int_box())
}

/// The **leaf-shift** auto-iso on [`int_box`]: forward is the integer successor
/// (`n ↦ n + 1`), backward the predecessor (`n ↦ n − 1`). A genuine `CodeIso`
/// member — its round trips hold on every integer — that is nonetheless
/// **replay-distinct from the identity** (it disagrees on every leaf). Unlike a
/// constructor permutation, it reads and rewrites the leaf *content*, so no
/// leaf-natural (structural) certificate replays it — the U3.0d witness.
pub fn leaf_shift() -> CodeIso
{
    let forward: Translate =
        Rc::new(|value: &DescValue| int_leaf(read_int_leaf(value).successor()));
    let backward: Translate =
        Rc::new(|value: &DescValue| int_leaf(read_int_leaf(value).predecessor()));
    CodeIso::new("leaf-shift", int_box(), int_box(), forward, backward)
}

/// Read the integer an [`int_box`] value carries, parsing its decimal-ASCII
/// leaf (the inverse of [`int_leaf`]).
fn read_int_leaf(value: &DescValue) -> IntBoxLeaf
{
    let Payload::Leaf(ref bytes) = value.payload
    else {
        panic!("an IntBox value carries a Leaf payload");
    };
    let value = core::str::from_utf8(bytes)
        .expect("the IntBox leaf is decimal-ASCII")
        .parse::<i64>()
        .expect("the IntBox leaf parses as an integer");
    IntBoxLeaf(value)
}
