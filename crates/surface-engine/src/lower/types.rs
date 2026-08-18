//! CST type nodes → core [`ValueType`]/[`CompType`] — the covered fragment's
//! "Types" row, in both strictness modes.
//!
//! Covered: atoms (`primitive_type`, `type_identifier`), the `?` unknown
//! atom, `F`, `U[r]`, `->`, `*`, `+`, `&`, parenthesized. Everything else
//! (unions, intersections, `forall`, `at`, sessions, type application, type
//! variables) is [`LowerError::Unsupported`]. Sorts are decided structurally;
//! a type of the wrong polarity for its position is
//! [`LowerError::TypeSortMismatch`]. The one exception is `?` (gandr-89k):
//! the unknown type has one spelling on both sorts and the consuming position
//! decides — a value position lowers it to `ValueType::Unknown`, a
//! computation position to `CompType::Unknown`, so `F ?` is the pure
//! returner over the value unknown while a bare `?` in a computation
//! position is the computation top.
//!
//! In [`Strictness::Total`] mode every failure lowers to the position's
//! sort of `Unknown` instead — types cannot be holes, and the `Unknown` is
//! itself the visible signal (the lower-module doc records this decision).
//! [SPECULATIVE DECISION] In the one sort-*free* position (`def name : T;`
//! signatures via [`lower_ty`]) the fallback is `Ty::Value(Unknown)`:
//! every Stage-1 surface item that can carry a signature is value-shaped
//! (`def` values and `def`-function thunks).

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use gandr_core_term::boundary::GradeBound;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::DataId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::boundary::PipelineSource;
use crate::boundary::SignificantIndex;
use crate::boundary::TypeName;
use crate::boundary::UnknownAtomFlag;
use crate::lower::LowerError;
use crate::lower::LowerResult;
use crate::lower::Strictness;
use crate::lower::node_kinds;
use crate::lower::node_text;
use crate::lower::required_field;
use crate::synnode::SynNode;

/// Sort description used by [`LowerError::TypeSortMismatch`] when a value
/// type was required.
const SORT_VALUE_TYPE: &str = "a value type";
/// Sort description used by [`LowerError::TypeSortMismatch`] when a
/// computation type was required.
const SORT_COMP_TYPE: &str = "a computation type";

/// Resolves a surface type-head name to its declared-datatype [`DataId`], or
/// `None` for a non-declared name (a primitive, a type variable, a built-in
/// former). Threaded through the whole type lowering so a declared datatype
/// rewrites to the [`ValueType::Data`] nominal handle at **any** depth (the
/// design record) — not only at a top-level ascription: `List(Maybe(a))`,
/// `Maybe(a) -> X`, and a record field `#{ m : Maybe(a) }` all intercept. The
/// pipeline seam (`Lowerer`) supplies a closure over its declared-data
/// registry; a caller with no registry passes a `|_| None` resolver.
pub type DataResolver<'resolver> = &'resolver dyn Fn(&str) -> Option<DataId>;

/// Lowers a CST type node, requiring the value sort. Total mode resolves
/// every failure — including a sort mismatch — to [`ValueType::Unknown`].
///
/// # Contract
/// - requires: `node` is a type-position CST node from `source`.
/// - ensures: returns the value type the node denotes; total mode resolves
///   every failure — including a sort mismatch — to `ValueType::Unknown`.
/// - fails: in strict mode, `LowerError::TypeSortMismatch` when the node lowers
///   to a computation type, plus the `lower_ty` failures; total mode never
///   errs.
/// - panics: none.
///
/// # Errors
///
/// In strict mode: [`LowerError::TypeSortMismatch`] when the node lowers to
/// a computation type, plus the [`lower_ty`] errors. Total mode never errs.
pub fn lower_value_ty(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
    strictness: Strictness,
    resolve: DataResolver<'_>,
) -> LowerResult<ValueType>
{
    value_result(
        lower_type_tree(source, node, strictness, resolve, &no_manifest_types),
        node,
        strictness,
    )
}

/// Resolves a module signature's **manifest type component** to the type it was
/// declared equal to.
///
/// A signature's `type T = τ` elaborates `τ` once, where the signature is
/// spelled, and every later occurrence of `T` in that signature expands to the
/// already-elaborated `τ`. Consulting this before the declared-data resolver is
/// what makes the expansion **manifest rather than ambient**: an enclosing
/// datatype also called `T` cannot capture the component, because the name
/// never reaches the ambient resolver at all.
pub type ManifestTypes<'manifest> = &'manifest dyn Fn(TypeName<'_>) -> Option<ValueType>;

/// The manifest environment everywhere outside a module signature: no
/// component is in scope, so every type name resolves ambiently as it always
/// did.
#[inline]
#[must_use]
pub fn no_manifest_types(_name: TypeName<'_>) -> Option<ValueType>
{
    None
}

/// Lowers a primitive type name.
///
/// [SPECULATIVE DECISION] The design maps atoms/`primitive_type` to core
/// atoms; two primitives get structural meanings so the surface is coherent
/// with the literal story: `Unit` is core `1`, and `Boolean` is `1 + 1` —
/// the type of `true`/`false`, which lower to annotated injections into
/// `1 + 1` ("Booleans need no core change"). A third
/// keyword, `Unknown`, is the gradual top [`ValueType::Unknown`] — the
/// consistency hole an ascription names explicitly — NOT the rigid
/// `atom("Unknown")` the opaque fallback would otherwise give. All other
/// primitives stay opaque atoms (in particular `Integer`, matching
/// `Value::Int`).
fn lower_primitive(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
) -> LowerResult<ValueType>
{
    let name = node_text(source, node)?;
    match name.0 {
        | node_kinds::NAME_UNIT_TYPE => Ok(ValueType::Unit),
        | node_kinds::NAME_BOOLEAN_TYPE => Ok(ValueType::sum(ValueType::Unit, ValueType::Unit)),
        | node_kinds::NAME_UNKNOWN_TYPE => Ok(ValueType::Unknown),
        | other => Ok(ValueType::atom(other)),
    }
}

/// Lowers a CST type node, requiring the computation sort. Total mode
/// resolves every failure — including a sort mismatch — to
/// [`CompType::Unknown`].
///
/// # Contract
/// - requires: `node` is a type-position CST node from `source`.
/// - ensures: returns the computation type the node denotes; total mode
///   resolves every failure — including a sort mismatch — to
///   `CompType::Unknown`.
/// - fails: in strict mode, `LowerError::TypeSortMismatch` when the node lowers
///   to a value type, plus the `lower_ty` failures; total mode never errs.
/// - panics: none.
///
/// # Errors
///
/// In strict mode: [`LowerError::TypeSortMismatch`] when the node lowers to
/// a value type, plus the [`lower_ty`] errors. Total mode never errs.
pub fn lower_comp_ty(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
    strictness: Strictness,
    resolve: DataResolver<'_>,
) -> LowerResult<CompType>
{
    comp_result(
        lower_type_tree(source, node, strictness, resolve, &no_manifest_types),
        node,
        strictness,
    )
}

/// Parses the optional `grade` field of a `u_type` / `thunk_expression`
/// node; absent means the default `ω` (plan: "grade default ω").
///
/// # Contract
/// - requires: `node` is a `u_type` or `thunk_expression` CST node from
///   `source` (the carrier of the optional `grade` field).
/// - ensures: returns the `grade` field's `u64` numeral or `ω`, or the default
///   `Grade::OMEGA` when the field is absent.
/// - fails: `LowerError::InvalidGrade` for non-`u64` numerals and for grade
///   variables (Stage 2); `LowerError::MalformedNode` for a structurally
///   damaged grade node. This errs in both modes — the consuming position holes
///   the result in total mode.
/// - panics: none.
///
/// # Errors
///
/// [`LowerError::InvalidGrade`] for non-`u64` numerals and for grade
/// *variables* (identifiers), which are Stage 2.
pub fn parse_grade(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
) -> LowerResult<Grade>
{
    let Some(grade_node) = node.child_by_field_name(node_kinds::FIELD_GRADE)
    else {
        return Ok(Grade::OMEGA);
    };
    let Some(inner) = grade_node.child(SignificantIndex(0))
    else {
        return Err(LowerError::MalformedNode {
            kind: grade_node.kind(),
            byte_range: grade_node.byte_range(),
        });
    };
    let text = node_text(source, grade_node)?;
    match inner.kind() {
        | node_kinds::OMEGA => Ok(Grade::OMEGA),
        | node_kinds::NUMBER => match text.parse::<u64>() {
            | Ok(bound) => Ok(Grade::fin(GradeBound::from(bound))),
            // Floats, exponents, and overflowing numerals are not grades.
            | Err(_parse_error) => Err(LowerError::InvalidGrade {
                text: text.to_owned(),
                byte_range: grade_node.byte_range(),
            }),
        },
        // Grade variables (identifiers) arrive with Stage 2.
        | _ => Err(LowerError::InvalidGrade {
            text: text.to_owned(),
            byte_range: grade_node.byte_range(),
        }),
    }
}

/// Right-nests an n-ary member list with a binary constructor, matching the
/// right-nesting of n-ary tuples (`A * B * C` ⇒ `A × (B × C)`).
///
/// # Errors
///
/// [`LowerError::MalformedNode`] on an empty member list (impossible on
/// grammar-conformant trees; kept total).
fn nest_right<T>(
    node: SynNode<'_>,
    members: Vec<T>,
    combine: fn(T, T) -> T,
) -> LowerResult<T>
{
    members
        .into_iter()
        .rev()
        .reduce(|rest, member| combine(member, rest))
        .ok_or_else(|| LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
}

/// Lowers a CST type node to a type of either sort.
///
/// # Contract
/// - requires: `node` is a type-position CST node from `source`.
/// - ensures: returns the value- or computation-sorted `Ty` the node denotes;
///   in total mode every failure resolves to the sort-free
///   `Ty::Value(ValueType::Unknown)` fallback.
/// - provides: the polarity-deciding type lowering shared by the value- and
///   computation-sorted entries.
/// - fails: in strict mode, a `LowerError` for out-of-fragment or malformed
///   types; total mode never errs.
/// - panics: none.
///
/// # Errors
///
/// Returns a [`LowerError`] for out-of-fragment or malformed types in
/// strict mode; total mode absorbs every failure into the sort-free
/// `Ty::Value(Unknown)` fallback (see the module doc) and never errs.
pub fn lower_ty(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
    strictness: Strictness,
    resolve: DataResolver<'_>,
) -> LowerResult<Ty>
{
    lower_ty_manifest(source, node, strictness, resolve, &no_manifest_types)
}

/// Lowers a type under a module signature's manifest type components.
///
/// Identical to [`lower_ty`] except that a bare type name matching a manifest
/// component expands to that component's already-elaborated type **before** the
/// declared-data resolver is consulted, so the expansion cannot be captured by
/// an ambient declaration of the same name.
///
/// # Contract
/// - ensures: a type name `manifest` answers becomes exactly the type it
///   answers with, wherever it occurs in `node`.
/// - ensures: every other name lowers exactly as [`lower_ty`] lowers it.
/// - fails: as [`lower_ty`].
/// - panics: none.
///
/// # Errors
///
/// As [`lower_ty`].
///
/// # Adequacy
/// - hypothesis: consulting the manifest first, at the one type-name site,
///   makes expansion independent of the ambient environment.
/// - mutants: consult the data resolver first; consult the manifest only at the
///   root.
/// - witnesses: `gandr-surface-engine` `tests/acceptance.rs` —
///   `a_manifest_type_component_is_not_captured_by_an_ambient_datatype`.
#[inline]
pub fn lower_ty_manifest(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
    strictness: Strictness,
    resolve: DataResolver<'_>,
    manifest: ManifestTypes<'_>,
) -> LowerResult<Ty>
{
    let result = lower_type_tree(source, node, strictness, resolve, manifest);
    if matches!(strictness, Strictness::Total) && result.is_err() {
        return Ok(Ty::Value(ValueType::Unknown));
    }
    result
}

/// One pending node or assembly frame in the iterative type lowerer.
enum TypeTask<'tree>
{
    /// Lower one CST type node.
    Node(SynNode<'tree>),
    /// Assemble a parent after its child types have lowered.
    Build(TypeFrame<'tree>),
}

/// Constructor metadata retained while child types are lowered.
enum TypeFrame<'tree>
{
    /// Assemble an `F` type from its lowered computation argument.
    F(SynNode<'tree>),
    /// Assemble a `U` type from its graded value argument.
    U
    {
        /// Value-argument CST node.
        argument: SynNode<'tree>,
        /// Grade annotation parsed at the `U`.
        grade: Grade,
    },
    /// Assemble a package type from its component labels and lowered payload.
    Package
    {
        /// The whole package-type node (for error spans).
        node: SynNode<'tree>,
        /// The payload CST node.
        argument: SynNode<'tree>,
        /// The abstract type component labels, in signature order.
        components: Vec<String>,
    },
    /// Assemble a function type from its lowered parameter and result.
    Function
    {
        /// Parameter CST node.
        parameter: SynNode<'tree>,
        /// Result CST node.
        result: SynNode<'tree>,
    },
    /// Assemble an n-ary member node from its lowered members.
    Members
    {
        /// Whole member-list CST node (for error spans).
        node: SynNode<'tree>,
        /// Member CST nodes in source order.
        members: Vec<SynNode<'tree>>,
        /// Product, sum, or with result constructor.
        sort: MemberSort,
    },
    /// Assemble a data-type application from its lowered arguments.
    Data
    {
        /// Resolved data-type identity.
        id: DataId,
        /// Argument CST nodes in source order.
        arguments: Vec<SynNode<'tree>>,
    },
    /// Assemble a list type from its lowered element.
    List(SynNode<'tree>),
    /// Assemble an equality path type from its carrier and endpoints.
    Path
    {
        /// Carrier-type CST node.
        carrier: SynNode<'tree>,
        /// Left endpoint value.
        lhs: Value,
        /// Right endpoint value.
        rhs: Value,
    },
    /// Assemble a record type from its lowered fields.
    Record
    {
        /// Whole record CST node (for error spans).
        node: SynNode<'tree>,
        /// `(label, grade, payload)` CST nodes per field in source order.
        fields: Vec<(String, SynNode<'tree>, SynNode<'tree>)>,
    },
}

/// Result sort and constructor for an n-ary type node.
enum MemberSort
{
    /// Assemble a product type.
    Product,
    /// Assemble a sum type.
    Sum,
    /// Assemble a with (coproduct) type.
    With,
}

/// Lowers a complete type tree with an explicit post-order worklist.
fn lower_type_tree(
    source: PipelineSource<'_>,
    root: SynNode<'_>,
    strictness: Strictness,
    resolve: DataResolver<'_>,
    manifest: ManifestTypes<'_>,
) -> LowerResult<Ty>
{
    let mut pending = vec![TypeTask::Node(root)];
    let mut results = Vec::new();
    while let Some(task) = pending.pop() {
        match task {
            | TypeTask::Node(node) => {
                if node.is_error().0 || node.is_missing().0 {
                    results.push(Err(LowerError::Syntax {
                        byte_range: node.byte_range(),
                    }));
                    continue;
                }
                match node.kind() {
                    | node_kinds::PRIMITIVE_TYPE => {
                        results.push(lower_primitive(source, node).map(Ty::Value));
                    },
                    // The `?` atom (gandr-89k) is the unknown type. The
                    // bottom-up walk cannot see the consuming position, so it
                    // lowers to the sort-free value default here; a
                    // computation-sorted consumer re-reads it as
                    // `CompType::Unknown` at the `comp_result` coercion.
                    | node_kinds::UNKNOWN_TYPE => {
                        results.push(Ok(Ty::Value(ValueType::Unknown)));
                    },
                    | node_kinds::TYPE_IDENTIFIER => {
                        results.push(node_text(source, node).map(|name| {
                            // The manifest environment answers first, so a
                            // signature's `type T = τ` expands to `τ` even where
                            // an ambient datatype of the same name exists.
                            Ty::Value(manifest(TypeName(name.0)).unwrap_or_else(|| {
                                resolve(name.0).map_or_else(
                                    || ValueType::atom(name.0),
                                    |id| ValueType::data(id, Vec::new()),
                                )
                            }))
                        }));
                    },
                    | node_kinds::F_TYPE => {
                        let Ok(argument) = required_field(node, node_kinds::FIELD_ARGUMENT)
                        else {
                            results.push(
                                required_field(node, node_kinds::FIELD_ARGUMENT)
                                    .map(|_| Ty::Value(ValueType::Unknown)),
                            );
                            continue;
                        };
                        pending.push(TypeTask::Build(TypeFrame::F(argument)));
                        pending.push(TypeTask::Node(argument));
                    },
                    | node_kinds::PACKAGE_TYPE => {
                        let components = match package_components(source, node) {
                            | Ok(components) => components,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        let argument = match required_field(node, node_kinds::FIELD_ARGUMENT) {
                            | Ok(argument) => argument,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        pending.push(TypeTask::Build(TypeFrame::Package {
                            node,
                            argument,
                            components,
                        }));
                        pending.push(TypeTask::Node(argument));
                    },
                    | node_kinds::U_TYPE => {
                        let grade = match parse_grade(source, node) {
                            | Ok(grade) => grade,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        let argument = match required_field(node, node_kinds::FIELD_ARGUMENT) {
                            | Ok(argument) => argument,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        pending.push(TypeTask::Build(TypeFrame::U { argument, grade }));
                        pending.push(TypeTask::Node(argument));
                    },
                    | node_kinds::FUNCTION_TYPE => {
                        let parameter = match required_field(node, node_kinds::FIELD_PARAMETER) {
                            | Ok(parameter) => parameter,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        let result = match required_field(node, node_kinds::FIELD_RESULT) {
                            | Ok(result) => result,
                            | Err(error) => {
                                results.push(Err(error));
                                continue;
                            },
                        };
                        pending.push(TypeTask::Build(TypeFrame::Function { parameter, result }));
                        pending.push(TypeTask::Node(result));
                        pending.push(TypeTask::Node(parameter));
                    },
                    | node_kinds::PRODUCT_TYPE
                    | node_kinds::SUM_TYPE
                    | node_kinds::LAZY_PRODUCT_TYPE => {
                        let members = member_nodes(node);
                        let sort = match node.kind() {
                            | node_kinds::PRODUCT_TYPE => MemberSort::Product,
                            | node_kinds::SUM_TYPE => MemberSort::Sum,
                            | _ => MemberSort::With,
                        };
                        pending.push(TypeTask::Build(TypeFrame::Members {
                            node,
                            members: members.clone(),
                            sort,
                        }));
                        pending.extend(members.into_iter().rev().map(TypeTask::Node));
                    },
                    | node_kinds::PARENTHESIZED_TYPE => {
                        match required_field(node, node_kinds::FIELD_TYPE) {
                            | Ok(inner) => pending.push(TypeTask::Node(inner)),
                            | Err(error) => results.push(Err(error)),
                        }
                    },
                    | node_kinds::TYPE_APPLICATION => {
                        schedule_type_application(
                            source,
                            node,
                            resolve,
                            &mut pending,
                            &mut results,
                        );
                    },
                    | node_kinds::RECORD_TYPE => {
                        schedule_record_type(source, node, &mut pending, &mut results);
                    },
                    | kind => results.push(Err(LowerError::Unsupported {
                        kind,
                        byte_range: node.byte_range(),
                    })),
                }
            },
            | TypeTask::Build(frame) => {
                assemble_type_frame(frame, strictness, &mut results);
            },
        }
    }
    results.pop().unwrap_or_else(|| {
        Err(LowerError::MalformedNode {
            kind: root.kind(),
            byte_range: root.byte_range(),
        })
    })
}

/// Schedules one type application or emits its immediate error.
fn schedule_type_application<'tree>(
    source: PipelineSource<'_>,
    node: SynNode<'tree>,
    resolve: DataResolver<'_>,
    pending: &mut Vec<TypeTask<'tree>>,
    results: &mut Vec<LowerResult<Ty>>,
)
{
    let constructor = match required_field(node, node_kinds::FIELD_CONSTRUCTOR) {
        | Ok(constructor) => constructor,
        | Err(error) => {
            results.push(Err(error));
            return;
        },
    };
    let arguments: Vec<SynNode<'tree>> = node.children_by_field_name(node_kinds::FIELD_ARGUMENT);
    let head = match node_text(source, constructor) {
        | Ok(head) => head,
        | Err(error) => {
            results.push(Err(error));
            return;
        },
    };
    if let Some(id) = resolve(head.0) {
        pending.push(TypeTask::Build(TypeFrame::Data {
            id,
            arguments: arguments.clone(),
        }));
        pending.extend(arguments.into_iter().rev().map(TypeTask::Node));
        return;
    }
    match head.0 {
        | node_kinds::NAME_LIST_TYPE => {
            let [element] = *arguments.as_slice()
            else {
                results.push(Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                }));
                return;
            };
            pending.push(TypeTask::Build(TypeFrame::List(element)));
            pending.push(TypeTask::Node(element));
        },
        | node_kinds::NAME_PATH_TYPE => {
            let [carrier, lhs, rhs] = *arguments.as_slice()
            else {
                results.push(Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                }));
                return;
            };
            let lhs = match lower_endpoint_value(source, lhs) {
                | Ok(lhs) => lhs,
                | Err(error) => {
                    results.push(Err(error));
                    return;
                },
            };
            let rhs = match lower_endpoint_value(source, rhs) {
                | Ok(rhs) => rhs,
                | Err(error) => {
                    results.push(Err(error));
                    return;
                },
            };
            pending.push(TypeTask::Build(TypeFrame::Path { carrier, lhs, rhs }));
            pending.push(TypeTask::Node(carrier));
        },
        | _ => results.push(Err(LowerError::Unsupported {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })),
    }
}

/// Schedules one record type or emits its immediate structural error.
fn schedule_record_type<'tree>(
    source: PipelineSource<'_>,
    node: SynNode<'tree>,
    pending: &mut Vec<TypeTask<'tree>>,
    results: &mut Vec<LowerResult<Ty>>,
)
{
    let mut fields = Vec::new();
    for field in node.named_children() {
        // A type component declares a name and belongs to a module signature,
        // where the module lowering reads it. Dropping it here would make
        // `#{ type T }` and `#{}` the same record type, so an ordinary type
        // position refuses it instead.
        if field.kind() == node_kinds::TYPE_COMPONENT {
            results.push(Err(LowerError::TypeSortMismatch {
                expected: "a record field; a `type` component is only meaningful in a module \
                           signature",
                kind: field.kind(),
                byte_range: field.byte_range(),
            }));
            return;
        }
        if field.kind() != node_kinds::RECORD_TYPE_FIELD {
            continue;
        }
        let name_node = match required_field(field, node_kinds::FIELD_NAME) {
            | Ok(name_node) => name_node,
            | Err(error) => {
                results.push(Err(error));
                return;
            },
        };
        let type_node = match required_field(field, node_kinds::FIELD_TYPE) {
            | Ok(type_node) => type_node,
            | Err(error) => {
                results.push(Err(error));
                return;
            },
        };
        let label = match node_text(source, name_node) {
            | Ok(label) => label.to_owned(),
            | Err(error) => {
                results.push(Err(error));
                return;
            },
        };
        fields.push((label, field, type_node));
    }
    pending.push(TypeTask::Build(TypeFrame::Record {
        node,
        fields: fields.clone(),
    }));
    pending.extend(
        fields
            .into_iter()
            .rev()
            .map(|(_, _, type_node)| TypeTask::Node(type_node)),
    );
}

/// Assembles one type constructor from its post-order child results.
fn assemble_type_frame(
    frame: TypeFrame<'_>,
    strictness: Strictness,
    results: &mut Vec<LowerResult<Ty>>,
)
{
    match frame {
        | TypeFrame::F(argument) => {
            let child = pop_type_result(results, argument);
            // gandr-89k: the returner former is honest — `F ?` (and the legacy
            // `F Unknown`) is the PURE returner over `ValueType::Unknown`, not
            // the computation-sort top. The computation top is the bare `?`
            // atom in a computation position (see `comp_result`).
            results.push(
                value_result(child, argument, strictness)
                    .map(|payload| Ty::Comp(CompType::returner(payload))),
            );
        },
        | TypeFrame::U { argument, grade } => {
            let child = pop_type_result(results, argument);
            results.push(
                comp_result(child, argument, strictness)
                    .map(|body| Ty::Value(ValueType::thunk(grade, body))),
            );
        },
        | TypeFrame::Package {
            node,
            argument,
            components,
        } => {
            let child = pop_type_result(results, argument);
            let assembled = value_result(child, argument, strictness).and_then(|payload| {
                // The package's grade IS the payload thunk's grade, so it is
                // read off rather than written twice. A payload of any other
                // shape is a malformed signature and is refused here, at
                // formation, rather than repaired into something checkable.
                match payload {
                    | ValueType::Thunk(grade, _) => Ok(Ty::Value(ValueType::Package {
                        grade,
                        abstracts: components,
                        payload: Rc::new(payload),
                    })),
                    | ValueType::Unknown if matches!(strictness, Strictness::Total) => {
                        Ok(Ty::Value(ValueType::Unknown))
                    },
                    | _ => Err(LowerError::PackagePayloadNotGradedThunk {
                        byte_range: node.byte_range(),
                    }),
                }
            });
            results.push(match assembled {
                | Ok(assembled) => Ok(assembled),
                | Err(_) if matches!(strictness, Strictness::Total) => {
                    Ok(Ty::Value(ValueType::Unknown))
                },
                | Err(error) => Err(error),
            });
        },
        | TypeFrame::Function { parameter, result } => {
            let result_ty = pop_type_result(results, result);
            let parameter_ty = pop_type_result(results, parameter);
            let assembled =
                value_result(parameter_ty, parameter, strictness).and_then(|argument| {
                    comp_result(result_ty, result, strictness)
                        .map(|result| Ty::Comp(CompType::arrow(argument, result)))
                });
            results.push(assembled);
        },
        | TypeFrame::Members {
            node,
            members,
            sort,
        } => {
            let child_results = pop_type_results(results, &members);
            let assembled = child_results.and_then(|child_results| match sort {
                | MemberSort::Product | MemberSort::Sum => {
                    let mut values = Vec::with_capacity(members.len());
                    for (child, member) in child_results.into_iter().zip(&members) {
                        let lowered_member = value_result(child, *member, strictness)?;
                        values.push(lowered_member);
                    }
                    let combine = match sort {
                        | MemberSort::Product => ValueType::prod,
                        | _ => ValueType::sum,
                    };
                    nest_right(node, values, combine).map(Ty::Value)
                },
                | MemberSort::With => {
                    let mut computations = Vec::with_capacity(members.len());
                    for (child, member) in child_results.into_iter().zip(&members) {
                        let lowered_member = comp_result(child, *member, strictness)?;
                        computations.push(lowered_member);
                    }
                    nest_right(node, computations, CompType::with).map(Ty::Comp)
                },
            });
            results.push(assembled);
        },
        | TypeFrame::Data { id, arguments } => {
            let child_results = pop_type_results(results, &arguments);
            let assembled = child_results.and_then(|child_results| {
                let mut lowered = Vec::with_capacity(arguments.len());
                for (child, argument) in child_results.into_iter().zip(&arguments) {
                    let lowered_argument = value_result(child, *argument, strictness)?;
                    lowered.push(lowered_argument);
                }
                Ok(Ty::Value(ValueType::data(id, lowered)))
            });
            results.push(assembled);
        },
        | TypeFrame::List(element) => {
            let child = pop_type_result(results, element);
            results.push(
                value_result(child, element, strictness)
                    .map(|element| Ty::Value(ValueType::list(element))),
            );
        },
        | TypeFrame::Path { carrier, lhs, rhs } => {
            let child = pop_type_result(results, carrier);
            results.push(
                value_result(child, carrier, strictness)
                    .map(|carrier| Ty::Value(ValueType::path(carrier, lhs, rhs))),
            );
        },
        | TypeFrame::Record { node, fields } => {
            let field_nodes: Vec<SynNode<'_>> = fields
                .iter()
                .map(|field| {
                    let (_, _, ref type_node) = *field;
                    *type_node
                })
                .collect();
            let child_results = pop_type_results(results, &field_nodes);
            let assembled = child_results.and_then(|child_results| {
                let mut lowered = BTreeMap::new();
                for (child, (label, field, type_node)) in child_results.into_iter().zip(fields) {
                    let field_ty = value_result(child, type_node, strictness)?;
                    if lowered.insert(label, field_ty).is_some()
                        && !matches!(strictness, Strictness::Total)
                    {
                        return Err(LowerError::Unsupported {
                            kind: field.kind(),
                            byte_range: field.byte_range(),
                        });
                    }
                }
                Ok(Ty::Value(ValueType::record(lowered)))
            });
            results.push(assembled.or_else(|error| {
                if matches!(strictness, Strictness::Total) {
                    Ok(Ty::Value(ValueType::Unknown))
                }
                else {
                    Err(error)
                }
            }));
            let _ = node;
        },
    }
}

/// Reads a package signature's abstract type component labels, in order.
///
/// Each is a `type_identifier` tile inside the `[ … ]` list. A label declared
/// twice is refused: the second binder would shadow the first everywhere in the
/// payload, so one of the two witnesses supplied at a `pack` could never be
/// reached — a defect in the signature, refused where it is written.
fn package_components(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
) -> LowerResult<Vec<String>>
{
    let mut components: Vec<String> = Vec::new();
    for component in node.children_by_field_name(node_kinds::FIELD_COMPONENT) {
        let name = node_text(source, component)?;
        let name = name.0.to_owned();
        if components.contains(&name) {
            return Err(LowerError::DuplicatePackageComponent {
                name,
                byte_range: component.byte_range(),
            });
        }
        components.push(name);
    }
    Ok(components)
}

/// Converts a lowered type into a value type at one consuming node.
fn value_result(
    result: LowerResult<Ty>,
    node: SynNode<'_>,
    strictness: Strictness,
) -> LowerResult<ValueType>
{
    let total = matches!(strictness, Strictness::Total);
    match result {
        | Ok(Ty::Value(value_ty)) => Ok(value_ty),
        | Ok(_) | Err(_) if total => Ok(ValueType::Unknown),
        | Ok(Ty::Comp(_)) => Err(LowerError::TypeSortMismatch {
            expected: SORT_VALUE_TYPE,
            kind: node.kind(),
            byte_range: node.byte_range(),
        }),
        | Err(error) => Err(error),
    }
}

/// Converts a lowered type into a computation type at one consuming node.
fn comp_result(
    result: LowerResult<Ty>,
    node: SynNode<'_>,
    strictness: Strictness,
) -> LowerResult<CompType>
{
    let total = matches!(strictness, Strictness::Total);
    match result {
        | Ok(Ty::Comp(comp_ty)) => Ok(comp_ty),
        | Ok(_) | Err(_) if total => Ok(CompType::Unknown),
        // The `?` atom in a computation position IS the computation-sort
        // unknown (gandr-89k): the consuming position decides the sort of the
        // one spelling. The legacy `Unknown` keyword keeps its value-only
        // reading — no existing witness places it in a computation position —
        // so it stays a sort mismatch below.
        | Ok(Ty::Value(ValueType::Unknown)) if is_unknown_atom(node).0 => Ok(CompType::Unknown),
        | Ok(Ty::Value(_)) => Err(LowerError::TypeSortMismatch {
            expected: SORT_COMP_TYPE,
            kind: node.kind(),
            byte_range: node.byte_range(),
        }),
        | Err(error) => Err(error),
    }
}

/// Whether `node` is the `?` unknown atom, looking through any
/// `parenthesized_type` wrappers (parentheses are transparent to lowering, so
/// `(?)` in a computation position denotes the computation top exactly as `?`
/// does). Iterative: the wrapper chain is input-scaled.
fn is_unknown_atom(node: SynNode<'_>) -> UnknownAtomFlag
{
    let mut current = node;
    loop {
        match current.kind() {
            | node_kinds::UNKNOWN_TYPE => return UnknownAtomFlag(true),
            | node_kinds::PARENTHESIZED_TYPE => {
                let Ok(inner) = required_field(current, node_kinds::FIELD_TYPE)
                else {
                    return UnknownAtomFlag(false);
                };
                current = inner;
            },
            | _ => return UnknownAtomFlag(false),
        }
    }
}

/// Pops one child type result or synthesizes an internal malformed-node error.
fn pop_type_result(
    results: &mut Vec<LowerResult<Ty>>,
    node: SynNode<'_>,
) -> LowerResult<Ty>
{
    results.pop().unwrap_or_else(|| {
        Err(LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
    })
}

/// Pops a child-result batch and restores source order.
fn pop_type_results(
    results: &mut Vec<LowerResult<Ty>>,
    nodes: &[SynNode<'_>],
) -> LowerResult<Vec<LowerResult<Ty>>>
{
    if results.len() < nodes.len() {
        let Some(node) = nodes.first().copied()
        else {
            return Ok(Vec::new());
        };
        return Err(LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        });
    }
    let split = results.len().saturating_sub(nodes.len());
    Ok(results.split_off(split))
}

/// Lowers one identity-type endpoint — a term in type position (the design
/// record). The rung-1 capture admits exactly a **number literal** (⇒
/// [`Value::Int`]) or a **variable** (⇒ [`Value::Var`]); every other shape is
/// the reserved term-in-type splice (rung 2, decided with the parser owner) —
/// [`LowerError::Unsupported`], holed by the consuming type position in total
/// mode.
///
/// A variable endpoint arrives as a `type_variable` node from the committed
/// tree-sitter view but as a `type_identifier` node from the melder CST (the
/// PBG resolves a type-position word to one atom kind); both spellings lower
/// to the same [`Value::Var`], keyed by the source text.
///
/// # Errors
///
/// [`LowerError::InvalidIntegerLiteral`] for a non-`i64` numeral;
/// [`LowerError::Unsupported`] for any non-number, non-variable endpoint.
fn lower_endpoint_value(
    source: PipelineSource<'_>,
    node: SynNode<'_>,
) -> LowerResult<Value>
{
    match node.kind() {
        | node_kinds::NUMBER => {
            let text = node_text(source, node)?;
            match text.parse::<i64>() {
                | Ok(literal) => Ok(Value::Int(literal)),
                | Err(_parse_error) => Err(LowerError::InvalidIntegerLiteral {
                    text: text.to_owned(),
                    byte_range: node.byte_range(),
                }),
            }
        },
        | node_kinds::TYPE_VARIABLE | node_kinds::TYPE_IDENTIFIER => {
            node_text(source, node).map(|text| Value::var(text.0))
        },
        | kind => Err(LowerError::Unsupported {
            kind,
            byte_range: node.byte_range(),
        }),
    }
}

/// The `member` field nodes of a product/sum/lazy-product type (the grammar
/// guarantees at least two, but totality is rechecked in [`nest_right`]).
fn member_nodes(node: SynNode<'_>) -> Vec<SynNode<'_>>
{
    node.children_by_field_name(node_kinds::FIELD_MEMBER)
}
