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
use rustc_hir::GenericBound;
use rustc_hir::HirId;
use rustc_hir::Item;
use rustc_hir::ItemKind;
use rustc_hir::Node;
use rustc_hir::OpaqueTy;
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
    lint_store.register_late_lint_pass(Box::new(|_| Box::<GandrTypeBoundaries>::default()));
}

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
    /// The function's stable rustc def-path string, used for deterministic SCC ordering.
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
        if let ItemKind::Struct(_, _, variant_data) = &item.kind
            && variant_data.fields().len() == 1_usize
            && !has_transparent_repr(cx, item)
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

        if !implements_non_local_trait(cx, def_id) {
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
                if termination_doc_is_valid(cx, def_id, &scc, &self.functions, &self.edges) {
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
) -> bool
{
    let attrs = cx.tcx.hir_attrs(item.hir_id());
    find_attr!(attrs, Repr { reprs, .. } if reprs.iter().any(|(repr, _)| *repr == ReprAttr::ReprTransparent))
}

/// Return whether `def_id` is a method implementing a non-local trait.
fn implements_non_local_trait(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
) -> bool
{
    trait_ref_of_method(cx, rustc_hir::OwnerId { def_id }).is_some_and(|trait_ref| {
        trait_ref
            .trait_def_id()
            .is_some_and(|trait_def_id| !trait_def_id.is_local())
    })
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
        .filter(|def_id| {
            let mut visited = HashSet::new();
            reaches_target(*def_id, *def_id, edges, functions, &mut visited)
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
            .filter(|other| {
                *other == def_id
                    || (reaches(def_id, *other, edges, functions)
                        && reaches(*other, def_id, edges, functions))
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
) -> bool
{
    let mut visited = HashSet::new();
    reaches_target(start, target, edges, functions, &mut visited)
}

/// DFS step for [`reaches`].
fn reaches_target(
    current: LocalDefId,
    target: LocalDefId,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
    functions: &HashMap<LocalDefId, FunctionNode>,
    visited: &mut HashSet<LocalDefId>,
) -> bool
{
    let Some(callees) = edges.get(&current)
    else {
        return false;
    };
    for edge in callees {
        if edge.callee == target {
            return true;
        }
        if functions.contains_key(&edge.callee)
            && visited.insert(edge.callee)
            && reaches_target(edge.callee, target, edges, functions, visited)
        {
            return true;
        }
    }
    false
}

/// Return whether a recursive function has the required termination doc block.
fn termination_doc_is_valid(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> bool
{
    let lines = rustdoc_lines(cx, def_id);
    let Some(start) = lines.iter().position(|line| line == "# Termination")
    else {
        return false;
    };
    let mut section_lines = lines
        .get(start.saturating_add(1) ..)
        .unwrap_or_default()
        .iter()
        .filter(|line| !line.is_empty());
    let Some(reason) = section_lines.next()
    else {
        return false;
    };
    let Some(measure) = section_lines.next()
    else {
        return false;
    };
    let Some(boundedness) = section_lines.next()
    else {
        return false;
    };
    let Some(input_recursion) = section_lines.next()
    else {
        return false;
    };

    required_bullet_has_value(reason, "- reason:")
        && required_bullet_has_value(measure, "- measure:")
        && required_bullet_has_value(boundedness, "- boundedness:")
        && input_recursion_is_valid(cx, def_id, input_recursion, scc, functions, edges)
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
    line: &str,
    prefix: &str,
) -> bool
{
    line.strip_prefix(prefix)
        .is_some_and(|value| !value.trim().is_empty())
}

/// Return whether the `- input recursion:` bullet satisfies gandr's policy.
fn input_recursion_is_valid(
    cx: &LateContext<'_>,
    def_id: LocalDefId,
    line: &str,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> bool
{
    if line == "- input recursion: none." {
        return !scc_has_input_derived_recursive_call(cx, scc, functions, edges);
    }
    allows_model_checker_input_recursion(cx, def_id)
        && required_bullet_has_value(line, "- input recursion:")
}

/// Return whether any call edge inside `scc` passes caller-input-derived data.
fn scc_has_input_derived_recursive_call(
    cx: &LateContext<'_>,
    scc: &[LocalDefId],
    functions: &HashMap<LocalDefId, FunctionNode>,
    edges: &HashMap<LocalDefId, Vec<CallEdge>>,
) -> bool
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
                    return true;
                };
                if expr_contains_derived_binding(cx, &derived, expr) {
                    return true;
                }
            }
        }
    }
    false
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
        local: &'tcx rustc_hir::LetStmt<'tcx>,
    )
    {
        if let Some(init) = local.init
            && expr_contains_derived_binding(self.cx, self.derived, init)
        {
            self.changed |= mark_pattern_bindings(local.pat, self.derived);
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
                if expr_contains_derived_binding(self.cx, self.derived, let_expr.init) {
                    self.changed |= mark_pattern_bindings(let_expr.pat, self.derived);
                }
            },
            | ExprKind::Match(scrutinee, arms, _) => {
                if expr_contains_derived_binding(self.cx, self.derived, scrutinee) {
                    for arm in arms {
                        self.changed |= mark_pattern_bindings(arm.pat, self.derived);
                    }
                }
            },
            | ExprKind::Assign(target, value, _) => {
                if expr_contains_derived_binding(self.cx, self.derived, value) {
                    self.changed |= mark_local_references(target, self.cx, self.derived);
                }
            },
            | ExprKind::AssignOp(_, target, value) => {
                if expr_contains_derived_binding(self.cx, self.derived, target)
                    || expr_contains_derived_binding(self.cx, self.derived, value)
                {
                    self.changed |= mark_local_references(target, self.cx, self.derived);
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
) -> bool
{
    let mut bindings = Vec::new();
    collect_pattern_bindings(pat, &mut bindings);
    let mut changed = false;
    for binding in bindings {
        changed |= derived.insert(binding);
    }
    changed
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
) -> bool
{
    let mut visitor = DerivedBindingFinder {
        cx,
        derived,
        found: false,
    };
    visitor.visit_expr(expr);
    visitor.found
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
) -> bool
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
    changed
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
/// `gandr_core_checker::checker::Rec`.
///
/// # Contract
///
/// - requires: `def_id` names a local function definition visited by the late
///   lint pass.
/// - ensures: returns `true` exactly when the item is an inherent method whose
///   fully peeled receiver `ADT` is `gandr_core_checker::checker::Rec`.
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
) -> bool
{
    if cx.tcx.inherent_impl_of_assoc(def_id.to_def_id()).is_none() {
        return false;
    }
    let assoc_item = cx.tcx.associated_item(def_id.to_def_id());
    if !assoc_item.is_method() {
        return false;
    }

    let fn_sig = cx
        .tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .skip_binder();
    let Some(receiver_ty) = fn_sig.inputs().first().copied()
    else {
        return false;
    };
    let receiver_ty = peel_reference_ty(normalize_middle_ty(cx, receiver_ty));
    let Some(adt_def) = receiver_ty.ty_adt_def()
    else {
        return false;
    };
    is_model_checker_rec_path(&absolute_def_path(cx, adt_def.did()))
}

/// Peel reference layers from a receiver type before checking the receiver ADT.
fn peel_reference_ty(mut ty: rustc_ty::Ty<'_>) -> rustc_ty::Ty<'_>
{
    loop {
        match ty.kind() {
            | rustc_ty::Ref(_, inner, _) => ty = *inner,
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
fn is_model_checker_rec_path(path: &str) -> bool
{
    path == "gandr_core_checker::checker::Rec"
}

/// Check every input and explicit output type in one function declaration.
fn check_fn_decl<'tcx>(
    cx: &LateContext<'tcx>,
    fn_decl: &FnDecl<'tcx>,
    fn_sig: rustc_ty::FnSig<'tcx>,
) -> bool
{
    let mut emitted = false;
    for (input, semantic_ty) in fn_decl.inputs.iter().zip(fn_sig.inputs()) {
        emitted |= check_ty(cx, input, *semantic_ty);
    }
    if let FnRetTy::Return(output) = fn_decl.output {
        emitted |= check_ty(cx, output, fn_sig.output());
    }
    emitted
}

/// Check a HIR type, descending through structural type constructors while
/// asking rustc's semantic type for alias-expanded primitive reachability.
fn check_ty<'tcx, Unambig>(
    cx: &LateContext<'tcx>,
    ty: &'tcx Ty<'tcx, Unambig>,
    semantic_ty: rustc_ty::Ty<'tcx>,
) -> bool
{
    let semantic_ty = normalize_middle_ty(cx, semantic_ty);
    if !middle_ty_contains_primitive(cx, semantic_ty) {
        return check_hir_ty_for_primitive(cx, ty);
    }

    match ty.kind {
        | TyKind::Slice(inner) | TyKind::Array(inner, _) => match semantic_ty.kind() {
            | rustc_ty::Slice(semantic_inner) | rustc_ty::Array(semantic_inner, _) => {
                check_ty(cx, inner, *semantic_inner)
            },
            | _ => emit_ty_primitive(cx, ty),
        },
        | TyKind::Ptr(mut_ty) => match semantic_ty.kind() {
            | rustc_ty::RawPtr(semantic_inner, _) => check_ty(cx, mut_ty.ty, *semantic_inner),
            | _ => emit_ty_primitive(cx, ty),
        },
        | TyKind::Ref(_, mut_ty) => match semantic_ty.kind() {
            | rustc_ty::Ref(_, semantic_inner, _) => check_ty(cx, mut_ty.ty, *semantic_inner),
            | _ => emit_ty_primitive(cx, ty),
        },
        | TyKind::FnPtr(fn_ptr) => match semantic_ty.kind() {
            | rustc_ty::FnPtr(sig_tys, header) => {
                check_fn_decl(cx, fn_ptr.decl, sig_tys.with(*header).skip_binder())
            },
            | _ => emit_ty_primitive(cx, ty),
        },
        | TyKind::Tup(types) => match semantic_ty.kind() {
            | rustc_ty::Tuple(semantic_types) if semantic_types.len() == types.len() => {
                let mut emitted = false;
                for (inner, semantic_inner) in types.iter().zip(semantic_types.iter()) {
                    emitted |= check_ty(cx, inner, semantic_inner);
                }
                emitted
            },
            | _ => emit_ty_primitive(cx, ty),
        },
        | TyKind::Path(ref qpath) => check_path_ty(cx, ty, qpath, semantic_ty),
        | TyKind::Pat(inner, _) | TyKind::FieldOf(inner, _) => check_ty(cx, inner, semantic_ty),
        | TyKind::OpaqueDef(opaque) => check_opaque_ty_for_primitive(cx, opaque),
        | TyKind::InferDelegation(_)
        | TyKind::UnsafeBinder(_)
        | TyKind::Never
        | TyKind::TraitAscription(_)
        | TyKind::TraitObject(..)
        | TyKind::Err(_)
        | TyKind::Infer(_) => false,
    }
}

/// Check HIR syntax for direct primitive reachability when semantic type
/// normalization hides the declared type, as async function signatures do.
fn check_hir_ty_for_primitive<'tcx, Unambig>(
    cx: &LateContext<'tcx>,
    ty: &'tcx Ty<'tcx, Unambig>,
) -> bool
{
    match ty.kind {
        | TyKind::Slice(inner) | TyKind::Array(inner, _) => check_hir_ty_for_primitive(cx, inner),
        | TyKind::Ptr(mut_ty) | TyKind::Ref(_, mut_ty) => check_hir_ty_for_primitive(cx, mut_ty.ty),
        | TyKind::FnPtr(fn_ptr) => check_hir_fn_decl_for_primitive(cx, fn_ptr.decl),
        | TyKind::Tup(types) => {
            let mut emitted = false;
            for inner in types {
                emitted |= check_hir_ty_for_primitive(cx, inner);
            }
            emitted
        },
        | TyKind::Path(ref qpath) => match cx.qpath_res(qpath, ty.hir_id) {
            | Res::PrimTy(primitive) if is_disallowed_primitive(primitive) => {
                emit_ty_primitive(cx, ty)
            },
            | _ => false,
        },
        | TyKind::Pat(inner, _) | TyKind::FieldOf(inner, _) => {
            check_hir_ty_for_primitive(cx, inner)
        },
        | TyKind::OpaqueDef(opaque) => check_opaque_ty_for_primitive(cx, opaque),
        | TyKind::InferDelegation(_)
        | TyKind::UnsafeBinder(_)
        | TyKind::Never
        | TyKind::TraitAscription(_)
        | TyKind::TraitObject(..)
        | TyKind::Err(_)
        | TyKind::Infer(_) => false,
    }
}

/// Check the explicit `Output` type hidden inside an async function's opaque
/// future return type.
fn check_opaque_ty_for_primitive<'tcx>(
    cx: &LateContext<'tcx>,
    opaque: &'tcx OpaqueTy<'tcx>,
) -> bool
{
    let Some(trait_ref) = future_trait_ref(cx, opaque)
    else {
        return false;
    };
    let Some(output) = future_output_ty(trait_ref)
    else {
        return false;
    };
    check_hir_ty_for_primitive(cx, output)
}

/// Return the `Future` bound on an opaque async return type, when present.
fn future_trait_ref<'tcx>(
    cx: &LateContext<'tcx>,
    opaque: &'tcx OpaqueTy<'tcx>,
) -> Option<&'tcx TraitRef<'tcx>>
{
    if let Some(trait_ref) = opaque.bounds.iter().find_map(|bound| match bound {
        | GenericBound::Trait(poly) => Some(&poly.trait_ref),
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
        && let [constraint] = args.constraints
        && constraint.ident.name == sym::Output
        && let Some(output) = constraint.ty()
    {
        return Some(output);
    }
    None
}

/// Check a HIR-only function pointer signature for primitive exposure.
fn check_hir_fn_decl_for_primitive<'tcx>(
    cx: &LateContext<'tcx>,
    fn_decl: &FnDecl<'tcx>,
) -> bool
{
    let mut emitted = false;
    for input in fn_decl.inputs {
        emitted |= check_hir_ty_for_primitive(cx, input);
    }
    if let FnRetTy::Return(output) = fn_decl.output {
        emitted |= check_hir_ty_for_primitive(cx, output);
    }
    emitted
}

/// Check a path type using its fully substituted semantic type, while retaining
/// the most precise available HIR span for diagnostics.
fn check_path_ty<'tcx, Unambig>(
    cx: &LateContext<'tcx>,
    ty: &'tcx Ty<'tcx, Unambig>,
    qpath: &QPath<'tcx>,
    semantic_ty: rustc_ty::Ty<'tcx>,
) -> bool
{
    if check_generic_type_args(cx, qpath, semantic_ty) {
        return true;
    }
    emit_ty_primitive(cx, ty)
}

/// Check type arguments on a path and return whether a diagnostic was emitted.
fn check_generic_type_args<'tcx>(
    cx: &LateContext<'tcx>,
    qpath: &QPath<'tcx>,
    semantic_ty: rustc_ty::Ty<'tcx>,
) -> bool
{
    let Some(args) = last_segment_args(qpath)
    else {
        return false;
    };
    let semantic_type_args = semantic_type_args(cx, semantic_ty);
    let mut semantic_type_args = semantic_type_args.iter();
    let mut emitted = false;
    for arg in args.args {
        if let GenericArg::Type(ty) = arg
            && let Some(semantic_arg) = semantic_type_args.next()
        {
            emitted |= check_ty(cx, ty, *semantic_arg);
        }
    }
    emitted
}

/// Return substituted semantic type arguments represented by a path type.
fn semantic_type_args<'tcx>(
    cx: &LateContext<'tcx>,
    semantic_ty: rustc_ty::Ty<'tcx>,
) -> Vec<rustc_ty::Ty<'tcx>>
{
    let semantic_ty = normalize_middle_ty(cx, semantic_ty);
    match semantic_ty.kind() {
        | rustc_ty::Adt(_, args) => args.iter().filter_map(rustc_ty::GenericArg::as_type).collect(),
        | rustc_ty::Tuple(types) => types.iter().collect(),
        | _ => Vec::new(),
    }
}

/// Return the last path segment's generic arguments, if any.
fn last_segment_args<'hir>(qpath: &QPath<'hir>) -> Option<&'hir rustc_hir::GenericArgs<'hir>>
{
    match qpath {
        | QPath::Resolved(_, path) => {
            let segment = path.segments.last()?;
            segment.args
        },
        | QPath::TypeRelative(_, segment) => segment.args,
    }
}

/// Return whether `primitive` is one of gandr's banned signature primitives.
fn is_disallowed_primitive(primitive: PrimTy) -> bool
{
    matches!(
        primitive,
        PrimTy::Bool
            | PrimTy::Char
            | PrimTy::Int(_)
            | PrimTy::Uint(_)
            | PrimTy::Float(_)
            | PrimTy::Str
    )
}

/// Return whether an ADT is a semantic wrapper boundary for type-boundary
/// linting.
fn is_semantic_boundary_adt(adt: rustc_ty::AdtDef<'_>) -> bool
{
    adt.did().is_local() && adt.repr().transparent()
}

/// Return whether a substituted semantic type contains a disallowed primitive
/// before a nominal gandr boundary.
fn middle_ty_contains_primitive<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_ty::Ty<'tcx>,
) -> bool
{
    let ty = normalize_middle_ty(cx, ty);
    match ty.kind() {
        | rustc_ty::Bool
        | rustc_ty::Char
        | rustc_ty::Int(_)
        | rustc_ty::Uint(_)
        | rustc_ty::Float(_)
        | rustc_ty::Str => true,
        | rustc_ty::Array(inner, _)
        | rustc_ty::Pat(inner, _)
        | rustc_ty::Slice(inner)
        | rustc_ty::RawPtr(inner, _)
        | rustc_ty::Ref(_, inner, _) => middle_ty_contains_primitive(cx, *inner),
        | rustc_ty::Tuple(types) => types
            .iter()
            .any(|inner| middle_ty_contains_primitive(cx, inner)),
        | rustc_ty::FnPtr(sig_tys, header) => {
            let sig = sig_tys.with(*header).skip_binder();
            sig.inputs()
                .iter()
                .any(|input| middle_ty_contains_primitive(cx, *input))
                || middle_ty_contains_primitive(cx, sig.output())
        },
        | rustc_ty::Adt(adt, _) if is_semantic_boundary_adt(*adt) => false,
        | rustc_ty::Adt(_, args) => args
            .iter()
            .filter_map(rustc_ty::GenericArg::as_type)
            .any(|arg_ty| middle_ty_contains_primitive(cx, arg_ty)),
        | _ => false,
    }
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

/// Emit the primitive signature diagnostic at a HIR type span and return
/// `true`.
fn emit_ty_primitive<Unambig>(
    cx: &LateContext<'_>,
    ty: &Ty<'_, Unambig>,
) -> bool
{
    emit_primitive(cx, ty.span);
    true
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
            is_model_checker_rec_path("gandr_core_checker::checker::Rec"),
            "the exact checker receiver path is accepted"
        );
        assert!(
            !is_model_checker_rec_path("gandr_core_checker::mark::Rec"),
            "a same-named type in another module is rejected"
        );
        assert!(
            !is_model_checker_rec_path("termination::gandr_core_checker::checker::Rec"),
            "a prefixed lookalike path is rejected"
        );
    }
}
