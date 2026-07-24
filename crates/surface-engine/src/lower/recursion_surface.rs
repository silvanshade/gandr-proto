//! Recursive-scope resolution for the ratified call-site evidence surface.
//!
//! This module is deliberately pre-kernel. It resolves which bare names are
//! fix-bound and classifies instantiation-slot residents before ordinary CBPV
//! lowering sees the expression. The current rung validates scope and declines
//! reserved residents; guardedness and sized elaboration remain later rungs.

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::boundary::NodeText;
use crate::boundary::ShadowPresence;
use crate::lower::LowerError;
use crate::lower::LowerResult;
use crate::lower::node_kinds;
use crate::synnode::SynNode;

/// Validate every recursion marker and fix-bound reference in one parsed file.
#[cfg(test)]
fn validate(root: SynNode<'_>) -> LowerResult<()>
{
    for item in root.named_children() {
        validate_item(item)?;
    }
    Ok(())
}

/// Validate recursion markers and fix-bound references in one source item.
///
/// # Contract
/// - requires: `item` belongs to the current parsed source tree.
/// - ensures: every direction-marked target belongs to its enclosing recursive
///   scope and no fix-bound term name is referenced without a marker.
/// - provides: item-local resolver/checker validation, preserving lowering's
///   source-order failure and total-mode recovery contracts.
/// - fails: a named [`LowerError`] for scope violations or reserved residents.
/// - panics: none.
pub(super) fn validate_item(item: SynNode<'_>) -> LowerResult<()>
{
    match item.kind() {
        | node_kinds::DEF_REC => validate_scope(&[item]),
        | node_kinds::REC_BLOCK => {
            let members = item.children_by_field_name(node_kinds::FIELD_MEMBER);
            validate_scope(&members)
        },
        | _ => validate_nodes([item], &BTreeSet::new()),
    }
}

/// Arena handle for one persistent lexical-shadow chain.
///
/// # Contract
/// - invariant: `None` is the empty chain; `Some(index)` names an existing
///   [`Shadow`] entry while validation is in progress.
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
struct ShadowId(Option<usize>);

/// One recursive-name shadow and the preceding visible shadow chain.
///
/// # Contract
/// - invariant: `parent` predates this entry in the validation arena.
struct Shadow
{
    /// Recursive name hidden by the lexical binder.
    name: String,
    /// Shadow chain visible before this binder.
    parent: ShadowId,
}

/// Pending syntax traversal paired with its visible lexical shadows.
///
/// # Contract
/// - invariant: `shadows` belongs to the arena used to validate `node`.
#[derive(Clone, Copy)]
struct Work<'tree>
{
    /// Syntax node awaiting validation.
    node: SynNode<'tree>,
    /// Lexical-shadow chain visible at `node`.
    shadows: ShadowId,
}

/// Validate one standalone or grouped recursive scope.
fn validate_scope(definitions: &[SynNode<'_>]) -> LowerResult<()>
{
    let names: BTreeSet<String> = definitions
        .iter()
        .filter_map(|definition| definition.child_by_field_name(node_kinds::FIELD_NAME))
        .map(|name| name.text().as_ref().to_owned())
        .collect();

    for &definition in definitions {
        let mut shadow_arena = Vec::new();
        let shadows = bind_parameters(definition, &names, ShadowId::default(), &mut shadow_arena);
        if definition.kind() == node_kinds::DEF_REC
            && bool::from(definition.def_rec_has_copattern_body())
        {
            for clause in definition.children_by_field_name(node_kinds::FIELD_CLAUSE) {
                if let Some(body) = clause.child_by_field_name(node_kinds::FIELD_BODY) {
                    validate_nodes_from([body], &names, shadows, &mut shadow_arena)?;
                }
            }
        }
        else if let Some(body) = definition
            .child_by_field_name(node_kinds::FIELD_BODY)
            .or_else(|| definition.child_by_field_name(node_kinds::FIELD_VALUE))
        {
            validate_nodes_from([body], &names, shadows, &mut shadow_arena)?;
        }
    }
    Ok(())
}

/// Validate a forest without using the native call stack.
fn validate_nodes<'tree>(
    roots: impl IntoIterator<Item = SynNode<'tree>>,
    scope: &BTreeSet<String>,
) -> LowerResult<()>
{
    let mut shadow_arena = Vec::new();
    validate_nodes_from(roots, scope, ShadowId::default(), &mut shadow_arena)
}

/// Validate a forest under one persistent lexical-shadow chain.
fn validate_nodes_from<'tree>(
    roots: impl IntoIterator<Item = SynNode<'tree>>,
    scope: &BTreeSet<String>,
    initial_shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
) -> LowerResult<()>
{
    let mut stack: Vec<Work<'tree>> = roots
        .into_iter()
        .map(|node| Work {
            node,
            shadows: initial_shadows,
        })
        .collect();
    stack.reverse();
    while let Some(work) = stack.pop() {
        let node = work.node;
        match node.kind() {
            | node_kinds::IDENTIFIER
                if scope.contains(node.text().as_ref())
                    && !bool::from(is_shadowed(
                        work.shadows,
                        NodeText(node.text().as_ref()),
                        shadow_arena,
                    )) =>
            {
                return Err(unmarked(node));
            },
            | node_kinds::MODULE_DECLARATION => {
                push_nodes(
                    node.children_by_field_name(node_kinds::FIELD_MEMBER),
                    work.shadows,
                    &mut stack,
                );
            },
            | node_kinds::BLOCK => {
                push_block(node, scope, work.shadows, shadow_arena, &mut stack);
            },
            | node_kinds::LAMBDA_EXPRESSION | node_kinds::DEF_FUNCTION => {
                push_parameterized_body(node, scope, work.shadows, shadow_arena, &mut stack);
            },
            | node_kinds::ARM => {
                push_arm(node, scope, work.shadows, shadow_arena, &mut stack);
            },
            | node_kinds::LET_STATEMENT
            | node_kinds::DEF_VALUE
            | node_kinds::CO_FIELD
            | node_kinds::RECORD_FIELD
            | node_kinds::PROJECTION_EXPRESSION => {
                if let Some(value) = node.child_by_field_name(node_kinds::FIELD_VALUE) {
                    stack.push(Work {
                        node: value,
                        shadows: work.shadows,
                    });
                }
            },
            | node_kinds::BIND_STATEMENT => {
                if let Some(source) = node.child_by_field_name(node_kinds::FIELD_SOURCE) {
                    stack.push(Work {
                        node: source,
                        shadows: work.shadows,
                    });
                }
            },
            | node_kinds::COPATTERN_CLAUSE => {
                if let Some(body) = node.child_by_field_name(node_kinds::FIELD_BODY) {
                    stack.push(Work {
                        node: body,
                        shadows: work.shadows,
                    });
                }
            },
            | node_kinds::CALL_EXPRESSION => {
                validate_call(node, scope, work.shadows, shadow_arena, &mut stack)?;
            },
            | node_kinds::INSTANTIATION_EXPRESSION => {
                validate_instantiation(node, scope, work.shadows, shadow_arena, &mut stack)?;
            },
            | node_kinds::PARAMETER
            | node_kinds::DEF_SIGNATURE
            | node_kinds::DATA_DECLARATION
            | node_kinds::CODATA_DECLARATION
            | node_kinds::EXTERN_BLOCK
            | node_kinds::EXTERN_TYPE
            | node_kinds::EXTERN_FUNCTION => {},
            | _ => push_children(node, work.shadows, &mut stack),
        }
    }
    Ok(())
}

/// Validate a call head before traversing its ordinary arguments.
fn validate_call<'tree>(
    call: SynNode<'tree>,
    scope: &BTreeSet<String>,
    shadows: ShadowId,
    shadow_arena: &[Shadow],
    stack: &mut Vec<Work<'tree>>,
) -> LowerResult<()>
{
    let arguments = call.children_by_field_name(node_kinds::FIELD_ARGUMENT);
    push_nodes(arguments, shadows, stack);
    let Some(function) = call.child_by_field_name(node_kinds::FIELD_FUNCTION)
    else {
        return Ok(());
    };
    if function.kind() == node_kinds::INSTANTIATION_EXPRESSION {
        validate_instantiation(function, scope, shadows, shadow_arena, stack)
    }
    else if function.kind() == node_kinds::IDENTIFIER
        && scope.contains(function.text().as_ref())
        && !bool::from(is_shadowed(
            shadows,
            NodeText(function.text().as_ref()),
            shadow_arena,
        ))
    {
        Err(unmarked(function))
    }
    else if function.kind() == node_kinds::PROJECTION_EXPRESSION {
        if let Some(value) = function.child_by_field_name(node_kinds::FIELD_VALUE) {
            stack.push(Work {
                node: value,
                shadows,
            });
        }
        Ok(())
    }
    else {
        stack.push(Work {
            node: function,
            shadows,
        });
        Ok(())
    }
}

/// Classify and scope-check one bracketed instantiation slot.
fn validate_instantiation<'tree>(
    instantiation: SynNode<'tree>,
    scope: &BTreeSet<String>,
    shadows: ShadowId,
    shadow_arena: &[Shadow],
    stack: &mut Vec<Work<'tree>>,
) -> LowerResult<()>
{
    let mut has_direction = false;
    for resident in instantiation.children_by_field_name(node_kinds::FIELD_INSTANTIATION) {
        let text = resident.text();
        let resident_text = text.as_ref().trim();
        match resident_text {
            | "<" | ">" => has_direction = true,
            | "tail" => {
                return Err(LowerError::ReservedTailAssertion {
                    byte_range: resident.byte_range(),
                });
            },
            | _ if resident_text.contains('=') => {
                let error = match resident_text
                    .split_once('=')
                    .map(|(name, _value)| name.trim())
                {
                    | Some("size") => LowerError::ReservedExplicitSize {
                        resident: resident_text.to_owned(),
                        byte_range: resident.byte_range(),
                    },
                    | Some("cost") => LowerError::ReservedCostBound {
                        resident: resident_text.to_owned(),
                        byte_range: resident.byte_range(),
                    },
                    | _ => LowerError::ReservedExplicitInstantiation {
                        resident: resident_text.to_owned(),
                        byte_range: resident.byte_range(),
                    },
                };
                return Err(error);
            },
            | _ if resident_text.ends_with('<') => {
                return Err(LowerError::ReservedNamedMeasure {
                    resident: resident_text.to_owned(),
                    byte_range: resident.byte_range(),
                });
            },
            | _ => {},
        }
    }

    let Some(target) = instantiation.child_by_field_name(node_kinds::FIELD_TARGET)
    else {
        return Ok(());
    };
    let target_text = target.text();
    let target_name = target_text.as_ref().trim();
    let target_is_bound = target.kind() == node_kinds::IDENTIFIER && scope.contains(target_name);
    if has_direction && !target_is_bound {
        return Err(LowerError::MarkedReferenceOutsideRecursiveScope {
            target: target_name.to_owned(),
            byte_range: instantiation.byte_range(),
        });
    }
    if !has_direction
        && target_is_bound
        && !bool::from(is_shadowed(shadows, NodeText(target_name), shadow_arena))
    {
        return Err(unmarked(target));
    }
    if target.kind() != node_kinds::IDENTIFIER {
        stack.push(Work {
            node: target,
            shadows,
        });
    }
    Ok(())
}

/// Push one block's expressions with the lexical shadows visible at each step.
fn push_block<'tree>(
    block: SynNode<'tree>,
    scope: &BTreeSet<String>,
    shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
    stack: &mut Vec<Work<'tree>>,
)
{
    let mut visible = shadows;
    let mut pending = Vec::new();
    for child in block.named_children() {
        match child.kind() {
            | node_kinds::LET_STATEMENT => {
                if let Some(value) = child.child_by_field_name(node_kinds::FIELD_VALUE) {
                    pending.push(Work {
                        node: value,
                        shadows: visible,
                    });
                }
                if let Some(pattern) = child.child_by_field_name(node_kinds::FIELD_PATTERN) {
                    visible = bind_pattern(pattern, scope, visible, shadow_arena);
                }
            },
            | node_kinds::BIND_STATEMENT => {
                if let Some(source) = child.child_by_field_name(node_kinds::FIELD_SOURCE) {
                    pending.push(Work {
                        node: source,
                        shadows: visible,
                    });
                }
                if let Some(pattern) = child.child_by_field_name(node_kinds::FIELD_PATTERN) {
                    visible = bind_pattern(pattern, scope, visible, shadow_arena);
                }
            },
            | _ => pending.push(Work {
                node: child,
                shadows: visible,
            }),
        }
    }
    for work in pending.into_iter().rev() {
        stack.push(work);
    }
}

/// Push a lambda or function body under shadows introduced by its parameters.
fn push_parameterized_body<'tree>(
    node: SynNode<'tree>,
    scope: &BTreeSet<String>,
    shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
    stack: &mut Vec<Work<'tree>>,
)
{
    let body_shadows = bind_parameters(node, scope, shadows, shadow_arena);
    if let Some(body) = node.child_by_field_name(node_kinds::FIELD_BODY) {
        stack.push(Work {
            node: body,
            shadows: body_shadows,
        });
    }
}

/// Push a case arm body under shadows introduced by its pattern.
fn push_arm<'tree>(
    arm: SynNode<'tree>,
    scope: &BTreeSet<String>,
    shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
    stack: &mut Vec<Work<'tree>>,
)
{
    let body_shadows = arm
        .child_by_field_name(node_kinds::FIELD_PATTERN)
        .map_or(shadows, |pattern| {
            bind_pattern(pattern, scope, shadows, shadow_arena)
        });
    if let Some(body) = arm.child_by_field_name(node_kinds::FIELD_BODY) {
        stack.push(Work {
            node: body,
            shadows: body_shadows,
        });
    }
}

/// Extend a shadow chain with recursive names bound by parameters.
fn bind_parameters(
    node: SynNode<'_>,
    scope: &BTreeSet<String>,
    mut shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
) -> ShadowId
{
    let Some(parameters) = node.child_by_field_name(node_kinds::FIELD_PARAMETERS)
    else {
        return shadows;
    };
    for parameter in parameters.named_children() {
        if let Some(name) = parameter.child_by_field_name(node_kinds::FIELD_NAME) {
            shadows = bind_name(name, scope, shadows, shadow_arena);
        }
    }
    shadows
}

/// Extend a shadow chain with every recursive name bound by a pattern.
fn bind_pattern(
    pattern: SynNode<'_>,
    scope: &BTreeSet<String>,
    mut shadows: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
) -> ShadowId
{
    let mut stack = alloc::vec![pattern];
    while let Some(node) = stack.pop() {
        if node.kind() == node_kinds::IDENTIFIER {
            shadows = bind_name(node, scope, shadows, shadow_arena);
        }
        else {
            for child in node.named_children().into_iter().rev() {
                stack.push(child);
            }
        }
    }
    shadows
}

/// Extend a shadow chain when `name` hides a recursive binding.
fn bind_name(
    name: SynNode<'_>,
    scope: &BTreeSet<String>,
    parent: ShadowId,
    shadow_arena: &mut Vec<Shadow>,
) -> ShadowId
{
    let text = name.text().as_ref().to_owned();
    if !scope.contains(&text) {
        return parent;
    }
    let id = shadow_arena.len();
    shadow_arena.push(Shadow { name: text, parent });
    ShadowId(Some(id))
}

/// Whether `name` occurs in one persistent lexical-shadow chain.
fn is_shadowed(
    mut shadows: ShadowId,
    name: NodeText<'_>,
    shadow_arena: &[Shadow],
) -> ShadowPresence
{
    while let Some(index) = shadows.0 {
        let Some(shadow) = shadow_arena.get(index)
        else {
            return false.into();
        };
        if shadow.name == name.as_ref() {
            return true.into();
        }
        shadows = shadow.parent;
    }
    false.into()
}

/// Push named children in source order onto a LIFO work stack.
fn push_children<'tree>(
    node: SynNode<'tree>,
    shadows: ShadowId,
    stack: &mut Vec<Work<'tree>>,
)
{
    push_nodes(node.named_children(), shadows, stack);
}

/// Push nodes in source order onto a LIFO work stack.
///
/// # Contract
/// - ensures: popping `stack` visits `nodes` from first to last.
/// - panics: none.
fn push_nodes<'tree, Nodes>(
    nodes: Nodes,
    shadows: ShadowId,
    stack: &mut Vec<Work<'tree>>,
) where
    Nodes: IntoIterator<Item = SynNode<'tree>>,
    Nodes::IntoIter: DoubleEndedIterator,
{
    for node in nodes.into_iter().rev() {
        stack.push(Work { node, shadows });
    }
}

/// Build the hard error for one missing call-site evidence marker.
fn unmarked(node: SynNode<'_>) -> LowerError
{
    let name = node.text().as_ref().to_owned();
    LowerError::UnmarkedRecursiveReference {
        suggestion: format!("{name}[<](…)"),
        name,
        byte_range: node.byte_range(),
    }
}

#[cfg(test)]
mod tests
{
    use super::validate;
    use crate::boundary::PipelineSource;
    use crate::lower::LowerError;
    use crate::lower::lower_source;
    use crate::synnode::SynTree;

    #[test]
    fn marked_self_reference_resolves() -> Result<(), String>
    {
        validate_source(("def rec f(n: Integer) -> F Integer { ret f[<](n) }").into())
    }

    #[test]
    fn unmarked_self_reference_has_a_marked_suggestion() -> Result<(), String>
    {
        let error = validation_error(("def rec f(n: Integer) -> F Integer { ret f(n) }").into())?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference {
                ref name,
                ref suggestion,
                ..
            } if name == "f" && suggestion == "f[<](…)"
        ));
        Ok(())
    }

    #[test]
    fn bare_self_reference_has_a_marked_suggestion() -> Result<(), String>
    {
        let error = validation_error(("def rec f(n: Integer) -> F Integer { ret f }").into())?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference {
                ref name,
                ref suggestion,
                ..
            } if name == "f" && suggestion == "f[<](…)"
        ));
        Ok(())
    }

    #[test]
    fn qualified_outer_reference_is_not_captured() -> Result<(), String>
    {
        validate_source(("def rec f(n: Integer) -> F Integer { ret (outer.f)(n) }").into())
    }

    #[test]
    fn marked_reference_outside_the_scope_is_rejected() -> Result<(), String>
    {
        let error = validation_error(("def f(n: Integer) -> F Integer { ret f[<](n) }").into())?;
        assert!(matches!(
            error,
            LowerError::MarkedReferenceOutsideRecursiveScope { ref target, .. }
                if target == "f"
        ));
        Ok(())
    }

    #[test]
    fn mutual_scope_resolves_marked_peers() -> Result<(), String>
    {
        validate_source(
            (concat!(
                "rec { ",
                "def even(n: Integer) -> F Integer { ret odd[<](n) } ",
                "def odd(n: Integer) -> F Integer { ret even[<](n) }",
                " }",
            ))
            .into(),
        )
    }
    #[test]
    fn value_members_require_marked_peer_references() -> Result<(), String>
    {
        validate_source(("rec { def f = g[<]; def g = f[<]; }").into())?;
        let error = validation_error(("rec { def f = g; def g = f; }").into())?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference {
                ref name,
                ref suggestion,
                ..
            } if name == "g" && suggestion == "g[<](…)"
        ));
        Ok(())
    }

    #[test]
    fn unmarked_mutual_peer_has_a_marked_suggestion() -> Result<(), String>
    {
        let error = validation_error(
            (concat!(
                "rec { ",
                "def even(n: Integer) -> F Integer { ret odd(n) } ",
                "def odd(n: Integer) -> F Integer { ret even[<](n) }",
                " }",
            ))
            .into(),
        )?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference {
                ref name,
                ref suggestion,
                ..
            } if name == "odd" && suggestion == "odd[<](…)"
        ));
        Ok(())
    }

    #[test]
    fn composite_instantiation_target_is_still_scope_checked() -> Result<(), String>
    {
        let error = validation_error(
            ("def rec f(n: Integer) -> F Integer { ret (f)[Integer](n) }").into(),
        )?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference { ref name, .. } if name == "f"
        ));
        Ok(())
    }

    #[test]
    fn lexical_binders_shadow_recursive_names() -> Result<(), String>
    {
        let cases = [
            "def rec f(f: Integer) -> F Integer { ret f }",
            "def rec f() -> F Integer { val f = 1; ret f }",
            "def rec f() -> F Integer { ret fn(f: Integer) { ret f } }",
            "def rec f(n: Integer) -> F Integer { case n { f => ret f } }",
        ];
        for source in cases {
            validate_source((source).into())?;
        }
        Ok(())
    }

    #[test]
    fn explicit_marker_selects_fix_binding_through_a_shadow() -> Result<(), String>
    {
        validate_source(("def rec f(f: Integer) -> F Integer { ret f[<](f) }").into())
    }

    #[test]
    fn reserved_residents_have_named_declines() -> Result<(), String>
    {
        let measure =
            validation_error(("def rec f(n: Integer) -> F Integer { ret f[n <](n) }").into())?;
        assert!(matches!(measure, LowerError::ReservedNamedMeasure { .. }));

        let explicit =
            validation_error(("def rec f(n: Integer) -> F Integer { ret f[n = 1](n) }").into())?;
        assert!(matches!(
            explicit,
            LowerError::ReservedExplicitInstantiation { .. }
        ));

        let size =
            validation_error(("def rec f(n: Integer) -> F Integer { ret f[size = 1](n) }").into())?;
        assert!(matches!(size, LowerError::ReservedExplicitSize { .. }));

        let cost =
            validation_error(("def rec f(n: Integer) -> F Integer { ret f[cost = 1](n) }").into())?;
        assert!(matches!(cost, LowerError::ReservedCostBound { .. }));

        let tail =
            validation_error(("def rec f(n: Integer) -> F Integer { ret f[tail](n) }").into())?;
        assert!(matches!(tail, LowerError::ReservedTailAssertion { .. }));
        Ok(())
    }

    #[test]
    fn strict_lowering_runs_the_recursion_resolver() -> Result<(), String>
    {
        let source = "def rec f(n: Integer) -> F Integer { ret f(n) }";
        let error = lower_source(source.into())
            .err()
            .ok_or_else(|| "strict lowering must reject unmarked recursion".to_owned())?;
        assert!(matches!(
            error,
            LowerError::UnmarkedRecursiveReference { ref name, .. } if name == "f"
        ));
        Ok(())
    }

    fn validate_source(source: PipelineSource<'_>) -> Result<(), String>
    {
        let tree = SynTree::parse(source.0).map_err(|error| format!("parse failed: {error:?}"))?;
        if !tree.obligations().is_empty() {
            return Err(format!("parse obligations: {:?}", tree.obligations()));
        }
        validate(tree.root()).map_err(|error| format!("validation failed: {error:?}"))
    }

    fn validation_error(source: PipelineSource<'_>) -> Result<LowerError, String>
    {
        let tree = SynTree::parse(source.0).map_err(|error| format!("parse failed: {error:?}"))?;
        if !tree.obligations().is_empty() {
            return Err(format!("parse obligations: {:?}", tree.obligations()));
        }
        validate(tree.root())
            .err()
            .ok_or_else(|| "validation must fail".to_owned())
    }
}
