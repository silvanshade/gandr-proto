//! The **cell-admission linearity boundary**.
//!
//! Cell patterns are linear (owner ruling, 2026-08-01;
//! `docs/gandr/spec/implementation/circuit-terms.md` §"The design questions",
//! `circuit-terms-question-17`).
//!
//! A metavariable occurring twice on a cell's left-hand side is a **copy on a
//! wire**: free in a term-shaped store, because substitution copies, but at the
//! circuit rung it needs a comonoid the type may not have. The ruling refuses
//! it, and the per-type cocommutative comonoid that would re-admit it is the
//! named later generalization (`circuit-terms-question-18`), not part of this
//! module.
//!
//! # Why the refusal lives here and not in the metadata derivation
//!
//! The check governs **admission**, never construction (owner decision,
//! 2026-08-02). [`crate::sequent::CellMeta::derive`] keeps *computing* the
//! per-hole metadata and rejects nothing, because non-linear command patterns
//! are legitimate internal shapes: the multi-sum contract witnesses exhibit a
//! genuinely non-linear pattern to show that composition at a seam is a family
//! rather than a single fused rule, and unification goals routinely carry a
//! repeated metavariable. What the ruling governs is which cells a description
//! may put **into a store** — so the refusal is invoked from the elaboration
//! path ([`crate::elaborate`]), on the cells a
//! [`gandr_theory_levitation::DataDesc`] contributes, and nowhere deeper.
//!
//! # A hole at two polarities is a seam, not a copy
//!
//! Holes are identified by **name** across a cell's two faces, so a name worn
//! by a producer *and* a consumer metavariable is one hole at two polarities —
//! the dinaturality seam the composition gate reads
//! ([`crate::compose::compose_directed`]). That shape is **not** a copy and is
//! **not** refused: the copy relation is per `(name, category)` pair, which is
//! exactly [`crate::pattern::MetaVar`]'s own equality.

use crate::alphabet::CellAlphabet;
use crate::cell::Cell;
use crate::pattern::Cat;
use crate::pattern::MetaVar;

/// A **refused non-linear cell pattern** — the admission diagnostic, naming the
/// copy (owner ruling, 2026-08-01).
///
/// The `copied` metavariable is the leftmost hole whose `(name, category)` pair
/// occurs more than once in the refused cell's left-hand side. Rendering it
/// ([`core::fmt::Display`]) names the hole *and* points at the respelling, so a
/// reader is told what to write instead rather than only that the rule was
/// rejected.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NonLinearPattern
{
    /// The copied hole — the metavariable whose `(name, category)` pair occurs
    /// more than once in the refused left-hand side.
    pub copied: MetaVar,
}

impl core::fmt::Display for NonLinearPattern
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        let side = match self.copied.cat {
            | Cat::Producer => "producer",
            | Cat::Consumer => "consumer",
        };
        write!(
            f,
            "non-linear cell pattern: the {side} hole `{name}` occurs more than once on the \
             left-hand side, which is a copy on a wire, and a copy needs a comonoid the type may \
             not have; cell patterns are linear. Respell the rule with the copy named: an \
             idempotence or cancellation law written with a repeated hole — `and(x, x) ~> x`, \
             `x - x ~> 0` — is written instead by matching through the copying cell, exactly as a \
             fan-in cell must name its monoid, and a type supplying a cocommutative comonoid may \
             host the copy explicitly.",
            side = side,
            name = self.copied.name,
        )
    }
}

impl core::error::Error for NonLinearPattern
{
}

/// The leftmost hole `cell`'s left-hand side **copies**, if any — the
/// alphabet-neutral half of the linearity boundary.
///
/// Copying is judged by the alphabet's own metavariable equality over the
/// left-to-right occurrence list ([`CellAlphabet::metavariables`], which
/// preserves repeats). For the sequent alphabet that equality is the
/// `(name, category)` pair, so a hole worn at both polarities — the
/// dinaturality seam — contributes one occurrence per polarity and is not a
/// copy.
///
/// # Contract
/// - requires: `A::metavariables` yields one entry per occurrence in
///   left-to-right order (the trait's own contract).
/// - ensures: `Some(var)` for the leftmost occurrence equal to a later
///   occurrence of the same left-hand side, `None` when every occurrence is
///   distinct; the right-hand side is never consulted, because linearity is a
///   redex-side condition.
/// - provides: the diagnostic payload the admission refusal names.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the three decision surfaces (repeat found,
///   polarity discrimination, side selection) are separated by one copied
///   pattern, one two-polarity seam, and one cell that is linear on the left
///   and repeats on the right.
/// - witness: `linearity::tests::a_repeated_producer_hole_is_the_copy`
/// - witness: `linearity::tests::a_hole_at_both_polarities_is_not_a_copy`
/// - witness: `linearity::tests::a_repeat_on_the_right_hand_side_is_not_a_copy`
#[inline]
#[must_use]
pub fn copied_hole<A>(cell: &Cell<A>) -> Option<A::Var>
where
    A: CellAlphabet,
{
    let occurrences = A::metavariables(&cell.lhs);
    for (index, var) in occurrences.iter().enumerate() {
        let rest = occurrences.iter().skip(index.saturating_add(1));
        let mut repeats = rest.filter(|other| *other == var);
        if repeats.next().is_some() {
            return Some(var.clone());
        }
    }
    None
}

/// The **admission boundary** — admit `cell` only when its left-hand side
/// copies no hole (owner ruling, 2026-08-01).
///
/// This is the reusable refusal every description-sourced cell path runs
/// before a cell enters a store; [`crate::elaborate::elaborate_data_desc`] is
/// its caller today. It rejects nothing that
/// [`crate::sequent::CellMeta::derive`] computes: metadata derivation and
/// admission are deliberately separate, so internally-constructed non-linear
/// patterns (multi-sum contract witnesses, unification goals) stay
/// constructible.
///
/// # Contract
/// - ensures: `Ok(())` exactly when [`copied_hole`] finds no copy; the cell is
///   neither read nor modified otherwise.
/// - fails: [`NonLinearPattern`] naming the leftmost copied hole, rendering to
///   a diagnostic that names the hole and the respelling.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the decision surface is one predicate, so one
///   refused copy (whose diagnostic is asserted to name the hole and the
///   respelling) and one admitted two-polarity seam separate it.
/// - witness: `linearity::tests::the_diagnostic_names_the_copy_and_the_respelling`
/// - witness: `linearity::tests::a_hole_at_both_polarities_is_admitted`
/// - witness: `linearity::tests::a_linear_cell_is_admitted`
#[inline]
pub fn admit_linear_cell(cell: &Cell) -> Result<(), NonLinearPattern>
{
    match copied_hole(cell) {
        | Some(copied) => Err(NonLinearPattern { copied }),
        | None => Ok(()),
    }
}

#[cfg(test)]
mod tests
{
    use alloc::format;

    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::boundary::CellInvertibility;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;

    /// A surface-rule cell over the two given faces.
    fn rule_cell(
        lhs: CmdPat,
        rhs: CmdPat,
    ) -> Cell
    {
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// `⟨x | and(x; α)⟩` — the elaborated shape of `rule and(x, x) ~> x`.
    fn idempotence_lhs() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("x"),
            ConsPat::op("and", [ProdPat::meta("x")], ConsPat::meta("alpha")),
        )
    }

    /// `⟨r | seam(; r)⟩` — one hole worn at both polarities (the seam).
    fn seam_lhs() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("r"),
            ConsPat::op("seam", [], ConsPat::meta("r")),
        )
    }

    #[test]
    fn a_repeated_producer_hole_is_the_copy()
    {
        let cell = rule_cell(
            idempotence_lhs(),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::meta("alpha"),
            ),
        );
        let copied = copied_hole(&cell).expect("the repeated hole is found");
        assert_eq!(&*copied.name, "x", "the copy is named");
        assert_eq!(Cat::Producer, copied.cat, "the copy is a producer hole");
    }

    #[test]
    fn a_hole_at_both_polarities_is_not_a_copy()
    {
        // `r` is worn by a producer metavariable and a consumer metavariable:
        // one hole at two polarities, so the copy relation — which is per
        // `(name, category)` — sees two distinct occurrences, not a repeat.
        let cell = rule_cell(seam_lhs(), seam_lhs());
        assert_eq!(None, copied_hole(&cell), "a seam is not a copy");
    }

    #[test]
    fn a_repeat_on_the_right_hand_side_is_not_a_copy()
    {
        // `⟨x | dup(; α)⟩ ~> ⟨Pair(x; x) | α⟩` — linearity is a redex-side
        // condition, so duplication in the contractum is admitted.
        let cell = rule_cell(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("dup", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("x")]),
                ConsPat::meta("alpha"),
            ),
        );
        assert_eq!(
            None,
            copied_hole(&cell),
            "only the left-hand side is consulted"
        );
    }

    #[test]
    fn the_diagnostic_names_the_copy_and_the_respelling()
    {
        let cell = rule_cell(
            idempotence_lhs(),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::meta("alpha"),
            ),
        );
        let refusal = admit_linear_cell(&cell).expect_err("a copied hole is refused");
        assert_eq!(
            &*refusal.copied.name, "x",
            "the refusal carries the copied hole"
        );
        let diagnostic = format!("{refusal}");
        assert!(
            diagnostic.contains("the producer hole `x`"),
            "the diagnostic names the copy: {diagnostic}"
        );
        assert!(
            diagnostic.contains("and(x, x) ~> x"),
            "the diagnostic points at the idempotence respelling: {diagnostic}"
        );
        assert!(
            diagnostic.contains("x - x ~> 0"),
            "the diagnostic points at the cancellation respelling: {diagnostic}"
        );
        assert!(
            diagnostic.contains("cocommutative comonoid"),
            "the diagnostic names the hosting generalization: {diagnostic}"
        );
    }

    #[test]
    fn a_hole_at_both_polarities_is_admitted()
    {
        let cell = rule_cell(seam_lhs(), seam_lhs());
        assert_eq!(
            Ok(()),
            admit_linear_cell(&cell),
            "the dinaturality seam is admitted"
        );
        let meta =
            crate::sequent::CellMeta::derive(&cell.lhs, &cell.rhs, CellInvertibility::from(false));
        assert!(
            meta.vars.iter().all(|var| bool::from(var.linear)),
            "and the derived metadata agrees that the seam is linear"
        );
    }

    #[test]
    fn a_linear_cell_is_admitted()
    {
        let cell = rule_cell(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("m"),
                ConsPat::op(
                    "add",
                    [ProdPat::meta("n")],
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
            ),
        );
        assert_eq!(
            Ok(()),
            admit_linear_cell(&cell),
            "every hole occurs once on the left"
        );
    }
}
