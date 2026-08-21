//! **Substitutions**, one-sided **matching** (cell application), and two-sided
//! **unification** (overlap enumeration) over the command-pattern IL
//! (`proposal-sequent-kernel.md` §7.3).
//!
//! A [`Subst`] binds producer metavariables to [`ProdPat`]s and consumer
//! metavariables to [`ConsPat`]s — two maps disjoined by [`Cat`], keyed on the
//! ordered [`MetaVar`] so iteration is deterministic (`iter_over_hash_type` is
//! denied project-wide; a `BTreeMap` is the deterministic carrier).
//!
//! - [`match_cmd`] is **one-sided**: a cell's left-hand side (with
//!   metavariables) against a *ground* command configuration, binding the
//!   pattern's metavariables to the configuration's subterms. This is what cell
//!   *application* (`gandr_theory_coherent_resolutions::rewrite`) runs.
//!   Non-linear patterns are admitted (VDC addendum §12.4): a repeated
//!   metavariable must bind structurally-equal subterms, checked by [`Eq`].
//! - [`unify_cmd`] is **two-sided**: two cell patterns (metavariables on both
//!   sides, kept apart by `gandr_theory_coherent_resolutions::overlap`'s
//!   renaming) unified into a most general unifier — the superposition step of
//!   overlap enumeration (§7.3.2).
//!
//! Both are **iterative** (an explicit goal worklist, never input-scaled
//! recursion; ADR-47), so a deeply-nested generated ground configuration cannot
//! overflow the stack.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::boundary::SubstitutionBindingCount;
use crate::boundary::SubstitutionDecision;
use crate::boundary::SubstitutionEmptyStatus;
use crate::pattern::Cat;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::Node;
use crate::pattern::ProdPat;
use crate::pattern::collect_cons_metavars;
use crate::pattern::collect_prod_metavars;
use crate::pattern::transform_node;

/// A **substitution** — a binding of producer metavariables to producer
/// patterns and consumer metavariables to consumer patterns.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Subst
{
    /// Producer-metavariable bindings.
    prods: BTreeMap<MetaVar, ProdPat>,
    /// Consumer-metavariable bindings.
    conss: BTreeMap<MetaVar, ConsPat>,
}

impl Subst
{
    /// The empty substitution.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Whether the substitution binds nothing.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> SubstitutionEmptyStatus
    {
        SubstitutionEmptyStatus::from(self.prods.is_empty() && self.conss.is_empty())
    }

    /// The number of bound metavariables (producer plus consumer).
    #[inline]
    #[must_use]
    pub fn len(&self) -> SubstitutionBindingCount
    {
        SubstitutionBindingCount::from(self.prods.len().saturating_add(self.conss.len()))
    }

    /// The producer pattern bound to `mv`, if any.
    ///
    /// # Contract
    /// - ensures: `Some` iff `mv` (a [`Cat::Producer`] metavariable) is bound.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get_prod(
        &self,
        mv: &MetaVar,
    ) -> Option<&ProdPat>
    {
        self.prods.get(mv)
    }

    /// The consumer pattern bound to `mv`, if any.
    ///
    /// # Contract
    /// - ensures: `Some` iff `mv` (a [`Cat::Consumer`] metavariable) is bound.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get_cons(
        &self,
        mv: &MetaVar,
    ) -> Option<&ConsPat>
    {
        self.conss.get(mv)
    }

    /// Bind a producer metavariable, returning `false` on a conflicting rebind.
    ///
    /// # Contract
    /// - requires: `mv.cat == Cat::Producer`.
    /// - ensures: `true` when `mv` was unbound or already bound to `value`;
    ///   `false` (leaving the binding intact) on a conflicting rebind.
    /// - panics: none.
    #[inline]
    pub fn bind_prod(
        &mut self,
        mv: MetaVar,
        value: ProdPat,
    ) -> SubstitutionDecision
    {
        SubstitutionDecision::from(match self.prods.get(&mv) {
            | Some(existing) => *existing == value,
            | None => {
                self.prods.insert(mv, value);
                true
            },
        })
    }

    /// Bind a consumer metavariable, returning `false` on a conflicting rebind.
    ///
    /// # Contract
    /// - requires: `mv.cat == Cat::Consumer`.
    /// - ensures: `true` when `mv` was unbound or already bound to `value`;
    ///   `false` (leaving the binding intact) on a conflicting rebind.
    /// - panics: none.
    #[inline]
    pub fn bind_cons(
        &mut self,
        mv: MetaVar,
        value: ConsPat,
    ) -> SubstitutionDecision
    {
        SubstitutionDecision::from(match self.conss.get(&mv) {
            | Some(existing) => *existing == value,
            | None => {
                self.conss.insert(mv, value);
                true
            },
        })
    }

    /// Fully resolve every binding image by iterated application, so the
    /// substitution becomes **idempotent**: one application pass then fully
    /// instantiates any pattern.
    ///
    /// Bindings produced by [`unify_cmd`] may be *triangular* — an image may
    /// mention a metavariable bound after it, because the goal order is not
    /// topological. Each pass resolves only images that still mention bound
    /// metavariables (the skip-traversal guard: resolved bindings are never
    /// re-walked), and a pass resolves every image whose dependencies are
    /// already resolved, so at most one pass per binding layer runs.
    ///
    /// # Contract
    /// - ensures: no binding's image mentions a bound metavariable
    ///   (idempotent); the denoted instantiation is unchanged (each image is
    ///   replaced by its fixpoint, never by a different pattern).
    /// - panics: none.
    #[inline]
    pub fn resolve(&mut self)
    {
        for _ in 0 .. usize::from(self.len()) {
            let mut changed = false;
            let prod_keys: Vec<MetaVar> = self.prods.keys().cloned().collect();
            for key in prod_keys {
                let Some(image) = self.prods.get(&key).cloned()
                else {
                    continue;
                };
                if !bool::from(self.mentions_bound_prod(&image)) {
                    continue;
                }
                let resolved = self.apply_prod(&image);
                if resolved != image {
                    drop(self.prods.insert(key, resolved));
                    changed = true;
                }
            }
            let cons_keys: Vec<MetaVar> = self.conss.keys().cloned().collect();
            for key in cons_keys {
                let Some(image) = self.conss.get(&key).cloned()
                else {
                    continue;
                };
                if !bool::from(self.mentions_bound_cons(&image)) {
                    continue;
                }
                let resolved = self.apply_cons(&image);
                if resolved != image {
                    drop(self.conss.insert(key, resolved));
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// Whether any metavariable of a producer image is bound (the
    /// skip-traversal guard of [`Subst::resolve`]).
    ///
    /// # Contract
    /// - ensures: `true` iff some metavariable leaf of `image` is bound.
    /// - panics: none.
    #[inline]
    fn mentions_bound_prod(
        &self,
        image: &ProdPat,
    ) -> SubstitutionDecision
    {
        let mut vars = Vec::new();
        collect_prod_metavars(image, &mut vars);
        SubstitutionDecision::from(vars.iter().any(|mv| self.prods.contains_key(mv)))
    }

    /// Whether any metavariable of a consumer image is bound (see
    /// [`Subst::mentions_bound_prod`]; op arguments are producers).
    ///
    /// # Contract
    /// - ensures: `true` iff some metavariable leaf of `image` is bound.
    /// - panics: none.
    #[inline]
    fn mentions_bound_cons(
        &self,
        image: &ConsPat,
    ) -> SubstitutionDecision
    {
        let mut vars = Vec::new();
        collect_cons_metavars(image, &mut vars);
        SubstitutionDecision::from(vars.iter().any(|mv| match mv.cat {
            | Cat::Producer => self.prods.contains_key(mv),
            | Cat::Consumer => self.conss.contains_key(mv),
        }))
    }

    /// Apply the substitution to a command pattern.
    ///
    /// # Contract
    /// - ensures: every bound metavariable leaf is replaced by its binding;
    ///   unbound metavariables are left in place (a partial instantiation).
    ///   Application is a **single pass**: an image mentioning a bound
    ///   metavariable is inserted unresolved (see [`Subst::resolve`] for the
    ///   fixpoint form, which [`unify_cmd`] already runs on its unifiers).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn apply_cmd(
        &self,
        cmd: &CmdPat,
    ) -> CmdPat
    {
        // The skip-traversal guard: an empty substitution is the identity.
        if bool::from(self.is_empty()) {
            return cmd.clone();
        }
        let Some(Node::Cmd(cmd)) =
            transform_node(Node::Cmd(cmd.clone()), |node| Some(self.apply_node(node)))
        else {
            return cmd.clone();
        };
        cmd
    }

    /// Apply the substitution to a producer pattern (see [`Subst::apply_cmd`]).
    ///
    /// # Contract
    /// - ensures: as [`Subst::apply_cmd`], for a producer subtree.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn apply_prod(
        &self,
        prod: &ProdPat,
    ) -> ProdPat
    {
        // The skip-traversal guard (the Lean lesson): an empty substitution
        // is the identity — do not pay the rebuild.
        if bool::from(self.is_empty()) {
            return prod.clone();
        }
        let Some(Node::Prod(prod)) =
            transform_node(Node::Prod(prod.clone()), |node| Some(self.apply_node(node)))
        else {
            return prod.clone();
        };
        prod
    }

    /// Apply the substitution to a consumer pattern (see [`Subst::apply_cmd`]).
    ///
    /// # Contract
    /// - ensures: as [`Subst::apply_cmd`], for a consumer subtree.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn apply_cons(
        &self,
        cons: &ConsPat,
    ) -> ConsPat
    {
        if bool::from(self.is_empty()) {
            return cons.clone();
        }
        let Some(Node::Cons(cons)) =
            transform_node(Node::Cons(cons.clone()), |node| Some(self.apply_node(node)))
        else {
            return cons.clone();
        };
        cons
    }

    /// Apply one rebuilt node.
    #[inline]
    fn apply_node(
        &self,
        node: Node,
    ) -> Node
    {
        match node {
            | Node::Prod(ProdPat::Meta(ref mv)) => {
                self.prods.get(mv).cloned().map_or(node, Node::Prod)
            },
            | Node::Cons(ConsPat::Meta(ref mv)) => {
                self.conss.get(mv).cloned().map_or(node, Node::Cons)
            },
            | other => other,
        }
    }
}

/// A pending one-sided **match goal** — a pattern subterm against a ground
/// subterm (see [`match_cmd`]).
#[derive(Clone, Copy, Debug)]
enum MatchGoal<'goal>
{
    /// Match a producer pattern against a ground producer.
    Prod(&'goal ProdPat, &'goal ProdPat),
    /// Match a consumer pattern against a ground consumer.
    Cons(&'goal ConsPat, &'goal ConsPat),
}

/// Match a cell's left-hand-side command pattern against a **ground** command
/// configuration, extending `subst` with the metavariable bindings.
///
/// # Contract
/// - requires: `ground` is metavariable-free (a machine configuration or a
///   differential instance).
/// - ensures: `true` with `subst` extended so `subst.apply_cmd(pat) == *ground`
///   when the pattern matches; `false` (with `subst` possibly partially
///   extended) otherwise. Polarity must agree — a cell applies only at a cut of
///   its own orientation `ε` (K2).
/// - panics: none.
#[inline]
#[must_use]
pub fn match_cmd(
    pat: &CmdPat,
    ground: &CmdPat,
    subst: &mut Subst,
) -> SubstitutionDecision
{
    let CmdPat::Cut {
        pol: pp,
        prod: ref p_prod,
        cons: ref p_cons,
    } = *pat;
    let CmdPat::Cut {
        pol: gp,
        prod: ref g_prod,
        cons: ref g_cons,
    } = *ground;
    if pp != gp {
        return SubstitutionDecision::from(false);
    }
    let mut goals: Vec<MatchGoal<'_>> = alloc::vec![
        MatchGoal::Prod(p_prod, g_prod),
        MatchGoal::Cons(p_cons, g_cons)
    ];
    while let Some(goal) = goals.pop() {
        match goal {
            | MatchGoal::Prod(pat_node, ground_node) => {
                if !bool::from(match_prod_step(pat_node, ground_node, subst, &mut goals)) {
                    return SubstitutionDecision::from(false);
                }
            },
            | MatchGoal::Cons(pat_node, ground_node) => {
                if !bool::from(match_cons_step(pat_node, ground_node, subst, &mut goals)) {
                    return SubstitutionDecision::from(false);
                }
            },
        }
    }
    SubstitutionDecision::from(true)
}

/// Process one producer match goal, pushing child goals (see [`match_cmd`]).
///
/// # Contract
/// - ensures: `true` when the head shapes agree (binding a metavariable or
///   enqueuing argument goals); `false` on a shape / symbol / arity /
///   non-linear clash.
/// - panics: none.
#[inline]
fn match_prod_step<'goal>(
    pat: &'goal ProdPat,
    ground: &'goal ProdPat,
    subst: &mut Subst,
    goals: &mut Vec<MatchGoal<'goal>>,
) -> SubstitutionDecision
{
    SubstitutionDecision::from(match *pat {
        | ProdPat::Meta(ref mv) => bool::from(subst.bind_prod(mv.clone(), ground.clone())),
        | ProdPat::Ctor {
            ctor: ref pc,
            args: ref pa,
        } => match *ground {
            | ProdPat::Ctor {
                ctor: ref gc,
                args: ref ga,
            } if pc == gc && pa.len() == ga.len() => {
                for (p, g) in pa.iter().zip(ga.iter()) {
                    goals.push(MatchGoal::Prod(p, g));
                }
                true
            },
            | _ => false,
        },
    })
}

/// Process one consumer match goal, pushing child goals (see [`match_cmd`]).
///
/// # Contract
/// - ensures: `true` when the head shapes agree; `false` on a shape / symbol /
///   arity / non-linear clash.
/// - panics: none.
#[inline]
fn match_cons_step<'goal>(
    pat: &'goal ConsPat,
    ground: &'goal ConsPat,
    subst: &mut Subst,
    goals: &mut Vec<MatchGoal<'goal>>,
) -> SubstitutionDecision
{
    SubstitutionDecision::from(match *pat {
        | ConsPat::Meta(ref mv) => bool::from(subst.bind_cons(mv.clone(), ground.clone())),
        | ConsPat::Op {
            op: ref po,
            args: ref pa,
            ret: ref pr,
        } => match *ground {
            | ConsPat::Op {
                op: ref go,
                args: ref ga,
                ret: ref gr,
            } if po == go && pa.len() == ga.len() => {
                for (p, g) in pa.iter().zip(ga.iter()) {
                    goals.push(MatchGoal::Prod(p, g));
                }
                goals.push(MatchGoal::Cons(pr, gr));
                true
            },
            | _ => false,
        },
        | ConsPat::Frame {
            ctor: ref pc,
            ret: ref pr,
        } => match *ground {
            | ConsPat::Frame {
                ctor: ref gc,
                ret: ref gr,
            } if pc == gc => {
                goals.push(MatchGoal::Cons(pr, gr));
                true
            },
            | _ => false,
        },
        | ConsPat::Top => matches!(*ground, ConsPat::Top),
    })
}

/// A pending two-sided **unification goal** (see [`unify_cmd`]).
#[derive(Clone, Debug)]
enum UnifyGoal
{
    /// Unify two producer patterns.
    Prod(ProdPat, ProdPat),
    /// Unify two consumer patterns.
    Cons(ConsPat, ConsPat),
}

/// Unify two command patterns into a most general unifier, extending `subst`.
///
/// # Contract
/// - requires: the two patterns' metavariables are kept apart (overlap
///   enumeration renames one cell before unifying; a shared metavariable would
///   be treated as the same hole).
/// - ensures: `true` with `subst` a unifier — `subst.apply_cmd(a) ==
///   subst.apply_cmd(b)` — when the patterns unify; `false` on a symbol / arity
///   / polarity clash or an occurs-check failure (with `subst` possibly
///   partially extended).
/// - panics: none.
#[inline]
#[must_use]
pub fn unify_cmd(
    a: &CmdPat,
    b: &CmdPat,
    subst: &mut Subst,
) -> SubstitutionDecision
{
    let CmdPat::Cut {
        pol: pa,
        prod: ref prod_a,
        cons: ref cons_a,
    } = *a;
    let CmdPat::Cut {
        pol: pb,
        prod: ref prod_b,
        cons: ref cons_b,
    } = *b;
    if pa != pb {
        return SubstitutionDecision::from(false);
    }
    let mut goals = alloc::vec![
        UnifyGoal::Prod(prod_a.clone(), prod_b.clone()),
        UnifyGoal::Cons(cons_a.clone(), cons_b.clone()),
    ];
    while let Some(goal) = goals.pop() {
        let ok = match goal {
            | UnifyGoal::Prod(lhs, rhs) => unify_prod_step(&lhs, &rhs, subst, &mut goals),
            | UnifyGoal::Cons(lhs, rhs) => unify_cons_step(&lhs, &rhs, subst, &mut goals),
        };
        if !bool::from(ok) {
            return SubstitutionDecision::from(false);
        }
    }
    // Fully resolve the unifier: the goal order is not topological, so a
    // binding's image may mention a metavariable bound *after* it (a
    // triangular substitution), and single-pass application of such a
    // unifier would leave `apply_cmd(a) != apply_cmd(b)` — violating this
    // function's contract. Resolving to the fixpoint yields an idempotent
    // unifier: one application pass fully instantiates both sides.
    subst.resolve();
    SubstitutionDecision::from(true)
}

/// Process one producer unification goal (see [`unify_cmd`]).
///
/// # Contract
/// - ensures: `true` on a successful decomposition or binding; `false` on a
///   clash or occurs-check failure.
/// - panics: none.
#[inline]
fn unify_prod_step(
    lhs: &ProdPat,
    rhs: &ProdPat,
    subst: &mut Subst,
    goals: &mut Vec<UnifyGoal>,
) -> SubstitutionDecision
{
    let left = walk_prod(lhs, subst);
    let right = walk_prod(rhs, subst);
    match (left, right) {
        | (ProdPat::Meta(x), ProdPat::Meta(y)) if x == y => SubstitutionDecision::from(true),
        | (ProdPat::Meta(x), other) | (other, ProdPat::Meta(x)) => {
            if bool::from(occurs_in_prod(&x, &other, subst)) {
                return SubstitutionDecision::from(false);
            }
            subst.bind_prod(x, other)
        },
        | (ProdPat::Ctor { ctor: cl, args: al }, ProdPat::Ctor { ctor: cr, args: ar }) => {
            if cl != cr || al.len() != ar.len() {
                return SubstitutionDecision::from(false);
            }
            for (p, q) in al.iter().zip(ar.iter()) {
                goals.push(UnifyGoal::Prod(p.clone(), q.clone()));
            }
            SubstitutionDecision::from(true)
        },
    }
}

/// Resolve a producer pattern's head metavariable through `subst` (a "walk").
///
/// # Contract
/// - ensures: follows a metavariable-to-metavariable binding chain to the first
///   non-metavariable or unbound head, cloning the resolved pattern.
/// - panics: none.
#[inline]
fn walk_prod(
    prod: &ProdPat,
    subst: &Subst,
) -> ProdPat
{
    let mut cur = prod.clone();
    while let ProdPat::Meta(ref mv) = cur {
        match subst.get_prod(mv) {
            | Some(bound) => cur = bound.clone(),
            | None => break,
        }
    }
    cur
}

/// Process one consumer unification goal (see [`unify_cmd`]).
///
/// # Contract
/// - ensures: `true` on a successful decomposition or binding; `false` on a
///   clash or occurs-check failure.
/// - panics: none.
#[inline]
fn unify_cons_step(
    lhs: &ConsPat,
    rhs: &ConsPat,
    subst: &mut Subst,
    goals: &mut Vec<UnifyGoal>,
) -> SubstitutionDecision
{
    let left = walk_cons(lhs, subst);
    let right = walk_cons(rhs, subst);
    match (left, right) {
        | (ConsPat::Meta(x), ConsPat::Meta(y)) if x == y => SubstitutionDecision::from(true),
        | (ConsPat::Meta(x), other) | (other, ConsPat::Meta(x)) => {
            if bool::from(occurs_in_cons(&x, &other, subst)) {
                return SubstitutionDecision::from(false);
            }
            subst.bind_cons(x, other)
        },
        | (
            ConsPat::Op {
                op: ol,
                args: al,
                ret: rl,
            },
            ConsPat::Op {
                op: or,
                args: ar,
                ret: rr,
            },
        ) => {
            if ol != or || al.len() != ar.len() {
                return SubstitutionDecision::from(false);
            }
            for (p, q) in al.iter().zip(ar.iter()) {
                goals.push(UnifyGoal::Prod(p.clone(), q.clone()));
            }
            goals.push(UnifyGoal::Cons(*rl, *rr));
            SubstitutionDecision::from(true)
        },
        | (ConsPat::Frame { ctor: cl, ret: rl }, ConsPat::Frame { ctor: cr, ret: rr }) => {
            if cl != cr {
                return SubstitutionDecision::from(false);
            }
            goals.push(UnifyGoal::Cons(*rl, *rr));
            SubstitutionDecision::from(true)
        },
        | (ConsPat::Top, ConsPat::Top) => SubstitutionDecision::from(true),
        | _ => SubstitutionDecision::from(false),
    }
}

/// Resolve a consumer pattern's head metavariable through `subst` (a "walk").
///
/// # Contract
/// - ensures: as [`walk_prod`], for a consumer pattern.
/// - panics: none.
#[inline]
fn walk_cons(
    cons: &ConsPat,
    subst: &Subst,
) -> ConsPat
{
    let mut cur = cons.clone();
    while let ConsPat::Meta(ref mv) = cur {
        match subst.get_cons(mv) {
            | Some(bound) => cur = bound.clone(),
            | None => break,
        }
    }
    cur
}

/// Whether producer metavariable `mv` occurs in `prod` after walking through
/// `subst` — the occurs-check guarding against a cyclic binding.
///
/// # Contract
/// - requires: `mv.cat == Cat::Producer`.
/// - ensures: `true` iff `mv` reaches a leaf of the fully-walked `prod`.
/// - panics: none.
#[inline]
fn occurs_in_prod(
    mv: &MetaVar,
    prod: &ProdPat,
    subst: &Subst,
) -> SubstitutionDecision
{
    debug_assert!(
        mv.cat == Cat::Producer,
        "producer occurs-check on a producer metavariable"
    );
    let mut stack = alloc::vec![prod.clone()];
    while let Some(term) = stack.pop() {
        match walk_prod(&term, subst) {
            | ProdPat::Meta(ref found) => {
                if found == mv {
                    return SubstitutionDecision::from(true);
                }
            },
            | ProdPat::Ctor { ref args, .. } => stack.extend(args.iter().cloned()),
        }
    }
    SubstitutionDecision::from(false)
}

/// Whether consumer metavariable `mv` occurs in `cons` after walking through
/// `subst` — the occurs-check guarding against a cyclic binding.
///
/// # Contract
/// - requires: `mv.cat == Cat::Consumer`.
/// - ensures: `true` iff `mv` reaches a consumer leaf of the fully-walked
///   `cons` (a consumer metavariable never sits in a producer argument, so the
///   producer arguments of an operation frame are not scanned).
/// - panics: none.
#[inline]
fn occurs_in_cons(
    mv: &MetaVar,
    cons: &ConsPat,
    subst: &Subst,
) -> SubstitutionDecision
{
    debug_assert!(
        mv.cat == Cat::Consumer,
        "consumer occurs-check on a consumer metavariable"
    );
    let mut stack = alloc::vec![cons.clone()];
    while let Some(term) = stack.pop() {
        match walk_cons(&term, subst) {
            | ConsPat::Meta(ref found) => {
                if found == mv {
                    return SubstitutionDecision::from(true);
                }
            },
            | ConsPat::Op { ref ret, .. } | ConsPat::Frame { ref ret, .. } => {
                stack.push((**ret).clone());
            },
            | ConsPat::Top => {},
        }
    }
    SubstitutionDecision::from(false)
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;

    #[test]
    fn matching_binds_a_ground_configuration()
    {
        // ⟨Succ(m) | add(n; α)⟩ against ⟨Succ(Zero) | add(Zero; ★)⟩.
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let ground = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        let mut subst = Subst::new();
        assert!(
            bool::from(match_cmd(&lhs, &ground, &mut subst)),
            "the LHS matches the config"
        );
        assert_eq!(
            subst.apply_cmd(&lhs),
            ground,
            "the match reconstructs the config"
        );
    }

    #[test]
    fn a_polarity_clash_blocks_a_match()
    {
        let lhs = CmdPat::cut(Polarity::Positive, ProdPat::meta("x"), ConsPat::meta("a"));
        let ground = CmdPat::cut(Polarity::Negative, ProdPat::ctor("Zero", []), ConsPat::Top);
        let mut subst = Subst::new();
        assert!(
            !bool::from(match_cmd(&lhs, &ground, &mut subst)),
            "a positive cell does not apply at a negative cut"
        );
    }

    #[test]
    fn unification_finds_a_most_general_unifier()
    {
        // ⟨Succ(x) | α⟩ unifies with ⟨y | add(Zero; β)⟩ under x↦_, y↦Succ(x),
        // α↦add(Zero;β).
        let a = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("x")]),
            ConsPat::meta("a"),
        );
        let b = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("y"),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::meta("b")),
        );
        let mut subst = Subst::new();
        assert!(bool::from(unify_cmd(&a, &b, &mut subst)), "the cuts unify");
        assert_eq!(
            subst.apply_cmd(&a),
            subst.apply_cmd(&b),
            "the unifier equates both sides"
        );
    }

    #[test]
    fn the_occurs_check_rejects_a_cycle()
    {
        // x unified with Succ(x) must fail.
        let a = CmdPat::cut(Polarity::Positive, ProdPat::meta("x"), ConsPat::Top);
        let b = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("x")]),
            ConsPat::Top,
        );
        let mut subst = Subst::new();
        assert!(
            !bool::from(unify_cmd(&a, &b, &mut subst)),
            "x = Succ(x) is rejected by the occurs-check"
        );
    }

    #[test]
    fn unification_resolves_triangular_bindings()
    {
        // The crDC-suite finding (gandr-5lf.3): a binding whose image mentions
        // a metavariable bound *after* it (the goal order is not topological —
        // here `b2` binds first to an image mentioning x, and x binds
        // afterward) must be resolved to the fixpoint before the unifier is
        // returned, or single-pass application leaves `apply(a) != apply(b)`.
        let a = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("x"),
            ConsPat::op(
                "add",
                [ProdPat::ctor("Succ", [ProdPat::ctor("Succ", [
                    ProdPat::meta("x"),
                ])])],
                ConsPat::Top,
            ),
        );
        let b = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Cons", [
                ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                ProdPat::ctor("Succ", [ProdPat::ctor("Nil", [])]),
            ]),
            ConsPat::meta("b2"),
        );
        let mut subst = Subst::new();
        assert!(bool::from(unify_cmd(&a, &b, &mut subst)), "the cuts unify");
        assert_eq!(
            subst.apply_cmd(&a),
            subst.apply_cmd(&b),
            "the unifier equates both sides after one application pass"
        );
    }
}
