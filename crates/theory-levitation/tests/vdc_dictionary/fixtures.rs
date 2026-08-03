//! **Real-structure fixtures** for the dictionary suite: a `Nat` signature with
//! `plus` / `double` rewrite rules, renamed copies for signature morphisms,
//! relation interfaces, clause cells, and the corpora replay-equivalence is
//! decided over.
//!
//! Every description here is Field-free and parameter-free (or a builtin
//! retrofit), so the fixtures are built without naming
//! `gandr_core_checker::grade::Grade` — the crate is a normal, not a dev,
//! dependency and is not in scope for an integration test.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::BridgeArity;
use gandr_theory_levitation::CellFace;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::NumeralCount;
use gandr_theory_levitation::OpDesc;
use gandr_theory_levitation::SortRef;
use gandr_theory_levitation::SurfaceSpan;
use gandr_theory_levitation::wellformed::derive_cell_var_meta;

use super::harness::BaseInstance;
use super::harness::Cell;
use super::harness::CellClause;
use super::harness::CellKind;
use super::harness::FactorRoute;
use super::harness::LooseArrow;
use super::harness::LooseInstance;
use super::harness::Relation;
use super::harness::SigMorphism;
use super::harness::SigObj;

// ----------------------------------------------------------------------
// Ground Nat terms and small term constructors
// ----------------------------------------------------------------------

/// The numeral `n` as `Succⁿ(Zero)`.
pub fn nat(n: NumeralCount) -> FreeTerm
{
    (0 .. usize::from(n)).fold(zero(), |acc, _unused| succ(acc))
}
/// A single-generator relation over `Nat`, whose lone generating face is
/// `plus(x, Zero) ~> x` (a real in-signature face).
pub fn unary_relation(name: NameRef<'_>) -> Rc<Relation>
{
    let generator = face(
        FreeTerm::op("plus", [var("x".into()), zero()]),
        var("x".into()),
    );
    Rc::new(Relation {
        name: name.into(),
        src: nat_sig(),
        tgt: nat_sig(),
        gens: vec![generator],
    })
}
/// A **linear single-clause cell** `dom_rel ⇒ cod_rel` matching generator `0`
/// and emitting generator `0` with output variable `x` bound to the given
/// template (over the namespaced input variable `p0.x`).
pub fn relabel_cell(
    dom_rel: Rc<Relation>,
    cod_rel: Rc<Relation>,
    template_for_x: FreeTerm,
) -> Cell
{
    let mut emit_templates: BTreeMap<Name, FreeTerm> = BTreeMap::new();
    emit_templates.insert("x".into(), template_for_x);
    Cell {
        dom: vec![loose_of(dom_rel)],
        cod: loose_of(cod_rel),
        left_frame: SigMorphism::identity(&nat_sig()),
        right_frame: SigMorphism::identity(&nat_sig()),
        kind: CellKind::Clauses(vec![CellClause {
            matches: vec![0.into()],
            emit: vec![(0.into(), emit_templates)],
        }]),
    }
}

/// The loose arrow over a single named relation, framed by identities.
pub fn loose_of(rel: Rc<Relation>) -> LooseArrow
{
    LooseArrow::of_relation(rel)
}
/// The identity cell on a single loose arrow.
pub fn ident_cell(loose: LooseArrow) -> Cell
{
    Cell {
        dom: vec![loose.clone()],
        cod: loose,
        left_frame: SigMorphism::identity(&nat_sig()),
        right_frame: SigMorphism::identity(&nat_sig()),
        kind: CellKind::Ident,
    }
}
mod nat_fixture
{
    use super::*;

    /// The single-factor object over [`nat_desc`].
    pub fn nat_sig() -> SigObj
    {
        SigObj::single(nat_desc())
    }

    /// The `Nat` description: `Zero`, `Succ`, operations `plus` / `double`, and
    /// the three rewrite rules. Passes [`gandr_theory_levitation::check_desc`]
    /// cleanly.
    pub fn nat_desc() -> DataDesc
    {
        let cells = vec![
            // plus(Zero, n) ~> n
            face(
                FreeTerm::op("plus", [zero(), var("n".into())]),
                var("n".into()),
            ),
            // plus(Succ(m), n) ~> Succ(plus(m, n))
            face(
                FreeTerm::op("plus", [succ(var("m".into())), var("n".into())]),
                succ(FreeTerm::op("plus", [var("m".into()), var("n".into())])),
            ),
            // double(n) ~> plus(n, n)
            face(
                FreeTerm::op("double", [var("n".into())]),
                FreeTerm::op("plus", [var("n".into()), var("n".into())]),
            ),
        ];
        DataDesc::new(
            NominalId::new(0.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, "Nat", Attrs::empty()),
                CtorDesc::new("Succ", Code::var("Nat"), "Nat", Attrs::empty()),
            ],
            [
                OpDesc::new("plus", plus_arity(), Attrs::empty()),
                OpDesc::new("double", double_arity(), Attrs::empty()),
            ],
            cells,
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }
}

pub use nat_fixture::nat_desc;
pub use nat_fixture::nat_sig;
/// A small bank of **real-structure faces** over a named `Nat` signature —
/// covering a ctor rule, an op rule, a nested op rule, and a variable-only
/// rewrite — for the restriction functoriality proptests (Law 3(a)).
pub fn sample_faces(names: &NatNames) -> Vec<CellFace>
{
    let zero_t = FreeTerm::ctor(names.zero.clone(), Vec::new());
    let succ_m = FreeTerm::ctor(names.succ.clone(), [var("m".into())]);
    vec![
        // plus(Zero, x) ~> x
        face(
            FreeTerm::op(names.plus.clone(), [zero_t, var("x".into())]),
            var("x".into()),
        ),
        // double(n) ~> plus(n, n)
        face(
            FreeTerm::op(names.double.clone(), [var("n".into())]),
            FreeTerm::op(names.plus.clone(), [var("n".into()), var("n".into())]),
        ),
        // plus(Succ(m), n) ~> Succ(plus(m, n))
        face(
            FreeTerm::op(names.plus.clone(), [succ_m.clone(), var("n".into())]),
            FreeTerm::ctor(names.succ.clone(), [FreeTerm::op(names.plus.clone(), [
                var("m".into()),
                var("n".into()),
            ])]),
        ),
        // Succ(m) ~> Succ(m)  (identity-shaped, exercises ctor renaming)
        face(succ_m.clone(), succ_m),
    ]
}
/// A cell face over two terms, with the real derived per-variable metadata and
/// a throwaway provenance span.
pub fn face(
    lhs: FreeTerm,
    rhs: FreeTerm,
) -> CellFace
{
    let vars = derive_cell_var_meta(&lhs);
    CellFace::new(lhs, rhs, vars, SurfaceSpan::new(0.into(), 1.into()))
}
/// A variable term.
pub fn var(name: NameRef<'_>) -> FreeTerm
{
    FreeTerm::var(name)
}
/// The `Zero` constructor term.
pub fn zero() -> FreeTerm
{
    FreeTerm::ctor("Zero", Vec::new())
}
/// `Succ(inner)`.
pub fn succ(inner: FreeTerm) -> FreeTerm
{
    FreeTerm::ctor("Succ", [inner])
}

// ----------------------------------------------------------------------
// The Nat signature with plus / double rewrite rules
// ----------------------------------------------------------------------

/// The **renaming morphism** `f : source → target` between two `Nat`-shaped
/// objects: a valid signature morphism whose per-symbol map sends each target
/// symbol to its role-matched source symbol.
pub fn renaming(
    source_names: &NatNames,
    target_names: &NatNames,
) -> SigMorphism
{
    let mut map: BTreeMap<Name, Name> = BTreeMap::new();
    map.insert(target_names.zero.clone(), source_names.zero.clone());
    map.insert(target_names.succ.clone(), source_names.succ.clone());
    map.insert(target_names.plus.clone(), source_names.plus.clone());
    map.insert(target_names.double.clone(), source_names.double.clone());
    SigMorphism {
        src: nat_obj(source_names),
        tgt: nat_obj(target_names),
        routes: vec![FactorRoute {
            src_factor: 0.into(),
            map,
        }],
    }
}
/// The single-factor object over a named `Nat`-shaped description (no cells).
pub fn nat_obj(names: &NatNames) -> SigObj
{
    SigObj::single(nat_from_names(names, Vec::new()))
}
/// A `Nat`-shaped description with the given symbol names and (possibly empty)
/// cells. Ctor codes and op arities are role-fixed, so any two of these are
/// connected by a valid renaming.
pub fn nat_from_names(
    names: &NatNames,
    cells: Vec<CellFace>,
) -> DataDesc
{
    DataDesc::new(
        NominalId::new(0.into(), "Nat"),
        Vec::new(),
        [
            CtorDesc::new(names.zero.clone(), Code::Unit, "Nat", Attrs::empty()),
            CtorDesc::new(names.succ.clone(), Code::var("Nat"), "Nat", Attrs::empty()),
        ],
        [
            OpDesc::new(names.plus.clone(), plus_arity(), Attrs::empty()),
            OpDesc::new(names.double.clone(), double_arity(), Attrs::empty()),
        ],
        cells,
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// The `plus` bridge arity: `plus(m, n) -> q`.
fn plus_arity() -> BridgeArity
{
    BridgeArity::single_output(
        [SortRef::new("m", "Nat"), SortRef::new("n", "Nat")],
        SortRef::new("q", "Nat"),
    )
}

/// The `double` bridge arity: `double(n) -> q`.
fn double_arity() -> BridgeArity
{
    BridgeArity::single_output([SortRef::new("n", "Nat")], SortRef::new("q", "Nat"))
}

// ----------------------------------------------------------------------
// Renamed Nat copies and renaming signature morphisms (Laws 1, 3)
// ----------------------------------------------------------------------

/// The four symbol names of a `Nat`-shaped signature (two ctors, two ops).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NatNames
{
    /// The `Zero`-role constructor name (payload code `1`).
    pub zero: Name,
    /// The `Succ`-role constructor name (payload code `var`).
    pub succ: Name,
    /// The `plus`-role operation name (binary arity).
    pub plus: Name,
    /// The `double`-role operation name (unary arity).
    pub double: Name,
}

/// The `NatNames` spelling the real `Nat` signature's symbols (`Zero`, `Succ`,
/// `plus`, `double`) — for well-formedness witnesses that must exercise
/// [`gandr_theory_levitation::check_desc`] on in-signature names.
pub fn real_nat_names() -> NatNames
{
    NatNames {
        zero: "Zero".into(),
        succ: "Succ".into(),
        plus: "plus".into(),
        double: "double".into(),
    }
}

/// A `Nat`-name tuple for a tag, with role prefixes guaranteeing the four names
/// are distinct regardless of the tag.
pub fn nat_names(tag: NameRef<'_>) -> NatNames
{
    let tag = tag.as_ref();
    NatNames {
        zero: format!("Z_{tag}").into_boxed_str().into(),
        succ: format!("S_{tag}").into_boxed_str().into(),
        plus: format!("p_{tag}").into_boxed_str().into(),
        double: format!("d_{tag}").into_boxed_str().into(),
    }
}

// ----------------------------------------------------------------------
// Relation interfaces and loose arrows (Laws 2, 4, 5)
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// Clause cells (Laws 2, 4, 5)
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// Instances and corpora
// ----------------------------------------------------------------------

/// A single-factor generating loose instance binding variable `x` to `term`.
pub fn gen_x(term: FreeTerm) -> LooseInstance
{
    let mut subst: BTreeMap<Name, FreeTerm> = BTreeMap::new();
    subst.insert("x".into(), term);
    LooseInstance {
        per_factor: vec![BaseInstance::Gen {
            generator: 0.into(),
            subst,
        }],
    }
}

/// The **single-input corpus**: one input tuple per numeral `0 ..= 5`, each a
/// generating instance binding `x` to that numeral.
pub fn single_input_corpus() -> Vec<Vec<LooseInstance>>
{
    (0 ..= 5_usize)
        .map(|k| vec![gen_x(nat(k.into()))])
        .collect()
}
