//! The **name-dropping modification** `A_⊥`: step ① of the shared reduction
//! template (design doc §5), closing a nominal automaton's literal language
//! under α-equivalence.
//!
//! The literal language of a name-allocating automaton is not in general
//! closed under α-equivalence — a run may be unable to allocate a name it
//! still *remembers* but no longer needs, because the allocation freshness
//! gate sees the whole store (NDA §6; RNTA §5). The modification adds ε
//! transitions that **drop** names from the store: "at every point, register
//! contents may nondeterministically be erased (freeing the register)" —
//! the lossiness condition of RNTA Remark 5.6, and the engine of NDA
//! Construction 6.3 / Theorem 6.9 and RNTA Definition 5.3 / Theorem 5.5:
//!
//! - `L(A_⊥)` is the closure of `L(A)` under α-equivalence, so `A` and `A_⊥`
//!   have the same alphatic language;
//! - the degree is unchanged, and the orbit count grows by at most a factor
//!   `2^degree` (one subset of droppable registers per orbit).
//!
//! The construction here realizes dropping as explicit [`RuleKind::Drop`]
//! ε-rules — one per (control point, register) — so the modified automaton
//! is data in the same representation and the membership procedure
//! ([`Nda::accepts`]) decides α-closure membership on it directly. The
//! same-degree / bounded-blowup bounds of the theorems hold in the
//! representation: the construction adds no registers and exactly
//! Σ arity rules.
//!
//! [`RuleKind::Drop`]: crate::nda::RuleKind::Drop

use alloc::vec::Vec;

use crate::Sort;
use crate::handle::Control;
use crate::handle::Register;
use crate::handle::Transfer;
use crate::nda::Nda;
use crate::nda::Rule;

/// The name-dropping modification `A_⊥` of an NDA (NDA Constr 6.3; the
/// word-level instance of RNTA Def 5.3).
///
/// For every control point and every one of its registers, the construction
/// adds one ε-rule that forgets that register's name while carrying every
/// other register over — the nondeterministic register erasure of RNTA
/// Remark 5.6. The donor's rules, control table, initial configuration, and
/// final set are preserved unchanged, so every run of `A` is a run of
/// `A_⊥` (`L(A) ⊆ L(A_⊥)`), and the added ε-rules witness the α-closure of
/// the literal language (NDA Thm 6.9).
///
/// # Contract
/// - requires: `automaton` is well-formed ([`Nda::new`]).
/// - ensures: the result contains exactly the donor's rules plus one
///   [`RuleKind::Drop`][drop-kind] rule per (control point, register) pair, on
///   the same control table — the degree is unchanged and the added-rule count
///   is the sum of the arities (the representation-level shadow of the
///   `2^degree` orbit bound, RNTA Thm 5.5).
/// - provides: an NDA deciding α-closure membership through [`Nda::accepts`].
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the construction is separated from the identity by
///   the added drop rules, whose two observable effects (rule-count growth
///   equal to the register total, and acceptance of an α-variant the donor
///   rejects) are each witnessed exactly; language monotonicity is witnessed
///   universally over random words on the session monitor.
/// - witness: `dropping::tests::name_dropping_adds_one_drop_rule_per_register`
/// - witness: `dropping::tests::name_dropping_closes_language_under_alpha`
/// - witness: `dropping::tests::name_dropping_only_enlarges_the_language`
///
/// [drop-kind]: crate::nda::RuleKind::Drop
#[inline]
#[must_use]
pub fn name_dropping<S>(automaton: &Nda<S>) -> Nda<S>
where
    S: Sort,
{
    let mut rules: Vec<Rule> = automaton.rules().to_vec();
    for (index, arity) in automaton.arities().iter().enumerate() {
        let control = Control::from(index);
        for register in 0 .. u32::from(*arity) {
            let register = Register::from(register);
            let transfer = (0 .. u32::from(*arity))
                .map(|slot| {
                    if slot == u32::from(register) {
                        Transfer::Empty
                    }
                    else {
                        Transfer::Keep(Register::from(slot))
                    }
                })
                .collect();
            rules.push(Rule::forget(control, register, control, transfer));
        }
    }
    return Nda::from_validated(
        automaton.arities().to_vec(),
        automaton.initial().clone(),
        automaton.finals().clone(),
        rules,
    );
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use alloc::vec;
    use alloc::vec::Vec;

    use proptest::prelude::*;

    use super::name_dropping;
    use crate::Atom;
    use crate::Gensym;
    use crate::Sort;
    use crate::Unifiability;
    use crate::handle::Arity;
    use crate::handle::Configuration;
    use crate::handle::Control;
    use crate::handle::Membership;
    use crate::handle::Register;
    use crate::handle::Store;
    use crate::handle::Transfer;
    use crate::letter::Letter;
    use crate::nda::Nda;
    use crate::nda::Rule;
    use crate::nda::RuleKind;

    /// A single-role sort for the construction tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        /// An atom-role sort for resource names.
        Resource,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            Unifiability::ATOM_ROLE
        }
    }

    /// The α-closure witness automaton: its initial store already holds a
    /// concrete name `c`, so the literal word `[⟦c, c]` — an α-variant of
    /// the accepted `[⟦x, x]` — is rejected, because `c` cannot be
    /// allocated while remembered (NDA §6; RNTA §5).
    struct Witness
    {
        /// The automaton before name-dropping.
        automaton: Nda<Role>,
        /// The remembered name.
        remembered: Atom<Role>,
        /// A name the initial store does not hold.
        fresh: Atom<Role>,
    }

    /// Build the witness automaton.
    fn witness() -> Witness
    {
        let mut gensym = Gensym::new(Role::Resource);
        let remembered = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let fresh = gensym
            .fresh()
            .expect("a new allocator can mint the second atom");
        let q0 = Control::ZERO;
        let q1 = Control::from(1_u32);
        let q2 = Control::from(2_u32);
        let register_remembered = Register::ZERO;
        let register_allocated = Register::from(1_u32);
        let initial = Configuration::new(
            q0,
            Store::try_new(vec![Some(remembered)]).expect("an injective store is valid"),
        );
        let rules = vec![
            Rule::open(q0, q1, vec![
                Transfer::Keep(register_remembered),
                Transfer::Allocated,
            ]),
            Rule::free(q1, register_allocated, q2, vec![Transfer::Keep(
                register_remembered,
            )]),
        ];
        let automaton = Nda::new(
            vec![
                Arity::from(1_usize),
                Arity::from(2_usize),
                Arity::from(1_usize),
            ],
            initial,
            BTreeSet::from([q2]),
            rules,
        )
        .expect("the witness automaton is well-formed");
        return Witness {
            automaton,
            remembered,
            fresh,
        };
    }

    /// The literal language of the witness is not α-closed: `[⟦x, x]` is
    /// accepted but its α-variant `[⟦c, c]` is rejected, because the
    /// freshness gate on allocation sees the remembered `c`.
    #[test]
    fn literal_language_is_not_alpha_closed_before_dropping()
    {
        let witness = witness();
        let word = vec![Letter::Open(witness.fresh), Letter::Free(witness.fresh)];
        assert_eq!(
            witness.automaton.accepts(&word),
            Membership::ACCEPTED,
            "the unnamed-name variant is accepted"
        );
        let word = vec![
            Letter::Open(witness.remembered),
            Letter::Free(witness.remembered),
        ];
        assert_eq!(
            witness.automaton.accepts(&word),
            Membership::REJECTED,
            "its α-variant over the remembered name is rejected"
        );
    }

    /// After name-dropping the α-variant is accepted — the run first
    /// forgets the remembered name, then allocates it — while the donor's
    /// accepted word stays accepted (NDA Thm 6.9).
    #[test]
    fn name_dropping_closes_language_under_alpha()
    {
        let witness = witness();
        let dropped = name_dropping(&witness.automaton);
        let word = vec![
            Letter::Open(witness.remembered),
            Letter::Free(witness.remembered),
        ];
        assert_eq!(
            dropped.accepts(&word),
            Membership::ACCEPTED,
            "the α-variant is accepted once the remembered name can be dropped"
        );
        let word = vec![Letter::Open(witness.fresh), Letter::Free(witness.fresh)];
        assert_eq!(
            dropped.accepts(&word),
            Membership::ACCEPTED,
            "the donor's word stays accepted"
        );
    }

    /// The modification adds exactly one ε drop rule per (control point,
    /// register) pair, leaves the degree unchanged, and every added rule is
    /// a drop rule — the representation-level shadow of the same-degree /
    /// bounded-blowup bounds of RNTA Thm 5.5.
    #[test]
    fn name_dropping_adds_one_drop_rule_per_register()
    {
        let witness = witness();
        let donor_rules = witness.automaton.rules().len();
        let donor_degree = witness.automaton.degree();
        let dropped = name_dropping(&witness.automaton);
        let register_total = 1_usize.saturating_add(2_usize).saturating_add(1_usize);
        assert_eq!(
            dropped.rules().len(),
            donor_rules.saturating_add(register_total),
            "one drop rule per register"
        );
        assert_eq!(
            dropped.degree(),
            donor_degree,
            "the degree is unchanged (RNTA Thm 5.5)"
        );
        let added = dropped
            .rules()
            .iter()
            .skip(donor_rules)
            .all(|rule| matches!(rule.kind(), RuleKind::Drop { .. }));
        assert!(added, "every added rule is an ε drop rule");
    }

    /// The session monitor from `nda::tests`, rebuilt here for the
    /// monotonicity property: one admin plus at most one concurrent user,
    /// accepting exactly the logs drained of non-admin names at the end.
    fn session_monitor(admin: Atom<Role>) -> Nda<Role>
    {
        let q0 = Control::ZERO;
        let q1 = Control::from(1_u32);
        let register_admin = Register::ZERO;
        let register_user = Register::from(1_u32);
        let initial = Configuration::new(
            q0,
            Store::try_new(vec![Some(admin)]).expect("an injective store is valid"),
        );
        let rules = vec![
            Rule::open(q0, q1, vec![
                Transfer::Keep(register_admin),
                Transfer::Allocated,
            ]),
            Rule::close(q1, register_user, q0, vec![Transfer::Keep(register_admin)]),
            Rule::free(q0, register_admin, q0, vec![Transfer::Keep(register_admin)]),
            Rule::free(q1, register_admin, q1, vec![
                Transfer::Keep(register_admin),
                Transfer::Keep(register_user),
            ]),
        ];
        return Nda::new(
            vec![Arity::from(1_usize), Arity::from(2_usize)],
            initial,
            BTreeSet::from([q0]),
            rules,
        )
        .expect("the session monitor is well-formed");
    }

    /// A small index into the property's eight-letter alphabet, wrapped so
    /// the helper's signature stays nominal.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct LetterIndex(u32);

    /// Map a small index to one of the eight letters over the two property
    /// atoms; out-of-range indices yield no letter.
    fn letter_at(
        admin: Atom<Role>,
        user: Atom<Role>,
        index: LetterIndex,
    ) -> Option<Letter<Role>>
    {
        return match index.0 {
            | 0 => Some(Letter::Free(admin)),
            | 1 => Some(Letter::Open(admin)),
            | 2 => Some(Letter::Close(admin)),
            | 3 => Some(Letter::OpenClose(admin)),
            | 4 => Some(Letter::Free(user)),
            | 5 => Some(Letter::Open(user)),
            | 6 => Some(Letter::Close(user)),
            | 7 => Some(Letter::OpenClose(user)),
            | _ => None,
        };
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Every word the donor accepts, the modification accepts:
        /// `L(A) ⊆ L(A_⊥)` — the soundness half of NDA Thm 6.9, over random
        /// words of the session monitor's two-name alphabet.
        #[test]
        fn name_dropping_only_enlarges_the_language(indices in prop::collection::vec(0u32 .. 8, 0 .. 8))
        {
            let mut gensym = Gensym::new(Role::Resource);
            let admin = gensym
                .fresh()
                .expect("a new allocator can mint the first atom");
            let user = gensym
                .fresh()
                .expect("a new allocator can mint the second atom");
            let monitor = session_monitor(admin);
            let dropped = name_dropping(&monitor);
            let word: Vec<Letter<Role>> = indices
                .iter()
                .filter_map(|index| letter_at(admin, user, LetterIndex(*index)))
                .collect();
            if bool::from(monitor.accepts(&word)) {
                prop_assert_eq!(
                    dropped.accepts(&word),
                    Membership::ACCEPTED,
                    "every donor-accepted word stays accepted"
                );
            }
        }
    }
}
