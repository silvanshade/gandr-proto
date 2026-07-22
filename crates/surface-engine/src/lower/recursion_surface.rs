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

use crate::lower::LowerError;
use crate::lower::LowerResult;
use crate::lower::node_kinds;
use crate::synnode::SynNode;

/// Validate every recursion marker and fix-bound reference in one parsed file.
///
/// # Contract
/// - requires: strict-mode syntax obligations have already been rejected.
/// - ensures: every direction-marked target belongs to its enclosing recursive
///   scope and no fix-bound term name is referenced without a marker.
/// - provides: the resolver/checker boundary for the first productivity rung.
/// - fails: a named [`LowerError`] for scope violations or reserved residents.
/// - panics: none.
pub(super) fn validate(root: SynNode<'_>) -> LowerResult<()>
{
    for item in root.named_children() {
        match item.kind() {
            | node_kinds::DEF_REC => validate_scope(&[item])?,
            | node_kinds::REC_BLOCK => {
                let members = item.children_by_field_name(node_kinds::FIELD_MEMBER);
                validate_scope(&members)?;
            },
            | _ => validate_nodes([item], &BTreeSet::new())?,
        }
    }
    Ok(())
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
        if definition.kind() == node_kinds::DEF_REC
            && bool::from(definition.def_rec_has_copattern_body())
        {
            for clause in definition.children_by_field_name(node_kinds::FIELD_CLAUSE) {
                if let Some(body) = clause.child_by_field_name(node_kinds::FIELD_BODY) {
                    validate_nodes([body], &names)?;
                }
            }
        }
        else if let Some(body) = definition.child_by_field_name(node_kinds::FIELD_BODY) {
            validate_nodes([body], &names)?;
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
    let mut stack: Vec<SynNode<'tree>> = roots.into_iter().collect();
    stack.reverse();
    while let Some(node) = stack.pop() {
        match node.kind() {
            | node_kinds::CALL_EXPRESSION => validate_call(node, scope, &mut stack)?,
            | node_kinds::INSTANTIATION_EXPRESSION => {
                validate_instantiation(node, scope)?;
            },
            | node_kinds::PROJECTION_EXPRESSION => {
                if let Some(value) = node.child_by_field_name(node_kinds::FIELD_VALUE) {
                    stack.push(value);
                }
            },
            | _ => push_children(node, &mut stack),
        }
    }
    Ok(())
}

/// Validate a call head before traversing its ordinary arguments.
fn validate_call<'tree>(
    call: SynNode<'tree>,
    scope: &BTreeSet<String>,
    stack: &mut Vec<SynNode<'tree>>,
) -> LowerResult<()>
{
    let arguments = call.children_by_field_name(node_kinds::FIELD_ARGUMENT);
    for argument in arguments.into_iter().rev() {
        stack.push(argument);
    }
    let Some(function) = call.child_by_field_name(node_kinds::FIELD_FUNCTION)
    else {
        return Ok(());
    };
    if function.kind() == node_kinds::INSTANTIATION_EXPRESSION {
        validate_instantiation(function, scope)
    }
    else if function.kind() == node_kinds::IDENTIFIER && scope.contains(function.text().as_ref())
    {
        Err(unmarked(function))
    }
    else if function.kind() == node_kinds::PROJECTION_EXPRESSION {
        if let Some(value) = function.child_by_field_name(node_kinds::FIELD_VALUE) {
            stack.push(value);
        }
        Ok(())
    }
    else {
        stack.push(function);
        Ok(())
    }
}

/// Classify and scope-check one bracketed instantiation slot.
fn validate_instantiation(
    instantiation: SynNode<'_>,
    scope: &BTreeSet<String>,
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
                return Err(LowerError::ReservedExplicitInstantiation {
                    resident: resident_text.to_owned(),
                    byte_range: resident.byte_range(),
                });
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
    if !has_direction && target_is_bound {
        return Err(unmarked(target));
    }
    Ok(())
}

/// Push named children in source order onto a LIFO work stack.
fn push_children<'tree>(
    node: SynNode<'tree>,
    stack: &mut Vec<SynNode<'tree>>,
)
{
    for child in node.named_children().into_iter().rev() {
        stack.push(child);
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
    use crate::lower::LowerError;
    use crate::lower::lower_source;
    use crate::synnode::SynTree;

    #[test]
    fn marked_self_reference_resolves() -> Result<(), String>
    {
        validate_source("def rec f(n: Integer) -> F Integer { ret f[<](n) }")
    }

    #[test]
    fn unmarked_self_reference_has_a_marked_suggestion() -> Result<(), String>
    {
        let error = validation_error("def rec f(n: Integer) -> F Integer { ret f(n) }")?;
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
        validate_source("def rec f(n: Integer) -> F Integer { ret outer.f(n) }")
    }

    #[test]
    fn marked_reference_outside_the_scope_is_rejected() -> Result<(), String>
    {
        let error = validation_error("def f(n: Integer) -> F Integer { ret f[<](n) }")?;
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
        validate_source(concat!(
            "rec { ",
            "def even(n: Integer) -> F Integer { ret odd[<](n) } ",
            "def odd(n: Integer) -> F Integer { ret even[<](n) }",
            " }",
        ))
    }

    #[test]
    fn reserved_residents_have_named_declines() -> Result<(), String>
    {
        let measure = validation_error("def rec f(n: Integer) -> F Integer { ret f[n <](n) }")?;
        assert!(matches!(measure, LowerError::ReservedNamedMeasure { .. }));

        let explicit = validation_error("def rec f(n: Integer) -> F Integer { ret f[n = 1](n) }")?;
        assert!(matches!(
            explicit,
            LowerError::ReservedExplicitInstantiation { .. }
        ));

        let tail = validation_error("def rec f(n: Integer) -> F Integer { ret f[tail](n) }")?;
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

    fn validate_source(source: &str) -> Result<(), String>
    {
        let tree = SynTree::parse(source).map_err(|error| format!("parse failed: {error:?}"))?;
        validate(tree.root()).map_err(|error| format!("validation failed: {error:?}"))
    }

    fn validation_error(source: &str) -> Result<LowerError, String>
    {
        let tree = SynTree::parse(source).map_err(|error| format!("parse failed: {error:?}"))?;
        validate(tree.root())
            .err()
            .ok_or_else(|| "validation must fail".to_owned())
    }
}
