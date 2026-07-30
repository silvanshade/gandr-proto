//! **Host-side well-formedness** — the stage-0 stand-in for stage-1 typing
//! (proposal-levitation.md §3, §8; VDC addendum §A).
//!
//! Stage 0 has no typed decoder, so a [`DataDesc`]'s invariants are checked by
//! this Rust pass, which produces **inspectable diagnostics** (proposal §3).
//! The checks discharge the pathological goldens of proposal §8 and the VDC
//! declined-declaration golden (addendum §A/§C):
//!
//! * a **cell face** may mention only in-signature symbols (constructor / op
//!   names of this datatype), and its right-hand side may not introduce a
//!   variable absent from its left-hand side (`desc-illformed-cell`);
//! * a **bridge arity**'s three maps must compose (`desc-arity-mismatch`);
//! * a symbol may not **declare derived metadata**: the per-variable variance /
//!   linearity of a cell face is *derived* ([`derive_cell_var_meta`]), so an
//!   attribute Σ that names it is declined (the VDC declined-declaration
//!   golden).
//!
//! The higher-order-field decline (`desc-higher-order-field`, proposal §8) is
//! not represented here because [`crate::Code`] *cannot encode* a higher-order
//! field — that decline lands at elaboration, before a `DataDesc` exists.

use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::boundary::StringText;

use crate::arity::BridgeArity;
use crate::boundary::DiagnosticMessage;
use crate::cell::CellFace;
use crate::cell::CellVarMeta;
use crate::cell::FreeTerm;
use crate::cell::Variance;
use crate::code::Attrs;
use crate::code::Name;
use crate::desc::DataDesc;
use crate::desc::SurfaceSpan;

/// The reserved-derived metadata marker names an attribute Σ may **not**
/// declare: they name the [`CellVarMeta`] fields, which are *derived* from the
/// faces, never declared (VDC addendum §A).
pub const RESERVED_DERIVED_MARKERS: [&str; 3] = ["variance", "linear", "linearity"];

/// The classification of a well-formedness failure (for programmatic matching
/// by consumers and goldens).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WfKind
{
    /// A cell face mentions a constructor / operation not in this datatype's
    /// signature.
    OutOfSignatureCell,
    /// A cell face's right-hand side introduces a variable absent from its
    /// left-hand side.
    UnboundRhsVariable,
    /// A bridge arity's maps do not compose.
    ArityDoesNotCompose,
    /// A symbol declares derived per-variable metadata (variance / linearity).
    DeclaresDerivedMetadata,
}

/// One well-formedness **diagnostic** — an inspectable failure with a message
/// and (when known) a surface span.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a diagnostic is exactly its {kind, message, optional span}; it is produced by the checks and read by consumers"
)]
pub struct WfDiagnostic
{
    /// The failure classification.
    pub kind: WfKind,
    /// A human-readable description of the failure.
    pub message: DiagnosticMessage,
    /// The surface span the failure was located at, when known.
    pub span: Option<SurfaceSpan>,
}

impl WfDiagnostic
{
    /// A diagnostic of the given kind, message, and optional span.
    #[inline]
    #[must_use]
    pub fn new(
        kind: WfKind,
        message: DiagnosticMessage,
        span: Option<SurfaceSpan>,
    ) -> Self
    {
        Self {
            kind,
            message,
            span,
        }
    }
}

/// **Check** a description, returning one diagnostic per well-formedness
/// failure (an empty vector means well-formed).
///
/// # Contract
/// - requires: none.
/// - ensures: returns every failure of the §"well-formedness" rules, in a
///   deterministic order (attribute declines, then per-cell checks, then
///   per-arity checks); an empty result witnesses a well-formed description.
/// - fails: never; total on any (even mis-built) description.
///
/// # Adequacy
/// - hypothesis: L3 — a clean description passes; an out-of-signature cell, a
///   non-composing arity, and a reserved-derived attribute each surface exactly
///   their diagnostic.
/// - witness: `wellformed::tests::*`.
#[inline]
#[must_use]
pub fn check_desc(desc: &DataDesc) -> Vec<WfDiagnostic>
{
    let mut diagnostics = Vec::new();

    // Reserved-derived metadata: no attribute Σ may declare it (VDC §A golden).
    check_attrs(
        &desc.attrs,
        &desc.id.name,
        "the datatype".into(),
        &mut diagnostics,
    );
    for ctor in &desc.ctors {
        check_attrs(
            &ctor.attrs,
            &ctor.name,
            "the constructor".into(),
            &mut diagnostics,
        );
    }
    for param in &desc.params {
        check_attrs(
            &param.attrs,
            &param.name,
            "the parameter".into(),
            &mut diagnostics,
        );
    }
    for op in &desc.ops {
        check_attrs(
            &op.attrs,
            &op.name,
            "the operation".into(),
            &mut diagnostics,
        );
    }

    // Cell faces: in-signature symbols and no fresh right-hand-side variable.
    let signature = signature_names(desc);
    for cell in &desc.cells {
        check_cell(cell, &signature, &mut diagnostics);
    }

    // Bridge arities: the three maps compose.
    for op in &desc.ops {
        check_arity(&op.arity, op.name.as_ref().into(), &mut diagnostics);
    }

    diagnostics
}

/// Append a `DeclaresDerivedMetadata` diagnostic for each reserved-derived
/// marker in `attrs`.
fn check_attrs(
    attrs: &Attrs,
    owner: &Name,
    role: StringText<'_>,
    diagnostics: &mut Vec<WfDiagnostic>,
)
{
    for marker in RESERVED_DERIVED_MARKERS {
        if bool::from(attrs.contains(marker.into())) {
            diagnostics.push(WfDiagnostic::new(
                WfKind::DeclaresDerivedMetadata,
                format!(
                    "{} `{owner}` declares reserved derived metadata `{marker}`: cell \
                     variable variance and linearity are derived from the faces, never declared \
                     (proposal-levitation-addendum-vdc.md §A)",
                    role.as_ref()
                )
                .into(),
                None,
            ));
        }
    }
}

/// The set of in-signature symbol names: every constructor and operation of
/// `desc`.
fn signature_names(desc: &DataDesc) -> Vec<Name>
{
    let mut names: Vec<Name> = Vec::new();
    for ctor in &desc.ctors {
        names.push(ctor.name.clone());
    }
    for op in &desc.ops {
        names.push(op.name.clone());
    }
    names
}

/// Check one cell face: every applied symbol is in-signature and the
/// right-hand side introduces no fresh variable.
fn check_cell(
    cell: &CellFace,
    signature: &[Name],
    diagnostics: &mut Vec<WfDiagnostic>,
)
{
    check_free_term_symbols(&cell.lhs, signature, cell.provenance, diagnostics);
    check_free_term_symbols(&cell.rhs, signature, cell.provenance, diagnostics);

    let lhs_vars = cell.lhs.collect_vars();
    let rhs_vars = cell.rhs.collect_vars();
    for var in &rhs_vars {
        if !lhs_vars.contains(var) {
            diagnostics.push(WfDiagnostic::new(
                WfKind::UnboundRhsVariable,
                format!(
                    "cell rule's right-hand side introduces variable `{var}` not bound by its \
                     left-hand side"
                )
                .into(),
                Some(cell.provenance),
            ));
        }
    }
}

/// Append an `OutOfSignatureCell` diagnostic for each applied symbol of `term`
/// not present in `signature`.
fn check_free_term_symbols(
    term: &FreeTerm,
    signature: &[Name],
    span: SurfaceSpan,
    diagnostics: &mut Vec<WfDiagnostic>,
)
{
    let mut stack = vec![term];
    while let Some(current) = stack.pop() {
        match *current {
            | FreeTerm::Var(_) => {},
            | FreeTerm::Ctor(ref name, ref args) | FreeTerm::Op(ref name, ref args) => {
                if !signature.contains(name) {
                    diagnostics.push(WfDiagnostic::new(
                        WfKind::OutOfSignatureCell,
                        format!(
                            "cell rule mentions symbol `{name}` not in the datatype's signature"
                        )
                        .into(),
                        Some(span),
                    ));
                }
                for arg in args.iter().rev() {
                    stack.push(arg);
                }
            },
        }
    }
}

/// Check that a bridge arity's three maps compose (proposal §4.2).
fn check_arity(
    arity: &BridgeArity,
    owner: NameRef<'_>,
    diagnostics: &mut Vec<WfDiagnostic>,
)
{
    let mut fail = |reason: String| {
        diagnostics.push(WfDiagnostic::new(
            WfKind::ArityDoesNotCompose,
            format!(
                "operation `{}`'s bridge arity does not compose: {reason}",
                owner.as_ref()
            )
            .into(),
            None,
        ));
    };

    // `t : I → B` has one entry per monomial, matching `|I| = factors.len()`.
    if arity.dest.len() != arity.factors.len() {
        fail(format!(
            "|I| mismatch: {} monomials by `factors` but {} by `dest`",
            arity.factors.len(),
            arity.dest.len()
        ));
    }
    // `s : J → A` has one entry per factor, matching `Σ factors`.
    let total_factors: usize = arity
        .factors
        .iter()
        .map(|&count| usize::try_from(count).unwrap_or(usize::MAX))
        .sum();
    if arity.source.len() != total_factors {
        fail(format!(
            "|J| mismatch: Σ factors = {total_factors} but `source` has {} entries",
            arity.source.len()
        ));
    }
    // `s` lands in `A`, `t` lands in `B`.
    for &input in &arity.source {
        if usize::try_from(input).unwrap_or(usize::MAX) >= arity.inputs.len() {
            fail(format!(
                "`source` reads input {input} but there are only {} inputs",
                arity.inputs.len()
            ));
        }
    }
    for &output in &arity.dest {
        if usize::try_from(output).unwrap_or(usize::MAX) >= arity.outputs.len() {
            fail(format!(
                "`dest` feeds output {output} but there are only {} outputs",
                arity.outputs.len()
            ));
        }
    }
}

/// **Derive** the per-variable [`CellVarMeta`] for a face's left-hand-side
/// variables (VDC addendum §A).
///
/// Each variable's variance is the stage-0 constant [`Variance::Producer`], and
/// its linearity is whether it occurs exactly once in the left-hand side.
///
/// This is the derivation the elaborator runs to populate
/// [`CellFace::vars`](crate::CellFace); the metadata is derived here, never
/// declared (a surface attempt to declare it is declined by [`check_desc`]).
///
/// # Contract
/// - ensures: one [`CellVarMeta`] per *distinct* left-hand-side variable, in
///   first-occurrence order; `linear` is `true` exactly when the variable
///   occurs once.
/// - fails: never.
#[inline]
#[must_use]
pub fn derive_cell_var_meta(lhs: &FreeTerm) -> Vec<CellVarMeta>
{
    let occurrences = lhs.collect_vars();
    let mut meta: Vec<CellVarMeta> = Vec::new();
    for var in &occurrences {
        if meta.iter().any(|existing| existing.var == *var) {
            continue;
        }
        let count = occurrences.iter().filter(|&other| other == var).count();
        meta.push(CellVarMeta::new(
            var.clone(),
            Variance::Producer,
            (count == 1).into(),
        ));
    }
    meta
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::arity::SortRef;
    use crate::cell::CellFace;
    use crate::code::Attr;
    use crate::code::Attrs;
    use crate::code::Code;
    use crate::desc::CtorDesc;
    use crate::desc::DeclPolarity;
    use crate::desc::NominalId;
    use crate::desc::OpDesc;

    #[test]
    fn a_clean_description_passes()
    {
        let face = CellFace::new(
            FreeTerm::op("id", [FreeTerm::ctor("Zero", Vec::new())]),
            FreeTerm::ctor("Zero", Vec::new()),
            Vec::new(),
            SurfaceSpan::new(0.into(), 1.into()),
        );
        let op = OpDesc::new(
            "id",
            BridgeArity::single_output([SortRef::new("m", "Nat")], SortRef::new("q", "Nat")),
            Attrs::empty(),
        );
        assert!(
            check_desc(&nat_with(vec![face], vec![op], Attrs::empty())).is_empty(),
            "a well-formed description has no diagnostics"
        );
    }
    #[test]
    fn an_out_of_signature_cell_is_declined()
    {
        // `bogus(x) ~> x`: `bogus` is not a constructor or op of `Nat`.
        let face = CellFace::new(
            FreeTerm::op("bogus", [FreeTerm::var("x")]),
            FreeTerm::var("x"),
            Vec::new(),
            SurfaceSpan::new(0.into(), 1.into()),
        );
        let diagnostics = check_desc(&nat_with(vec![face], Vec::new(), Attrs::empty()));
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.kind == WfKind::OutOfSignatureCell
                    && diag.message.as_ref().contains("bogus")),
            "an out-of-signature symbol is declined"
        );
    }
    #[test]
    fn a_fresh_right_hand_side_variable_is_declined()
    {
        // `Succ(x) ~> y`: `y` is unbound.
        let face = CellFace::new(
            FreeTerm::ctor("Succ", [FreeTerm::var("x")]),
            FreeTerm::var("y"),
            Vec::new(),
            SurfaceSpan::new(0.into(), 1.into()),
        );
        let diagnostics = check_desc(&nat_with(vec![face], Vec::new(), Attrs::empty()));
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.kind == WfKind::UnboundRhsVariable
                    && diag.message.as_ref().contains('y')),
            "a fresh right-hand-side variable is declined"
        );
    }
    #[test]
    fn a_non_composing_arity_is_declined()
    {
        // `dest` feeds output 5 but there are no outputs.
        let arity = BridgeArity::new(
            [SortRef::new("m", "Nat")],
            [1u32],
            [0u32],
            [5u32],
            Vec::new(),
        );
        let op = OpDesc::new("bad", arity, Attrs::empty());
        let diagnostics = check_desc(&nat_with(Vec::new(), vec![op], Attrs::empty()));
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.kind == WfKind::ArityDoesNotCompose),
            "a non-composing arity is declined"
        );
    }
    #[test]
    fn declaring_derived_metadata_is_declined()
    {
        // The VDC declined-declaration golden: a constructor declares the
        // reserved-derived marker `variance`.
        let attrs = Attrs::new([Attr::marker("variance")]);
        let diagnostics = check_desc(&nat_with(Vec::new(), Vec::new(), attrs));
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.kind == WfKind::DeclaresDerivedMetadata
                    && diag.message.as_ref().contains("variance")),
            "declaring derived variance metadata is declined"
        );
    }
    /// A minimal `Nat`-like description with the given cells / ops / attrs.
    fn nat_with(
        cells: Vec<CellFace>,
        ops: Vec<OpDesc>,
        attrs: Attrs,
    ) -> DataDesc
    {
        DataDesc::new(
            NominalId::new(0.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, None, Attrs::empty()),
                CtorDesc::new("Succ", Code::Var, None, attrs),
            ],
            ops,
            cells,
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    #[test]
    fn cell_var_meta_derivation_reads_variance_and_linearity()
    {
        // `f(x, g(x))`: `x` occurs twice (non-linear), variance constant Producer.
        let lhs = FreeTerm::op("f", [
            FreeTerm::var("x"),
            FreeTerm::ctor("g", [FreeTerm::var("x")]),
        ]);
        let meta = derive_cell_var_meta(&lhs);
        assert_eq!(1, meta.len(), "one distinct variable");
        assert_eq!("x", meta[0].var.as_ref(), "the variable is `x`");
        assert_eq!(
            Variance::Producer,
            meta[0].variance,
            "variance is the stage-0 constant"
        );
        assert!(
            !bool::from(meta[0].linear),
            "a twice-occurring variable is non-linear"
        );

        // `Succ(n)`: `n` occurs once (linear).
        let linear = derive_cell_var_meta(&FreeTerm::ctor("Succ", [FreeTerm::var("n")]));
        assert!(
            bool::from(linear[0].linear),
            "a once-occurring variable is linear"
        );
    }
}
