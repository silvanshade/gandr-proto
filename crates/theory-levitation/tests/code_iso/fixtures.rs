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

use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::DescValue;
use gandr_theory_levitation::NameRef;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::Payload;
use gandr_theory_levitation::Side;
use gandr_theory_levitation::builtin::bool_desc;

use crate::harness::CodeIso;
use crate::harness::Translate;

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
pub fn rgb() -> DataDesc
{
    DataDesc::new(
        NominalId::new(2.into(), "RGB"),
        Vec::new(),
        [
            nullary("R".into()),
            nullary("G".into()),
            nullary("B".into()),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// A nullary constructor of the given name (payload code `1`).
fn nullary(name: NameRef<'_>) -> CtorDesc
{
    CtorDesc::new(name, Code::Unit, None, Attrs::empty())
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
pub fn bool_two_ctor() -> DataDesc
{
    bool_desc()
}

/// The **inline `1 + 1` Boolean** (`BoolSum { MkBool = (1 + 1) }`) — one
/// constructor whose payload is the inline sum. The other side of the flagship
/// inter-code instance: structurally distinct codes describing the same
/// two-element value space (design note §4.2).
pub fn bool_sum() -> DataDesc
{
    DataDesc::new(
        NominalId::new(1.into(), "BoolSum"),
        Vec::new(),
        [CtorDesc::new(
            "MkBool",
            Code::sum(Code::Unit, Code::Unit),
            None,
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
