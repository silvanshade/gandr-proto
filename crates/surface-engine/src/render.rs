//! Shared type renderers from semantic values to presentation strings.
//!
//! The type-rendering trio is the shared machine-state → presentation
//! projection used by the REPL and LSP, kept outside the core in accordance
//! with decision D3 (no core `Display`; the `use_debug` lint forbids `Debug` in
//! user-facing output).
//!
//! This duplication of the REPL bin's renderer remains sanctioned-temporary:
//! the shared pretty-printer family (`gandr-doc`,
//! `proposal-pretty-printing.md`) replaces both copies.

use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;

/// Renders a type (either polarity).
///
/// # Contract
/// - ensures: total over `Ty` (unknown variants render as `?`).
/// - panics: none for interactive-scale types; recursion follows the type
///   structure, whose depth callers guard.
#[inline]
#[must_use]
pub fn ty(ty: &Ty) -> String
{
    match *ty {
        | Ty::Value(ref value_type) => value_ty(value_type),
        | Ty::Comp(ref comp_type) => comp_ty(comp_type),
        | _ => "?".to_owned(),
    }
}

/// Renders a value type.
///
/// # Contract
/// - ensures: total over `ValueType`; the grade is elided (the fuller pretty
///   printer is scoped work, `proposal-pretty-printing.md`).
/// - panics: none.
#[inline]
#[must_use]
pub fn value_ty(value_type: &ValueType) -> String
{
    render_type(RenderNode::Value(value_type))
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
    render_type(RenderNode::Comp(comp_type))
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

/// Renders a finite value/computation-type tree with an explicit work stack.
fn render_type(root: RenderNode<'_>) -> String
{
    let mut pending = vec![RenderTask::Node(root)];
    let mut rendered = Vec::new();
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
                | _ => rendered.push("?".to_owned()),
            },
            | RenderTask::Node(RenderNode::Comp(comp_type)) => match *comp_type {
                | CompType::F(ref payload, ref row) => {
                    pending.push(RenderTask::Returner(bool::from(row.is_empty())));
                    pending.push(RenderTask::Node(RenderNode::Value(payload)));
                },
                | CompType::Arrow(ref arg, ref res) => {
                    pending.push(RenderTask::Infix("→"));
                    pending.push(RenderTask::Node(RenderNode::Comp(res)));
                    pending.push(RenderTask::Node(RenderNode::Value(arg)));
                },
                | CompType::With(ref fst, ref snd) => {
                    pending.push(RenderTask::Infix("&"));
                    pending.push(RenderTask::Node(RenderNode::Comp(snd)));
                    pending.push(RenderTask::Node(RenderNode::Comp(fst)));
                },
                | _ => rendered.push("?".to_owned()),
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
    rendered.pop().unwrap_or_else(|| "?".to_owned())
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::TypeAtomName;
    use gandr_core_checker::effect::EffectRow;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::DataId;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
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
            // The declared-data handle renders its surface spelling (the design record): a
            // parameterless datatype is its bare name, an applied one is
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
