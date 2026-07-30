#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use core::error::Error;

    use gandr_surface_grammar::Assoc;
    use gandr_surface_grammar::NamedKind;
    use gandr_surface_grammar::NamedKindRealization;
    use gandr_surface_grammar::PBG_ONLY_KINDS;
    use gandr_surface_grammar::Pbg;
    use gandr_surface_grammar::PbgError;
    use gandr_surface_grammar::Prec;
    use gandr_surface_grammar::PrecDag;
    use gandr_surface_grammar::PrecName;
    use gandr_surface_grammar::PrecSpec;
    use gandr_surface_grammar::Sort;
    use gandr_surface_grammar::TREE_SITTER_NAMED_KINDS;
    use gandr_surface_grammar::built_in;
    use gandr_surface_grammar::built_in_prec_table;
    use gandr_surface_grammar::named_kind_parity;
    use gandr_surface_grammar::named_kind_realization;
    use gandr_surface_parser::parse;
    use gandr_surface_syntax::GroutSort;
    use gandr_surface_syntax::SourceSlice;

    const EXPECTED_PRECEDENCE_GROUPS: &[(&str, Option<Assoc>)] = &[
        ("item.singleton", None),
        ("expression.atom", None),
        ("expression.postfix", Some(Assoc::Left)),
        ("expression.unary", None),
        ("expression.mul", Some(Assoc::Left)),
        ("expression.add", Some(Assoc::Left)),
        ("expression.cmp", Some(Assoc::Left)),
        ("expression.and", Some(Assoc::Left)),
        ("expression.or", Some(Assoc::Left)),
        ("expression.ret", Some(Assoc::Right)),
        ("pattern.atom", None),
        ("pattern.as", Some(Assoc::Left)),
        ("pattern.or", Some(Assoc::Left)),
        ("type.atom", None),
        ("type.application", None),
        ("type.product", Some(Assoc::Right)),
        ("type.sum", Some(Assoc::Right)),
        ("type.union", Some(Assoc::Right)),
        ("type.intersection", Some(Assoc::Right)),
        ("type.lazy_product", Some(Assoc::Right)),
        ("type.arrow", Some(Assoc::Right)),
    ];

    const EXPECTED_PRECEDENCE_EDGES: &[(&str, &str)] = &[
        ("expression.atom", "expression.postfix"),
        ("expression.postfix", "expression.unary"),
        ("expression.unary", "expression.mul"),
        ("expression.mul", "expression.add"),
        ("expression.add", "expression.cmp"),
        ("expression.cmp", "expression.and"),
        ("expression.and", "expression.or"),
        ("expression.or", "expression.ret"),
        ("pattern.atom", "pattern.as"),
        ("pattern.as", "pattern.or"),
        ("type.atom", "type.application"),
        ("type.application", "type.product"),
        ("type.product", "type.sum"),
        ("type.sum", "type.union"),
        ("type.sum", "type.intersection"),
        ("type.sum", "type.lazy_product"),
        ("type.union", "type.arrow"),
        ("type.intersection", "type.arrow"),
        ("type.lazy_product", "type.arrow"),
    ];

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CaseLabel(&'static str);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ParseSource(&'static str);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ParseCase
    {
        label: CaseLabel,
        source: ParseSource,
    }

    #[test]
    fn named_kind_coverage_is_semantic() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let committed = TREE_SITTER_NAMED_KINDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            TREE_SITTER_NAMED_KINDS.len(),
            124,
            "committed named-kind count"
        );
        assert_eq!(124, committed.len(), "named kinds must be unique");

        // The W4d PBG-only registry is disjoint from the committed tree-sitter
        // kinds: the parity exemption never overlaps real tree-sitter parity.
        let pbg_only = PBG_ONLY_KINDS.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(
            PBG_ONLY_KINDS.len(),
            pbg_only.len(),
            "PBG-only kinds must be unique"
        );
        assert!(
            pbg_only.is_disjoint(&committed),
            "PBG-only kinds must not overlap committed tree-sitter named kinds"
        );
        // Provenance / adaptation surfaces may be committed OR PBG-only.
        let recognised = committed.union(&pbg_only).copied().collect::<BTreeSet<_>>();

        // No terminal-only coverage rules remain in the PBG.
        for rule in pbg.rules() {
            assert!(
                !rule.name().as_ref().starts_with("tree_sitter_named_kind."),
                "terminal-only coverage rules must have left the PBG: {}",
                rule.name()
            );
        }

        // Every rule's provenance is a committed named kind or a PBG-only kind.
        let provenances = pbg
            .rules()
            .iter()
            .map(|rule| rule.provenance().0)
            .collect::<BTreeSet<_>>();
        assert!(
            provenances.is_subset(&recognised),
            "no rule provenance outside the committed or PBG-only kind set"
        );
        assert!(
            provenances.contains("module_declaration"),
            "the checked PBG realises module_declaration as a rule provenance"
        );

        // Kinds folded into a factored family (or a PBG-only member) are
        // realised by the representative rule and recorded as adaptations
        // naming the original form; they cover their kind exactly as a
        // dedicated provenance would.
        let folded = pbg
            .adaptations()
            .iter()
            .map(|adaptation| adaptation.surface)
            .collect::<BTreeSet<_>>();
        assert!(
            folded.is_subset(&recognised),
            "no folded adaptation kind outside the committed or PBG-only kind set"
        );
        // Every PBG-only kind is actually realised (a provenance or adaptation
        // surface), so the registry carries no dead entries.
        let realised = provenances.union(&folded).copied().collect::<BTreeSet<_>>();
        for kind in &pbg_only {
            assert!(
                realised.contains(kind),
                "PBG-only kind {kind:?} must be realised by a rule provenance or adaptation surface"
            );
        }

        // Every committed named kind maps to a real form (or the file root).
        let item_has_forms = pbg
            .forms()
            .keys()
            .any(|&(form_sort, _prec)| form_sort == Sort::Item);
        for entry in named_kind_parity() {
            match entry.realization {
                | NamedKindRealization::StructuralForms => assert!(
                    provenances.contains(entry.kind) || folded.contains(entry.kind),
                    "named kind {:?} must be generated by a real structural form (directly or as a factored family adaptation)",
                    entry.kind
                ),
                | NamedKindRealization::FileRoot => assert!(
                    item_has_forms,
                    "the file root {:?} must be the Item-sort item sequence",
                    entry.kind
                ),
            }
        }

        // The parity inventory covers exactly the committed kinds.
        let parity_kinds = named_kind_parity()
            .into_iter()
            .map(|entry| entry.kind)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            parity_kinds, committed,
            "the parity inventory must enumerate exactly the committed named kinds"
        );
        assert_eq!(
            NamedKindRealization::FileRoot,
            named_kind_realization(NamedKind("source_file")),
            "the file root is realised by the Item sort, not a terminal-only rule"
        );
        assert_eq!(
            NamedKindRealization::StructuralForms,
            named_kind_realization(NamedKind("def_value")),
            "a committed construct is realised by a structural form"
        );

        assert_has_checked_form(&pbg, Sort::Item);
        assert_has_checked_form(&pbg, Sort::Pattern);
        assert_has_checked_form(&pbg, Sort::Expression);
        assert_has_checked_form(&pbg, Sort::Type);
        assert_has_checked_form(&pbg, Sort::Instantiation);
        Ok(())
    }

    fn assert_has_checked_form(
        pbg: &Pbg,
        sort: Sort,
    )
    {
        assert!(
            pbg.forms()
                .keys()
                .any(|&(form_sort, _prec)| form_sort == sort),
            "built-in PBG must have at least one checked {} form",
            sort.name()
        );
    }

    #[test]
    fn sort_decode_contract()
    {
        let cases = [
            (0, Sort::Item),
            (1, Sort::Pattern),
            (2, Sort::Expression),
            (3, Sort::Type),
            (4, Sort::Instantiation),
        ];
        for (tag, expected) in cases {
            assert_eq!(Ok(expected), Sort::try_from_tag(GroutSort(tag)));
        }
        assert_eq!(
            Err(PbgError::InvalidSort { sort: 5 }),
            Sort::try_from_tag(GroutSort(5))
        );
    }

    #[test]
    fn built_in_precedence_bands_are_exact() -> Result<(), Box<dyn Error>>
    {
        let precs = built_in_prec_table()?;
        let dag = precs.dag();
        let groups = dag
            .groups()
            .map(|(_prec, name, assoc)| (<&str>::from(name), assoc))
            .collect::<Vec<_>>();
        assert_eq!(
            EXPECTED_PRECEDENCE_GROUPS,
            groups.as_slice(),
            "dense precedence groups"
        );

        let edges = dag
            .edges()
            .map(|(tighter, looser)| {
                let tighter_name = dag.name(tighter).expect("known tighter precedence");
                let looser_name = dag.name(looser).expect("known looser precedence");
                (<&str>::from(tighter_name), <&str>::from(looser_name))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            EXPECTED_PRECEDENCE_EDGES,
            edges.as_slice(),
            "exact tighter-to-looser edges"
        );

        assert_chain(dag, &[
            PrecName("expression.atom"),
            PrecName("expression.postfix"),
            PrecName("expression.unary"),
            PrecName("expression.mul"),
            PrecName("expression.add"),
            PrecName("expression.cmp"),
            PrecName("expression.and"),
            PrecName("expression.or"),
            PrecName("expression.ret"),
        ]);
        assert_chain(dag, &[
            PrecName("pattern.atom"),
            PrecName("pattern.as"),
            PrecName("pattern.or"),
        ]);
        assert_chain(dag, &[
            PrecName("type.atom"),
            PrecName("type.application"),
            PrecName("type.product"),
            PrecName("type.sum"),
        ]);
        assert_chain(dag, &[
            PrecName("type.sum"),
            PrecName("type.union"),
            PrecName("type.arrow"),
        ]);
        assert_chain(dag, &[
            PrecName("type.sum"),
            PrecName("type.intersection"),
            PrecName("type.arrow"),
        ]);
        assert_chain(dag, &[
            PrecName("type.sum"),
            PrecName("type.lazy_product"),
            PrecName("type.arrow"),
        ]);

        for name in [
            PrecName("item.singleton"),
            PrecName("expression.atom"),
            PrecName("expression.unary"),
            PrecName("pattern.atom"),
            PrecName("type.atom"),
            PrecName("type.application"),
        ] {
            assert_non_assoc(dag, name);
        }
        for name in [
            PrecName("expression.postfix"),
            PrecName("expression.mul"),
            PrecName("expression.add"),
            PrecName("expression.cmp"),
            PrecName("expression.and"),
            PrecName("expression.or"),
            PrecName("pattern.as"),
            PrecName("pattern.or"),
        ] {
            assert_left_assoc(dag, name);
        }
        assert_right_assoc(dag, PrecName("expression.ret"));
        assert_right_assoc(dag, PrecName("type.product"));
        assert_right_assoc(dag, PrecName("type.sum"));
        assert_right_assoc(dag, PrecName("type.union"));
        assert_right_assoc(dag, PrecName("type.intersection"));
        assert_right_assoc(dag, PrecName("type.lazy_product"));
        assert_right_assoc(dag, PrecName("type.arrow"));

        assert_incomparable(dag, PrecName("item.singleton"), PrecName("expression.atom"));
        assert_incomparable(dag, PrecName("item.singleton"), PrecName("pattern.atom"));
        assert_incomparable(dag, PrecName("item.singleton"), PrecName("type.atom"));
        assert_incomparable(dag, PrecName("expression.ret"), PrecName("pattern.or"));
        assert_incomparable(dag, PrecName("expression.atom"), PrecName("type.arrow"));
        assert_incomparable(dag, PrecName("pattern.atom"), PrecName("type.atom"));
        assert_incomparable(dag, PrecName("type.union"), PrecName("type.intersection"));
        assert_incomparable(dag, PrecName("type.union"), PrecName("type.lazy_product"));
        assert_incomparable(
            dag,
            PrecName("type.intersection"),
            PrecName("type.lazy_product"),
        );
        Ok(())
    }

    fn assert_chain(
        dag: &PrecDag,
        names: &[PrecName],
    )
    {
        for pair in names.windows(2) {
            let [tighter_name, looser_name] = pair else {
                continue;
            };
            let tighter = prec(dag, *tighter_name);
            let looser = prec(dag, *looser_name);
            assert!(
                bool::from(dag.gt(tighter, looser, None)),
                "{} must bind tighter than {}",
                tighter_name,
                looser_name
            );
            assert!(
                bool::from(dag.lt(looser, tighter, None)),
                "{} must bind looser than {}",
                looser_name,
                tighter_name
            );
        }
    }

    fn assert_non_assoc(
        dag: &PrecDag,
        name: PrecName,
    )
    {
        let group = prec(dag, name);
        assert_eq!(Some(None), dag.assoc(group), "{name} associativity");
        assert!(
            bool::from(dag.eq(group, group, None)),
            "{name} admits equality only without associativity"
        );
        assert!(
            !bool::from(dag.gt(group, group, Some(Assoc::Left))),
            "{name} is not left associative"
        );
        assert!(
            !bool::from(dag.lt(group, group, Some(Assoc::Right))),
            "{name} is not right associative"
        );
    }

    fn assert_left_assoc(
        dag: &PrecDag,
        name: PrecName,
    )
    {
        let group = prec(dag, name);
        assert_eq!(
            Some(Some(Assoc::Left)),
            dag.assoc(group),
            "{name} associativity"
        );
        assert!(
            bool::from(dag.gt(group, group, Some(Assoc::Left))),
            "{name} is left associative"
        );
        assert!(
            !bool::from(dag.lt(group, group, Some(Assoc::Left))),
            "{name} is not right associative"
        );
        assert!(
            !bool::from(dag.eq(group, group, None)),
            "{name} is not non-associative equality"
        );
    }

    fn assert_right_assoc(
        dag: &PrecDag,
        name: PrecName,
    )
    {
        let group = prec(dag, name);
        assert_eq!(
            Some(Some(Assoc::Right)),
            dag.assoc(group),
            "{name} associativity"
        );
        assert!(
            bool::from(dag.lt(group, group, Some(Assoc::Right))),
            "{name} is right associative"
        );
        assert!(
            !bool::from(dag.gt(group, group, Some(Assoc::Right))),
            "{name} is not left associative"
        );
        assert!(
            !bool::from(dag.eq(group, group, None)),
            "{name} is not non-associative equality"
        );
    }

    fn assert_incomparable(
        dag: &PrecDag,
        left: PrecName,
        right: PrecName,
    )
    {
        let left_prec = prec(dag, left);
        let right_prec = prec(dag, right);
        assert!(
            !bool::from(dag.comparable(left_prec, right_prec)),
            "{left} and {right} are cross-sort incomparable"
        );
        assert!(
            !bool::from(dag.lt(left_prec, right_prec, None)),
            "{left} is not looser than {right}"
        );
        assert!(
            !bool::from(dag.gt(left_prec, right_prec, None)),
            "{left} is not tighter than {right}"
        );
    }

    fn prec(
        dag: &PrecDag,
        name: PrecName,
    ) -> Prec
    {
        dag.groups()
            .find_map(|(prec, candidate, _assoc)| (candidate == name.as_ref()).then_some(prec))
            .unwrap_or_else(|| panic!("missing precedence group `{name}`"))
    }

    #[test]
    fn right_associative_type_operator_chains_parse_cleanly() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let cases = [
            ParseCase {
                label: CaseLabel("three-member product"),
                source: ParseSource("def f : A * B * C;"),
            },
            ParseCase {
                label: CaseLabel("four-member product"),
                source: ParseSource("def f : A * B * C * D;"),
            },
            ParseCase {
                label: CaseLabel("three-member sum"),
                source: ParseSource("def f : A + B + C;"),
            },
            ParseCase {
                label: CaseLabel("four-member sum"),
                source: ParseSource("def f : A + B + C + D;"),
            },
            ParseCase {
                label: CaseLabel("three-member lazy product"),
                source: ParseSource("def f : F A & F B & F C;"),
            },
            ParseCase {
                label: CaseLabel("four-member lazy product"),
                source: ParseSource("def f : F A & F B & F C & F D;"),
            },
            ParseCase {
                label: CaseLabel("three-member union"),
                source: ParseSource("def f : A | B | C;"),
            },
            ParseCase {
                label: CaseLabel("four-member union"),
                source: ParseSource("def f : A | B | C | D;"),
            },
            ParseCase {
                label: CaseLabel("three-member intersection"),
                source: ParseSource("def f : A /\\ B /\\ C;"),
            },
            ParseCase {
                label: CaseLabel("four-member intersection"),
                source: ParseSource("def f : A /\\ B /\\ C /\\ D;"),
            },
        ];
        for case in cases {
            assert_parses_clean(&pbg, case)?;
        }
        Ok(())
    }

    #[test]
    fn recursion_marker_instantiations_parse_cleanly() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let cases = [
            ParseCase {
                label: CaseLabel("descending recursion marker"),
                source: ParseSource("def rec f() { f[<]() }"),
            },
            ParseCase {
                label: CaseLabel("productive recursion marker"),
                source: ParseSource("def rec f() { f[>]() }"),
            },
            ParseCase {
                label: CaseLabel("productive copattern marker"),
                source: ParseSource("def rec n() { .head => 0, .tail => n[>]() }"),
            },
            ParseCase {
                label: CaseLabel("combined recursion markers"),
                source: ParseSource("def rec f() { f[<, >]() }"),
            },
            ParseCase {
                label: CaseLabel("reserved named measure"),
                source: ParseSource("def rec f() { f[n <]() }"),
            },
            ParseCase {
                label: CaseLabel("reserved explicit resident"),
                source: ParseSource("def rec f() { f[n = 1]() }"),
            },
            ParseCase {
                label: CaseLabel("reserved explicit size"),
                source: ParseSource("def rec f() { f[size = 1]() }"),
            },
            ParseCase {
                label: CaseLabel("reserved cost bound"),
                source: ParseSource("def rec f() { f[cost = 1]() }"),
            },
            ParseCase {
                label: CaseLabel("reserved tail resident"),
                source: ParseSource("def rec f() { f[tail]() }"),
            },
            ParseCase {
                label: CaseLabel("qualified outer reference"),
                source: ParseSource("def rec f() { (outer.f)() }"),
            },
        ];
        for case in cases {
            assert_parses_clean(&pbg, case)?;
        }
        Ok(())
    }

    fn assert_parses_clean(
        pbg: &Pbg,
        case: ParseCase,
    ) -> Result<(), Box<dyn Error>>
    {
        let result = parse(pbg, SourceSlice::from(case.source.0))?;
        assert!(
            bool::from(result.is_clean()),
            "{} must parse cleanly; obligations: {:?}",
            case.label.0,
            result
                .obligations()
                .iter()
                .map(|obligation| (obligation.class, obligation.span))
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn mixed_set_type_operators_require_parentheses() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let cases = [
            ParseCase {
                label: CaseLabel("union before intersection"),
                source: ParseSource("def f : A | B /\\ C;"),
            },
            ParseCase {
                label: CaseLabel("intersection before union"),
                source: ParseSource("def f : A /\\ B | C;"),
            },
            ParseCase {
                label: CaseLabel("union before lazy product"),
                source: ParseSource("def f : A | B & C;"),
            },
            ParseCase {
                label: CaseLabel("lazy product before union"),
                source: ParseSource("def f : A & B | C;"),
            },
            ParseCase {
                label: CaseLabel("intersection before lazy product"),
                source: ParseSource("def f : A /\\ B & C;"),
            },
            ParseCase {
                label: CaseLabel("lazy product before intersection"),
                source: ParseSource("def f : A & B /\\ C;"),
            },
        ];
        for case in cases {
            assert_parses_with_obligations(&pbg, case)?;
        }
        Ok(())
    }

    fn assert_parses_with_obligations(
        pbg: &Pbg,
        case: ParseCase,
    ) -> Result<(), Box<dyn Error>>
    {
        let result = parse(pbg, SourceSlice::from(case.source.0))?;
        assert!(
            !result.obligations().is_empty(),
            "{} must require a parser obligation",
            case.label.0
        );
        assert!(
            !bool::from(result.is_clean()),
            "{} must not parse cleanly",
            case.label.0
        );
        Ok(())
    }

    #[test]
    fn cyclic_named_precedence_spec_reports_closed_named_witness() -> Result<(), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let a = spec.insert("cycle.a", None)?;
        let b = spec.insert("cycle.b", Some(Assoc::Left))?;
        let c = spec.insert("cycle.c", Some(Assoc::Right))?;
        spec.add_edge(a, b)?;
        spec.add_edge(b, c)?;
        spec.add_edge(c, a)?;

        let cycle = PrecDag::build(&spec).expect_err("cyclic precedence spec must be rejected");
        let named = cycle
            .witness
            .iter()
            .map(|prec_id| match *prec_id {
                | prec if prec == a => PrecName("cycle.a"),
                | prec if prec == b => PrecName("cycle.b"),
                | prec if prec == c => PrecName("cycle.c"),
                | _ => panic!("unexpected precedence id in cycle witness: {prec_id:?}"),
            })
            .collect::<Vec<_>>();
        let error = PbgError::precedence_cycle(named.clone());

        let PbgError::PrecedenceCycle { witness } = error
        else {
            panic!("cycle conversion must use PbgError::PrecedenceCycle");
        };
        let expected_witness = named.iter().map(|name| name.0).collect::<Vec<_>>();
        assert_eq!(
            witness, expected_witness,
            "named witness must be preserved exactly"
        );
        assert!(witness.len() >= 2, "cycle witness must not be empty");
        assert_eq!(
            witness.first(),
            witness.last(),
            "cycle witness must be closed"
        );

        let named_edges = spec
            .edges()
            .map(|(from, to)| {
                let from_name = spec.name(from).expect("known source precedence");
                let to_name = spec.name(to).expect("known target precedence");
                (<&str>::from(from_name), <&str>::from(to_name))
            })
            .collect::<BTreeSet<_>>();
        for edge in witness.windows(2) {
            let [from, to] = edge else {
                continue;
            };
            assert!(
                named_edges.contains(&(*from, *to)),
                "closed named witness edge {edge:?} must be present in the cyclic spec"
            );
        }
        Ok(())
    }

    #[test]
    fn built_in_adaptations_name_concrete_rules_without_relaxing_checks()
    -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let rule_names = pbg.rule_names();
        if pbg.adaptations().is_empty() {
            assert!(
                pbg.adaptations().is_empty(),
                "built-in surface currently carries no adaptations"
            );
        }
        else {
            for adaptation in pbg.adaptations() {
                assert!(
                    rule_names.contains(adaptation.rule),
                    "adaptation rule `{}` must name a checked source rule",
                    adaptation.rule
                );
                assert!(
                    !adaptation.surface.is_empty(),
                    "adaptation surface must be concrete"
                );
                assert!(
                    !adaptation.reason.is_empty(),
                    "adaptation reason must be auditable"
                );
            }
            let rebuilt = built_in()?;
            assert_eq!(
                rebuilt.rule_names(),
                rule_names,
                "adaptations must not relax checked grammar construction"
            );
        }
        Ok(())
    }
}
