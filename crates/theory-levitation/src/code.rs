//! The first-order **code universe** `{1, var, ×, σ}` plus the V5 leaf
//! decorations (the levitation design's §3–§4.3).
//!
//! A [`Code`] describes one constructor's payload. The tag σ over constructors
//! lives at the [`crate::SignDesc`] level (the tagged description, V2); the
//! [`Code::Sum`] former additionally represents an inline sum so retrofitted
//! builtins like `Boolean = 1 + 1` are one code (see [`crate::builtin`]). The
//! fragment is deliberately first-order — no function-typed fields — **because
//! that is what keeps code equality decidable** (proposal §3): [`Code`] derives
//! total structural [`Eq`]/[`Hash`], the property content-addressing
//! ([`crate::CodeInterner`]) and matching-modulo rely on.

use core::fmt;

use gandr_core_checker::discipline::boundary::NameRef;
use gandr_core_checker::discipline::grade::Grade;

use crate::boundary::AttributeEmptiness;
use crate::boundary::AttributePresence;
use crate::boundary::FirstOrderStatus;
use crate::boundary::RecursiveStatus;

/// A description-level name (constructor, field, parameter, type head, …).
///
/// A boxed string: cheap to clone as a shared owner is unnecessary at the decl
/// table's scale, and it keeps [`Code`] a plain value with derived structural
/// equality.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(Box<str>);

impl From<&str> for Name
{
    #[inline]
    fn from(value: &str) -> Self
    {
        Self(value.into())
    }
}
impl From<NameRef<'_>> for Name
{
    #[inline]
    fn from(value: NameRef<'_>) -> Self
    {
        Self(value.as_ref().into())
    }
}

impl From<String> for Name
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value.into_boxed_str())
    }
}

impl From<Box<str>> for Name
{
    #[inline]
    fn from(value: Box<str>) -> Self
    {
        Self(value)
    }
}

impl From<Name> for Box<str>
{
    #[inline]
    fn from(value: Name) -> Self
    {
        value.0
    }
}

impl AsRef<str> for Name
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl fmt::Display for Name
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}

/// A retrofitted **primitive value type** a non-recursive field can carry
/// (proposal §3: "the primitive formers get retrofitted descriptions").
///
/// The closed set of core primitive types the description leaf recognizes; any
/// other named type is a [`ValueTypeRef::Ctor`]. `decode` erases nothing here —
/// a primitive field is an opaque leaf the value decoder reads by its core
/// type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimTy
{
    /// The unbounded integer type `Integer`.
    Integer,
    /// The boolean type `Boolean` (retrofit target `1 + 1`).
    Boolean,
    /// The unit type `Unit`.
    Unit,
    /// The string type `String`.
    StringTy,
    /// The character type `Char`.
    Char,
    /// The fixed-width numeric type `u32`.
    U32,
    /// The fixed-width numeric type `u64`.
    U64,
    /// The fixed-width numeric type `i32`.
    I32,
    /// The fixed-width numeric type `i64`.
    I64,
    /// The floating type `f32`.
    F32,
    /// The floating type `f64`.
    F64,
    /// The gradual top `Unknown` (the consistency hole a field type can name).
    Unknown,
}

impl PrimTy
{
    /// Classify a surface primitive-type spelling, or [`None`] when the
    /// spelling is not a recognized core primitive (it is then a named
    /// type).
    ///
    /// # Contract
    /// - requires: `label` is a surface type token spelling.
    /// - ensures: returns the matching primitive, or [`None`] for a
    ///   non-primitive spelling.
    /// - fails: never; an unrecognized spelling is `None`, not an error.
    #[inline]
    #[must_use]
    pub fn from_label(label: NameRef<'_>) -> Option<Self>
    {
        let prim = match label.as_ref() {
            | "Integer" => Self::Integer,
            | "Boolean" => Self::Boolean,
            | "Unit" => Self::Unit,
            | "String" => Self::StringTy,
            | "Char" => Self::Char,
            | "u32" => Self::U32,
            | "u64" => Self::U64,
            | "i32" => Self::I32,
            | "i64" => Self::I64,
            | "f32" => Self::F32,
            | "f64" => Self::F64,
            | "Unknown" => Self::Unknown,
            | _ => return None,
        };
        Some(prim)
    }

    /// The canonical spelling of this primitive (the inverse of
    /// [`Self::from_label`] on recognized inputs).
    #[inline]
    #[must_use]
    pub fn label(self) -> NameRef<'static>
    {
        match self {
            | Self::Integer => "Integer",
            | Self::Boolean => "Boolean",
            | Self::Unit => "Unit",
            | Self::StringTy => "String",
            | Self::Char => "Char",
            | Self::U32 => "u32",
            | Self::U64 => "u64",
            | Self::I32 => "i32",
            | Self::I64 => "i64",
            | Self::F32 => "f32",
            | Self::F64 => "f64",
            | Self::Unknown => "Unknown",
        }
        .into()
    }
}

/// A **reference** to the core value type a non-recursive field carries (the
/// `ValueTypeRef` of the proposal sketch, §3).
///
/// Stage 0 keeps this *symbolic* — surface-level, before type lowering — so the
/// description faithfully mirrors the declared field without committing to the
/// core `ValueType` universe (that mapping is the value decoder's job, gated on
/// the `μ⁺` rung). A field whose type is a recursive occurrence of the datatype
/// being described is **not** a `ValueTypeRef` at all: it is [`Code::Var`]
/// (proposal §3, "a self-reference is an ordinary type application").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ValueTypeRef
{
    /// A datatype **type parameter**, by name (`a` in `Maybe(a)`).
    Param(Name),
    /// A retrofitted **primitive** value type.
    Prim(PrimTy),
    /// A **named or applied** type — another declared type or an unresolved
    /// reference — as a head plus (possibly empty) argument references
    /// (`NatOp`, `Vec(a)`).
    Ctor
    {
        /// The type head (`Vec`, `NatOp`).
        head: Name,
        /// The applied arguments, left to right (empty for a bare name).
        args: Box<[Self]>,
    },
}

/// One **attribute marker** in a per-symbol attribute slot (`[ctor, assoc]`;
/// proposal §4.3, the attribute-Σ attachment point).
///
/// Stage 0 carries the marker name; a typed payload is erased by decoding and
/// is not modelled here (a residual — the surface data-block slot admits bare
/// markers only today).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Attr
{
    /// The attribute marker name.
    pub name: Name,
}

impl Attr
{
    /// A bare marker attribute of the given name.
    #[inline]
    #[must_use]
    pub fn marker<N>(name: N) -> Self
    where
        N: Into<Name>,
    {
        Self { name: name.into() }
    }
}

/// The **attribute Σ** attached to a symbol (proposal §4.3): a set of markers,
/// erased by the value decoder and read by the attribute decoder.
///
/// Kept in declaration order (small, and order is provenance); membership tests
/// scan linearly.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Attrs
{
    /// The markers, in declaration order.
    pub markers: Box<[Attr]>,
}

impl Attrs
{
    /// The empty attribute Σ.
    #[inline]
    #[must_use]
    pub fn empty() -> Self
    {
        Self {
            markers: Box::default(),
        }
    }

    /// An attribute Σ from an ordered marker collection.
    #[inline]
    #[must_use]
    pub fn new<M>(markers: M) -> Self
    where
        M: Into<Box<[Attr]>>,
    {
        Self {
            markers: markers.into(),
        }
    }

    /// Whether the Σ carries no markers.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> AttributeEmptiness
    {
        self.markers.is_empty().into()
    }

    /// Whether a marker of the given name is present.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        name: NameRef<'_>,
    ) -> AttributePresence
    {
        self.markers
            .iter()
            .any(|attr| attr.name.as_ref() == name.as_ref())
            .into()
    }
}

/// The **sort** of an atom a [`Code::Bind`] abstracts over (proposal §4.3;
/// ADR-41).
///
/// Stage 0 keeps the sort symbolic (its name); interpreting `bind A T` into an
/// atom-abstraction functor over `gandr-nominal` is a later lane. No surface
/// data-block field produces a binder today, so this rides reserved.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct AtomSort
{
    /// The atom sort's name.
    pub name: Name,
}

impl AtomSort
{
    /// An atom sort of the given name.
    #[inline]
    #[must_use]
    pub fn named<N>(name: N) -> Self
    where
        N: Into<Name>,
    {
        Self { name: name.into() }
    }
}

/// A **first-order description code** — one constructor's payload shape
/// (proposal §3).
///
/// The fragment is `{1, var, ×, σ}` ([`Unit`](Self::Unit), [`Var`](Self::Var),
/// [`Prod`](Self::Prod), [`Sum`](Self::Sum)) plus the V5 leaf decorations
/// ([`Field`](Self::Field), [`Bind`](Self::Bind)). It is **finitary and
/// higher-order-free**, so structural equality is decidable — the load-bearing
/// property (proposal §3): [`Code`] derives total [`Eq`]/[`Hash`], keyed on by
/// [`crate::CodeInterner`] and compared by matching-modulo.
///
/// # Adequacy
/// - hypothesis: L3 — the six variants are distinguished by the equality and
///   interning witnesses (two structurally identical codes are equal and intern
///   to one id; codes differing in a single leaf, decoration, factor, or
///   variant are unequal and intern distinctly).
/// - witness: `code::tests::decidable_equality_distinguishes_every_variant`,
///   `intern::tests::interning_is_by_decidable_code_equality`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Code
{
    /// `1` — the unit code (a field-less / nullary payload).
    Unit,
    /// `var S` — a recursive occurrence of a declared **sort** of the
    /// signature being described, carrying that sort's name.
    ///
    /// The sort argument is the **sort index** of the description universe
    /// (the signature unification's universe move): a multi-sorted signature
    /// is one description over a sort set, and a recursive occurrence names
    /// which sort it targets. In the degenerate single-sort block the index is
    /// the block's own name. The index is symbolic at stage 0 like every other
    /// sort spelling; [`crate::check_desc`] checks membership against the
    /// declared sort set, and the stage-1 value decoder accepts only the
    /// decoded description's own sort (cross-sort recursion is outside the
    /// single-sort decode fragment).
    Var(Name),
    /// `A × B` — a product of two payloads (fields pair right-nested).
    Prod(Box<Self>, Box<Self>),
    /// `A + B` — an inline sum (σ). The primary tag σ of a declared datatype is
    /// its constructor enumeration ([`crate::SignDesc::ctors`]); this former
    /// represents an *inline* sum, used to retrofit builtin sums.
    Sum(Box<Self>, Box<Self>),
    /// A **non-recursive leaf field** of a core value type, carrying its grade
    /// (erased by the value decoder, read by the grade decoder) and attribute
    /// Σ (V5).
    Field(ValueTypeRef, Grade, Attrs),
    /// `⟨a⟩T` — an **atom-abstraction** field over an atom sort (V5; ADR-41).
    Bind(AtomSort, Box<Self>),
}

impl Code
{
    /// The unit code `1`.
    #[inline]
    #[must_use]
    pub const fn unit() -> Self
    {
        Self::Unit
    }

    /// The recursive-occurrence code `var S`, targeting the sort named
    /// `sort`.
    #[inline]
    #[must_use]
    pub fn var<S>(sort: S) -> Self
    where
        S: Into<Name>,
    {
        Self::Var(sort.into())
    }

    /// The product code `left × right`.
    #[inline]
    #[must_use]
    pub fn prod(
        left: Self,
        right: Self,
    ) -> Self
    {
        Self::Prod(Box::new(left), Box::new(right))
    }

    /// The inline sum code `left + right`.
    #[inline]
    #[must_use]
    pub fn sum(
        left: Self,
        right: Self,
    ) -> Self
    {
        Self::Sum(Box::new(left), Box::new(right))
    }

    /// A leaf field code over a value type, with a grade and attribute Σ.
    #[inline]
    #[must_use]
    pub const fn field(
        ty: ValueTypeRef,
        grade: Grade,
        attrs: Attrs,
    ) -> Self
    {
        Self::Field(ty, grade, attrs)
    }

    /// An atom-abstraction code `⟨sort⟩body`.
    #[inline]
    #[must_use]
    pub fn bind(
        sort: AtomSort,
        body: Self,
    ) -> Self
    {
        Self::Bind(sort, Box::new(body))
    }

    /// Fold a left-to-right field list into a right-nested product, with
    /// [`Code::Unit`] for the empty list (the nullary payload) and the single
    /// field itself for a one-field list.
    ///
    /// # Contract
    /// - requires: `fields` are the constructor's field codes, in source order.
    /// - ensures: `[]` ↦ `1`, `[f]` ↦ `f`, `[f, g, h]` ↦ `f × (g × h)`.
    /// - fails: never.
    #[inline]
    #[must_use]
    pub fn product_of(fields: Vec<Self>) -> Self
    {
        let mut iter = fields.into_iter().rev();
        let Some(mut acc) = iter.next()
        else {
            return Self::Unit;
        };
        for field in iter {
            acc = Self::prod(field, acc);
        }
        acc
    }

    /// Whether this code lies in the decidable first-order fragment.
    ///
    /// Always `true` for a [`Code`]: the type *cannot represent* a higher-order
    /// code, so the fragment invariant holds by construction. The method is the
    /// executable statement of the load-bearing property (proposal §3), not a
    /// runtime gate.
    #[inline]
    #[must_use]
    pub fn is_first_order(&self) -> FirstOrderStatus
    {
        true.into()
    }

    /// Whether this code contains a recursive occurrence ([`Code::Var`]) — i.e.
    /// the described constructor is recursive.
    #[inline]
    #[must_use]
    pub fn is_recursive(&self) -> RecursiveStatus
    {
        let mut stack = vec![self];
        while let Some(code) = stack.pop() {
            match *code {
                | Self::Var(_) => return true.into(),
                | Self::Unit | Self::Field(..) => {},
                | Self::Prod(ref left, ref right) | Self::Sum(ref left, ref right) => {
                    stack.push(right);
                    stack.push(left);
                },
                | Self::Bind(_, ref body) => stack.push(body),
            }
        }
        false.into()
    }
}

#[cfg(test)]
mod tests
{
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn product_of_folds_right_nested_with_unit_and_singleton_bases()
    {
        assert_eq!(Code::Unit, Code::product_of(vec![]), "empty ↦ 1");
        assert_eq!(
            Code::var("Nat"),
            Code::product_of(vec![Code::var("Nat")]),
            "singleton ↦ the field itself"
        );
        assert_eq!(
            Code::product_of(vec![Code::Unit, Code::var("Nat"), some_code()]),
            Code::prod(Code::Unit, Code::prod(Code::var("Nat"), some_code())),
            "n-ary ↦ right-nested product"
        );
    }
    #[test]
    fn decidable_equality_distinguishes_every_variant()
    {
        // Reflexivity across the whole grammar.
        for code in [
            Code::Unit,
            Code::var("Nat"),
            Code::prod(Code::var("Nat"), Code::Unit),
            Code::sum(Code::Unit, Code::Unit),
            some_code(),
            Code::bind(AtomSort::named("x"), Code::var("Nat")),
        ] {
            assert_eq!(code.clone(), code, "equality is reflexive");
        }
        // Per-variant distinctness — one edit flips equality.
        assert_ne!(Code::Unit, Code::var("Nat"), "1 ≠ var");
        assert_ne!(
            Code::var("Nat"),
            Code::var("Tree"),
            "the sort index is significant — recursive occurrences of \
             different sorts are different codes"
        );
        assert_ne!(
            Code::prod(Code::var("Nat"), Code::Unit),
            Code::sum(Code::var("Nat"), Code::Unit),
            "× ≠ σ at the same children"
        );
        assert_ne!(
            Code::prod(Code::var("Nat"), Code::Unit),
            Code::prod(Code::Unit, Code::var("Nat")),
            "factor order matters"
        );
        // A single decoration edit (grade) breaks equality.
        let graded = Code::field(
            ValueTypeRef::Param("a".into()),
            Grade::OMEGA,
            Attrs::empty(),
        );
        assert_ne!(some_code(), graded, "the grade decoration is significant");
        // A single leaf-type edit breaks equality.
        let primed = Code::field(
            ValueTypeRef::Prim(PrimTy::Integer),
            Grade::ONE,
            Attrs::empty(),
        );
        assert_ne!(some_code(), primed, "the field's value type is significant");
    }
    #[test]
    fn code_is_usable_as_a_hash_map_key()
    {
        // The exact claim of proposal §3: content-addressing interns on code
        // equality. A hash map keyed by `Code` witnesses `Eq + Hash`.
        let mut table: HashMap<Code, u32> = HashMap::new();
        table.insert(some_code(), 7u32);
        assert_eq!(
            Some(7u32),
            table.get(&some_code()).copied(),
            "a structurally identical code recovers its entry"
        );
        assert_eq!(
            None,
            table.get(&Code::var("Nat")).copied(),
            "a distinct code misses"
        );
    }
    #[test]
    fn recursion_and_fragment_predicates_hold()
    {
        assert!(
            bool::from(Code::var("Nat").is_recursive()),
            "var is recursive"
        );
        assert!(
            bool::from(Code::prod(some_code(), Code::var("Nat")).is_recursive()),
            "recursion under a product is detected"
        );
        assert!(
            !bool::from(some_code().is_recursive()),
            "a plain field is non-recursive"
        );
        assert!(
            bool::from(Code::bind(AtomSort::named("x"), Code::var("Nat")).is_first_order()),
            "every code is first-order by construction"
        );
    }
    /// A `Maybe`-shaped payload code for a `Some(x: a)` constructor.
    fn some_code() -> Code
    {
        Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty())
    }

    #[test]
    fn primitive_labels_round_trip()
    {
        for prim in [
            PrimTy::Integer,
            PrimTy::Boolean,
            PrimTy::Unit,
            PrimTy::StringTy,
            PrimTy::Char,
            PrimTy::U32,
            PrimTy::U64,
            PrimTy::I32,
            PrimTy::I64,
            PrimTy::F32,
            PrimTy::F64,
            PrimTy::Unknown,
        ] {
            assert_eq!(
                PrimTy::from_label(prim.label()),
                Some(prim),
                "primitive spelling round-trips"
            );
        }
        assert_eq!(
            None,
            PrimTy::from_label("NatOp".into()),
            "a user type is not a primitive"
        );
    }

    #[test]
    fn attribute_membership_scans_markers()
    {
        let attrs = Attrs::new([Attr::marker("ctor"), Attr::marker("assoc")]);
        assert!(
            bool::from(attrs.contains("ctor".into())),
            "declared marker is present"
        );
        assert!(
            !bool::from(attrs.contains("infix".into())),
            "undeclared marker is absent"
        );
        assert!(!bool::from(attrs.is_empty()), "a populated Σ is non-empty");
        assert!(
            bool::from(Attrs::empty().is_empty()),
            "the empty Σ is empty"
        );
    }
}
