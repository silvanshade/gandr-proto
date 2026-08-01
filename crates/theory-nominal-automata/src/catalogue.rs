//! The **decision-procedure catalogue** (design doc §6.3, §10): which
//! problems are decidable for which model of the family, by which named
//! construction, with the source papers' theorem numbers.
//!
//! The register, as a table (the [`CATALOGUE`] constant below is its
//! machine-checkable shadow):
//!
//! | Model | Problem | Status | Construction | Citation |
//! |-------|---------|--------|--------------|----------|
//! | RNNA | membership | inferred | forward reachability over configurations, as implemented for the NDA ([`crate::nda::Nda::accepts`]) | not stated in the papers; via the §5 template (§6.3 "Membership / run") |
//! | RNNA | emptiness | inferred | final-control reachability after name-dropping + S-restriction — classical NFA emptiness (NL) | not stated in the papers (design doc §9 hazard 5) |
//! | RNNA | inclusion | decidable | name-dropping → bounded S-restriction → classical NFA inclusion | Schröder–Kozen–Milius–Wißmann, `FoSSaCS` 2017 (arXiv:1603.01455), Cor 7.4 — via NDA \[21]; EXPSPACE, PSPACE-parametrized in the degree |
//! | RNNA | equivalence | decidable | mutual inclusion | as RNNA inclusion |
//! | RNNA | determinization | **impossible** | — | NDA Example 8.1 ("some letter has occurred before" needs unbounded support) |
//! | NDA | membership | inferred — **implemented here** | forward reachability ([`crate::nda::Nda::accepts`]); on [`crate::dropping::name_dropping`] output it decides α-closure membership (NDA Thm 6.9); on a DDA it is the deterministic online monitor (NDA Def 8.2; design doc §7.3) | not a stated theorem; engineering inference via the §5 template |
//! | NDA | emptiness | inferred | final-control reachability after name-dropping and S-restriction (`\|S\| = deg+1`, NDA Constr 7.11) — classical NFA emptiness | not stated in the paper (design doc §9 hazard 5) |
//! | NDA | inclusion | decidable | NDA→RNNA translation (NDA Prop 5.11) + RNNA inclusion | NDA Theorem 5.12; EXPSPACE, PSPACE-parametrized in the degree |
//! | NDA | equivalence | decidable | mutual inclusion | NDA Theorem 5.12 |
//! | NDA | determinization | decidable | name-dropping (Constr 6.3) → S-restriction (Constr 7.11) → disciplined S-restriction (Constr 8.3) → classical powerset → nominalization (Constr 7.8) | NDA Theorem 8.14; Corollary 8.15 (RNNA with empty-support initial state, determinized *as an NDA*) |
//! | NDA | Kleene compilation | decidable | regular deallocation expressions ⇄ D-NFA ⇄ NDA (Constr 7.8/7.11) | NDA Theorems 7.19 and 7.20 |
//! | RNTA | membership | inferred | top-down run simulation / S-restriction to classical NFTA membership (`\|S\| = d·n_ar + 1`, RNTA Lemma 6.2) | not stated in the paper; via the §5 template |
//! | RNTA | emptiness | inferred | NFTA emptiness (in P) after name-dropping + S-restriction | not stated in the paper (design doc §9 hazard 5) |
//! | RNTA | inclusion | decidable | name-dropping (Thm 5.5) → S-restriction (Lemma 6.2) → classical NFTA inclusion | RNTA Theorem 6.3 (alphatic = global = branchwise, via Lemma 3.7), Theorem 6.6 (local freshness); 2-EXPTIME, singly-exponential-parametrized in the degree; floor EXPTIME-complete (Seidl 1990, RNTA \[35]) |
//! | RNTA | equivalence | decidable | mutual inclusion | RNTA Theorems 6.3 and 6.6 |
//! | RNTA | determinization | **open** | — | top-down determinization not treated; bottom-up determinization produces orbit-infinite automata (Van Heerdt et al., RNTA \[41]; design doc §9 hazard 6) |
//!
//! The honesty discipline of design doc §9 is carried in the statuses:
//! paper theorems are `Decidable` with their verbatim theorem numbers;
//! consequences of the §5 reduction template that neither paper states are
//! `Inferred`; known impossibilities are `Impossible`; and problems the
//! literature leaves untreated are `Open`.

/// One model of the RNNA/NDA/RNTA family (design doc §6.2).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Model
{
    /// Regular nondeterministic nominal automata (`FoSSaCS` 2017) — the
    /// allocation-only word model, the family's shared ancestor.
    Rnna,
    /// Nondeterministic deallocation automata (arXiv:2603.24468v1) — the
    /// alloc+dealloc word model, and the family's determinizable member.
    Nda,
    /// Regular nominal tree automata (CONCUR 2024) — the allocation-only
    /// tree/term model.
    Rnta,
}

/// A problem posed to an automaton model.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Problem
{
    /// Is a given word or tree accepted?
    Membership,
    /// Is the accepted language empty?
    Emptiness,
    /// Is one automaton's language included in another's?
    Inclusion,
    /// Do two automata accept the same language?
    Equivalence,
    /// Can the model be determinized within its class?
    Determinization,
    /// Is there a Kleene-style expression syntax compiling to and from the
    /// model?
    KleeneCompilation,
}

/// The decidability status of one (model, problem) pair, with its provenance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Status
{
    /// Decidable by a named construction, stated as a theorem in the source
    /// papers.
    Decidable
    {
        /// The named decision construction.
        construction: &'static str,
        /// The paper theorem(s) owning the result, verbatim.
        citation: &'static str,
        /// The stated complexity bound and its degree-parametrization.
        complexity: &'static str,
    },
    /// Decidable as an engineering consequence of the design doc's §5
    /// reduction template — **not** a theorem stated in the papers
    /// (design doc §9 hazards 5 and 7).
    Inferred
    {
        /// The decision construction the template yields.
        construction: &'static str,
        /// The provenance qualification.
        note: &'static str,
    },
    /// Known to be impossible, stated in the source papers.
    Impossible
    {
        /// The paper example or theorem owning the impossibility.
        citation: &'static str,
    },
    /// Not treated in the source literature.
    Open
    {
        /// What the literature says around the gap.
        note: &'static str,
    },
}

/// One row of the decision-procedure catalogue.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CatalogueEntry
{
    /// The model the problem is posed to.
    pub model: Model,
    /// The problem posed.
    pub problem: Problem,
    /// The decidability status and its provenance.
    pub status: Status,
}

/// The decision-procedure register: the machine-checkable shadow of the
/// module-level table.
///
/// # Adequacy
/// - hypothesis: L3 only — the register is data, separated from an incomplete
///   register by the four core problems per model and by the three headline
///   facts (NDA determinizable, RNNA not, RNTA untreated), each witnessed
///   exactly.
/// - witness: `catalogue::tests::every_model_covers_the_four_core_problems`
/// - witness: `catalogue::tests::decidable_entries_name_construction_and_citation`
/// - witness: `catalogue::tests::headline_facts_are_registered`
pub const CATALOGUE: &[CatalogueEntry] = &[
    CatalogueEntry {
        model: Model::Rnna,
        problem: Problem::Membership,
        status: Status::Inferred {
            construction: "forward reachability over configurations, as implemented for the NDA (nda::Nda::accepts)",
            note: "not stated in the papers; via the design doc §5 template (§6.3 'Membership / run')",
        },
    },
    CatalogueEntry {
        model: Model::Rnna,
        problem: Problem::Emptiness,
        status: Status::Inferred {
            construction: "final-control reachability after name-dropping + S-restriction — classical NFA emptiness (in NL)",
            note: "neither paper states an emptiness theorem (design doc §9 hazard 5)",
        },
    },
    CatalogueEntry {
        model: Model::Rnna,
        problem: Problem::Inclusion,
        status: Status::Decidable {
            construction: "name-dropping → bounded S-restriction → classical NFA inclusion",
            citation: "Schröder–Kozen–Milius–Wißmann, FoSSaCS 2017 (arXiv:1603.01455), Cor 7.4 — via NDA [21]; NDA Thm 5.12",
            complexity: "EXPSPACE, PSPACE-parametrized in the degree",
        },
    },
    CatalogueEntry {
        model: Model::Rnna,
        problem: Problem::Equivalence,
        status: Status::Decidable {
            construction: "mutual inclusion",
            citation: "via RNNA inclusion (FoSSaCS 2017, Cor 7.4)",
            complexity: "EXPSPACE, PSPACE-parametrized in the degree",
        },
    },
    CatalogueEntry {
        model: Model::Rnna,
        problem: Problem::Determinization,
        status: Status::Impossible {
            citation: "NDA Example 8.1 — the language 'some letter has occurred before' needs unbounded support in a deterministic RNNA",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::Membership,
        status: Status::Inferred {
            construction: "forward reachability — implemented: nda::Nda::accepts (literal language); on dropping::name_dropping output it decides α-closure membership (NDA Thm 6.9); on a DDA it is the deterministic online monitor (NDA Def 8.2, design doc §7.3)",
            note: "not a stated theorem of the paper; engineering inference via the design doc §5 template",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::Emptiness,
        status: Status::Inferred {
            construction: "final-control reachability after name-dropping and S-restriction (|S| = deg+1, NDA Constr 7.11) — classical NFA emptiness",
            note: "not a stated theorem of the paper (design doc §9 hazard 5)",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::Inclusion,
        status: Status::Decidable {
            construction: "NDA→RNNA translation (NDA Prop 5.11) + RNNA inclusion [21, Cor 7.4]",
            citation: "NDA Theorem 5.12",
            complexity: "EXPSPACE, PSPACE-parametrized in the degree",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::Equivalence,
        status: Status::Decidable {
            construction: "mutual inclusion",
            citation: "NDA Theorem 5.12",
            complexity: "EXPSPACE, PSPACE-parametrized in the degree",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::Determinization,
        status: Status::Decidable {
            construction: "name-dropping (Constr 6.3) → S-restriction (Constr 7.11) → disciplined S-restriction (Constr 8.3) → classical powerset → nominalization (Constr 7.8)",
            citation: "NDA Theorem 8.14; Corollary 8.15 (RNNA with empty-support initial state, determinized as an NDA)",
            complexity: "powerset blowup; no bound stated in the paper",
        },
    },
    CatalogueEntry {
        model: Model::Nda,
        problem: Problem::KleeneCompilation,
        status: Status::Decidable {
            construction: "regular deallocation expressions ⇄ D-NFA ⇄ NDA (Constr 7.8 nominalization / Constr 7.11 S-restriction)",
            citation: "NDA Theorems 7.19 and 7.20",
            complexity: "standard Kleene conversions",
        },
    },
    CatalogueEntry {
        model: Model::Rnta,
        problem: Problem::Membership,
        status: Status::Inferred {
            construction: "top-down run simulation / S-restriction to classical NFTA membership (|S| = d·n_ar + 1, RNTA Lemma 6.2)",
            note: "not stated in the paper; via the design doc §5 template",
        },
    },
    CatalogueEntry {
        model: Model::Rnta,
        problem: Problem::Emptiness,
        status: Status::Inferred {
            construction: "NFTA emptiness (in P) after name-dropping + S-restriction",
            note: "not a stated theorem of the paper (design doc §9 hazard 5)",
        },
    },
    CatalogueEntry {
        model: Model::Rnta,
        problem: Problem::Inclusion,
        status: Status::Decidable {
            construction: "name-dropping (Thm 5.5) → S-restriction (Lemma 6.2) → classical NFTA inclusion",
            citation: "RNTA Theorem 6.3 (alphatic = global = branchwise, via Lemma 3.7); Theorem 6.6 (local freshness)",
            complexity: "2-EXPTIME, singly-exponential-parametrized in the degree; floor EXPTIME-complete (Seidl 1990, RNTA [35])",
        },
    },
    CatalogueEntry {
        model: Model::Rnta,
        problem: Problem::Equivalence,
        status: Status::Decidable {
            construction: "mutual inclusion",
            citation: "RNTA Theorems 6.3 and 6.6",
            complexity: "2-EXPTIME, singly-exponential-parametrized in the degree",
        },
    },
    CatalogueEntry {
        model: Model::Rnta,
        problem: Problem::Determinization,
        status: Status::Open {
            note: "top-down determinization not treated; bottom-up determinization produces orbit-infinite automata (Van Heerdt et al., RNTA [41]; design doc §9 hazard 6)",
        },
    },
];

#[cfg(test)]
mod tests
{
    use super::CATALOGUE;
    use super::Model;
    use super::Problem;
    use super::Status;

    /// Every model carries an entry for each of the four core problems of
    /// the catalogue's mandate (membership, emptiness, equivalence,
    /// inclusion).
    #[test]
    fn every_model_covers_the_four_core_problems()
    {
        let models = [Model::Rnna, Model::Nda, Model::Rnta];
        let problems = [
            Problem::Membership,
            Problem::Emptiness,
            Problem::Inclusion,
            Problem::Equivalence,
        ];
        for model in models {
            for problem in problems {
                assert!(
                    CATALOGUE
                        .iter()
                        .any(|entry| entry.model == model && entry.problem == problem),
                    "every model covers every core problem"
                );
            }
        }
    }

    /// Decidable entries name their construction, citation, and complexity;
    /// inferred entries name their construction and provenance note.
    #[test]
    fn decidable_entries_name_construction_and_citation()
    {
        for entry in CATALOGUE {
            match entry.status {
                | Status::Decidable {
                    construction,
                    citation,
                    complexity,
                } => {
                    assert!(!construction.is_empty());
                    assert!(!citation.is_empty());
                    assert!(!complexity.is_empty());
                },
                | Status::Inferred { construction, note } => {
                    assert!(!construction.is_empty());
                    assert!(!note.is_empty());
                },
                | Status::Impossible { citation } => {
                    assert!(!citation.is_empty());
                },
                | Status::Open { note } => {
                    assert!(!note.is_empty());
                },
            }
        }
    }

    /// The family's headline facts are registered with the right statuses:
    /// NDA determinization is decidable (Thm 8.14), RNNA determinization is
    /// impossible (Ex 8.1), RNTA determinization is untreated (\[41]).
    #[test]
    fn headline_facts_are_registered()
    {
        let nda = CATALOGUE
            .iter()
            .find(|entry| entry.model == Model::Nda && entry.problem == Problem::Determinization);
        assert!(
            matches!(nda, Some(entry) if matches!(entry.status, Status::Decidable { .. })),
            "NDA determinization is decidable"
        );
        let rnna = CATALOGUE
            .iter()
            .find(|entry| entry.model == Model::Rnna && entry.problem == Problem::Determinization);
        assert!(
            matches!(rnna, Some(entry) if matches!(entry.status, Status::Impossible { .. })),
            "RNNA determinization is impossible"
        );
        let rnta = CATALOGUE
            .iter()
            .find(|entry| entry.model == Model::Rnta && entry.problem == Problem::Determinization);
        assert!(
            matches!(rnta, Some(entry) if matches!(entry.status, Status::Open { .. })),
            "RNTA determinization is untreated"
        );
    }
}
