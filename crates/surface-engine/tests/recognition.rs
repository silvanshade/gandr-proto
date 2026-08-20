//! Recognition as scoped resolution: the outermost scope, its shadow policy,
//! and the equivalence with the constant-table recognizer it replaced.
//!
//! The suite has three jobs, and the third is the one that binds. The first two
//! are ordinary: the scope holds what the prelude and host tables describe, and
//! a source declaration over one of those names is a shadow event a policy
//! settles. The third is **resolution equivalence** — every program the retired
//! projection-site recognizer accepted resolves the same way through the scope,
//! and the enumeration is exhaustive over that recognizer's whole domain rather
//! than a sample of it, because a recognizer is exactly a finite table and
//! nothing less than all of it is a proof.

use alloc::borrow::ToOwned as _;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_surface_engine::boundary::MatchDecision;
use gandr_surface_engine::boundary::ScopeSegment;
use gandr_surface_engine::boundary::SourceRange;
use gandr_surface_engine::diag::DiagnosticDetail;
use gandr_surface_engine::diag::Severity;
use gandr_surface_engine::host::HOST_MODULES;
use gandr_surface_engine::lower::LowerError;
use gandr_surface_engine::lower::Strictness;
use gandr_surface_engine::lower::lower_source;
use gandr_surface_engine::lower::lower_source_total;
use gandr_surface_engine::lower::lower_source_with_shadow_policy;
use gandr_surface_engine::namespace::Binding;
use gandr_surface_engine::namespace::Segment;
use gandr_surface_engine::namespace::Trie;
use gandr_surface_engine::prelude::prelude_ctx;
use gandr_surface_engine::recognition::PathResolution;
use gandr_surface_engine::recognition::Recognition;
use gandr_surface_engine::recognition::RecognitionSite;
use gandr_surface_engine::recognition::Recognized;
use gandr_surface_engine::recognition::ShadowPolicy;
use gandr_surface_engine::recognition::member_path;
use gandr_surface_engine::recognition::namespace_path;
use gandr_surface_engine::session::Session;

use crate::common::TestText;

/// The prelude's module-qualified builtins, read back through the scope the
/// prelude table seeded — the enumeration the equivalence tests quantify over.
fn prelude_members(recognition: &Recognition) -> Vec<(String, String)>
{
    let mut out = Vec::new();
    for binding in prelude_ctx().bindings() {
        let Some((module, member)) = binding.0.split_once('.')
        else {
            continue;
        };
        if matches!(
            recognition.resolve_member(ScopeSegment(module), ScopeSegment(member)),
            Some(&Recognized::PreludeMember { .. })
        ) {
            out.push((module.to_owned(), member.to_owned()));
        }
    }
    out
}

/// A one-binding namespace for a source declaration at its own root.
fn declaration(site: SourceRange) -> Trie<Recognized, RecognitionSite>
{
    let mut namespace = Trie::empty();
    drop(namespace.insert(
        gandr_surface_engine::namespace::NamePath::root(),
        Binding::new(Recognized::Definition, RecognitionSite::Source(site)),
    ));
    namespace
}

#[test]
fn the_outermost_scope_resolves_every_prelude_and_host_name()
{
    let recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    let members = prelude_members(&recognition);
    assert!(
        !members.is_empty(),
        "the prelude seeds at least one module builtin"
    );
    for entry in &members {
        let (module, member) = (&entry.0, &entry.1);
        assert!(
            matches!(
                recognition.resolve_name(ScopeSegment(module.as_str())),
                Some(&Recognized::PreludeNamespace)
            ),
            "`{module}` resolves as a prelude namespace"
        );
        assert!(
            matches!(
                recognition
                    .resolve_member(ScopeSegment(module.as_str()), ScopeSegment(member.as_str())),
                Some(&Recognized::PreludeMember { .. })
            ),
            "`{module}.{member}` resolves as a prelude member"
        );
    }
    for host in HOST_MODULES {
        assert!(
            matches!(
                recognition.resolve_name(ScopeSegment(host.name)),
                Some(&Recognized::HostNamespace { .. })
            ),
            "`{}` resolves as a host namespace",
            host.name
        );
        for member in host.members {
            assert!(
                matches!(
                    recognition.resolve_member(ScopeSegment(host.name), ScopeSegment(member.op)),
                    Some(&Recognized::HostMember { .. })
                ),
                "`{}.{}` resolves as a host member",
                host.name,
                member.op
            );
        }
    }
    assert!(
        recognition
            .resolve_name(ScopeSegment("definitely_not_a_builtin"))
            .is_none(),
        "a name the tables do not carry resolves to nothing"
    );
    assert!(
        recognition.shadowed().is_empty(),
        "a freshly seeded scope has shadowed nothing"
    );
}

#[test]
fn only_governed_namespaces_decline_an_unknown_member()
{
    let declining = [
        Recognized::PreludeNamespace,
        Recognized::HostNamespace {
            module: 0_usize.into(),
        },
        Recognized::ModuleNamespace,
    ];
    let permitting = [
        Recognized::PreludeMember {
            qualified: "prim.id".to_owned(),
        },
        Recognized::HostMember {
            module: 0_usize.into(),
            member: 0_usize.into(),
        },
        Recognized::ForeignNamespace,
        Recognized::ForeignMember,
        Recognized::ModuleComponent,
        Recognized::Definition,
    ];
    for kind in &declining {
        assert!(
            kind.declines_unknown_member(),
            "{kind:?} declines an unknown member"
        );
    }
    for kind in &permitting {
        assert!(
            !kind.declines_unknown_member(),
            "{kind:?} falls through to the ordinary projection"
        );
    }
}

/// Only a host member is call-only, so only its bare selection is refused.
///
/// The negative half carries the weight. A foreign member is *also* reachable
/// only by calling it, and it is still excluded here, because a bare selection
/// under an `extern` namespace fell through to the ordinary projection before
/// recognition graduated and the equivalence check pins that. The asymmetry is
/// a preserved disposition, not an oversight.
#[test]
fn only_a_host_member_is_call_only()
{
    assert!(
        Recognized::HostMember {
            module: 0_usize.into(),
            member: 0_usize.into(),
        }
        .is_call_only(),
        "a host member exists only as a call"
    );
    for kind in [
        Recognized::PreludeNamespace,
        Recognized::PreludeMember {
            qualified: "prim.id".to_owned(),
        },
        Recognized::HostNamespace {
            module: 0_usize.into(),
        },
        Recognized::ForeignNamespace,
        Recognized::ForeignMember,
        Recognized::ModuleNamespace,
        Recognized::ModuleComponent,
        Recognized::Definition,
    ] {
        assert!(
            !kind.is_call_only(),
            "{kind:?} is selectable without being called"
        );
    }
}

/// A path's verdict is set by the deepest prefix that resolved, never by
/// whether the whole path resolved.
///
/// The separation this makes is the one the projection site needs and a
/// whole-path `resolve` cannot give: `M.nope` and `stranger.nope` both fail to
/// resolve, and only the first is an error. It also pins the case that keeps
/// ordinary record projection alive under a module — `M.cfg.port`, where `cfg`
/// is an exported value whose own fields the record carrier owns.
#[test]
fn a_path_is_governed_by_its_deepest_resolved_prefix()
{
    let site = SourceRange(0 .. 8);
    let mut recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    let mut namespace = Trie::empty();
    drop(namespace.insert(
        gandr_surface_engine::namespace::NamePath::root(),
        Binding::new(
            Recognized::ModuleNamespace,
            RecognitionSite::Source(site.clone()),
        ),
    ));
    // `cfg` is an ordinary exported value; `inner` is a nested module with one
    // component of its own. The pair is what separates the two fall-through
    // reasons from the one decline.
    drop(namespace.insert(
        namespace_path(ScopeSegment("cfg")),
        Binding::new(
            Recognized::ModuleComponent,
            RecognitionSite::Source(site.clone()),
        ),
    ));
    drop(namespace.insert(
        namespace_path(ScopeSegment("inner")),
        Binding::new(
            Recognized::ModuleNamespace,
            RecognitionSite::Source(site.clone()),
        ),
    ));
    drop(namespace.insert(
        member_path(ScopeSegment("inner"), ScopeSegment("answer")),
        Binding::new(
            Recognized::ModuleComponent,
            RecognitionSite::Source(site.clone()),
        ),
    ));
    recognition
        .declare(Segment::from("M"), namespace, site)
        .expect("declaring M succeeds");

    let path = |segments: &[&str]| {
        recognition.resolve_path(
            &segments
                .iter()
                .map(|segment| ScopeSegment(segment))
                .collect::<Vec<_>>(),
        )
    };

    assert_eq!(
        path(&["M", "inner", "answer"]),
        PathResolution::Complete(Recognized::ModuleComponent),
        "a fully bound nested path resolves completely, at depth 3"
    );
    assert_eq!(
        path(&["M", "inner"]),
        PathResolution::Complete(Recognized::ModuleNamespace),
        "a nested module is itself a complete resolution"
    );
    assert_eq!(
        path(&["M", "nope"]),
        PathResolution::UnknownMember {
            depth: 1,
            namespace: Recognized::ModuleNamespace,
        },
        "an absent component under a module is governed and declines"
    );
    assert_eq!(
        path(&["M", "inner", "nope"]),
        PathResolution::UnknownMember {
            depth: 2,
            namespace: Recognized::ModuleNamespace,
        },
        "the decline follows nesting to the depth that actually governs"
    );
    assert_eq!(
        path(&["M", "cfg", "port"]),
        PathResolution::Ungoverned,
        "a value component's own fields belong to the record carrier"
    );
    assert_eq!(
        path(&["stranger", "nope"]),
        PathResolution::Ungoverned,
        "an unbound root is ungoverned, which is what a record projection is"
    );
}

#[test]
fn a_declaration_displaces_the_whole_builtin_subtree()
{
    let mut recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    assert!(
        recognition
            .resolve_member(ScopeSegment("list"), ScopeSegment("each"))
            .is_some(),
        "the prelude seeds `list.each` before the declaration"
    );
    recognition
        .declare(
            Segment::from("list"),
            declaration(SourceRange(0 .. 7)),
            SourceRange(0 .. 7),
        )
        .expect("warn-and-allow accepts the shadow");
    assert!(
        matches!(
            recognition.resolve_name(ScopeSegment("list")),
            Some(&Recognized::Definition)
        ),
        "the declaration takes the name"
    );
    assert!(
        recognition
            .resolve_member(ScopeSegment("list"), ScopeSegment("each"))
            .is_none(),
        "the whole displaced subtree goes with it, so no member is left behind"
    );
    assert!(
        recognition
            .resolve_member(ScopeSegment("prim"), ScopeSegment("id"))
            .is_some(),
        "an unrelated namespace is untouched"
    );
}

#[test]
fn shadowing_a_builtin_warns_by_default_and_rejects_under_policy()
{
    let mut warning = Recognition::new(ShadowPolicy::WarnAndAllow);
    warning
        .declare(
            Segment::from("record"),
            declaration(SourceRange(4 .. 10)),
            SourceRange(4 .. 10),
        )
        .expect("warn-and-allow accepts");
    assert_eq!(
        1,
        warning.shadowed().len(),
        "warn-and-allow records exactly one event"
    );
    assert_eq!(
        namespace_path(ScopeSegment("record")),
        warning.shadowed()[0].path,
        "the event names the shadowed path"
    );
    assert_eq!(
        SourceRange(4 .. 10),
        warning.shadowed()[0].byte_range,
        "the event carries the declaration's span"
    );

    let mut rejecting = Recognition::new(ShadowPolicy::Reject);
    let refused = rejecting
        .declare(
            Segment::from("record"),
            declaration(SourceRange(4 .. 10)),
            SourceRange(4 .. 10),
        )
        .expect_err("the reject policy refuses");
    assert_eq!(
        &namespace_path(ScopeSegment("record")),
        refused.path(),
        "the refusal names the path"
    );
    assert!(
        rejecting
            .resolve_member(ScopeSegment("record"), ScopeSegment("get"))
            .is_some(),
        "a refused declaration leaves the scope as it was"
    );
}

#[test]
fn redeclaring_a_source_name_is_not_a_shadow_event()
{
    let mut recognition = Recognition::new(ShadowPolicy::Reject);
    recognition
        .declare(
            Segment::from("mine"),
            declaration(SourceRange(0 .. 4)),
            SourceRange(0 .. 4),
        )
        .expect("a fresh name shadows nothing");
    recognition
        .declare(
            Segment::from("mine"),
            declaration(SourceRange(9 .. 13)),
            SourceRange(9 .. 13),
        )
        .expect("one source declaration over another is ordinary rebinding");
    assert!(
        recognition.shadowed().is_empty(),
        "neither declaration displaced a builtin"
    );
}

#[test]
fn resuming_carries_the_names_and_drops_the_events()
{
    let mut first = Recognition::new(ShadowPolicy::WarnAndAllow);
    first
        .declare(
            Segment::from("string"),
            declaration(SourceRange(0 .. 6)),
            SourceRange(0 .. 6),
        )
        .expect("warn-and-allow accepts");
    assert_eq!(
        1,
        first.shadowed().len(),
        "the first run recorded its event"
    );

    let resumed = Recognition::resumed(&first, ShadowPolicy::WarnAndAllow);
    assert!(
        matches!(
            resumed.resolve_name(ScopeSegment("string")),
            Some(&Recognized::Definition)
        ),
        "the declaration is still in scope on the next submission"
    );
    assert!(
        resumed
            .resolve_member(ScopeSegment("string"), ScopeSegment("escape"))
            .is_none(),
        "and so is the displacement it performed"
    );
    assert!(
        resumed.shadowed().is_empty(),
        "each submission reports only its own events"
    );
}

#[test]
fn a_resumed_declaration_binds_without_reporting()
{
    let mut recognition = Recognition::new(ShadowPolicy::Reject);
    recognition.declare_resumed(Segment::from("path"), declaration(SourceRange(0 .. 4)));
    assert!(
        matches!(
            recognition.resolve_name(ScopeSegment("path")),
            Some(&Recognized::Definition)
        ),
        "the carried-over declaration binds"
    );
    assert!(
        recognition.shadowed().is_empty(),
        "an earlier submission's shadowing is not this submission's event"
    );
}

#[test]
fn a_binder_over_a_builtin_reports_without_shadowing()
{
    let mut recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    recognition
        .note_binder(ScopeSegment("env"), SourceRange(7 .. 10))
        .expect("warn-and-allow accepts a binder collision");
    assert_eq!(
        1,
        recognition.shadowed().len(),
        "the binder collision is reported"
    );
    assert!(
        matches!(
            recognition.resolve_name(ScopeSegment("env")),
            Some(&Recognized::HostNamespace { .. })
        ),
        "the binder changes no resolution: `env` still names the host module"
    );
    assert!(
        recognition
            .resolve_member(ScopeSegment("env"), ScopeSegment("get"))
            .is_some(),
        "and its members are still reachable"
    );

    let mut quiet = Recognition::new(ShadowPolicy::Reject);
    quiet
        .note_binder(ScopeSegment("not_a_builtin"), SourceRange(0 .. 3))
        .expect("a binder over nothing is not an event");
    assert!(quiet.shadowed().is_empty(), "and reports nothing");
    quiet
        .note_binder(ScopeSegment("env"), SourceRange(0 .. 3))
        .expect_err("the reject policy refuses a binder collision too");
}

/// Fable's pinned witness: a parameter named `env` produces the collision
/// diagnostic **and** leaves `env.get` resolving to the host module. Both
/// halves are the point — the diagnostic exists precisely because the
/// resolution does not move, and a later rung's value environment is what will
/// move it.
#[test]
fn a_parameter_named_for_a_host_module_reports_and_still_resolves_to_the_host()
{
    let lowered =
        lower_source_total("def wrapper(env: Integer) -> F String { env.get(\"HOME\") }".into())
            .expect("total lowering never fails on parseable input");
    assert_eq!(
        1,
        lowered.shadowed_builtins().len(),
        "the binder collision is reported once: {:?}",
        lowered.shadowed_builtins()
    );
    assert_eq!(
        namespace_path(ScopeSegment("env")),
        lowered.shadowed_builtins()[0].path,
        "the report names the host module the binder took"
    );
    let performs = lowered.items.iter().any(|item| {
        let Term::Value(Value::Thunk(_, ref body)) = item.term
        else {
            return false;
        };
        contains_perform(body).0
    });
    assert!(
        performs,
        "`env.get` still elaborates to the host perform: {:?}",
        lowered.items
    );
}

/// Whether a computation contains a `perform` anywhere, over an explicit
/// worklist so the walk carries no call-stack depth.
fn contains_perform(comp: &Comp) -> MatchDecision
{
    let mut pending: Vec<&Comp> = Vec::from([comp]);
    while let Some(current) = pending.pop() {
        match *current {
            | Comp::Perform(..) => return MatchDecision(true),
            | Comp::Abs(_, _, ref body) => pending.push(body),
            | Comp::Bind(ref bound, _, ref rest) => {
                pending.push(bound);
                pending.push(rest);
            },
            | Comp::App(ref head, _) => pending.push(head),
            | _ => {},
        }
    }
    MatchDecision(false)
}

#[test]
fn a_shadowed_builtin_is_reported_as_a_warning()
{
    const SOURCE: &str = "def list = 1;\ndef used = list;";
    let lowered = lower_source_total(SOURCE.into()).expect("total lowering");
    let report = gandr_surface_engine::diag::report(&lowered, &prelude_ctx());
    let warnings: Vec<&gandr_surface_engine::diag::Diagnostic> = report
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.severity, Severity::Warning))
        .collect();
    assert_eq!(1, warnings.len(), "one warning: {:?}", report.diagnostics);
    assert!(
        matches!(
            warnings[0].detail,
            DiagnosticDetail::ShadowedName { ref path } if path == "list"
        ),
        "the warning names the shadowed path: {:?}",
        warnings[0]
    );
    let span = &warnings[0].annotations[0].span;
    assert_eq!(
        Some("list"),
        SOURCE.get(span.start .. span.end),
        "the warning primary must be the shadowing declaration name"
    );
}

#[test]
fn a_declaration_shadowing_a_builtin_is_rejected_under_policy()
{
    let source = "def list = 1;";
    lower_source_with_shadow_policy(
        source.into(),
        Strictness::Strict,
        ShadowPolicy::WarnAndAllow,
    )
    .expect("warn-and-allow accepts the declaration");
    let refused =
        lower_source_with_shadow_policy(source.into(), Strictness::Strict, ShadowPolicy::Reject)
            .expect_err("the reject policy refuses it");
    assert!(
        matches!(refused, LowerError::ShadowedBuiltin { .. }),
        "the refusal is the recognition one: {refused:?}"
    );
    let refused_totally =
        lower_source_with_shadow_policy(source.into(), Strictness::Total, ShadowPolicy::Reject)
            .expect_err("a refused policy is a decision about the program, not a damaged region");
    assert!(
        matches!(refused_totally, LowerError::ShadowedBuiltin { .. }),
        "total mode does not recover a policy refusal: {refused_totally:?}"
    );
}

#[test]
fn a_session_rejects_a_shadowed_builtin_under_policy()
{
    let mut permissive = Session::new();
    permissive
        .submit("def record = 1;")
        .expect("warn-and-allow accepts");

    let mut strict = Session::new();
    strict.set_shadow_policy(ShadowPolicy::Reject);
    let refused = strict
        .submit("def record = 1;")
        .expect_err("the session policy refuses");
    assert!(
        matches!(refused, LowerError::ShadowedBuiltin { .. }),
        "the session refusal is the recognition one: {refused:?}"
    );
}

#[test]
fn a_session_carries_a_shadowing_declaration_into_the_next_submission()
{
    let mut session = Session::new();
    session
        .submit("def list = 1;")
        .expect("the shadowing declaration submits");
    let second = session
        .submit("def used = list;")
        .expect("the next submission lowers");
    assert!(
        second
            .report
            .diagnostics
            .iter()
            .all(|diagnostic| !matches!(diagnostic.severity, Severity::Warning)),
        "the shadowing is reported once, on the submission that performed it: {:?}",
        second.report.diagnostics
    );
}

// --- Resolution equivalence
// ------------------------------------------------------------

/// Every `module.member` the retired constant-table recognizer accepted still
/// elaborates to the same flat qualified `Var`.
///
/// The quantification is the point: the retired recognizer was the prelude
/// module table and nothing else, so sweeping every entry of that table is the
/// whole of its accepting domain rather than a sample of it.
#[test]
fn every_prelude_selection_resolves_identically_through_the_scope()
{
    let recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    for entry in prelude_members(&recognition) {
        let (module, member) = (entry.0, entry.1);
        let source = format!("{module}.{member}");
        let lowered = lower_source(TestText(source.as_str()).0.into())
            .unwrap_or_else(|error| panic!("`{source}` must lower: {error}"));
        let item = lowered.items.first().expect("one item");
        assert!(
            matches!(item.term, Term::Value(Value::Var(ref name)) if *name == source),
            "`{source}` elaborates to the flat qualified Var, got {:?}",
            item.term
        );
    }
}

/// Every host call the retired recognizer accepted still performs against the
/// same signature and operation, and the payload convention is unchanged.
#[test]
fn every_host_call_resolves_identically_through_the_scope()
{
    for host in HOST_MODULES {
        for member in host.members {
            let arguments: Vec<String> = member
                .params
                .iter()
                .map(|_param| "\"x\"".to_owned())
                .collect();
            let source = format!("{}.{}({})", host.name, member.op, arguments.join(", "));
            let lowered = lower_source(TestText(source.as_str()).0.into())
                .unwrap_or_else(|error| panic!("`{source}` must lower: {error}"));
            let item = lowered.items.first().expect("one item");
            let Term::Comp(ref comp) = item.term
            else {
                panic!("`{source}` performs, got {:?}", item.term);
            };
            let performed = performed_operation(comp);
            assert_eq!(
                Some((host.sig().name().as_ref().to_owned(), member.op.to_owned())),
                performed,
                "`{source}` performs its own signature's operation"
            );
        }
    }
}

/// The signature name and operation of the sole `perform` in a computation,
/// walking past the hoists a computation argument introduces.
fn performed_operation(comp: &Comp) -> Option<(String, String)>
{
    let mut current = comp;
    loop {
        match *current {
            | Comp::Perform(ref sig, ref op, _) => {
                return Some((sig.name().as_ref().to_owned(), (*op).clone()));
            },
            | Comp::Bind(_, _, ref rest) => current = rest,
            | _ => return None,
        }
    }
}

/// Every unknown member under a prelude or host namespace is still declined,
/// and a name under no namespace at all still falls through to the ordinary
/// projection — the two halves of the retired gate's `None` case.
#[test]
fn every_declined_selection_is_declined_identically()
{
    let recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    let mut namespaces: Vec<String> = HOST_MODULES
        .iter()
        .map(|host| host.name.to_owned())
        .collect();
    for entry in prelude_members(&recognition) {
        if !namespaces.contains(&entry.0) {
            namespaces.push(entry.0);
        }
    }
    for namespace in &namespaces {
        let source = format!("{namespace}.definitely_not_a_member");
        let error = lower_source(TestText(source.as_str()).0.into())
            .err()
            .unwrap_or_else(|| panic!("`{source}` must be declined"));
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "`{source}` is declined as Unsupported, got {error:?}"
        );
    }
    let lowered = lower_source("def r = #{ a = 1 };\nr.a".into())
        .expect("a genuine record projection is not a module selection");
    assert!(
        lowered
            .items
            .iter()
            .any(|item| matches!(item.term, Term::Comp(Comp::RecordProj { .. }))),
        "a path the scope does not bind falls through to the record projection: {:?}",
        lowered.items
    );
}

/// The **one** observable delta: a user declaration over a prelude name changes
/// what a selection under it resolves to, and says so.
#[test]
fn user_shadowing_is_the_only_observable_delta()
{
    let unshadowed = lower_source_total("prim.id".into()).expect("total lowering");
    assert!(
        matches!(
            unshadowed.items.first().map(|item| &item.term),
            Some(&Term::Value(Value::Var(ref name))) if name == "prim.id"
        ),
        "without a declaration the prelude selection is unchanged: {:?}",
        unshadowed.items
    );
    assert!(
        unshadowed.shadowed_builtins().is_empty(),
        "and nothing is reported"
    );

    let shadowed =
        lower_source_total("def prim = #{ id = 1 };\nprim.id".into()).expect("total lowering");
    assert_eq!(
        1,
        shadowed.shadowed_builtins().len(),
        "the declaration is reported"
    );
    assert!(
        shadowed.items.iter().any(|item| matches!(
            item.term,
            Term::Comp(Comp::RecordProj { ref label, .. }) if label == "id"
        )),
        "and the selection is now an ordinary projection on the user's value: {:?}",
        shadowed.items
    );
}

/// An `extern` block named after a host module wins, and now says so — the
/// pre-existing precedence, restated as a shadow event.
#[test]
fn an_extern_declaration_shadows_a_host_module_and_reports_it()
{
    let lowered = lower_source(
        "extern \"c\" from \"fs\" {\n  def read(x: f64) -> f64;\n}\nfs.read(2.0f64)".into(),
    )
    .expect("the shadowing extern lowers");
    let item = lowered.items.first().expect("the call lowers to an item");
    let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
    else {
        panic!("the call performs, got {:?}", item.term);
    };
    assert_eq!("fs", sig.name().as_ref(), "against the extern's signature");
    assert_eq!("read", op);
    assert!(
        matches!(**arg, Value::Record(_)),
        "with the foreign argument-record convention, got {arg:?}"
    );
    assert_eq!(
        1,
        lowered.shadowed_builtins().len(),
        "and the displacement of the host module is reported"
    );
    assert_eq!(
        namespace_path(ScopeSegment("fs")),
        lowered.shadowed_builtins()[0].path,
        "naming the host module it took"
    );
}

/// A prelude member path and a host member path are two segments, and the
/// helpers that build them agree with what the scope was seeded under.
#[test]
fn member_paths_agree_with_the_seeded_scope()
{
    let recognition = Recognition::new(ShadowPolicy::WarnAndAllow);
    assert!(
        recognition
            .resolve(&member_path(ScopeSegment("prim"), ScopeSegment("id")))
            .is_some(),
        "the two-segment path is what the seed bound"
    );
    assert!(
        recognition
            .resolve(&namespace_path(ScopeSegment("prim")))
            .is_some(),
        "and the one-segment path names its namespace"
    );
}
