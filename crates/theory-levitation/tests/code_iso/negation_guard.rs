//! **U3.0c — the permanent negation negative test** (design note §4.3; the iu
//! `BlanketBase` pattern; proposal §6.4 kill-signal **P-c**).
//!
//! # What this guards
//!
//! That `CodeIso(Boolean, Boolean)` has **at least two replay-inequivalent
//! members** — the identity and negation certificates. Both have the *same*
//! source code and the *same* target code (they are auto-isos of one
//! description), yet they are distinct transformations: negation disagrees with
//! the identity on `False` (it yields `True`). The identity of certificates is
//! **replay-equivalence** (ADR-69 D1), not equality of codes.
//!
//! # Why it is permanent
//!
//! `ua-base` states `CodeIso(x ⨟ y) ≅ (x ⤳ y)` (design note §4.1). If anyone
//! ever realizes the path protype `⤳` as **decidable code equality** — a
//! "blanket base", the iu `BlanketBase` cliff — then the instance set at
//! `(Boolean, Boolean)` collapses to a *singleton*: two auto-isos of one
//! description have equal boundary codes, so a code-equality identity would
//! merge identity and negation into one member. That collapse is exactly kill
//! signal **P-c** ("a UIP-style collapse forced at any stratum … is a STOP")
//! and it would make ua-base **refuted at the base stratum** (its completeness
//! half needs the nontrivial member to have a code-path preimage).
//!
//! This test is written so the collapse **fails loudly here** rather than
//! silently: it pins that two isos which decidable code equality *would*
//! identify (asserted: equal source codes, equal target codes) are nevertheless
//! replay-distinct. Should the certificate identity ever drift from
//! replay-equivalence toward code equality, this named test breaks — it is a
//! standing guard, not a design-note footnote, and must not be deleted or
//! weakened without owner sign-off (it enforces P-c).

#[cfg(test)]
mod u30c
{
    use gandr_theory_levitation::DescValue;
    use gandr_theory_levitation::Payload;
    use gandr_theory_levitation::ReplayClassCount;
    use gandr_theory_levitation::generic_eq;

    use crate::code_iso::fixtures;
    use crate::code_iso::harness::CodeIso;
    use crate::code_iso::harness::replay_disagreement;
    use crate::code_iso::harness::replay_equivalent;

    #[test]
    fn code_iso_bool_bool_has_at_least_two_replay_inequivalent_members()
    {
        let identity = fixtures::identity_bool();
        let negation = fixtures::negation_bool();
        let samples = fixtures::bool_two_ctor_values();

        // The core guard: identity and negation are NOT the same transformation.
        assert!(
            !bool::from(replay_equivalent(&identity, &negation, &samples, &samples)),
            "identity and negation are replay-inequivalent — CodeIso(Bool, Bool) is not a singleton"
        );
        assert!(
            usize::from(distinct_up_to_replay(
                &[identity, negation],
                &samples,
                &samples
            )) >= 2,
            "CodeIso(Bool, Bool) has at least two replay-inequivalent members"
        );
    }
    /// The number of certificates in `members` that are pairwise
    /// **replay-inequivalent** (a greedy dedup by [`replay_equivalent`] over
    /// the samples).
    fn distinct_up_to_replay(
        members: &[CodeIso],
        source_samples: &[DescValue],
        target_samples: &[DescValue],
    ) -> ReplayClassCount
    {
        let mut representatives: Vec<&CodeIso> = Vec::new();
        for member in members {
            let already = representatives.iter().any(|representative| {
                bool::from(replay_equivalent(
                    representative,
                    member,
                    source_samples,
                    target_samples,
                ))
            });
            if !already {
                representatives.push(member);
            }
        }
        representatives.len().into()
    }

    #[test]
    fn identity_and_negation_share_their_boundary_codes()
    {
        // The premise decidable code equality would exploit: both are auto-isos
        // of one description, so their source codes coincide and their target
        // codes coincide. A code-equality identity therefore CANNOT tell them
        // apart — only replay can.
        let identity = fixtures::identity_bool();
        let negation = fixtures::negation_bool();
        assert_eq!(
            identity.source(),
            negation.source(),
            "identity and negation have the same source code"
        );
        assert_eq!(
            identity.target(),
            negation.target(),
            "identity and negation have the same target code"
        );
        assert_eq!(
            identity.source(),
            identity.target(),
            "the boundary is the endo-boundary (Boolean, Boolean)"
        );
    }

    #[test]
    fn the_disagreement_is_witnessed_on_false()
    {
        // The inspectable witness the guard rests on: at `False`, the identity
        // yields `False` and negation yields `True`. Were `⤳` decidable code
        // equality, no such witness could exist (the members would be equal).
        let identity = fixtures::identity_bool();
        let negation = fixtures::negation_bool();
        let samples = fixtures::bool_two_ctor_values();
        let false_value = DescValue::new(0.into(), Payload::Unit);
        let true_value = DescValue::new(1.into(), Payload::Unit);

        let (input, identity_image, negation_image) =
            replay_disagreement(&identity, &negation, &samples, &samples)
                .expect("identity and negation disagree under replay");
        let boolean = fixtures::bool_two_ctor();
        assert!(
            bool::from(generic_eq(&boolean, &input, &false_value)),
            "the earliest disagreement is on False"
        );
        assert!(
            bool::from(generic_eq(&boolean, &identity_image, &false_value)),
            "the identity fixes False"
        );
        assert!(
            bool::from(generic_eq(&boolean, &negation_image, &true_value)),
            "negation sends False to True"
        );
        assert!(
            !bool::from(generic_eq(&boolean, &identity_image, &negation_image)),
            "the two images are distinct — the collapse guard bites"
        );
    }
}
