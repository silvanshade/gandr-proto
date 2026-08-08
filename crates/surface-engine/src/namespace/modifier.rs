//! The name-modifier language and its interpreter.
//!
//! A **name modifier** is a term of a small language whose meaning is a
//! transformation of one namespace into another, run wherever a namespace
//! meets another: at an import, at an include, at the close of a section.
//!
//! # The six constructors
//!
//! The core is exactly six constructors, and every other builder is derived
//! from them:
//!
//! ```text
//! m ::= assert-nonempty     perform not-found when the namespace is empty
//!     | in p m              run m on the subtree at p, keep everything outside it
//!     | renaming p p'       move the subtree at p to p', dropping whatever was at p'
//!     | seq (m₁, …, mₙ)     run in order; seq () is the identity
//!     | union (m₁, …, mₙ)   run each on the same input, union the results
//!     | hook h              an extension point, performed as an event
//! ```
//!
//! The derived builders, each spelled out by [`Modifier`]'s constructors:
//!
//! | builder | derivation |
//! | --- | --- |
//! | [`Modifier::all`] | `assert-nonempty` |
//! | [`Modifier::id`] | `seq ()` |
//! | [`Modifier::none`] | `seq (assert-nonempty, union ())` |
//! | [`Modifier::only`] | `seq (in p assert-nonempty, renaming p ., renaming . p)` |
//! | [`Modifier::except`] | `in p none` |
//! | [`Modifier::renaming`] | `seq (in p assert-nonempty, renaming p p')` |
//!
//! **The emptiness check is deliberately a separate constructor**, and the
//! derivations above are where that pays. Folding the check into the core
//! relocation would make `only p` perform two not-found events when the subtree
//! at `p` is empty — one for `p`, which is the one a user wants, and a second
//! for the root, which names the whole namespace and is simply wrong. Keeping
//! the check separate lets each derived builder place exactly one, at exactly
//! the path a user would want named in the diagnostic.
//!
//! The split runs the other way too, and conflating the two directions is the
//! failure this module is arranged to prevent: [`Modifier::Renaming`] is the
//! **unchecked** core relocation, while [`Modifier::renaming`] is the checked
//! builder the modifier language means by `renaming p p'`. A `renaming` built
//! from the core constructor never reports a typo.
//!
//! # Three semantic decisions carry the weight
//!
//! * **Operations are subtree-grained.** `only nat` keeps every binding under
//!   `nat`, not one binding named `nat`. Per-binding modifiers were tried in
//!   the design record's implementation and found useless.
//! * **Renaming drops its target.** `renaming p p'` discards whatever was
//!   already under `p'`, so a client patching an API need not fear what a newer
//!   version of it added under the target name. The safety net is that a
//!   wrongly dropped binding fails later at scope checking, whereas a silently
//!   kept one changes meaning.
//! * **Union is recursive, and a conflict is an event.** Because namespaces are
//!   implicit, union merges pointwise rather than letting one namespace shadow
//!   another wholesale; where two bindings land on one path the shadow event
//!   decides. The design record's example, reproduced exactly by this
//!   interpreter: with `a.x` and `b.a.y` bound, `union (all, renaming b .)`
//!   yields `a.x`, `a.y`, and `b.a.y`, where an explicit-module `open` would
//!   have shadowed `a.x` away entirely.
//!
//! # The interpreter is iterative
//!
//! Modifier nesting depth is user-controlled, so the interpreter is an
//! explicit instruction stack over a single current namespace rather than a
//! recursive walk ([`Modifier::apply`]). The three instruction forms are the
//! defunctionalized continuations of the recursive reading: apply a modifier,
//! regraft a finished `in` subtree, and fold a finished `union` branch.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::namespace::event::EventRejection;
use crate::namespace::event::NamespaceEventHandler;
use crate::namespace::path::NamePath;
use crate::namespace::path::Segment;
use crate::namespace::trie::Emptiness;
use crate::namespace::trie::Trie;

/// A term of the name-modifier language.
///
/// The variants are the six core constructors; the derived builders are the
/// associated functions below, each of which expands to core constructors so
/// that one interpreter covers the whole language.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Modifier<Label>
{
    /// `assert-nonempty` — perform not-found when the namespace is empty, and
    /// otherwise change nothing.
    AssertNonEmpty,
    /// `in p m` — run `inner` on the subtree at `path`, keeping every binding
    /// outside it intact.
    In
    {
        /// The subtree the inner modifier runs on.
        path: NamePath,
        /// The modifier to run there.
        inner: Box<Self>,
    },
    /// `renaming p p'` — move the subtree at `source` to `target`, dropping
    /// whatever was already under `target`.
    ///
    /// This is the **core** relocation: it performs no emptiness check. The
    /// checked builder users normally want is [`Modifier::renaming`].
    Renaming
    {
        /// The subtree to relocate.
        source: NamePath,
        /// Where to put it.
        target: NamePath,
    },
    /// `seq (m₁, …, mₙ)` — run the modifiers in order, each on the previous
    /// one's result. The empty sequence is the identity.
    Seq(Vec<Self>),
    /// `union (m₁, …, mₙ)` — run each modifier on the same input namespace and
    /// union the results left to right, performing shadow on each collision.
    /// The empty union is the empty namespace.
    Union(Vec<Self>),
    /// `hook h` — hand the whole namespace to the handler under the label `h`.
    Hook(Label),
}

impl<Label> Modifier<Label>
{
    /// `all` — keep everything, performing not-found when the namespace is
    /// empty.
    #[inline]
    #[must_use]
    pub const fn all() -> Self
    {
        Self::AssertNonEmpty
    }

    /// `id` — keep everything, checking nothing. The identity of `seq`.
    #[inline]
    #[must_use]
    pub const fn id() -> Self
    {
        Self::Seq(Vec::new())
    }

    /// `none` — drop everything, performing not-found when the namespace was
    /// already empty.
    #[inline]
    #[must_use]
    pub fn none() -> Self
    {
        Self::Seq(Vec::from([Self::AssertNonEmpty, Self::Union(Vec::new())]))
    }

    /// `only p` — keep the subtree at `path` and drop everything else,
    /// performing not-found at `path` when that subtree is empty.
    #[inline]
    #[must_use]
    pub fn only(path: NamePath) -> Self
    {
        Self::Seq(Vec::from([
            Self::In {
                path: path.clone(),
                inner: Box::new(Self::AssertNonEmpty),
            },
            Self::Renaming {
                source: path.clone(),
                target: NamePath::root(),
            },
            Self::Renaming {
                source: NamePath::root(),
                target: path,
            },
        ]))
    }

    /// `except p` — drop the subtree at `path`, performing not-found at `path`
    /// when that subtree was already empty.
    #[inline]
    #[must_use]
    pub fn except(path: NamePath) -> Self
    {
        Self::In {
            path,
            inner: Box::new(Self::none()),
        }
    }

    /// `renaming p p'` — relocate the subtree at `source` to `target`,
    /// performing not-found at `source` when that subtree is empty.
    ///
    /// This is the checked builder. The unchecked core relocation is
    /// [`Modifier::Renaming`].
    #[inline]
    #[must_use]
    pub fn renaming(
        source: NamePath,
        target: NamePath,
    ) -> Self
    {
        Self::Seq(Vec::from([
            Self::In {
                path: source.clone(),
                inner: Box::new(Self::AssertNonEmpty),
            },
            Self::Renaming { source, target },
        ]))
    }

    /// `in p m` — run `inner` on the subtree at `path`.
    #[inline]
    #[must_use]
    pub fn in_subtree(
        path: NamePath,
        inner: Self,
    ) -> Self
    {
        Self::In {
            path,
            inner: Box::new(inner),
        }
    }

    /// `seq (…)` — run the modifiers in order.
    #[inline]
    #[must_use]
    pub const fn seq(modifiers: Vec<Self>) -> Self
    {
        Self::Seq(modifiers)
    }

    /// `union (…)` — run each modifier on the same input and union the
    /// results.
    #[inline]
    #[must_use]
    pub const fn union(modifiers: Vec<Self>) -> Self
    {
        Self::Union(modifiers)
    }

    /// `hook h` — the labelled extension point.
    #[inline]
    #[must_use]
    pub const fn hook(label: Label) -> Self
    {
        Self::Hook(label)
    }

    /// The modifier today's `import "URI" as name ;` clause means.
    ///
    /// # Contract
    /// - ensures: equal to `renaming . name`, that is [`Modifier::renaming`]
    ///   from the root to the one-segment path `name`.
    /// - provides: the executable form of the desugaring recorded in
    ///   [`crate::namespace`]; the import declaration's own lowering is not
    ///   part of this layer.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L0 plus L3 — the equality with the general builder is
    ///   asserted exactly, so any change to either side separates them
    ///   (including a change to the *unchecked* core constructor, which the
    ///   equality would then reject), the qualifying behaviour is witnessed on
    ///   a two-level namespace, and the emptiness check it inherits is
    ///   witnessed on the empty namespace.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `as_name_is_renaming_to_the_alias`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `as_name_qualifies_every_imported_path`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `as_name_on_an_empty_import_performs_not_found`
    #[inline]
    #[must_use]
    pub fn alias_as(alias: Segment) -> Self
    {
        Self::renaming(
            NamePath::root(),
            NamePath::from_segments(Vec::from([alias])),
        )
    }

    /// Runs this modifier on `subject`, settling every event through
    /// `handler`.
    ///
    /// # Contract
    /// - requires: `handler` interprets the same hook vocabulary this modifier
    ///   is labelled with.
    /// - ensures: the result is the design record's semantics of this modifier
    ///   — subtree-grained selection, target-dropping relocation, and pointwise
    ///   recursive union.
    /// - ensures: every not-found, shadow, and hook event reaches `handler`
    ///   with the accumulated prefix of the point that performed it, so an
    ///   event inside `in p m` reports a path under `p`.
    /// - provides: the whole language's meaning through one iterative
    ///   interpreter — the derived builders add no interpretation.
    /// - fails: propagates a handler's [`EventRejection`] unchanged, abandoning
    ///   the run at the point that performed the event.
    /// - intension: the interpreter never recurses — it is an explicit
    ///   instruction stack whose height is bounded by the modifier's nesting
    ///   depth. Its memory is not bounded by that depth alone: a suspended
    ///   `union` frame retains a clone of the namespace its branches run on
    ///   together with the accumulation so far, and a suspended `in` frame
    ///   retains the bindings outside the subtree, so nested unions cost depth
    ///   times namespace size in the worst case. Nothing about the traversal is
    ///   otherwise observable.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the [`EventRejection`] a handler produced for a not-found,
    /// shadow, or hook event.
    ///
    /// # Adequacy
    /// - hypothesis: L2 plus L3 — the derived builders are checked against the
    ///   design record's worked examples as an external oracle (selective
    ///   import, qualified import, the deep-patch union, re-export control,
    ///   typo resistance), and the residue is the per-constructor boundary set:
    ///   the empty namespace for `assert-nonempty`, the checked builder against
    ///   the unchecked core constructor on an absent source, the root as both
    ///   source and target of a relocation, the empty `seq` and empty `union`,
    ///   a `union` whose first branch is not the identity, and a nested `in`
    ///   whose not-found, shadow, and hook paths must each carry the outer
    ///   prefix.
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `only_keeps_the_named_subtree_and_drops_the_rest`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `except_drops_the_named_subtree`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `in_runs_the_inner_modifier_on_one_subtree`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `renaming_to_the_root_unqualifies`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `renaming_drops_whatever_was_at_the_target`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `the_checked_renaming_builder_performs_not_found_on_an_absent_source`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `the_core_relocation_performs_no_emptiness_check`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_rejecting_handler_refuses_a_missing_renaming_source`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_deep_patch_merges_instead_of_capturing`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `each_union_branch_runs_on_the_original_namespace`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `the_empty_sequence_is_the_identity`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `the_empty_union_is_the_empty_namespace`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_selection_that_matched_nothing_performs_not_found`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_nested_event_reports_the_accumulated_prefix`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_nested_shadow_reports_the_accumulated_prefix`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_nested_hook_reports_the_accumulated_prefix`
    /// - witness: `gandr-surface-engine` `tests/namespace.rs` —
    ///   `a_hook_can_replace_the_namespace`
    #[inline]
    pub fn apply<Data, Tag, Handler>(
        &self,
        subject: Trie<Data, Tag>,
        handler: &mut Handler,
    ) -> Result<Trie<Data, Tag>, EventRejection>
    where
        Data: Clone,
        Tag: Clone,
        Handler: NamespaceEventHandler<Data, Tag, Label = Label>,
    {
        let mut current = subject;
        let mut stack: Vec<Instruction<'_, Data, Tag, Label>> = Vec::from([Instruction::Apply {
            prefix: NamePath::root(),
            modifier: self,
        }]);

        while let Some(instruction) = stack.pop() {
            match instruction {
                | Instruction::Apply { prefix, modifier } => {
                    step_apply(&mut current, &mut stack, handler, prefix, modifier)?;
                },
                | Instruction::Regraft {
                    prefix,
                    mut outside,
                } => {
                    let inner = core::mem::take(&mut current);
                    outside.graft_subtree(&prefix, inner);
                    current = outside;
                },
                | Instruction::UnionFold {
                    prefix,
                    input,
                    mut accumulated,
                    mut remaining,
                } => {
                    let branch = core::mem::take(&mut current);
                    accumulated.union_resolving(branch, &mut |path, collision| {
                        let path = prefix.extended(path);
                        handler.shadow(&path, collision)
                    })?;
                    match remaining.next() {
                        | Some(next) => {
                            current = input.clone();
                            stack.push(Instruction::UnionFold {
                                prefix: prefix.clone(),
                                input,
                                accumulated,
                                remaining,
                            });
                            stack.push(Instruction::Apply {
                                prefix,
                                modifier: next,
                            });
                        },
                        | None => {
                            current = accumulated;
                        },
                    }
                },
            }
        }

        Ok(current)
    }
}

/// One step of the interpreter's explicit stack.
///
/// The three forms are the defunctionalized continuations of the recursive
/// reading of [`Modifier::apply`]: what is left to run, what to do with a
/// finished `in` subtree, and what to do with a finished `union` branch.
enum Instruction<'modifier, Data, Tag, Label>
{
    /// Run `modifier` on the current namespace, reporting events under
    /// `prefix`.
    Apply
    {
        /// The accumulated prefix events are reported under.
        prefix: NamePath,
        /// The modifier still to run.
        modifier: &'modifier Modifier<Label>,
    },
    /// The inner modifier of an `in` finished: put the current namespace back
    /// under `prefix` inside `outside`.
    Regraft
    {
        /// Where the inner namespace came from.
        prefix: NamePath,
        /// The bindings that were outside the subtree.
        outside: Trie<Data, Tag>,
    },
    /// A union branch finished: fold the current namespace into `accumulated`,
    /// then start the next branch on `input` or finish.
    UnionFold
    {
        /// The accumulated prefix events are reported under.
        prefix: NamePath,
        /// The namespace every branch of this union runs on.
        input: Trie<Data, Tag>,
        /// The union of the branches completed so far.
        accumulated: Trie<Data, Tag>,
        /// The branches not yet started.
        remaining: core::slice::Iter<'modifier, Modifier<Label>>,
    },
}

/// Runs one constructor of the modifier language against the current
/// namespace, pushing whatever continuations it needs.
///
/// # Contract
/// - ensures: exactly the design record's meaning of the constructor, with the
///   nested cases expressed as pushed [`Instruction`]s rather than recursion.
/// - fails: propagates the handler's [`EventRejection`] unchanged.
/// - panics: none.
///
/// # Errors
/// Returns the [`EventRejection`] the handler produced for the event this step
/// performed.
///
/// # Adequacy
/// - hypothesis: shares [`Modifier::apply`]'s hypothesis — this is that
///   method's step function and has no separately reachable behaviour.
/// - witness: `gandr-surface-engine` `tests/namespace.rs` —
///   `only_keeps_the_named_subtree_and_drops_the_rest`
/// - witness: `gandr-surface-engine` `tests/namespace.rs` —
///   `a_deep_patch_merges_instead_of_capturing`
#[inline]
fn step_apply<'modifier, Data, Tag, Label, Handler>(
    current: &mut Trie<Data, Tag>,
    stack: &mut Vec<Instruction<'modifier, Data, Tag, Label>>,
    handler: &mut Handler,
    prefix: NamePath,
    modifier: &'modifier Modifier<Label>,
) -> Result<(), EventRejection>
where
    Data: Clone,
    Tag: Clone,
    Handler: NamespaceEventHandler<Data, Tag, Label = Label>,
{
    match *modifier {
        | Modifier::AssertNonEmpty => {
            if current.emptiness() == Emptiness::EMPTY {
                handler.not_found(&prefix)?;
            }
        },
        | Modifier::In {
            path: ref subtree,
            ref inner,
        } => {
            let detached = current.detach_subtree(subtree);
            let outside = core::mem::replace(current, detached);
            stack.push(Instruction::Regraft {
                prefix: subtree.clone(),
                outside,
            });
            stack.push(Instruction::Apply {
                prefix: prefix.extended(subtree),
                modifier: inner,
            });
        },
        | Modifier::Renaming {
            ref source,
            ref target,
        } => {
            let moved = current.detach_subtree(source);
            current.graft_subtree(target, moved);
        },
        | Modifier::Seq(ref modifiers) => {
            for modifier in modifiers.iter().rev() {
                stack.push(Instruction::Apply {
                    prefix: prefix.clone(),
                    modifier,
                });
            }
        },
        | Modifier::Union(ref modifiers) => {
            let mut remaining = modifiers.iter();
            match remaining.next() {
                | None => {
                    *current = Trie::empty();
                },
                | Some(first) => {
                    stack.push(Instruction::UnionFold {
                        prefix: prefix.clone(),
                        input: current.clone(),
                        accumulated: Trie::empty(),
                        remaining,
                    });
                    stack.push(Instruction::Apply {
                        prefix,
                        modifier: first,
                    });
                },
            }
        },
        | Modifier::Hook(ref label) => {
            let subject = core::mem::take(current);
            *current = handler.hook(&prefix, label, subject)?;
        },
    }
    Ok(())
}
