//! The handler seam — the three elaboration-time namespace events.
//!
//! The engine core is deliberately oblivious to policy. Everything it would
//! otherwise have to decide is performed as one of three events and settled by
//! a [`NamespaceEventHandler`] the caller installs:
//!
//! | event | performed when | the policy question it asks |
//! | --- | --- | --- |
//! | not-found | an emptiness check found nothing | is a selection that matched nothing a typo, and is a typo fatal? |
//! | shadow | a union found two bindings on one path | which binding survives, and is a collision permitted at all? |
//! | hook | a `hook` constructor was reached | what does this labelled extension point do? |
//!
//! The design record states these as algebraic effects, installed by the
//! embedding language at the top of a run and interposable locally. Rust has
//! no effect handlers, so the same surface is a trait with three methods — the
//! shape the design record names for a language without them. Two consequences
//! are worth stating because they are not incidental:
//!
//! - **Policy is not in the engine.** Warn-and-allow on a collision and reject
//!   on a collision are two handlers, not two engine modes; a typo guard on
//!   not-found is a handler that consults a spell-checker the engine never
//!   sees.
//! - **A handler carries its own state.** The design record's optional
//!   user-context argument — there so a warning can say *which* import matched
//!   nothing — is `&mut self` here, which is strictly more general: the handler
//!   owns whatever it needs, including an event log.
//!
//! [`PermissiveHandler`] is the warn-and-allow default: it records every event
//! and rejects none, resolving each collision the design record's way (later
//! shadows earlier). A rejecting policy is a different implementation of the
//! same trait; nothing in the engine changes.

use alloc::string::String;
use alloc::vec::Vec;

use crate::namespace::path::NamePath;
use crate::namespace::trie::Binding;
use crate::namespace::trie::Collision;
use crate::namespace::trie::Trie;

/// Which of the three events a rejection refused.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventKind
{
    /// An emptiness check found nothing under the reported path.
    NotFound,
    /// A union found two bindings on the reported path.
    Shadow,
    /// A `hook` constructor reached the reported path.
    Hook,
}

impl core::fmt::Display for EventKind
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        let text = match *self {
            | Self::NotFound => "not-found",
            | Self::Shadow => "shadow",
            | Self::Hook => "hook",
        };
        f.write_str(text)
    }
}

/// A handler's explanation for refusing an event.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RejectionReason(String);

impl From<&str> for RejectionReason
{
    #[inline]
    fn from(text: &str) -> Self
    {
        Self(String::from(text))
    }
}

impl From<String> for RejectionReason
{
    #[inline]
    fn from(text: String) -> Self
    {
        Self(text)
    }
}

impl AsRef<str> for RejectionReason
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_str()
    }
}

impl core::fmt::Display for RejectionReason
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0.as_str())
    }
}

/// A handler refused an event, aborting the run that performed it.
///
/// A rejection is how a policy says "no" — the engine has no opinion about
/// whether a collision or a missing selection is fatal, so it propagates the
/// refusal unchanged and stops.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
#[error("the {kind} event at `{path}` was rejected: {reason}")]
pub struct EventRejection
{
    /// The refused event.
    kind: EventKind,
    /// The path the event was performed at.
    path: NamePath,
    /// The handler's explanation.
    reason: RejectionReason,
}

impl EventRejection
{
    /// Records a handler's refusal of an event at a path.
    #[inline]
    #[must_use]
    pub const fn new(
        kind: EventKind,
        path: NamePath,
        reason: RejectionReason,
    ) -> Self
    {
        Self { kind, path, reason }
    }

    /// The refused event.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> EventKind
    {
        self.kind
    }

    /// The path the refused event was performed at.
    #[inline]
    #[must_use]
    pub const fn path(&self) -> &NamePath
    {
        &self.path
    }

    /// The handler's explanation.
    #[inline]
    #[must_use]
    pub const fn reason(&self) -> &RejectionReason
    {
        &self.reason
    }
}

/// One performed event, as a permissive handler records it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NamespaceEvent<Label>
{
    /// An emptiness check at `path` found nothing — the design record's typo
    /// signal.
    NotFound
    {
        /// The path whose subtree was empty.
        path: NamePath,
    },
    /// A union found two bindings on `path`.
    Shadow
    {
        /// The colliding path.
        path: NamePath,
    },
    /// The hook labelled `label` ran on the namespace at `path`.
    Hook
    {
        /// The prefix the hook ran under.
        path: NamePath,
        /// The hook's label.
        label: Label,
    },
}

/// The policy seam: what the three elaboration-time namespace events mean.
///
/// `Data` and `Tag` are the carrier's binding components, so one handler can
/// serve any binding payload. `Label` is an associated type because a handler
/// is what gives hook labels meaning — a handler that understands one label
/// vocabulary cannot serve another.
pub trait NamespaceEventHandler<Data, Tag>
{
    /// The hook vocabulary this handler interprets.
    type Label;

    /// An emptiness check at `path` found nothing.
    ///
    /// # Contract
    /// - ensures: returning `Ok` lets the run continue with the namespace
    ///   unchanged; the event is a signal, not a transformation.
    /// - fails: returning an [`EventRejection`] aborts the run.
    ///
    /// # Errors
    /// Returns [`EventRejection`] when the policy treats a selection that
    /// matched nothing as fatal.
    fn not_found(
        &mut self,
        path: &NamePath,
    ) -> Result<(), EventRejection>;

    /// A union found two bindings on `path`.
    ///
    /// # Contract
    /// - ensures: the returned binding is the one that survives at `path`; a
    ///   handler may return either side of the collision or a binding it
    ///   synthesized.
    /// - fails: returning an [`EventRejection`] aborts the run.
    ///
    /// # Errors
    /// Returns [`EventRejection`] when the policy forbids the collision.
    fn shadow(
        &mut self,
        path: &NamePath,
        collision: Collision<Data, Tag>,
    ) -> Result<Binding<Data, Tag>, EventRejection>;

    /// The hook labelled `label` reached the namespace `subject` at `path`.
    ///
    /// # Contract
    /// - ensures: the returned namespace replaces `subject` in the run; the
    ///   identity is a valid interpretation of any label.
    /// - fails: returning an [`EventRejection`] aborts the run.
    ///
    /// # Errors
    /// Returns [`EventRejection`] when the policy does not recognize `label`
    /// or refuses to run it here.
    fn hook(
        &mut self,
        path: &NamePath,
        label: &Self::Label,
        subject: Trie<Data, Tag>,
    ) -> Result<Trie<Data, Tag>, EventRejection>;
}

/// The warn-and-allow policy: record every event, reject none.
///
/// Collisions resolve the design record's way — later shadows earlier — and
/// hooks are the identity, so a run under this handler always succeeds and the
/// recorded [`NamespaceEvent`]s are what a diagnostic layer reports.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissiveHandler<Label>
{
    /// Every event performed since construction, in the order performed.
    events: Vec<NamespaceEvent<Label>>,
}

impl<Label> Default for PermissiveHandler<Label>
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl<Label> PermissiveHandler<Label>
{
    /// A handler with no recorded events.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        Self { events: Vec::new() }
    }

    /// Every event performed so far, in the order performed.
    #[inline]
    #[must_use]
    pub fn events(&self) -> &[NamespaceEvent<Label>]
    {
        self.events.as_slice()
    }

    /// Forgets the recorded events, keeping the handler usable.
    #[inline]
    pub fn clear(&mut self)
    {
        self.events.clear();
    }
}

impl<Data, Tag, Label> NamespaceEventHandler<Data, Tag> for PermissiveHandler<Label>
where
    Label: Clone,
{
    type Label = Label;

    #[inline]
    fn not_found(
        &mut self,
        path: &NamePath,
    ) -> Result<(), EventRejection>
    {
        self.events
            .push(NamespaceEvent::NotFound { path: path.clone() });
        Ok(())
    }

    #[inline]
    fn shadow(
        &mut self,
        path: &NamePath,
        collision: Collision<Data, Tag>,
    ) -> Result<Binding<Data, Tag>, EventRejection>
    {
        self.events
            .push(NamespaceEvent::Shadow { path: path.clone() });
        Ok(collision.latter)
    }

    #[inline]
    fn hook(
        &mut self,
        path: &NamePath,
        label: &Label,
        subject: Trie<Data, Tag>,
    ) -> Result<Trie<Data, Tag>, EventRejection>
    {
        self.events.push(NamespaceEvent::Hook {
            path: path.clone(),
            label: label.clone(),
        });
        Ok(subject)
    }
}
