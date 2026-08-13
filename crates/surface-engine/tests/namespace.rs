//! Name-modifier scope-engine acceptance tests.
//!
//! Drives the engine through the design record's worked cases — selective
//! import, qualified import as a renaming of the root, the deep patch that
//! merges instead of capturing, re-export control, and typo resistance — plus
//! the visible/export split under a section and all three elaboration-time
//! events under both a permissive and a rejecting policy.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Name-modifier scope-engine acceptance tests.
#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_surface_engine::lower::ImportIndex;
    use gandr_surface_engine::lower::lower_source;
    use gandr_surface_engine::namespace::Binding;
    use gandr_surface_engine::namespace::Collision;
    use gandr_surface_engine::namespace::DottedName;
    use gandr_surface_engine::namespace::EventKind;
    use gandr_surface_engine::namespace::EventRejection;
    use gandr_surface_engine::namespace::Modifier;
    use gandr_surface_engine::namespace::NamePath;
    use gandr_surface_engine::namespace::NamespaceEvent;
    use gandr_surface_engine::namespace::NamespaceEventHandler;
    use gandr_surface_engine::namespace::PermissiveHandler;
    use gandr_surface_engine::namespace::RejectionReason;
    use gandr_surface_engine::namespace::Scope;
    use gandr_surface_engine::namespace::ScopeError;
    use gandr_surface_engine::namespace::Segment;
    use gandr_surface_engine::namespace::Trie;

    /// The payload a test binds — a marker standing in for whatever an
    /// elaborator resolves a path to.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct Payload(u32);

    /// The hook vocabulary these tests give meaning to.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum HookLabel
    {
        /// Replace the namespace the hook reached with the empty one.
        DropEverything,
        /// Leave the namespace the hook reached alone.
        KeepEverything,
    }

    /// One entry of a test namespace: a dotted path and the payload it binds.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Entry<'text>
    {
        /// The dotted rendering of the bound path.
        path: DottedName<'text>,
        /// The payload bound there.
        payload: Payload,
    }

    /// A policy that refuses every event it is asked about.
    ///
    /// The counterpart to [`PermissiveHandler`]: the engine core is identical
    /// under both, which is what makes the seam a seam.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct RejectingHandler;

    impl NamespaceEventHandler<Payload, ()> for RejectingHandler
    {
        type Label = HookLabel;

        fn not_found(
            &mut self,
            path: &NamePath,
        ) -> Result<(), EventRejection>
        {
            Err(EventRejection::new(
                EventKind::NotFound,
                path.clone(),
                RejectionReason::from("nothing matched here; check for a typo"),
            ))
        }

        fn shadow(
            &mut self,
            path: &NamePath,
            _collision: Collision<Payload, ()>,
        ) -> Result<Binding<Payload, ()>, EventRejection>
        {
            Err(EventRejection::new(
                EventKind::Shadow,
                path.clone(),
                RejectionReason::from("this policy forbids shadowing"),
            ))
        }

        fn hook(
            &mut self,
            path: &NamePath,
            _label: &HookLabel,
            _subject: Trie<Payload, ()>,
        ) -> Result<Trie<Payload, ()>, EventRejection>
        {
            Err(EventRejection::new(
                EventKind::Hook,
                path.clone(),
                RejectionReason::from("this policy recognizes no hooks"),
            ))
        }
    }

    /// A permissive policy whose hooks actually do something.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct RewritingHandler;

    impl NamespaceEventHandler<Payload, ()> for RewritingHandler
    {
        type Label = HookLabel;

        fn not_found(
            &mut self,
            _path: &NamePath,
        ) -> Result<(), EventRejection>
        {
            Ok(())
        }

        fn shadow(
            &mut self,
            _path: &NamePath,
            collision: Collision<Payload, ()>,
        ) -> Result<Binding<Payload, ()>, EventRejection>
        {
            Ok(collision.latter)
        }

        fn hook(
            &mut self,
            _path: &NamePath,
            label: &HookLabel,
            subject: Trie<Payload, ()>,
        ) -> Result<Trie<Payload, ()>, EventRejection>
        {
            match *label {
                | HookLabel::DropEverything => Ok(Trie::empty()),
                | HookLabel::KeepEverything => Ok(subject),
            }
        }
    }

    /// Builds a path from its dotted rendering.
    fn path<'text>(text: impl Into<DottedName<'text>>) -> NamePath
    {
        NamePath::from(text.into())
    }

    /// Names one test-namespace entry.
    fn entry<'text>(
        text: impl Into<DottedName<'text>>,
        payload: Payload,
    ) -> Entry<'text>
    {
        Entry {
            path: text.into(),
            payload,
        }
    }

    /// Builds a namespace from dotted paths paired with payloads.
    fn namespace(entries: &[Entry<'_>]) -> Trie<Payload, ()>
    {
        entries
            .iter()
            .map(|item| (path(item.path), Binding::new(item.payload, ())))
            .collect()
    }

    /// The namespace's dotted paths with their payloads, in ascending order.
    fn listing(subject: &Trie<Payload, ()>) -> Vec<(String, Payload)>
    {
        subject
            .iter()
            .map(|(path, binding)| (alloc::format!("{path}"), binding.data))
            .collect()
    }

    /// The expected listing, spelled as dotted paths and payloads.
    fn expected(entries: &[Entry<'_>]) -> Vec<(String, Payload)>
    {
        entries
            .iter()
            .map(|item| (String::from(item.path.as_ref()), item.payload))
            .collect()
    }

    /// Runs a modifier under the warn-and-allow policy, returning the result
    /// and the events it recorded.
    fn permissive(
        modifier: &Modifier<HookLabel>,
        subject: Trie<Payload, ()>,
    ) -> (Trie<Payload, ()>, Vec<NamespaceEvent<HookLabel>>)
    {
        let mut handler = PermissiveHandler::<HookLabel>::new();
        let outcome = modifier
            .apply(subject, &mut handler)
            .expect("the permissive policy rejects nothing");
        (outcome, handler.events().to_vec())
    }

    /// The `arith` namespace the design record's import examples use.
    fn arith() -> Trie<Payload, ()>
    {
        namespace(&[
            entry("nat.plus.assoc", Payload(1)),
            entry("nat.times.assoc", Payload(2)),
            entry("int.plus.assoc", Payload(3)),
        ])
    }

    // --- The modifier language's worked cases ------------------------------

    #[test]
    fn only_keeps_the_named_subtree_and_drops_the_rest()
    {
        let (outcome, events) = permissive(&Modifier::only(path("nat")), arith());
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("nat.plus.assoc", Payload(1)),
                entry("nat.times.assoc", Payload(2)),
            ]),
            "selective import keeps the whole subtree and nothing else"
        );
        assert!(
            events.is_empty(),
            "a selection that matched something performs no event"
        );
    }

    #[test]
    fn except_drops_the_named_subtree()
    {
        let (outcome, events) = permissive(&Modifier::except(path("nat")), arith());
        assert_eq!(
            listing(&outcome),
            expected(&[entry("int.plus.assoc", Payload(3))]),
            "exclusion is subtree-grained, exactly like selection"
        );
        assert!(
            events.is_empty(),
            "an exclusion that matched something performs no event"
        );
    }

    #[test]
    fn in_runs_the_inner_modifier_on_one_subtree()
    {
        let modifier = Modifier::in_subtree(path("nat"), Modifier::only(path("plus")));
        let (outcome, events) = permissive(&modifier, arith());
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("int.plus.assoc", Payload(3)),
                entry("nat.plus.assoc", Payload(1)),
            ]),
            "`in nat only plus` keeps nat.plus.assoc, drops nat.times.assoc, and leaves int alone"
        );
        assert!(events.is_empty(), "nothing was missing");
    }

    #[test]
    fn qualified_import_renames_the_root_to_the_alias()
    {
        let modifier = Modifier::renaming(NamePath::root(), path("arith"));
        let (outcome, _) = permissive(&modifier, arith());
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("arith.int.plus.assoc", Payload(3)),
                entry("arith.nat.plus.assoc", Payload(1)),
                entry("arith.nat.times.assoc", Payload(2)),
            ]),
            "qualified import is a renaming of the root, so every path gains the alias"
        );
    }

    #[test]
    fn renaming_to_the_root_unqualifies()
    {
        let modifier = Modifier::renaming(path("nat"), NamePath::root());
        let (outcome, _) = permissive(&modifier, arith());
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("plus.assoc", Payload(1)),
                entry("times.assoc", Payload(2)),
            ]),
            "renaming to the root hoists the subtree and drops everything else"
        );
    }

    #[test]
    fn the_checked_renaming_builder_performs_not_found_on_an_absent_source()
    {
        let (outcome, events) =
            permissive(&Modifier::renaming(path("nta"), path("natural")), arith());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound { path: path("nta") }]),
            "the public builder carries the emptiness check, so a mistyped source is reported at \
             the path the user wrote"
        );
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "and relocating an empty subtree moves nothing"
        );
    }

    #[test]
    fn the_core_relocation_performs_no_emptiness_check()
    {
        let (outcome, events) = permissive(
            &Modifier::Renaming {
                source: path("nta"),
                target: path("natural"),
            },
            arith(),
        );
        assert!(
            events.is_empty(),
            "the core constructor is the unchecked relocation — the check lives in the builder \
             that wraps it, which is why the two are separate items"
        );
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "and it moves nothing either, silently"
        );
    }

    #[test]
    fn a_rejecting_handler_refuses_a_missing_renaming_source()
    {
        let mut handler = RejectingHandler;
        let rejection = Modifier::renaming(path("nta"), path("natural"))
            .apply(arith(), &mut handler)
            .expect_err("this policy treats a typo as fatal");
        assert_eq!(
            rejection.kind(),
            EventKind::NotFound,
            "typo resistance reaches renaming, not only selection"
        );
        assert_eq!(
            *rejection.path(),
            path("nta"),
            "and names the source the user wrote"
        );
    }

    #[test]
    fn renaming_drops_whatever_was_at_the_target()
    {
        let subject = namespace(&[
            entry("patch.is_prefix", Payload(1)),
            entry("string.length", Payload(2)),
        ]);
        let modifier = Modifier::renaming(path("patch"), path("string"));
        let (outcome, events) = permissive(&modifier, subject);
        assert_eq!(
            listing(&outcome),
            expected(&[entry("string.is_prefix", Payload(1))]),
            "the target subtree is discarded rather than merged — the design record's deliberate choice"
        );
        assert!(
            events.is_empty(),
            "dropping the target is silent; the safety net is a later scope-check failure"
        );
    }

    #[test]
    fn a_deep_patch_merges_instead_of_capturing()
    {
        let subject = namespace(&[entry("a.x", Payload(1)), entry("b.a.y", Payload(2))]);
        let modifier = Modifier::union(Vec::from([
            Modifier::all(),
            Modifier::renaming(path("b"), NamePath::root()),
        ]));
        let (outcome, events) = permissive(&modifier, subject);
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("a.x", Payload(1)),
                entry("a.y", Payload(2)),
                entry("b.a.y", Payload(2)),
            ]),
            "the design record's worked example: `a.y` joins the implicit `a` namespace instead of \
             shadowing `a.x`"
        );
        assert!(
            events.is_empty(),
            "no two bindings landed on one path, so nothing was shadowed"
        );
    }

    #[test]
    fn each_union_branch_runs_on_the_original_namespace()
    {
        let modifier = Modifier::union(Vec::from([
            Modifier::only(path("nat")),
            Modifier::only(path("int")),
        ]));
        let (outcome, events) = permissive(&modifier, arith());
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "both branches see the whole input, so selecting each half and unioning restores it"
        );
        assert!(
            events.is_empty(),
            "neither selection ran on the other's result, so neither matched nothing"
        );
    }

    #[test]
    fn the_empty_sequence_is_the_identity()
    {
        let (outcome, events) = permissive(&Modifier::id(), arith());
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "`seq ()` changes nothing"
        );
        assert!(events.is_empty(), "and checks nothing");
    }

    #[test]
    fn the_empty_union_is_the_empty_namespace()
    {
        let (outcome, events) = permissive(&Modifier::union(Vec::new()), arith());
        assert_eq!(
            listing(&outcome),
            expected(&[]),
            "`union ()` drops everything without checking anything"
        );
        assert!(events.is_empty(), "the empty union performs no event");
    }

    #[test]
    fn except_on_an_absent_subtree_performs_not_found()
    {
        let (outcome, events) = permissive(&Modifier::except(path("nta")), arith());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound { path: path("nta") }]),
            "`except p` derives as `in p none`, and `none` checks emptiness before it drops — so \
             excluding a subtree that is not there is the same typo signal selecting one gives, \
             at the path the user wrote"
        );
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "and excluding an empty subtree excludes nothing"
        );
    }

    #[test]
    fn the_identity_checks_nothing_on_an_empty_namespace()
    {
        let (outcome, events) = permissive(&Modifier::id(), Trie::empty());
        assert!(
            events.is_empty(),
            "`id` is `seq ()`, which carries no emptiness check — the whole difference from \
             `all`, and what makes `id` the identity of `seq` rather than a checked no-op"
        );
        assert_eq!(listing(&outcome), expected(&[]), "and it changes nothing");
    }

    #[test]
    fn none_drops_everything_and_flags_an_empty_input()
    {
        let (outcome, events) = permissive(&Modifier::none(), arith());
        assert_eq!(listing(&outcome), expected(&[]), "`none` drops everything");
        assert!(
            events.is_empty(),
            "the input was not empty, so no not-found event was performed"
        );

        let (outcome, events) = permissive(&Modifier::none(), Trie::empty());
        assert_eq!(listing(&outcome), expected(&[]), "still nothing");
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound {
                path: NamePath::root(),
            }]),
            "`none` on an already-empty namespace performs not-found at the root"
        );
    }

    #[test]
    fn all_performs_not_found_on_an_empty_namespace()
    {
        let (_, events) = permissive(&Modifier::all(), Trie::empty());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound {
                path: NamePath::root(),
            }]),
            "`all` is the emptiness check, so an empty namespace is the signal"
        );
    }

    #[test]
    fn a_selection_that_matched_nothing_performs_not_found()
    {
        let (outcome, events) = permissive(&Modifier::only(path("nta")), arith());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound { path: path("nta") }]),
            "typo resistance: the misspelling is reported at the path the user wrote, once"
        );
        assert_eq!(
            listing(&outcome),
            expected(&[]),
            "and the selection still yields nothing, so the mistake cannot pass unnoticed"
        );
    }

    #[test]
    fn a_nested_event_reports_the_accumulated_prefix()
    {
        let modifier = Modifier::in_subtree(path("nat"), Modifier::only(path("mnus")));
        let (_, events) = permissive(&modifier, arith());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound {
                path: path("nat.mnus"),
            }]),
            "an event inside `in nat` names the full path, not the subtree-relative one"
        );
    }

    #[test]
    fn a_nested_shadow_reports_the_accumulated_prefix()
    {
        let subject = namespace(&[entry("nat.a.x", Payload(1)), entry("nat.x", Payload(2))]);
        let modifier = Modifier::in_subtree(
            path("nat"),
            Modifier::union(Vec::from([
                Modifier::id(),
                Modifier::renaming(path("a"), NamePath::root()),
            ])),
        );
        let (outcome, events) = permissive(&modifier, subject);
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::Shadow {
                path: path("nat.x"),
            }]),
            "a collision inside `in nat` names the whole path, not the subtree-relative one"
        );
        assert_eq!(
            listing(&outcome),
            expected(&[entry("nat.a.x", Payload(1)), entry("nat.x", Payload(1))]),
            "and the union still merges inside the subtree it ran on"
        );
    }

    #[test]
    fn a_nested_hook_reports_the_accumulated_prefix()
    {
        let modifier = Modifier::in_subtree(path("nat"), Modifier::hook(HookLabel::KeepEverything));
        let (_, events) = permissive(&modifier, arith());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::Hook {
                path: path("nat"),
                label: HookLabel::KeepEverything,
            }]),
            "a hook inside `in nat` is told the prefix it ran under"
        );
    }

    #[test]
    fn a_hook_can_replace_the_namespace()
    {
        let mut handler = RewritingHandler;
        let outcome = Modifier::hook(HookLabel::DropEverything)
            .apply(arith(), &mut handler)
            .expect("this policy accepts the label");
        assert_eq!(
            listing(&outcome),
            expected(&[]),
            "the handler's return value is the modifier's result"
        );

        let outcome = Modifier::hook(HookLabel::KeepEverything)
            .apply(arith(), &mut handler)
            .expect("this policy accepts the label");
        assert_eq!(
            listing(&outcome),
            listing(&arith()),
            "and a different label means something different, with no engine change"
        );
    }

    // --- The `as name` clause, re-read -------------------------------------

    #[test]
    fn as_name_is_renaming_to_the_alias()
    {
        assert_eq!(
            Modifier::<HookLabel>::alias_as(Segment::from("parse")),
            Modifier::renaming(NamePath::root(), path("parse")),
            "`as parse` is `renaming . parse`, not a special form"
        );
    }

    #[test]
    fn as_name_qualifies_every_imported_path()
    {
        let imported = namespace(&[
            entry("lexer.token", Payload(1)),
            entry("parser", Payload(2)),
        ]);
        let (outcome, _) = permissive(&Modifier::alias_as(Segment::from("parse")), imported);
        assert_eq!(
            listing(&outcome),
            expected(&[
                entry("parse.lexer.token", Payload(1)),
                entry("parse.parser", Payload(2)),
            ]),
            "today's import clause is the one-constructor case of the general language"
        );
    }

    #[test]
    fn source_import_reaches_the_namespace_engine_and_exposes_its_alias()
    {
        let lowered = lower_source(
            "import \"file:///lib/parse.gandr\" as parse ;\n\
             import \"file:///lib/list.gandr\" as list_ext ;"
                .into(),
        )
        .expect("import declarations must lower");

        assert!(lowered.items.is_empty(), "an import is not a runnable item");
        assert_eq!(
            lowered
                .imports
                .iter()
                .map(|import| (import.uri.as_str(), import.alias.as_ref()))
                .collect::<Vec<(&str, &str)>>(),
            Vec::from([
                ("file:///lib/parse.gandr", "parse"),
                ("file:///lib/list.gandr", "list_ext"),
            ]),
            "imports retain source order and surface operands"
        );

        let parse = lowered
            .import_scope()
            .resolve(&path("parse"))
            .expect("the first import must resolve in the visible namespace");
        let list = lowered
            .import_scope()
            .resolve(&path("list_ext"))
            .expect("the second import must resolve in the visible namespace");
        assert_eq!(
            (parse.data, list.data),
            (ImportIndex(0), ImportIndex(1)),
            "namespace bindings point to their source-order declarations"
        );
        assert_eq!(
            lowered.import_scope().export(),
            &Trie::empty(),
            "imports do not re-export their bindings"
        );
    }

    #[test]
    fn as_name_on_an_empty_import_performs_not_found()
    {
        let (outcome, events) =
            permissive(&Modifier::alias_as(Segment::from("parse")), Trie::empty());
        assert_eq!(
            events,
            Vec::from([NamespaceEvent::NotFound {
                path: NamePath::root(),
            }]),
            "the desugaring inherits the checked builder's typo resistance: aliasing an empty \
             import is reported at the root"
        );
        assert_eq!(
            listing(&outcome),
            expected(&[]),
            "and qualifying nothing yields nothing"
        );
    }

    // --- The visible/export split ------------------------------------------

    #[test]
    fn include_touches_both_namespaces()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides in an empty scope");
        assert_eq!(
            listing(scope.visible()),
            listing(&arith()),
            "an include is usable here"
        );
        assert_eq!(
            listing(scope.export()),
            listing(&arith()),
            "and visible to importers"
        );
    }

    #[test]
    fn import_touches_only_the_visible_namespace()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .import_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides in an empty scope");
        assert_eq!(
            listing(scope.visible()),
            listing(&arith()),
            "an import is usable here"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[]),
            "and is not a re-export"
        );
    }

    #[test]
    fn an_import_arrives_under_its_prefix()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .import_subtree(
                &path("lib"),
                namespace(&[entry("parse.token", Payload(1))]),
                &mut handler,
            )
            .expect("nothing collides in an empty scope");
        assert_eq!(
            listing(scope.visible()),
            expected(&[entry("lib.parse.token", Payload(1))]),
            "an import is merged under the prefix it was handed, not at the root — every other \
             caller of this operation happens to pass the root"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[]),
            "and prefixing it still does not make it a re-export"
        );
    }

    #[test]
    fn a_refused_modifier_leaves_the_visible_namespace_as_it_was()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut permissive_handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut permissive_handler)
            .expect("nothing collides in an empty scope");
        let mut handler = RejectingHandler;
        let failure = scope
            .modify_visible(&Modifier::only(path("nta")), &mut handler)
            .expect_err("this policy treats a typo as fatal");
        assert_eq!(
            failure,
            ScopeError::Rejected(EventRejection::new(
                EventKind::NotFound,
                path("nta"),
                RejectionReason::from("nothing matched here; check for a typo"),
            )),
            "the policy's refusal reaches the caller whole, as the structured scope failure"
        );
        assert_eq!(
            listing(scope.visible()),
            listing(&arith()),
            "and the namespace the modifier was rewriting is exactly as it was: the rewrite lands \
             only once the modifier has succeeded, so a caller can report the rejection and keep \
             elaborating"
        );
        assert_eq!(
            listing(scope.export()),
            listing(&arith()),
            "with the namespace it never touches untouched throughout"
        );
    }

    #[test]
    fn a_refused_modifier_leaves_the_export_namespace_as_it_was()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut permissive_handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut permissive_handler)
            .expect("nothing collides in an empty scope");
        let mut handler = RejectingHandler;
        let failure = scope
            .modify_export(&Modifier::only(path("nta")), &mut handler)
            .expect_err("this policy treats a typo as fatal");
        assert_eq!(
            failure,
            ScopeError::Rejected(EventRejection::new(
                EventKind::NotFound,
                path("nta"),
                RejectionReason::from("nothing matched here; check for a typo"),
            )),
            "the same refusal, on the other namespace"
        );
        assert_eq!(
            listing(scope.export()),
            listing(&arith()),
            "and the export namespace is exactly as it was"
        );
        assert_eq!(
            listing(scope.visible()),
            listing(&arith()),
            "with the visible namespace untouched throughout"
        );
    }

    #[test]
    fn a_refused_re_export_leaves_the_prior_export_as_it_was()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut permissive_handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("already.exported", Payload(4))]),
                &mut permissive_handler,
            )
            .expect("nothing collides in an empty scope");
        let mut handler = RejectingHandler;
        let failure = scope
            .export_visible(&Modifier::only(path("nta")), &mut handler)
            .expect_err("this policy treats a typo as fatal");
        assert_eq!(
            failure,
            ScopeError::Rejected(EventRejection::new(
                EventKind::NotFound,
                path("nta"),
                RejectionReason::from("nothing matched here; check for a typo"),
            )),
            "the selection is refused before anything is passed on"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[entry("already.exported", Payload(4))]),
            "a refused selection re-exports nothing and takes nothing away: what the unit had \
             already exported survives"
        );
    }

    #[test]
    fn modifying_visible_leaves_export_alone()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides in an empty scope");
        scope
            .modify_visible(&Modifier::only(path("nat")), &mut handler)
            .expect("the subtree exists");
        assert_eq!(
            listing(scope.visible()),
            expected(&[
                entry("nat.plus.assoc", Payload(1)),
                entry("nat.times.assoc", Payload(2)),
            ]),
            "the visible namespace was narrowed"
        );
        assert_eq!(
            listing(scope.export()),
            listing(&arith()),
            "the export namespace was not"
        );
    }

    #[test]
    fn modifying_export_leaves_visible_alone()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides in an empty scope");
        scope
            .import_subtree(
                &NamePath::root(),
                namespace(&[entry("borrowed", Payload(5))]),
                &mut handler,
            )
            .expect("nothing collides with the included namespace");
        scope
            .modify_export(&Modifier::except(path("nat")), &mut handler)
            .expect("the subtree exists");
        assert_eq!(
            listing(scope.visible()),
            expected(&[
                entry("borrowed", Payload(5)),
                entry("int.plus.assoc", Payload(3)),
                entry("nat.plus.assoc", Payload(1)),
                entry("nat.times.assoc", Payload(2)),
            ]),
            "the visible namespace was not narrowed"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[entry("int.plus.assoc", Payload(3))]),
            "the export namespace was, and it was the export namespace the modifier read — the \
             imported binding never reached it"
        );
    }

    #[test]
    fn export_visible_re_exports_a_selection()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("already.exported", Payload(4))]),
                &mut handler,
            )
            .expect("nothing collides in an empty scope");
        scope
            .import_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides with the included namespace");
        scope
            .export_visible(&Modifier::only(path("nat")), &mut handler)
            .expect("the subtree exists");
        assert_eq!(
            listing(scope.visible()),
            expected(&[
                entry("already.exported", Payload(4)),
                entry("int.plus.assoc", Payload(3)),
                entry("nat.plus.assoc", Payload(1)),
                entry("nat.times.assoc", Payload(2)),
            ]),
            "re-exporting does not narrow what resolves here"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[
                entry("already.exported", Payload(4)),
                entry("nat.plus.assoc", Payload(1)),
                entry("nat.times.assoc", Payload(2)),
            ]),
            "a unit chooses which of the things it can see it passes on, and the choice is merged \
             into what it already exported rather than replacing it"
        );
    }

    #[test]
    fn include_merges_the_visible_namespace_before_the_export()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut permissive_handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("m.b", Payload(1))]),
                &mut permissive_handler,
            )
            .expect("nothing collides in an empty scope");
        let mut handler = RejectingHandler;
        let outcome = scope.include_subtree(
            &NamePath::root(),
            namespace(&[entry("m.a", Payload(2)), entry("m.b", Payload(3))]),
            &mut handler,
        );
        let ScopeError::Rejected(rejection) =
            outcome.expect_err("the collision at `m.b` is refused")
        else {
            panic!("the failure is an event rejection, not a structural one");
        };
        assert_eq!(
            (rejection.kind(), rejection.path().clone()),
            (EventKind::Shadow, path("m.b")),
            "and it is the collision, at the colliding path"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[entry("m.b", Payload(1))]),
            "the visible merge runs first, so a rejection there leaves the export namespace \
             untouched"
        );
        assert_eq!(
            listing(scope.visible()),
            expected(&[entry("m.a", Payload(2)), entry("m.b", Payload(1))]),
            "and the visible namespace keeps the bindings merged before the refused one — an \
             include is not atomic"
        );
    }

    #[test]
    fn a_section_inherits_the_visible_namespace_and_exports_nothing_yet()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides in an empty scope");
        scope.begin_section();
        assert_eq!(
            listing(scope.visible()),
            listing(&arith()),
            "the section can see what surrounds it"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[]),
            "and starts with nothing of its own to export, however much the parent exported"
        );
    }

    #[test]
    fn a_sections_closing_modifier_chooses_what_it_passes_on()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope.begin_section();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[
                    entry("public", Payload(1)),
                    entry("scaffolding", Payload(2)),
                ]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        scope
            .end_section(
                &path("group"),
                &Modifier::only(path("public")),
                &mut handler,
            )
            .expect("a section is open and the selection matched");
        assert_eq!(
            listing(scope.export()),
            expected(&[entry("group.public", Payload(1))]),
            "the closing modifier runs over the section's export before the prefixed include"
        );
        assert_eq!(
            listing(scope.visible()),
            expected(&[entry("group.public", Payload(1))]),
            "and the same selection is what the parent can see"
        );
    }

    #[test]
    fn a_section_exports_under_its_prefix()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope.begin_section();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("lemma", Payload(1))]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        scope
            .end_section(&path("group"), &Modifier::id(), &mut handler)
            .expect("a section is open");
        assert_eq!(
            listing(scope.visible()),
            expected(&[entry("group.lemma", Payload(1))]),
            "the section's export arrives under the section's prefix"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[entry("group.lemma", Payload(1))]),
            "and is itself re-exported, because closing a section is an include"
        );
    }

    #[test]
    fn a_sections_imports_evaporate_at_close()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope.begin_section();
        scope
            .import_subtree(&NamePath::root(), arith(), &mut handler)
            .expect("nothing collides inside a fresh section");
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("lemma", Payload(1))]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        assert_eq!(
            listing(scope.visible()).len(),
            4,
            "inside the section the import resolves alongside the definition"
        );
        scope
            .end_section(&path("group"), &Modifier::id(), &mut handler)
            .expect("a section is open");
        assert_eq!(
            listing(scope.visible()),
            expected(&[entry("group.lemma", Payload(1))]),
            "only what the section exported survives; its imports evaporate"
        );
    }

    #[test]
    fn closing_without_an_open_section_fails()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        let outcome = scope.end_section(&path("group"), &Modifier::id(), &mut handler);
        assert_eq!(
            outcome,
            Err(ScopeError::NoOpenSection),
            "closing a section that was never opened is a structured failure, not a panic"
        );
    }

    #[test]
    fn closing_a_section_restores_what_the_parent_already_held()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("outer_lemma", Payload(1))]),
                &mut handler,
            )
            .expect("nothing collides in an empty scope");
        scope.begin_section();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("inner_lemma", Payload(2))]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        scope
            .end_section(&path("group"), &Modifier::id(), &mut handler)
            .expect("a section is open");
        assert_eq!(
            listing(scope.visible()),
            expected(&[
                entry("group.inner_lemma", Payload(2)),
                entry("outer_lemma", Payload(1)),
            ]),
            "everything the enclosing scope held before the section is still there afterwards, \
             beside what the section exported"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[
                entry("group.inner_lemma", Payload(2)),
                entry("outer_lemma", Payload(1)),
            ]),
            "in both namespaces, because closing restores the enclosing scope rather than \
             replacing it"
        );
    }

    #[test]
    fn nested_sections_close_innermost_first()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope.begin_section();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("outer_lemma", Payload(1))]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        scope.begin_section();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("inner_lemma", Payload(2))]),
                &mut handler,
            )
            .expect("nothing collides inside a fresh section");
        scope
            .end_section(&path("inner"), &Modifier::id(), &mut handler)
            .expect("the inner section is open");
        assert_eq!(
            listing(scope.export()),
            expected(&[
                entry("inner.inner_lemma", Payload(2)),
                entry("outer_lemma", Payload(1)),
            ]),
            "closing the inner section returns into the *enclosing section*, not into the \
             outermost scope — the enclosing scopes are a stack"
        );
        scope
            .end_section(&path("outer"), &Modifier::id(), &mut handler)
            .expect("the outer section is open");
        assert_eq!(
            listing(scope.export()),
            expected(&[
                entry("outer.inner.inner_lemma", Payload(2)),
                entry("outer.outer_lemma", Payload(1)),
            ]),
            "and closing the outer one prefixes both again, so section prefixes nest"
        );
    }

    // --- The handler seam ---------------------------------------------------

    #[test]
    fn the_permissive_handler_records_all_three_events()
    {
        let subject = namespace(&[entry("a.x", Payload(1)), entry("x", Payload(2))]);
        let modifier = Modifier::seq(Vec::from([
            Modifier::in_subtree(path("gone"), Modifier::all()),
            Modifier::union(Vec::from([
                Modifier::id(),
                Modifier::renaming(path("a"), NamePath::root()),
            ])),
            Modifier::hook(HookLabel::KeepEverything),
        ]));
        let (outcome, events) = permissive(&modifier, subject);
        assert_eq!(
            events,
            Vec::from([
                NamespaceEvent::NotFound { path: path("gone") },
                NamespaceEvent::Shadow { path: path("x") },
                NamespaceEvent::Hook {
                    path: NamePath::root(),
                    label: HookLabel::KeepEverything,
                },
            ]),
            "all three events reach the handler, in the order the run performs them"
        );
        assert_eq!(
            listing(&outcome),
            expected(&[entry("a.x", Payload(1)), entry("x", Payload(1))]),
            "and the warn-and-allow policy lets the run finish, later shadowing earlier"
        );
    }

    #[test]
    fn a_rejecting_handler_refuses_a_missing_selection()
    {
        let mut handler = RejectingHandler;
        let outcome = Modifier::only(path("nta")).apply(arith(), &mut handler);
        let rejection = outcome.expect_err("this policy treats a typo as fatal");
        assert_eq!(
            rejection.kind(),
            EventKind::NotFound,
            "the rejection names the event it refused"
        );
        assert_eq!(
            *rejection.path(),
            path("nta"),
            "and the path the user actually wrote"
        );
    }

    #[test]
    fn a_rejecting_handler_refuses_a_shadow()
    {
        let subject = namespace(&[entry("a.x", Payload(1)), entry("x", Payload(2))]);
        let modifier = Modifier::union(Vec::from([
            Modifier::id(),
            Modifier::renaming(path("a"), NamePath::root()),
        ]));
        let mut handler = RejectingHandler;
        let rejection = modifier
            .apply(subject, &mut handler)
            .expect_err("this policy forbids shadowing");
        assert_eq!(
            rejection.kind(),
            EventKind::Shadow,
            "the rejection names the event it refused"
        );
        assert_eq!(
            *rejection.path(),
            path("x"),
            "and the colliding path, as a whole path"
        );
    }

    #[test]
    fn a_rejecting_handler_refuses_a_hook()
    {
        let mut handler = RejectingHandler;
        let rejection = Modifier::hook(HookLabel::KeepEverything)
            .apply(arith(), &mut handler)
            .expect_err("this policy recognizes no hooks");
        assert_eq!(
            rejection.kind(),
            EventKind::Hook,
            "the rejection names the event it refused"
        );
        assert_eq!(
            *rejection.path(),
            NamePath::root(),
            "at the prefix the hook ran under"
        );
    }

    #[test]
    fn a_rejection_renders_its_event_kind_path_and_reason()
    {
        let mut handler = RejectingHandler;
        let missing = Modifier::only(path("nta"))
            .apply(arith(), &mut handler)
            .expect_err("this policy treats a typo as fatal");
        assert_eq!(
            alloc::format!("{missing}"),
            "the not-found event at `nta` was rejected: nothing matched here; check for a typo",
            "a rejection is the diagnostic a policy hands back to a consumer with no other \
             reporting layer, so it renders all three of its parts"
        );
        assert_eq!(
            missing.reason().as_ref(),
            "nothing matched here; check for a typo",
            "and the policy's own explanation is readable as text without going through the \
             rendering"
        );

        let colliding = Modifier::union(Vec::from([
            Modifier::id(),
            Modifier::renaming(path("a"), NamePath::root()),
        ]))
        .apply(
            namespace(&[entry("a.x", Payload(1)), entry("x", Payload(2))]),
            &mut handler,
        )
        .expect_err("this policy forbids shadowing");
        assert_eq!(
            alloc::format!("{colliding}"),
            "the shadow event at `x` was rejected: this policy forbids shadowing",
            "each of the three events renders under its own name"
        );

        let hooked = Modifier::hook(HookLabel::KeepEverything)
            .apply(arith(), &mut handler)
            .expect_err("this policy recognizes no hooks");
        assert_eq!(
            alloc::format!("{hooked}"),
            "the hook event at `.` was rejected: this policy recognizes no hooks",
            "and a rejection at the root names the root, as the design record's bare period"
        );
    }

    #[test]
    fn a_scope_failure_renders_its_message_or_its_rejection()
    {
        let mut scope: Scope<Payload, ()> = Scope::new();
        let mut handler = PermissiveHandler::<HookLabel>::new();
        let structural = scope
            .end_section(&path("group"), &Modifier::id(), &mut handler)
            .expect_err("no section was ever opened");
        assert_eq!(
            alloc::format!("{structural}"),
            "no open section to close",
            "the structural failure carries a message of its own"
        );

        let mut rejecting = RejectingHandler;
        let rejected = scope
            .modify_visible(&Modifier::all(), &mut rejecting)
            .expect_err("the fresh scope is empty and this policy calls that fatal");
        assert_eq!(
            alloc::format!("{rejected}"),
            "the not-found event at `.` was rejected: nothing matched here; check for a typo",
            "while a refused event renders transparently as the rejection itself, so wrapping it \
             adds no layer to what a user reads"
        );
    }

    #[test]
    fn clearing_a_permissive_handler_forgets_what_it_recorded()
    {
        let mut handler = PermissiveHandler::<HookLabel>::new();
        let first = Modifier::<HookLabel>::all()
            .apply(Trie::<Payload, ()>::empty(), &mut handler)
            .expect("the permissive policy rejects nothing");
        assert_eq!(
            listing(&first),
            expected(&[]),
            "the emptiness check transforms nothing"
        );
        assert_eq!(
            handler.events(),
            Vec::from([NamespaceEvent::NotFound {
                path: NamePath::root(),
            }])
            .as_slice(),
            "and it was recorded"
        );

        handler.clear();
        assert!(
            handler.events().is_empty(),
            "clearing forgets every recorded event"
        );

        let second = Modifier::<HookLabel>::all()
            .apply(Trie::<Payload, ()>::empty(), &mut handler)
            .expect("the permissive policy rejects nothing");
        assert_eq!(
            listing(&second),
            expected(&[]),
            "and the handler is still usable afterwards"
        );
        assert_eq!(
            handler.events(),
            Vec::from([NamespaceEvent::NotFound {
                path: NamePath::root(),
            }])
            .as_slice(),
            "recording only what happened after the clear"
        );
    }

    // --- Namespace graduation, prepared ------------------------------------

    /// The shape the prelude, host, and extern tables would take as an initial
    /// visible namespace. Built by hand: the real tables are deliberately not
    /// wired to this layer.
    fn prelude_shaped() -> Trie<Payload, ()>
    {
        namespace(&[
            entry("fs.read", Payload(10)),
            entry("env.get", Payload(11)),
            entry("prim.id", Payload(12)),
        ])
    }

    #[test]
    fn init_visible_seeds_only_the_visible_namespace()
    {
        let scope: Scope<Payload, ()> = Scope::with_init_visible(prelude_shaped());
        assert_eq!(
            listing(scope.visible()),
            listing(&prelude_shaped()),
            "the graduated prelude is what resolves here"
        );
        assert_eq!(
            listing(scope.export()),
            expected(&[]),
            "elaborating against a prelude does not re-export it"
        );
        assert_eq!(
            scope.resolve(&path("fs.read")).map(|binding| binding.data),
            Some(Payload(10)),
            "recognition becomes ordinary resolution, not a name-table check"
        );
    }

    #[test]
    fn a_user_binding_over_the_prelude_is_a_shadow_event()
    {
        let mut scope: Scope<Payload, ()> = Scope::with_init_visible(prelude_shaped());
        let mut handler = PermissiveHandler::<HookLabel>::new();
        scope
            .include_subtree(
                &NamePath::root(),
                namespace(&[entry("env.get", Payload(99))]),
                &mut handler,
            )
            .expect("the warn-and-allow policy permits the shadow");
        assert_eq!(
            handler.events(),
            Vec::from([NamespaceEvent::Shadow {
                path: path("env.get"),
            }])
            .as_slice(),
            "a script declaring its own `env.get` is a reportable event, not a prohibition"
        );
        assert_eq!(
            scope.resolve(&path("env.get")).map(|binding| binding.data),
            Some(Payload(99)),
            "and under this policy the user's declaration wins"
        );
    }

    #[test]
    fn a_rejecting_policy_forbids_shadowing_the_prelude()
    {
        let mut scope: Scope<Payload, ()> = Scope::with_init_visible(prelude_shaped());
        let mut handler = RejectingHandler;
        let outcome = scope.include_subtree(
            &NamePath::root(),
            namespace(&[entry("env.get", Payload(99))]),
            &mut handler,
        );
        let ScopeError::Rejected(rejection) =
            outcome.expect_err("this policy forbids shadowing the prelude")
        else {
            panic!("the failure is an event rejection, not a structural one");
        };
        assert_eq!(
            rejection.kind(),
            EventKind::Shadow,
            "the same collision, refused instead of allowed — the policy moved, the engine did not"
        );
    }

    /// Keeps the boxed-modifier import live for the crate-level lint wall.
    #[test]
    fn a_nested_modifier_survives_a_round_trip()
    {
        let inner: Box<Modifier<HookLabel>> = Box::new(Modifier::all());
        assert_eq!(
            Modifier::in_subtree(path("nat"), *inner),
            Modifier::In {
                path: path("nat"),
                inner: Box::new(Modifier::AssertNonEmpty),
            },
            "`in_subtree` is the boxing constructor of the `In` variant"
        );
    }
}
