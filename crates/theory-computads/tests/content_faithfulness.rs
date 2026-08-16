//! Reusable executable witnesses for the [`CellAlphabet`] faithfulness
//! contract.

extern crate alloc;

use gandr_core_sequent::il::Polarity;
use gandr_theory_computads::Cell;
use gandr_theory_computads::CellAlphabet;
use gandr_theory_computads::PositionOrder;
use gandr_theory_computads::normal_form::prim_address;
use gandr_theory_computads::pattern::CmdPat;
use gandr_theory_computads::pattern::ConsPat;
use gandr_theory_computads::pattern::ProdPat;
use gandr_theory_computads::sequent::CellProvenance;
use gandr_theory_computads::sequent::Orientation;
use gandr_theory_computads::sequent::SequentAlphabet;

use crate::adversarial_alphabet::CollidingAddresses;
use crate::adversarial_alphabet::IncomparablePositions;
use crate::adversarial_alphabet::Lying;
use crate::toy_alphabet::Toy;
use crate::toy_alphabet::ToyAlphabet;
use crate::toy_alphabet::derived_toy_cell;
use crate::toy_alphabet::reoriented_toy_cell;
use crate::toy_alphabet::toy_cell;

fn assert_digest_distinguishes<A>(
    left: &Cell<A>,
    right: &Cell<A>,
) where
    A: CellAlphabet,
{
    assert_ne!(left, right, "the cells must describe distinct content");
    let root = A::root_position();
    assert_ne!(
        prim_address(left, &root),
        prim_address(right, &root),
        "the cell content is omitted from the local ordering digest"
    );
}

/// The two constructor names the sequent fixtures distinguish cells by.
#[derive(Clone, Copy)]
enum SequentCtor
{
    Zero,
    Succ,
}

fn sequent_cell(
    lhs: SequentCtor,
    rhs: SequentCtor,
    orientation: Orientation,
    provenance: CellProvenance,
) -> Cell<SequentAlphabet>
{
    let name = |ctor: SequentCtor| match ctor {
        | SequentCtor::Zero => "Zero",
        | SequentCtor::Succ => "Succ",
    };
    Cell::new(
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor(name(lhs), []),
            ConsPat::Top,
        ),
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor(name(rhs), []),
            ConsPat::Top,
        ),
        orientation,
        provenance,
    )
}

#[test]
fn production_alphabets_satisfy_content_faithfulness()
{
    let toy_lhs = toy_cell(Toy::Zero, Toy::Zero);
    assert_digest_distinguishes(&toy_lhs, &toy_cell(Toy::succ(Toy::Zero), Toy::Zero));
    assert_digest_distinguishes(&toy_lhs, &toy_cell(Toy::Zero, Toy::succ(Toy::Zero)));
    assert_digest_distinguishes(&toy_lhs, &reoriented_toy_cell(Toy::Zero, Toy::Zero));
    assert_digest_distinguishes(
        &reoriented_toy_cell(Toy::Zero, Toy::Zero),
        &derived_toy_cell(Toy::Zero, Toy::Zero),
    );

    let sequent_lhs = sequent_cell(
        SequentCtor::Zero,
        SequentCtor::Zero,
        Orientation::PolarityDerived,
        CellProvenance::SurfaceRule,
    );
    assert_digest_distinguishes(
        &sequent_lhs,
        &sequent_cell(
            SequentCtor::Succ,
            SequentCtor::Zero,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ),
    );
    assert_digest_distinguishes(
        &sequent_lhs,
        &sequent_cell(
            SequentCtor::Zero,
            SequentCtor::Succ,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ),
    );
    assert_digest_distinguishes(
        &sequent_lhs,
        &sequent_cell(
            SequentCtor::Zero,
            SequentCtor::Zero,
            Orientation::CompletionDerived,
            CellProvenance::SurfaceRule,
        ),
    );
    assert_digest_distinguishes(
        &sequent_lhs,
        &sequent_cell(
            SequentCtor::Zero,
            SequentCtor::Zero,
            Orientation::PolarityDerived,
            CellProvenance::MuMuTilde,
        ),
    );
}

#[test]
fn adversarial_alphabets_are_explicitly_exempt()
{
    let given = crate::adversarial_alphabet::lying_cell::<CollidingAddresses>(
        Toy::succ(Toy::Zero),
        Toy::Zero,
    );
    let derived = crate::adversarial_alphabet::reoriented_lying_cell::<CollidingAddresses>(
        Toy::succ(Toy::Zero),
        Toy::Zero,
    );
    assert_ne!(given, derived);
    assert_eq!(
        prim_address(
            &given,
            &<Lying<CollidingAddresses> as CellAlphabet>::root_position()
        ),
        prim_address(
            &derived,
            &<Lying<CollidingAddresses> as CellAlphabet>::root_position()
        ),
        "the named collision alphabet retains its obstruction witness"
    );

    let position = <Lying<IncomparablePositions> as CellAlphabet>::root_position();
    assert_eq!(
        PositionOrder::Incomparable,
        <Lying<IncomparablePositions> as CellAlphabet>::position_order(&position, &position),
        "the named irreflexive alphabet remains an explicit adversarial witness"
    );
}

#[test]
fn same_position_dependence_separates_multiplicity()
{
    let root = <ToyAlphabet as CellAlphabet>::root_position();
    assert_eq!(
        PositionOrder::Same,
        ToyAlphabet::position_order(&root, &root),
        "two occurrences at one position remain dependent"
    );

    let nested = crate::toy_alphabet::ToyPos(alloc::vec![0_usize].into_boxed_slice());
    assert_eq!(
        PositionOrder::Encloses,
        ToyAlphabet::position_order(&root, &nested),
        "the same primitive at nested positions receives a later causal layer"
    );
}

#[test]
fn mutations_are_rejected_by_the_fixture()
{
    // This is intentionally an ordinary positive fixture, not a mutant-specific
    // branch: deleting orientation/tag hashing makes the first assertion fail.
    let given = toy_cell(Toy::Zero, Toy::Zero);
    let reoriented = reoriented_toy_cell(Toy::Zero, Toy::Zero);
    assert_digest_distinguishes(&given, &reoriented);

    // Making position_order irreflexive changes Same to Incomparable and fails
    // the same contract assertion used by production alphabets.
    let root = <ToyAlphabet as CellAlphabet>::root_position();
    assert_eq!(
        PositionOrder::Same,
        ToyAlphabet::position_order(&root, &root)
    );
}
