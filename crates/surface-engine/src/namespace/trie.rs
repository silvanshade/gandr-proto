//! The ordered-map trie carrier — hierarchical names to bindings.
//!
//! A namespace is a finite map from [`NamePath`] to a [`Binding`] (payload
//! plus tag). The design record's central move is that **namespaces are
//! implicit**: there is no namespace object, so `nat` names nothing on its own
//! — it is exactly the bindings whose path starts with `nat` — and `nat` and
//! `nat.plus` are independent bindings that coexist.
//!
//! # Representation, and why it is a trie
//!
//! The carrier is one [`BTreeMap`] keyed by the whole path, not a tree of
//! per-segment nodes. The two representations agree because
//! [`NamePath`]'s lexicographic order makes the extensions of a path
//! order-convex ([`crate::namespace::path`]): a subtree is exactly a
//! contiguous run of keys, so "the subtree at `p`" is a well-defined
//! operation on the flat map and the prefix relation *is* the trie structure.
//!
//! Two properties earned the choice, and both are contract-level rather than
//! micro-optimizations:
//!
//! - **Union is pointwise on full paths, structurally.** The design record
//!   requires that union merge subtrees recursively rather than let a namespace
//!   shadow wholesale. In a node tree that is an algorithm one can get wrong;
//!   in a flat path-keyed map two bindings collide only when their *whole
//!   paths* are equal, so the recursive merge is the representation's own
//!   behaviour and cannot drift.
//! - **No recursion anywhere.** A node tree is a recursive owned type, so its
//!   traversals, its derived `Clone`/`PartialEq`, and its drop glue all recur
//!   at a depth the user controls. Every operation here is a single pass or an
//!   ordered-map lookup, which is what the repository's no-unbounded-recursion
//!   rule asks for.
//!
//! The cost is that a subtree operation is a linear pass over the namespace
//! rather than a descent to a node. Namespaces are elaboration-time scope
//! tables, so the pass is over tens to thousands of bindings; the exchange
//! buys the two properties above.

use alloc::collections::BTreeMap;
use alloc::collections::btree_map;

use crate::namespace::path::NamePath;

/// One binding: the elaboration-facing payload a path reaches, plus the
/// metadata the carrier moves with it.
///
/// The tag is the design record's second component of a binding. It is opaque
/// to every operation here — the carrier relocates and merges it but never
/// reads it — so a consumer chooses what it carries (a source location, a
/// visibility marker, `()` when nothing is needed).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Binding<Data, Tag>
{
    /// The elaboration-facing payload the path resolves to.
    pub data: Data,
    /// The metadata retained alongside the payload.
    pub tag: Tag,
}

impl<Data, Tag> Binding<Data, Tag>
{
    /// Pairs a payload with its tag.
    #[inline]
    pub const fn new(
        data: Data,
        tag: Tag,
    ) -> Self
    {
        Self { data, tag }
    }
}

/// The two bindings a union found on one path.
///
/// `former` is the binding already accumulated; `latter` is the one arriving
/// from the branch being folded in. The design record's default resolution is
/// later-shadows-earlier, but the choice belongs to the handler, which is why
/// both are handed over intact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Collision<Data, Tag>
{
    /// The binding already present at the colliding path.
    pub former: Binding<Data, Tag>,
    /// The binding arriving at the colliding path.
    pub latter: Binding<Data, Tag>,
}

/// Whether a namespace holds any binding at all.
///
/// The emptiness check is a first-class constructor of the modifier language,
/// so its result is a named value rather than a bare predicate.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Emptiness(pub bool);

impl Emptiness
{
    /// The namespace holds no binding.
    pub const EMPTY: Self = Self(true);
    /// The namespace holds at least one binding.
    pub const OCCUPIED: Self = Self(false);
}

/// The number of bindings in a namespace.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingCount(pub usize);

/// A namespace: the finite trie from hierarchical names to bindings.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trie<Data, Tag>
{
    /// The bindings, keyed by whole path and ordered so that every subtree is
    /// one contiguous run.
    entries: BTreeMap<NamePath, Binding<Data, Tag>>,
}

impl<Data, Tag> Default for Trie<Data, Tag>
{
    #[inline]
    fn default() -> Self
    {
        Self::empty()
    }
}

impl<Data, Tag> Trie<Data, Tag>
{
    /// The namespace with no bindings.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self
    {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Binds `path`, returning whatever it displaced.
    ///
    /// # Contract
    /// - ensures: a later insert at the same path replaces the earlier binding
    ///   and hands it back; no [`Collision`] is reported, because a direct
    ///   insert is not a union.
    /// - provides: the primitive every namespace is built from.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (whether a path was already
    ///   bound), separated by the boundary pair fresh-path / repeat-path with
    ///   the returned `Option` asserted as an exact value on each.
    /// - witness: `trie::tests::inserting_returns_the_binding_it_displaced`
    #[inline]
    pub fn insert(
        &mut self,
        path: NamePath,
        binding: Binding<Data, Tag>,
    ) -> Option<Binding<Data, Tag>>
    {
        self.entries.insert(path, binding)
    }

    /// The binding at `path`, if any.
    ///
    /// This is the resolution primitive: an exact whole-path lookup. Because
    /// namespaces are implicit, a path with bindings *below* it and none *at*
    /// it resolves to nothing.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        path: &NamePath,
    ) -> Option<&Binding<Data, Tag>>
    {
        self.entries.get(path)
    }

    /// Every binding, in ascending path order.
    #[inline]
    pub fn iter(&self) -> btree_map::Iter<'_, NamePath, Binding<Data, Tag>>
    {
        self.entries.iter()
    }

    /// The number of bindings.
    #[inline]
    #[must_use]
    pub fn binding_count(&self) -> BindingCount
    {
        BindingCount(self.entries.len())
    }

    /// Whether the namespace is empty — the emptiness check the modifier
    /// language's `assert-nonempty` constructor performs on.
    #[inline]
    #[must_use]
    pub fn emptiness(&self) -> Emptiness
    {
        Emptiness(self.entries.is_empty())
    }

    /// Removes the subtree at `prefix` and returns it, rebased to the root.
    ///
    /// # Contract
    /// - ensures: the returned namespace holds exactly the bindings whose path
    ///   extended `prefix`, each keyed by the remainder after `prefix`; this
    ///   namespace retains exactly the rest.
    /// - ensures: detaching at the root empties this namespace and returns all
    ///   of it, because the root prefixes every path.
    /// - provides: the source half of `renaming` and the working namespace of
    ///   `in`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (the prefix test, already
    ///   L3-witnessed in [`crate::namespace::path`]) plus the partition's two
    ///   sides, separated by the boundary inputs root-prefix, absent-prefix,
    ///   and a prefix with both matching and non-matching siblings, each
    ///   asserted as an exact binding set.
    /// - witness: `trie::tests::detaching_at_the_root_takes_everything`
    /// - witness: `trie::tests::detaching_rebases_and_leaves_the_rest`
    /// - witness: `trie::tests::detaching_an_absent_prefix_yields_the_empty_namespace`
    #[inline]
    #[must_use]
    pub fn detach_subtree(
        &mut self,
        prefix: &NamePath,
    ) -> Self
    {
        let mut detached: BTreeMap<NamePath, Binding<Data, Tag>> = BTreeMap::new();
        let mut retained: BTreeMap<NamePath, Binding<Data, Tag>> = BTreeMap::new();
        for (path, binding) in core::mem::take(&mut self.entries) {
            match path.strip_prefix(prefix) {
                | Some(suffix) => {
                    detached.insert(suffix, binding);
                },
                | None => {
                    retained.insert(path, binding);
                },
            }
        }
        self.entries = retained;
        Self { entries: detached }
    }

    /// Grafts `subtree` at `prefix`, dropping whatever was already there.
    ///
    /// # Contract
    /// - ensures: every binding previously under `prefix` is gone, and each
    ///   binding of `subtree` is rebound at its path prefixed by `prefix`.
    /// - ensures: grafting at the root discards this namespace entirely and
    ///   replaces it with `subtree` — the mechanism behind `renaming p .`
    ///   keeping only the subtree at `p`.
    /// - provides: the target half of `renaming`, and the way `in` puts a
    ///   modified subtree back.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — two decision surfaces (dropping the target, and
    ///   the rebasing direction), separated by the boundary inputs
    ///   graft-at-root, graft-over-an-occupied-target, and
    ///   graft-beside-a-sibling, each asserted as an exact binding set.
    /// - witness: `trie::tests::grafting_at_the_root_replaces_everything`
    /// - witness: `trie::tests::grafting_drops_whatever_was_at_the_target`
    /// - witness: `trie::tests::grafting_keeps_bindings_outside_the_target`
    #[inline]
    pub fn graft_subtree(
        &mut self,
        prefix: &NamePath,
        subtree: Self,
    )
    {
        let mut retained: BTreeMap<NamePath, Binding<Data, Tag>> = BTreeMap::new();
        for (path, binding) in core::mem::take(&mut self.entries) {
            if path.strip_prefix(prefix).is_none() {
                retained.insert(path, binding);
            }
        }
        for (path, binding) in subtree.entries {
            retained.insert(path.prefixed_by(prefix), binding);
        }
        self.entries = retained;
    }

    /// This namespace with every path moved under `prefix`.
    #[inline]
    #[must_use]
    pub fn into_prefixed(
        self,
        prefix: &NamePath,
    ) -> Self
    {
        let mut prefixed: BTreeMap<NamePath, Binding<Data, Tag>> = BTreeMap::new();
        for (path, binding) in self.entries {
            prefixed.insert(path.prefixed_by(prefix), binding);
        }
        Self { entries: prefixed }
    }

    /// Merges `later` into this namespace, asking `resolve` to settle each
    /// collision.
    ///
    /// # Contract
    /// - requires: `resolve` returns the binding that survives at the path it
    ///   is handed; the carrier imposes no default.
    /// - ensures: the merge is pointwise on whole paths, so two bindings
    ///   collide only when their paths are equal and a namespace never shadows
    ///   another wholesale.
    /// - ensures: collisions are presented in ascending path order.
    /// - fails: propagates `resolve`'s failure unchanged. The merge is not
    ///   atomic — every binding of `later` ordered before the refused path has
    ///   already been merged and stays merged — but it is never destructive:
    ///   the binding this namespace held at the refused path survives
    ///   unchanged, so a declined collision loses nothing that was here.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns whatever `resolve` returns on the first collision it declines
    /// to settle.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — four decision surfaces (the collision test, the
    ///   resolved binding's placement, the early return, and what the early
    ///   return leaves behind), separated by the boundary inputs
    ///   disjoint-namespaces, one-colliding-path with a resolver that picks
    ///   each side in turn, and a declining resolver reached after an earlier
    ///   non-colliding binding has merged, each asserted as an exact binding
    ///   set or an exact error value.
    /// - witness: `trie::tests::union_of_disjoint_namespaces_merges_pointwise`
    /// - witness: `trie::tests::union_consults_the_resolver_on_a_collision`
    /// - witness: `trie::tests::union_reports_collisions_in_path_order`
    /// - witness: `trie::tests::union_propagates_a_declining_resolver`
    /// - witness: `trie::tests::a_declined_collision_keeps_the_binding_it_found`
    #[inline]
    pub fn union_resolving<Resolve, Failure>(
        &mut self,
        later: Self,
        resolve: &mut Resolve,
    ) -> Result<(), Failure>
    where
        Data: Clone,
        Tag: Clone,
        Resolve: FnMut(&NamePath, Collision<Data, Tag>) -> Result<Binding<Data, Tag>, Failure>,
    {
        for (path, latter) in later.entries {
            let Some(former) = self.entries.get(&path)
            else {
                self.entries.insert(path, latter);
                continue;
            };
            let collision = Collision {
                former: former.clone(),
                latter,
            };
            let resolved = resolve(&path, collision)?;
            self.entries.insert(path, resolved);
        }
        Ok(())
    }
}

impl<'namespace, Data, Tag> IntoIterator for &'namespace Trie<Data, Tag>
{
    type IntoIter = btree_map::Iter<'namespace, NamePath, Binding<Data, Tag>>;
    type Item = (&'namespace NamePath, &'namespace Binding<Data, Tag>);

    #[inline]
    fn into_iter(self) -> Self::IntoIter
    {
        self.iter()
    }
}

impl<Data, Tag> FromIterator<(NamePath, Binding<Data, Tag>)> for Trie<Data, Tag>
{
    #[inline]
    fn from_iter<Source>(iter: Source) -> Self
    where
        Source: IntoIterator<Item = (NamePath, Binding<Data, Tag>)>,
    {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

/// Carrier unit tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::Binding;
    use super::BindingCount;
    use super::Collision;
    use super::Emptiness;
    use super::Trie;
    use crate::namespace::path::DottedName;
    use crate::namespace::path::NamePath;

    /// The payload a carrier test binds — a marker standing in for whatever an
    /// elaborator would resolve a path to.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct Payload(u32);

    /// One entry of a test namespace: a dotted path and the payload it binds.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Entry<'text>
    {
        /// The dotted rendering of the bound path.
        path: DottedName<'text>,
        /// The payload bound there.
        payload: Payload,
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

    #[test]
    fn the_empty_namespace_is_empty()
    {
        assert_eq!(
            Trie::<Payload, ()>::empty().emptiness(),
            Emptiness::EMPTY,
            "a fresh namespace holds nothing"
        );
        assert_eq!(
            namespace(&[entry("nat", Payload(1))]).emptiness(),
            Emptiness::OCCUPIED,
            "one binding is enough to occupy a namespace"
        );
    }

    #[test]
    fn a_path_and_its_extension_are_independent_bindings()
    {
        let subject = namespace(&[entry("nat", Payload(1)), entry("nat.plus", Payload(2))]);
        assert_eq!(
            subject.binding_count(),
            BindingCount(2),
            "namespaces are implicit, so `nat` and `nat.plus` coexist"
        );
        assert_eq!(
            subject.get(&path("nat")).map(|binding| binding.data),
            Some(Payload(1)),
            "the shorter path resolves on its own"
        );
    }

    #[test]
    fn inserting_returns_the_binding_it_displaced()
    {
        let mut subject = namespace(&[entry("nat", Payload(1))]);
        assert_eq!(
            subject.insert(path("nat.plus"), Binding::new(Payload(2), ())),
            None,
            "a fresh path displaces nothing"
        );
        assert_eq!(
            subject.insert(path("nat"), Binding::new(Payload(3), ())),
            Some(Binding::new(Payload(1), ())),
            "a repeat insert hands back exactly what it replaced"
        );
        assert_eq!(
            listing(&subject),
            expected(&[entry("nat", Payload(3)), entry("nat.plus", Payload(2))]),
            "and the later binding is what the path now reaches"
        );
    }

    #[test]
    fn detaching_at_the_root_takes_everything()
    {
        let mut subject = namespace(&[entry("a.x", Payload(1)), entry("b.y", Payload(2))]);
        let detached = subject.detach_subtree(&NamePath::root());
        assert_eq!(
            subject.emptiness(),
            Emptiness::EMPTY,
            "the root prefixes every path, so nothing is retained"
        );
        assert_eq!(
            listing(&detached),
            expected(&[entry("a.x", Payload(1)), entry("b.y", Payload(2))]),
            "detaching at the root rebases nothing"
        );
    }

    #[test]
    fn detaching_rebases_and_leaves_the_rest()
    {
        let mut subject = namespace(&[
            entry("nat.plus", Payload(1)),
            entry("nat.times", Payload(2)),
            entry("int.plus", Payload(3)),
        ]);
        let detached = subject.detach_subtree(&path("nat"));
        assert_eq!(
            listing(&detached),
            expected(&[entry("plus", Payload(1)), entry("times", Payload(2))]),
            "the detached subtree is rebased to the root"
        );
        assert_eq!(
            listing(&subject),
            expected(&[entry("int.plus", Payload(3))]),
            "bindings outside the prefix are retained in place"
        );
    }

    #[test]
    fn detaching_an_absent_prefix_yields_the_empty_namespace()
    {
        let mut subject = namespace(&[entry("nat.plus", Payload(1))]);
        let detached = subject.detach_subtree(&path("rational"));
        assert_eq!(
            detached.emptiness(),
            Emptiness::EMPTY,
            "an absent prefix detaches nothing"
        );
        assert_eq!(
            listing(&subject),
            expected(&[entry("nat.plus", Payload(1))]),
            "and leaves the namespace untouched"
        );
    }

    #[test]
    fn grafting_at_the_root_replaces_everything()
    {
        let mut subject = namespace(&[entry("a.x", Payload(1)), entry("b.y", Payload(2))]);
        subject.graft_subtree(&NamePath::root(), namespace(&[entry("c.z", Payload(3))]));
        assert_eq!(
            listing(&subject),
            expected(&[entry("c.z", Payload(3))]),
            "grafting at the root drops the whole target namespace"
        );
    }

    #[test]
    fn grafting_drops_whatever_was_at_the_target()
    {
        let mut subject = namespace(&[entry("lib.old", Payload(1)), entry("lib.kept", Payload(2))]);
        subject.graft_subtree(&path("lib"), namespace(&[entry("new", Payload(3))]));
        assert_eq!(
            listing(&subject),
            expected(&[entry("lib.new", Payload(3))]),
            "the target subtree is replaced, not merged"
        );
    }

    #[test]
    fn grafting_keeps_bindings_outside_the_target()
    {
        let mut subject = namespace(&[entry("lib.old", Payload(1)), entry("app.main", Payload(2))]);
        subject.graft_subtree(&path("lib"), namespace(&[entry("new", Payload(3))]));
        assert_eq!(
            listing(&subject),
            expected(&[entry("app.main", Payload(2)), entry("lib.new", Payload(3))]),
            "only the target subtree is affected"
        );
    }

    #[test]
    fn prefixing_moves_every_path()
    {
        let subject = namespace(&[entry("x", Payload(1)), entry("y.z", Payload(2))]);
        assert_eq!(
            listing(&subject.into_prefixed(&path("section"))),
            expected(&[
                entry("section.x", Payload(1)),
                entry("section.y.z", Payload(2)),
            ]),
            "prefixing is what a section's export undergoes at close"
        );
    }

    #[test]
    fn union_of_disjoint_namespaces_merges_pointwise()
    {
        let mut subject = namespace(&[entry("a.x", Payload(1))]);
        let mut collisions: Vec<String> = Vec::new();
        let outcome: Result<(), ()> = subject.union_resolving(
            namespace(&[entry("a.y", Payload(2))]),
            &mut |path, collision| {
                collisions.push(alloc::format!("{path}"));
                Ok(collision.latter)
            },
        );
        assert_eq!(outcome, Ok(()), "a disjoint union cannot fail");
        assert!(
            collisions.is_empty(),
            "sharing the prefix `a` is not a collision — only equal whole paths collide"
        );
        assert_eq!(
            listing(&subject),
            expected(&[entry("a.x", Payload(1)), entry("a.y", Payload(2))]),
            "the implicit namespace `a` is merged, not shadowed"
        );
    }

    #[test]
    fn union_consults_the_resolver_on_a_collision()
    {
        for (choose_latter, survivor) in [(true, Payload(2)), (false, Payload(1))] {
            let mut subject = namespace(&[entry("a.x", Payload(1))]);
            let outcome: Result<(), ()> = subject.union_resolving(
                namespace(&[entry("a.x", Payload(2))]),
                &mut |_, collision| {
                    Ok(if choose_latter {
                        collision.latter
                    }
                    else {
                        collision.former
                    })
                },
            );
            assert_eq!(outcome, Ok(()), "an accepted collision cannot fail");
            assert_eq!(
                listing(&subject),
                expected(&[entry("a.x", survivor)]),
                "the resolver, not the carrier, chooses the survivor"
            );
        }
    }

    #[test]
    fn union_reports_collisions_in_path_order()
    {
        let mut subject = namespace(&[entry("b", Payload(1)), entry("a", Payload(2))]);
        let mut collisions: Vec<String> = Vec::new();
        let outcome: Result<(), ()> = subject.union_resolving(
            namespace(&[entry("b", Payload(3)), entry("a", Payload(4))]),
            &mut |path, collision| {
                collisions.push(alloc::format!("{path}"));
                Ok(collision.latter)
            },
        );
        assert_eq!(outcome, Ok(()), "both collisions are accepted");
        assert_eq!(
            collisions,
            Vec::from([String::from("a"), String::from("b")]),
            "collisions arrive in ascending path order, not insertion order"
        );
    }

    #[test]
    fn union_propagates_a_declining_resolver()
    {
        let mut subject = namespace(&[entry("a.x", Payload(1))]);
        let outcome: Result<(), Collision<Payload, ()>> = subject.union_resolving(
            namespace(&[entry("a.x", Payload(2))]),
            &mut |_, collision| Err(collision),
        );
        assert_eq!(
            outcome,
            Err(Collision {
                former: Binding::new(Payload(1), ()),
                latter: Binding::new(Payload(2), ()),
            }),
            "the declining resolver's exact value reaches the caller"
        );
    }

    #[test]
    fn a_declined_collision_keeps_the_binding_it_found()
    {
        let mut subject = namespace(&[entry("m.b", Payload(1))]);
        let outcome: Result<(), Collision<Payload, ()>> = subject.union_resolving(
            namespace(&[entry("m.a", Payload(2)), entry("m.b", Payload(3))]),
            &mut |_, collision| Err(collision),
        );
        assert_eq!(
            outcome,
            Err(Collision {
                former: Binding::new(Payload(1), ()),
                latter: Binding::new(Payload(3), ()),
            }),
            "the resolver declines the collision at `m.b`, and only that one"
        );
        assert_eq!(
            listing(&subject),
            expected(&[entry("m.a", Payload(2)), entry("m.b", Payload(1))]),
            "the merge stops where it was refused, keeping what it had already merged and — the \
             point of the case — never dropping the binding whose collision was refused"
        );
    }
}
