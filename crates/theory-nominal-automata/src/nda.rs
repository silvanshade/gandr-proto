//! The **NDA** model inventory — nondeterministic deallocation automata
//! (NDA Def 5.1) — over the crate's sort-tagged atoms, together with its
//! membership decision procedure.
//!
//! An NDA `A = (Q, Δ, i, F)` has an orbit-finite nominal state set `Q`, an
//! equivariant transition relation `Δ ⊆ Q × Â × Q` over the extended
//! alphabet [`Letter`], an initial state `i`, and an equivariant final set
//! `F`, subject to left α-invariance, **name erasure** (a deallocated name
//! is actually forgotten), and finite branching. The representation here is
//! the papers' finitary one (design doc §6.4): states are
//! [`Configuration`]s — a control point plus a partial injective register
//! store — and each [`Rule`] denotes one *orbit* of `Δ`, defunctionalized
//! into a source control, a letter kind, a target control, and a register
//! [`Transfer`] saying how the target store is populated. The support
//! lemma's arithmetic (NDA Lemma 5.7) is enforced by construction and
//! validation: free and close moves read a register, allocation lands in at
//! most one target register, and a close rule may not keep the register it
//! deallocates (name erasure, NDA Def 5.1).
//!
//! The decision procedure shipped in this slice is **literal-language
//! membership** — `w ∈ L(A)` — decided by forward reachability over
//! configurations, the classical NFA simulation the design doc's §5
//! reduction template delegates to. Combined with the name-dropping
//! modification ([`crate::dropping`]) it decides **α-closure membership**
//! `w ∈ L(A_⊥)` (NDA Thm 6.9), the form the inclusion and determinization
//! constructions consume. On a deterministic deallocation automaton (DDA,
//! NDA Def 8.2) the same run is deterministic and online — the forward
//! runtime monitor of design doc §7.3.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::Atom;
use crate::Sort;
use crate::handle::Arity;
use crate::handle::AutomatonError;
use crate::handle::Configuration;
use crate::handle::Control;
use crate::handle::Controls;
use crate::handle::Degree;
use crate::handle::Membership;
use crate::handle::Register;
use crate::handle::Store;
use crate::handle::Transfer;
use crate::letter::Letter;

/// The letter kind a rule fires on, and the register the letter's name is
/// read from where the kind reads one.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuleKind
{
    /// `a` — free use: fires on [`Letter::Free`] when the named register
    /// holds the letter's atom (NDA Lemma 5.7(1)).
    Free
    {
        /// The register the letter's name is read from.
        register: Register,
    },
    /// `⟦a` — allocation: fires on [`Letter::Open`] when the letter's atom is
    /// fresh for the current store (NDA Lemma 5.7(2)).
    Open,
    /// `a⟧` — deallocation: fires on [`Letter::Close`] when the named
    /// register holds the letter's atom; the name is erased from memory
    /// (NDA Lemma 5.7(3) and the name-erasure condition of Def 5.1).
    Close
    {
        /// The register whose name is deallocated.
        register: Register,
    },
    /// `⟦a⟧` — allocate and immediately deallocate: fires on
    /// [`Letter::OpenClose`] when the letter's atom is fresh for the current
    /// store; the name is never stored (NDA Lemma 5.7(4)).
    OpenClose,
    /// ε name-dropping: forgets the named register's name without consuming
    /// a letter — the lossiness transition the name-dropping modification
    /// adds (NDA Constr 6.3; RNTA Remark 5.6).
    Drop
    {
        /// The register whose name is forgotten.
        register: Register,
    },
}

/// A symbolic transition: one orbit of `Δ ⊆ Q × Â × Q` (NDA Def 5.1) in the
/// register presentation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Rule
{
    /// The control point the rule fires from.
    source: Control,
    /// The letter kind the rule fires on.
    kind: RuleKind,
    /// The control point the rule moves to.
    target: Control,
    /// How the target store is populated; its length is the target's arity.
    transfer: Vec<Transfer>,
}

impl Rule
{
    /// A free-use rule `q →a q′` reading `register`.
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
            kind: RuleKind::Free { register },
            target,
            transfer,
        };
    }

    /// An allocation rule `q →⟦a q′`.
    #[inline]
    #[must_use]
    pub fn open(
        source: Control,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RuleKind::Open,
            target,
            transfer,
        };
    }

    /// A deallocation rule `q →a⟧ q′` erasing `register`.
    #[inline]
    #[must_use]
    pub fn close(
        source: Control,
        register: Register,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RuleKind::Close { register },
            target,
            transfer,
        };
    }

    /// An allocate-and-immediately-deallocate rule `q →⟦a⟧ q′`.
    #[inline]
    #[must_use]
    pub fn open_close(
        source: Control,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RuleKind::OpenClose,
            target,
            transfer,
        };
    }

    /// An ε name-dropping rule forgetting `register` (NDA Constr 6.3).
    #[inline]
    #[must_use]
    pub fn forget(
        source: Control,
        register: Register,
        target: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self {
            source,
            kind: RuleKind::Drop { register },
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
    pub fn kind(&self) -> RuleKind
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

/// A **nondeterministic deallocation automaton** (NDA Def 5.1) in the
/// finitary register presentation of design doc §6.4.
///
/// The final set is a set of control points: an equivariant set of states
/// is a union of orbits, and in the strong-nominal representation an orbit
/// is exactly one control point (design doc §6.4). Acceptance is therefore
/// "the run ends at a final control point".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nda<S>
{
    /// The register arity of each control point; the index is the control.
    arities: Vec<Arity>,
    /// The initial configuration `i`.
    initial: Configuration<S>,
    /// The final control points `F` (a union of orbits).
    finals: BTreeSet<Control>,
    /// The symbolic transition relation `Δ`.
    rules: Vec<Rule>,
}

impl<S> Nda<S>
where
    S: Sort,
{
    /// Build an NDA, checking the finite handle's well-formedness.
    ///
    /// # Contract
    /// - requires: every store handed in is injective (enforced by
    ///   [`Store::try_new`]).
    /// - ensures: on success, every rule's source and target are control points
    ///   of the automaton, every transfer length matches its target's arity,
    ///   every register read is in range, [`Transfer::Allocated`] occurs only
    ///   in [`RuleKind::Open`] rules and at most once, and no
    ///   [`RuleKind::Close`] rule keeps the register it deallocates (name
    ///   erasure, NDA Def 5.1).
    /// - provides: a well-formed NDA ready for [`Nda::accepts`] and
    ///   [`crate::dropping::name_dropping`].
    /// - fails: the first violated invariant, as an [`AutomatonError`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the [`AutomatonError`] variant naming the first violated
    /// invariant.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — each check is separated from acceptance by one
    ///   mutated handle field, exercised by one rejection test per variant plus
    ///   one accepting construction.
    /// - witness: `nda::tests::construction_rejects_invalid_control`
    /// - witness: `nda::tests::construction_rejects_arity_mismatch`
    /// - witness: `nda::tests::construction_rejects_unknown_register`
    /// - witness: `nda::tests::construction_rejects_misplaced_allocated_name`
    /// - witness: `nda::tests::construction_rejects_kept_deallocated_name`
    /// - witness: `nda::tests::session_monitor_accepts_drained_log`
    #[inline]
    pub fn new(
        arities: Vec<Arity>,
        initial: Configuration<S>,
        finals: BTreeSet<Control>,
        rules: Vec<Rule>,
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
            match rule.kind() {
                | RuleKind::Free { register } | RuleKind::Drop { register } => {
                    if register >= register_of(source_arity) {
                        return Err(AutomatonError::UnknownRegister {
                            control: rule.source(),
                            register,
                        });
                    }
                },
                | RuleKind::Close { register } => {
                    if register >= register_of(source_arity) {
                        return Err(AutomatonError::UnknownRegister {
                            control: rule.source(),
                            register,
                        });
                    }
                    if rule.transfer().contains(&Transfer::Keep(register)) {
                        return Err(AutomatonError::KeptDeallocatedName {
                            control: rule.source(),
                            register,
                        });
                    }
                },
                | RuleKind::Open | RuleKind::OpenClose => {},
            }
            let allocations = rule
                .transfer()
                .iter()
                .filter(|entry| matches!(entry, Transfer::Allocated))
                .count();
            let misplaced = match rule.kind() {
                | RuleKind::Open => allocations > 1,
                | RuleKind::Free { .. }
                | RuleKind::Close { .. }
                | RuleKind::OpenClose
                | RuleKind::Drop { .. } => allocations > 0,
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

    /// Build an NDA whose well-formedness is established by construction.
    ///
    /// Used by constructions that derive an automaton from an already-valid
    /// one (the name-dropping modification) — the derived handle reuses the
    /// donor's control table, and every added rule is built to the same
    /// invariants [`Nda::new`] checks.
    ///
    /// # Contract
    /// - requires: the arguments satisfy every invariant [`Nda::new`] checks.
    /// - provides: the automaton, without re-running validation.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn from_validated(
        arities: Vec<Arity>,
        initial: Configuration<S>,
        finals: BTreeSet<Control>,
        rules: Vec<Rule>,
    ) -> Self
    {
        return Self {
            arities,
            initial,
            finals,
            rules,
        };
    }

    /// The number of control points of the finite handle.
    #[inline]
    #[must_use]
    pub fn controls(&self) -> Controls
    {
        return Controls::from(self.arities.len());
    }

    /// The register arity of `control`, if it is a control point of the
    /// automaton.
    #[inline]
    #[must_use]
    pub fn arity(
        &self,
        control: Control,
    ) -> Option<Arity>
    {
        return self.arities.get(usize::from(control)).copied();
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
    pub fn rules(&self) -> &[Rule]
    {
        return &self.rules;
    }

    /// The automaton's degree: the maximum register count over its control
    /// points — the representation-level upper bound on concurrently-live
    /// names, and the complexity parameter of every catalogue procedure
    /// (NDA Def 5.1; RNTA §4; design doc §5).
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

impl<S> Nda<S>
where
    S: Sort + Ord,
{
    /// Decide **literal-language membership** `w ∈ L(A)` by forward
    /// reachability over configurations.
    ///
    /// The run is the classical NFA simulation the design doc's §5 reduction
    /// template delegates to: the frontier starts at the ε-closure (under
    /// [`RuleKind::Drop`] transitions) of the initial configuration, each
    /// letter steps every frontier configuration through every matching
    /// rule, and the word is accepted exactly when some configuration
    /// reaches a final control point after the last letter. Applied to the
    /// name-dropping modification `A_⊥` ([`crate::dropping::name_dropping`])
    /// the same procedure decides membership in the α-closure of `L(A)`
    /// (NDA Thm 6.9); on a DDA (NDA Def 8.2) the frontier is a singleton
    /// and the run is the deterministic online monitor of design doc §7.3.
    ///
    /// Termination is structural: the word is finite, and each step's
    /// configuration set is finite — a configuration is a control point
    /// plus an injective store over the finitely many atoms of the word and
    /// the initial store, and the ε-closure's visited set is drawn from the
    /// same finite space.
    ///
    /// # Contract
    /// - provides: whether some run of the automaton spells `word` and ends at
    ///   a final control point.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — acceptance and rejection are separated per
    ///   letter kind by words over the session-monitor automaton that differ in
    ///   one letter, plus the ε-closure boundary witnessed through the
    ///   name-dropping modification.
    /// - witness: `nda::tests::session_monitor_accepts_drained_log`
    /// - witness: `nda::tests::session_monitor_rejects_leaked_login`
    /// - witness: `nda::tests::session_monitor_rejects_logout_without_login`
    /// - witness: `nda::tests::session_monitor_rejects_unknown_actor`
    /// - witness: `nda::tests::session_monitor_bounds_concurrent_logins`
    /// - witness: `dropping::tests::name_dropping_closes_language_under_alpha`
    #[inline]
    #[must_use]
    pub fn accepts(
        &self,
        word: &[Letter<S>],
    ) -> Membership
    {
        let mut frontier = self.close_under_drops(alloc::vec![self.initial.clone()]);
        for letter in word {
            let mut stepped: Vec<Configuration<S>> = Vec::new();
            for configuration in &frontier {
                let mut successors = self.step(configuration, *letter);
                stepped.append(&mut successors);
            }
            let closed = self.close_under_drops(stepped);
            if closed.is_empty() {
                return Membership::REJECTED;
            }
            frontier = closed;
        }
        let accepted = frontier
            .iter()
            .any(|configuration| self.finals.contains(&configuration.control()));
        return if accepted {
            Membership::ACCEPTED
        }
        else {
            Membership::REJECTED
        };
    }

    /// The ε-closure of a configuration set under [`RuleKind::Drop`]
    /// transitions: every configuration reachable without consuming a
    /// letter.
    ///
    /// # Contract
    /// - ensures: the result contains `seed` and is closed under one-step drop
    ///   successors; duplicate configurations are coalesced.
    /// - panics: none.
    fn close_under_drops(
        &self,
        seed: Vec<Configuration<S>>,
    ) -> Vec<Configuration<S>>
    {
        let mut visited: BTreeSet<Configuration<S>> = BTreeSet::new();
        let mut work: Vec<Configuration<S>> = Vec::new();
        for configuration in seed {
            if visited.insert(configuration.clone()) {
                work.push(configuration);
            }
        }
        while let Some(configuration) = work.pop() {
            for rule in &self.rules {
                if !matches!(rule.kind(), RuleKind::Drop { .. }) {
                    continue;
                }
                if rule.source() != configuration.control() {
                    continue;
                }
                let store = apply_transfer(configuration.store(), rule.transfer(), None);
                let successor = Configuration::new(rule.target(), store);
                if visited.insert(successor.clone()) {
                    work.push(successor);
                }
            }
        }
        return visited.into_iter().collect();
    }

    /// The one-letter successors of a configuration: every configuration a
    /// matching rule can move to on `letter`.
    ///
    /// # Contract
    /// - ensures: a rule contributes exactly when its kind matches the letter
    ///   and its freshness or register-read side condition holds.
    /// - panics: none.
    fn step(
        &self,
        configuration: &Configuration<S>,
        letter: Letter<S>,
    ) -> Vec<Configuration<S>>
    {
        let mut successors: Vec<Configuration<S>> = Vec::new();
        for rule in &self.rules {
            if rule.source() != configuration.control() {
                continue;
            }
            let allocated = match (rule.kind(), letter) {
                | (RuleKind::Free { register }, Letter::Free(atom))
                | (RuleKind::Close { register }, Letter::Close(atom)) => {
                    if configuration.store().name(register) != Some(atom) {
                        continue;
                    }
                    None
                },
                | (RuleKind::Open, Letter::Open(atom))
                | (RuleKind::OpenClose, Letter::OpenClose(atom)) => {
                    if !bool::from(configuration.store().freshness(atom)) {
                        continue;
                    }
                    matches!(rule.kind(), RuleKind::Open).then_some(atom)
                },
                | (..) => continue,
            };
            let store = apply_transfer(configuration.store(), rule.transfer(), allocated);
            successors.push(Configuration::new(rule.target(), store));
        }
        return successors;
    }
}

/// The first register beyond an arity: the boundary value register reads
/// are validated against.
///
/// # Contract
/// - provides: the smallest out-of-range register for a control point of the
///   given arity.
/// - panics: none.
fn register_of(arity: Arity) -> Register
{
    return Register::from(u32::from(arity));
}

/// Populate a target store from a source store and a rule's transfer.
///
/// # Contract
/// - requires: `allocated` is `Some` only when the firing rule is an
///   [`RuleKind::Open`] rule, and is then fresh for `store` — the freshness
///   gate in [`Nda::step`] — so injectivity is preserved: `Keep` entries copy
///   from an injective source, and the single `Allocated` entry carries a name
///   the source does not hold.
/// - provides: the target store, built through [`Store::from_validated`].
/// - panics: none.
fn apply_transfer<S>(
    store: &Store<S>,
    transfer: &[Transfer],
    allocated: Option<Atom<S>>,
) -> Store<S>
where
    S: Sort,
{
    let slots = transfer
        .iter()
        .map(|&entry| match entry {
            | Transfer::Keep(register) => store.name(register),
            | Transfer::Allocated => allocated,
            | Transfer::Empty => None,
        })
        .collect();
    return Store::from_validated(slots);
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use alloc::vec;

    use super::Arity;
    use super::Atom;
    use super::AutomatonError;
    use super::Configuration;
    use super::Control;
    use super::Letter;
    use super::Membership;
    use super::Nda;
    use super::Register;
    use super::Rule;
    use super::Sort;
    use super::Store;
    use super::Transfer;
    use crate::Gensym;
    use crate::Unifiability;

    /// A single-role sort for the automaton tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        /// An atom-role sort for endpoint names.
        Endpoint,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            Unifiability::ATOM_ROLE
        }
    }

    /// A minted pair of endpoint atoms for the session tests: the admin `c`
    /// and one user name.
    struct Names
    {
        /// The admin name, permanently held in register zero.
        admin: Atom<Role>,
        /// A user name, allocated and deallocated by the test words.
        user: Atom<Role>,
    }

    /// Mint the test names.
    fn names() -> Names
    {
        let mut gensym = Gensym::new(Role::Endpoint);
        let admin = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let user = gensym
            .fresh()
            .expect("a new allocator can mint the second atom");
        return Names { admin, user };
    }

    /// The session-lifecycle monitor of NDA Example 5.6, specialized to one
    /// concurrent non-admin participant: control `q0` (arity 1) holds only
    /// the admin, control `q1` (arity 2) additionally holds one logged-in
    /// user; the final set is `{q0}` — "all users except the admin are
    /// logged out at the end".
    fn session_monitor(names: &Names) -> Nda<Role>
    {
        let q0 = Control::ZERO;
        let q1 = Control::from(1_u32);
        let register_admin = Register::ZERO;
        let register_user = Register::from(1_u32);
        let initial = Configuration::new(
            q0,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let finals = BTreeSet::from([q0]);
        let rules = vec![
            // A fresh user logs in: `⟦a`.
            Rule::open(q0, q1, vec![
                Transfer::Keep(register_admin),
                Transfer::Allocated,
            ]),
            // The logged-in user logs out: `a⟧`.
            Rule::close(q1, register_user, q0, vec![Transfer::Keep(register_admin)]),
            // The admin acts while nobody is logged in.
            Rule::free(q0, register_admin, q0, vec![Transfer::Keep(register_admin)]),
            // The admin acts while a user is logged in.
            Rule::free(q1, register_admin, q1, vec![
                Transfer::Keep(register_admin),
                Transfer::Keep(register_user),
            ]),
        ];
        return Nda::new(
            vec![Arity::from(1_usize), Arity::from(2_usize)],
            initial,
            finals,
            rules,
        )
        .expect("the session monitor is well-formed");
    }

    /// A log in which the user logs in and back out — with admin actions
    /// interleaved — is accepted: every non-admin resource is drained at the
    /// end.
    #[test]
    fn session_monitor_accepts_drained_log()
    {
        let names = names();
        let monitor = session_monitor(&names);
        let word = vec![
            Letter::Open(names.user),
            Letter::Free(names.admin),
            Letter::Close(names.user),
            Letter::Free(names.admin),
        ];
        assert_eq!(
            Membership::ACCEPTED,
            monitor.accepts(&word),
            "a drained session log is accepted"
        );
    }

    /// A log that ends with the user still logged in is rejected: the run
    /// ends at `q1`, outside the final set — the automata-level leak
    /// detection of design doc §7.2.
    #[test]
    fn session_monitor_rejects_leaked_login()
    {
        let names = names();
        let monitor = session_monitor(&names);
        let word = vec![Letter::Open(names.user), Letter::Free(names.admin)];
        assert_eq!(
            Membership::REJECTED,
            monitor.accepts(&word),
            "a log with a live login at the end is rejected"
        );
    }

    /// A logout with no matching login is rejected: no rule fires on
    /// `Close` from `q0`.
    #[test]
    fn session_monitor_rejects_logout_without_login()
    {
        let names = names();
        let monitor = session_monitor(&names);
        let word = vec![Letter::Close(names.user)];
        assert_eq!(
            Membership::REJECTED,
            monitor.accepts(&word),
            "a close without a matching open is rejected"
        );
    }

    /// A free action by a name held in no register is rejected: free moves
    /// require the name in memory (NDA Lemma 5.7(1)).
    #[test]
    fn session_monitor_rejects_unknown_actor()
    {
        let names = names();
        let monitor = session_monitor(&names);
        let word = vec![Letter::Free(names.user)];
        assert_eq!(
            Membership::REJECTED,
            monitor.accepts(&word),
            "a free use of an unallocated name is rejected"
        );
    }

    /// A second concurrent login is rejected: the monitor has no allocation
    /// rule from `q1`, so two concurrently-live users exceed its degree —
    /// the degree-as-budget story of design doc §5.
    #[test]
    fn session_monitor_bounds_concurrent_logins()
    {
        let mut gensym = Gensym::new(Role::Endpoint);
        let admin = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let user = gensym
            .fresh()
            .expect("a new allocator can mint the second atom");
        let other = gensym
            .fresh()
            .expect("a new allocator can mint the third atom");
        let names = Names { admin, user };
        let monitor = session_monitor(&names);
        let word = vec![Letter::Open(user), Letter::Open(other)];
        assert_eq!(
            Membership::REJECTED,
            monitor.accepts(&word),
            "a second concurrent login exceeds the monitor's degree"
        );
    }

    /// The monitor's degree is the maximum arity, two registers at `q1`.
    #[test]
    fn session_monitor_degree_is_maximum_arity()
    {
        let names = names();
        let monitor = session_monitor(&names);
        assert_eq!(2, u32::from(monitor.degree()));
    }

    /// A rule naming a control point outside the control table is rejected.
    #[test]
    fn construction_rejects_invalid_control()
    {
        let names = names();
        let initial = Configuration::new(
            Control::ZERO,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![Rule::open(Control::from(7_u32), Control::ZERO, vec![])];
        let result = Nda::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            result.expect_err("a rule from an unknown control is invalid"),
            AutomatonError::InvalidControl {
                control: Control::from(7_u32),
                controls: 1,
            }
        );
    }

    /// A transfer whose length differs from the target's arity is rejected.
    #[test]
    fn construction_rejects_arity_mismatch()
    {
        let names = names();
        let initial = Configuration::new(
            Control::ZERO,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![Rule::free(
            Control::ZERO,
            Register::ZERO,
            Control::ZERO,
            vec![],
        )];
        let result = Nda::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            result.expect_err("a short transfer is invalid"),
            AutomatonError::ArityMismatch {
                control: Control::ZERO,
                expected: Arity::from(1_usize),
                actual: Arity::from(0_usize),
            }
        );
    }

    /// A rule reading a register its source control point does not have is
    /// rejected.
    #[test]
    fn construction_rejects_unknown_register()
    {
        let names = names();
        let initial = Configuration::new(
            Control::ZERO,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![Rule::close(
            Control::ZERO,
            Register::from(3_u32),
            Control::ZERO,
            vec![Transfer::Empty],
        )];
        let result = Nda::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            result.expect_err("an out-of-range register is invalid"),
            AutomatonError::UnknownRegister {
                control: Control::ZERO,
                register: Register::from(3_u32),
            }
        );
    }

    /// `Transfer::Allocated` in a non-allocation rule is rejected.
    #[test]
    fn construction_rejects_misplaced_allocated_name()
    {
        let names = names();
        let initial = Configuration::new(
            Control::ZERO,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![Rule::open_close(Control::ZERO, Control::ZERO, vec![
            Transfer::Allocated,
        ])];
        let result = Nda::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            AutomatonError::MisplacedAllocatedName {
                control: Control::ZERO,
            },
            result.expect_err("an allocated-name transfer in an open-close rule is invalid")
        );
    }

    /// A deallocation rule that keeps the deallocated register's name is
    /// rejected — the name-erasure condition of NDA Def 5.1.
    #[test]
    fn construction_rejects_kept_deallocated_name()
    {
        let names = names();
        let initial = Configuration::new(
            Control::ZERO,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![Rule::close(
            Control::ZERO,
            Register::ZERO,
            Control::ZERO,
            vec![Transfer::Keep(Register::ZERO)],
        )];
        let result = Nda::new(vec![Arity::from(1_usize)], initial, BTreeSet::new(), rules);
        assert_eq!(
            AutomatonError::KeptDeallocatedName {
                control: Control::ZERO,
                register: Register::ZERO,
            },
            result.expect_err("keeping a deallocated name is invalid")
        );
    }

    /// `⟦a⟧` consumes a letter without storing the name: the word
    /// `[⟦a⟧, c]` is accepted by a monitor with an open-close rule, and
    /// allocating a name held in memory through `⟦a⟧` is freshness-gated.
    #[test]
    fn open_close_allocates_and_immediately_forgets()
    {
        let names = names();
        let q0 = Control::ZERO;
        let initial = Configuration::new(
            q0,
            Store::try_new(vec![Some(names.admin)]).expect("an injective store is valid"),
        );
        let rules = vec![
            Rule::open_close(q0, q0, vec![Transfer::Keep(Register::ZERO)]),
            Rule::free(q0, Register::ZERO, q0, vec![Transfer::Keep(Register::ZERO)]),
        ];
        let monitor = Nda::new(
            vec![Arity::from(1_usize)],
            initial,
            BTreeSet::from([q0]),
            rules,
        )
        .expect("the open-close monitor is well-formed");
        let word = vec![Letter::OpenClose(names.user), Letter::Free(names.admin)];
        assert_eq!(
            Membership::ACCEPTED,
            monitor.accepts(&word),
            "open-close consumes a letter and keeps no name"
        );
        let word = vec![Letter::OpenClose(names.admin)];
        assert_eq!(
            Membership::REJECTED,
            monitor.accepts(&word),
            "open-close of a name already in memory is freshness-gated"
        );
    }
}
