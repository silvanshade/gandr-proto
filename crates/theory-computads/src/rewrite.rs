//! **Direct command-level rewriting** — applying cells to command
//! configurations and normalizing under a budget.
//!
//! Spec: `proposal-sequent-kernel.md` §7.3.4, §4.1 — the "direct command-level
//! rewriting" the fused≡two-step differential runs on.
//!
//! Because the cell-visible fragment has no nested commands (a cut's children
//! are a producer and a consumer, never another command — [`crate::pattern`]),
//! every redex sits at a **command position**, and for a single cut that is the
//! root. [`apply_once`] tries each cell at each command position (deterministic
//! cell order, outermost position first); [`normalize`] iterates to a normal
//! form or a **budget** exhaustion — the same decline-and-report posture as the
//! completion budget (§7.3.3) and the machine's step budget.
//!
//! # η-polarity discipline
//!
//! An η cell ([`crate::cell::CellProvenance::Eta`]) may fire **only** at a cut
//! whose polarity matches its [`crate::cell::EtaKind`] (§5, K2: data η at
//! positive cuts, codata η at negative cuts). [`rewrite_at`] checks this before
//! contracting, so an
//! η-step at the wrong polarity is rejected — the `eta-wrong-polarity`
//! pathological pin (§11).

use alloc::vec::Vec;

use crate::boundary::NormalizationBudget;
use crate::boundary::PositionStep;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::pattern::CmdPat;
use crate::pattern::Node;
use crate::pattern::Pos;
use crate::pattern::splice_at;
use crate::pattern::subterm_at;
use crate::subst::Subst;
use crate::subst::match_cmd;

/// One **rewrite step** — a cell applied at a position (`CellApp` of the
/// tracelet sketch, `proposal-sequent-kernel.md` §7.3).
///
/// A step records only *which* cell fired *where*, never the matched
/// substitution: replay ([`crate::tracelet`]) re-matches and re-contracts, so a
/// certificate is re-executed rather than trusted (ADR-69).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CellApp
{
    /// The cell that fired.
    pub cell: CellId,
    /// The position it fired at.
    pub at: Pos,
}

/// The result of a single successful rewrite — the step and the rewritten term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rewrite
{
    /// The step that fired.
    pub step: CellApp,
    /// The whole term after the contraction.
    pub result: CmdPat,
}

/// The outcome of [`normalize`] — the normal form, the path taken, and whether
/// the budget was exhausted before a normal form was reached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Normalization
{
    /// The term reached (a normal form when `!exhausted`).
    pub normal: CmdPat,
    /// The rewrite path from the input to `normal`.
    pub path: Vec<CellApp>,
    /// Whether the budget ran out before a normal form was reached.
    pub exhausted: bool,
}

/// Normalize `term` under `store`, taking at most `budget` steps.
///
/// # Contract
/// - requires: the ground/skolemized fragment (matching binds every LHS
///   metavariable).
/// - ensures: the returned [`Normalization`] reaches a normal form with
///   `exhausted == false` within `budget` steps, or stops at `budget` steps
///   with `exhausted == true`; `path` records every fired step in order and
///   re-running it reproduces `normal`. Never diverges or panics (the budget is
///   the guard).
/// - panics: none.
#[inline]
#[must_use]
pub fn normalize(
    store: &CellStore,
    term: &CmdPat,
    budget: NormalizationBudget,
) -> Normalization
{
    let mut current = term.clone();
    let mut path = Vec::new();
    let mut remaining = usize::from(budget);
    loop {
        if remaining == 0 {
            let exhausted = apply_once(store, &current).is_some();
            return Normalization {
                normal: current,
                path,
                exhausted,
            };
        }
        match apply_once(store, &current) {
            | Some(rewrite) => {
                path.push(rewrite.step);
                current = rewrite.result;
                remaining = remaining.saturating_sub(1);
            },
            | None => {
                return Normalization {
                    normal: current,
                    path,
                    exhausted: false,
                };
            },
        }
    }
}

/// Fire the first applicable cell at the outermost command position.
///
/// # Contract
/// - ensures: `Some(rewrite)` for the first `(position, cell)` pair — outermost
///   position, then store order — whose [`rewrite_at`] succeeds; `None` when
///   the term is a normal form (no cell fires anywhere).
/// - panics: none.
#[inline]
#[must_use]
pub fn apply_once(
    store: &CellStore,
    term: &CmdPat,
) -> Option<Rewrite>
{
    for pos in command_positions(term) {
        for (id, cell) in store.iter() {
            if let Some(result) = rewrite_at(cell, term, &pos) {
                return Some(Rewrite {
                    step: CellApp { cell: id, at: pos },
                    result,
                });
            }
        }
    }
    None
}

/// Every **command position** in `term` — the positions a command cell can fire
/// at (`proposal-sequent-kernel.md` §7.3.2, "the seam is the root").
///
/// # Contract
/// - ensures: the returned positions each address a [`Node::Cmd`] subterm, in a
///   deterministic outer-to-inner order; for a single cut this is exactly the
///   root.
/// - panics: none.
#[inline]
#[must_use]
pub fn command_positions(term: &CmdPat) -> Vec<Pos>
{
    let mut out = Vec::new();
    let mut work = alloc::vec![(Pos::root(), Node::Cmd(term.clone()))];
    while let Some((pos, node)) = work.pop() {
        if matches!(node, Node::Cmd(_)) {
            out.push(pos.clone());
        }
        for (index, child) in node.children().into_iter().enumerate() {
            work.push((pos.child(PositionStep::from(index)), child));
        }
    }
    out.sort_unstable_by_key(|pos| pos.as_ref().len());
    out
}

/// Try to fire `cell` at `pos` in `term`, returning the rewritten term.
///
/// # Contract
/// - ensures: `Some(result)` when the subterm at `pos` is a command matching
///   `cell.lhs` (ground matching, polarity included) **and** the η-polarity
///   discipline permits it; `None` when there is no redex, the position is not
///   a command, or an η cell would fire at the wrong-polarity cut.
/// - panics: none.
#[inline]
#[must_use]
pub fn rewrite_at(
    cell: &Cell,
    term: &CmdPat,
    pos: &Pos,
) -> Option<CmdPat>
{
    let Node::Cmd(sub) = subterm_at(&Node::Cmd(term.clone()), pos)?
    else {
        return None;
    };
    // η-polarity discipline (§5): an η cell fires only at its required polarity.
    if let Some(required) = cell.eta_requirement()
        && sub.polarity() != required
    {
        return None;
    }
    let mut subst = Subst::new();
    if !bool::from(match_cmd(&cell.lhs, &sub, &mut subst)) {
        return None;
    }
    let contractum = subst.apply_cmd(&cell.rhs);
    let Node::Cmd(rebuilt) = splice_at(&Node::Cmd(term.clone()), pos, Node::Cmd(contractum))?
    else {
        return None;
    };
    Some(rebuilt)
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::cell::CellProvenance;
    use crate::cell::EtaKind;
    use crate::cell::Orientation;
    use crate::cell::frame_defining_cell;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;

    #[test]
    fn a_frame_defining_cell_fires_at_the_root()
    {
        // ⟨Zero | Succ⁻(★)⟩ ~> ⟨Succ(Zero) | ★⟩.
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let out = normalize(&store, &term, 16_usize.into());
        let expected = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
            ConsPat::Top,
        );
        assert_eq!(out.normal, expected, "the μ̃ reduction wrapped Zero in Succ");
        assert_eq!(1, out.path.len(), "exactly one step");
        assert!(!out.exhausted, "reached a normal form");
    }

    #[test]
    fn an_eta_cell_is_rejected_at_the_wrong_polarity()
    {
        // A data-η cell (requires positive) built over a negative cut is refused.
        let lhs = CmdPat::cut(Polarity::Negative, ProdPat::meta("x"), ConsPat::meta("a"));
        let rhs = lhs.clone();
        let eta = Cell::new(
            lhs.clone(),
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::Eta(EtaKind::Data),
        );
        // The target is a negative cut; data-η requires positive.
        assert_eq!(
            None,
            rewrite_at(&eta, &lhs, &Pos::root()),
            "data-η must not fire at a negative cut"
        );
    }

    #[test]
    fn a_budget_of_zero_reports_a_pending_redex()
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let out = normalize(&store, &term, 0_usize.into());
        assert!(out.exhausted, "a redex remained but the budget was zero");
        assert_eq!(0, out.path.len(), "no step was taken");
    }
}
