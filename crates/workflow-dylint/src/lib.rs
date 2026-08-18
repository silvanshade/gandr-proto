//! Project-local Dylint rules enforcing gandr's Rust type boundaries:
//! `#[repr(transparent)]` on single-field wrappers, semantic-wrapper
//! (non-primitive) function and method signatures, and `# Termination`
//! documentation on recursive functions.

#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
use std::collections::HashMap;
use std::collections::HashSet;

use clippy_utils::diagnostics::span_lint;
use clippy_utils::diagnostics::span_lint_hir;
use clippy_utils::trait_ref_of_method;
use rustc_hir::Attribute;
use rustc_hir::Body;
use rustc_hir::Expr;
use rustc_hir::ExprKind;
use rustc_hir::FnDecl;
use rustc_hir::FnRetTy;
use rustc_hir::ForeignItem;
use rustc_hir::ForeignItemKind;
use rustc_hir::GenericArg;
use rustc_hir::GenericArgs;
use rustc_hir::GenericBound;
use rustc_hir::HirId;
use rustc_hir::Item;
use rustc_hir::ItemKind;
use rustc_hir::LetStmt;
use rustc_hir::Node;
use rustc_hir::OpaqueTy;
use rustc_hir::OwnerId;
use rustc_hir::Pat;
use rustc_hir::PatKind;
use rustc_hir::PrimTy;
use rustc_hir::QPath;
use rustc_hir::TraitFn;
use rustc_hir::TraitItem;
use rustc_hir::TraitItemKind;
use rustc_hir::TraitRef;
use rustc_hir::Ty;
use rustc_hir::TyKind;
use rustc_hir::attrs::ReprAttr;
use rustc_hir::def::DefKind;
use rustc_hir::def::Res;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::find_attr;
use rustc_hir::intravisit::FnKind;
use rustc_hir::intravisit::Visitor;
use rustc_hir::intravisit::walk_expr;
use rustc_hir::intravisit::walk_local;
use rustc_hir::intravisit::walk_pat;
use rustc_lint::LateContext;
use rustc_lint::LateLintPass;
use rustc_lint::LintStore;
use rustc_middle::ty as rustc_ty;
use rustc_session::Session;
use rustc_session::declare_lint;
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::symbol::sym;

dylint_linting::dylint_library!();

declare_lint! {
    /// ### What it does
    ///
    /// Requires every single-field named or tuple struct to declare
    /// `#[repr(transparent)]`.
    ///
    /// ### Why is this bad?
    ///
    /// gandr uses single-field structs as semantic domain wrappers. Making their
    /// transparent representation explicit preserves the intended ABI/layout
    /// contract while keeping the wrapper nominal at the type boundary.
    ///
    /// ### Example
    ///
    /// ```rust
    /// struct UserId(u64);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[repr(transparent)]
    /// struct UserId(u64);
    /// ```
    pub SINGLE_FIELD_STRUCT_NEEDS_TRANSPARENT_REPR,
    Warn,
    "single-field structs must declare #[repr(transparent)]"
}

declare_lint! {
    /// ### What it does
    ///
    /// Rejects function and method signatures that expose Rust primitives in
    /// gandr-owned APIs, including primitives below structural type layers,
    /// selected transparent containers, and type aliases.
    ///
    /// ### Why is this bad?
    ///
    /// Bare primitives erase semantic roles at crate boundaries. Nominal domain
    /// wrappers keep distinct meanings distinct for humans, tools, and agents.
    /// This lint establishes only that the signature reaches a local nominal
    /// transparent boundary. The wrapper's field visibility, conversion traits,
    /// documentation, and other workspace lint obligations remain the
    /// responsibility of Clippy and `docs/workflow/rust.md`.
    ///
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn parse(offset: usize) -> Option<bool> { Some(true) }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[repr(transparent)]
    /// struct ByteOffset(usize);
    ///
    /// #[repr(transparent)]
    /// struct ParseSucceeded(bool);
    ///
    /// fn parse(offset: ByteOffset) -> Option<ParseSucceeded> { Some(ParseSucceeded(true)) }
    /// ```
    pub PRIMITIVE_SIGNATURE,
    Warn,
    "function signatures must use semantic wrappers instead of Rust primitives"
}

declare_lint! {
    /// ### What it does
    ///
    /// Requires every recursive free function or method to document the
    /// termination argument in a fixed `# Termination` rustdoc section.
    ///
    /// ### Why is this bad?
    ///
    /// Recursive control flow must make its decreasing measure and recursion
    /// over inputs explicit so gandr's proof-carrying code remains reviewable.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn depth(node: Node) -> usize { depth(node.child()) }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// # Termination
    /// /// - reason: recursive descent over the finite node tree.
    /// /// - measure: remaining tree height.
    /// /// - boundedness: each branch removes one level.
    /// /// - input recursion: none.
    /// fn depth(node: Node) -> usize { depth(node.child()) }
    /// ```
    pub RECURSIVE_FUNCTION_NEEDS_TERMINATION,
    Warn,
    "recursive functions must document a termination argument"
}

impl_lint_pass!(GandrTypeBoundaries => [
    SINGLE_FIELD_STRUCT_NEEDS_TRANSPARENT_REPR,
    PRIMITIVE_SIGNATURE,
    RECURSIVE_FUNCTION_NEEDS_TERMINATION,
]);

/// Register gandr's project-local Dylint passes.
#[expect(
    clippy::no_mangle_with_rust_abi,
    reason = "dylint's driver loads `register_lints` by exact symbol name and passes rustc-internal \
              types (`Session`, `LintStore`), so a C ABI is impossible by design"
)]
#[unsafe(no_mangle)]
pub fn register_lints(
    sess: &Session,
    lint_store: &mut LintStore,
)
{
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        SINGLE_FIELD_STRUCT_NEEDS_TRANSPARENT_REPR,
        PRIMITIVE_SIGNATURE,
        RECURSIVE_FUNCTION_NEEDS_TERMINATION,
    ]);
    lint_store.register_late_pass(|_| Box::<GandrTypeBoundaries>::default());
}

/// Define a transparent copyable semantic wrapper with bidirectional `From`
/// conversions (the workspace's `DataMention(bool)` pattern).
macro_rules! semantic_copy {
    ($(#[$meta:meta])* struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct $name($inner);

        impl From<$inner> for $name {
            #[inline]
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

/// Define a transparent borrowed-text semantic wrapper with `From`
/// conversions (the workspace's `semantic_borrowed_str` pattern).
macro_rules! semantic_borrowed_str {
    ($(#[$meta:meta])* struct $name:ident;) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct $name<'text>(&'text str);

        impl<'text> From<&'text str> for $name<'text> {
            #[inline]
            fn from(value: &'text str) -> Self {
                Self(value)
            }
        }

        impl<'text> From<&'text String> for $name<'text> {
            #[inline]
            fn from(value: &'text String) -> Self {
                Self(value.as_str())
            }
        }

        impl<'text> From<$name<'text>> for &'text str {
            #[inline]
            fn from(value: $name<'text>) -> Self {
                value.0
            }
        }
    };
}

semantic_copy!(
    /// Whether a single-field struct already declares `#[repr(transparent)]`.
    struct TransparentReprDeclared(bool);
);
semantic_copy!(
    /// Whether a function is a method implementing a non-local trait.
    struct NonLocalTraitImpl(bool);
);
semantic_copy!(
    /// Whether one crate-local function can reach another through local calls.
    struct Reachable(bool);
);
semantic_copy!(
    /// Whether a recursive function carries a valid `# Termination` block.
    struct ValidTerminationDoc(bool);
);
semantic_copy!(
    /// Whether a required `# Termination` bullet has a non-empty value.
    struct BulletHasValue(bool);
);
semantic_copy!(
    /// Whether the `- input recursion:` bullet satisfies gandr's policy.
    struct ValidInputRecursion(bool);
);
semantic_copy!(
    /// Whether a recursive SCC passes caller-input-derived data on some edge.
    struct HasInputDerivedRecursiveCall(bool);
);
semantic_copy!(
    /// Whether an expression references an input-derived local.
    struct ContainsDerivedBinding(bool);
);
semantic_copy!(
    /// Whether a provenance pass added a binding to the input-derived set.
    struct ProvenanceChanged(bool);
);
semantic_copy!(
    /// Whether a function is the model checker's allowed input recursion.
    struct ModelCheckerInputRecursion(bool);
);
semantic_copy!(
    /// Whether a def path is the allowed checker recursion receiver path.
    struct ModelCheckerRecPath(bool);
);
semantic_copy!(
    /// Whether a signature traversal emitted a primitive diagnostic.
    struct PrimitiveDiagnosticEmitted(bool);
);
semantic_copy!(
    /// Whether a primitive is one of gandr's banned signature primitives.
    struct DisallowedPrimitive(bool);
);
semantic_copy!(
    /// Whether an ADT is a semantic wrapper boundary for type-boundary linting.
    struct SemanticBoundaryAdt(bool);
);
semantic_copy!(
    /// Whether a semantic type contains a disallowed primitive before a
    /// nominal gandr boundary.
    struct ContainsPrimitive(bool);
);
semantic_copy!(
    /// Number of primitive-signature diagnostics a traversal has emitted.
    struct DiagnosticCount(usize);
);
semantic_borrowed_str!(
    /// A trimmed rustdoc line of a `# Termination` section.
    struct TerminationLine;
);
semantic_borrowed_str!(
    /// The literal prefix of a required `# Termination` bullet.
    struct BulletPrefix;
);
semantic_borrowed_str!(
    /// A rustc def path rendered as absolute text.
    struct DefPathText;
);

/// Late lint pass implementing gandr's semantic Rust boundary rules.
#[derive(Default)]
struct GandrTypeBoundaries
{
    /// Crate-local function metadata by definition id, filled by `check_fn`.
    functions: HashMap<LocalDefId, FunctionNode>,
    /// Crate-local callsites by caller definition id, filled by `check_fn`.
    edges: HashMap<LocalDefId, Vec<CallEdge>>,
}

/// Crate-local function metadata needed for crate-post recursion diagnostics.
struct FunctionNode
{
    /// The function's whole-item span, used as the diagnostic target.
    span: Span,
    /// The function's stable rustc def-path string, used for deterministic SCC
    /// ordering.
    path: String,
    /// The HIR id of the function body expression root.
    body_hir_id: HirId,
    /// HIR ids of the function's parameter pattern bindings.
    input_bindings: Vec<HirId>,
}

/// One crate-local callsite, preserving the callee and argument HIR roots.
struct CallEdge
{
    /// The callee's crate-local definition id.
    callee: LocalDefId,
    /// HIR ids of the argument expressions, receiver first for method calls.
    args: Vec<HirId>,
}

impl<'tcx> LateLintPass<'tcx> for GandrTypeBoundaries
{
    fn check_item(
        &mut self,
        cx: &LateContext<'tcx>,
        item: &'tcx Item<'tcx>,
    )
    {
        if let ItemKind::Struct(_, _, ref variant_data) = item.kind
            && variant_data.fields().len() == 1_usize
            && !has_transparent_repr(cx, item).0
        {
            span_lint(
                cx,
                SINGLE_FIELD_STRUCT_NEEDS_TRANSPARENT_REPR,
                item.kind.ident().map_or(item.span, |ident| ident.span),
                "single-field struct must declare #[repr(transparent)]",
            );
        }
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        fn_decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        def_id: LocalDefId,
    )
    {
        if matches!(fn_kind, FnKind::Closure) {
            return;
        }

        if !implements_non_local_trait(cx, def_id).0 {
            let semantic_sig = cx
                .tcx
                .fn_sig(def_id)
                .instantiate_identity()
                .skip_norm_wip()
                .skip_binder();
            check_fn_decl(cx, fn_decl, semantic_sig);
        }

        self.functions.insert(def_id, FunctionNode {
            span,
            path: cx.tcx.def_path_str(def_id.to_def_id()),
            body_hir_id: body.value.hir_id,
            input_bindings: parameter_binding_ids(body),
        });
        self.edges.insert(def_id, local_call_edges(cx, body.value));
    }

    fn check_trait_item(
        &mut self,
        cx: &LateContext<'tcx>,
        trait_item: &'tcx TraitItem<'tcx>,
    )
    {
        if let TraitItemKind::Fn(fn_sig, TraitFn::Required(_)) = trait_item.kind {
            let semantic_sig = cx
                .tcx
                .fn_sig(trait_item.owner_id.def_id)
                .instantiate_identity()
                .skip_norm_wip()
                .skip_binder();
            check_fn_decl(cx, fn_sig.decl, semantic_sig);
        }
    }

    fn check_foreign_item(
        &mut self,
        cx: &LateContext<'tcx>,
        item: &'tcx ForeignItem<'tcx>,
    )
    {
        if let ForeignItemKind::Fn(fn_sig, ..) = item.kind {
            let semantic_sig = cx
                .tcx
                .fn_sig(item.owner_id.def_id)
                .instantiate_identity()
                .skip_norm_wip()
                .skip_binder();
            check_fn_decl(cx, fn_sig.decl, semantic_sig);
        }
    }

    fn check_crate_post(
        &mut self,
        cx: &LateContext<'tcx>,
    )
    {
        for scc in recursive_sccs(&self.functions, &self.edges) {
            for def_id in scc.iter().copied() {
                let Some(node) = self.functions.get(&def_id)
                else {
                    continue;
                };
                if termination_doc_is_valid(cx, def_id, &scc, &self.functions, &self.edges).0 {
                    continue;
                }
                span_lint_hir(
                    cx,
                    RECURSIVE_FUNCTION_NEEDS_TERMINATION,
                    cx.tcx.local_def_id_to_hir_id(def_id),
                    node.span,
                    "recursive function must document termination with gandr's rustdoc section",
                );
            }
        }
    }
}

/// Return whether `item` already declares `#[repr(transparent)]`.
fn has_transparent_repr(
    cx: &LateContext<'_>,
    item: &Item<'_>,
) -> TransparentReprDeclared
{
    let attrs = cx.tcx.hir_attrs(item.hir_id());
    TransparentReprDeclared(
        find_attr!(attrs, Repr { reprs, .. } if reprs.iter().any(|&(repr, _)| repr == ReprAttr::ReprTransparent)),
    )
}

/// Return whether `def_id` is a method implementing a non-local trait.
fn implements_non_local_trait(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
) -> NonLocalTraitImpl
{
    NonLocalTraitImpl(
        trait_ref_of_method(cx, OwnerId { def_id }).is_some_and(|trait_ref| {
            trait_ref
                .trait_def_id()
                .is_some_and(|trait_def_id| !trait_def_id.is_local())
        }),
    )
}

/// Collect crate-local free-function and method callsites under `expr`.
fn local_call_edges<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Vec<CallEdge>
{
    let mut collector = LocalCallCollector {
        cx,
        calls: Vec::new(),
    };
    collector.visit_expr(expr);
    collector.calls
}

/// HIR visitor that records direct local function/method callsites.
struct LocalCallCollector<'cx, 'tcx>
{
    /// The late lint context used for path and method resolution.
    cx: &'cx LateContext<'tcx>,
    /// The callsites recorded so far, in visit order.
    calls: Vec<CallEdge>,
}

#[expect(
    clippy::renamed_function_params,
    reason = "rustc declares the `Visitor` methods with single-letter parameter names (`ex`, `l`, \
              `p`); the implementation keeps descriptive names"
)]
impl<'tcx> Visitor<'tcx> for LocalCallCollector<'_, 'tcx>
{
    fn visit_expr(
        &mut self,
        expr: &'tcx Expr<'_>,
    )
    {
        match expr.kind {
            | ExprKind::Call(callee, args) => {
                if let ExprKind::Path(ref qpath) = callee.kind
                    && let Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) =
                        self.cx.qpath_res(qpath, callee.hir_id)
                    && let Some(local_def_id) = def_id.as_local()
                {
                    self.calls.push(CallEdge {
                        callee: local_def_id,
                        args: args.iter().map(|arg| arg.hir_id).collect(),
                    });
                }
            },
            | ExprKind::MethodCall(_, receiver, args, _) => {
                if let Some(def_id) = self.cx.typeck_results().type_dependent_def_id(expr.hir_id)
                    && let Some(local_def_id) = def_id.as_local()
                {
                    let mut call_args = Vec::with_capacity(args.len().saturating_add(1_usize));
                    call_args.push(receiver.hir_id);
                    call_args.extend(args.iter().map(|arg| arg.hir_id));
                    self.calls.push(CallEdge {
                        callee: local_def_id,
                        args: call_args,
                    });
                }
            },
            | _ => {},
        }
        walk_expr(self, expr);
    }
}

/// Return every crate-local recursive SCC, each sorted by stable rustc path.
fn recursive_sccs(
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> Vec<Vec<LocalDefId>>
{
    let ids = sorted_function_ids(functions);
    let recursive: HashSet<_> = ids
        .iter()
        .copied()
        .filter(|&def_id| {
            let mut visited = HashSet::new();
            reaches_target(def_id, def_id, edges, functions, &mut visited).0
        })
        .collect();

    let mut claimed = HashSet::new();
    let mut sccs = Vec::new();
    for def_id in ids {
        if !recursive.contains(&def_id) || claimed.contains(&def_id) {
            continue;
        }
        let mut scc: Vec<_> = sorted_function_ids(functions)
            .into_iter()
            .filter(|&other| {
                other == def_id
                    || (reaches(def_id, other, edges, functions).0
                        && reaches(other, def_id, edges, functions).0)
            })
            .collect();
        scc.sort_by_key(|def_id| functions.get(def_id).map(|node| node.path.as_str()));
        for member in &scc {
            claimed.insert(*member);
        }
        sccs.push(scc);
    }
    sccs
}

/// Return crate-local function ids sorted by rustc's stable path string.
fn sorted_function_ids(functions: &HashMap<LocalDefId, FunctionNode>) -> Vec<LocalDefId>
{
    let mut ids: Vec<_> = functions.keys().copied().collect();
    ids.sort_by_key(|def_id| functions.get(def_id).map(|node| node.path.as_str()));
    ids
}

/// Return whether `start` can reach `target` through crate-local calls.
fn reaches(
    start: LocalDefId,
    target: LocalDefId,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
    functions: &HashMap<LocalDefId, FunctionNode>,
) -> Reachable
{
    let mut visited = HashSet::new();
    reaches_target(start, target, edges, functions, &mut visited)
}

/// Depth-first search for [`reaches`], driven by an explicit stack so the
/// traversal never recurses over caller input.
fn reaches_target(
    start: LocalDefId,
    target: LocalDefId,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
    functions: &HashMap<LocalDefId, FunctionNode>,
    visited: &mut HashSet<LocalDefId>,
) -> Reachable
{
    let mut pending: Vec<LocalDefId> = Vec::new();
    if let Some(callees) = edges.get(&start) {
        for edge in callees {
            if edge.callee == target {
                return Reachable(true);
            }
            if functions.contains_key(&edge.callee) && visited.insert(edge.callee) {
                pending.push(edge.callee);
            }
        }
    }
    while let Some(current) = pending.pop() {
        let Some(callees) = edges.get(&current)
        else {
            continue;
        };
        for edge in callees {
            if edge.callee == target {
                return Reachable(true);
            }
            if functions.contains_key(&edge.callee) && visited.insert(edge.callee) {
                pending.push(edge.callee);
            }
        }
    }
    Reachable(false)
}

/// Return whether a recursive function has the required termination doc block.
fn termination_doc_is_valid(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> ValidTerminationDoc
{
    let lines = rustdoc_lines(cx, def_id);
    let Some(start) = lines.iter().position(|line| line == "# Termination")
    else {
        return ValidTerminationDoc(false);
    };
    let mut section_lines = lines
        .get(start.saturating_add(1) ..)
        .unwrap_or_default()
        .iter()
        .filter(|line| !line.is_empty());
    let Some(reason) = section_lines.next()
    else {
        return ValidTerminationDoc(false);
    };
    let Some(measure) = section_lines.next()
    else {
        return ValidTerminationDoc(false);
    };
    let Some(boundedness) = section_lines.next()
    else {
        return ValidTerminationDoc(false);
    };
    let Some(input_recursion) = section_lines.next()
    else {
        return ValidTerminationDoc(false);
    };

    ValidTerminationDoc(
        required_bullet_has_value(reason.into(), "- reason:".into()).0
            && required_bullet_has_value(measure.into(), "- measure:".into()).0
            && required_bullet_has_value(boundedness.into(), "- boundedness:".into()).0
            && input_recursion_is_valid(cx, def_id, input_recursion.into(), scc, functions, edges)
                .0,
    )
}

/// Return rustdoc lines attached to `def_id`, trimmed for structural matching.
fn rustdoc_lines(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
) -> Vec<String>
{
    let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);
    cx.tcx
        .hir_attrs(hir_id)
        .iter()
        .filter_map(Attribute::doc_str)
        .flat_map(|doc| {
            doc.as_str()
                .lines()
                .map(|line| line.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Return whether `line` is a required bullet with a non-empty value.
fn required_bullet_has_value(
    line: TerminationLine<'_>,
    prefix: BulletPrefix<'_>,
) -> BulletHasValue
{
    BulletHasValue(
        line.0
            .strip_prefix(prefix.0)
            .is_some_and(|value| !value.trim().is_empty()),
    )
}

/// Return whether the `- input recursion:` bullet satisfies gandr's policy.
fn input_recursion_is_valid(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
    line: TerminationLine<'_>,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> ValidInputRecursion
{
    if line.0 == "- input recursion: none." {
        return ValidInputRecursion(
            !scc_has_input_derived_recursive_call(cx, scc, functions, edges).0,
        );
    }
    ValidInputRecursion(
        allows_model_checker_input_recursion(cx, def_id).0
            && required_bullet_has_value(line, "- input recursion:".into()).0,
    )
}

/// Return whether any call edge inside `scc` passes caller-input-derived data.
fn scc_has_input_derived_recursive_call(
    cx: &LateContext<'_>,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> HasInputDerivedRecursiveCall
{
    let members: HashSet<_> = scc.iter().copied().collect();
    for caller in scc {
        let Some(node) = functions.get(caller)
        else {
            continue;
        };
        let derived = input_derived_bindings(cx, node);
        let Some(call_edges) = edges.get(caller)
        else {
            continue;
        };
        for edge in call_edges {
            if !members.contains(&edge.callee) {
                continue;
            }
            for arg in &edge.args {
                let Some(expr) = expr_for_hir_id(cx, *arg)
                else {
                    return HasInputDerivedRecursiveCall(true);
                };
                if expr_contains_derived_binding(cx, &derived, expr).0 {
                    return HasInputDerivedRecursiveCall(true);
                }
            }
        }
    }
    HasInputDerivedRecursiveCall(false)
}

/// Return the final flow-insensitive set of locals derived from function
/// inputs.
fn input_derived_bindings(
    cx: &LateContext<'_>,
    node: &FunctionNode,
) -> HashSet<HirId>
{
    let mut derived: HashSet<_> = node.input_bindings.iter().copied().collect();
    let Some(body) = expr_for_hir_id(cx, node.body_hir_id)
    else {
        return derived;
    };
    loop {
        let mut propagation = ProvenancePropagation {
            cx,
            derived: &mut derived,
            changed: false,
        };
        propagation.visit_expr(body);
        if !propagation.changed {
            break;
        }
    }
    derived
}

/// HIR visitor that grows the input-provenance set to a fixed point.
struct ProvenancePropagation<'derived, 'cx, 'tcx>
{
    /// The late lint context used for local resolution.
    cx: &'cx LateContext<'tcx>,
    /// The input-provenance set being grown to a fixed point.
    derived: &'derived mut HashSet<HirId>,
    /// Whether the current pass added any binding to `derived`.
    changed: bool,
}

#[expect(
    clippy::renamed_function_params,
    reason = "rustc declares the `Visitor` methods with single-letter parameter names (`ex`, `l`, \
              `p`); the implementation keeps descriptive names"
)]
impl<'tcx> Visitor<'tcx> for ProvenancePropagation<'_, '_, 'tcx>
{
    fn visit_local(
        &mut self,
        local: &'tcx LetStmt<'tcx>,
    )
    {
        if let Some(init) = local.init
            && expr_contains_derived_binding(self.cx, self.derived, init).0
        {
            self.changed |= mark_pattern_bindings(local.pat, self.derived).0;
        }
        walk_local(self, local);
    }

    fn visit_expr(
        &mut self,
        expr: &'tcx Expr<'_>,
    )
    {
        match expr.kind {
            | ExprKind::Let(let_expr) => {
                if expr_contains_derived_binding(self.cx, self.derived, let_expr.init).0 {
                    self.changed |= mark_pattern_bindings(let_expr.pat, self.derived).0;
                }
            },
            | ExprKind::Match(scrutinee, arms, _) => {
                if expr_contains_derived_binding(self.cx, self.derived, scrutinee).0 {
                    for arm in arms {
                        self.changed |= mark_pattern_bindings(arm.pat, self.derived).0;
                    }
                }
            },
            | ExprKind::Assign(target, value, _) => {
                if expr_contains_derived_binding(self.cx, self.derived, value).0 {
                    self.changed |= mark_local_references(target, self.cx, self.derived).0;
                }
            },
            | ExprKind::AssignOp(_, target, value) => {
                if expr_contains_derived_binding(self.cx, self.derived, target).0
                    || expr_contains_derived_binding(self.cx, self.derived, value).0
                {
                    self.changed |= mark_local_references(target, self.cx, self.derived).0;
                }
            },
            | _ => {},
        }
        walk_expr(self, expr);
    }
}

/// Return all parameter pattern bindings in `body`.
fn parameter_binding_ids(body: &Body<'_>) -> Vec<HirId>
{
    let mut bindings = Vec::new();
    for param in body.params {
        collect_pattern_bindings(param.pat, &mut bindings);
    }
    bindings
}

/// Mark every binding introduced by `pat` as input-derived.
fn mark_pattern_bindings(
    pat: &Pat<'_>,
    derived: &mut HashSet<HirId>,
) -> ProvenanceChanged
{
    let mut bindings = Vec::new();
    collect_pattern_bindings(pat, &mut bindings);
    let mut changed = false;
    for binding in bindings {
        changed |= derived.insert(binding);
    }
    ProvenanceChanged(changed)
}

/// Collect every binding introduced by `pat`.
fn collect_pattern_bindings(
    pat: &Pat<'_>,
    bindings: &mut Vec<HirId>,
)
{
    let mut collector = PatternBindingCollector { bindings };
    collector.visit_pat(pat);
}

/// Pattern visitor that records local bindings.
#[repr(transparent)]
struct PatternBindingCollector<'bindings>
{
    /// The binding HIR ids recorded so far, in visit order.
    bindings: &'bindings mut Vec<HirId>,
}

#[expect(
    clippy::renamed_function_params,
    reason = "rustc declares the `Visitor` methods with single-letter parameter names (`ex`, `l`, \
              `p`); the implementation keeps descriptive names"
)]
impl<'tcx> Visitor<'tcx> for PatternBindingCollector<'_>
{
    fn visit_pat(
        &mut self,
        pat: &'tcx Pat<'_>,
    )
    {
        if let PatKind::Binding(_, hir_id, ..) = pat.kind
            && !self.bindings.contains(&hir_id)
        {
            self.bindings.push(hir_id);
        }
        walk_pat(self, pat);
    }
}

/// Return whether `expr` contains any reference to an input-derived local.
fn expr_contains_derived_binding<'tcx>(
    cx: &LateContext<'tcx>,
    derived: &HashSet<HirId>,
    expr: &'tcx Expr<'tcx>,
) -> ContainsDerivedBinding
{
    let mut visitor = DerivedBindingFinder {
        cx,
        derived,
        found: false,
    };
    visitor.visit_expr(expr);
    ContainsDerivedBinding(visitor.found)
}

/// Expression visitor that finds references to already-derived locals.
struct DerivedBindingFinder<'derived, 'cx, 'tcx>
{
    /// The late lint context used for local resolution.
    cx: &'cx LateContext<'tcx>,
    /// The current input-provenance set.
    derived: &'derived HashSet<HirId>,
    /// Whether a reference to a derived local has been found.
    found: bool,
}

#[expect(
    clippy::renamed_function_params,
    reason = "rustc declares the `Visitor` methods with single-letter parameter names (`ex`, `l`, \
              `p`); the implementation keeps descriptive names"
)]
impl<'tcx> Visitor<'tcx> for DerivedBindingFinder<'_, '_, 'tcx>
{
    fn visit_expr(
        &mut self,
        expr: &'tcx Expr<'_>,
    )
    {
        if self.found {
            return;
        }
        if let ExprKind::Path(ref qpath) = expr.kind
            && let Res::Local(hir_id) = self.cx.qpath_res(qpath, expr.hir_id)
            && self.derived.contains(&hir_id)
        {
            self.found = true;
            return;
        }
        walk_expr(self, expr);
    }
}

/// Mark every local reference in `expr`; used conservatively for assignments.
fn mark_local_references<'tcx>(
    expr: &'tcx Expr<'tcx>,
    cx: &LateContext<'tcx>,
    derived: &mut HashSet<HirId>,
) -> ProvenanceChanged
{
    let mut collector = LocalReferenceCollector {
        cx,
        locals: Vec::new(),
    };
    collector.visit_expr(expr);
    let mut changed = false;
    for local in collector.locals {
        changed |= derived.insert(local);
    }
    ProvenanceChanged(changed)
}

/// Expression visitor that records every referenced local binding.
struct LocalReferenceCollector<'cx, 'tcx>
{
    /// The late lint context used for local resolution.
    cx: &'cx LateContext<'tcx>,
    /// The referenced local binding HIR ids recorded so far, in visit order.
    locals: Vec<HirId>,
}

#[expect(
    clippy::renamed_function_params,
    reason = "rustc declares the `Visitor` methods with single-letter parameter names (`ex`, `l`, \
              `p`); the implementation keeps descriptive names"
)]
impl<'tcx> Visitor<'tcx> for LocalReferenceCollector<'_, 'tcx>
{
    fn visit_expr(
        &mut self,
        expr: &'tcx Expr<'_>,
    )
    {
        if let ExprKind::Path(ref qpath) = expr.kind
            && let Res::Local(hir_id) = self.cx.qpath_res(qpath, expr.hir_id)
            && !self.locals.contains(&hir_id)
        {
            self.locals.push(hir_id);
        }
        walk_expr(self, expr);
    }
}

/// Return an expression by HIR id when the id still names an expression node.
fn expr_for_hir_id<'tcx>(
    cx: &LateContext<'tcx>,
    hir_id: HirId,
) -> Option<&'tcx Expr<'tcx>>
{
    match cx.tcx.hir_node(hir_id) {
        | Node::Expr(expr) => Some(expr),
        | _ => None,
    }
}

/// Return whether `def_id` is an inherent method on
/// `gandr_core_checker::judgements::checker::Rec`.
///
/// # Contract
///
/// - requires: `def_id` names a local function definition visited by the late
///   lint pass.
/// - ensures: returns an affirmative [`ModelCheckerInputRecursion`] exactly
///   when the item is an inherent method whose fully peeled receiver `ADT` is
///   `gandr_core_checker::judgements::checker::Rec`.
/// - provides: the sole model-checker exception to the input-recursion policy.
/// - panics: none under rustc's late-lint function-definition invariants.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — the UI matrix accepts the exact checker
///   receiver and rejects free functions plus similarly named or prefixed
///   paths.
/// - witness: `ui`.
fn allows_model_checker_input_recursion(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
) -> ModelCheckerInputRecursion
{
    if cx.tcx.inherent_impl_of_assoc(def_id.to_def_id()).is_none() {
        return ModelCheckerInputRecursion(false);
    }
    let assoc_item = cx.tcx.associated_item(def_id.to_def_id());
    if !assoc_item.is_method() {
        return ModelCheckerInputRecursion(false);
    }

    let fn_sig = cx
        .tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .skip_binder();
    let Some(receiver_ty) = fn_sig.inputs().first().copied()
    else {
        return ModelCheckerInputRecursion(false);
    };
    let receiver_ty = peel_reference_ty(normalize_middle_ty(cx, receiver_ty));
    let Some(adt_def) = receiver_ty.ty_adt_def()
    else {
        return ModelCheckerInputRecursion(false);
    };
    ModelCheckerInputRecursion(
        is_model_checker_rec_path(DefPathText::from(&absolute_def_path(cx, adt_def.did()))).0,
    )
}

/// Peel reference layers from a receiver type before checking the receiver ADT.
fn peel_reference_ty(mut ty: rustc_ty::Ty<'_>) -> rustc_ty::Ty<'_>
{
    loop {
        match *ty.kind() {
            | rustc_ty::Ref(_, inner, _) => ty = inner,
            | _ => return ty,
        }
    }
}

/// Return a rustc def path prefixed with the local crate name when needed.
fn absolute_def_path(
    cx: &LateContext<'_>,
    def_id: rustc_hir::def_id::DefId,
) -> String
{
    let path = cx.tcx.def_path_str(def_id);
    if def_id.is_local() {
        format!("{}::{path}", cx.tcx.crate_name(LOCAL_CRATE))
    }
    else {
        path
    }
}

/// Return whether `path` is the one allowed checker recursion receiver path.
fn is_model_checker_rec_path(path: DefPathText<'_>) -> ModelCheckerRecPath
{
    ModelCheckerRecPath(path.0 == "gandr_core_checker::judgements::checker::Rec")
}

/// One work item of the order-preserving primitive-signature traversal.
enum PrimitiveWork<'tcx>
{
    /// A HIR type paired with its substituted semantic type.
    SemanticTy(&'tcx Ty<'tcx>, rustc_ty::Ty<'tcx>),
    /// A HIR function declaration paired with its semantic signature.
    SemanticFnDecl(&'tcx FnDecl<'tcx>, rustc_ty::FnSig<'tcx>),
    /// A HIR type checked syntactically because semantic normalization hid
    /// the declared type, as async function signatures do.
    HirTy(&'tcx Ty<'tcx>),
    /// A HIR-only function pointer declaration.
    HirFnDecl(&'tcx FnDecl<'tcx>),
    /// An opaque async return type whose declared `Future::Output` is checked.
    Opaque(&'tcx OpaqueTy<'tcx>),
    /// Emit at a path node when no generic-argument descendant emitted.
    PathFallback(&'tcx Ty<'tcx>, DiagnosticCount),
}

/// Check every input and explicit output type in one function declaration,
/// descending through structural type constructors with an explicit worklist
/// (never input recursion) and emitting the primitive signature diagnostic at
/// each offending HIR span in declaration order.
fn check_fn_decl<'tcx>(
    cx: &LateContext<'tcx>,
    fn_decl: &'tcx FnDecl<'tcx>,
    fn_sig: rustc_ty::FnSig<'tcx>,
) -> PrimitiveDiagnosticEmitted
{
    let mut work = vec![PrimitiveWork::SemanticFnDecl(fn_decl, fn_sig)];
    let mut diagnostics = DiagnosticCount(0_usize);
    while let Some(item) = work.pop() {
        match item {
            | PrimitiveWork::SemanticTy(ty, semantic_ty) => {
                let semantic_ty = normalize_middle_ty(cx, semantic_ty);
                if !middle_ty_contains_primitive(cx, semantic_ty).0 {
                    work.push(PrimitiveWork::HirTy(ty));
                    continue;
                }
                match ty.kind {
                    | TyKind::Slice(inner) | TyKind::Array(inner, _) => match *semantic_ty.kind() {
                        | rustc_ty::Slice(semantic_inner) | rustc_ty::Array(semantic_inner, _) => {
                            work.push(PrimitiveWork::SemanticTy(inner, semantic_inner));
                        },
                        | _ => {
                            emit_primitive(cx, ty.span);
                            diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                        },
                    },
                    | TyKind::Ptr(mut_ty) => match *semantic_ty.kind() {
                        | rustc_ty::RawPtr(semantic_inner, _) => {
                            work.push(PrimitiveWork::SemanticTy(mut_ty.ty, semantic_inner));
                        },
                        | _ => {
                            emit_primitive(cx, ty.span);
                            diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                        },
                    },
                    | TyKind::Ref(_, mut_ty) => match *semantic_ty.kind() {
                        | rustc_ty::Ref(_, semantic_inner, _) => {
                            work.push(PrimitiveWork::SemanticTy(mut_ty.ty, semantic_inner));
                        },
                        | _ => {
                            emit_primitive(cx, ty.span);
                            diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                        },
                    },
                    | TyKind::FnPtr(fn_ptr) => match *semantic_ty.kind() {
                        | rustc_ty::FnPtr(sig_tys, header) => {
                            work.push(PrimitiveWork::SemanticFnDecl(
                                fn_ptr.decl,
                                sig_tys.with(header).skip_binder(),
                            ));
                        },
                        | _ => {
                            emit_primitive(cx, ty.span);
                            diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                        },
                    },
                    | TyKind::Tup(types) => match *semantic_ty.kind() {
                        | rustc_ty::Tuple(semantic_types) if semantic_types.len() == types.len() => {
                            for (inner, semantic_inner) in
                                types.iter().zip(semantic_types.iter()).rev()
                            {
                                work.push(PrimitiveWork::SemanticTy(inner, semantic_inner));
                            }
                        },
                        | _ => {
                            emit_primitive(cx, ty.span);
                            diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                        },
                    },
                    | TyKind::Path(ref qpath) => {
                        work.push(PrimitiveWork::PathFallback(ty, diagnostics));
                        if let Some(generic_args) = last_segment_args(qpath) {
                            let semantic_args = semantic_type_args(cx, semantic_ty);
                            let mut pairs: Vec<_> = generic_args
                                .args
                                .iter()
                                .filter_map(|arg| match arg {
                                    | &GenericArg::Type(generic_ty) => {
                                        Some(generic_ty.as_unambig_ty())
                                    },
                                    | _ => None,
                                })
                                .zip(semantic_args)
                                .collect();
                            while let Some((hir_ty, semantic_arg)) = pairs.pop() {
                                work.push(PrimitiveWork::SemanticTy(hir_ty, semantic_arg));
                            }
                        }
                    },
                    | TyKind::Pat(inner, _) | TyKind::FieldOf(inner, _) => {
                        work.push(PrimitiveWork::SemanticTy(inner, semantic_ty));
                    },
                    | TyKind::OpaqueDef(opaque) => {
                        work.push(PrimitiveWork::Opaque(opaque));
                    },
                    | TyKind::InferDelegation(_)
                    | TyKind::UnsafeBinder(_)
                    | TyKind::Never
                    | TyKind::TraitAscription(_)
                    | TyKind::TraitObject(..)
                    | TyKind::Err(_)
                    | TyKind::Infer(()) => {},
                }
            },
            | PrimitiveWork::SemanticFnDecl(decl, sig) => {
                if let FnRetTy::Return(output) = decl.output {
                    work.push(PrimitiveWork::SemanticTy(output, sig.output()));
                }
                for (input, semantic_input) in decl.inputs.iter().zip(sig.inputs()).rev() {
                    work.push(PrimitiveWork::SemanticTy(input, *semantic_input));
                }
            },
            | PrimitiveWork::HirTy(ty) => match ty.kind {
                | TyKind::Slice(inner)
                | TyKind::Array(inner, _)
                | TyKind::Pat(inner, _)
                | TyKind::FieldOf(inner, _) => {
                    work.push(PrimitiveWork::HirTy(inner));
                },
                | TyKind::Ptr(mut_ty) | TyKind::Ref(_, mut_ty) => {
                    work.push(PrimitiveWork::HirTy(mut_ty.ty));
                },
                | TyKind::FnPtr(fn_ptr) => {
                    work.push(PrimitiveWork::HirFnDecl(fn_ptr.decl));
                },
                | TyKind::Tup(types) => {
                    for inner in types.iter().rev() {
                        work.push(PrimitiveWork::HirTy(inner));
                    }
                },
                | TyKind::Path(ref qpath) => {
                    if let Res::PrimTy(primitive) = cx.qpath_res(qpath, ty.hir_id)
                        && is_disallowed_primitive(primitive).0
                    {
                        emit_primitive(cx, ty.span);
                        diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                    }
                },
                | TyKind::OpaqueDef(opaque) => {
                    work.push(PrimitiveWork::Opaque(opaque));
                },
                | TyKind::InferDelegation(_)
                | TyKind::UnsafeBinder(_)
                | TyKind::Never
                | TyKind::TraitAscription(_)
                | TyKind::TraitObject(..)
                | TyKind::Err(_)
                | TyKind::Infer(()) => {},
            },
            | PrimitiveWork::HirFnDecl(decl) => {
                if let FnRetTy::Return(output) = decl.output {
                    work.push(PrimitiveWork::HirTy(output));
                }
                for input in decl.inputs.iter().rev() {
                    work.push(PrimitiveWork::HirTy(input));
                }
            },
            | PrimitiveWork::Opaque(opaque) => {
                if let Some(trait_ref) = future_trait_ref(cx, opaque)
                    && let Some(output) = future_output_ty(trait_ref)
                {
                    work.push(PrimitiveWork::HirTy(output));
                }
            },
            | PrimitiveWork::PathFallback(ty, snapshot) => {
                if diagnostics == snapshot {
                    emit_primitive(cx, ty.span);
                    diagnostics.0 = diagnostics.0.saturating_add(1_usize);
                }
            },
        }
    }
    PrimitiveDiagnosticEmitted(diagnostics.0 > 0_usize)
}

/// Return the `Future` bound on an opaque async return type, when present.
fn future_trait_ref<'tcx>(
    cx: &LateContext<'tcx>,
    opaque: &'tcx OpaqueTy<'tcx>,
) -> Option<&'tcx TraitRef<'tcx>>
{
    if let Some(trait_ref) = opaque.bounds.iter().find_map(|bound| match bound {
        | &GenericBound::Trait(ref poly) => Some(&poly.trait_ref),
        | _ => None,
    }) && trait_ref.trait_def_id() == cx.tcx.lang_items().future_trait()
    {
        return Some(trait_ref);
    }
    None
}

/// Return the declared `Future::Output` type from a future trait reference.
fn future_output_ty<'tcx>(trait_ref: &'tcx TraitRef<'tcx>) -> Option<&'tcx Ty<'tcx>>
{
    if let Some(segment) = trait_ref.path.segments.last()
        && let Some(args) = segment.args
        && args.constraints.len() == 1_usize
        && let Some(constraint) = args.constraints.first()
        && constraint.ident.name == sym::Output
        && let Some(output) = constraint.ty()
    {
        return Some(output);
    }
    None
}

/// Return substituted semantic type arguments represented by a path type.
fn semantic_type_args<'tcx>(
    cx: &LateContext<'tcx>,
    semantic_ty: rustc_ty::Ty<'tcx>,
) -> Vec<rustc_ty::Ty<'tcx>>
{
    let semantic_ty = normalize_middle_ty(cx, semantic_ty);
    match *semantic_ty.kind() {
        | rustc_ty::Adt(_, args) => args
            .iter()
            .filter_map(rustc_ty::GenericArg::as_type)
            .collect(),
        | rustc_ty::Tuple(types) => types.iter().collect(),
        | _ => Vec::new(),
    }
}

/// Return the last path segment's generic arguments, if any.
fn last_segment_args<'hir>(qpath: &QPath<'hir>) -> Option<&'hir GenericArgs<'hir>>
{
    match *qpath {
        | QPath::Resolved(_, path) => {
            let segment = path.segments.last()?;
            segment.args
        },
        | QPath::TypeRelative(_, segment) => segment.args,
    }
}

/// Return whether `primitive` is one of gandr's banned signature primitives.
fn is_disallowed_primitive(primitive: PrimTy) -> DisallowedPrimitive
{
    DisallowedPrimitive(matches!(
        primitive,
        PrimTy::Bool
            | PrimTy::Char
            | PrimTy::Int(_)
            | PrimTy::Uint(_)
            | PrimTy::Float(_)
            | PrimTy::Str
    ))
}

/// Return whether an ADT is a semantic wrapper boundary for type-boundary
/// linting.
fn is_semantic_boundary_adt(adt: rustc_ty::AdtDef<'_>) -> SemanticBoundaryAdt
{
    SemanticBoundaryAdt(adt.did().is_local() && adt.repr().transparent())
}

/// Return whether a substituted semantic type contains a disallowed primitive
/// before a nominal gandr boundary, using an explicit worklist.
fn middle_ty_contains_primitive<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_ty::Ty<'tcx>,
) -> ContainsPrimitive
{
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        let ty = normalize_middle_ty(cx, ty);
        match *ty.kind() {
            | rustc_ty::Bool
            | rustc_ty::Char
            | rustc_ty::Int(_)
            | rustc_ty::Uint(_)
            | rustc_ty::Float(_)
            | rustc_ty::Str => return ContainsPrimitive(true),
            | rustc_ty::Array(inner, _)
            | rustc_ty::Pat(inner, _)
            | rustc_ty::Slice(inner)
            | rustc_ty::RawPtr(inner, _)
            | rustc_ty::Ref(_, inner, _) => pending.push(inner),
            | rustc_ty::Tuple(types) => pending.extend(types.iter()),
            | rustc_ty::FnPtr(sig_tys, header) => {
                let sig = sig_tys.with(header).skip_binder();
                pending.push(sig.output());
                pending.extend(sig.inputs().iter().rev());
            },
            | rustc_ty::Adt(adt, _) if is_semantic_boundary_adt(adt).0 => {},
            | rustc_ty::Adt(_, args) => {
                pending.extend(args.iter().filter_map(rustc_ty::GenericArg::as_type));
            },
            | _ => {},
        }
    }
    ContainsPrimitive(false)
}

/// Normalize aliases/projections where rustc can do so in this typing context.
fn normalize_middle_ty<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_ty::Ty<'tcx>,
) -> rustc_ty::Ty<'tcx>
{
    cx.tcx
        .try_normalize_erasing_regions(cx.typing_env(), rustc_ty::Unnormalized::new_wip(ty))
        .unwrap_or(ty)
}

/// Emit the primitive signature diagnostic at `span`.
fn emit_primitive(
    cx: &LateContext<'_>,
    span: Span,
)
{
    span_lint(
        cx,
        PRIMITIVE_SIGNATURE,
        span,
        "signature exposes a Rust primitive before a semantic wrapper boundary",
    );
}

#[cfg(test)]
mod tests
{
    use super::is_model_checker_rec_path;

    #[test]
    fn ui()
    {
        dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
    }

    #[test]
    fn ui_model_checker_rec_path_scope()
    {
        assert!(
            is_model_checker_rec_path("gandr_core_checker::judgements::checker::Rec".into()).0,
            "the exact checker receiver path is accepted"
        );
        assert!(
            !is_model_checker_rec_path("gandr_core_checker::discipline::mark::Rec".into()).0,
            "a same-named type in another module is rejected"
        );
        assert!(
            !is_model_checker_rec_path(
                "termination::gandr_core_checker::judgements::checker::Rec".into()
            )
            .0,
            "a prefixed lookalike path is rejected"
        );
    }
}
