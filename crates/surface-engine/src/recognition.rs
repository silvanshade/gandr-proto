//! Recognition — prelude, host, and foreign names as an outermost scope.
//!
//! Before this module existed, "is `fs.read` a host call?" and "is `prim.id` a
//! builtin?" were **syntactic** questions asked at the projection site: the
//! lowerer consulted a constant table by name, and a script that declared its
//! own `env` collided with a reservation instead of shadowing a binding. That
//! table lookup was a second name authority sitting beside
//! [`crate::namespace`]'s scope engine, and the two could not be reconciled
//! once module paths became ordinary names.
//!
//! Recognition now **graduates into scoped resolution**. The prelude modules
//! (`prim`, `list`, `record`, …), the host modules (`fs`, `env`, `proc`), and
//! every `extern`-declared foreign module are bindings in one outermost visible
//! namespace, built over [`crate::namespace::Scope::with_init_visible`]. Asking
//! whether `fs.read` is a host call is [`Recognition::resolve`] — the same
//! lookup a user module path takes.
//!
//! # What a user declaration does to a builtin
//!
//! A top-level `def`, `module`, or `extern` declaration **shadows** whatever
//! the outermost scope held at that name, together with its whole subtree: a
//! `module list { … }` displaces `list`, `list.each`, and every other
//! `list.*` binding at once, so `list.each` afterwards means the user's
//! component and not the prelude's. That is what makes shadowing coherent — a
//! partially shadowed namespace would resolve `list` to the user and
//! `list.each` to the prelude.
//!
//! The shadow is performed as the scope engine's ordinary shadow event, so its
//! **policy is a handler** rather than an engine mode:
//!
//! | policy | what a shadowed builtin does |
//! | --- | --- |
//! | [`ShadowPolicy::WarnAndAllow`] | the user binding wins; the event is recorded as a [`ShadowedBuiltin`] a diagnostic layer reports |
//! | [`ShadowPolicy::Reject`] | the event is refused, and lowering fails with the refusal |
//!
//! Warn-and-allow is the default, so the graduation changes no program that
//! did not already collide with a reservation.
//!
//! # What is deliberately not here
//!
//! **Inner binders are reported but do not shadow.** A `let list = …` or a
//! lambda parameter named `list` is checked against the outermost scope's roots
//! at its introduction ([`Recognition::note_binder`]) and reported under the
//! same policy, and `list.each` after it still resolves to the prelude, because
//! the lowerer carries no value environment to bind the binder in. The two
//! facts together are what the diagnostic is for. The machinery does not care:
//! an inner scope is a [`crate::namespace::Scope::begin_section`] away, and
//! threading one through expression lowering is its own change.
//!
//! **The tables are seed data, not authority.** [`crate::prelude`] and
//! [`crate::host`] still own what the builtins *are* — their primitives,
//! signatures, and parameter shapes — and this module reads them once, at
//! construction, to build the namespace. Nothing downstream asks them whether a
//! name is recognized.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use alloc::vec::Vec;

use crate::boundary::HostMemberIndex;
use crate::boundary::HostModuleIndex;
use crate::boundary::MatchDecision;
use crate::boundary::QualifiedName;
use crate::boundary::ScopeSegment;
use crate::boundary::SourceRange;
use crate::host::HOST_MODULES;
use crate::namespace::Binding;
use crate::namespace::Collision;
use crate::namespace::EventKind;
use crate::namespace::EventRejection;
use crate::namespace::Modifier;
use crate::namespace::NamePath;
use crate::namespace::NamespaceEventHandler;
use crate::namespace::RejectionReason;
use crate::namespace::Scope;
use crate::namespace::Segment;
use crate::namespace::Trie;
use crate::prelude::MODULE_BUILTINS;
use crate::prelude::qualified;

/// What an outermost-scope path resolves to.
///
/// The payload is what the *elaborator* needs once the name is recognized: a
/// prelude member carries the flat qualified name it becomes, a host member
/// carries the coordinates of its signature and parameter list, and a user
/// module component carries nothing because its elaboration is the ordinary
/// dotted projection it always was.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Recognized
{
    /// A prelude module namespace — `prim`, `list`, `record`, `string`,
    /// `regex`, `path`.
    PreludeNamespace,
    /// A prelude module member, with the flat qualified name both preludes
    /// bind it under (`prim.id`).
    PreludeMember
    {
        /// The `module.member` name the module-select elaboration emits.
        qualified: String,
    },
    /// A host module namespace — `fs`, `env`, `proc`.
    HostNamespace
    {
        /// The module's position in [`HOST_MODULES`].
        module: HostModuleIndex,
    },
    /// A host module member, whose call elaborates to a `perform`.
    HostMember
    {
        /// The module's position in [`HOST_MODULES`].
        module: HostModuleIndex,
        /// The member's position in that module's member list.
        member: HostMemberIndex,
    },
    /// An `extern`-declared foreign module namespace.
    ForeignNamespace,
    /// An `extern`-declared foreign function, whose call elaborates to a
    /// `perform` against the module's per-library signature.
    ForeignMember,
    /// A `module` declaration, or one of its nested module components.
    ModuleNamespace,
    /// A value component of a `module` declaration.
    ModuleComponent,
    /// An ordinary top-level `def`.
    Definition,
}

impl Recognized
{
    /// Whether an unknown member under this namespace is a lowering error
    /// rather than a record projection.
    ///
    /// A prelude or host module namespace is **not a record value**, so
    /// `prim.nope` is declined instead of falling through to a structural
    /// projection. A user module namespace declines for a sharper reason: the
    /// scope holds *exactly* its exported components, so `M.nope` is not an
    /// unknown quantity to be guessed at — recognition knows the module and
    /// knows the component is not in it. A hidden member declines by the same
    /// token, because matching removed it and [`Recognition`] never bound it.
    ///
    /// A **foreign** namespace still falls through. Its members exist only as
    /// calls, lowered before projection is reached, and the scope holds no
    /// per-member binding to check a bare selection against.
    ///
    /// # Contract
    /// - ensures: true exactly for [`Self::PreludeNamespace`],
    ///   [`Self::HostNamespace`], and [`Self::ModuleNamespace`].
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which namespaces decline),
    ///   separated by asserting the verdict for every variant.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `only_governed_namespaces_decline_an_unknown_member`
    #[inline]
    #[must_use]
    pub const fn declines_unknown_member(&self) -> MatchDecision
    {
        MatchDecision(matches!(
            *self,
            Self::PreludeNamespace | Self::HostNamespace { .. } | Self::ModuleNamespace
        ))
    }

    /// Whether a **bare** selection of this name is an error rather than a
    /// value.
    ///
    /// A host member exists only as a call: the call path lowers `fs.read(p)`
    /// to a `perform` against the module's signature before projection is
    /// reached, so `fs.read` standing alone names nothing a projection could
    /// produce. It is declined exactly as the retired constant-table gate
    /// declined it.
    ///
    /// A **foreign** member is excluded, and that is deliberate rather than an
    /// oversight: a bare selection under an `extern` namespace fell through to
    /// the ordinary projection before recognition graduated, and the
    /// resolution-equivalence check pins that disposition. Changing it is a
    /// separate decision from wiring the module stratum.
    ///
    /// # Contract
    /// - ensures: true exactly for [`Self::HostMember`].
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which names are
    ///   call-only), separated by asserting the verdict for every variant.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `only_a_host_member_is_call_only`
    #[inline]
    #[must_use]
    pub const fn is_call_only(&self) -> MatchDecision
    {
        MatchDecision(matches!(*self, Self::HostMember { .. }))
    }

    /// The flat qualified name a prelude member elaborates to.
    ///
    /// # Contract
    /// - ensures: `Some` exactly for [`Self::PreludeMember`], carrying the
    ///   `module.member` name both preludes bind the builtin under.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn prelude_qualified(&self) -> Option<QualifiedName<'_>>
    {
        match *self {
            | Self::PreludeMember { ref qualified } => Some(QualifiedName(qualified.as_str())),
            | _ => None,
        }
    }
}

/// What the outermost scope says about a whole dotted path.
///
/// A projection chain like `M.inner.y` is one path, not a target plus a field,
/// and the three verdicts are the only dispositions lowering has for it. The
/// distinction that matters is the middle one: **a path can be governed
/// without resolving**, and that is exactly the case a guessed record
/// projection used to swallow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathResolution
{
    /// Every segment resolved. The payload is what the whole path names.
    Complete(Recognized),
    /// A proper prefix resolved to a namespace that governs its members, and
    /// the very next segment is not one of them — an absent or hidden
    /// component. Lowering declines rather than guessing a projection.
    UnknownMember
    {
        /// How many leading segments resolved.
        depth: usize,
        /// The governing namespace those segments named.
        namespace: Recognized,
    },
    /// Recognition does not govern this path: its root is unbound, or the
    /// deepest thing it resolved is an ordinary value — a `def`, a module
    /// component, a foreign namespace — whose own fields are the record
    /// carrier's business rather than the scope's.
    Ungoverned,
}

/// Where a binding came from.
///
/// The distinction is the whole of the shadow rule: displacing a
/// [`Self::Builtin`] binding is the event a policy settles, while one source
/// declaration replacing another is ordinary rebinding and reports nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecognitionSite
{
    /// Seeded from the prelude or host tables when the scope was built.
    Builtin,
    /// Declared by the source being lowered, at this byte range.
    Source(SourceRange),
}

/// The shadow policy for the outermost scope.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShadowPolicy
{
    /// The user declaration wins and the event is recorded for reporting.
    #[default]
    WarnAndAllow,
    /// The shadow is refused and lowering fails.
    Reject,
}

/// One recorded shadowing of a builtin by a source declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowedBuiltin
{
    /// The path the source declaration took over.
    pub path: NamePath,
    /// The declaration's byte range.
    pub byte_range: SourceRange,
}

/// The handler that gives the three namespace events their recognition
/// meaning.
///
/// Not-found and hook events are inert here: `except` performs not-found
/// whenever a declaration shadows nothing, which is the ordinary case, and no
/// recognition modifier carries a hook.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RecognitionHandler
{
    /// How a shadowed builtin is settled.
    policy: ShadowPolicy,
    /// Every builtin a source declaration displaced, in declaration order.
    shadowed: Vec<ShadowedBuiltin>,
    /// The byte range of the declaration currently being bound, used as the
    /// recorded event's site.
    site: Option<SourceRange>,
}

impl NamespaceEventHandler<Recognized, RecognitionSite> for RecognitionHandler
{
    type Label = ();

    #[inline]
    fn not_found(
        &mut self,
        _path: &NamePath,
    ) -> Result<(), EventRejection>
    {
        Ok(())
    }

    #[inline]
    fn shadow(
        &mut self,
        path: &NamePath,
        collision: Collision<Recognized, RecognitionSite>,
    ) -> Result<Binding<Recognized, RecognitionSite>, EventRejection>
    {
        if matches!(collision.former.tag, RecognitionSite::Builtin) {
            if matches!(self.policy, ShadowPolicy::Reject) {
                return Err(EventRejection::new(
                    EventKind::Shadow,
                    path.clone(),
                    RejectionReason::from(
                        "this declaration shadows a prelude or host name, which the active policy \
                         forbids",
                    ),
                ));
            }
            self.shadowed.push(ShadowedBuiltin {
                path: path.clone(),
                byte_range: match self.site {
                    | Some(ref range) => range.clone(),
                    | None => SourceRange(0 .. 0),
                },
            });
        }
        Ok(collision.latter)
    }

    #[inline]
    fn hook(
        &mut self,
        _path: &NamePath,
        _label: &(),
        subject: Trie<Recognized, RecognitionSite>,
    ) -> Result<Trie<Recognized, RecognitionSite>, EventRejection>
    {
        Ok(subject)
    }
}

/// The outermost visible scope plus the shadow policy that governs it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recognition
{
    /// The scope engine's value, seeded with the builtin namespace.
    scope: Scope<Recognized, RecognitionSite>,
    /// The event handler carrying the policy and the recorded shadowings.
    handler: RecognitionHandler,
}

impl Default for Recognition
{
    #[inline]
    fn default() -> Self
    {
        Self::new(ShadowPolicy::WarnAndAllow)
    }
}

impl Recognition
{
    /// The outermost scope, seeded from the prelude and host tables.
    ///
    /// Prelude names are seeded before host names, so a host module of the same
    /// name as a prelude module would win — an ordering that decides nothing
    /// today, because the two name sets are disjoint, and is fixed here so a
    /// future overlap has a stated answer rather than a discovered one.
    ///
    /// # Contract
    /// - ensures: every `(module, member)` of the prelude's module builtins and
    ///   every host module member resolves, as does each of their module
    ///   namespaces; nothing else does.
    /// - ensures: every seeded binding is tagged [`RecognitionSite::Builtin`],
    ///   so a source declaration over any of them is a shadow event.
    /// - provides: the graduated replacement for the retired projection-site
    ///   constant-table gate.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (which paths are seeded,
    ///   and the site every seeded binding carries), separated by resolving a
    ///   prelude member, a prelude namespace, a host member, a host namespace,
    ///   and an absent name, and by asserting the site of one seeded binding.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `the_outermost_scope_resolves_every_prelude_and_host_name`
    #[inline]
    #[must_use]
    pub fn new(policy: ShadowPolicy) -> Self
    {
        let mut builtins: Trie<Recognized, RecognitionSite> = Trie::empty();
        for &(module, member, _prim) in MODULE_BUILTINS {
            drop(builtins.insert(
                namespace_path(ScopeSegment(module)),
                Binding::new(Recognized::PreludeNamespace, RecognitionSite::Builtin),
            ));
            drop(builtins.insert(
                member_path(ScopeSegment(module), ScopeSegment(member)),
                Binding::new(
                    Recognized::PreludeMember {
                        qualified: qualified(module, member),
                    },
                    RecognitionSite::Builtin,
                ),
            ));
        }
        for (module_index, module) in HOST_MODULES.iter().enumerate() {
            drop(builtins.insert(
                namespace_path(ScopeSegment(module.name)),
                Binding::new(
                    Recognized::HostNamespace {
                        module: HostModuleIndex(module_index),
                    },
                    RecognitionSite::Builtin,
                ),
            ));
            for (member_index, member) in module.members.iter().enumerate() {
                drop(builtins.insert(
                    member_path(ScopeSegment(module.name), ScopeSegment(member.op)),
                    Binding::new(
                        Recognized::HostMember {
                            module: HostModuleIndex(module_index),
                            member: HostMemberIndex(member_index),
                        },
                        RecognitionSite::Builtin,
                    ),
                ));
            }
        }
        Self {
            scope: Scope::with_init_visible(builtins),
            handler: RecognitionHandler {
                policy,
                shadowed: Vec::new(),
                site: None,
            },
        }
    }

    /// The scope a previous lowering left, ready for the next submission.
    ///
    /// A REPL session carries recognition forward exactly as it carries the
    /// import scope forward: a `def list = 1;` on one line still shadows the
    /// prelude `list` on the next. The recorded shadowings are **not** carried
    /// — each submission reports its own — and the policy is re-supplied,
    /// because it belongs to the run and not to the accumulated names.
    ///
    /// # Contract
    /// - ensures: resolution agrees with `previous` on every path.
    /// - ensures: [`Self::shadowed`] is empty.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (what is carried and what
    ///   is reset), separated by resuming a scope that both holds a shadowing
    ///   declaration and recorded its event, then asserting resolution and the
    ///   empty event list.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `resuming_carries_the_names_and_drops_the_events`
    #[inline]
    #[must_use]
    pub fn resumed(
        previous: &Self,
        policy: ShadowPolicy,
    ) -> Self
    {
        Self {
            scope: previous.scope.clone(),
            handler: RecognitionHandler {
                policy,
                shadowed: Vec::new(),
                site: None,
            },
        }
    }

    /// What `path` resolves to in the outermost visible scope.
    #[inline]
    #[must_use]
    pub fn resolve(
        &self,
        path: &NamePath,
    ) -> Option<&Recognized>
    {
        self.scope.resolve(path).map(|binding| &binding.data)
    }

    /// What the one-segment name `name` resolves to.
    #[inline]
    #[must_use]
    pub fn resolve_name(
        &self,
        name: ScopeSegment<'_>,
    ) -> Option<&Recognized>
    {
        self.resolve(&namespace_path(name))
    }

    /// What the two-segment path `module.member` resolves to.
    #[inline]
    #[must_use]
    pub fn resolve_member(
        &self,
        module: ScopeSegment<'_>,
        member: ScopeSegment<'_>,
    ) -> Option<&Recognized>
    {
        self.resolve(&member_path(module, member))
    }

    /// What the whole dotted path `segments` resolves to, walked prefix by
    /// prefix.
    ///
    /// This is the projection-governing entry point, and it is deliberately
    /// **not** `resolve` on the full path: a path that fails to resolve is not
    /// automatically ungoverned. `M.nope` and `stranger.nope` both fail, and
    /// only the first is an error. Telling them apart needs the deepest prefix
    /// that *did* resolve, so the walk stops at the first unbound segment and
    /// reports what it had reached.
    ///
    /// The same walk is what makes a value component's own fields reachable:
    /// for `M.cfg.port` where `cfg` is an ordinary exported value, the walk
    /// resolves `M` and `M.cfg`, finds `M.cfg.port` unbound, and reports
    /// [`PathResolution::Ungoverned`] because the deepest thing it reached is a
    /// [`Recognized::ModuleComponent`] rather than a namespace. The record
    /// carrier owns `port`, exactly as before.
    ///
    /// # Contract
    /// - requires: `segments` is non-empty.
    /// - ensures: [`PathResolution::Complete`] iff every segment resolved.
    /// - ensures: [`PathResolution::UnknownMember`] iff a proper prefix
    ///   resolved to a namespace whose [`Recognized::declines_unknown_member`]
    ///   holds; `depth` is that prefix's length and is strictly less than
    ///   `segments.len()`.
    /// - ensures: [`PathResolution::Ungoverned`] otherwise, which includes an
    ///   unbound root.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which verdict a path
    ///   takes), separated by resolving paths that stop at each depth and under
    ///   each kind of deepest binding.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `a_path_is_governed_by_its_deepest_resolved_prefix`
    #[inline]
    #[must_use]
    pub fn resolve_path(
        &self,
        segments: &[ScopeSegment<'_>],
    ) -> PathResolution
    {
        let mut walked = Vec::with_capacity(segments.len());
        let mut deepest: Option<Recognized> = None;
        // Counted rather than derived from `walked`, so the reported depth is
        // the number of segments that actually resolved with no arithmetic
        // between the two.
        let mut resolved = 0_usize;
        for segment in segments {
            walked.push(Segment::from(segment.0.to_owned()));
            match self.resolve(&NamePath::from_segments(walked.clone())) {
                | Some(found) => {
                    deepest = Some(found.clone());
                    resolved = resolved.saturating_add(1);
                },
                | None => {
                    return match deepest {
                        | Some(namespace) if namespace.declines_unknown_member().0 => {
                            PathResolution::UnknownMember {
                                depth: resolved,
                                namespace,
                            }
                        },
                        | _ => PathResolution::Ungoverned,
                    };
                },
            }
        }
        deepest.map_or(PathResolution::Ungoverned, PathResolution::Complete)
    }

    /// Every builtin a source declaration displaced, in declaration order.
    #[inline]
    #[must_use]
    pub fn shadowed(&self) -> &[ShadowedBuiltin]
    {
        self.handler.shadowed.as_slice()
    }

    /// Binds one top-level source declaration, shadowing whatever the outermost
    /// scope held under `name`.
    ///
    /// `subtree` is the declaration's own namespace **relative to `name`**: its
    /// root binding is the declaration itself, and any deeper path is a
    /// component of it. Everything previously under `name` — the binding and
    /// its whole subtree — is displaced together, so a shadowed prelude module
    /// leaves no member behind for a later path to find.
    ///
    /// # Contract
    /// - ensures: after a successful call, `name` and every path in `subtree`
    ///   resolve to `subtree`'s bindings, and no path under `name` resolves to
    ///   anything the scope held before.
    /// - ensures: displacing a [`RecognitionSite::Builtin`] binding performs
    ///   one shadow event at `name`; displacing a source binding performs none.
    /// - fails: [`EventRejection`] under [`ShadowPolicy::Reject`] when the
    ///   declaration would shadow a builtin, leaving the scope unchanged.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`EventRejection`] when the policy refuses the shadow.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — four decision surfaces (subtree displacement,
    ///   the event's site test, the policy branch, and the rejection leaving
    ///   the scope alone), separated by declaring over a prelude module and
    ///   resolving one of its former members, by declaring over an unoccupied
    ///   name, by declaring over a prior source declaration, and by the same
    ///   builtin shadow under each policy.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `a_declaration_displaces_the_whole_builtin_subtree`
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `shadowing_a_builtin_warns_by_default_and_rejects_under_policy`
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `redeclaring_a_source_name_is_not_a_shadow_event`
    #[inline]
    pub fn declare(
        &mut self,
        name: Segment,
        subtree: Trie<Recognized, RecognitionSite>,
        site: SourceRange,
    ) -> Result<(), EventRejection>
    {
        let path = NamePath::from_segments(Vec::from([name]));
        // One clone, for the synthesized binding the collision may need; the
        // site itself moves into the handler.
        let synthesized = site.clone();
        self.handler.site = Some(site);
        if let Some(former) = self.displaced_binding(&path) {
            let latter = subtree.get(&NamePath::root()).cloned().unwrap_or_else(|| {
                Binding::new(Recognized::Definition, RecognitionSite::Source(synthesized))
            });
            if let Err(refused) = self.handler.shadow(&path, Collision { former, latter }) {
                self.handler.site = None;
                return Err(refused);
            }
        }
        // `except` drops the whole displaced subtree, so nothing the shadowed
        // namespace held can be reached under the new binding; its not-found
        // event on an unoccupied name is inert here.
        let cleared = self
            .scope
            .modify_visible(&Modifier::except(path.clone()), &mut self.handler);
        let bound =
            cleared.and_then(|()| self.scope.import_subtree(&path, subtree, &mut self.handler));
        self.handler.site = None;
        match bound {
            | Ok(()) => Ok(()),
            | Err(error) => Err(rejection_of(error, &path)),
        }
    }

    /// Reports one **value binder** whose name collides with an outermost-scope
    /// root, changing nothing.
    ///
    /// A `let`, a lambda parameter, a `case` arm binder — every binder the
    /// source writes — is checked at its introduction against the roots of the
    /// visible namespace, through the same handler and the same policy a
    /// declaration's shadow takes. **It is a point check and not a scope**: the
    /// namespace is untouched, so `fun env` still resolves `env.get` to the
    /// host module afterwards, and the diagnostic is what tells the reader that
    /// their binder and that resolution disagree. Making the binder actually
    /// shadow is a value environment's job, and this engine carries none.
    ///
    /// Only a **root** collision is reported. A binder is one segment, so it
    /// can only ever collide at depth one, and a deeper path is not a name a
    /// binder could take.
    ///
    /// # Contract
    /// - ensures: the scope is unchanged, whatever the outcome.
    /// - ensures: a binder matching a [`RecognitionSite::Builtin`] root records
    ///   one shadow event; a binder matching a source root or nothing records
    ///   none.
    /// - fails: [`EventRejection`] under [`ShadowPolicy::Reject`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`EventRejection`] when the policy refuses the collision.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — three decision surfaces (the root test, the
    ///   builtin-site test, and that the scope is left alone), separated by a
    ///   binder over a builtin root, a binder over a source root, a binder over
    ///   nothing, and resolution asserted exactly after each.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `a_binder_over_a_builtin_reports_without_shadowing`
    #[inline]
    pub fn note_binder(
        &mut self,
        name: ScopeSegment<'_>,
        site: SourceRange,
    ) -> Result<(), EventRejection>
    {
        let path = namespace_path(name);
        let Some(former) = self.scope.resolve(&path).cloned()
        else {
            return Ok(());
        };
        if !matches!(former.tag, RecognitionSite::Builtin) {
            return Ok(());
        }
        let latter = Binding::new(
            Recognized::Definition,
            RecognitionSite::Source(site.clone()),
        );
        self.handler.site = Some(site);
        let refused = self.handler.shadow(&path, Collision { former, latter });
        self.handler.site = None;
        refused.map(drop)
    }

    /// Binds a declaration carried over from an earlier submission, recording
    /// no event and refusing nothing.
    ///
    /// Its shadowing of a builtin, if any, belonged to the submission that
    /// wrote it: reporting it again on every later submission would turn one
    /// warning into an unbounded stream, and refusing it under a policy set
    /// afterwards would retroactively reject accepted source.
    ///
    /// # Contract
    /// - ensures: resolution afterwards is as [`Self::declare`] would leave it.
    /// - ensures: [`Self::shadowed`] is unchanged, whatever the policy.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (the binding, and the
    ///   event suppression), separated by resuming a declaration over a builtin
    ///   under [`ShadowPolicy::Reject`] and asserting both resolution and the
    ///   empty event list.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `a_resumed_declaration_binds_without_reporting`
    #[inline]
    pub fn declare_resumed(
        &mut self,
        name: Segment,
        subtree: Trie<Recognized, RecognitionSite>,
    )
    {
        let policy = core::mem::replace(&mut self.handler.policy, ShadowPolicy::WarnAndAllow);
        let before = self.handler.shadowed.len();
        drop(self.declare(name, subtree, SourceRange(0 .. 0)));
        self.handler.shadowed.truncate(before);
        self.handler.policy = policy;
    }

    /// The binding a declaration at `path` would displace, if any.
    ///
    /// The topmost binding under `path` is reported, so a declaration over a
    /// prelude module whose own path carries no binding still sees the
    /// namespace it is taking over.
    fn displaced_binding(
        &self,
        path: &NamePath,
    ) -> Option<Binding<Recognized, RecognitionSite>>
    {
        self.scope
            .visible()
            .iter()
            .find(|entry| entry.0.strip_prefix(path).is_some())
            .map(|entry| entry.1.clone())
    }
}

/// The one-segment path of a module or top-level name.
#[inline]
#[must_use]
pub fn namespace_path(name: ScopeSegment<'_>) -> NamePath
{
    NamePath::from_segments(Vec::from([Segment::from(name.0.to_owned())]))
}

/// The two-segment path of a `module.member` selection.
#[inline]
#[must_use]
pub fn member_path(
    module: ScopeSegment<'_>,
    member: ScopeSegment<'_>,
) -> NamePath
{
    NamePath::from_segments(Vec::from([
        Segment::from(module.0.to_owned()),
        Segment::from(member.0.to_owned()),
    ]))
}

/// Renders a scope failure as the rejection a caller reports.
///
/// [`Scope`] wraps a refusal in [`crate::namespace::ScopeError`]; the only
/// other variant is a missing section, which recognition never opens, so it is
/// re-stated as a rejection at the path rather than given a second error path
/// nothing can reach.
fn rejection_of(
    error: crate::namespace::ScopeError,
    path: &NamePath,
) -> EventRejection
{
    match error {
        | crate::namespace::ScopeError::Rejected(rejection) => rejection,
        | crate::namespace::ScopeError::NoOpenSection => EventRejection::new(
            EventKind::Shadow,
            path.clone(),
            RejectionReason::from("the recognition scope opened no section"),
        ),
    }
}
