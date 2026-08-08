//! **Adversarial [`CellAlphabet`] inhabitants** — alphabets that withhold
//! exactly one of the answers the shift guard's conjuncts rest on, so the
//! tracelet normal form's defensive checks become falsifiable.
//!
//! # Why an alphabet, rather than a fixture
//!
//! The shift guard decides independence from three things: the alphabet's
//! [`CellAlphabet::position_order`], the overlap enumerator's answer about the
//! cell pair, and the alphabet's [`CellAlphabet::convexity_discharge`]. Two of
//! those three are supplied by the *alphabet*, and both alphabets shipped in
//! this tree answer them honestly and unconditionally — the sequent alphabet
//! and the toy alphabet each compute `position_order` from the position path
//! and each discharge convexity for every store. So no fixture over a shipped
//! alphabet can reach the code that runs when one of those answers is wrong,
//! and three of the tracelet normal form's own `- fails:` modes — the two
//! kill-signal refusals and the convexity conjunct's contribution — have no
//! witness at all. That is a **missing-input** gap whose fix is an input, not
//! an assertion: a new alphabet.
//!
//! Each inhabitant here is [`ToyAlphabet`] in every respect but one, and the
//! one is named in its own documentation. That is deliberate: a refusal a
//! fixture obtains over `Lying<L>` is attributable to `L`'s single lie, because
//! nothing else about the alphabet moved.
//!
//! # These alphabets are contract violations, on purpose
//!
//! An `AlphabetLie` breaks a `- requires:` or `- ensures:` clause that
//! [`CellAlphabet`] states, and it is only ever instantiated inside this test
//! crate. That is the point rather than a caveat: the tracelet normal form
//! replays its own canonical schedule *because* the guard's soundness is
//! conditional on those clauses, and a check defending against a broken
//! implementor cannot be exercised by a correct one.
//!
//! [`CellAlphabet::position_order`]: gandr_theory_computads::CellAlphabet::position_order
//! [`CellAlphabet::convexity_discharge`]: gandr_theory_computads::CellAlphabet::convexity_discharge

pub use tests::CollidingAddresses;
pub use tests::IncomparablePositions;
pub use tests::Lying;
pub use tests::NonLocalSplice;
pub use tests::WithheldConvexity;
pub use tests::lying_cell;
pub use tests::reoriented_lying_cell;

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;
    use core::marker::PhantomData;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellAlphabet;
    use gandr_theory_computads::CellInvertibility;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::ConvexityDischarge;
    use gandr_theory_computads::FiringPermission;
    use gandr_theory_computads::PositionOrder;
    use gandr_theory_computads::SeamRole;
    use gandr_theory_computads::SubstitutionDecision;
    use gandr_theory_computads::rewrite::rewrite_at;

    use crate::toy_alphabet::Toy;
    use crate::toy_alphabet::ToyAlphabet;
    use crate::toy_alphabet::ToyPos;
    use crate::toy_alphabet::toy_cell;

    extern crate alloc;

    /// The one question a [`Lying`] alphabet answers dishonestly.
    ///
    /// Every method of the trait has an honest default, so an inhabitant
    /// overrides exactly the one it lies about and the resulting alphabet
    /// differs from [`ToyAlphabet`] in that answer alone.
    ///
    /// The honest defaults delegate to [`ToyAlphabet`] where the signature
    /// allows it. [`AlphabetLie::convexity_discharge`] cannot delegate — its
    /// [`CellAlphabet`] counterpart is keyed by a `CellStore<Self>`, and a
    /// `CellStore<Lying<L>>` is not a `CellStore<ToyAlphabet>` — so it restates
    /// the constant the toy alphabet's own implementation returns, which that
    /// implementation documents as unconditional for a first-order term
    /// language.
    pub trait AlphabetLie:
        Clone + Copy + Default + Eq + Ord + core::fmt::Debug + core::hash::Hash
    {
        /// The order this alphabet reports for a position pair.
        #[must_use]
        fn position_order(
            left: &ToyPos,
            right: &ToyPos,
        ) -> PositionOrder
        {
            ToyAlphabet::position_order(left, right)
        }

        /// The convexity warrant this alphabet supplies for any store.
        #[must_use]
        fn convexity_discharge() -> ConvexityDischarge
        {
            ConvexityDischarge::LeftConnectedOverAcyclicTarget
        }

        /// Replace the subterm at `pos`, as [`CellAlphabet::splice_cmd_at`]
        /// requires.
        #[must_use]
        fn splice_cmd_at(
            cmd: &Toy,
            pos: &ToyPos,
            replacement: Toy,
        ) -> Option<Toy>
        {
            ToyAlphabet::splice_cmd_at(cmd, pos, replacement)
        }

        /// Feed an orientation tag to a hasher.
        ///
        /// The honest answer streams the tag itself, so a cell over a lying
        /// alphabet hashes exactly as the toy cell it was built from.
        fn hash_orientation<H>(
            orient: ToyOrient,
            state: &mut H,
        ) where
            H: core::hash::Hasher,
        {
            core::hash::Hash::hash(&orient, state);
        }
    }

    /// The toy alphabet's orientation tag, named through the trait because the
    /// toy module does not export it.
    type ToyOrient = <ToyAlphabet as CellAlphabet>::Orientation;

    /// An orientation tag whose [`core::hash::Hash`] is `L`'s to decide.
    ///
    /// It exists so an inhabitant can make two structurally *distinct* cells
    /// hash identically without touching anything else about them —
    /// [`CellAlphabet::Orientation`] enters [`gandr_theory_computads::Cell`]'s
    /// derived hash and nothing else in the trait's surface, which makes it the
    /// smallest place to put a legal digest collision.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct LyingOrient<L>(pub ToyOrient, pub PhantomData<L>);

    impl<L> From<ToyOrient> for LyingOrient<L>
    {
        fn from(orient: ToyOrient) -> Self
        {
            Self(orient, PhantomData)
        }
    }

    impl<L> core::hash::Hash for LyingOrient<L>
    where
        L: AlphabetLie,
    {
        fn hash<H>(
            &self,
            state: &mut H,
        ) where
            H: core::hash::Hasher,
        {
            L::hash_orientation(self.0, state);
        }
    }

    /// An alphabet that reports **every** position pair as
    /// [`PositionOrder::Incomparable`], including a nesting pair and a position
    /// with itself.
    ///
    /// It breaks [`CellAlphabet::position_order`]'s `- requires:` — that
    /// incomparable positions address disjoint subtrees — which is the one
    /// guard premise the shift guard takes on trust and never checks. An
    /// application that encloses another is thereby licensed to commute with
    /// it, so the canonical schedule may place the outer application first,
    /// where its redex does not yet exist.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct IncomparablePositions;

    impl AlphabetLie for IncomparablePositions
    {
        fn position_order(
            _left: &ToyPos,
            _right: &ToyPos,
        ) -> PositionOrder
        {
            PositionOrder::Incomparable
        }
    }

    /// An alphabet that **withholds** the convexity warrant, answering
    /// [`ConvexityDischarge::ReCheckRequired`] for every store.
    ///
    /// Unlike the other two inhabitants this one breaks no clause: withholding
    /// the warrant is a legitimate answer that
    /// [`CellAlphabet::convexity_discharge`] explicitly provides for, and an
    /// alphabet whose left-hand sides could be matched non-convexly is obliged
    /// to give it. It is adversarial only in the sense that no *shipped*
    /// alphabet gives it, so the third guard conjunct's effect on the normal
    /// form has no other input that can reach it.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct WithheldConvexity;

    impl AlphabetLie for WithheldConvexity
    {
        fn convexity_discharge() -> ConvexityDischarge
        {
            ConvexityDischarge::ReCheckRequired
        }
    }

    /// An alphabet whose splice is **non-local**: replacing the subterm at a
    /// one-step position under a binary root also resets that position's
    /// sibling to `Add(Zero, Zero)`.
    ///
    /// It breaks [`CellAlphabet::splice_cmd_at`]'s `- ensures:` — that the
    /// result is `cmd` with the subterm at `pos` replaced and nothing else —
    /// and it is the only inhabitant here whose lie the guard cannot see even
    /// in principle. The guard reads positions and cell contents; a term
    /// algebra where a rewrite at one address changes another is invisible to
    /// both, so two applications at genuinely incomparable positions over cells
    /// with genuinely trivial overlap can still fail to commute. That is the
    /// precise defect [`gandr_theory_computads::normal_form`]'s canonical
    /// replay exists to catch, and the only route to it that does not wait on
    /// the overlap enumerator's metavariable-seam over-approximation.
    ///
    /// The lie is narrow by construction: only a `[i]` position under a binary
    /// root is entangled, so the honest splices [`ToyAlphabet::skolemize`] and
    /// [`ToyAlphabet::apply_subst`] perform internally are untouched (they call
    /// the toy alphabet's own splice, not this one).
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct NonLocalSplice;

    impl AlphabetLie for NonLocalSplice
    {
        fn splice_cmd_at(
            cmd: &Toy,
            pos: &ToyPos,
            replacement: Toy,
        ) -> Option<Toy>
        {
            let spliced = ToyAlphabet::splice_cmd_at(cmd, pos, replacement)?;
            let [index] = *pos.0
            else {
                return Some(spliced);
            };
            if !matches!(*cmd, Toy::Add(..)) {
                return Some(spliced);
            }
            let sibling = ToyPos(alloc::vec![1_usize.saturating_sub(index)].into_boxed_slice());
            ToyAlphabet::splice_cmd_at(&spliced, &sibling, Toy::add(Toy::Zero, Toy::Zero))
        }
    }

    /// An alphabet whose orientation tag hashes to **nothing**, so two cells
    /// that differ only in orientation take the same content address.
    ///
    /// It breaks no clause either. [`core::hash::Hash`]'s own contract is
    /// one-directional — equal values must hash equally, unequal values may —
    /// and [`gandr_theory_computads::prim_address`]'s `- intension:` says in as
    /// many words that distinct inputs may in principle collide. So a
    /// degenerate-but-legal tag hash is the honest triggering input for
    /// [`gandr_theory_computads::NormalFormObstruction::ContentAddressCollision`],
    /// whose arm was twice recorded as unreachable by construction on the
    /// grounds that a [`CellStore`] deduplicating on structural equality gives
    /// one content one identifier. That premise is true and does not close the
    /// arm: two cells differing only in a field the digest cannot see are two
    /// identifiers whose *contents hash the same*, which is exactly what the
    /// arm is written to refuse.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct CollidingAddresses;

    impl AlphabetLie for CollidingAddresses
    {
        fn hash_orientation<H>(
            _orient: ToyOrient,
            _state: &mut H,
        ) where
            H: core::hash::Hasher,
        {
        }
    }

    /// [`ToyAlphabet`], except for the one answer `L` withholds.
    ///
    /// Every associated type is the toy alphabet's own type, so a
    /// `Cell<Lying<L>>` built by [`lying_cell`] is field-for-field a
    /// `Cell<ToyAlphabet>` and hashes identically — which is what lets a
    /// fixture over a lying alphabet predict the canonical schedule's
    /// content-address tie-break from the toy suite's.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Lying<L>(PhantomData<L>);

    impl<L> CellAlphabet for Lying<L>
    where
        L: AlphabetLie,
    {
        type Cmd = <ToyAlphabet as CellAlphabet>::Cmd;
        type Hole = <ToyAlphabet as CellAlphabet>::Hole;
        type Meta = <ToyAlphabet as CellAlphabet>::Meta;
        type Orientation = LyingOrient<L>;
        type Pos = <ToyAlphabet as CellAlphabet>::Pos;
        type Provenance = <ToyAlphabet as CellAlphabet>::Provenance;
        type Subst = <ToyAlphabet as CellAlphabet>::Subst;
        type Var = <ToyAlphabet as CellAlphabet>::Var;

        fn match_cmd(
            pattern: &Self::Cmd,
            target: &Self::Cmd,
            subst: &mut Self::Subst,
        ) -> SubstitutionDecision
        {
            ToyAlphabet::match_cmd(pattern, target, subst)
        }

        fn unify_cmd(
            lhs: &Self::Cmd,
            rhs: &Self::Cmd,
            subst: &mut Self::Subst,
        ) -> SubstitutionDecision
        {
            ToyAlphabet::unify_cmd(lhs, rhs, subst)
        }

        fn apply_subst(
            subst: &Self::Subst,
            cmd: &Self::Cmd,
        ) -> Self::Cmd
        {
            ToyAlphabet::apply_subst(subst, cmd)
        }

        fn metavariables(cmd: &Self::Cmd) -> Vec<Self::Var>
        {
            ToyAlphabet::metavariables(cmd)
        }

        fn command_positions(cmd: &Self::Cmd) -> Vec<Self::Pos>
        {
            ToyAlphabet::command_positions(cmd)
        }

        fn root_position() -> Self::Pos
        {
            ToyAlphabet::root_position()
        }

        fn position_order(
            left: &Self::Pos,
            right: &Self::Pos,
        ) -> PositionOrder
        {
            L::position_order(left, right)
        }

        fn convexity_discharge(_store: &CellStore<Self>) -> ConvexityDischarge
        {
            L::convexity_discharge()
        }

        fn subterm_cmd_at(
            cmd: &Self::Cmd,
            pos: &Self::Pos,
        ) -> Option<Self::Cmd>
        {
            ToyAlphabet::subterm_cmd_at(cmd, pos)
        }

        fn splice_cmd_at(
            cmd: &Self::Cmd,
            pos: &Self::Pos,
            replacement: Self::Cmd,
        ) -> Option<Self::Cmd>
        {
            L::splice_cmd_at(cmd, pos, replacement)
        }

        fn reduction_cmp(
            lhs: &Self::Cmd,
            rhs: &Self::Cmd,
        ) -> core::cmp::Ordering
        {
            ToyAlphabet::reduction_cmp(lhs, rhs)
        }

        fn rename_apart(
            anchor: (&Self::Cmd, &Self::Cmd),
            renamed: (&Self::Cmd, &Self::Cmd),
        ) -> (Self::Cmd, Self::Cmd)
        {
            ToyAlphabet::rename_apart(anchor, renamed)
        }

        fn skolemize(cmd: &Self::Cmd) -> Self::Cmd
        {
            ToyAlphabet::skolemize(cmd)
        }

        fn hole_of(var: &Self::Var) -> Self::Hole
        {
            ToyAlphabet::hole_of(var)
        }

        fn completion_certificate(provenance: &Self::Provenance) -> CellInvertibility
        {
            ToyAlphabet::completion_certificate(provenance)
        }

        fn derive_meta(
            lhs: &Self::Cmd,
            rhs: &Self::Cmd,
            invertible: CellInvertibility,
        ) -> Self::Meta
        {
            ToyAlphabet::derive_meta(lhs, rhs, invertible)
        }

        fn hole_flow(
            meta: &Self::Meta,
            hole: &Self::Hole,
        ) -> Vec<(Self::Var, SeamRole)>
        {
            ToyAlphabet::hole_flow(meta, hole)
        }

        fn may_fire(
            provenance: &Self::Provenance,
            target: &Self::Cmd,
        ) -> FiringPermission
        {
            ToyAlphabet::may_fire(provenance, target)
        }

        fn derived_orientation() -> Self::Orientation
        {
            LyingOrient::from(ToyAlphabet::derived_orientation())
        }

        fn derived_provenance() -> Self::Provenance
        {
            ToyAlphabet::derived_provenance()
        }
    }

    /// A toy rule cell over a [`Lying`] alphabet.
    ///
    /// It is re-typed from [`toy_cell`] field for field rather than rebuilt, so
    /// the orientation and provenance are the toy alphabet's own values and the
    /// cell's [`core::hash::Hash`] — hence every
    /// [`gandr_theory_computads::prim_address`] taken over it — agrees with the
    /// toy suite's.
    #[must_use]
    pub fn lying_cell<L>(
        lhs: Toy,
        rhs: Toy,
    ) -> Cell<Lying<L>>
    where
        L: AlphabetLie,
    {
        let template = toy_cell(lhs, rhs);
        Cell::new(
            template.lhs,
            template.rhs,
            LyingOrient::from(template.orient),
            template.provenance,
        )
    }

    /// The same cell as [`lying_cell`], tagged with the **derived** orientation
    /// instead of the given one.
    ///
    /// The two are structurally distinct cells, so one store holds both under
    /// two identifiers; under [`CollidingAddresses`] they nonetheless take one
    /// content address, which is the only way a fixture can reach
    /// [`gandr_theory_computads::NormalFormObstruction::ContentAddressCollision`].
    #[must_use]
    pub fn reoriented_lying_cell<L>(
        lhs: Toy,
        rhs: Toy,
    ) -> Cell<Lying<L>>
    where
        L: AlphabetLie,
    {
        let template = toy_cell(lhs, rhs);
        Cell::new(
            template.lhs,
            template.rhs,
            <Lying<L> as CellAlphabet>::derived_orientation(),
            template.provenance,
        )
    }

    #[test]
    fn the_incomparable_positions_alphabet_calls_a_nesting_pair_independent()
    {
        // The lie, and its exact shape: the honest alphabet reports the
        // enclosing pair, and this one reports the relation that licenses a
        // shift. `Same` is included, which matters — a primitive is dependent
        // on its own repeat only because `Same` is not `Incomparable`.
        let root = ToyPos(alloc::vec![].into_boxed_slice());
        let child = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            PositionOrder::Encloses,
            ToyAlphabet::position_order(&root, &child),
            "the honest alphabet reports the nesting"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&root, &child),
            "and the lying one reports the pair as commutable"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&root, &root),
            "including a position against itself"
        );
    }

    #[test]
    fn the_withheld_convexity_alphabet_declines_to_discharge_the_conjunct()
    {
        let honest: CellStore<ToyAlphabet> = CellStore::new();
        let withheld: CellStore<Lying<WithheldConvexity>> = CellStore::new();
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            ToyAlphabet::convexity_discharge(&honest),
            "the toy alphabet discharges the conjunct for every store"
        );
        assert_eq!(
            ConvexityDischarge::ReCheckRequired,
            <Lying<WithheldConvexity> as CellAlphabet>::convexity_discharge(&withheld),
            "and this one withholds the warrant instead"
        );
    }

    #[test]
    fn the_non_local_splice_alphabet_disturbs_a_sibling_it_was_not_asked_about()
    {
        // The lie is in `splice_cmd_at`, so it shows up through `rewrite_at`:
        // firing at `[0]` also restores `[1]`, which the honest alphabet leaves
        // exactly as it found it.
        let peel = lying_cell::<NonLocalSplice>(Toy::succ(Toy::Zero), Toy::Zero);
        let honest_peel = toy_cell(Toy::succ(Toy::Zero), Toy::Zero);
        let term = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let at_left = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            Some(Toy::add(Toy::Zero, Toy::succ(Toy::Zero))),
            rewrite_at(&honest_peel, &term, &at_left),
            "the honest splice touches only the position it was given"
        );
        assert_eq!(
            Some(Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero))),
            rewrite_at(&peel, &term, &at_left),
            "and the non-local one resets the sibling as well"
        );
    }

    #[test]
    fn the_colliding_addresses_alphabet_gives_two_distinct_cells_one_address()
    {
        // The lie's whole effect, isolated: the two cells differ (so a store
        // holds both) and their content addresses agree (so the factorization
        // is asked to hold two different primitives under one key).
        let given = lying_cell::<CollidingAddresses>(Toy::succ(Toy::Zero), Toy::Zero);
        let derived = reoriented_lying_cell::<CollidingAddresses>(Toy::succ(Toy::Zero), Toy::Zero);
        let root = ToyPos(alloc::vec![].into_boxed_slice());
        assert_ne!(
            given, derived,
            "the two cells are structurally distinct, so one store holds both"
        );
        assert_eq!(
            gandr_theory_computads::prim_address(&given, &root),
            gandr_theory_computads::prim_address(&derived, &root),
            "and the orientation tag they differ in is invisible to the digest"
        );
        // The honest inhabitant keeps them apart, which is what makes the
        // collision attributable to this alphabet rather than to the fixture.
        let honest_given = lying_cell::<IncomparablePositions>(Toy::succ(Toy::Zero), Toy::Zero);
        let honest_derived =
            reoriented_lying_cell::<IncomparablePositions>(Toy::succ(Toy::Zero), Toy::Zero);
        assert_ne!(
            gandr_theory_computads::prim_address(&honest_given, &root),
            gandr_theory_computads::prim_address(&honest_derived, &root),
            "an honest orientation hash separates them"
        );
    }

    #[test]
    fn a_lying_alphabet_delegates_everything_it_does_not_lie_about()
    {
        // Non-vacuity for every fixture built on `Lying<L>`: the lie is the
        // ONLY difference, so a refusal obtained over the lying alphabet is
        // attributable to `L`. Rewriting is the composite that reads six of the
        // delegated methods at once (subterm, firing permission, matching,
        // substitution, splicing) and the content address reads the cell's
        // whole hash.
        let cell = toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let lying = lying_cell::<IncomparablePositions>(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let term = Toy::add(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let at_left = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            rewrite_at(&cell, &term, &at_left),
            rewrite_at(&lying, &term, &at_left),
            "the two alphabets rewrite identically"
        );
        assert_eq!(
            gandr_theory_computads::prim_address(&cell, &at_left),
            gandr_theory_computads::prim_address(&lying, &at_left),
            "and one content address serves both, so the tie-break is shared"
        );
        assert_eq!(
            ToyAlphabet::skolemize(&Toy::var(crate::toy_alphabet::ToyNameRef("x"))),
            <Lying<IncomparablePositions> as CellAlphabet>::skolemize(&Toy::var(
                crate::toy_alphabet::ToyNameRef("x")
            )),
            "and skolemization is the toy alphabet's own, not a re-derivation"
        );
    }
}
