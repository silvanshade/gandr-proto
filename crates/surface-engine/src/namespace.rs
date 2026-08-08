//! The name-modifier scope engine: hierarchical names, the modifier language,
//! and the visible/export scope split.
//!
//! This module is the elaboration layer's answer to *which declarations are in
//! scope where*. It has four pieces, each in its own submodule:
//!
//! - [`path`] — hierarchical names as ordered segment lists;
//! - [`trie`] — the ordered-map trie carrier from names to bindings;
//! - [`modifier`] — the six-constructor modifier language and its iterative
//!   interpreter;
//! - [`event`] — the handler seam for the three elaboration-time events;
//! - [`scope`] — the scope value carrying a visible and an export namespace,
//!   with sections.
//!
//! # What this layer is not
//!
//! It is deliberately an internal mechanism with no surface of its own:
//!
//! - **No surface syntax.** Nothing here is spelled in gandr source. The
//!   modifier language is built by callers, not parsed.
//! - **No import lowering.** `import "URI" as name ;` parses today
//!   (`surface-grammar`'s `import_declaration` rule) and is not lowered; this
//!   module does not change that. What it adds is the *meaning* the clause will
//!   have — see "The `as name` clause" below.
//! - **No operator-table or attribute-registry wiring.** Both are named in the
//!   design record as clients of this mechanism; neither is connected here.
//! - **Nothing kernel-side.** Paths never leave the elaboration layer.
//!
//! # The external design this absorbs
//!
//! The design is implemented as the OCaml package *yuujinchou: a library for
//! hierarchical names and lexical scoping*, release 5.2.0 (The `RedPRL`
//! Development Team, maintained by favonia; Apache-2.0 with LLVM-exception;
//! `url:https://github.com/RedPRL/yuujinchou`, rendered documentation at
//! `url:https://redprl.org/yuujinchou/yuujinchou/`). The semantics recorded in
//! [`modifier`] were checked against that release's `src/Language.ml`,
//! `src/Modifier.ml`, and `src/LanguageSigs.ml`, and the rationale paraphrased
//! below against its `docs/design.mld`. The code is not ported: this is a Rust
//! reimplementation of the API design, and the two differ wherever Rust and
//! OCaml differ — most visibly, algebraic effects become the
//! [`event::NamespaceEventHandler`] trait.
//!
//! Two things the absorbed design carries that this layer deliberately does
//! not, recorded so a later reader does not mistake the omission for an
//! oversight. Its scope module threads an *export prefix* through the enclosing
//! sections, so an event performed while merging the export namespace inside
//! `section p` reports its path under `p`; here the prefixing happens once, at
//! [`scope::Scope::end_section`], so such an event reports the unprefixed path.
//! Its `include`/`import` operations take a modifier applied to the incoming
//! namespace before the merge; here a caller composes [`modifier::Modifier`]'s
//! `apply` with the merge instead. Neither changes which bindings result.
//!
//! # The `as name` clause, re-read
//!
//! Today's one hard-coded renaming is the one-constructor special case of the
//! general language:
//!
//! ```text
//! import "file:///lib/parse.gandr" as parse ;   ≡   renaming . parse
//! ```
//!
//! — relocate the whole imported namespace (the root, `.`) to the one-segment
//! path `parse`, so an imported `lexer.token` arrives as `parse.lexer.token`.
//! [`modifier::Modifier::alias_as`] is that modifier, and it is defined as
//! exactly `Modifier::renaming(NamePath::root(), …)` rather than as a special
//! form. The generalization is therefore backward-compatible by construction:
//! whatever richer import clause lands later, today's spelling is one point in
//! it. Lowering the declaration itself stays gated on the module layer and is
//! not part of this change.
//!
//! ```
//! use gandr_surface_engine::namespace::event::PermissiveHandler;
//! use gandr_surface_engine::namespace::modifier::Modifier;
//! use gandr_surface_engine::namespace::path::DottedName;
//! use gandr_surface_engine::namespace::path::NamePath;
//! use gandr_surface_engine::namespace::path::Segment;
//! use gandr_surface_engine::namespace::trie::Binding;
//! use gandr_surface_engine::namespace::trie::Trie;
//!
//! let imported: Trie<u32, ()> = core::iter::once((
//!     NamePath::from(DottedName::from("lexer.token")),
//!     Binding::new(1_u32, ()),
//! ))
//! .collect();
//!
//! let mut handler = PermissiveHandler::<()>::new();
//! let qualified = Modifier::alias_as(Segment::from("parse"))
//!     .apply(imported, &mut handler)
//!     .expect("the permissive handler rejects nothing");
//!
//! assert!(
//!     qualified
//!         .get(&NamePath::from(DottedName::from("parse.lexer.token")))
//!         .is_some(),
//!     "`as parse` qualifies every imported path"
//! );
//! ```
//!
//! # Namespace graduation, prepared
//!
//! gandr's prelude modules (`prim`, `list`, `record`, …) and host modules
//! (`fs`, `env`, `proc`) are reserved by a **syntactic** gate as built: the
//! lowerer asks [`crate::prelude::is_module_member`] and
//! [`crate::host::host_module`] whether a projection head is a known table
//! entry, and [`crate::host`]'s own documentation states the gate's nature —
//! reserved module *names*, not scoped bindings. A script that declares its own
//! `env` therefore collides with a reservation rather than shadowing a binding.
//!
//! [`scope::Scope::with_init_visible`] is the shape that graduation takes: the
//! three tables become an initial **visible** namespace over the same scope
//! value, recognition becomes ordinary resolution against it, and "a user
//! declaration shadows a builtin" becomes a shadow event whose policy is a
//! handler — warn-and-allow being one, reject being another. Nothing in this
//! change performs that graduation: the tables are untouched, the lowerer still
//! asks the syntactic question, and the graduation waits on the module layer
//! that owns import lowering. What lands here is that the target shape exists
//! and is exercised, so the graduation is a rewiring rather than a redesign.
//!
//! # The division of labour against the nominal machinery
//!
//! The word "name" means two different things in this tree, at two different
//! layers, and the two are complements rather than competitors. **A path is
//! reachability; an atom is identity.** The statements below were checked
//! against the definitions, not against the prose beside them.
//!
//! **The nominal layer.** [`gandr_theory_nominal_automata::Atom`] is a sort tag
//! plus an integer identity and carries no text whatsoever; it is minted by
//! [`gandr_theory_nominal_automata::Gensym`], a monotone per-sort counter.
//! `gandr-theory-nominal-automata` has no notion of a hierarchical name at all
//! — nothing in it takes, produces, or compares a path. Inside this crate the
//! atom consumers are the lowerer's own machine-minted names: the `%tmp`
//! hoist binders (`GandrSort::TmpHoist`) and hole addresses
//! (`GandrSort::HoleAddr`).
//!
//! **The modifier layer.** Everything in this module manipulates paths, and
//! nothing in it mints anything: there is no [`gandr_theory_nominal_automata`]
//! dependency in these submodules, and a binding's payload is an opaque type
//! parameter the carrier relocates without inspecting.
//!
//! **Where the two meet, as built.** Two places, and the first is more nuanced
//! than "paths resolve to atoms":
//!
//! 1. **Resolution at the elaboration boundary.** A path is resolved — here by
//!    [`scope::Scope::resolve`] — to whatever the consumer bound at it, and
//!    what that payload *is* varies today. A declared datatype's identity is a
//!    [`gandr_core_checker::types::DataId`]: a declaration-order serial plus
//!    the declared name, minted per declaration, so two `data Boolean`
//!    declarations are distinct types. A user definition is still a string
//!    binder in the checker's context. Only lowerer-minted binders and hole
//!    addresses are atoms. What this layer contributes at that boundary is
//!    narrower and exact: because paths are inert data, a *reachability*
//!    collision surfaces as a shadow event instead of silently redirecting a
//!    path to a different identity.
//! 2. **Freshness is untouched by renaming.** `Gensym`'s mint reads no name, so
//!    no path operation can perturb an atom's identity. A `DataId` embeds the
//!    *declaration site's* name together with its serial, and a modifier moves
//!    paths rather than declarations, so neither component can be perturbed
//!    either. An identity may be reached as `m.t`, renamed to `t`, or hidden by
//!    `except`, and is the same identity throughout — which is the operational
//!    content of "namespaces are untyped": nothing the modifier engine does can
//!    leak an abstract identity, because leaking is a property of identities
//!    and this layer only ever holds paths.
//!
//! [`gandr_theory_nominal_automata::Atom`]: gandr_theory_nominal_automata::Atom
//! [`gandr_theory_nominal_automata::Gensym`]: gandr_theory_nominal_automata::Gensym
//! [`gandr_core_checker::types::DataId`]: gandr_core_checker::types::DataId

pub mod event;
pub mod modifier;
pub mod path;
pub mod scope;
pub mod trie;

pub use crate::namespace::event::EventKind;
pub use crate::namespace::event::EventRejection;
pub use crate::namespace::event::NamespaceEvent;
pub use crate::namespace::event::NamespaceEventHandler;
pub use crate::namespace::event::PermissiveHandler;
pub use crate::namespace::event::RejectionReason;
pub use crate::namespace::modifier::Modifier;
pub use crate::namespace::path::DottedName;
pub use crate::namespace::path::NamePath;
pub use crate::namespace::path::Segment;
pub use crate::namespace::scope::Scope;
pub use crate::namespace::scope::ScopeError;
pub use crate::namespace::scope::ScopeResult;
pub use crate::namespace::trie::Binding;
pub use crate::namespace::trie::Collision;
pub use crate::namespace::trie::Trie;
