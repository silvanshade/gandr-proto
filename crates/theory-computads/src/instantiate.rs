//! **Instantiating a circuit rule** — the site where the earned
//! shift-equivalence identification is well-posed.
//!
//! Spec: `spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-15` and `circuit-terms-spike-07` (the decided
//! guard); the placement is the schema finding recorded at
//! `circuit-terms-spike-07`'s as-built note.
//!
//! # Why the licence cannot be asked for at the declaration
//!
//! A circuit rule is a **schema**. Its body applies rewrite-*sorted ports* —
//! the binders `rule p : Nat ==> Nat` of its parameter telescope — not cells,
//! so a two-redex body names no pair of cells to ask the shift guard about.
//! Asking at the declaration would be asking whether *every* instantiation of
//! `p` and `q` commutes, which is a different and much stronger question than
//! the one the guard decides, and no answer to it is available from the
//! declaration.
//!
//! So [`gandr_theory_levitation::elaborate_body`] keeps declining a two-redex
//! body a single whiskered composite
//! ([`gandr_theory_levitation::CircuitElaborationError::ManyRedexOccurrences`]),
//! and that decline is not the place the identification is refused — it is the
//! place the question is deferred. This module is where it is *asked*: at an
//! **application**, where each port is instantiated by a stored cell, so the
//! pair is a pair of cells at two positions of one term and the guard's three
//! conjuncts are decidable.
//!
//! # What the record supplies, and what the caller supplies
//!
//! The two positions come from the block's own **occurrence record**
//! ([`gandr_theory_levitation::redex_occurrences`]): the argument-index paths
//! the declared output port's unfolding reaches its two redexes at, which is
//! where the whiskered composite exposes them. They are read, never fabricated
//! — a caller that could name the positions could name incomparable ones for a
//! nested pair and buy the licence with the argument instead of earning it.
//!
//! The caller supplies exactly two things the declaration cannot know: which
//! stored cell each rewrite port is instantiated by ([`RewriteBinding`]), and
//! the **peak** the rule is applied to.
//!
//! The bindings are an **environment**, so they are a *function* from port to
//! cell: each port is bound at most once, and a port presented twice is refused
//! ([`CircuitShiftObstruction::DuplicateBinding`]) rather than resolved by a
//! precedence rule. A precedence rule would make the identification depend on
//! the order two bindings were written in, which is not something the caller is
//! saying anything about — and the environment is checked before the occurrence
//! record is read, so an ill-formed one never reaches the guard.
//!
//! Neither the bindings nor the peak is checked against the rule's declared
//! sphere here, because there is no matching relation between a
//! [`gandr_theory_levitation::FreeTerm`] boundary and a
//! [`CellAlphabet::Cmd`] term; what happens instead is that the instance is
//! *exercised* — [`derive_shift_equivalence`] fires both orders — so a peak
//! that is not an instance of the block reaches
//! [`gandr_theory_deep_inference::shift::ShiftObstruction::StepDoesNotFire`]
//! rather than a silent identification. Whether a body's redex heads are the
//! telescope's ports at all is [`gandr_theory_levitation::check_desc`]'s check,
//! not this one's.
//!
//! # Nothing here decides independence
//!
//! Every conjunct is [`derive_shift_equivalence`]'s, and every refusal is
//! carried verbatim as the constructor's own
//! [`gandr_theory_deep_inference::shift::ShiftObstruction`]. In particular a
//! genuinely overlapping pair is refused here by the *overlap enumerator's*
//! verdict reaching this site through the shift guard — this module holds no
//! second overlap oracle, and the crate's one independence relation stays the
//! one [`gandr_theory_deep_inference::shift`] owns.
//!
//! The licence is also **confirmed rather than granted**: a witness that clears
//! the guard is replayed
//! ([`gandr_theory_deep_inference::shift::ShiftEquivalence::replay`], ADR-69)
//! before it is handed back, and a composite neither sequentialization reaches
//! is a refusal rather than a recorded identification.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::PositionStep;
use gandr_theory_cell_complexes::boundary::RedexOccurrenceCount;
use gandr_theory_cell_complexes::boundary::ShiftReplay;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;
use gandr_theory_coherent_resolutions::rewrite::CellApp;
use gandr_theory_deep_inference::shift::ShiftEquivalence;
use gandr_theory_deep_inference::shift::ShiftObstruction;
use gandr_theory_deep_inference::shift::derive_shift_equivalence;
use gandr_theory_levitation::CircuitDerivationError;
use gandr_theory_levitation::CircuitRule;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::RedexOccurrence;
use gandr_theory_levitation::redex_occurrences;

/// One entry of a circuit rule's **instantiation**: the stored cell a
/// rewrite-sorted port is applied at.
///
/// The port is named the way the body names it — a redex line's applied rewrite
/// — so the binding and the occurrence record meet on the same name. A slice of
/// these is an environment rather than a list: one entry per port, and a port
/// bound twice is refused
/// ([`CircuitShiftObstruction::DuplicateBinding`]). Two *occurrences* of one
/// port are the ordinary reconvergent case and resolve through that port's
/// single binding.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RewriteBinding
{
    /// The rewrite-sorted port being instantiated.
    pub port: Name,
    /// The stored cell it is instantiated by.
    pub cell: CellId,
}

impl RewriteBinding
{
    /// The binding of `port` to `cell`.
    #[inline]
    #[must_use]
    pub fn new<N>(
        port: N,
        cell: CellId,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            port: port.into(),
            cell,
        }
    }
}

/// The **earned identification** of an instantiated two-redex circuit rule's
/// two sequentializations.
///
/// Holding one is not the claim that the two readings are the same
/// transformation; it is the record of which guard granted the identification,
/// at which two applications, and that the composite was re-executed under both
/// orders before the record was handed back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitShift<A: CellAlphabet = SequentAlphabet>
{
    /// The witness the pair earned, spanning the peak and the composite.
    pub witness: ShiftEquivalence<A>,
    /// The replay verdict — positive by construction, because a composite that
    /// does not replay is
    /// [`CircuitShiftObstruction::CompositeDoesNotReplay`] rather than a
    /// [`CircuitShift`].
    pub replay: ShiftReplay,
}

/// Why an instantiated circuit rule earns **no** identification of its two
/// sequentializations.
///
/// Refusal is data. The first four variants are about the instantiation being
/// ill-formed — there is no pair to ask about at all — and the last two are
/// about the pair itself: the guard's own refusal, carried verbatim, and a
/// granted witness whose composite did not re-execute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitShiftObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// The instantiation binds one port **twice**, so it is not a function from
    /// port to cell and there is no one cell the port's occurrences resolve to.
    ///
    /// This is about the *environment*, never about the body: a port occurring
    /// twice in the wiring is reconvergence, and it resolves through that
    /// port's single binding.
    DuplicateBinding
    {
        /// The port bound more than once, in the order the bindings present
        /// them — the earliest port whose binding repeats.
        port: Name,
    },
    /// The body's wiring does not unfold, so there is no occurrence record to
    /// read positions from.
    Wiring(CircuitDerivationError),
    /// The body does not hold **exactly two** redex occurrences, so it is not
    /// the horizontal composite this site identifies: none or one is a rule
    /// whose composite [`gandr_theory_levitation::elaborate_body`] already
    /// builds, and three or more is a wider composite nothing here decomposes.
    NotTwoOccurrences
    {
        /// How many occurrences the body unfolds to.
        occurrences: RedexOccurrenceCount,
    },
    /// A redex applies a rewrite the instantiation binds no cell for, so one of
    /// the two applications has no cell to fire.
    UnboundRewrite
    {
        /// The rewrite the body applies and the instantiation does not bind.
        rewrite: Name,
    },
    /// The pair was **refused the witness** by the shift guard, and this is its
    /// own obstruction — a genuine overlap between the instantiating cells,
    /// comparable positions, an undischarged convexity conjunct, an
    /// unresolvable cell id, or a step that does not fire in this peak.
    Refused(Box<ShiftObstruction<A>>),
    /// The guard granted the witness and the composite did **not** replay, so
    /// the identification is not confirmed and is not handed back.
    CompositeDoesNotReplay
    {
        /// The unconfirmed witness, carried so the caller can see what was
        /// granted and what failed to re-execute.
        witness: Box<ShiftEquivalence<A>>,
    },
}

/// **Instantiate** a two-redex circuit rule at `peak` and earn — or refuse —
/// the identification of its two sequentializations.
///
/// The environment is checked to be a function from port to cell; the rule's
/// body is unfolded to its occurrence record; its two occurrences' argument
/// paths become the two positions ([`CellAlphabet::position_at_path`]); each
/// occurrence's rewrite is resolved to the cell `bindings` instantiates it by;
/// and the resulting pair of applications is put to
/// [`derive_shift_equivalence`] against `peak`. A pair that clears the guard is
/// replayed before it is returned.
///
/// # Contract
/// - requires: none.
/// - ensures: `Ok` exactly when `bindings` binds each port at most once, the
///   body unfolds to two redex occurrences, both their rewrites are bound, the
///   pair earns the shift guard's three conjuncts at the record's two positions
///   in `peak`, both sequentializations fire from `peak` to one composite, and
///   that composite replays under both orders. The returned witness's two
///   applications carry the record's positions and the instantiation's cells,
///   in the record's own left-to-right order; two occurrences of one port carry
///   that port's single cell at their two distinct positions.
/// - provides: the per-pair licence a two-redex circuit rule's horizontal
///   composite rests on, earned where the rule is applied rather than assumed
///   where it is declared.
/// - fails: [`CircuitShiftObstruction::DuplicateBinding`] when a port is bound
///   more than once; [`CircuitShiftObstruction::Wiring`] when the body does not
///   unfold; [`CircuitShiftObstruction::NotTwoOccurrences`] when it does not
///   hold exactly two redex occurrences;
///   [`CircuitShiftObstruction::UnboundRewrite`] when a redex's rewrite is not
///   instantiated; [`CircuitShiftObstruction::Refused`] carrying the shift
///   guard's own obstruction; and
///   [`CircuitShiftObstruction::CompositeDoesNotReplay`] when a granted witness
///   does not re-execute.
/// - panics: none.
/// - intension: the conditions are decided in this order — the environment,
///   then the wiring, then the occurrence count, then the bindings the record
///   asks for, then the guard, then the replay — so an instantiation failing
///   several is refused by the earliest, and the returned variant is that
///   observation. The occurrence record's order is the source reading's
///   left-to-right unfolding, so the witness's `first` is the leftmost
///   occurrence and the diagram order the block was written in is the order the
///   record reads in.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L3 — the two halves are separated pointwise. The
///   instantiation-shape declines are separated by a port bound twice, a
///   single-redex body (one occurrence), a two-redex body missing one binding,
///   and a cyclic body. The pair's own outcomes are separated by the `cong2`
///   instantiation, which earns the witness and replays; by an instantiation
///   whose two cells genuinely overlap, which is refused with the enumerator's
///   verdict reaching this site through the guard; and by an instantiation into
///   a peak that carries no redex at the record's positions, which is refused
///   by the instance check rather than by the guard — the last also pinning
///   that the positions are the record's rather than the root. The environment
///   condition is separated from the *body* by a reconvergent block, whose two
///   occurrences of one port resolve through that port's single binding at two
///   distinct positions: a repeated port in the record is not a repeated
///   binding.
/// - witness: `circuit_instantiation::tests::an_instantiated_cong2_rule_earns_its_shift_witness`
/// - witness: `circuit_instantiation::tests::the_instantiated_applications_carry_the_records_positions`
/// - witness: `circuit_instantiation::tests::a_genuinely_overlapping_instantiation_is_refused_at_the_application_site`
/// - witness: `circuit_instantiation::tests::a_sequential_two_redex_body_is_refused_comparable_positions`
/// - witness: `circuit_instantiation::tests::a_reconvergent_body_resolves_both_occurrences_through_one_binding`
/// - witness: `instantiate::tests::a_single_redex_rule_has_no_pair_to_identify`
/// - witness: `instantiate::tests::an_unbound_rewrite_port_declines_the_instantiation`
/// - witness: `instantiate::tests::a_port_bound_twice_is_not_a_functional_environment`
/// - witness: `instantiate::tests::a_cyclic_body_declines_before_any_pair_is_read`
/// - witness: `instantiate::tests::a_peak_carrying_no_redex_at_the_records_positions_is_refused`
#[inline]
pub fn instantiate_two_redex_rule<A>(
    store: &CellStore<A>,
    rule: &CircuitRule,
    bindings: &[RewriteBinding],
    peak: &A::Cmd,
) -> Result<CircuitShift<A>, CircuitShiftObstruction<A>>
where
    A: CellAlphabet,
{
    functional_environment(bindings)?;
    let occurrences = redex_occurrences(&rule.body).map_err(CircuitShiftObstruction::Wiring)?;
    let [ref first, ref second] = *occurrences
    else {
        return Err(CircuitShiftObstruction::NotTwoOccurrences {
            occurrences: RedexOccurrenceCount::from(occurrences.len()),
        });
    };
    let first = application::<A>(first, bindings)?;
    let second = application::<A>(second, bindings)?;
    let witness = derive_shift_equivalence(store, peak, &first, &second)
        .map_err(|obstruction| CircuitShiftObstruction::Refused(Box::new(obstruction)))?;
    let replay = witness.replay(store);
    if !bool::from(replay) {
        return Err(CircuitShiftObstruction::CompositeDoesNotReplay {
            witness: Box::new(witness),
        });
    }
    Ok(CircuitShift { witness, replay })
}

/// Check that the instantiation is a **function** from port to cell.
///
/// The check is over the presentation rather than over the pairs: a port
/// presented twice is refused whether or not the two entries name the same
/// cell, because two entries for one port say nothing about which the caller
/// meant and nothing distinguishes an intended rebinding from a duplicated
/// line.
///
/// # Contract
/// - ensures: `Ok(())` iff no two entries of `bindings` name the same port.
/// - fails: [`CircuitShiftObstruction::DuplicateBinding`] naming the earliest
///   port whose binding repeats, so the refusal does not depend on iteration
///   order.
/// - panics: none.
#[inline]
fn functional_environment<A>(bindings: &[RewriteBinding]) -> Result<(), CircuitShiftObstruction<A>>
where
    A: CellAlphabet,
{
    for (index, binding) in bindings.iter().enumerate() {
        if bindings
            .iter()
            .take(index)
            .any(|earlier| earlier.port == binding.port)
        {
            return Err(CircuitShiftObstruction::DuplicateBinding {
                port: binding.port.clone(),
            });
        }
    }
    Ok(())
}

/// The application one recorded occurrence instantiates to: the record's
/// position, and the cell the occurrence's rewrite is bound to.
///
/// # Contract
/// - requires: `bindings` is a function from port to cell
///   ([`functional_environment`] has passed), so the binding found is the only
///   one naming the occurrence's rewrite.
/// - ensures: `Ok(app)` whose position is [`CellAlphabet::position_at_path`] of
///   the occurrence's argument path and whose cell is the one its rewrite is
///   bound to; two occurrences of one rewrite therefore carry one cell at two
///   positions.
/// - fails: [`CircuitShiftObstruction::UnboundRewrite`] when no binding names
///   it.
/// - panics: none.
#[inline]
fn application<A>(
    occurrence: &RedexOccurrence,
    bindings: &[RewriteBinding],
) -> Result<CellApp<A>, CircuitShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let Some(binding) = bindings
        .iter()
        .find(|binding| binding.port == occurrence.rewrite)
    else {
        return Err(CircuitShiftObstruction::UnboundRewrite {
            rewrite: occurrence.rewrite.clone(),
        });
    };
    let path: Vec<PositionStep> = occurrence
        .position
        .iter()
        .copied()
        .map(|index| PositionStep::from(usize::from(index)))
        .collect();
    Ok(CellApp {
        cell: binding.cell,
        at: A::position_at_path(&path),
    })
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::cell::Cell;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::Pos;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_levitation::CircuitBody;
    use gandr_theory_levitation::CircuitFrame;
    use gandr_theory_levitation::CircuitNode;
    use gandr_theory_levitation::CircuitRedex;
    use gandr_theory_levitation::FrameHead;
    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::RuleFace;
    use gandr_theory_levitation::SurfaceSpan;

    use super::*;

    /// A face carrying the two terms, with no telescope and a zero span.
    fn face(
        lhs: FreeTerm,
        rhs: FreeTerm,
    ) -> RuleFace
    {
        RuleFace::new(
            lhs,
            rhs,
            Vec::new(),
            SurfaceSpan::new(0_usize.into(), 0_usize.into()),
        )
    }

    /// The `cong2` body: two redexes at the two argument positions of one
    /// `add` frame, which is the two-redex block this site exists for.
    fn cong2_body() -> CircuitBody
    {
        CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Redex(CircuitRedex::new(
                    "q",
                    FreeTerm::var("y"),
                    FreeTerm::var("y\u{2032}"),
                    "y\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y\u{2032}")],
                    "z",
                )),
            ],
            "z",
        )
    }

    /// A rule over `body`, declared at the sphere its wiring derives.
    fn rule_over(body: CircuitBody) -> CircuitRule
    {
        let derived = gandr_theory_levitation::derive_boundaries(&body)
            .expect("the fixture bodies derive their boundaries");
        CircuitRule::new("cong2", face(derived.source, derived.target), body)
    }

    /// (add-zero): `⟨Zero | add(Zero; ★)⟩ ~> ⟨Zero | ★⟩` — a ground cell whose
    /// right-hand side offers no seam, so the pair `(ground, ground)` has
    /// trivial overlap and the guard's first two conjuncts pass.
    fn add_zero_ground() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    #[test]
    fn a_single_redex_rule_has_no_pair_to_identify()
    {
        // The `cong1` body: one redex whiskered into one frame. It has a
        // whiskered composite of its own, so there is nothing here to identify.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y")],
                    "z",
                )),
            ],
            "z",
        );
        let mut store = CellStore::new();
        let ground = store.insert(add_zero_ground());
        let refusal = instantiate_two_redex_rule(
            &store,
            &rule_over(body),
            &[RewriteBinding::new("p", ground)],
            &CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
        )
        .expect_err("one occurrence is not a pair");
        assert_eq!(
            CircuitShiftObstruction::NotTwoOccurrences {
                occurrences: RedexOccurrenceCount::from(1_usize),
            },
            refusal,
            "the decline reports how many occurrences the body unfolds to"
        );
    }

    #[test]
    fn an_unbound_rewrite_port_declines_the_instantiation()
    {
        // `q` is applied by the body and instantiated by nothing, so the second
        // application has no cell to fire and the pair is never assembled.
        let mut store = CellStore::new();
        let ground = store.insert(add_zero_ground());
        let refusal = instantiate_two_redex_rule(
            &store,
            &rule_over(cong2_body()),
            &[RewriteBinding::new("p", ground)],
            &CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
        )
        .expect_err("an unbound port is not an instantiation");
        assert_eq!(
            CircuitShiftObstruction::UnboundRewrite {
                rewrite: "q".into(),
            },
            refusal,
            "the decline names the rewrite the instantiation left unbound"
        );
    }

    #[test]
    fn a_port_bound_twice_is_not_a_functional_environment()
    {
        // `p` is presented twice, at two different cells. There is no rule for
        // which one `p`'s occurrence resolves to, so the environment is refused
        // before the body is even unfolded — the body here is the perfectly good
        // `cong2` one.
        let mut store = CellStore::new();
        let ground = store.insert(add_zero_ground());
        let other = store.insert(Cell::new(
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let refusal = instantiate_two_redex_rule(
            &store,
            &rule_over(cong2_body()),
            &[
                RewriteBinding::new("p", ground),
                RewriteBinding::new("q", ground),
                RewriteBinding::new("p", other),
            ],
            &CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
        )
        .expect_err("a port bound twice is not an environment");
        assert_eq!(
            CircuitShiftObstruction::DuplicateBinding { port: "p".into() },
            refusal,
            "the refusal names the port whose binding repeats"
        );
    }

    #[test]
    fn a_cyclic_body_declines_before_any_pair_is_read()
    {
        // The wiring does not unfold, so there is no occurrence record and
        // therefore no positions to read; the derivation's own refusal is what
        // the site reports.
        let body = CircuitBody::new(
            [
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("f".into()),
                    [FreeTerm::var("b")],
                    "a",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("g".into()),
                    [FreeTerm::var("a")],
                    "b",
                )),
            ],
            "a",
        );
        let rule = CircuitRule::new("wheel", face(FreeTerm::var("a"), FreeTerm::var("a")), body);
        let refusal = instantiate_two_redex_rule::<SequentAlphabet>(
            &CellStore::new(),
            &rule,
            &[],
            &CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
        )
        .expect_err("a cyclic wiring unfolds to no record");
        assert_eq!(
            CircuitShiftObstruction::Wiring(CircuitDerivationError::CyclicWiring("a".into())),
            refusal,
            "the wiring refusal reaches the site unchanged"
        );
    }

    #[test]
    fn a_peak_carrying_no_redex_at_the_records_positions_is_refused()
    {
        // The guard passes — one cell that overlaps nothing, instantiating both
        // ports, at the record's two incomparable positions — and the instance
        // check declines: a sequent term has exactly one command position, so
        // nothing fires at the frame's argument slots. That the refusal is the
        // instance check rather than `ComparablePositions` is what pins the two
        // positions as the record's `[0]` and `[1]` rather than the root twice.
        let mut store = CellStore::new();
        let ground = store.insert(add_zero_ground());
        let refusal = instantiate_two_redex_rule(
            &store,
            &rule_over(cong2_body()),
            &[
                RewriteBinding::new("p", ground),
                RewriteBinding::new("q", ground),
            ],
            &CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
            ),
        )
        .expect_err("no redex sits at either recorded position");
        assert_eq!(
            CircuitShiftObstruction::Refused(Box::new(ShiftObstruction::StepDoesNotFire {
                step: Box::new(CellApp {
                    cell: ground,
                    at: Pos::from_indices([0_usize]),
                }),
            })),
            refusal,
            "the guard's own obstruction is carried, naming the step and its recorded position"
        );
    }
}
