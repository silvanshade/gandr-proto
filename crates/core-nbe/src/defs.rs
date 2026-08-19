//! The **definitional environment**: the per-scope table of names that carry an
//! unfolding rule, with the heights and the transparency the engine layer
//! reads.
//!
//! # Why it is scoped from the first line, not from the rung that needs it
//!
//! Transparent ascription is the one module-layer hole that constrains the
//! *shape* of the normalizer rather than adding a redex to it: a manifest type
//! component contributes an unfolding rule to a **per-scope** environment, and
//! strengthening re-adds equations to a sealed-then-transparently-viewed
//! module. A single global table is therefore wrong — the same atom is manifest
//! inside a sealed module and opaque outside it.
//!
//! That hole is not plugged here. What is paid here is its mitigation, and the
//! reason to pay it now is that it is free: an empty per-scope environment
//! degenerates exactly to the seed, so scoping costs nothing before the rung
//! lands and costs the environment's shape afterwards.
//!
//! # Transparency is engine policy, never kernel semantics
//!
//! Definitional equality depends on declaration **form** only. A sealed atom is
//! opaque because it has *no entry here at all* — not because an annotation
//! marks it opaque — which is why opacity falls out of a closed match rather
//! than a guard, and why an elaborator that lied about sealing something still
//! cannot make anything unfold it.
//!
//! [`Transparency::Irreducible`] is the reserved opt-out on top of that:
//! reducible by default, with heights doing the ordering, and an explicit
//! irreducible marking available for the engine's own heuristics. It is a
//! performance hint and it changes no judgement — an irreducible definition is
//! still equal to its body, it is simply not unfolded speculatively.

use alloc::borrow::ToOwned as _;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::DefinitionCount;
use gandr_core_term::boundary::DefinitionHeightLevel;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::ScopeDepth;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;

/// Whether the engine may unfold a definition speculatively.
///
/// This is a heuristic split and nothing more: both settings name definitions
/// that are equal to their bodies, and the only difference is whether the
/// conversion engine spends an unfolding on them without being forced to.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Transparency
{
    /// The default: the engine may unfold this definition whenever the
    /// pipeline's height and progress rules call for it.
    #[default]
    Reducible,
    /// The reserved opt-out: the engine unfolds this definition only when a
    /// comparison cannot be decided any other way.
    Irreducible,
}

/// One definition: a name's body, its definitional height, and its
/// transparency.
#[derive(Clone, Debug)]
pub struct Definition
{
    /// The body the name unfolds to, named in the syntax store rather than
    /// owned: a definitional environment holds handles for the same reason the
    /// semantic arena does.
    body: ValueNodeId,
    /// The definitional height: one above the tallest definition the body
    /// mentions, so "unfold the taller side" is a total order on a finite
    /// environment.
    height: DefinitionHeightLevel,
    /// Whether the engine may unfold this definition speculatively.
    transparency: Transparency,
}

impl Definition
{
    /// The body this definition unfolds to.
    #[inline]
    #[must_use]
    pub fn body(&self) -> ValueNodeId
    {
        self.body
    }

    /// The definitional height.
    #[inline]
    #[must_use]
    pub fn height(&self) -> DefinitionHeightLevel
    {
        self.height
    }

    /// Whether the engine may unfold this definition speculatively.
    #[inline]
    #[must_use]
    pub fn transparency(&self) -> Transparency
    {
        self.transparency
    }
}

/// The per-scope definitional environment.
///
/// Scopes stack, innermost last, and a lookup answers from the innermost scope
/// that binds the name. A name absent from every scope has no unfolding rule at
/// all, which is what makes a sealed atom, a native primitive, and a genuinely
/// free variable one case rather than three.
///
/// # Contract
/// - requires: nothing; the empty environment is the valid initial state and is
///   exactly the pre-unfolding conversion seed.
/// - ensures: [`Self::lookup`] answers from the innermost scope binding a name;
///   [`Self::define`] computes a height that is strictly greater than every
///   height it references.
/// - provides: the table both the readback face and the conversion face consult
///   when they ask whether a head unfolds — one table, so the two policies
///   cannot drift apart.
/// - panics: none.
#[derive(Clone, Debug, Default)]
#[repr(transparent)]
pub struct Definitions
{
    /// The scope stack, outermost first. The root scope is always present.
    scopes: Vec<BTreeMap<String, Definition>>,
}

impl Definitions
{
    /// An empty definitional environment with one root scope.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            scopes: alloc::vec![BTreeMap::new()],
        }
    }

    /// The number of open scopes, root included.
    #[inline]
    #[must_use]
    pub fn depth(&self) -> ScopeDepth
    {
        ScopeDepth::from(self.scopes.len().max(1))
    }

    /// The number of definitions visible across every open scope, counting a
    /// shadowed name once.
    #[inline]
    #[must_use]
    pub fn len(&self) -> DefinitionCount
    {
        let mut names = alloc::collections::BTreeSet::new();
        for scope in &self.scopes {
            names.extend(scope.keys());
        }
        DefinitionCount::from(names.len())
    }

    /// Whether no scope binds any name.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> gandr_core_term::boundary::InternerEmptyStatus
    {
        gandr_core_term::boundary::InternerEmptyStatus::from(usize::from(self.len()) == 0)
    }

    /// Opens a nested scope.
    ///
    /// # Contract
    /// - ensures: subsequent definitions land in the new scope, and every name
    ///   visible before stays visible until it is shadowed.
    /// - panics: none.
    #[inline]
    pub fn open_scope(&mut self)
    {
        self.scopes.push(BTreeMap::new());
    }

    /// Closes the innermost scope, discarding its definitions.
    ///
    /// # Contract
    /// - ensures: the innermost scope's names stop resolving; the root scope is
    ///   never closed, so the environment always has somewhere to define into.
    /// - panics: none.
    #[inline]
    pub fn close_scope(&mut self)
    {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Defines `name` as `body` in the innermost scope, reducible, at a
    /// mechanically computed height.
    ///
    /// # Contract
    /// - ensures: `name` resolves to `body` in this and every nested scope
    ///   until it is shadowed or its scope closes; the recorded height is one
    ///   greater than the tallest definition `body` mentions, saturating at the
    ///   wrapper's maximum.
    /// - provides: the unfolding rule the conversion pipeline's height step
    ///   orders its choices by.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the height rule has two decision surfaces, the
    ///   base case and the successor case, separated by defining a body that
    ///   mentions nothing and a body that mentions a definition of known
    ///   height, each observed as an exact height.
    /// - witness: `crate::tests::definition_height_is_one_above_what_the_body_mentions`
    /// - witness: `crate::tests::definition_height_sees_through_a_packed_module_and_its_elimination`
    #[inline]
    pub fn define<'source, N>(
        &mut self,
        arena: &FlatArena,
        name: N,
        body: ValueNodeId,
    ) where
        N: Into<NameRef<'source>>,
    {
        self.define_with(arena, name, body, Transparency::Reducible);
    }

    /// Defines `name` as `body` with an explicit transparency — the reserved
    /// irreducible opt-out.
    ///
    /// # Contract
    /// - ensures: as [`Self::define`], with `transparency` recorded verbatim.
    /// - panics: none.
    #[inline]
    pub fn define_with<'source, N>(
        &mut self,
        arena: &FlatArena,
        name: N,
        body: ValueNodeId,
        transparency: Transparency,
    ) where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        let height = self.height_of(arena, body);
        let definition = Definition {
            body,
            height,
            transparency,
        };
        match self.scopes.last_mut() {
            | Some(scope) => {
                scope.insert(name.as_ref().to_owned(), definition);
            },
            | None => {
                let mut scope = BTreeMap::new();
                scope.insert(name.as_ref().to_owned(), definition);
                self.scopes.push(scope);
            },
        }
    }

    /// Resolves `name` in the innermost scope that binds it.
    ///
    /// # Contract
    /// - ensures: returns the innermost binding, and `None` when no scope binds
    ///   `name` — which is how a sealed atom, a primitive, and a free variable
    ///   all arrive at the same rigid answer.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        name: NameRef<'_>,
    ) -> Option<&Definition>
    {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name.as_ref()))
    }

    /// The height a body would be defined at: one above the tallest definition
    /// it mentions.
    ///
    /// Heights are **mechanical from the definition graph** — nothing here
    /// reads an annotation, so an environment built in any order records the
    /// same heights for the same bodies.
    ///
    /// # Termination
    /// - reason: the scan drains an explicit worklist over one finite body, not
    ///   a recursive descent.
    /// - measure: pending subterms on the worklist.
    /// - boundedness: the body is a finite term and already-defined names are
    ///   read from the table rather than re-entered.
    /// - input recursion: none.
    fn height_of(
        &self,
        arena: &FlatArena,
        body: ValueNodeId,
    ) -> DefinitionHeightLevel
    {
        let mut tallest = 0_u32;
        for name in mentioned_names(arena, body) {
            if let Some(definition) = self.lookup(NameRef::from(name.as_str())) {
                tallest = tallest.max(u32::from(definition.height));
            }
        }
        DefinitionHeightLevel::from(tallest.saturating_add(1))
    }
}

/// Every variable name mentioned anywhere in the term rooted at `body`, bound
/// occurrences included.
///
/// Over-approximating is deliberate and cheap: a shadowed occurrence can only
/// raise a height, and a height that is too tall costs one extra unfolding
/// choice rather than a wrong answer.
///
/// # Termination
/// - reason: the walk drains an explicit worklist over one finite node graph.
/// - measure: pending nodes on the worklist.
/// - boundedness: node graphs are finite and each node queues only its own
///   children.
/// - input recursion: none.
fn mentioned_names(
    arena: &FlatArena,
    body: ValueNodeId,
) -> Vec<String>
{
    /// One pending node on the name scan's worklist.
    enum Task
    {
        /// A value node still to scan.
        Value(ValueNodeId),
        /// A computation node still to scan.
        Comp(CompNodeId),
    }

    let mut names = Vec::new();
    let mut work = alloc::vec![Task::Value(body)];
    while let Some(task) = work.pop() {
        match task {
            | Task::Value(id) => {
                let Some(node) = arena.values.get(id)
                else {
                    continue;
                };
                match *node {
                    | ValueNode::Var(ref name) => names.push(name.clone()),
                    // Leaves and a reified stack alike mention no name a
                    // definition could unfold: a stack's own bodies are frozen
                    // syntax that no unfolding rule reaches.
                    | ValueNode::Unit
                    | ValueNode::Int(_)
                    | ValueNode::Str(_)
                    | ValueNode::Num(_)
                    | ValueNode::Hole(_)
                    | ValueNode::Stk(_) => {},
                    | ValueNode::Pair(fst, snd) => {
                        work.push(Task::Value(fst));
                        work.push(Task::Value(snd));
                    },
                    | ValueNode::Inj(_, carried)
                    | ValueNode::Here(carried)
                    | ValueNode::Ctor {
                        payload: carried, ..
                    } => work.push(Task::Value(carried)),
                    | ValueNode::List(ref elements) => {
                        work.extend(elements.iter().map(|element| Task::Value(*element)));
                    },
                    | ValueNode::Record(ref fields) => {
                        work.extend(fields.values().map(|field| Task::Value(*field)));
                    },
                    // A thunk suspends a computation and an embedding names
                    // the value one returns; heights must see the names either
                    // mentions, so both descend into the computation child.
                    | ValueNode::Thunk(_, body) | ValueNode::Run(body) => {
                        work.push(Task::Comp(body));
                    },
                    // A packed module's witnesses are types, and this scan
                    // walks no type at all — an ascription's is skipped for the
                    // same reason. Heights order *definitions*, and a
                    // definition binds a value name.
                    | ValueNode::Pack { payload, .. } => work.push(Task::Value(payload)),
                    | ValueNode::Annot(inner, _) => work.push(Task::Value(inner)),
                }
            },
            | Task::Comp(id) => {
                let Some(node) = arena.comps.get(id)
                else {
                    continue;
                };
                match *node {
                    | CompNode::Abs(_, _, body)
                    | CompNode::Prj(_, body)
                    | CompNode::Reset(body)
                    | CompNode::Shift(_, body)
                    | CompNode::Fix(_, body) => work.push(Task::Comp(body)),
                    | CompNode::App(head, arg) => {
                        work.push(Task::Comp(head));
                        work.push(Task::Value(arg));
                    },
                    | CompNode::Ret(carried)
                    | CompNode::Force(carried)
                    | CompNode::Dup(carried)
                    | CompNode::Drop(carried)
                    | CompNode::Perform(_, _, carried) => work.push(Task::Value(carried)),
                    | CompNode::Bind(bound, _, cont) | CompNode::With(bound, cont) => {
                        work.push(Task::Comp(bound));
                        work.push(Task::Comp(cont));
                    },
                    | CompNode::Case(scrut, ref left, ref right) => {
                        work.push(Task::Value(scrut));
                        work.push(Task::Comp(left.1));
                        work.push(Task::Comp(right.1));
                    },
                    | CompNode::DataCase { scrut, ref arms } => {
                        work.push(Task::Value(scrut));
                        work.extend(arms.iter().map(|arm| Task::Comp(arm.1)));
                    },
                    | CompNode::ListCase {
                        scrut, nil, cons, ..
                    } => {
                        work.push(Task::Value(scrut));
                        work.push(Task::Comp(nil));
                        work.push(Task::Comp(cons));
                    },
                    // A package elimination reaches a name through its scrutinee
                    // and its body; its signature is a type and its atoms are
                    // nominal identities, so neither can name a definition.
                    | CompNode::Split { scrut, body, .. }
                    | CompNode::Unpack { scrut, body, .. } => {
                        work.push(Task::Value(scrut));
                        work.push(Task::Comp(body));
                    },
                    | CompNode::RecordProj { record, .. } => work.push(Task::Value(record)),
                    | CompNode::Handle {
                        scrutinee,
                        ref ret,
                        ref ops,
                        ..
                    } => {
                        work.push(Task::Comp(scrutinee));
                        work.push(Task::Comp(ret.1));
                        work.extend(ops.iter().map(|clause| Task::Comp(clause.body)));
                    },
                    | CompNode::Resume(carried, body) => {
                        work.push(Task::Value(carried));
                        work.push(Task::Comp(body));
                    },
                    | CompNode::Native { ref args, .. } => {
                        work.extend(args.iter().map(|arg| Task::Value(*arg)));
                    },
                    | CompNode::Walk {
                        scrut, ref base, ..
                    } => {
                        work.push(Task::Value(scrut));
                        work.push(Task::Comp(base.body));
                    },
                    | CompNode::Hole(_) => {},
                }
            },
        }
    }
    names
}
