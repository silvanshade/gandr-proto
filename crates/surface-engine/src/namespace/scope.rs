//! The scope value — lexical scoping as two namespaces.
//!
//! A [`Scope`] carries two namespaces at once:
//!
//! - **visible** — what resolves *here*, in the unit being elaborated;
//! - **export** — what an importer of this unit will see.
//!
//! Keeping them apart is what makes "in scope here" and "re-exported"
//! independent axes, and every operation below is characterised by which of
//! the two it touches:
//!
//! | operation | visible | export |
//! | --- | --- | --- |
//! | [`Scope::include_subtree`] | yes | yes |
//! | [`Scope::import_subtree`] | yes | no |
//! | [`Scope::modify_visible`] | yes | no |
//! | [`Scope::modify_export`] | no | yes |
//! | [`Scope::export_visible`] | no | yes |
//! | [`Scope::end_section`] | yes | yes |
//!
//! # Sections
//!
//! A section is a child scope. [`Scope::begin_section`] opens one that
//! **inherits the parent's visible namespace** — so definitions inside it can
//! see what surrounds them — and starts with an **empty export**.
//! [`Scope::end_section`] runs a modifier over the child's export, prefixes the
//! result, and includes it into the parent.
//!
//! The consequence is the re-export discipline: a section's *imports* touch
//! only its visible namespace, and its visible namespace is discarded at close,
//! so they evaporate; only what the section exported survives, under the
//! section's prefix. That is the mechanism behind Coq-style sections and the
//! reason a private definition is a binding in a section that is never
//! exported.

use alloc::vec::Vec;

use crate::namespace::event::EventRejection;
use crate::namespace::event::NamespaceEventHandler;
use crate::namespace::modifier::Modifier;
use crate::namespace::path::NamePath;
use crate::namespace::trie::Binding;
use crate::namespace::trie::Trie;

/// A structured scope-operation failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ScopeError
{
    /// A handler refused one of the three namespace events.
    #[error(transparent)]
    Rejected(#[from] EventRejection),
    /// A section was closed while no section was open.
    #[error("no open section to close")]
    NoOpenSection,
}

/// The result type of a scope operation.
pub type ScopeResult<T> = Result<T, ScopeError>;

/// One scope's pair of namespaces.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Namespaces<Data, Tag>
{
    /// What resolves here.
    visible: Trie<Data, Tag>,
    /// What an importer of this unit will see.
    export: Trie<Data, Tag>,
}

impl<Data, Tag> Namespaces<Data, Tag>
{
    /// A pair with both namespaces empty.
    fn empty() -> Self
    {
        Self {
            visible: Trie::empty(),
            export: Trie::empty(),
        }
    }
}

/// A lexical scope: a visible namespace, an export namespace, and the stack of
/// sections enclosing them.
///
/// The current scope is a field rather than the top of a stack, so "there is
/// always a current scope" is a fact about the type instead of an invariant
/// the operations have to re-establish.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope<Data, Tag>
{
    /// The innermost scope — the one every operation acts on.
    current: Namespaces<Data, Tag>,
    /// The enclosing scopes, outermost first; non-empty exactly when a section
    /// is open.
    enclosing: Vec<Namespaces<Data, Tag>>,
}

impl<Data, Tag> Default for Scope<Data, Tag>
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl<Data, Tag> Scope<Data, Tag>
{
    /// A scope with both namespaces empty and no section open.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            current: Namespaces::empty(),
            enclosing: Vec::new(),
        }
    }

    /// A scope whose visible namespace starts as `init_visible`.
    ///
    /// This is the **namespace-graduation shape**: the prelude, host, and
    /// extern tables become an initial visible namespace rather than a
    /// name-table check in projection position, and a user declaration that
    /// reaches the same path becomes a shadow event with a policy instead of a
    /// syntactic prohibition. See [`crate::namespace`] for what that graduation
    /// would and would not change.
    ///
    /// # Contract
    /// - ensures: the visible namespace is exactly `init_visible`; the export
    ///   namespace is empty, because a unit does not re-export the prelude it
    ///   was elaborated against merely by being elaborated against it.
    /// - provides: the entry point a graduated reserved-namespace resolution
    ///   would use.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which namespace the
    ///   initial trie lands in), separated by asserting both namespaces exactly
    ///   on a non-empty initial trie.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `init_visible_seeds_only_the_visible_namespace`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_user_binding_over_the_prelude_is_a_shadow_event`
    #[inline]
    #[must_use]
    pub fn with_init_visible(init_visible: Trie<Data, Tag>) -> Self
    {
        Self {
            current: Namespaces {
                visible: init_visible,
                export: Trie::empty(),
            },
            enclosing: Vec::new(),
        }
    }

    /// What resolves in this scope.
    #[inline]
    #[must_use]
    pub const fn visible(&self) -> &Trie<Data, Tag>
    {
        &self.current.visible
    }

    /// What an importer of this scope will see.
    #[inline]
    #[must_use]
    pub const fn export(&self) -> &Trie<Data, Tag>
    {
        &self.current.export
    }

    /// Resolves a hierarchical name against the visible namespace.
    ///
    /// This is the elaboration boundary's lookup: a path in, a binding out.
    /// What the binding's payload *is* — an identity, a location, a table
    /// entry — is the caller's business; nothing in this layer inspects it.
    #[inline]
    #[must_use]
    pub fn resolve(
        &self,
        path: &NamePath,
    ) -> Option<&Binding<Data, Tag>>
    {
        self.current.visible.get(path)
    }
}

impl<Data, Tag> Scope<Data, Tag>
where
    Data: Clone,
    Tag: Clone,
{
    /// Merges `subtree` under `prefix` into **both** namespaces.
    ///
    /// # Contract
    /// - ensures: the visible and export namespaces each receive `subtree`
    ///   prefixed by `prefix`, merged pointwise, with every collision settled
    ///   by `handler`.
    /// - provides: the `include` operation — the one that makes a binding both
    ///   usable here and visible to importers.
    /// - fails: propagates a handler rejection from either merge; the visible
    ///   merge runs first, so a rejection there leaves the export namespace
    ///   untouched. Neither merge is atomic: a rejected collision abandons the
    ///   merge in place, so the namespace being merged when the rejection
    ///   arrived keeps every binding it had already taken. A caller that needs
    ///   an all-or-nothing include holds its own copy.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::Rejected`] when `handler` refuses a collision.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (which namespaces are
    ///   touched, and in which order), the first separated from
    ///   [`Self::import_subtree`] by asserting both namespaces exactly after
    ///   each operation on the same input, the second by a rejecting handler
    ///   refusing a collision that follows a non-colliding binding in path
    ///   order, with both namespaces then asserted exactly.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `include_touches_both_namespaces`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `import_touches_only_the_visible_namespace`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `include_merges_the_visible_namespace_before_the_export`
    #[inline]
    pub fn include_subtree<Handler>(
        &mut self,
        prefix: &NamePath,
        subtree: Trie<Data, Tag>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let prefixed = subtree.into_prefixed(prefix);
        self.current
            .visible
            .union_resolving(prefixed.clone(), &mut |path, collision| {
                handler.shadow(path, collision)
            })?;
        self.current
            .export
            .union_resolving(prefixed, &mut |path, collision| {
                handler.shadow(path, collision)
            })?;
        Ok(())
    }

    /// Merges `subtree` under `prefix` into the **visible** namespace only.
    ///
    /// # Contract
    /// - ensures: the export namespace is unchanged, so an import is not a
    ///   re-export.
    /// - provides: the `import` operation.
    /// - fails: propagates a handler rejection.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::Rejected`] when `handler` refuses a collision.
    ///
    /// # Adequacy
    /// - hypothesis: shares [`Self::include_subtree`]'s hypothesis — the two
    ///   are separated by the same pair of witnesses.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `import_touches_only_the_visible_namespace`
    #[inline]
    pub fn import_subtree<Handler>(
        &mut self,
        prefix: &NamePath,
        subtree: Trie<Data, Tag>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let prefixed = subtree.into_prefixed(prefix);
        self.current
            .visible
            .union_resolving(prefixed, &mut |path, collision| {
                handler.shadow(path, collision)
            })?;
        Ok(())
    }

    /// Replaces the visible namespace with `modifier`'s result on it.
    ///
    /// # Contract
    /// - ensures: the export namespace is unchanged.
    /// - fails: propagates a handler rejection, leaving both namespaces as they
    ///   were.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::Rejected`] when `handler` refuses an event the
    /// modifier performed.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which namespace is
    ///   rewritten), separated from [`Self::modify_export`] by asserting both
    ///   namespaces exactly after each.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `modifying_visible_leaves_export_alone`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `modifying_export_leaves_visible_alone`
    #[inline]
    pub fn modify_visible<Handler>(
        &mut self,
        modifier: &Modifier<Handler::Label>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let modified = modifier.apply(self.current.visible.clone(), handler)?;
        self.current.visible = modified;
        Ok(())
    }

    /// Replaces the export namespace with `modifier`'s result on it.
    ///
    /// # Contract
    /// - ensures: the visible namespace is unchanged.
    /// - fails: propagates a handler rejection, leaving both namespaces as they
    ///   were.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::Rejected`] when `handler` refuses an event the
    /// modifier performed.
    ///
    /// # Adequacy
    /// - hypothesis: shares [`Self::modify_visible`]'s hypothesis, with the
    ///   further surface of *which* namespace the modifier reads separated by
    ///   holding a binding in the visible namespace that the export namespace
    ///   does not have.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `modifying_export_leaves_visible_alone`
    #[inline]
    pub fn modify_export<Handler>(
        &mut self,
        modifier: &Modifier<Handler::Label>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let modified = modifier.apply(self.current.export.clone(), handler)?;
        self.current.export = modified;
        Ok(())
    }

    /// Copies the visible namespace through `modifier` into the export
    /// namespace.
    ///
    /// # Contract
    /// - ensures: the visible namespace is unchanged; the export namespace
    ///   receives the modifier's result, merged pointwise with what it already
    ///   held.
    /// - provides: the re-export control the design record's `export_visible`
    ///   names — a unit chooses which of the things it can see it passes on.
    /// - fails: propagates a handler rejection.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::Rejected`] when `handler` refuses an event the
    /// modifier or the merge performed.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — three decision surfaces (which namespace the
    ///   modifier reads, which namespace receives its result, and that the
    ///   result merges rather than replaces), separated by a selective modifier
    ///   over a visible namespace holding bindings the export namespace does
    ///   not, run against a non-empty prior export, asserting both namespaces
    ///   exactly.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `export_visible_re_exports_a_selection`
    #[inline]
    pub fn export_visible<Handler>(
        &mut self,
        modifier: &Modifier<Handler::Label>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let selected = modifier.apply(self.current.visible.clone(), handler)?;
        self.current
            .export
            .union_resolving(selected, &mut |path, collision| {
                handler.shadow(path, collision)
            })?;
        Ok(())
    }

    /// Opens a section: a child scope inheriting the visible namespace and
    /// starting with an empty export.
    ///
    /// # Contract
    /// - ensures: the visible namespace is carried into the child unchanged, so
    ///   the section can see its surroundings; the child's export starts empty.
    /// - ensures: the parent's namespaces are preserved and restored by
    ///   [`Self::end_section`].
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (which namespace the child
    ///   inherits, and that the child's export starts empty rather than
    ///   inheriting too), separated by opening a section over a scope whose
    ///   visible **and** export namespaces are both non-empty, then asserting
    ///   the child's two namespaces exactly.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_section_inherits_the_visible_namespace_and_exports_nothing_yet`
    #[inline]
    pub fn begin_section(&mut self)
    {
        let inherited = self.current.visible.clone();
        let parent = core::mem::replace(&mut self.current, Namespaces {
            visible: inherited,
            export: Trie::empty(),
        });
        self.enclosing.push(parent);
    }

    /// Closes the innermost section, including its export under `prefix`.
    ///
    /// # Contract
    /// - requires: a section is open.
    /// - ensures: the child's export is run through `modifier`, prefixed by
    ///   `prefix`, and included into the parent — reaching both the parent's
    ///   visible and export namespaces.
    /// - ensures: the child's *visible* namespace is discarded, so bindings the
    ///   section only imported evaporate at close and bindings it never
    ///   exported stay private.
    /// - fails: [`ScopeError::NoOpenSection`] when no section is open; a
    ///   handler rejection from the modifier or the merge. The section is
    ///   popped before the modifier runs, so a rejection from either leaves the
    ///   section closed and its export discarded rather than reopened.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`ScopeError::NoOpenSection`] when called outside a section, and
    /// [`ScopeError::Rejected`] when `handler` refuses an event.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — four decision surfaces (which namespace of the
    ///   child survives, the closing modifier, the prefixing, and the
    ///   missing-section guard), separated by a section that both imports and
    ///   exports, closed once at a prefix under the identity, once at a prefix
    ///   under a selective modifier over a two-binding export, and once with no
    ///   section open, asserting the exact namespaces and the exact error
    ///   variant.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_section_exports_under_its_prefix`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_sections_imports_evaporate_at_close`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_sections_closing_modifier_chooses_what_it_passes_on`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `closing_without_an_open_section_fails`
    #[inline]
    pub fn end_section<Handler>(
        &mut self,
        prefix: &NamePath,
        modifier: &Modifier<Handler::Label>,
        handler: &mut Handler,
    ) -> ScopeResult<()>
    where
        Handler: NamespaceEventHandler<Data, Tag>,
    {
        let Some(parent) = self.enclosing.pop()
        else {
            return Err(ScopeError::NoOpenSection);
        };
        let child_export = core::mem::replace(&mut self.current, parent).export;
        let selected = modifier.apply(child_export, handler)?;
        self.include_subtree(prefix, selected, handler)
    }
}
