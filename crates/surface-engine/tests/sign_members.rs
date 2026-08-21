//! What a `sign` block does when its members were not separated where the
//! reader could see it.
//!
//! `sort`, `oper`, `rule` and `data` are contextual rather than reserved, so a
//! member lead following an unterminated member is molded as whatever its
//! position suggests — the second `sort` of a newline-separated pair reads as a
//! type variable — and the member split, which looks for the lead's own label,
//! cannot see the boundary. Everything from that point on is dropped.
//!
//! The failure this suite exists for is what that used to produce: the higher-
//! cells flagship's own signature, whose members are newline-separated,
//! elaborated to a description holding one sort, no operations, no rules, and
//! **no diagnostic of any kind** — a presentation of a theory nobody wrote.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable `sign` member-separation tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_surface_engine::desc_elab::elaborate_data_descs;

    /// **A terminated indexed member is read as written.** The direct circuit
    /// signature keeps an indexed sort's parenthesized telescope in the member
    /// shape, so the description route can present it without inventing a
    /// missing-terminator diagnosis.
    #[test]
    fn a_terminated_block_is_not_told_it_lacks_terminators()
    {
        let source = "sign S {\n  sort Ob : Type;\n  sort Hom(dom: Ob, cod: Ob) : Type;\n  oper id : \
                      (a : Ob) --> Ob;\n}\n";
        let elab = elaborate_data_descs(source);
        let desc = elab.descs.first().expect("one description");
        assert_eq!(
            2,
            desc.sorts.len(),
            "both the base and indexed sorts are presented"
        );
        assert_eq!(1, desc.opers.len(), "the operation is presented");
        let reported = messages(&elab.diagnostics);
        assert!(
            !reported
                .iter()
                .any(|message| message.contains("were not separated")
                    || message.contains("reading stopped inside a member")),
            "terminated members are not reported as unread, got {reported:?}"
        );
    }

    /// **The silent case, and the one this check exists for.** The flagship's
    /// own shape: members separated by newlines rather than terminated.
    ///
    /// What made it dangerous is that the result looked like a successful
    /// elaboration. One sort was presented, the indexed sort and every
    /// operation and rule were gone, and nothing was reported — so a reader
    /// had no way to tell a partial reading from a small signature.
    #[test]
    fn unseparated_members_are_declined_rather_than_dropped()
    {
        // Separation is the ONLY defect here: two plain sorts, neither
        // terminated. A fixture carrying a second cause would be told the
        // observational message instead, which is the point of the sibling test.
        let source = "sign S {\n  sort Ob : Type\n  sort Ar : Type\n}\n";
        let elab = elaborate_data_descs(source);
        assert!(
            elab.descs.is_empty(),
            "a block read only in part presents nothing, got {} descriptions",
            elab.descs.len()
        );
        assert!(
            elab.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("were not separated")),
            "and the decline says where reading stopped and why, got {:?}",
            messages(&elab.diagnostics)
        );
    }

    /// **The separating case.** The same members, terminated, are all read —
    /// so the check keys on the separation rather than on the member kinds or
    /// on the block being large.
    #[test]
    fn terminated_members_are_all_presented()
    {
        let source = "sign S {\n  sort Ob : Type;\n  sort Ar : Type;\n  oper f : (a : Ob) --> \
                      Ob;\n}\n";
        let elab = elaborate_data_descs(source);
        let desc = elab.descs.first().expect("one description");
        assert_eq!(2, desc.sorts.len(), "both sorts are presented");
        assert_eq!(1, desc.opers.len(), "and so is the operation");
        assert!(
            !messages(&elab.diagnostics)
                .iter()
                .any(|message| message.contains("were not separated")
                    || message.contains("reading stopped inside a member")),
            "with no decline at all, got {:?}",
            messages(&elab.diagnostics)
        );
    }

    /// A block with one member cannot lose a second one, so it is unaffected
    /// whether or not that member is terminated.
    #[test]
    fn a_single_member_needs_no_terminator()
    {
        let source = "sign S {\n  oper f : (a : Ob) --> Ob\n}\n";
        let elab = elaborate_data_descs(source);
        let desc = elab.descs.first().expect("one description");
        assert_eq!(1, desc.opers.len(), "the sole member is read");
        assert!(
            !messages(&elab.diagnostics)
                .iter()
                .any(|message| message.contains("were not separated")
                    || message.contains("reading stopped inside a member")),
            "and nothing is declined, got {:?}",
            messages(&elab.diagnostics)
        );
    }

    /// A member the reader could not classify keeps its **own** report, at the
    /// member rather than at the block: the separation check must not displace
    /// the more precise decline.
    #[test]
    fn an_unreadable_member_keeps_its_own_report()
    {
        let source = "sign Adder {\n  sort Nat : Type;\n  oper add : (Nat, Nat) --> Nat;\n  rule \
                      unit ==> add;\n}\n";
        let elab = elaborate_data_descs(source);
        let reported = messages(&elab.diagnostics);
        assert!(
            reported.iter().any(|message| message.contains("unit")),
            "the report names the member, got {reported:?}"
        );
        assert!(
            !reported
                .iter()
                .any(|message| message.contains("were not separated")),
            "and not the block's separation, got {reported:?}"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// Every diagnostic message, in report order.
    fn messages(diagnostics: &[gandr_surface_engine::desc_elab::ElabDiagnostic]) -> Vec<String>
    {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    }
}
