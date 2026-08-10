//! The **F0 test-side realization** of the virtual double category the rewrite
//! layer is specified to be (the VDC reflection design's §2).
//!
//! # What is real, and what is a stand-in
//!
//! Objects, tight arrows, and restriction are built **directly on the landed
//! stage-0 structures** of `gandr-theory-levitation`: a [`SigObj`] is a tuple
//! of real [`SignDesc`]s, a tight arrow renames real ctor / op symbols,
//! restriction recomputes real [`RuleFace`]s through the real
//! [`derive_cell_var_meta`]. The derivation / cell machinery ([`Cell`],
//! [`replay`], [`graft`]), by contrast, is a **test-side stand-in** beside
//! the landed `gandr-theory-computads` (the fusion engine) and
//! `gandr-theory-virtual-doctrines` (the reflection face), which now carry the
//! real `Tracelet` and replay checker this stand-in models; the object /
//! tight-arrow / restriction realization here is the stage-0 reading the suite
//! pins, and the law obligations this
//! suite locates become the cell store's invariants.
//!
//! # Determinism
//!
//! Every substitution and environment is a [`BTreeMap`] iterated in sorted key
//! order; every enumeration (positions, one-step rewrites, path search) is a
//! deterministic left-to-right walk. There is no [`std::collections::HashMap`]
//! in harness state, so replay and equality are reproducible.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use gandr_theory_levitation::BridgeArity;
use gandr_theory_levitation::CellEquivalence;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::DescriptorFactorCount;
use gandr_theory_levitation::DescriptorFactorIndex;
use gandr_theory_levitation::DiagnosticMessage;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::GeneratorIndex;
use gandr_theory_levitation::LooseInstanceEquality;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;
use gandr_theory_levitation::PatternMatch;
use gandr_theory_levitation::RewriteDepth;
use gandr_theory_levitation::RuleFace;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::SymbolPresence;
use gandr_theory_levitation::TermPositionIndex;
use gandr_theory_levitation::wellformed::derive_cell_var_meta;

// ======================================================================
// Objects: SigObj (a tuple of described signatures; empty = terminal)
// ======================================================================

/// A **VDC object**: a finite product of described signatures (proposal §2,
/// "objects = a `SignDesc` or a finite product of them").
///
/// The empty tuple is the terminal object `⊤`; the chosen binary product is
/// factor concatenation ([`SigObj::product`]). Object identity is structural
/// equality on the underlying [`SignDesc`]s.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigObj
{
    /// The signature factors, left to right.
    pub factors: Vec<SignDesc>,
}

impl SigObj
{
    /// A one-factor object over a single description.
    pub fn single(desc: SignDesc) -> Self
    {
        Self {
            factors: vec![desc],
        }
    }

    /// The terminal object `⊤` — the empty tuple.
    pub fn terminal() -> Self
    {
        Self {
            factors: Vec::new(),
        }
    }

    /// The chosen binary product `A × B` — factor concatenation.
    pub fn product(
        left: &Self,
        right: &Self,
    ) -> Self
    {
        let mut factors = left.factors.clone();
        factors.extend(right.factors.iter().cloned());
        Self { factors }
    }

    /// The number of factors (`0` for `⊤`).
    pub fn arity(&self) -> DescriptorFactorCount
    {
        self.factors.len().into()
    }
}

// ======================================================================
// Tight arrows: signature morphisms as per-target-factor symbol renamings
// ======================================================================

/// One **target factor's routing**: which source factor it is drawn from, plus
/// the renaming of that target factor's ctor / op symbols to symbols of the
/// routed source factor (proposal §2, "tight arrows are term substitutions",
/// realized as renamings at stage 0).
///
/// The `map` sends *target* symbol names to *source* symbol names — the
/// contravariant direction of the term action ([`apply_term`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorRoute
{
    /// The source factor index this target factor is routed from.
    pub src_factor: DescriptorFactorIndex,
    /// The renaming: every ctor and op name of the target factor mapped to a
    /// symbol of the routed source factor.
    pub map: BTreeMap<Name, Name>,
}

/// A **tight arrow** `f : A → B`: one [`FactorRoute`] per *target* factor
/// (proposal §2). Its term action sends `B`-terms to `A`-terms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigMorphism
{
    /// The source object `A`.
    pub src: SigObj,
    /// The target object `B`.
    pub tgt: SigObj,
    /// One route per target factor of `B`.
    pub routes: Vec<FactorRoute>,
}

/// The identity renaming on a description (every symbol to itself).
fn identity_map(desc: &SignDesc) -> BTreeMap<Name, Name>
{
    symbol_names(desc)
        .into_iter()
        .map(|name| (name.clone(), name))
        .collect()
}
/// **Check** that a tight arrow is a valid signature morphism (proposal §2):
/// one route per target factor, every ctor / op of each target factor mapped to
/// a same-kind source symbol with **structurally identical** payload
/// ([`Code`] for ctors, `BridgeArity` for ops).
///
/// Returns one [`MorphismError`] per failure (empty = valid). The identity of
/// payloads is the stage-0 pinning of "sort- and arity-respecting";
/// derived-term translations are the F2 extension noted in the module docs.
pub fn check_morphism(morphism: &SigMorphism) -> Vec<MorphismError>
{
    let mut errors: Vec<MorphismError> = Vec::new();
    if morphism.routes.len() != usize::from(morphism.tgt.arity()) {
        errors.push(MorphismError {
            message: format!(
                "route count {} does not match target arity {}",
                morphism.routes.len(),
                morphism.tgt.arity()
            )
            .into(),
        });
        return errors;
    }
    for (factor, route) in morphism.routes.iter().enumerate() {
        let target = &morphism.tgt.factors[factor];
        let Some(source) = morphism.src.factors.get(usize::from(route.src_factor))
        else {
            errors.push(MorphismError {
                message: format!(
                    "target factor {factor} routes from absent source factor {}",
                    route.src_factor
                )
                .into(),
            });
            continue;
        };
        // Coverage: every ctor and op of the target factor is mapped.
        for symbol in symbol_names(target) {
            if !route.map.contains_key(&symbol) {
                errors.push(MorphismError {
                    message: format!("target symbol `{symbol}` is unmapped in factor {factor}")
                        .into(),
                });
            }
        }
        // Validity: each mapping respects kind and payload.
        for (target_symbol, source_symbol) in &route.map {
            check_symbol_mapping(
                target,
                source,
                target_symbol,
                source_symbol,
                factor.into(),
                &mut errors,
            );
        }
    }
    errors
}
/// The ctor and op names of a description, in declaration order (ctors then
/// ops) — the symbols a factor's renaming must cover.
pub fn symbol_names(desc: &SignDesc) -> Vec<Name>
{
    let mut names: Vec<Name> = Vec::new();
    for ctor in &desc.ctors {
        names.push(ctor.name.clone());
    }
    for op in &desc.opers {
        names.push(op.name.clone());
    }
    names
}

impl SigMorphism
{
    /// The **identity** tight arrow on an object (proposal §3, Law 1).
    pub fn identity(obj: &SigObj) -> Self
    {
        let routes = obj
            .factors
            .iter()
            .enumerate()
            .map(|(index, desc)| FactorRoute {
                src_factor: index.into(),
                map: identity_map(desc),
            })
            .collect();
        Self {
            src: obj.clone(),
            tgt: obj.clone(),
            routes,
        }
    }

    /// The `idx`-th **projection** `A₀ × … × Aₙ → Aᵢ` (proposal §2, cartesian
    /// structure).
    pub fn projection(
        product: &SigObj,
        idx: DescriptorFactorIndex,
    ) -> Self
    {
        let desc = product.factors[usize::from(idx)].clone();
        let route = FactorRoute {
            src_factor: idx,
            map: identity_map(&desc),
        };
        Self {
            src: product.clone(),
            tgt: SigObj::single(desc),
            routes: vec![route],
        }
    }

    /// The **diagonal** `A → A × A` (proposal §2, cartesian structure).
    pub fn diagonal(obj: &SigObj) -> Self
    {
        let width = usize::from(obj.arity());
        let target = SigObj::product(obj, obj);
        let routes = target
            .factors
            .iter()
            .enumerate()
            .map(|(index, desc)| FactorRoute {
                src_factor: index
                    .checked_rem(width.max(1))
                    .expect("max(1) is nonzero")
                    .into(),
                map: identity_map(desc),
            })
            .collect();
        Self {
            src: obj.clone(),
            tgt: target,
            routes,
        }
    }

    /// The **pairing** `⟨f, g⟩ : A → B × C` of two arrows with a shared source
    /// (proposal §2, cartesian structure).
    pub fn pairing(
        left: &Self,
        right: &Self,
    ) -> Self
    {
        let mut routes = left.routes.clone();
        routes.extend(right.routes.iter().cloned());
        Self {
            src: left.src.clone(),
            tgt: SigObj::product(&left.tgt, &right.tgt),
            routes,
        }
    }

    /// The unique **terminal arrow** `! : A → ⊤` (proposal §2, cartesian
    /// structure) — no target factors, hence no routes.
    pub fn terminal(obj: &SigObj) -> Self
    {
        Self {
            src: obj.clone(),
            tgt: SigObj::terminal(),
            routes: Vec::new(),
        }
    }
}

/// **Compose** two tight arrows: `compose(first, second)` is `second ∘ first`,
/// i.e. `first : A → B` then `second : B → C`, yielding `A → C`.
///
/// The term action is *contravariant*, so the composite's renaming sends a
/// `C`-symbol through `second`'s map (to `B`) and then through `first`'s map
/// (to `A`) — see [`apply_term`] and Law 1's functoriality test.
pub fn compose(
    first: &SigMorphism,
    second: &SigMorphism,
) -> SigMorphism
{
    let routes = second
        .routes
        .iter()
        .map(|outer| {
            let inner = &first.routes[usize::from(outer.src_factor)];
            let map = outer
                .map
                .iter()
                .map(|(c_sym, b_sym)| {
                    let a_sym = inner
                        .map
                        .get(b_sym)
                        .cloned()
                        .unwrap_or_else(|| b_sym.clone());
                    (c_sym.clone(), a_sym)
                })
                .collect();
            FactorRoute {
                src_factor: inner.src_factor,
                map,
            }
        })
        .collect();
    SigMorphism {
        src: first.src.clone(),
        tgt: second.tgt.clone(),
        routes,
    }
}

/// A single **validity failure** of a tight arrow (`check_morphism`).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphismError
{
    /// A human-readable description of the failure.
    pub message: DiagnosticMessage,
}

/// Check one `target_symbol ↦ source_symbol` mapping for kind and payload
/// identity.
fn check_symbol_mapping(
    target: &SignDesc,
    source: &SignDesc,
    target_symbol: &Name,
    source_symbol: &Name,
    factor: DescriptorFactorIndex,
    errors: &mut Vec<MorphismError>,
)
{
    if let Some(target_ctor_code) = ctor_code(target, target_symbol.as_ref().into()) {
        match ctor_code(source, source_symbol.as_ref().into()) {
            | Some(source_ctor_code) if source_ctor_code == target_ctor_code => {},
            | Some(_) => errors.push(MorphismError {
                message: format!(
                    "ctor `{target_symbol}` ↦ `{source_symbol}` in factor {factor}: payload codes \
                     differ"
                )
                .into(),
            }),
            | None => errors.push(MorphismError {
                message: format!(
                    "ctor `{target_symbol}` ↦ `{source_symbol}` in factor {factor}: source symbol \
                     is not a constructor with the same code"
                )
                .into(),
            }),
        }
    }
    else if bool::from(is_op(target, target_symbol.as_ref().into())) {
        match (
            op_arity(target, target_symbol.as_ref().into()),
            op_arity(source, source_symbol.as_ref().into()),
        ) {
            | (Some(target_arity), Some(source_arity)) if target_arity == source_arity => {},
            | (Some(_), _) => errors.push(MorphismError {
                message: format!(
                    "op `{target_symbol}` ↦ `{source_symbol}` in factor {factor}: bridge arities \
                     differ or source is not an op"
                )
                .into(),
            }),
            | (None, _) => {},
        }
    }
    else {
        errors.push(MorphismError {
            message: format!(
                "mapped symbol `{target_symbol}` in factor {factor} is neither a ctor nor an op of \
                 the target"
            )
            .into(),
        });
    }
}

/// Look up a constructor's payload code by name.
fn ctor_code<'desc>(
    desc: &'desc SignDesc,
    name: NameRef<'_>,
) -> Option<&'desc Code>
{
    let name = name.as_ref();
    desc.ctors
        .iter()
        .find(|ctor| ctor.name.as_ref() == name)
        .map(|ctor| &ctor.code)
}

/// Whether a name is an operation of a description.
fn is_op(
    desc: &SignDesc,
    name: NameRef<'_>,
) -> SymbolPresence
{
    let name = name.as_ref();
    desc.opers.iter().any(|op| op.name.as_ref() == name).into()
}
/// Look up an operation's bridge arity by name.
fn op_arity<'desc>(
    desc: &'desc SignDesc,
    name: NameRef<'_>,
) -> Option<&'desc BridgeArity>
{
    let name = name.as_ref();
    desc.opers
        .iter()
        .find(|op| op.name.as_ref() == name)
        .map(|op| &op.arity)
}

/// The **face action** of a tight arrow: map both terms of a [`RuleFace`]
/// through [`apply_term`] and recompute the per-variable metadata with the real
/// [`derive_cell_var_meta`] (proposal §2, restriction = pattern substitution).
pub fn apply_face(
    morphism: &SigMorphism,
    tgt_factor: DescriptorFactorIndex,
    face: &RuleFace,
) -> RuleFace
{
    let lhs = apply_term(morphism, tgt_factor, &face.lhs);
    let rhs = apply_term(morphism, tgt_factor, &face.rhs);
    let vars = derive_cell_var_meta(&lhs);
    RuleFace::new(lhs, rhs, vars, face.provenance)
}
/// Replay path induction on a **saturated** middle: transport the left endpoint
/// along the absorbed path so it meets the right, then fire the base cell.
///
/// On an empty absorbed path this is exactly the `refl` case, so the value is
/// β-compatible with [`replay`] of [`CellKind::PathInd`]; on a non-empty path
/// it is *defined*, evidencing that the saturation invariant is sufficient (not
/// merely necessary) for the unit's universal property.
pub fn replay_path_ind_saturated(
    base: &Cell,
    left_input: &LooseInstance,
    saturated: &SaturatedInstance,
    right_input: &LooseInstance,
    cells: &[RuleFace],
) -> Option<LooseInstance>
{
    // The absorbed path transports the left endpoint; the worked example simply
    // confirms it reaches a term (a real reduction) before the base fires.
    let _transported = apply_path(&saturated.path_start, &saturated.absorbed, cells)?;
    replay(base, &[left_input.clone(), right_input.clone()])
}
/// The left boundary of a base instance under a formal restriction: the face's
/// left term (or path start) under the substitution, translated through the
/// left frame's tight action.
pub fn left_endpoint(
    factor: &FormalRestriction,
    instance: &BaseInstance,
) -> Option<FreeTerm>
{
    let raw = match *instance {
        | BaseInstance::Gen {
            generator,
            ref subst,
        } => {
            let BaseLoose::Named(ref rel) = factor.base
            else {
                return None;
            };
            let face = rel.gens.get(usize::from(generator))?;
            subst_term(&face.lhs, subst)
        },
        | BaseInstance::Path { ref start, .. } => start.clone(),
    };
    Some(apply_term(&factor.left, 0.into(), &raw))
}
/// The right boundary of a base instance under a formal restriction: the face's
/// right term (or path endpoint) under the substitution, translated through the
/// right frame's tight action.
pub fn right_endpoint(
    factor: &FormalRestriction,
    instance: &BaseInstance,
) -> Option<FreeTerm>
{
    let raw = match *instance {
        | BaseInstance::Gen {
            generator,
            ref subst,
        } => {
            let BaseLoose::Named(ref rel) = factor.base
            else {
                return None;
            };
            let face = rel.gens.get(usize::from(generator))?;
            subst_term(&face.rhs, subst)
        },
        | BaseInstance::Path {
            ref start,
            ref steps,
        } => {
            let BaseLoose::Path { ref sig } = factor.base
            else {
                return None;
            };
            let first_factor = sig.factors.first()?;
            let rules = &first_factor.rules;
            apply_path(start, steps, rules)?
        },
    };
    Some(apply_term(&factor.right, 0.into(), &raw))
}
mod substitution_support
{
    use super::*;

    /// **Substitute** ground terms for variables throughout a term.
    pub fn subst_term(
        term: &FreeTerm,
        subst: &BTreeMap<Name, FreeTerm>,
    ) -> FreeTerm
    {
        match *term {
            | FreeTerm::Var(ref name) => subst
                .get(name)
                .cloned()
                .unwrap_or_else(|| FreeTerm::Var(name.clone())),
            | FreeTerm::Ctor(ref name, ref args) => FreeTerm::Ctor(
                name.clone(),
                args.iter().map(|arg| subst_term(arg, subst)).collect(),
            ),
            | FreeTerm::Op(ref name, ref args) => FreeTerm::Op(
                name.clone(),
                args.iter().map(|arg| subst_term(arg, subst)).collect(),
            ),
        }
    }
}

mod matching_support
{
    use super::*;

    /// **Match** a pattern against a ground term (patterns vs ground — plain
    /// first-order matching, no unification): a variable binds to the aligned
    /// subterm (consistently on repeats), a ctor / op must agree on head and
    /// arity and match argument-wise. Bindings accumulate into `out`; the
    /// boolean reports success.
    pub fn match_pattern(
        pattern: &FreeTerm,
        ground: &FreeTerm,
        out: &mut BTreeMap<Name, FreeTerm>,
    ) -> PatternMatch
    {
        let matched = match *pattern {
            | FreeTerm::Var(ref name) => match out.get(name) {
                | Some(existing) => existing == ground,
                | None => {
                    out.insert(name.clone(), ground.clone());
                    true
                },
            },
            | FreeTerm::Ctor(ref name, ref args) => match *ground {
                | FreeTerm::Ctor(ref g_name, ref g_args)
                    if name == g_name && args.len() == g_args.len() =>
                {
                    args.iter()
                        .zip(g_args.iter())
                        .all(|(pat, gnd)| bool::from(match_pattern(pat, gnd, out)))
                },
                | _ => false,
            },
            | FreeTerm::Op(ref name, ref args) => match *ground {
                | FreeTerm::Op(ref g_name, ref g_args)
                    if name == g_name && args.len() == g_args.len() =>
                {
                    args.iter()
                        .zip(g_args.iter())
                        .all(|(pat, gnd)| bool::from(match_pattern(pat, gnd, out)))
                },
                | _ => false,
            },
        };
        matched.into()
    }
}

mod position_support
{
    use super::*;

    /// The subterm at a position (a path of argument indices), or [`None`] if
    /// the position leaves the term.
    pub(super) fn subterm_at<'term>(
        term: &'term FreeTerm,
        position: &[TermPositionIndex],
    ) -> Option<&'term FreeTerm>
    {
        let mut cursor = term;
        for &index in position {
            cursor = match *cursor {
                | FreeTerm::Ctor(_, ref args) | FreeTerm::Op(_, ref args) => {
                    args.get(usize::from(index))?
                },
                | FreeTerm::Var(_) => return None,
            };
        }
        Some(cursor)
    }
    /// Replace the subterm at a position, returning the rebuilt whole term (or
    /// [`None`] if the position leaves the term).
    pub(super) fn replace_at(
        term: &FreeTerm,
        position: &[TermPositionIndex],
        replacement: FreeTerm,
    ) -> Option<FreeTerm>
    {
        let mut cursor = term;
        let mut parents: Vec<(bool, Name, Vec<FreeTerm>, usize)> = Vec::new();
        for &head in position {
            let index = usize::from(head);
            match *cursor {
                | FreeTerm::Ctor(ref name, ref args) => {
                    let child = args.get(index)?;
                    parents.push((true, name.clone(), args.to_vec(), index));
                    cursor = child;
                },
                | FreeTerm::Op(ref name, ref args) => {
                    let child = args.get(index)?;
                    parents.push((false, name.clone(), args.to_vec(), index));
                    cursor = child;
                },
                | FreeTerm::Var(_) => return None,
            }
        }

        let mut rebuilt = replacement;
        for (is_ctor, name, mut args, index) in parents.into_iter().rev() {
            args[index] = rebuilt;
            rebuilt = if is_ctor {
                FreeTerm::Ctor(name, args.into())
            }
            else {
                FreeTerm::Op(name, args.into())
            };
        }
        Some(rebuilt)
    }
}

mod rewrite_support
{
    use super::*;

    /// Replay a stored path: fold each step's rewrite over the running term,
    /// returning the endpoint (or [`None`] if a step no longer applies).
    pub fn apply_path(
        start: &FreeTerm,
        steps: &[RewriteStep],
        cells: &[RuleFace],
    ) -> Option<FreeTerm>
    {
        let mut current = start.clone();
        for step in steps {
            let face = cells.get(usize::from(step.cell))?;
            let (_, whole) = rewrite_with_face(&current, &step.pos, face)?;
            current = whole;
        }
        Some(current)
    }
    /// Rewrite at a position with a face: match `face.lhs` against the subterm,
    /// then splice `face.rhs` (under the match) back in. Returns the match
    /// binding and the rewritten whole term.
    pub(super) fn rewrite_with_face(
        term: &FreeTerm,
        position: &[TermPositionIndex],
        face: &RuleFace,
    ) -> Option<(BTreeMap<Name, FreeTerm>, FreeTerm)>
    {
        let subterm = subterm_at(term, position)?;
        let mut binding: BTreeMap<Name, FreeTerm> = BTreeMap::new();
        if !bool::from(match_pattern(&face.lhs, subterm, &mut binding)) {
            return None;
        }
        let rewritten = subst_term(&face.rhs, &binding);
        let whole = replace_at(term, position, rewritten)?;
        Some((binding, whole))
    }
}

pub use matching_support::match_pattern;
use position_support::replace_at;
use position_support::subterm_at;
pub use rewrite_support::apply_path;
use rewrite_support::rewrite_with_face;
pub use substitution_support::subst_term;

/// The **term action** of a tight arrow: rename the ctor / op symbols of a term
/// over target factor `tgt_factor` into symbols of the routed source factor,
/// leaving variables untouched (proposal §2; the action sends `B`-terms to
/// `A`-terms).
pub fn apply_term(
    morphism: &SigMorphism,
    tgt_factor: DescriptorFactorIndex,
    term: &FreeTerm,
) -> FreeTerm
{
    rename_term(term, morphism.routes.get(usize::from(tgt_factor)))
}

/// Rename a term's symbols through an optional route (an absent route or
/// unmapped symbol leaves the symbol unchanged, keeping the action total).
fn rename_term(
    term: &FreeTerm,
    route: Option<&FactorRoute>,
) -> FreeTerm
{
    match *term {
        | FreeTerm::Var(ref name) => FreeTerm::Var(name.clone()),
        | FreeTerm::Ctor(ref name, ref args) => {
            let renamed = route
                .and_then(|route| route.map.get(name))
                .cloned()
                .unwrap_or_else(|| name.clone());
            FreeTerm::Ctor(
                renamed,
                args.iter().map(|arg| rename_term(arg, route)).collect(),
            )
        },
        | FreeTerm::Op(ref name, ref args) => {
            let renamed = route
                .and_then(|route| route.map.get(name))
                .cloned()
                .unwrap_or_else(|| name.clone());
            FreeTerm::Op(
                renamed,
                args.iter().map(|arg| rename_term(arg, route)).collect(),
            )
        },
    }
}

// ======================================================================
// First-order matching, substitution, rewriting, and path enumeration
// (the test-side term machinery gandr-theory-levitation does not ship —
// obligation H1)
// ======================================================================

/// **Bounded, deterministic** enumeration of rewrite paths from a start term:
/// every reduction sequence of length `0 ..= max_depth`, including the empty
/// path (`refl`). Each entry pairs the step list with the reached term.
pub fn enumerate_paths(
    start: &FreeTerm,
    cells: &[RuleFace],
    max_depth: RewriteDepth,
) -> Vec<(Vec<RewriteStep>, FreeTerm)>
{
    let mut out: Vec<(Vec<RewriteStep>, FreeTerm)> = vec![(Vec::new(), start.clone())];
    let mut frontier: Vec<(Vec<RewriteStep>, FreeTerm)> = vec![(Vec::new(), start.clone())];
    for _ in 0 .. usize::from(max_depth) {
        let mut next: Vec<(Vec<RewriteStep>, FreeTerm)> = Vec::new();
        for entry in &frontier {
            let (steps, term) = (&entry.0, &entry.1);
            for (step, whole) in one_step_rewrites(term, cells) {
                let mut extended = steps.clone();
                extended.push(step);
                out.push((extended.clone(), whole.clone()));
                next.push((extended, whole));
            }
        }
        frontier = next;
    }
    out
}
/// One deterministic step of rewriting: for every position (pre-order) and
/// every cell (index order), the rewrite steps that fire.
pub fn one_step_rewrites(
    term: &FreeTerm,
    cells: &[RuleFace],
) -> Vec<(RewriteStep, FreeTerm)>
{
    let mut out: Vec<(RewriteStep, FreeTerm)> = Vec::new();
    for position in positions(term) {
        for (cell_index, face) in cells.iter().enumerate() {
            if let Some((binding, whole)) = rewrite_with_face(term, &position, face) {
                out.push((
                    RewriteStep {
                        cell: cell_index.into(),
                        pos: position.clone(),
                        subst: binding,
                    },
                    whole,
                ));
            }
        }
    }
    out
}
/// Every subterm position of a term, in deterministic pre-order (root first).
fn positions(term: &FreeTerm) -> Vec<Vec<TermPositionIndex>>
{
    let mut out = Vec::new();
    let mut stack: Vec<(Vec<TermPositionIndex>, &FreeTerm)> = vec![(Vec::new(), term)];
    while let Some((position, current)) = stack.pop() {
        out.push(position.clone());
        if let FreeTerm::Ctor(_, ref args) | FreeTerm::Op(_, ref args) = *current {
            for (index, arg) in args.iter().enumerate().rev() {
                let mut child_position = position.clone();
                child_position.push(index.into());
                stack.push((child_position, arg));
            }
        }
    }
    out
}

// ======================================================================
// Loose arrows: the split representation (Nasu Def 3.2.6 / Lemma 3.2.8)
// ======================================================================

/// A **named relation interface** `R : I ⇸ J`: a set of generating
/// [`RuleFace`]s between two objects (proposal §2; the named-relation
/// granularity settling §10.1 for F0).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relation
{
    /// The relation's name.
    pub name: Name,
    /// The source object `I`.
    pub src: SigObj,
    /// The target object `J`.
    pub tgt: SigObj,
    /// The generating faces.
    pub gens: Vec<RuleFace>,
}

/// The **base** of a formal restriction: a named relation, or the
/// rewrite-*path* relation over a single-description object (proposal §2,
/// units).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BaseLoose
{
    /// A named relation interface (shared, hence [`Rc`]).
    Named(Rc<Relation>),
    /// The path relation `x ⇝ y` over a single-description object.
    Path
    {
        /// The single-factor object the path relation ranges over.
        sig: SigObj,
    },
}

impl BaseLoose
{
    /// The base's source object.
    pub fn src(&self) -> SigObj
    {
        match *self {
            | Self::Named(ref rel) => rel.src.clone(),
            | Self::Path { ref sig } => sig.clone(),
        }
    }

    /// The base's target object.
    pub fn tgt(&self) -> SigObj
    {
        match *self {
            | Self::Named(ref rel) => rel.tgt.clone(),
            | Self::Path { ref sig } => sig.clone(),
        }
    }
}

/// A **formal restriction** `base[left # right]` (Nasu Def 3.2.6, realized): a
/// base loose arrow together with the two tight arrows that precompose its
/// faces. Restriction never touches the base; it only composes into `left` /
/// `right` — this is what makes restriction split (Lemma 3.2.8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormalRestriction
{
    /// The left frame `left : I → base.src`.
    pub left: SigMorphism,
    /// The base loose arrow.
    pub base: BaseLoose,
    /// The right frame `right : J → base.tgt`.
    pub right: SigMorphism,
}

/// A **loose arrow** `α : I ⇸ J`: a `∧`-tuple of formal restrictions (proposal
/// §2). The empty tuple is `⊤`; concatenation is `∧`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LooseArrow
{
    /// The source object `I`.
    pub src: SigObj,
    /// The target object `J`.
    pub tgt: SigObj,
    /// The `∧`-tuple of formal restrictions.
    pub factors: Vec<FormalRestriction>,
}

impl LooseArrow
{
    /// A loose arrow with a single formal-restriction factor over a named
    /// relation, framed by identities.
    pub fn of_relation(rel: Rc<Relation>) -> Self
    {
        let src = rel.src.clone();
        let tgt = rel.tgt.clone();
        let left = SigMorphism::identity(&src);
        let right = SigMorphism::identity(&tgt);
        Self {
            src,
            tgt,
            factors: vec![FormalRestriction {
                left,
                base: BaseLoose::Named(rel),
                right,
            }],
        }
    }

    /// The path loose arrow over a single-description object, framed by
    /// identities (the unit; proposal §2).
    pub fn path(sig: &SigObj) -> Self
    {
        let frame = SigMorphism::identity(sig);
        Self {
            src: sig.clone(),
            tgt: sig.clone(),
            factors: vec![FormalRestriction {
                left: frame.clone(),
                base: BaseLoose::Path { sig: sig.clone() },
                right: frame,
            }],
        }
    }

    /// The terminal loose arrow `⊤ : I ⇸ J` — the empty `∧`-tuple.
    pub fn top(
        src: &SigObj,
        tgt: &SigObj,
    ) -> Self
    {
        Self {
            src: src.clone(),
            tgt: tgt.clone(),
            factors: Vec::new(),
        }
    }

    /// The **meet** `α ∧ β` — factor concatenation (both must share endpoints).
    pub fn meet(
        left: &Self,
        right: &Self,
    ) -> Self
    {
        let mut factors = left.factors.clone();
        factors.extend(right.factors.iter().cloned());
        Self {
            src: left.src.clone(),
            tgt: left.tgt.clone(),
            factors,
        }
    }
}

/// **Factor a framed cell through its restricted globular form** (Nasu
/// fibrationality; proposal §3 Law 3(c)): push the frames into the codomain's
/// formal restriction, leaving identity frames. The kind is preserved, so the
/// clause data is identical — the factorization is unique.
pub fn factor_globular(cell: &Cell) -> Cell
{
    let cod = restrict(&cell.cod, &cell.left_frame, &cell.right_frame);
    let left_frame = SigMorphism::identity(&cod.src);
    let right_frame = SigMorphism::identity(&cod.tgt);
    Cell {
        dom: cell.dom.clone(),
        cod,
        left_frame,
        right_frame,
        kind: cell.kind.clone(),
    }
}
/// **Restriction** `α[s # t]` (proposal §2; Nasu Def 3.2.6): compose `s` into
/// every factor's left frame and `t` into every factor's right frame. Split by
/// construction — the base is untouched.
pub fn restrict(
    alpha: &LooseArrow,
    s: &SigMorphism,
    t: &SigMorphism,
) -> LooseArrow
{
    let factors = alpha
        .factors
        .iter()
        .map(|factor| FormalRestriction {
            left: compose(s, &factor.left),
            base: factor.base.clone(),
            right: compose(t, &factor.right),
        })
        .collect();
    LooseArrow {
        src: s.src.clone(),
        tgt: t.src.clone(),
        factors,
    }
}

// ======================================================================
// Instances: base and loose instances, with computed boundaries
// ======================================================================

/// One **rewrite step** of a path instance: which cell fired, at which
/// position, under which match binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewriteStep
{
    /// The index of the fired cell in its signature's cell list.
    pub cell: GeneratorIndex,
    /// The position (argument-index path) the rewrite fired at.
    pub pos: Vec<TermPositionIndex>,
    /// The match binding recorded when the step fired.
    pub subst: BTreeMap<Name, FreeTerm>,
}

/// An **instance of a base loose arrow**: a generating cell applied under a
/// ground substitution, or a path (finite reduction sequence).
///
/// Boundaries are **computed on read** ([`left_endpoint`] /
/// [`right_endpoint`]), never stored (proposal §2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BaseInstance
{
    /// A generating-cell instance: generator index plus a ground substitution
    /// for its variables.
    Gen
    {
        /// The generator index into the named relation's `gens`.
        generator: GeneratorIndex,
        /// The ground substitution for the generator's variables.
        subst: BTreeMap<Name, FreeTerm>,
    },
    /// A path instance: a start term plus a (possibly empty) step list. `refl`
    /// is the empty step list.
    Path
    {
        /// The start term of the path.
        start: FreeTerm,
        /// The rewrite steps (empty = `refl`).
        steps: Vec<RewriteStep>,
    },
}

/// An **instance of a loose arrow**: one [`BaseInstance`] per
/// formal-restriction factor.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LooseInstance
{
    /// One base instance per `∧`-tuple factor.
    pub per_factor: Vec<BaseInstance>,
}

/// Equality of two base instances **after boundary normalization**: generating
/// instances compare on generator index and substitution; path instances
/// compare on start and step list.
fn base_instance_eq(
    left: &BaseInstance,
    right: &BaseInstance,
) -> LooseInstanceEquality
{
    let equal = match (left, right) {
        | (
            &BaseInstance::Gen {
                generator: left_gen,
                subst: ref left_subst,
            },
            &BaseInstance::Gen {
                generator: right_gen,
                subst: ref right_subst,
            },
        ) => left_gen == right_gen && left_subst == right_subst,
        | (
            &BaseInstance::Path {
                start: ref left_start,
                steps: ref left_steps,
            },
            &BaseInstance::Path {
                start: ref right_start,
                steps: ref right_steps,
            },
        ) => left_start == right_start && left_steps == right_steps,
        | _ => false,
    };
    equal.into()
}

/// Equality of two loose instances: factorwise [`base_instance_eq`].
pub fn loose_instance_eq(
    left: &LooseInstance,
    right: &LooseInstance,
) -> LooseInstanceEquality
{
    let equal = left.per_factor.len() == right.per_factor.len()
        && left
            .per_factor
            .iter()
            .zip(right.per_factor.iter())
            .all(|(one, other)| bool::from(base_instance_eq(one, other)));
    equal.into()
}

// ======================================================================
// Cells: the test-side stand-in for certificate-backed transformations
// ======================================================================

/// A **clause** of a [`CellKind::Clauses`] cell: the expected generator index
/// at each input position, and the generator plus variable templates to emit at
/// each codomain tuple-factor.
///
/// A template value is a [`FreeTerm`] over **namespaced input variables**
/// spelled `p{i}.{v}` — position `i`'s bound variable `v` — instantiated at
/// replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellClause
{
    /// The expected generator index per input position.
    pub matches: Vec<GeneratorIndex>,
    /// Per codomain tuple-factor: the emitted generator and its variable
    /// templates.
    pub emit: Vec<(GeneratorIndex, BTreeMap<Name, FreeTerm>)>,
}

/// The **kind** of a multi-ary cell (the test-side stand-in for the polygraph
/// `Tracelet`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellKind
{
    /// The identity cell on a single loose arrow.
    Ident,
    /// A clause program: the first matching clause emits the output.
    Clauses(Vec<CellClause>),
    /// A pairing of two cells sharing a domain (cartesian intro).
    Pair(Box<Cell>, Box<Cell>),
    /// A projection onto a codomain factor.
    Proj
    {
        /// The factor index projected out.
        idx: DescriptorFactorIndex,
    },
    /// The unique cell into `⊤`.
    Bang,
    /// Path induction: `(a, refl, b) ↦ base(a, b)`, declining on a non-empty
    /// path.
    PathInd
    {
        /// The base cell the induction reduces to on `refl`.
        base: Box<Cell>,
    },
}

/// A **multi-ary cell** `dom ⇒ cod` framed by two tight arrows (proposal §2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cell
{
    /// The domain chain of loose arrows.
    pub dom: Vec<LooseArrow>,
    /// The codomain loose arrow.
    pub cod: LooseArrow,
    /// The left frame.
    pub left_frame: SigMorphism,
    /// The right frame.
    pub right_frame: SigMorphism,
    /// The cell's kind.
    pub kind: CellKind,
}

/// **Replay-level composition**: replay each inner cell on its own inputs, then
/// replay the outer cell on the resulting instances. Always available (proposal
/// §2, the `⋄` of the cartesian law), independent of whether [`graft`] supports
/// the shapes symbolically.
pub fn replay_compose(
    outer: &Cell,
    inners: &[&Cell],
    inner_inputs: &[Vec<LooseInstance>],
) -> Option<LooseInstance>
{
    let mut middle: Vec<LooseInstance> = Vec::new();
    for (inner, inputs) in inners.iter().zip(inner_inputs.iter()) {
        let replayed = replay(inner, inputs)?;
        middle.push(replayed);
    }
    replay(outer, &middle)
}
/// **Replay** a cell on a chain of input instances — the checker (proposal §2;
/// F3 replay-equivalence). Partial: [`None`] ("declined") is meaningful data,
/// not an error.
pub fn replay(
    cell: &Cell,
    inputs: &[LooseInstance],
) -> Option<LooseInstance>
{
    enum ReplayFrame<'cell>
    {
        Eval
        {
            cell: &'cell Cell,
            inputs: Vec<LooseInstance>,
        },
        PairRight
        {
            right: &'cell Cell,
            inputs: Vec<LooseInstance>,
        },
        PairCombine
        {
            left_out: LooseInstance
        },
    }

    let mut stack = vec![ReplayFrame::Eval {
        cell,
        inputs: inputs.to_vec(),
    }];
    let mut last: Option<Option<LooseInstance>> = None;
    while let Some(frame) = stack.pop() {
        match frame {
            | ReplayFrame::Eval {
                cell: node,
                inputs: args,
            } => match node.kind {
                | CellKind::Ident => {
                    last = Some((args.len() == 1).then(|| args[0].clone()));
                },
                | CellKind::Bang => {
                    last = Some(Some(LooseInstance {
                        per_factor: Vec::new(),
                    }));
                },
                | CellKind::Proj { idx } => {
                    last = Some(args.first().and_then(|instance| {
                        instance
                            .per_factor
                            .get(usize::from(idx))
                            .map(|factor| LooseInstance {
                                per_factor: vec![factor.clone()],
                            })
                    }));
                },
                | CellKind::Pair(ref left, ref right) => {
                    stack.push(ReplayFrame::PairRight {
                        right,
                        inputs: args.clone(),
                    });
                    stack.push(ReplayFrame::Eval {
                        cell: left,
                        inputs: args,
                    });
                },
                | CellKind::Clauses(ref clauses) => {
                    last = Some(replay_clauses(clauses, &args));
                },
                | CellKind::PathInd { ref base } => {
                    let Some(left_input) = args.first().cloned()
                    else {
                        last = Some(None);
                        continue;
                    };
                    let Some(middle_input) = args.get(1)
                    else {
                        last = Some(None);
                        continue;
                    };
                    let Some(middle) = middle_input.per_factor.first()
                    else {
                        last = Some(None);
                        continue;
                    };
                    let Some(right_input) = args.get(2).cloned()
                    else {
                        last = Some(None);
                        continue;
                    };
                    if matches!(*middle, BaseInstance::Path { ref steps, .. } if steps.is_empty()) {
                        stack.push(ReplayFrame::Eval {
                            cell: base,
                            inputs: vec![left_input, right_input],
                        });
                    }
                    else {
                        last = Some(None);
                    }
                },
            },
            | ReplayFrame::PairRight {
                right,
                inputs: pending,
            } => {
                let taken = last.take().flatten();
                let Some(left_out) = taken
                else {
                    last = Some(None);
                    continue;
                };
                stack.push(ReplayFrame::PairCombine { left_out });
                stack.push(ReplayFrame::Eval {
                    cell: right,
                    inputs: pending,
                });
            },
            | ReplayFrame::PairCombine { left_out } => {
                let taken = last.take().flatten();
                let Some(right_out) = taken
                else {
                    last = Some(None);
                    continue;
                };
                let mut per_factor = left_out.per_factor;
                per_factor.extend(right_out.per_factor);
                last = Some(Some(LooseInstance { per_factor }));
            },
        }
    }
    last.flatten()
}

/// Replay a clause program: the first clause whose `matches` agree with the
/// inputs' leading generators fires, emitting one generating instance per
/// codomain factor with templates instantiated over the namespaced input
/// bindings.
fn replay_clauses(
    clauses: &[CellClause],
    inputs: &[LooseInstance],
) -> Option<LooseInstance>
{
    for clause in clauses {
        if clause.matches.len() != inputs.len() {
            continue;
        }
        let mut env: BTreeMap<Name, FreeTerm> = BTreeMap::new();
        let mut matched = true;
        for (position, instance) in inputs.iter().enumerate() {
            match instance.per_factor.first() {
                | Some(&BaseInstance::Gen {
                    generator,
                    ref subst,
                }) if generator == clause.matches[position] => {
                    for (variable, term) in subst {
                        env.insert(
                            format!("p{position}.{variable}").into_boxed_str().into(),
                            term.clone(),
                        );
                    }
                },
                | _ => {
                    matched = false;
                    break;
                },
            }
        }
        if !matched {
            continue;
        }
        let per_factor = clause
            .emit
            .iter()
            .map(|&(generator, ref templates)| {
                let subst = templates
                    .iter()
                    .map(|(variable, template)| (variable.clone(), subst_term(template, &env)))
                    .collect();
                BaseInstance::Gen { generator, subst }
            })
            .collect();
        return Some(LooseInstance { per_factor });
    }
    None
}

/// **Replay-equivalence** decided over a supplied corpus of chained input
/// tuples (proposal §2; certificate identity, F3).
///
/// The corpus is explicit — at F0 equality is relative to a generated corpus
/// (proptest plus exhaustive small enumeration); a cached derivation normal
/// form (proposal §10.2) is the F2 upgrade. Two cells are equivalent when they
/// decline together and, where both fire, produce boundary-normalized-equal
/// outputs on every corpus tuple.
pub fn cells_equal(
    left: &Cell,
    right: &Cell,
    corpus: &[Vec<LooseInstance>],
) -> CellEquivalence
{
    let equal = corpus.iter().all(
        |inputs| match (replay(left, inputs), replay(right, inputs)) {
            | (None, None) => true,
            | (Some(ref one), Some(ref other)) => bool::from(loose_instance_eq(one, other)),
            | _ => false,
        },
    );
    equal.into()
}

/// **Symbolic grafting** (multicategorical composition): substitute the inner
/// cells for the outer cell's leaves (proposal §2). Total only on the supported
/// shapes — the unital cases and the linear single-clause chain — returning
/// [`None`] elsewhere (replay-level composition, [`replay_compose`], covers the
/// rest).
pub fn graft(
    outer: &Cell,
    inners: &[Cell],
) -> Option<Cell>
{
    // Unit: grafting the identity outer returns the sole inner.
    if matches!(outer.kind, CellKind::Ident) && inners.len() == 1 {
        return Some(inners[0].clone());
    }
    // Unit: grafting all-identity inners returns the outer.
    if !inners.is_empty()
        && inners
            .iter()
            .all(|inner| matches!(inner.kind, CellKind::Ident))
    {
        return Some(outer.clone());
    }
    // Linear single-clause chain: Clauses ∘ [Clauses].
    if let CellKind::Clauses(ref outer_clauses) = outer.kind
        && inners.len() == 1
        && let CellKind::Clauses(ref inner_clauses) = inners[0].kind
    {
        return graft_linear_clause(outer, outer_clauses, &inners[0], inner_clauses);
    }
    None
}

/// Graft a single-clause outer over a single-clause, single-input inner by
/// composing the emit templates through the matched middle generator.
fn graft_linear_clause(
    outer: &Cell,
    outer_clauses: &[CellClause],
    inner: &Cell,
    inner_clauses: &[CellClause],
) -> Option<Cell>
{
    // Exactly one clause each (the linear single-clause chain).
    if outer_clauses.len() != 1 || inner_clauses.len() != 1 {
        return None;
    }
    let outer_clause = outer_clauses.first()?;
    let inner_clause = inner_clauses.first()?;
    // The outer must take exactly one input (the inner's single output).
    if outer_clause.matches.len() != 1 || inner_clause.emit.len() != 1 {
        return None;
    }
    let first_emit = inner_clause.emit.first()?;
    let inner_emit_gen = first_emit.0;
    let inner_templates = &first_emit.1;
    let &outer_match = outer_clause.matches.first()?;
    // The outer's expected middle generator must be what the inner emits.
    if outer_match != inner_emit_gen {
        return None;
    }
    // Namespace the inner templates as the outer's `p0.*` middle variables.
    let mut middle: BTreeMap<Name, FreeTerm> = BTreeMap::new();
    for (variable, template) in inner_templates {
        middle.insert(
            format!("p0.{variable}").into_boxed_str().into(),
            template.clone(),
        );
    }
    let emit = outer_clause
        .emit
        .iter()
        .map(|&(generator, ref templates)| {
            let composed = templates
                .iter()
                .map(|(variable, template)| (variable.clone(), subst_term(template, &middle)))
                .collect();
            (generator, composed)
        })
        .collect();
    Some(Cell {
        dom: inner.dom.clone(),
        cod: outer.cod.clone(),
        left_frame: inner.left_frame.clone(),
        right_frame: outer.right_frame.clone(),
        kind: CellKind::Clauses(vec![CellClause {
            matches: inner_clause.matches.clone(),
            emit,
        }]),
    })
}

// ======================================================================
// Saturation probe (Law 5): instances-as-modules over the path relation
// ======================================================================

/// A **saturated instance** (Law 5's worked example): a generating instance
/// together with an absorbed boundary path.
///
/// This is the shape the unit universal property's bijectivity needs — the
/// future cell store's saturation invariant, "instances closed under path
/// action". It is hand-built here for one worked example, not a general
/// mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaturatedInstance
{
    /// The generating instance whose left endpoint the path transports.
    pub generator: BaseInstance,
    /// The absorbed boundary path (empty = `refl`).
    pub absorbed: Vec<RewriteStep>,
    /// The start term of the absorbed path.
    pub path_start: FreeTerm,
}
