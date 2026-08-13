//! The **RNNA** model inventory — the family's shared ancestor (design doc
//! §2.2).
//!
//! Regular nondeterministic nominal automata (Schröder–Kozen–Milius–Wißmann,
//! *Nominal Automata with Name Binding*, `FoSSaCS` 2017, arXiv:1603.01455;
//! RNTA \[34] / NDA \[21]).
//!
//! RNNA are the allocation-only word model: their letters are free uses `a`
//! and binding allocations `|a` — the [`Letter::Free`] / [`Letter::Open`]
//! fragment of the NDA alphabet, with NDA Proposition 5.11 (`a⟧ → a`,
//! `⟦a → |a`) translating between the two presentations and establishing
//! that, under local freshness, NDA add *discipline and determinism, not
//! expressivity* (design doc §1 finding 3). The register presentation is
//! the same strong-nominal-set handle as the NDA's (design doc §6.4); only
//! the transition vocabulary is smaller.
//!
//! This module is the type-level inventory — configurations, transitions,
//! acceptance — as data. The RNNA's decision procedures are catalogue
//! entries ([`crate::catalogue`]); membership is decided by the same
//! forward-reachability procedure as the NDA's, reached through the
//! Prop 5.11 embedding.
//!
//! [`Letter::Free`]: crate::letter::Letter::Free
//! [`Letter::Open`]: crate::letter::Letter::Open

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::Sort;
use crate::handle::Arity;
use crate::handle::AutomatonError;
use crate::handle::Configuration;
use crate::handle::Control;
use crate::handle::Controls;
use crate::handle::Degree;
use crate::handle::Register;
use crate::handle::Transfer;

/// The letter kind an RNNA rule fires on — the allocation-only fragment of
/// the NDA vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RnnaRuleKind
{
    /// `a` — free use: the letter's atom is read from the named register.
    Free
    {
        /// The register the letter's name is read from.
        register: Register,
    },
    /// `|a` — binding allocation: the letter's atom is fresh for the
    /// current store.
    Allocate,
}

/// A symbolic RNNA transition: one orbit of the transition relation in the
/// register presentation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RnnaRule
{
    /// The control point the rule fires from.
    source: Control,
    /// The letter kind the rule fires on.
    kind: RnnaRuleKind,
    /// The control point the rule moves to.
    target: Control,
    /// How the target store is populated; its length is the target's arity.
    transfer: Vec<Transfer>,
}

impl RnnaRule
{
    /// A free-use rule reading `register`.
    #[inline]
    #[must_use]
    pub fn free(
        source: Control,
        register: Register,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RnnaRuleKind::Free { register },
            target,
            transfer,
        };
    }

    /// A binding-allocation rule `q →|a q′`.
    #[inline]
    #[must_use]
    pub fn allocate(
        source: Control,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RnnaRuleKind::Allocate,
            target,
            transfer,
        };
    }

    /// The control point the rule fires from.
    #[inline]
    #[must_use]
    pub fn source(&self) -> Control
    {
        return self.source;
    }

    /// The letter kind the rule fires on.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> RnnaRuleKind
    {
        return self.kind;
    }

    /// The control point the rule moves to.
    #[inline]
    #[must_use]
    pub fn target(&self) -> Control
    {
        return self.target;
    }

    /// The register transfer populating the target store.
    #[inline]
    #[must_use]
    pub fn transfer(&self) -> &[Transfer]
    {
        return &self.transfer;
    }
}

/// A **regular nondeterministic nominal automaton** in the finitary
/// register presentation of design doc §6.4.
///
/// As with the NDA, the final set is a set of control points: an
/// equivariant set of states is a union of orbits, and an orbit in the
/// strong-nominal representation is one control point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rnna<S>
{
    /// The register arity of each control point; the index is the control.
    arities: Vec<Arity>,
    /// The initial configuration.
    initial: Configuration<S>,
    /// The final control points.
    finals: BTreeSet<Control>,
    /// The symbolic transition relation.
    rules: Vec<RnnaRule>,
}

impl<S> Rnna<S>
where
    S: Sort,
{
    /// Build an RNNA, checking the finite handle's well-formedness.
    ///
    /// # Contract
    /// - ensures: on success, every rule's source and target are control points
    ///   of the automaton, every transfer length matches its target's arity,
    ///   every register read is in range, and [`Transfer::Allocated`] occurs
    ///   only in allocation rules, at most once.
    /// - provides: a well-formed RNNA.
    /// - fails: the first violated invariant, as an [`AutomatonError`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the [`AutomatonError`] variant naming the first violated
    /// invariant.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — each check is separated from acceptance by one
    ///   mutated handle field, exercised by one rejection test plus one
    ///   accepting construction.
    /// - witness: `rnna::tests::construction_accepts_a_well_formed_rnna`
    /// - witness: `rnna::tests::construction_rejects_misplaced_allocated_name`
    #[inline]
    pub fn new(
        arities: Vec<Arity>,
        initial: Configuration<S>,
        finals: BTreeSet<Control>,
        rules: Vec<RnnaRule>,
    ) -> Result<Self, AutomatonError>
    {
        let controls = Controls::from(arities.len());
        let arity_of = |control: Control| arities.get(usize::from(control)).copied();

        let Some(initial_arity) = arity_of(initial.control())
        else {
            return Err(AutomatonError::InvalidControl {
                control: initial.control(),
                controls: u32::from(controls),
            });
        };
        if initial.store().arity() != initial_arity {
            return Err(AutomatonError::ArityMismatch {
                control: initial.control(),
                expected: initial_arity,
                actual: initial.store().arity(),
            });
        }
        for final_control in &finals {
            if arity_of(*final_control).is_none() {
                return Err(AutomatonError::InvalidControl {
                    control: *final_control,
                    controls: u32::from(controls),
                });
            }
        }
        for rule in &rules {
            let Some(source_arity) = arity_of(rule.source())
            else {
                return Err(AutomatonError::InvalidControl {
                    control: rule.source(),
                    controls: u32::from(controls),
                });
            };
            let Some(target_arity) = arity_of(rule.target())
            else {
                return Err(AutomatonError::InvalidControl {
                    control: rule.target(),
                    controls: u32::from(controls),
                });
            };
            let actual = Arity::from(rule.transfer().len());
            if actual != target_arity {
                return Err(AutomatonError::ArityMismatch {
                    control: rule.target(),
                    expected: target_arity,
                    actual,
                });
            }
            if let RnnaRuleKind::Free { register } = rule.kind()
                && u32::from(register) >= u32::from(source_arity)
            {
                return Err(AutomatonError::UnknownRegister {
                    control: rule.source(),
                    register,
                });
            }
            let allocations = rule
                .transfer()
                .iter()
                .filter(|entry| matches!(entry, Transfer::Allocated))
                .count();
            let misplaced = match rule.kind() {
                | RnnaRuleKind::Allocate => allocations > 1,
                | RnnaRuleKind::Free { .. } => allocations > 0,
            };
            if misplaced {
                return Err(AutomatonError::MisplacedAllocatedName {
                    control: rule.source(),
                });
            }
        }
        return Ok(Self {
            arities,
            initial,
            finals,
            rules,
        });
    }

    /// The register arity of each control point, in control order.
    #[inline]
    #[must_use]
    pub fn arities(&self) -> &[Arity]
    {
        return &self.arities;
    }

    /// The initial configuration.
    #[inline]
    #[must_use]
    pub fn initial(&self) -> &Configuration<S>
    {
        return &self.initial;
    }

    /// The final control points.
    #[inline]
    #[must_use]
    pub fn finals(&self) -> &BTreeSet<Control>
    {
        return &self.finals;
    }

    /// The symbolic transition relation.
    #[inline]
    #[must_use]
    pub fn rules(&self) -> &[RnnaRule]
    {
        return &self.rules;
    }

    /// The automaton's degree: the maximum register count over its control
    /// points — the complexity parameter of every catalogue procedure
    /// (design doc §5).
    #[inline]
    #[must_use]
    pub fn degree(&self) -> Degree
    {
        return self
            .arities
            .iter()
            .max()
            .map_or(Degree::ZERO, |&arity| Degree::from(arity));
    }
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use alloc::vec;

    use super::Arity;
    use super::AutomatonError;
    use super::Configuration;
    use super::Control;
    use super::Register;
    use super::Rnna;
    use super::RnnaRule;
    use super::Sort;
    use super::Transfer;
    use crate::Gensym;
    use crate::Unifiability;
    use crate::handle::Store;

    /// A single-role sort for the RNNA tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        /// An atom-role sort for channel names.
        Channel,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            Unifiability::ATOM_ROLE
        }
    }

    /// A well-formed allocation-only automaton — one control point holding
    /// one channel — is accepted by the constructor and reports its degree.
    #[test]
    fn construction_accepts_a_well_formed_rnna()
    {
        let mut gensym = Gensym::new(Role::Channel);
        let channel = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let q0 = Control::ZERO;
        let initial: Configuration<Role> =
            Configuration::new(q0, Store::empty(Arity::from(0_usize)));
        let q1 = Control::from(1_u32);
        let rules = vec![
            RnnaRule::allocate(q0, q1, vec![Transfer::Allocated]),
            RnnaRule::free(q1, Register::ZERO, q1, vec![Transfer::Keep(Register::ZERO)]),
        ];
        let automaton = Rnna::new(
            vec![Arity::from(0_usize), Arity::from(1_usize)],
            initial,
            BTreeSet::from([q1]),
            rules,
        )
        .expect("the allocation-only automaton is well-formed");
        assert_eq!(1, u32::from(automaton.degree()));
        assert_eq!(2, automaton.rules().len());
        assert_eq!(Role::Channel, channel.sort());
    }

    /// `Transfer::Allocated` in a free-use rule is rejected.
    #[test]
    fn construction_rejects_misplaced_allocated_name()
    {
        let q0 = Control::ZERO;
        let initial: Configuration<Role> =
            Configuration::new(q0, Store::empty(Arity::from(1_usize)));
        let rules = vec![RnnaRule::free(q0, Register::ZERO, q0, vec![
            Transfer::Allocated,
        ])];
        let result = Rnna::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            result.expect_err("an allocated name in a free rule is invalid"),
            AutomatonError::MisplacedAllocatedName { control: q0 }
        );
    }
}
