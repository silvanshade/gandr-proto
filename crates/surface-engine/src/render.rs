//! Shared renderers from semantic values to presentation strings.
//!
//! The type-rendering trio and the structural value renderer are the
//! machine-state → presentation projection, kept outside the core (no core
//! `Display`; the `use_debug` lint forbids `Debug` in user-facing output).
//!
//! This is the tree's single spelling of these values, and there is no second
//! copy to reconcile with: the REPL, the language-server face, and the corpus
//! harness's expectations all consume it. Until a fuller pretty-printer lands,
//! a face consumes this module.

use core::fmt::Write as _;

use gandr_core_term::outcome::Eval;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::NumLit;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

/// Renders a type (either polarity).
///
/// # Contract
/// - ensures: total over `Ty`.
/// - panics: none for interactive-scale types; recursion follows the type
///   structure, whose depth callers guard.
#[inline]
#[must_use]
pub fn ty(ty: &Ty) -> String
{
    match *ty {
        | Ty::Value(ref value_type) => value_ty(value_type),
        | Ty::Comp(ref comp_type) => comp_ty(comp_type),
    }
}
/// Renders a type only when every node has a shared surface spelling.
///
/// This keeps unsupported-node detection structural: a `?` byte in an atom or
/// declared name remains ordinary user data rather than masquerading as the
/// renderer's wildcard.
#[inline]
#[must_use]
pub(crate) fn faithful_ty(ty: &Ty) -> Option<String>
{
    let rendered = match *ty {
        | Ty::Value(ref value_type) => render_type(RenderNode::Value(value_type)),
        | Ty::Comp(ref comp_type) => render_type(RenderNode::Comp(comp_type)),
    };
    rendered.faithful.then_some(rendered.text)
}

/// Renders a value type.
///
/// # Contract
/// - ensures: total over `ValueType`; the grade is elided (the fuller pretty
///   printer is designed direction).
/// - panics: none.
#[inline]
#[must_use]
pub fn value_ty(value_type: &ValueType) -> String
{
    render_type(RenderNode::Value(value_type)).text
}

/// Renders a computation type.
///
/// # Contract
/// - ensures: total over `CompType`.
/// - panics: none.
#[inline]
#[must_use]
pub fn comp_ty(comp_type: &CompType) -> String
{
    render_type(RenderNode::Comp(comp_type)).text
}

/// A borrowed type node waiting to be rendered.
enum RenderNode<'ty>
{
    /// A value-type node.
    Value(&'ty ValueType),
    /// A computation-type node.
    Comp(&'ty CompType),
}

/// A pending assembly step in the iterative type renderer.
enum RenderTask<'ty>
{
    /// Render one type node.
    Node(RenderNode<'ty>),
    /// Assemble two rendered operands around an infix symbol.
    Infix(&'static str),
    /// Prefix one rendered operand.
    Prefix(&'static str),
    /// Assemble a dependent function type around its binder.
    Pi(String),
    /// Assemble a computation returner and its effect marker.
    Returner(bool),
    /// Assemble a declared-data application.
    Data
    {
        /// The nominal type name.
        name: String,
        /// The number of rendered type arguments.
        argument_count: usize,
    },
}
/// One rendered type plus whether every node had a shared spelling.
struct TypeRendering
{
    /// Rendered presentation text.
    text: String,
    /// Whether no unsupported-node wildcard was required.
    faithful: bool,
}

/// Renders a finite value/computation-type tree with an explicit work stack.
fn render_type(root: RenderNode<'_>) -> TypeRendering
{
    let mut pending = vec![RenderTask::Node(root)];
    let mut rendered = Vec::new();
    let mut faithful = true;
    while let Some(task) = pending.pop() {
        match task {
            | RenderTask::Node(RenderNode::Value(value_type)) => match *value_type {
                | ValueType::Atom(ref name) => rendered.push(name.clone()),
                | ValueType::Unit => rendered.push("Unit".to_owned()),
                | ValueType::Prod(ref fst, ref snd) => {
                    pending.push(RenderTask::Infix("×"));
                    pending.push(RenderTask::Node(RenderNode::Value(snd)));
                    pending.push(RenderTask::Node(RenderNode::Value(fst)));
                },
                | ValueType::Sum(ref lhs, ref rhs) => {
                    pending.push(RenderTask::Infix("+"));
                    pending.push(RenderTask::Node(RenderNode::Value(rhs)));
                    pending.push(RenderTask::Node(RenderNode::Value(lhs)));
                },
                | ValueType::List(ref element) => {
                    pending.push(RenderTask::Prefix("List "));
                    pending.push(RenderTask::Node(RenderNode::Value(element)));
                },
                | ValueType::Thunk(_, ref body) => {
                    pending.push(RenderTask::Prefix("U "));
                    pending.push(RenderTask::Node(RenderNode::Comp(body)));
                },
                | ValueType::Stk(ref consumes, ref delivers) => {
                    pending.push(RenderTask::Infix("Stk"));
                    pending.push(RenderTask::Node(RenderNode::Comp(delivers)));
                    pending.push(RenderTask::Node(RenderNode::Comp(consumes)));
                },
                | ValueType::Data { ref id, ref args } => {
                    pending.push(RenderTask::Data {
                        name: id.name().as_ref().to_owned(),
                        argument_count: args.len(),
                    });
                    pending.extend(
                        args.iter()
                            .rev()
                            .map(|arg| RenderTask::Node(RenderNode::Value(arg))),
                    );
                },
                | _ => {
                    faithful = false;
                    rendered.push("?".to_owned());
                },
            },
            | RenderTask::Node(RenderNode::Comp(comp_type)) => match *comp_type {
                | CompType::F(ref payload, ref row) => {
                    pending.push(RenderTask::Returner(bool::from(row.is_empty())));
                    pending.push(RenderTask::Node(RenderNode::Value(payload)));
                },
                // A non-dependent arrow renders exactly as it always did. A
                // dependent one renders its binder, because a reader shown
                // `A → B` for `Π(x : A). B` cannot see where the codomain's
                // occurrences of `x` come from.
                | CompType::Arrow {
                    binder: None,
                    ref arg,
                    ref res,
                } => {
                    pending.push(RenderTask::Infix("→"));
                    pending.push(RenderTask::Node(RenderNode::Comp(res)));
                    pending.push(RenderTask::Node(RenderNode::Value(arg)));
                },
                | CompType::Arrow {
                    binder: Some(ref binder),
                    ref arg,
                    ref res,
                } => {
                    pending.push(RenderTask::Pi(binder.clone()));
                    pending.push(RenderTask::Node(RenderNode::Comp(res)));
                    pending.push(RenderTask::Node(RenderNode::Value(arg)));
                },
                | CompType::With(ref fst, ref snd) => {
                    pending.push(RenderTask::Infix("&"));
                    pending.push(RenderTask::Node(RenderNode::Comp(snd)));
                    pending.push(RenderTask::Node(RenderNode::Comp(fst)));
                },
                | _ => {
                    faithful = false;
                    rendered.push("?".to_owned());
                },
            },
            | RenderTask::Infix(symbol) => {
                let rhs = rendered.pop().unwrap_or_else(|| "?".to_owned());
                let lhs = rendered.pop().unwrap_or_else(|| "?".to_owned());
                if symbol == "Stk" {
                    rendered.push(format!("Stk({lhs}, {rhs})"));
                }
                else {
                    rendered.push(format!("({lhs} {symbol} {rhs})"));
                }
            },
            | RenderTask::Pi(binder) => {
                let res = rendered.pop().unwrap_or_else(|| "?".to_owned());
                let arg = rendered.pop().unwrap_or_else(|| "?".to_owned());
                rendered.push(format!("Π({binder} : {arg}). {res}"));
            },
            | RenderTask::Prefix(prefix) => {
                let body = rendered.pop().unwrap_or_else(|| "?".to_owned());
                rendered.push(format!("{prefix}{body}"));
            },
            | RenderTask::Returner(empty) => {
                let payload = rendered.pop().unwrap_or_else(|| "?".to_owned());
                rendered.push(if empty {
                    format!("F {payload}")
                }
                else {
                    format!("F {payload} !ε")
                });
            },
            | RenderTask::Data {
                name,
                argument_count,
            } => {
                if argument_count == 0 {
                    rendered.push(name);
                    continue;
                }
                let start = rendered.len().saturating_sub(argument_count);
                let arguments = rendered.split_off(start);
                rendered.push(format!("{name}({})", arguments.join(", ")));
            },
        }
    }
    let text = rendered.pop().unwrap_or_else(|| {
        faithful = false;
        "?".to_owned()
    });
    TypeRendering { text, faithful }
}

/// The maximum depth [`value`] descends before rendering `<deep>`
/// (bounded rendering; the ADR-47 posture).
pub const RENDER_DEPTH_LIMIT: RenderDepth = RenderDepth(32);

/// Current depth in the bounded value renderer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RenderDepth(usize);

impl RenderDepth
{
    /// Root depth for a freshly rendered value.
    pub const ROOT: Self = Self(0);

    /// Descends one level without overflowing the host representation.
    #[inline]
    fn descend(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }
}

impl From<usize> for RenderDepth
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

/// Renders a machine [`Value`] in the structural notation every face reads.
///
/// Annotations are transparent, booleans appear as their `1 + 1` encoding
/// (`Inl(())` / `Inr(())`), thunks render opaquely, and anything unrecognized
/// renders `<opaque>`. Rendering is depth-bounded (`<deep>` beyond
/// [`RENDER_DEPTH_LIMIT`]). Records iterate their `BTreeMap` order, so output
/// is deterministic for a given value.
///
/// # Contract
/// - ensures: deterministic output for a given value; total — never panics.
#[inline]
#[must_use]
pub fn value<T>(
    value: &Value,
    depth: T,
) -> String
where
    T: Into<RenderDepth>,
{
    enum RenderStep<'value>
    {
        Value
        {
            value: &'value Value,
            depth: RenderDepth,
        },
        Text(&'value str),
    }

    let mut output = String::new();
    let mut steps = vec![RenderStep::Value {
        value,
        depth: depth.into(),
    }];
    while let Some(step) = steps.pop() {
        let RenderStep::Value { value, depth } = step
        else {
            let RenderStep::Text(text) = step
            else {
                continue;
            };
            output.push_str(text);
            continue;
        };
        if depth >= RENDER_DEPTH_LIMIT {
            output.push_str("<deep>");
            continue;
        }
        let below = depth.descend();
        match *value {
            | Value::Unit => output.push_str("()"),
            | Value::Int(int) => {
                let _infallible = write!(&mut output, "{int}");
            },
            | Value::Str(ref text) => {
                output.push('"');
                for ch in text.chars() {
                    output.extend(ch.escape_debug());
                }
                output.push('"');
            },
            | Value::Num(num) => output.push_str(&render_num(num)),
            | Value::Pair(ref fst, ref snd) => {
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: snd.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text(", "));
                steps.push(RenderStep::Value {
                    value: fst.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text("("));
            },
            | Value::Inj(side, ref payload) => {
                let prefix = match side {
                    | Side::Fst => "Inl(",
                    | Side::Snd => "Inr(",
                };
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: payload.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text(prefix));
            },
            | Value::List(ref items) => {
                steps.push(RenderStep::Text("]"));
                for (index, item) in items.iter().enumerate().rev() {
                    if index.saturating_add(1) < items.len() {
                        steps.push(RenderStep::Text(", "));
                    }
                    steps.push(RenderStep::Value {
                        value: item.as_ref(),
                        depth: below,
                    });
                }
                steps.push(RenderStep::Text("["));
            },
            | Value::Record(ref fields) => {
                steps.push(RenderStep::Text("}"));
                for (index, (label, field)) in fields.iter().enumerate().rev() {
                    if index.saturating_add(1) < fields.len() {
                        steps.push(RenderStep::Text(", "));
                    }
                    steps.push(RenderStep::Value {
                        value: field.as_ref(),
                        depth: below,
                    });
                    steps.push(RenderStep::Text(" = "));
                    steps.push(RenderStep::Text(label));
                }
                steps.push(RenderStep::Text("#{"));
            },
            | Value::Thunk(..) => output.push_str("<thunk>"),
            | Value::Annot(ref payload, _) => steps.push(RenderStep::Value {
                value: payload.as_ref(),
                depth: below,
            }),
            | Value::Var(ref name) => {
                output.push_str("<var ");
                output.push_str(name.as_ref());
                output.push('>');
            },
            // A reflexivity proof renders through its witness (ADR-76): the
            // canonical inhabitant of a closed identity type, `here(4)`.
            | Value::Here(ref witness) => {
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: witness.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text("here("));
            },
            | _ => output.push_str("<opaque>"),
        }
    }
    output
}

/// Renders an evaluation outcome in the transcript notation.
///
/// A produced value renders in the structural notation ([`value`]); a
/// function terminal renders `<fun>`, and a defined halt or an undefined
/// stuck renders its class — the corpus harness's directive grammar carries
/// the fine-grained blame and stuck labels, which a transcript line does not
/// need. Total over [`Eval`].
///
/// # Contract
/// - ensures: total over `Eval`; never panics.
#[inline]
#[must_use]
pub fn eval(eval: &Eval) -> String
{
    match *eval {
        | Eval::Value(Comp::Ret(ref produced)) => value(produced.as_ref(), RenderDepth::ROOT),
        | Eval::Value(Comp::Abs(..)) => String::from("<fun>"),
        | Eval::Value(_) => String::from("<opaque>"),
        | Eval::Blame(_) => String::from("<blame>"),
        | Eval::Stuck(_) => String::from("<stuck>"),
    }
}

/// Renders a typed numeric literal (`5u32`, `1.5f64`, …).
fn render_num(num: NumLit) -> String
{
    match num {
        | NumLit::U32(n) => format!("{n}u32"),
        | NumLit::U64(n) => format!("{n}u64"),
        | NumLit::I32(n) => format!("{n}i32"),
        | NumLit::I64(n) => format!("{n}i64"),
        | NumLit::F32(bits) => format!("{}f32", f32::from_bits(bits)),
        | NumLit::F64(bits) => format!("{}f64", f64::from_bits(bits)),
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_term::boundary::TypeAtomName;
    use gandr_core_term::effect::EffectRow;
    use gandr_core_term::effect::EffectSig;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::DataId;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;
    #[test]
    fn value_ty_covers_every_reachable_former()
    {
        // Every positive former, plus the two `?`-wildcard cases (`Record`,
        // `Unknown`), built through the public constructors.
        let cases: [(ValueType, &str); 11] = [
            (atom("A"), "A"),
            (ValueType::Unit, "Unit"),
            (ValueType::prod(atom("A"), ValueType::Unit), "(A × Unit)"),
            (ValueType::sum(atom("L"), atom("R")), "(L + R)"),
            (ValueType::list(atom("E")), "List E"),
            (
                ValueType::thunk(Grade::ONE, CompType::returner(ValueType::Unit)),
                "U F Unit",
            ),
            (
                ValueType::stk(CompType::returner(atom("B")), CompType::returner(atom("C"))),
                "Stk(F B, F C)",
            ),
            // The declared-data handle renders its surface spelling (the declared-data contract):
            // a parameterless datatype is its bare name, an applied one is
            // name-and-arguments.
            (
                ValueType::data(DataId::new(0, "Celsius"), Vec::new()),
                "Celsius",
            ),
            (
                ValueType::data(DataId::new(1, "Maybe"), Vec::from([ValueType::integer()])),
                "Maybe(Integer)",
            ),
            (ValueType::record([("x".to_owned(), atom("A"))]), "?"),
            (ValueType::Unknown, "?"),
        ];
        for case in &cases {
            let input = &case.0;
            let expected = case.1;
            assert_eq!(super::value_ty(input), expected, "rendering {input:?}");
        }
    }
    #[test]
    fn comp_ty_covers_every_reachable_former()
    {
        // Pure returner (empty row) vs. effectful returner (`!ε` suffix).
        assert_eq!(
            "F Unit",
            super::comp_ty(&CompType::returner(ValueType::Unit))
        );
        let row = EffectRow::singleton(EffectSig::new("St".into(), Vec::new()));
        assert_eq!(
            "F A !ε",
            super::comp_ty(&CompType::returner_eff(atom("A"), row))
        );

        let arrow = CompType::arrow(atom("A"), CompType::returner(ValueType::Unit));
        assert_eq!("(A → F Unit)", super::comp_ty(&arrow));

        let with = CompType::with(CompType::returner(atom("B")), CompType::returner(atom("C")));
        assert_eq!("(F B & F C)", super::comp_ty(&with));

        // The `Unknown` computation type hits the `?` wildcard.
        assert_eq!("?", super::comp_ty(&CompType::Unknown));
    }
    #[test]
    fn ty_dispatches_on_polarity()
    {
        assert_eq!("A", super::ty(&Ty::Value(atom("A"))));
        assert_eq!("?", super::ty(&Ty::Value(ValueType::Unknown)));
        assert_eq!("?", super::ty(&Ty::Comp(CompType::Unknown)));
    }
    #[test]
    fn fidelity_tracks_unsupported_nodes_not_user_punctuation()
    {
        assert_eq!(
            Some(String::from("A?")),
            super::faithful_ty(&Ty::Value(atom("A?")))
        );
        assert_eq!(None, super::faithful_ty(&Ty::Value(ValueType::Unknown)));
    }

    #[test]
    fn eval_renders_each_outcome_class()
    {
        use gandr_core_term::outcome::Blame;
        use gandr_core_term::outcome::Eval;
        use gandr_core_term::outcome::StuckReason;
        use gandr_core_term::syntax::Comp;
        use gandr_core_term::syntax::Value;

        assert_eq!(
            "42",
            super::eval(&Eval::Value(Comp::ret(Value::int(42)))),
            "a produced value renders in the structural notation"
        );
        assert_eq!(
            "\"line\\n\\t\\\"\\\\tail\"",
            super::eval(&Eval::Value(Comp::ret(Value::string("line\n\t\"\\tail")))),
            "string values stay one escaped transcript line"
        );
        let lambda = Comp::Abs(
            String::from("x"),
            None,
            alloc::rc::Rc::new(Comp::ret(Value::var("x"))),
        );
        assert_eq!(
            "<fun>",
            super::eval(&Eval::Value(lambda)),
            "a function terminal renders opaquely"
        );
        assert_eq!(
            "<blame>",
            super::eval(&Eval::Blame(Blame::Hole)),
            "a defined halt renders its class"
        );
        assert_eq!(
            "<stuck>",
            super::eval(&Eval::Stuck(StuckReason::StepLimit)),
            "an undefined stuck renders its class"
        );
    }

    fn atom<'name>(name: impl Into<TypeAtomName<'name>>) -> ValueType
    {
        ValueType::atom(name)
    }

    #[test]
    fn types_render_without_debug()
    {
        let int = ValueType::atom("Integer");
        assert_eq!("Integer", super::value_ty(&int));
        assert_eq!(
            "F Unit",
            super::ty(&Ty::Comp(CompType::returner(ValueType::Unit)))
        );
    }
}
