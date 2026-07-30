#[cfg(test)]
mod contracts
{
    use alloc::boxed::Box;
    use alloc::vec;
    use core::error::Error;

    use gandr_surface_grammar::Adaptation;
    use gandr_surface_grammar::AdaptationReason;
    use gandr_surface_grammar::Fixity;
    use gandr_surface_grammar::MoldId;
    use gandr_surface_grammar::OperatorDecl;
    use gandr_surface_grammar::Pbg;
    use gandr_surface_grammar::PbgError;
    use gandr_surface_grammar::Prec;
    use gandr_surface_grammar::PrecDag;
    use gandr_surface_grammar::PrecIndex;
    use gandr_surface_grammar::PrecName;
    use gandr_surface_grammar::PrecSpec;
    use gandr_surface_grammar::PrecTable;
    use gandr_surface_grammar::Regex;
    use gandr_surface_grammar::Rule;
    use gandr_surface_grammar::RuleName;
    use gandr_surface_grammar::Sort;
    use gandr_surface_grammar::SurfaceForm;
    use gandr_surface_grammar::TileLabel;
    use gandr_surface_grammar::built_in;
    use gandr_surface_grammar::validate_assumption_3;
    use gandr_surface_grammar::validate_unique_tiles;

    #[test]
    fn pbg_rejects_direct_adjacent_sorts_in_sequence() -> Result<(), Box<dyn Error>>
    {
        let (_dag, base) = one_node_dag()?;
        let error = exact_error(vec![Rule::new(
            RuleName("direct-adjacent"),
            Sort::Expression,
            base,
            Regex::seq([Regex::sort(Sort::Item), Regex::sort(Sort::Pattern)]),
        )])?;

        assert_eq!(
            PbgError::AdjacentSorts {
                rule: "direct-adjacent",
                left: Sort::Item,
                right: Sort::Pattern,
            },
            error
        );
        Ok(())
    }

    #[test]
    fn pbg_rejects_adjacency_exposed_by_nullable_sequence_paths() -> Result<(), Box<dyn Error>>
    {
        let (_dag, base) = one_node_dag()?;
        let cases = [
            (
                RuleName("optional-gap"),
                Regex::seq([
                    Regex::sort(Sort::Item),
                    Regex::optional(Regex::tile(TileLabel("maybe-comma"))),
                    Regex::sort(Sort::Pattern),
                ]),
                Sort::Item,
                Sort::Pattern,
            ),
            (
                RuleName("empty-gap"),
                Regex::seq([
                    Regex::sort(Sort::Pattern),
                    Regex::empty(),
                    Regex::sort(Sort::Expression),
                ]),
                Sort::Pattern,
                Sort::Expression,
            ),
            (
                RuleName("repeat-gap"),
                Regex::seq([
                    Regex::sort(Sort::Expression),
                    Regex::repeat(Regex::tile(TileLabel("zero-or-more-sep"))),
                    Regex::sort(Sort::Type),
                ]),
                Sort::Expression,
                Sort::Type,
            ),
            (
                RuleName("alt-empty-gap"),
                Regex::seq([
                    Regex::sort(Sort::Type),
                    Regex::alt([Regex::tile(TileLabel("alt-sep")), Regex::empty()]),
                    Regex::sort(Sort::Item),
                ]),
                Sort::Type,
                Sort::Item,
            ),
        ];

        for (name, regex, left, right) in cases {
            let error = exact_error(vec![Rule::new(name, Sort::Expression, base, regex)])?;
            assert_eq!(error, PbgError::AdjacentSorts {
                rule: name.0,
                left,
                right,
            });
        }
        Ok(())
    }

    #[test]
    fn pbg_accepts_terminal_separators_between_sort_uses() -> Result<(), Box<dyn Error>>
    {
        let (dag, base) = one_node_dag()?;
        let pbg = Pbg::build(dag, vec![
            Rule::new(
                RuleName("literal-separator"),
                Sort::Expression,
                base,
                Regex::seq([
                    Regex::sort(Sort::Item),
                    Regex::tile(TileLabel("comma")),
                    Regex::sort(Sort::Pattern),
                ]),
            ),
            Rule::new(
                RuleName("alt-literal-separator"),
                Sort::Type,
                base,
                Regex::seq([
                    Regex::sort(Sort::Expression),
                    Regex::alt([
                        Regex::tile(TileLabel("fat-arrow")),
                        Regex::tile(TileLabel("thin-arrow")),
                    ]),
                    Regex::sort(Sort::Type),
                ]),
            ),
        ])?;

        assert_eq!(2, pbg.rules().len());
        assert!(pbg.rule_names().contains("literal-separator"));
        assert!(pbg.rule_names().contains("alt-literal-separator"));
        Ok(())
    }

    #[test]
    fn pbg_rejects_invalid_operator_form_even_with_adaptation() -> Result<(), Box<dyn Error>>
    {
        let (_dag, base) = one_node_dag()?;
        let error = exact_error(vec![Rule::with_adaptation(
            RuleName("adapted-but-invalid"),
            Sort::Expression,
            base,
            Regex::seq([Regex::sort(Sort::Pattern), Regex::sort(Sort::Type)]),
            Adaptation::new(
                RuleName("adapted-but-invalid"),
                SurfaceForm("pattern type"),
                AdaptationReason("documented but still not an operator-form separator"),
            ),
        )])?;

        assert_eq!(
            PbgError::AdjacentSorts {
                rule: "adapted-but-invalid",
                left: Sort::Pattern,
                right: Sort::Type,
            },
            error
        );
        Ok(())
    }

    #[test]
    fn unique_tiles_contract()
    {
        // Genuine redundancy: an alternation of identical branches interns two
        // occurrences of the same tile to one rctx within one rule.
        let redundant = vec![Rule::new(
            RuleName("identical-branches"),
            Sort::Expression,
            Prec::new(PrecIndex::from(0)),
            Regex::alt([
                Regex::tile(TileLabel("shared")),
                Regex::tile(TileLabel("shared")),
            ]),
        )];
        assert_eq!(
            Err(PbgError::DuplicateTile {
                label: "shared",
                sort: Sort::Expression,
                prec: Prec::new(PrecIndex::from(0)),
                first_rule: "identical-branches",
                second_rule: "identical-branches",
            }),
            validate_unique_tiles(&redundant)
        );

        // Distinct positions receive distinct contexts, so a cloned element
        // (the comma1/repeat1 class) is not a duplicate.
        let cloned = vec![Rule::new(
            RuleName("cloned-element"),
            Sort::Expression,
            Prec::new(PrecIndex::from(0)),
            Regex::seq([
                Regex::tile(TileLabel("id")),
                Regex::repeat(Regex::seq([
                    Regex::tile(TileLabel(",")),
                    Regex::tile(TileLabel("id")),
                ])),
            ]),
        )];
        assert_eq!(Ok(()), validate_unique_tiles(&cloned));
    }

    #[test]
    fn pbg_rejects_duplicate_rctx_tile() -> Result<(), Box<dyn Error>>
    {
        let error = exact_error(vec![Rule::new(
            RuleName("identical-branches"),
            Sort::Expression,
            Prec::new(PrecIndex::from(0)),
            Regex::alt([
                Regex::tile(TileLabel("shared")),
                Regex::tile(TileLabel("shared")),
            ]),
        )])?;

        assert_eq!(
            PbgError::DuplicateTile {
                label: "shared",
                sort: Sort::Expression,
                prec: Prec::new(PrecIndex::from(0)),
                first_rule: "identical-branches",
                second_rule: "identical-branches",
            },
            error
        );
        Ok(())
    }

    #[test]
    fn pbg_accepts_same_label_at_distinct_contexts() -> Result<(), Box<dyn Error>>
    {
        let (dag, base) = one_node_dag()?;
        let pbg = Pbg::build(dag, vec![
            // Same label at two distinct Seq positions: opening and closing
            // brackets get distinct zipper contexts, hence distinct molds.
            Rule::new(
                RuleName("delimited"),
                Sort::Expression,
                base,
                Regex::seq([
                    Regex::tile(TileLabel("bracket")),
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel("bracket")),
                ]),
            ),
            // Same label in a different rule: rule-scoped rctx keeps it distinct.
            Rule::new(
                RuleName("bare"),
                Sort::Type,
                base,
                Regex::tile(TileLabel("bracket")),
            ),
        ])?;

        assert_eq!(2, pbg.rules().len());
        assert_eq!(3, pbg.candidates(TileLabel("bracket")).len());
        Ok(())
    }

    #[test]
    fn assumption_3_contract()
    {
        // A form of Expression begins with Type and a form of Type ends with
        // Expression: distinct sorts r != s with s in FIRST(G(r, p)) and
        // r in LAST(G(s, q)).
        let conflict = vec![
            Rule::new(
                RuleName("expr-begins-type"),
                Sort::Expression,
                Prec::new(PrecIndex::from(0)),
                Regex::seq([Regex::sort(Sort::Type), Regex::tile(TileLabel("x"))]),
            ),
            Rule::new(
                RuleName("type-ends-expr"),
                Sort::Type,
                Prec::new(PrecIndex::from(0)),
                Regex::seq([Regex::tile(TileLabel("y")), Regex::sort(Sort::Expression)]),
            ),
        ];
        assert_eq!(
            Err(PbgError::Assumption3Conflict {
                first_sort: Sort::Expression,
                second_sort: Sort::Type,
            }),
            validate_assumption_3(&conflict)
        );

        // No cross-sort begin/end pair: accepted.
        let ok = vec![
            Rule::new(
                RuleName("expr-begins-type-only"),
                Sort::Expression,
                Prec::new(PrecIndex::from(0)),
                Regex::seq([Regex::sort(Sort::Type), Regex::tile(TileLabel("x"))]),
            ),
            Rule::new(
                RuleName("type-begins-tile"),
                Sort::Type,
                Prec::new(PrecIndex::from(0)),
                Regex::seq([Regex::tile(TileLabel("y")), Regex::sort(Sort::Type)]),
            ),
        ];
        assert_eq!(Ok(()), validate_assumption_3(&ok));
    }

    #[test]
    fn pbg_rejects_duplicate_rule_names_deterministically() -> Result<(), Box<dyn Error>>
    {
        let (_dag, base) = one_node_dag()?;
        let error = exact_error(vec![
            Rule::new(
                RuleName("repeated-rule"),
                Sort::Expression,
                base,
                Regex::tile(TileLabel("first-label")),
            ),
            Rule::new(
                RuleName("repeated-rule"),
                Sort::Type,
                base,
                Regex::tile(TileLabel("second-label")),
            ),
        ])?;

        assert_eq!(
            PbgError::DuplicateRule {
                name: "repeated-rule",
            },
            error
        );
        Ok(())
    }

    #[test]
    fn pbg_rejects_invalid_prec_before_later_header_errors() -> Result<(), Box<dyn Error>>
    {
        let (_dag, _base) = one_node_dag()?;
        let invalid = Prec::new(PrecIndex::from(1));
        let error = exact_error(vec![
            Rule::new(
                RuleName("invalid-before-duplicate"),
                Sort::Expression,
                invalid,
                Regex::tile(TileLabel("invalid-prec-label")),
            ),
            Rule::new(
                RuleName("invalid-before-duplicate"),
                Sort::Type,
                Prec::new(PrecIndex::from(0)),
                Regex::tile(TileLabel("duplicate-name-label")),
            ),
        ])?;

        assert_eq!(error, PbgError::InvalidPrec {
            rule: "invalid-before-duplicate",
            prec: invalid,
        });
        Ok(())
    }

    fn one_node_dag() -> Result<(PrecDag, Prec), Box<dyn Error>>
    {
        let (table, base) = one_node_table()?;
        Ok((table.into_dag(), base))
    }

    fn exact_error(rules: vec::Vec<Rule>) -> Result<PbgError, Box<dyn Error>>
    {
        let (dag, _base) = one_node_dag()?;
        match Pbg::build(dag, rules) {
            | Ok(_pbg) => panic!("PBG build unexpectedly succeeded"),
            | Err(error) => Ok(error),
        }
    }

    fn one_node_table() -> Result<(PrecTable, Prec), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let base = spec.insert("base", None)?;
        let dag = PrecDag::build(&spec)?;
        Ok((PrecTable::new(dag, [(PrecName("base"), base)]), base))
    }

    #[test]
    fn extend_folds_a_declared_operator() -> Result<(), Box<dyn Error>>
    {
        // `Pbg::extend` folds a declared operator into a fresh grammar: the base
        // has no such tile, the extension molds it at the Expression sort, and
        // the fingerprint shifts. The base is left untouched (extend clones).
        let base = built_in()?;
        assert!(
            base.candidates(TileLabel("<|>")).is_empty(),
            "the base grammar declares no `<|>` operator"
        );
        let base_fingerprint = base.fingerprint();
        let base_molds = base.mold_count();

        let extended = Pbg::extend(&base, &[OperatorDecl::new(
            TileLabel("<|>"),
            Fixity::Infixl,
        )])?;
        let molds = extended.candidates(TileLabel("<|>"));
        assert_eq!(1, molds.len(), "the extension declares one `<|>` mold");
        let def = extended.mold(molds[0])?;
        assert_eq!("<|>", def.label);
        assert_eq!(
            Sort::Expression,
            def.sort,
            "the operator molds an expression"
        );
        assert_ne!(
            extended.fingerprint(),
            base_fingerprint,
            "an extended grammar has a distinct fingerprint"
        );
        assert_eq!(
            extended.mold_count().0,
            base_molds.0.saturating_add(1),
            "the infix operator adds exactly its one tile occurrence"
        );

        // The base tables are unchanged: extend never mutates `base`.
        assert!(
            base.candidates(TileLabel("<|>")).is_empty(),
            "the base is untouched"
        );
        assert_eq!(base.fingerprint(), base_fingerprint);
        assert_eq!(base.mold_count(), base_molds);
        Ok(())
    }

    #[test]
    fn extend_preserves_base_mold_ids() -> Result<(), Box<dyn Error>>
    {
        // Mold identity is stable across extension: appending operator rules
        // preserves every base rule's prefix, so a `Cst` molded against the base
        // keeps its `MoldId`s. Every base candidate menu is unchanged, and the
        // new operator molds take fresh higher ids.
        let base = built_in()?;
        let extended = Pbg::extend(&base, &[
            OperatorDecl::new(TileLabel("<|>"), Fixity::Infixl),
            OperatorDecl::new(TileLabel("<?>"), Fixity::Prefix),
        ])?;
        // Every base label keeps exactly its base molds (same ids, same order).
        for (label, _count) in base.candidate_counts() {
            assert_eq!(
                base.candidates(label),
                extended.candidates(label),
                "base label {label:?} keeps its molds under extension"
            );
        }
        // The new operator molds are fresh ids beyond the base table.
        let base_max = base.mold_count();
        for spelling in ["<|>", "<?>"] {
            let molds: &[MoldId] = extended.candidates(TileLabel(spelling));
            for &mold in molds {
                let index = usize::try_from(u32::from(mold))?;
                assert!(
                    index >= base_max.0,
                    "the operator {spelling:?} mold {mold:?} is a fresh id ≥ {base_max:?}"
                );
            }
        }
        assert_eq!(extended.mold_count().0, base_max.0.saturating_add(2));
        Ok(())
    }

    #[test]
    fn extend_reports_a_duplicate_declaration() -> Result<(), Box<dyn Error>>
    {
        // Two identical declarations collide on the rule name (the spelling),
        // surfacing as a typed `DuplicateRule` — a defined failure, not a panic.
        let base = built_in()?;
        let error = Pbg::extend(&base, &[
            OperatorDecl::new(TileLabel("<|>"), Fixity::Infixl),
            OperatorDecl::new(TileLabel("<|>"), Fixity::Infixr),
        ])
        .expect_err("duplicate operator names must fail");
        assert_eq!(PbgError::DuplicateRule { name: "<|>" }, error);
        Ok(())
    }
}
