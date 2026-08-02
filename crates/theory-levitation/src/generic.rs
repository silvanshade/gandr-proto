//! **Generic programs over descriptions** — the stage-0 payoff (proposal §3,
//! §11).
//!
//! These functions are the real consumers that keep the decl table from being
//! "a beautiful decl table nobody decodes": each is one Rust program driven by
//! a [`DataDesc`] / [`Code`], covering declared data and retrofitted builtins
//! ([`crate::builtin`]) uniformly.
//!
//! * [`generic_eq`] — structural equality of two [`DescValue`]s, guided by the
//!   description (the derive-style structural-eq payoff);
//! * [`serialize_value`] — a canonical, deterministic byte encoding of a
//!   [`DescValue`], guided by the description (the wire-serialization payoff);
//! * [`serialize_desc`] — a canonical textual rendering of the whole
//!   description (the inspectable-IR payoff).
//!
//! A [`DescValue`] is a *generic* value of a described datatype: a constructor
//! tag plus a [`Payload`] shaped by that constructor's [`Code`]. Recursive
//! occurrences ([`Code::Var`]) nest a whole [`DescValue`] (tagged again), which
//! is why the generic programs are driven by the [`DataDesc`] (the σ tag), not
//! a bare [`Code`].

use gandr_core_checker::boundary::ConstructorTag;

use crate::boundary::GenericEquality;
use crate::boundary::SerializedDescText;
use crate::boundary::SerializedValueBytes;
use crate::code::Code;
use crate::code::Name;
use crate::code::ValueTypeRef;
use crate::desc::DataDesc;
use crate::desc::DeclPolarity;

/// Which side of an inline sum ([`Code::Sum`]) a value injects into.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side
{
    /// The left summand `A` of `A + B`.
    Left,
    /// The right summand `B` of `A + B`.
    Right,
}

/// The **payload** of one constructor, shaped by its [`Code`].
///
/// Each variant mirrors a code former: [`Payload::Unit`] for `1`,
/// [`Payload::Rec`] for `var` (a nested [`DescValue`] of the same datatype),
/// [`Payload::Pair`] for `×`, [`Payload::Inj`] for an inline `σ`,
/// [`Payload::Leaf`] for a [`Code::Field`] (the primitive's opaque bytes), and
/// [`Payload::Abs`] for a [`Code::Bind`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Payload
{
    /// The unit payload (for [`Code::Unit`]).
    Unit,
    /// A recursive occurrence: a value of the same datatype (for
    /// [`Code::Var`]).
    Rec(Box<DescValue>),
    /// A product of two payloads (for [`Code::Prod`]).
    Pair(Box<Self>, Box<Self>),
    /// An injection into an inline sum (for [`Code::Sum`]).
    Inj(Side, Box<Self>),
    /// A leaf field's opaque value bytes (for [`Code::Field`]).
    Leaf(Box<[u8]>),
    /// An atom-abstraction: the bound atom's name and the body (for
    /// [`Code::Bind`]).
    Abs(Name, Box<Self>),
}

/// A **generic value** of a described datatype: a constructor tag (an index
/// into [`DataDesc::ctors`]) plus its [`Payload`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DescValue
{
    /// The constructor index into [`DataDesc::ctors`].
    pub ctor: ConstructorTag,
    /// The constructor's payload, shaped by that constructor's code.
    pub payload: Payload,
}

impl DescValue
{
    /// A value of constructor `ctor` with the given payload.
    #[inline]
    #[must_use]
    pub fn new(
        ctor: ConstructorTag,
        payload: Payload,
    ) -> Self
    {
        Self { ctor, payload }
    }
}

/// **Generic structural equality** of two values of a described datatype,
/// guided by `desc` (proposal §3).
///
/// # Contract
/// - requires: `left` and `right` are intended values of `desc`.
/// - ensures: returns `true` exactly when the two values agree constructor by
///   constructor and, recursively, payload by payload; a value whose shape does
///   not match its constructor's code (a mis-built value) compares unequal
///   rather than panicking — the function is total.
/// - fails: never; total on any input (an out-of-range constructor index or a
///   code/payload shape mismatch yields `false`).
///
/// # Adequacy
/// - hypothesis: L3 — equal values, values differing in constructor, and values
///   differing in a single nested leaf are distinguished.
/// - witness: `generic::tests::generic_eq_is_description_driven_structural`.
#[inline]
#[must_use]
pub fn generic_eq(
    desc: &DataDesc,
    left: &DescValue,
    right: &DescValue,
) -> GenericEquality
{
    if left.ctor != right.ctor {
        return false.into();
    }
    let Some(ctor) = desc.ctors.get(usize::from(left.ctor))
    else {
        // An out-of-range tag is a mis-built value: unequal, never a panic.
        return false.into();
    };
    payload_eq(desc, &ctor.code, &left.payload, &right.payload)
}

/// Structural equality of two payloads guided by a [`Code`] (the recursion of
/// [`generic_eq`]).
fn payload_eq(
    desc: &DataDesc,
    code: &Code,
    left: &Payload,
    right: &Payload,
) -> GenericEquality
{
    let mut values = Vec::new();
    let mut payloads = vec![(code, left, right)];

    loop {
        while let Some((node, lhs, rhs)) = payloads.pop() {
            match (node, lhs, rhs) {
                | (&Code::Unit, &Payload::Unit, &Payload::Unit) => {},
                | (&Code::Var, &Payload::Rec(ref inner_left), &Payload::Rec(ref inner_right)) => {
                    values.push((inner_left.as_ref(), inner_right.as_ref()));
                },
                | (
                    &Code::Prod(ref code_left, ref code_right),
                    &Payload::Pair(ref left_first, ref left_second),
                    &Payload::Pair(ref right_first, ref right_second),
                ) => {
                    payloads.push((code_right, left_second, right_second));
                    payloads.push((code_left, left_first, right_first));
                },
                | (
                    &Code::Sum(ref code_left, ref code_right),
                    &Payload::Inj(left_side, ref left_body),
                    &Payload::Inj(right_side, ref right_body),
                ) => {
                    if left_side != right_side {
                        return false.into();
                    }
                    match left_side {
                        | Side::Left => payloads.push((code_left, left_body, right_body)),
                        | Side::Right => payloads.push((code_right, left_body, right_body)),
                    }
                },
                | (
                    &Code::Field(..),
                    &Payload::Leaf(ref left_bytes),
                    &Payload::Leaf(ref right_bytes),
                ) if left_bytes == right_bytes => {},
                | (
                    &Code::Bind(_, ref body_code),
                    &Payload::Abs(ref left_atom, ref left_body),
                    &Payload::Abs(ref right_atom, ref right_body),
                ) if left_atom == right_atom => payloads.push((body_code, left_body, right_body)),
                // Any code/payload shape mismatch: a mis-built value compares unequal.
                | _ => return false.into(),
            }
        }

        let Some((left_value, right_value)) = values.pop()
        else {
            return true.into();
        };
        if left_value.ctor != right_value.ctor {
            return false.into();
        }
        let Some(ctor) = desc.ctors.get(usize::from(left_value.ctor))
        else {
            return false.into();
        };
        payloads.push((&ctor.code, &left_value.payload, &right_value.payload));
    }
}

/// **Generic serialization** of a value to a canonical, deterministic byte
/// encoding, guided by `desc` (proposal §7, the wire-serialization payoff).
///
/// The encoding is a straightforward tag-and-payload walk: the constructor
/// index as a little-endian `u32`, then each field's bytes in code order (a
/// product concatenates, an inline sum emits a side byte then the injected
/// payload, a recursive occurrence recurses). It is **deterministic** — the
/// same value always encodes to the same bytes — which is what content
/// addressing and equality-by-bytes need.
///
/// # Contract
/// - requires: `value` is an intended value of `desc`.
/// - ensures: appends `value`'s canonical encoding to a fresh buffer and
///   returns it; equal values ([`generic_eq`]) encode to equal bytes.
/// - fails: never; a mis-built value encodes what structure it has (an
///   out-of-range tag encodes just the tag) rather than panicking.
#[inline]
#[must_use]
pub fn serialize_value(
    desc: &DataDesc,
    value: &DescValue,
) -> SerializedValueBytes
{
    enum EncodeTask<'value>
    {
        Value(&'value DescValue),
        Payload(&'value Code, &'value Payload),
    }

    let mut out = Vec::new();
    let mut stack = vec![EncodeTask::Value(value)];
    while let Some(task) = stack.pop() {
        match task {
            | EncodeTask::Value(current) => {
                let tag = u32::try_from(usize::from(current.ctor)).unwrap_or(u32::MAX);
                out.extend_from_slice(&tag.to_le_bytes());
                if let Some(ctor) = desc.ctors.get(usize::from(current.ctor)) {
                    stack.push(EncodeTask::Payload(&ctor.code, &current.payload));
                }
            },
            | EncodeTask::Payload(code, payload) => match (code, payload) {
                | (&Code::Var, &Payload::Rec(ref inner)) => stack.push(EncodeTask::Value(inner)),
                | (
                    &Code::Prod(ref code_left, ref code_right),
                    &Payload::Pair(ref first, ref second),
                ) => {
                    stack.push(EncodeTask::Payload(code_right, second));
                    stack.push(EncodeTask::Payload(code_left, first));
                },
                | (&Code::Sum(ref code_left, ref code_right), &Payload::Inj(side, ref body)) => {
                    match side {
                        | Side::Left => {
                            out.push(0u8);
                            stack.push(EncodeTask::Payload(code_left, body));
                        },
                        | Side::Right => {
                            out.push(1u8);
                            stack.push(EncodeTask::Payload(code_right, body));
                        },
                    }
                },
                | (&Code::Field(..), &Payload::Leaf(ref bytes)) => {
                    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(bytes);
                },
                | (&Code::Bind(_, ref body_code), &Payload::Abs(ref atom, ref body)) => {
                    let atom_bytes = atom.as_ref().as_bytes();
                    let len = u32::try_from(atom_bytes.len()).unwrap_or(u32::MAX);
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(atom_bytes);
                    stack.push(EncodeTask::Payload(body_code, body));
                },
                // `Code::Unit`/`Payload::Unit` contributes no bytes, as does any
                // mis-built payload.
                | _ => {},
            },
        }
    }
    out.into()
}

/// **Inspectable rendering** of a whole description to canonical text
/// (proposal §8's `desc-inspect` payoff).
///
/// The rendering is deterministic and structural — a testing/inspection
/// notation, not a surface pretty-printer (that surface is owned elsewhere). It
/// names the polarity, the datatype and its parameters, each constructor with
/// its rendered [`Code`], and (when present) the reserved operations and 2-cell
/// faces.
///
/// # Contract
/// - ensures: deterministic output for a given description; total — never
///   panics.
#[inline]
#[must_use]
pub fn serialize_desc(desc: &DataDesc) -> SerializedDescText
{
    let keyword = match desc.polarity {
        | DeclPolarity::Data => "data",
        | DeclPolarity::Codata => "codata",
    };
    let params: Vec<String> = desc
        .params
        .iter()
        .map(|param| param.name.to_string())
        .collect();
    let params = if params.is_empty() {
        String::new()
    }
    else {
        format!("({})", params.join(", "))
    };
    // Built functionally (`format!` + `join`/`collect`) so the rendering is one
    // expression, with no discarded write result.
    let ctor_parts: Vec<String> = desc
        .ctors
        .iter()
        .map(|ctor| {
            let base = format!("{} = {}", ctor.name, render_code(&ctor.code));
            match ctor.result {
                | Some(ref result) => format!("{base} : {result}"),
                | None => base,
            }
        })
        .collect();
    let body = if ctor_parts.is_empty() {
        String::new()
    }
    else {
        format!(" {} ", ctor_parts.join(", "))
    };
    let ops = desc
        .ops
        .iter()
        .map(|op| {
            format!(
                "; op {}/{}->{}",
                op.name,
                op.arity.inputs.len(),
                op.arity.outputs.len()
            )
        })
        .collect::<Vec<String>>()
        .concat();
    let cells = desc
        .cells
        .iter()
        .map(|cell| format!("; rule {}", render_face(cell)))
        .collect::<Vec<String>>()
        .concat();
    format!("{keyword} {}{params} {{{body}}}{ops}{cells}", desc.id.name).into()
}

/// Render a [`Code`] to the inspection notation.
fn render_code(code: &Code) -> String
{
    enum CodeFrame<'code>
    {
        Render(&'code Code),
        FinishProd,
        FinishSum,
        FinishBind(&'code Name),
    }

    let mut stack = vec![CodeFrame::Render(code)];
    let mut rendered = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | CodeFrame::Render(node) => match *node {
                | Code::Unit => rendered.push("1".to_owned()),
                | Code::Var => rendered.push("var".to_owned()),
                | Code::Prod(ref left, ref right) => {
                    stack.push(CodeFrame::FinishProd);
                    stack.push(CodeFrame::Render(right));
                    stack.push(CodeFrame::Render(left));
                },
                | Code::Sum(ref left, ref right) => {
                    stack.push(CodeFrame::FinishSum);
                    stack.push(CodeFrame::Render(right));
                    stack.push(CodeFrame::Render(left));
                },
                | Code::Field(ref ty, ref grade, ref attrs) => {
                    let base = render_type_ref(ty);
                    let rendered_field = if *grade == gandr_core_checker::grade::Grade::ONE {
                        base
                    }
                    else {
                        format!("{grade:?} {base}")
                    };
                    if bool::from(attrs.is_empty()) {
                        rendered.push(rendered_field);
                    }
                    else {
                        let markers: Vec<String> = attrs
                            .markers
                            .iter()
                            .map(|attr| attr.name.to_string())
                            .collect();
                        rendered.push(format!("{rendered_field} [{}]", markers.join(", ")));
                    }
                },
                | Code::Bind(ref sort, ref body) => {
                    stack.push(CodeFrame::FinishBind(&sort.name));
                    stack.push(CodeFrame::Render(body));
                },
            },
            | CodeFrame::FinishProd => {
                let right = rendered.pop().unwrap_or_default();
                let left = rendered.pop().unwrap_or_default();
                rendered.push(format!("({left} × {right})"));
            },
            | CodeFrame::FinishSum => {
                let right = rendered.pop().unwrap_or_default();
                let left = rendered.pop().unwrap_or_default();
                rendered.push(format!("({left} + {right})"));
            },
            | CodeFrame::FinishBind(sort) => {
                let body = rendered.pop().unwrap_or_default();
                rendered.push(format!("⟨{sort}⟩{body}"));
            },
        }
    }
    rendered.pop().unwrap_or_default()
}

/// Render a [`ValueTypeRef`] to the inspection notation.
fn render_type_ref(ty: &ValueTypeRef) -> String
{
    enum TypeFrame<'ty>
    {
        Render(&'ty ValueTypeRef),
        FinishCtor(&'ty Name, usize),
    }

    let mut stack = vec![TypeFrame::Render(ty)];
    let mut rendered = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | TypeFrame::Render(node) => match *node {
                | ValueTypeRef::Param(ref name) => rendered.push(name.to_string()),
                | ValueTypeRef::Prim(prim) => {
                    rendered.push(prim.label().as_ref().to_owned());
                },
                | ValueTypeRef::Ctor { ref head, ref args } if args.is_empty() => {
                    rendered.push(head.to_string());
                },
                | ValueTypeRef::Ctor { ref head, ref args } => {
                    stack.push(TypeFrame::FinishCtor(head, args.len()));
                    for arg in args.iter().rev() {
                        stack.push(TypeFrame::Render(arg));
                    }
                },
            },
            | TypeFrame::FinishCtor(head, arity) => {
                let mut args = Vec::with_capacity(arity);
                for _ in 0 .. arity {
                    args.push(rendered.pop().unwrap_or_default());
                }
                args.reverse();
                rendered.push(format!("{head}({})", args.join(", ")));
            },
        }
    }
    rendered.pop().unwrap_or_default()
}

/// Render a [`crate::CellFace`] to the inspection notation (`lhs ~> rhs`).
fn render_face(cell: &crate::cell::CellFace) -> String
{
    format!(
        "{} ~> {}",
        render_free_term(&cell.lhs),
        render_free_term(&cell.rhs)
    )
}

/// Render a [`crate::FreeTerm`] to the inspection notation.
///
/// Shared with [`crate::wellformed`], whose boundary diagnostics quote the two
/// terms they compare in the same notation the description IR renders them in.
pub(crate) fn render_free_term(term: &crate::cell::FreeTerm) -> String
{
    enum TermFrame<'term>
    {
        Render(&'term crate::cell::FreeTerm),
        FinishApp(&'term Name, usize),
    }

    let mut stack = vec![TermFrame::Render(term)];
    let mut rendered = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | TermFrame::Render(node) => match *node {
                | crate::cell::FreeTerm::Var(ref name) => {
                    rendered.push(name.to_string());
                },
                | crate::cell::FreeTerm::Ctor(ref name, ref args)
                | crate::cell::FreeTerm::Op(ref name, ref args)
                    if args.is_empty() =>
                {
                    rendered.push(name.to_string());
                },
                | crate::cell::FreeTerm::Ctor(ref name, ref args)
                | crate::cell::FreeTerm::Op(ref name, ref args) => {
                    stack.push(TermFrame::FinishApp(name, args.len()));
                    for arg in args.iter().rev() {
                        stack.push(TermFrame::Render(arg));
                    }
                },
            },
            | TermFrame::FinishApp(name, arity) => {
                let mut args = Vec::with_capacity(arity);
                for _ in 0 .. arity {
                    args.push(rendered.pop().unwrap_or_default());
                }
                args.reverse();
                rendered.push(format!("{name}({})", args.join(", ")));
            },
        }
    }
    rendered.pop().unwrap_or_default()
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::grade::Grade;

    use super::*;
    use crate::code::Attrs;
    use crate::code::PrimTy;
    use crate::desc::CtorDesc;
    use crate::desc::NominalId;

    #[test]
    fn generic_eq_is_description_driven_structural()
    {
        let desc = maybe_desc();
        let some_a = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"a"[..])));
        let some_a2 = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"a"[..])));
        let some_b = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"b"[..])));
        let none = DescValue::new(0.into(), Payload::Unit);

        assert!(
            bool::from(generic_eq(&desc, &some_a, &some_a2)),
            "equal Some values agree"
        );
        assert!(
            !bool::from(generic_eq(&desc, &some_a, &some_b)),
            "Some values differing in a leaf disagree"
        );
        assert!(
            !bool::from(generic_eq(&desc, &some_a, &none)),
            "different constructors disagree"
        );
        assert!(
            bool::from(generic_eq(&desc, &none, &none)),
            "None equals None"
        );
    }
    #[test]
    fn serialization_is_deterministic_and_agrees_with_equality()
    {
        let desc = maybe_desc();
        let some_a = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"a"[..])));
        let some_a2 = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"a"[..])));
        let some_b = DescValue::new(1.into(), Payload::Leaf(Box::from(&b"b"[..])));

        assert_eq!(
            serialize_value(&desc, &some_a),
            serialize_value(&desc, &some_a2),
            "equal values encode identically"
        );
        assert_ne!(
            serialize_value(&desc, &some_a),
            serialize_value(&desc, &some_b),
            "distinct values encode differently"
        );
        // Tag 1 (LE u32) + length 1 (LE u32) + byte 'a'.
        assert_eq!(
            &[1u8, 0, 0, 0, 1, 0, 0, 0, b'a'],
            serialize_value(&desc, &some_a).as_ref(),
            "the canonical encoding is tag-then-payload"
        );
    }
    #[test]
    fn desc_inspection_renders_the_structure()
    {
        assert_eq!(
            "data Maybe(a) { None = 1, Some = a }",
            serialize_desc(&maybe_desc()).as_ref(),
            "the inspection notation names the polarity, parameters, and coded constructors"
        );
    }
    /// The `Maybe(a)` description: `None = 1`, `Some = field a`.
    fn maybe_desc() -> DataDesc
    {
        DataDesc::new(
            NominalId::new(0.into(), "Maybe"),
            [crate::desc::ParamDesc::new("a", Grade::ONE, Attrs::empty())],
            [
                CtorDesc::new("None", Code::Unit, None, Attrs::empty()),
                CtorDesc::new(
                    "Some",
                    Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty()),
                    None,
                    Attrs::empty(),
                ),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    #[test]
    fn generic_eq_recurses_through_var()
    {
        let desc = nat_desc();
        // 2 = Succ(Succ(Zero)).
        let two = DescValue::new(
            1.into(),
            Payload::Rec(Box::new(DescValue::new(
                1.into(),
                Payload::Rec(Box::new(DescValue::new(0.into(), Payload::Unit))),
            ))),
        );
        let two_again = two.clone();
        // 1 = Succ(Zero).
        let one = DescValue::new(
            1.into(),
            Payload::Rec(Box::new(DescValue::new(0.into(), Payload::Unit))),
        );
        assert!(
            bool::from(generic_eq(&desc, &two, &two_again)),
            "2 == 2 through var"
        );
        assert!(
            !bool::from(generic_eq(&desc, &two, &one)),
            "2 ≠ 1 through var"
        );
    }
    /// A `Nat`-like recursive description: `Zero = 1`, `Succ = var`.
    fn nat_desc() -> DataDesc
    {
        DataDesc::new(
            NominalId::new(1.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, None, Attrs::empty()),
                CtorDesc::new("Succ", Code::Var, None, Attrs::empty()),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    #[test]
    fn desc_inspection_renders_graded_and_attributed_fields()
    {
        let desc = DataDesc::new(
            NominalId::new(0.into(), "Vec"),
            [crate::desc::ParamDesc::new("a", Grade::ONE, Attrs::empty())],
            [
                CtorDesc::new("Nil", Code::Unit, None, Attrs::empty()),
                CtorDesc::new(
                    "Cons",
                    Code::prod(
                        Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty()),
                        Code::Var,
                    ),
                    None,
                    Attrs::new([crate::code::Attr::marker("ctor")]),
                ),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            "data Vec(a) { Nil = 1, Cons = (a × var) }",
            serialize_desc(&desc).as_ref(),
            "a right-nested product renders its factors"
        );
    }

    #[test]
    fn desc_inspection_renders_primitive_fields()
    {
        let desc = DataDesc::new(
            NominalId::new(0.into(), "Wrap"),
            Vec::new(),
            [CtorDesc::new(
                "Wrap",
                Code::field(
                    ValueTypeRef::Prim(PrimTy::Integer),
                    Grade::ONE,
                    Attrs::empty(),
                ),
                None,
                Attrs::empty(),
            )],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            "data Wrap { Wrap = Integer }",
            serialize_desc(&desc).as_ref(),
            "a primitive field renders its spelling"
        );
    }
}
