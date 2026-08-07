//! **Generic programs over descriptions** — the stage-0 payoff (proposal §3,
//! §11).
//!
//! These functions are the real consumers that keep the decl table from being
//! "a beautiful decl table nobody decodes": each is one Rust program driven by
//! a [`SignDesc`] / [`Code`], covering declared data and retrofitted builtins
//! ([`crate::builtin`]) uniformly.
//!
//! * [`generic_eq`] — structural equality of two [`DescValue`]s, guided by the
//!   description (the derive-style structural-eq payoff);
//! * [`serialize_value`] — a canonical, deterministic byte encoding of a
//!   [`DescValue`], guided by the description (the wire-serialization payoff);
//! * [`serialize_desc`] — a canonical textual rendering of the description's
//!   `sign` normal form (the inspectable-IR payoff).
//!
//! A [`DescValue`] is a *generic* value of a described datatype: a constructor
//! tag plus a [`Payload`] shaped by that constructor's [`Code`]. Recursive
//! occurrences ([`Code::Var`]) nest a whole [`DescValue`] (tagged again), which
//! is why the generic programs are driven by the [`SignDesc`] (the σ tag), not
//! a bare [`Code`].

use gandr_core_checker::boundary::ConstructorTag;

use crate::boundary::GenericEquality;
use crate::boundary::SerializedDescText;
use crate::boundary::SerializedValueBytes;
use crate::code::Code;
use crate::code::Name;
use crate::desc::OperDesc;
use crate::desc::SignDesc;

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
/// into [`SignDesc::ctors`]) plus its [`Payload`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DescValue
{
    /// The constructor index into [`SignDesc::ctors`].
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
    desc: &SignDesc,
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
    desc: &SignDesc,
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
                | (
                    &Code::Var(_),
                    &Payload::Rec(ref inner_left),
                    &Payload::Rec(ref inner_right),
                ) => {
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
    desc: &SignDesc,
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
                | (&Code::Var(_), &Payload::Rec(ref inner)) => {
                    stack.push(EncodeTask::Value(inner));
                },
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

/// **Inspectable rendering** of a description's `sign` normal form to
/// canonical text (proposal §8's `desc-inspect` payoff), in the **ruled
/// surface spelling**.
///
/// Every description renders as its `sign` normal form — the canonical block
/// of the signature unification (gandr-ng9.18 ruling 5, subsuming the
/// inspection-notation half of gandr-r38) — with the declared sort set spelled
/// first and the arrow grid's `-->` / `==>` at every arrow position (`~>`
/// never appears). The normal form's members are sorts, operations, and rules
/// only: the item-level `data` / `codata` member is retired (a family is
/// declared once, whole, as a nested generator block), so constructor and
/// observation descriptors have no member spelling here and the render omits
/// them.
///
/// The rendering is deterministic and structural — a testing/inspection
/// notation, not a surface pretty-printer (that surface is owned elsewhere).
/// Members join with `; ` so the whole description stays one line.
///
/// # Contract
/// - ensures: deterministic output for a given description; total — never
///   panics.
#[inline]
#[must_use]
pub fn serialize_desc(desc: &SignDesc) -> SerializedDescText
{
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
    let mut members: Vec<String> = Vec::new();
    for sort in &desc.sorts {
        members.push(format!("sort {} : Type", sort.name));
    }
    // Constructor and observation descriptors are NOT rendered: the item-level
    // `data` / `codata` member is retired from the `sign` normal form — a
    // family is declared once, whole, as a nested generator block, and a
    // `sign` block presents sorts, operations, and rules only — so the sign
    // render carries no member spelling for them. The descriptors themselves
    // are unchanged and remain inspectable through `desc.ctors`.
    for oper in &desc.opers {
        members.push(render_oper_member(oper));
    }
    for rule in &desc.rules {
        members.push(format!("rule {}", render_face(rule)));
    }
    for rule in &desc.circuits {
        let telescope = render_telescope(&rule.ports);
        members.push(format!(
            "rule {} : {telescope}{}{}",
            rule.name,
            if telescope.is_empty() { "" } else { " " },
            render_face(&rule.sphere)
        ));
    }
    if members.is_empty() {
        format!("sign {}{params} {{}}", desc.id.name).into()
    }
    else {
        format!("sign {}{params} {{ {} }}", desc.id.name, members.join("; ")).into()
    }
}

/// Render one operation as its ruled judgment-style member, in the named-port
/// normal form the elaborator sees (`oper add : (m : Nat, n : Nat) --> (q :
/// Nat)`); a port with no name spells its sort bare, and a single unnamed
/// output drops its parentheses.
fn render_oper_member(oper: &OperDesc) -> String
{
    let inputs: Vec<String> = oper.arity.inputs.iter().map(render_port).collect();
    let outputs: Vec<String> = oper.arity.outputs.iter().map(render_port).collect();
    let outputs = match *oper.arity.outputs {
        | [ref only] if authored_port_name(only).is_none() => outputs.concat(),
        | _ => format!("({})", outputs.join(", ")),
    };
    format!("oper {} : ({}) --> {outputs}", oper.name, inputs.join(", "))
}

/// The port's authored name, or [`None`] for an anonymous port (an empty
/// name, or one of the minted underscore-led placeholders the named-port
/// normal form assigns to unnamed tuple entries).
fn authored_port_name(port: &crate::arity::SortRef) -> Option<&Name>
{
    let name = port.name.as_ref();
    if name.is_empty() || name.starts_with('_') {
        None
    }
    else {
        Some(&port.name)
    }
}

/// Render one named port (`m : Nat`), spelling the sort bare when the port
/// carries no authored name.
fn render_port(port: &crate::arity::SortRef) -> String
{
    match authored_port_name(port) {
        | Some(name) => format!("{name} : {}", port.sort),
        | None => port.sort.to_string(),
    }
}

/// Render a circuit rule's **parameter telescope** — its rewrite-sorted ports,
/// each in the ruled binder spelling.
///
/// An empty telescope renders as nothing at all. A port renders at the form
/// its declaration wrote: `rule p : Nat ==> Nat` for the sorted form, `rule p
/// : x ==> x′` for the pinned one — the same `==>` a face renders with.
fn render_telescope(ports: &[crate::elaborate::RewritePort]) -> String
{
    if ports.is_empty() {
        return String::new();
    }
    let rendered: Vec<String> = ports
        .iter()
        .map(|port| match port.face {
            | crate::elaborate::PortFace::Sorted(ref sort) => {
                format!("rule {} : {sort} ==> {sort}", port.name)
            },
            | crate::elaborate::PortFace::Pinned {
                ref source,
                ref target,
            } => format!(
                "rule {} : {} ==> {}",
                port.name,
                render_free_term(source),
                render_free_term(target)
            ),
        })
        .collect();
    format!("({})", rendered.join(", "))
}

/// Render a [`crate::RuleFace`] to the inspection notation (`lhs ==> rhs` —
/// the ruled rewrite-face former at every position; `~>` is retired).
fn render_face(rule: &crate::rule::RuleFace) -> String
{
    format!(
        "{} ==> {}",
        render_free_term(&rule.lhs),
        render_free_term(&rule.rhs)
    )
}

/// Render a [`crate::FreeTerm`] to the inspection notation.
///
/// Shared with [`crate::wellformed`], whose boundary diagnostics quote the two
/// terms they compare in the same notation the description IR renders them in.
pub(crate) fn render_free_term(term: &crate::rule::FreeTerm) -> String
{
    enum TermFrame<'term>
    {
        Render(&'term crate::rule::FreeTerm),
        FinishApp(&'term Name, usize),
    }

    let mut stack = vec![TermFrame::Render(term)];
    let mut rendered = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | TermFrame::Render(node) => match *node {
                | crate::rule::FreeTerm::Var(ref name) => {
                    rendered.push(name.to_string());
                },
                | crate::rule::FreeTerm::Ctor(ref name, ref args)
                | crate::rule::FreeTerm::Op(ref name, ref args)
                    if args.is_empty() =>
                {
                    rendered.push(name.to_string());
                },
                | crate::rule::FreeTerm::Ctor(ref name, ref args)
                | crate::rule::FreeTerm::Op(ref name, ref args) => {
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
    use crate::code::ValueTypeRef;
    use crate::desc::CtorDesc;
    use crate::desc::DeclPolarity;
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
            "sign Maybe(a) { sort Maybe : Type }",
            serialize_desc(&maybe_desc()).as_ref(),
            "the inspection notation names the parameters and the sort set; constructors have \
             no item-level member spelling in the sign normal form"
        );
    }
    /// The `Maybe(a)` description: `None = 1`, `Some = field a`.
    fn maybe_desc() -> SignDesc
    {
        SignDesc::new(
            NominalId::new(0.into(), "Maybe"),
            [crate::desc::ParamDesc::new("a", Grade::ONE, Attrs::empty())],
            [
                CtorDesc::new("None", Code::Unit, "Maybe", Attrs::empty()),
                CtorDesc::new(
                    "Some",
                    Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty()),
                    "Maybe",
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
    fn nat_desc() -> SignDesc
    {
        SignDesc::new(
            NominalId::new(1.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, "Nat", Attrs::empty()),
                CtorDesc::new("Succ", Code::var("Nat"), "Nat", Attrs::empty()),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    #[test]
    fn desc_inspection_omits_constructor_members()
    {
        let desc = SignDesc::new(
            NominalId::new(0.into(), "Vec"),
            [crate::desc::ParamDesc::new("a", Grade::ONE, Attrs::empty())],
            [
                CtorDesc::new("Nil", Code::Unit, "Vec", Attrs::empty()),
                CtorDesc::new(
                    "Cons",
                    Code::prod(
                        Code::field(ValueTypeRef::Param("a".into()), Grade::ONE, Attrs::empty()),
                        Code::var("Vec"),
                    ),
                    "Vec",
                    Attrs::new([crate::code::Attr::marker("ctor")]),
                ),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            "sign Vec(a) { sort Vec : Type }",
            serialize_desc(&desc).as_ref(),
            "constructors — graded, attributed, or right-nested — carry no member spelling: \
             the item-level `data` member is retired from the sign normal form"
        );
    }

    #[test]
    fn desc_inspection_renders_a_circuit_rule_and_its_telescope()
    {
        use crate::circuit::CircuitBody;
        use crate::circuit::CircuitFrame;
        use crate::circuit::CircuitNode;
        use crate::circuit::CircuitRedex;
        use crate::circuit::CircuitRule;
        use crate::circuit::FrameHead;
        use crate::elaborate::RewritePort;

        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    crate::rule::FreeTerm::var("x"),
                    crate::rule::FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [
                        crate::rule::FreeTerm::var("x\u{2032}"),
                        crate::rule::FreeTerm::var("y"),
                    ],
                    "z",
                )),
            ],
            "z",
        );
        let derived = crate::circuit::derive_boundaries(&body).expect("derives");
        let rule = CircuitRule::new(
            "cong1",
            crate::rule::RuleFace::new(
                derived.source,
                derived.target,
                Vec::new(),
                crate::desc::SurfaceSpan::new(0_usize.into(), 0_usize.into()),
            ),
            body,
        )
        .with_ports([RewritePort::sorted("p", "Nat")]);
        let desc = SignDesc::new(
            NominalId::new(0.into(), "Nat"),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        )
        .with_circuits([rule]);
        assert_eq!(
            "sign Nat { sort Nat : Type; rule cong1 : (rule p : Nat ==> Nat) add(x, y) ==> add(x\u{2032}, y) }",
            serialize_desc(&desc).as_ref(),
            "a circuit member renders its telescope and its sphere"
        );
    }

    #[test]
    fn a_description_carrying_no_circuit_renders_exactly_as_before()
    {
        let desc = SignDesc::new(
            NominalId::new(0.into(), "Bit"),
            Vec::new(),
            [CtorDesc::new("Off", Code::Unit, "Bit", Attrs::empty())],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            "sign Bit { sort Bit : Type }",
            serialize_desc(&desc).as_ref(),
            "the circuit slot is invisible when it is empty, and the lone constructor has no \
             member spelling"
        );
    }

    #[test]
    fn desc_inspection_omits_a_primitive_field_constructor()
    {
        let desc = SignDesc::new(
            NominalId::new(0.into(), "Wrap"),
            Vec::new(),
            [CtorDesc::new(
                "Wrap",
                Code::field(
                    ValueTypeRef::Prim(PrimTy::Integer),
                    Grade::ONE,
                    Attrs::empty(),
                ),
                "Wrap",
                Attrs::empty(),
            )],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            "sign Wrap { sort Wrap : Type }",
            serialize_desc(&desc).as_ref(),
            "a primitive-field constructor renders no member spelling either"
        );
    }
}
