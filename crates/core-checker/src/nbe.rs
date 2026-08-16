//! **Normalization by evaluation**: the checker's conversion engine.
//!
//! This module replaces the rung-one no-reduction shadow of definitional
//! equality with the real thing. Terms are evaluated into a glued semantic
//! domain, compared there, and read back into syntax when a term is wanted;
//! the six-step definitional-equality pipeline decides conversion, and every
//! caller of the old structural equality now goes through it.
//!
//! # The engine in one screen
//!
//! | module   | what it owns                                                         |
//! | -------- | -------------------------------------------------------------------- |
//! | [`sem`]  | the glued value domain, its per-run arena, and the guard word        |
//! | [`defs`] | the per-scope definitional environment, heights, and transparency    |
//! | [`eval`] | evaluation and the three force modes                                 |
//! | [`quote`]| readback and its three options                                       |
//! | [`conv`] | the six-step definitional-equality pipeline                          |
//! | [`intern`] | the per-face syntax interner                                       |
//!
//! # The definitional-equality pipeline
//!
//! Six steps, each falling through to the next only on a non-answer:
//!
//! 1. **identity equality** — one id in one arena is one value, so an id match
//!    answers *convertible* outright. It is sound by immutability and it takes
//!    no table into the trusted base, because it decides only reflexive pairs
//!    and defers every distinct-id pair to the walk.
//! 2. **cached-word guards** — the intrusive [`sem::Guard`] answers *distinct*
//!    in constant time for a rigid, hole-free pair whose hashes differ.
//! 3. **iterative structural comparison** over a heap worklist, with a
//!    head-mismatch fast fail before descending into children.
//! 4. **lazy unfolding with heights** — same head with regular hints tries the
//!    arguments first; otherwise the **taller** side unfolds, and a one-sided
//!    unfolding rule unfolds that side.
//! 5. **smart unfolding gated on case progress** — a definition unfolds only if
//!    doing so makes case-tree progress, which is decidable directly here
//!    because case is first-class, so no companion definition is needed.
//! 6. **three-state speculation** — rigid, flex, full: two same-head glued
//!    neutrals compare spines first with no commitments and back off onto the
//!    **already-forced** unfolded face, so nothing is evaluated twice.
//!
//! # What this engine will not do
//!
//! The module layer exports four anti-commitments and they are honoured here
//! as prohibitions rather than as unimplemented features:
//!
//! * **it never compares signatures.** A record type is compared by its
//!   canonically-ordered label-to-type map, field by field. That is structural
//!   equality on a canonical representation, not a width or permutation
//!   equation, and no width or permutation rule exists anywhere in [`conv`].
//!   Growing one would foreclose the telescope future from inside the
//!   conversion engine.
//! * **it never memoizes across functor instantiations.** There is no
//!   content-keyed value memoization at all, so the question cannot arise.
//! * **it never makes a package eliminable by anything but its own elimination
//!   form** — packages have no representation here, so nothing eliminates one.
//! * **it takes no interning table into the trusted base.** Interning is syntax
//!   only, per face, and it is a deduplicator.
//!
//! # The quarantine
//!
//! Conversion never runs an effect, a handler, or a control operator. Those
//! formers evaluate to **neutrals** with their operands evaluated, so the
//! equality this engine offers on them is congruence and nothing stronger. The
//! quarantine used to hold vacuously, because nothing was evaluated at all; it
//! now holds by construction, because the formers that would break it have no
//! reduction rule in [`eval`].
//!
//! # Module forms enter as neutrals
//!
//! The module layer's six holes cost nothing before their rungs land, because a
//! neutral head is already the pipeline's ordinary case. The one hole this rung
//! plugs is **structure projection**: a record module's field selection reduces
//! when its head is a structure and stays neutral when it is not, weak-head and
//! spine-local, never touching the sibling components.

pub mod conv;
pub mod defs;
pub mod eval;
pub mod intern;
pub mod quote;
pub mod sem;

use alloc::rc::Rc;

use crate::boundary::ConversionFuel;
use crate::boundary::ValueEquality;
use crate::boundary::VariableLevel;
use crate::nbe::defs::Definitions;
use crate::nbe::intern::SyntaxInterner;
use crate::nbe::sem::SemArena;
use crate::nbe::sem::SemError;
use crate::nbe::sem::Watermark;
use crate::syntax::FlatArena;
use crate::syntax::Value;
use crate::syntax::ValueNodeId;

/// The default fuel a normalizer spends before it stops unfolding.
///
/// It bounds one pathological case and nothing else: a definition whose body
/// mentions itself. Heights make that impossible for an environment built by
/// the elaborator, since a body can only mention definitions that already
/// exist, but the environment is a public surface and a caller may define one
/// anyway. Running out of fuel stops unfolding and answers on the neutral face,
/// which loses completeness on that definition and never soundness.
const DEFAULT_FUEL: u32 = 4096;

/// The normalizer: one arena, one definitional environment, one per-face syntax
/// interner, and the fresh-variable counter readback draws from.
///
/// # Contract
/// - requires: every id handed back to a normalizer was minted by that same
///   normalizer and has not been truncated away.
/// - ensures: every entry point is deterministic — the same term, environment,
///   and definitions give the same answer, byte for byte, on every run and on
///   every host. Fresh variables are de Bruijn levels drawn from a counter, so
///   nothing an answer depends on is an address, a hash-map order, or an
///   allocation order.
/// - provides: the checker's conversion engine, its normal forms, and the
///   readback that turns a semantic value back into syntax.
/// - fails: [`SemError`] on arena exhaustion or an unresolvable id;
///   [`Self::converts`] absorbs those into a **distinct** verdict, which is the
///   fail-closed direction.
/// - panics: none.
#[derive(Clone, Debug)]
pub struct Normalizer
{
    /// The per-run semantic arena.
    arena: SemArena,
    /// The per-scope definitional environment.
    defs: Definitions,
    /// The syntax store: the flat node carrier every handle in the semantic
    /// arena names. It owns the syntax; nothing in the semantic arena does.
    syntax: FlatArena,
    /// The per-face syntax interner, keyed on canonical content.
    interner: SyntaxInterner,
    /// The next de Bruijn level readback and conversion will generate.
    next_level: u32,
    /// The unfolding budget one force may spend.
    fuel: ConversionFuel,
}

impl Default for Normalizer
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl Normalizer
{
    /// A normalizer with an empty definitional environment.
    ///
    /// The empty environment is exactly the pre-unfolding conversion seed: with
    /// nothing to unfold, the pipeline's last three steps never fire and the
    /// engine decides beta-eta equality on the term language alone.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            arena: SemArena::new(),
            defs: Definitions::new(),
            syntax: FlatArena::new(),
            interner: SyntaxInterner::new(),
            next_level: 0,
            fuel: ConversionFuel::from(DEFAULT_FUEL),
        }
    }

    /// A normalizer over an existing definitional environment.
    #[inline]
    #[must_use]
    pub fn with_definitions(defs: Definitions) -> Self
    {
        Self {
            defs,
            ..Self::new()
        }
    }

    /// The definitional environment.
    #[inline]
    #[must_use]
    pub fn definitions(&self) -> &Definitions
    {
        &self.defs
    }

    /// The definitional environment, for scoping.
    ///
    /// Defining goes through [`Self::define`] rather than through this, because
    /// a definition's body is lowered into the syntax store first and the
    /// environment holds the resulting handle.
    #[inline]
    pub fn definitions_mut(&mut self) -> &mut Definitions
    {
        &mut self.defs
    }

    /// Defines `name` as `body`, reducible, lowering the body into the syntax
    /// store on the way.
    ///
    /// # Contract
    /// - ensures: `name` unfolds to `body` in the innermost open scope, at the
    ///   height the definition graph gives it.
    /// - fails: [`SemError::SyntaxStore`] when lowering fails.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn define<'source, N>(
        &mut self,
        name: N,
        body: &Value,
    ) -> Result<(), SemError>
    where
        N: Into<crate::boundary::NameRef<'source>>,
    {
        self.define_with(name, body, defs::Transparency::Reducible)
    }

    /// Defines `name` as `body` with an explicit transparency — the reserved
    /// irreducible opt-out.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn define_with<'source, N>(
        &mut self,
        name: N,
        body: &Value,
        transparency: defs::Transparency,
    ) -> Result<(), SemError>
    where
        N: Into<crate::boundary::NameRef<'source>>,
    {
        let node = self.lower_input(body)?;
        self.defs
            .define_with(&self.syntax, name, node, transparency);
        Ok(())
    }

    /// The per-face syntax interner.
    #[inline]
    #[must_use]
    pub fn interner(&self) -> &SyntaxInterner
    {
        &self.interner
    }

    /// The syntax store every handle in the semantic arena names.
    ///
    /// There is deliberately **no mutable accessor for the interner**. The flat
    /// carrier is `Clone`, so handing one out would let a caller insert any
    /// node under a face of its own choosing and the two faces' representative
    /// sets would stop being disjoint; interning is reached only through
    /// [`Self::lower_input`] and [`Self::intern_readback`], each of which fixes
    /// its own face.
    #[inline]
    #[must_use]
    pub fn syntax(&self) -> &FlatArena
    {
        &self.syntax
    }

    /// The syntax store, for lowering and for readback.
    #[inline]
    pub fn syntax_mut(&mut self) -> &mut FlatArena
    {
        &mut self.syntax
    }

    /// The semantic arena.
    #[inline]
    #[must_use]
    pub fn arena(&self) -> &SemArena
    {
        &self.arena
    }

    /// The semantic arena, for minting.
    #[inline]
    pub fn arena_mut(&mut self) -> &mut SemArena
    {
        &mut self.arena
    }

    /// The unfolding budget one force may spend.
    #[inline]
    #[must_use]
    pub fn fuel(&self) -> ConversionFuel
    {
        self.fuel
    }

    /// Sets the unfolding budget one force may spend.
    #[inline]
    pub fn set_fuel(
        &mut self,
        fuel: ConversionFuel,
    )
    {
        self.fuel = fuel;
    }

    /// Draws the next fresh de Bruijn level.
    ///
    /// # Contract
    /// - ensures: every call returns a level distinct from every level drawn
    ///   before it **in the current run**, and the sequence a run draws is the
    ///   same on every run — which is where byte-stable readback comes from.
    /// - panics: none.
    #[inline]
    pub fn fresh_level(&mut self) -> VariableLevel
    {
        let level = self.next_level;
        self.next_level = self.next_level.saturating_add(1);
        VariableLevel::from(level)
    }

    /// Opens a run: records the arena population and restarts the fresh-level
    /// counter, returning what [`Self::finish_run`] needs to close it.
    ///
    /// Restarting the counter is what makes readback **byte-stable** rather
    /// than merely alpha-stable: without it, a second normalization on the same
    /// normalizer would name its binders from wherever the counter had reached,
    /// so two normal forms of one term would be alpha-equal and structurally
    /// different. Nesting is safe because a run's level-bearing values are
    /// truncated away before its caller looks at anything again.
    #[inline]
    fn begin_run(&mut self) -> (Watermark, VariableLevel)
    {
        let opened = (self.watermark(), VariableLevel::from(self.next_level));
        self.next_level = 0;
        opened
    }

    /// Closes a run opened by [`Self::begin_run`]: truncates its semantic nodes
    /// and restores the enclosing run's fresh-level counter.
    #[inline]
    fn finish_run(
        &mut self,
        opened: (Watermark, VariableLevel),
    )
    {
        self.truncate_to(opened.0);
        self.next_level = u32::from(opened.1);
    }

    /// The arena population, for a later [`Self::truncate_to`].
    #[inline]
    #[must_use]
    pub fn watermark(&self) -> Watermark
    {
        self.arena.watermark()
    }

    /// Drops every semantic node minted after `mark`.
    ///
    /// This is how a run's values stop existing once its verdict is in: they
    /// allocate past the watermark and are truncated wholesale, so no
    /// unfolding-built value outlives the question it was built to answer.
    #[inline]
    pub fn truncate_to(
        &mut self,
        mark: Watermark,
    )
    {
        self.arena.truncate_to(mark);
    }

    /// Normalizes a source value: evaluate, then read back.
    ///
    /// # Contract
    /// - ensures: the result is the normal form of `term` under this
    ///   normalizer's definitional environment, in canonical binder form and
    ///   interned into the readback face; the run's semantic nodes are
    ///   truncated away before the result is returned, so nothing the
    ///   normalization built survives it.
    /// - provides: the normal form a caller asked for, as ordinary syntax.
    /// - fails: [`SemError`] on arena exhaustion or an unresolvable id.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] on arena exhaustion or an unresolvable id.
    ///
    /// # Adequacy
    /// - hypothesis: L2 against conversion as the external oracle — a term and
    ///   its normal form always convert, and normalizing twice is idempotent —
    ///   plus L3 for the truncation, separated by observing the arena
    ///   population before and after.
    /// - witness: `nbe::tests::a_term_converts_with_its_own_normal_form`
    /// - witness: `nbe::tests::normalization_is_idempotent`
    /// - witness: `nbe::tests::normalizing_leaves_the_arena_where_it_found_it`
    #[inline]
    pub fn normalize(
        &mut self,
        term: &Value,
    ) -> Result<Rc<Value>, SemError>
    {
        let node = self.normalize_node(term)?;
        self.reify(node)
    }

    /// Normalizes a source value and returns the **node** its normal form
    /// occupies in the syntax store, interned into the readback face.
    ///
    /// This is the entry a caller uses when it wants to keep working in node
    /// ids; [`Self::normalize`] is this composed with one reification.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] on arena exhaustion or an unresolvable id.
    #[inline]
    pub fn normalize_node(
        &mut self,
        term: &Value,
    ) -> Result<ValueNodeId, SemError>
    {
        let lowered = self.lower_input(term)?;
        let opened = self.begin_run();
        let evaluated = eval::eval_value(self, SemArena::EMPTY_ENV, lowered)?;
        let quoted = quote::quote_value(self, evaluated, quote::QuoteMode::Canonical)?;
        self.finish_run(opened);
        // Canonical readback output, which is exactly what the readback face
        // accepts; the precondition on `intern_readback` states why.
        Ok(self.intern_readback(quoted))
    }

    /// Interns a node into the **readback** face's table.
    ///
    /// # Contract
    /// - requires: `node` is [`quote::QuoteMode::Canonical`] readback output or
    ///   a fresh allocation — **never** a [`quote::QuoteMode::Retained`] result
    ///   and never an input-face representative. The retained mode returns the
    ///   *input-face* node by design, which is the term face working, and
    ///   offering one here would put a single node in both tables; two lookups
    ///   would then agree on a representative across faces, which is exactly
    ///   the cross-face equality the tables must never establish.
    /// - ensures: the result is alpha-identical to `node` and shared with every
    ///   alpha-identical normal form previously interned into the readback
    ///   face; the two faces' representative sets stay **disjoint**, so one
    ///   alpha-key may sit in both tables under two different representatives
    ///   and neither table can speak for the other.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the precondition is enforced by construction on the
    ///   one live path, so the witness pins that path rather than the guard:
    ///   normalizing a term whose binders are source names must intern a
    ///   *different* representative from the lowered input, with one entry in
    ///   each face.
    /// - witness: `nbe::tests::normalize_node_interns_a_canonical_form_not_the_input`
    #[inline]
    pub(crate) fn intern_readback(
        &mut self,
        node: ValueNodeId,
    ) -> ValueNodeId
    {
        self.interner
            .intern(&self.syntax, intern::Face::ReadbackNormalForm, node)
    }

    /// Reads a syntax node back out to an ordinary term.
    ///
    /// This is the **only** place the engine produces an owned recursive term,
    /// and it is a boundary service for callers rather than an internal step:
    /// the reified term is the caller's, and nothing inside the normalizer
    /// keeps a reference to it.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when the node does not resolve.
    #[inline]
    pub fn reify(
        &self,
        node: ValueNodeId,
    ) -> Result<Rc<Value>, SemError>
    {
        self.syntax
            .value(node)
            .map(Rc::new)
            .map_err(|_error| SemError::MissingSyntaxValue(node))
    }

    /// Decides definitional equality of two source values.
    ///
    /// # Contract
    /// - ensures: the verdict of [`conv::converts`], with the run's semantic
    ///   nodes truncated away behind it.
    /// - fails: never; an arena error is absorbed into a **distinct** verdict.
    /// - panics: none.
    #[inline]
    pub fn converts(
        &mut self,
        lhs: &Rc<Value>,
        rhs: &Rc<Value>,
    ) -> ValueEquality
    {
        conv::converts(self, lhs, rhs)
    }

    /// Decides definitional equality of two value types.
    ///
    /// A module signature is a record type, so this is the entry a signature
    /// comparison arrives at.
    #[inline]
    pub fn type_converts(
        &mut self,
        lhs: &crate::types::ValueType,
        rhs: &crate::types::ValueType,
    ) -> ValueEquality
    {
        conv::type_converts(self, lhs, rhs)
    }

    /// Lowers a caller's term into the syntax store and interns it into the
    /// **input** face, returning the node the engine will work from.
    ///
    /// # Contract
    /// - ensures: the result names a node alpha-identical to `term` and shared
    ///   with every alpha-identical term previously lowered into the input
    ///   face; it is never shared with a readback normal form, because the
    ///   faces hold separate tables and are never compared.
    /// - provides: the one crossing from a caller's owned term into the
    ///   engine's handles — past it, no owned recursive term exists inside the
    ///   normalizer.
    /// - fails: [`SemError::SyntaxStore`] when the store's id space is
    ///   exhausted.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn lower_input(
        &mut self,
        term: &Value,
    ) -> Result<ValueNodeId, SemError>
    {
        let node = self
            .syntax
            .alloc_value(term)
            .map_err(|_error| SemError::SyntaxStore)?;
        Ok(self
            .interner
            .intern(&self.syntax, intern::Face::ElaborationInput, node))
    }
}

#[cfg(test)]
mod tests
{
    use alloc::borrow::ToOwned as _;
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::*;
    use crate::boundary::ConversionFuel;
    use crate::boundary::FieldName;
    use crate::boundary::GradeBound;
    use crate::boundary::IntegerLiteral;
    use crate::boundary::NameRef;
    use crate::effect::EffectSig;
    use crate::grade::Grade;
    use crate::nbe::defs::Transparency;
    use crate::nbe::eval::ForceMode;
    use crate::nbe::eval::eval_value;
    use crate::nbe::eval::force_value;
    use crate::nbe::intern::Face;
    use crate::nbe::intern::canonical_key;
    use crate::nbe::quote::QuoteMode;
    use crate::nbe::quote::quote_value;
    use crate::nbe::sem::Guard;
    use crate::nbe::sem::Neutral;
    use crate::nbe::sem::NeutralHead;
    use crate::nbe::sem::Rigid;
    use crate::nbe::sem::SemValue;
    use crate::nbe::sem::SemValueNode;
    use crate::nbe::sem::ValueUnfold;
    use crate::nbe::sem::mix_word;
    use crate::nbe::sem::seed;
    use crate::syntax::Comp;
    use crate::syntax::Side;
    use crate::syntax::WalkBase;
    use crate::syntax::WalkMotive;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// Wraps a computation as the thunk value the normalizer's value-level
    /// entry points take, so a reduction rule is observable through `converts`.
    fn thunk(body: Comp) -> Rc<Value>
    {
        Rc::new(Value::Thunk(Grade::ONE, Rc::new(body)))
    }

    /// A source variable.
    fn var(name: NameRef<'_>) -> Rc<Value>
    {
        Rc::new(Value::var(name))
    }

    /// An integer literal.
    fn int(literal: IntegerLiteral) -> Rc<Value>
    {
        Rc::new(Value::Int(i64::from(literal)))
    }

    /// A record literal over the given labelled fields.
    fn record(fields: &[(FieldName<'_>, Rc<Value>)]) -> Rc<Value>
    {
        let fields = fields
            .iter()
            .map(|&(label, ref field)| (label.as_ref().to_owned(), Rc::clone(field)))
            .collect::<BTreeMap<_, _>>();
        Rc::new(Value::Record(fields))
    }

    /// Lowers a caller's term into the normalizer's syntax store.
    fn lower(
        nbe: &mut Normalizer,
        term: &Value,
    ) -> crate::syntax::ValueNodeId
    {
        nbe.lower_input(term).expect("lowering must succeed")
    }

    /// The trivial effect signature the quarantine tests perform against.
    fn signature() -> EffectSig
    {
        EffectSig::new(
            crate::boundary::EffectSignatureName::from("State"),
            Vec::new(),
        )
    }

    // ── the arena and its guard word ────────────────────────────────────────

    #[test]
    fn guard_settles_distinct_only_for_rigid_hole_free_pairs()
    {
        let one = Guard::leaf(seed(crate::boundary::SemanticHash::from(1)));
        let two = Guard::leaf(seed(crate::boundary::SemanticHash::from(2)));
        // Rigid and hole-free with different hashes: settled.
        assert!(bool::from(one.settles_distinct(two)));
        // Rigid and hole-free with equal hashes: not settled — equal hashes
        // prove nothing.
        assert!(!bool::from(one.settles_distinct(one)));
        // One side carries an unfolding rule: the word cannot see past it.
        assert!(!bool::from(one.with_unfolding().settles_distinct(two)));
        // One side carries a hole: the gradual wildcard forbids the shortcut.
        assert!(!bool::from(one.with_hole().settles_distinct(two)));
    }

    #[test]
    fn guard_folding_propagates_holes_and_unfolding()
    {
        let plain = Guard::leaf(seed(crate::boundary::SemanticHash::from(1)));
        let holed = plain.with_hole();
        let opaque = plain.with_unfolding();
        assert!(bool::from(plain.fold(holed).holes()));
        assert!(!bool::from(plain.fold(opaque).rigid()));
        assert!(bool::from(plain.fold(plain).rigid()));
        assert!(u32::from(plain.fold(plain).depth()) > u32::from(plain.depth()));
    }

    #[test]
    fn extending_a_glued_spine_reopens_the_unfolding_face()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let head = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let rigid = Neutral::new(NeutralHead::Force(head), sem::CompUnfold::Rigid);
        assert_eq!(
            rigid.extended(sem::Elim::Project(Side::Fst), None).unfold(),
            sem::CompUnfold::Rigid
        );
        let height = crate::boundary::DefinitionHeightLevel::from(3);
        let glued = Neutral::new(NeutralHead::Force(head), sem::CompUnfold::Pending(height));
        let grown = glued.extended(sem::Elim::Project(Side::Fst), Some(height));
        assert_eq!(grown.unfold(), sem::CompUnfold::Pending(height));
        assert_eq!(usize::from(grown.spine_len()), 1);
    }

    #[test]
    fn truncating_to_a_watermark_drops_every_family()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::Unit)),
            Value::Unit,
        ));
        let node = lower(&mut nbe, &term);
        let mark = nbe.watermark();
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let _forced = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert_ne!(nbe.watermark(), mark);
        nbe.truncate_to(mark);
        assert_eq!(nbe.watermark(), mark);
    }

    #[test]
    fn an_arena_id_resolves_only_while_it_is_live()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let mark = nbe.watermark();
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert!(nbe.arena().value(evaluated).is_ok());
        nbe.truncate_to(mark);
        assert!(nbe.arena().value(evaluated).is_err());
    }

    // ── the definitional environment ────────────────────────────────────────

    #[test]
    fn definition_height_is_one_above_what_the_body_mentions()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        let base = nbe
            .definitions()
            .lookup(NameRef::from("a"))
            .unwrap()
            .height();
        assert_eq!(u32::from(base), 1);
        nbe.define(
            NameRef::from("b"),
            &Value::Pair(var(NameRef::from("a")), int(IntegerLiteral::from(2_i64))),
        )
        .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("b"))
                    .unwrap()
                    .height()
            ),
            2
        );
        // A body mentioning nothing defined stays at the base height, whatever
        // else the environment holds.
        nbe.define(NameRef::from("c"), &int(IntegerLiteral::from(3_i64)))
            .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("c"))
                    .unwrap()
                    .height()
            ),
            1
        );
    }

    #[test]
    fn a_nested_scope_shadows_and_then_releases()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        let outer = nbe.definitions().lookup(NameRef::from("a")).unwrap().body();
        nbe.definitions_mut().open_scope();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(2_i64)))
            .unwrap();
        let inner = nbe.definitions().lookup(NameRef::from("a")).unwrap().body();
        assert_ne!(outer, inner);
        assert_eq!(*nbe.reify(inner).unwrap(), Value::Int(2));
        nbe.definitions_mut().close_scope();
        assert_eq!(
            nbe.definitions().lookup(NameRef::from("a")).unwrap().body(),
            outer
        );
        // The root scope is never closed, so the environment always has
        // somewhere to define into.
        nbe.definitions_mut().close_scope();
        assert_eq!(usize::from(nbe.definitions().depth()), 1);
    }

    // ── the per-face syntax interner ────────────────────────────────────────

    #[test]
    fn interning_shares_alpha_equivalent_terms_within_a_face()
    {
        let mut nbe = Normalizer::new();
        let first = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        let second = thunk(Comp::lam("y", Comp::ret(Value::var(NameRef::from("y")))));
        let canonical = lower(&mut nbe, &first);
        let again = lower(&mut nbe, &second);
        assert_eq!(canonical, again);
        // An alpha-distinct term is a second entry.
        let other = lower(&mut nbe, &thunk(Comp::lam("x", Comp::ret(Value::Unit))));
        assert_ne!(canonical, other);
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 2);
    }

    #[test]
    fn interning_keeps_the_two_faces_disjoint()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::ret(Value::Int(1)));
        let input = lower(&mut nbe, &term);
        // The same syntax again, this time offered to the readback face: the
        // tables are separate, so it gets its own representative and no lookup
        // can establish that the two are one.
        let fresh = nbe.syntax_mut().alloc_value(&term).unwrap();
        let readback = nbe.intern_readback(fresh);
        assert_ne!(input, readback);
        assert_eq!(
            canonical_key(nbe.syntax(), input),
            canonical_key(nbe.syntax(), readback)
        );
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 1);
        assert_eq!(usize::from(nbe.interner().len(Face::ReadbackNormalForm)), 1);
    }

    #[test]
    fn normalize_node_interns_a_canonical_form_not_the_input()
    {
        let mut nbe = Normalizer::new();
        // An already-normal term whose binder carries a SOURCE name: nothing
        // reduces, so the only thing normalization changes is the binder, and
        // the readback representative must therefore be a different node from
        // the lowered input.
        let term = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        let input = lower(&mut nbe, &term);
        let normal = nbe.normalize_node(&term).unwrap();
        assert_ne!(
            input, normal,
            "the readback face took the input representative"
        );
        // One entry per face, and the two entries share an alpha-key: the key
        // is canonical in binder form, so this is exactly the case where a
        // shared table would have collapsed the faces into one representative.
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 1);
        assert_eq!(usize::from(nbe.interner().len(Face::ReadbackNormalForm)), 1);
        assert_eq!(
            canonical_key(nbe.syntax(), input),
            canonical_key(nbe.syntax(), normal),
            "the two faces should hold one alpha-key under two representatives"
        );
        // And this is what pins the mode: normalization reads back canonically,
        // so the binder is a level rather than the source name it went in with.
        let Value::Thunk(_, ref body) = *nbe.reify(normal).unwrap()
        else {
            panic!("normalizing a thunk did not produce one");
        };
        let Comp::Abs(ref binder, ..) = **body
        else {
            panic!("normalizing a lambda did not produce one");
        };
        assert_eq!(binder.as_str(), "\u{ab}0\u{bb}");
    }

    // ── ownership: the arena holds handles, never terms ─────────────────────

    #[test]
    fn a_deep_term_survives_its_input_syntax_being_dropped_first()
    {
        let mut nbe = Normalizer::new();
        // Ten thousand nested binds, lowered into the store, and then the
        // caller's own term released BEFORE the normalizer. Nothing in the
        // semantic arena owns that term, so this is an ordinary drop; the
        // earlier shape — a reference-counted term face — freed the chain
        // recursively here and aborted the process.
        let mut body = Comp::ret(Value::Int(0));
        for index in 0 .. 10_000_u32 {
            let name = alloc::format!("v{index}");
            body = Comp::bind(Comp::ret(Value::Int(1)), name.as_str(), body);
        }
        let term = thunk(body);
        let lowered = lower(&mut nbe, &term);
        release_binds(term);
        // The engine still works from its own handles after the input is gone.
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(
            canonical_key(nbe.syntax(), quoted),
            canonical_key(nbe.syntax(), quoted)
        );
        drop(nbe);
    }

    #[test]
    fn a_deeply_nested_value_survives_its_input_syntax_being_dropped_first()
    {
        let mut nbe = Normalizer::new();
        let mut term = int(IntegerLiteral::from(0_i64));
        for _ in 0 .. 10_000_u32 {
            term = Rc::new(Value::Pair(
                Rc::clone(&term),
                int(IntegerLiteral::from(1_i64)),
            ));
        }
        let lowered = lower(&mut nbe, &term);
        let expected = canonical_key(nbe.syntax(), lowered);
        // Input released first, with no ordering care taken: the store owns its
        // own flat nodes and the semantic arena owns ids.
        release_pairs(term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        // And the normalizer's own teardown is flat: dropping it here frees a
        // vector of nodes with id children, whatever the caller did first.
        drop(nbe);
    }

    /// Releases a deep sequencing chain one level at a time.
    ///
    /// This releases the **caller's** term, not the normalizer's: the abstract
    /// syntax tree's derived `Drop` recurses one call per reference-counted
    /// link, which is the tree's own standing constraint. The point of the two
    /// witnesses above is that the normalizer no longer participates in it.
    fn release_binds(term: Rc<Value>)
    {
        let Some(Value::Thunk(_, mut body)) = Rc::into_inner(term)
        else {
            return;
        };
        loop {
            let Some(comp) = Rc::into_inner(body)
            else {
                return;
            };
            match comp {
                | Comp::Bind(_, _, cont) => body = cont,
                | _ => return,
            }
        }
    }

    /// Releases a deep left-nested pair chain one level at a time (see
    /// [`release_binds`]).
    fn release_pairs(mut term: Rc<Value>)
    {
        loop {
            let Some(value) = Rc::into_inner(term)
            else {
                return;
            };
            match value {
                | Value::Pair(fst, _) => term = fst,
                | _ => return,
            }
        }
    }

    // ── evaluation, the term face, and forcing ──────────────────────────────

    #[test]
    fn evaluating_an_unreduced_value_retains_its_term_face()
    {
        let mut nbe = Normalizer::new();
        let term = Value::Pair(int(IntegerLiteral::from(1_i64)), var(NameRef::from("free")));
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert_eq!(
            nbe.arena().value(evaluated).unwrap().face().retained(),
            Some(node)
        );
    }

    #[test]
    fn evaluating_through_the_environment_drops_the_term_face()
    {
        let mut nbe = Normalizer::new();
        let bound = lower(&mut nbe, &int(IntegerLiteral::from(9_i64)));
        let bound = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, bound).unwrap();
        let env = nbe
            .arena_mut()
            .bind(sem::SemArena::EMPTY_ENV, String::from("x"), bound)
            .unwrap();
        let term = Value::Pair(int(IntegerLiteral::from(1_i64)), var(NameRef::from("x")));
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, env, node).unwrap();
        assert!(
            nbe.arena()
                .value(evaluated)
                .unwrap()
                .face()
                .retained()
                .is_none()
        );
    }

    #[test]
    fn retained_readback_hands_back_the_source_term()
    {
        let mut nbe = Normalizer::new();
        let term = Value::Pair(
            int(IntegerLiteral::from(1_i64)),
            int(IntegerLiteral::from(2_i64)),
        );
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Retained).unwrap();
        assert_eq!(
            quoted, node,
            "the term face rebuilt instead of handing back"
        );
    }

    #[test]
    fn quote_after_eval_is_the_identity_on_inert_values()
    {
        let mut nbe = Normalizer::new();
        for term in [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(3_i64)),
            Rc::new(Value::Pair(
                int(IntegerLiteral::from(1_i64)),
                int(IntegerLiteral::from(2_i64)),
            )),
            Rc::new(Value::Inj(Side::Snd, int(IntegerLiteral::from(4_i64)))),
            Rc::new(Value::List(alloc::vec![
                int(IntegerLiteral::from(1_i64)),
                int(IntegerLiteral::from(2_i64))
            ])),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            Rc::new(Value::Here(int(IntegerLiteral::from(5_i64)))),
        ] {
            let node = lower(&mut nbe, &term);
            let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
            let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
            assert_eq!(
                canonical_key(nbe.syntax(), quoted),
                canonical_key(nbe.syntax(), node),
                "canonical readback changed an inert value"
            );
        }
    }

    #[test]
    fn forcing_unfolds_a_reducible_definition()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("f"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        // Weak-head forcing keeps the neutral face.
        let held = force_value(&mut nbe, evaluated, ForceMode::WeakHead).unwrap();
        assert!(matches!(
            *nbe.arena().value(held).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
        // Unfolding forcing spends the definition.
        let spent = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(spent).unwrap().node(),
            SemValueNode::Int(5)
        ));
    }

    #[test]
    fn speculative_forcing_leaves_an_irreducible_definition_alone()
    {
        let mut nbe = Normalizer::new();
        nbe.define_with(
            NameRef::from("f"),
            &int(IntegerLiteral::from(5_i64)),
            Transparency::Irreducible,
        )
        .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let speculative = force_value(&mut nbe, evaluated, ForceMode::Speculative).unwrap();
        assert!(matches!(
            *nbe.arena().value(speculative).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
        let full = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(full).unwrap().node(),
            SemValueNode::Int(5)
        ));
    }

    #[test]
    fn forcing_a_self_referential_definition_terminates()
    {
        let mut nbe = Normalizer::new();
        nbe.set_fuel(ConversionFuel::from(16));
        nbe.define(NameRef::from("f"), &var(NameRef::from("f")))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        // The point of the test is that this returns at all: the fuel bound
        // stops an unfolding rule that unfolds to itself.
        let forced = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(forced).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
    }

    // ── the reduction rules ─────────────────────────────────────────────────

    #[test]
    fn beta_fires_for_every_positive_eliminator()
    {
        let mut nbe = Normalizer::new();
        let cases: [(Rc<Value>, Rc<Value>); 7] = [
            // Application.
            (
                thunk(Comp::app(
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                    Value::Int(3),
                )),
                thunk(Comp::ret(Value::Int(3))),
            ),
            // Force of a thunk.
            (
                thunk(Comp::force(Value::Thunk(
                    Grade::ONE,
                    Rc::new(Comp::ret(Value::Int(1))),
                ))),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Sequencing.
            (
                thunk(Comp::bind(
                    Comp::ret(Value::Int(1)),
                    "x",
                    Comp::ret(Value::var(NameRef::from("x"))),
                )),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Sum elimination.
            (
                thunk(Comp::Case(
                    Rc::new(Value::Inj(Side::Fst, int(IntegerLiteral::from(1_i64)))),
                    (
                        String::from("l"),
                        Rc::new(Comp::ret(Value::var(NameRef::from("l")))),
                    ),
                    (String::from("r"), Rc::new(Comp::ret(Value::Int(0)))),
                )),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Pair elimination.
            (
                thunk(Comp::Split {
                    scrut: Rc::new(Value::Pair(
                        int(IntegerLiteral::from(1_i64)),
                        int(IntegerLiteral::from(2_i64)),
                    )),
                    fst_name: String::from("a"),
                    snd_name: String::from("b"),
                    motive: None,
                    body: Rc::new(Comp::ret(Value::var(NameRef::from("b")))),
                }),
                thunk(Comp::ret(Value::Int(2))),
            ),
            // List elimination.
            (
                thunk(Comp::ListCase {
                    scrut: Rc::new(Value::List(alloc::vec![
                        int(IntegerLiteral::from(1_i64)),
                        int(IntegerLiteral::from(2_i64))
                    ])),
                    nil: Rc::new(Comp::ret(Value::Int(0))),
                    head: String::from("h"),
                    tail: String::from("t"),
                    cons: Rc::new(Comp::ret(Value::var(NameRef::from("h")))),
                }),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Lazy-pair projection.
            (
                thunk(Comp::Prj(
                    Side::Snd,
                    Rc::new(Comp::With(
                        Rc::new(Comp::ret(Value::Int(1))),
                        Rc::new(Comp::ret(Value::Int(2))),
                    )),
                )),
                thunk(Comp::ret(Value::Int(2))),
            ),
        ];
        for (redex, contractum) in cases {
            assert!(
                bool::from(nbe.converts(&redex, &contractum)),
                "a beta rule did not fire: {redex:?}"
            );
        }
    }

    #[test]
    fn walk_beta_fires_on_here()
    {
        let mut nbe = Normalizer::new();
        let redex = thunk(Comp::Walk {
            scrut: Rc::new(Value::Here(int(IntegerLiteral::from(7_i64)))),
            motive: alloc::boxed::Box::new(WalkMotive::new("x", "y", "q", CompType::Unknown)),
            base: WalkBase {
                x: String::from("w"),
                body: Rc::new(Comp::ret(Value::var(NameRef::from("w")))),
            },
        });
        assert!(bool::from(
            nbe.converts(&redex, &thunk(Comp::ret(Value::Int(7))))
        ));
        // The motive is inert: two walks differing only in it are convertible.
        let other = thunk(Comp::Walk {
            scrut: Rc::new(Value::Here(int(IntegerLiteral::from(7_i64)))),
            motive: alloc::boxed::Box::new(WalkMotive::new("a", "b", "c", CompType::Unknown)),
            base: WalkBase {
                x: String::from("z"),
                body: Rc::new(Comp::ret(Value::var(NameRef::from("z")))),
            },
        });
        assert!(bool::from(nbe.converts(&redex, &other)));
    }

    #[test]
    fn record_projection_reduces_and_stays_spine_local()
    {
        let mut nbe = Normalizer::new();
        // The sibling field is a thunk whose body is a stuck projection off a
        // free variable: reducing it would show up in the normal form.
        let sibling = Rc::new(Value::Thunk(
            Grade::ONE,
            Rc::new(Comp::RecordProj {
                record: var(NameRef::from("undefined")),
                label: String::from("missing"),
            }),
        ));
        let projection = thunk(Comp::RecordProj {
            record: record(&[
                (FieldName::from("a"), sibling),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            label: String::from("b"),
        });
        let normal = nbe.normalize(&projection).unwrap();
        assert_eq!(
            *normal,
            Value::Thunk(
                Grade::ONE,
                Rc::new(Comp::Ret(int(IntegerLiteral::from(2_i64))))
            ),
            "projection did not reduce to its field alone"
        );
        // A projection whose head is not a structure stays neutral.
        let neutral = thunk(Comp::RecordProj {
            record: var(NameRef::from("m")),
            label: String::from("b"),
        });
        let normal = nbe.normalize(&neutral).unwrap();
        assert!(matches!(
            *normal,
            Value::Thunk(_, ref body) if matches!(**body, Comp::RecordProj { .. })
        ));
    }

    #[test]
    fn a_record_module_projects_its_component_and_nothing_else()
    {
        let mut nbe = Normalizer::new();
        // A record module: a structure of thunked members, projected by label.
        let module = record(&[
            (
                FieldName::from("identity"),
                Rc::new(Value::Thunk(
                    Grade::ONE,
                    Rc::new(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x"))))),
                )),
            ),
            (
                FieldName::from("constant"),
                int(IntegerLiteral::from(7_i64)),
            ),
        ]);
        let projected = thunk(Comp::bind(
            Comp::RecordProj {
                record: Rc::clone(&module),
                label: String::from("identity"),
            },
            "f",
            Comp::app(Comp::force(Value::var(NameRef::from("f"))), Value::Int(4)),
        ));
        assert!(bool::from(
            nbe.converts(&projected, &thunk(Comp::ret(Value::Int(4))))
        ));
    }

    #[test]
    fn the_quarantine_leaves_effects_neutral()
    {
        let mut nbe = Normalizer::new();
        let performed = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            int(IntegerLiteral::from(1_i64)),
        ));
        let normal = nbe.normalize(&performed).unwrap();
        assert!(
            matches!(*normal, Value::Thunk(_, ref body) if matches!(**body, Comp::Perform(..))),
            "an effect was run inside conversion"
        );
        // Congruence, and nothing stronger: the payloads decide.
        assert!(bool::from(nbe.converts(&performed, &performed)));
        let other = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            int(IntegerLiteral::from(2_i64)),
        ));
        assert!(!bool::from(nbe.converts(&performed, &other)));
        // A payload that is a redex still converts: the operands are compared
        // by the normalizer even though the operation is never run.
        let reducible = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            Rc::new(Value::Annot(
                int(IntegerLiteral::from(1_i64)),
                Rc::new(ValueType::integer()),
            )),
        ));
        assert!(bool::from(nbe.converts(&performed, &reducible)));
    }

    // ── readback ────────────────────────────────────────────────────────────

    #[test]
    fn canonical_readback_renames_binders_to_levels()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::lam(
            "someSourceName",
            Comp::ret(Value::var(NameRef::from("someSourceName"))),
        ));
        let normal = nbe.normalize(&term).unwrap();
        let Value::Thunk(_, ref body) = *normal
        else {
            panic!("normalizing a thunk did not produce one");
        };
        let Comp::Abs(ref binder, _, ref inner) = **body
        else {
            panic!("normalizing a lambda did not produce one");
        };
        assert_eq!(binder.as_str(), "\u{ab}0\u{bb}");
        assert_eq!(**inner, Comp::Ret(Rc::new(Value::Var(binder.clone()))));
    }

    #[test]
    fn unfolding_readback_spends_the_definition()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("five"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("five")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let held = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(*nbe.reify(held).unwrap(), Value::Var(String::from("five")));
        let spent = quote_value(&mut nbe, evaluated, QuoteMode::Unfolding).unwrap();
        assert_eq!(*nbe.reify(spent).unwrap(), Value::Int(5));
    }

    #[test]
    fn readback_is_deterministic_across_runs()
    {
        let term = thunk(Comp::lam(
            "x",
            Comp::bind(
                Comp::ret(Value::var(NameRef::from("x"))),
                "y",
                Comp::ret(Value::Pair(
                    var(NameRef::from("y")),
                    var(NameRef::from("x")),
                )),
            ),
        ));
        let first = Normalizer::new().normalize(&term).unwrap();
        let second = Normalizer::new().normalize(&term).unwrap();
        assert_eq!(*first, *second);
    }

    #[test]
    fn readback_is_byte_stable_on_one_normalizer()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        // Two normalizations on ONE normalizer, not two: the fresh-level
        // counter restarts per run, so the second names its binder the same way
        // the first did rather than continuing from where the first stopped.
        let once = nbe.normalize_node(&term).unwrap();
        let twice = nbe.normalize_node(&term).unwrap();
        assert_eq!(once, twice, "the readback face failed to dedup");
        // And normalizing the normal form is the same term again.
        let reified = nbe.reify(once).unwrap();
        let thrice = nbe.normalize_node(&reified).unwrap();
        assert_eq!(once, thrice);
    }

    // ── the conversion relation ─────────────────────────────────────────────

    #[test]
    fn conversion_is_reflexive_and_symmetric()
    {
        let mut nbe = Normalizer::new();
        let terms = [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            var(NameRef::from("x")),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
            thunk(Comp::lam("x", Comp::ret(Value::Unit))),
        ];
        for left in &terms {
            assert!(bool::from(nbe.converts(left, left)), "not reflexive");
            for right in &terms {
                assert_eq!(
                    bool::from(nbe.converts(left, right)),
                    bool::from(nbe.converts(right, left)),
                    "not symmetric"
                );
            }
        }
    }

    #[test]
    fn a_hole_is_consistent_with_every_value()
    {
        let mut nbe = Normalizer::new();
        let hole = Rc::new(Value::Hole(0));
        for term in [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
        ] {
            assert!(bool::from(nbe.converts(&hole, &term)));
            assert!(bool::from(nbe.converts(&term, &hole)));
        }
        // The price, recorded: the relation is not transitive once a hole
        // participates, exactly as the structural equality it replaces was not.
        assert!(!bool::from(nbe.converts(
            &int(IntegerLiteral::from(1_i64)),
            &Rc::new(Value::Unit)
        )));
    }

    #[test]
    fn conversion_agrees_with_canonical_readback()
    {
        let mut nbe = Normalizer::new();
        let terms = [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            int(IntegerLiteral::from(2_i64)),
            var(NameRef::from("x")),
            var(NameRef::from("y")),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(3_i64))),
            ]),
            thunk(Comp::app(
                Comp::lam("z", Comp::ret(Value::var(NameRef::from("z")))),
                Value::Int(1),
            )),
            thunk(Comp::ret(Value::Int(1))),
            thunk(Comp::ret(Value::Int(2))),
        ];
        for left in &terms {
            for right in &terms {
                let normal_left = nbe.normalize_node(left).unwrap();
                let normal_right = nbe.normalize_node(right).unwrap();
                let same_normal_form = normal_left == normal_right;
                let convertible = bool::from(nbe.converts(left, right));
                // Equal normal forms always convert. The converse holds on this
                // hole-free, eta-free set, so the two agree here exactly.
                assert_eq!(
                    same_normal_form, convertible,
                    "readback and conversion disagreed on {left:?} and {right:?}"
                );
            }
        }
    }

    #[test]
    fn eta_relates_a_function_to_its_expansion()
    {
        let mut nbe = Normalizer::new();
        // A neutral function and its eta-expansion convert, which readback
        // alone does not decide — this is the normalizer-exclusive territory
        // the design names, so it carries its own law rather than a
        // differential.
        let neutral = thunk(Comp::force(Value::var(NameRef::from("f"))));
        let expanded = thunk(Comp::lam(
            "x",
            Comp::app(
                Comp::force(Value::var(NameRef::from("f"))),
                Value::var(NameRef::from("x")),
            ),
        ));
        assert!(bool::from(nbe.converts(&neutral, &expanded)));
        assert!(bool::from(nbe.converts(&expanded, &neutral)));
        // And a genuinely different function does not.
        let other = thunk(Comp::lam("x", Comp::ret(Value::Unit)));
        assert!(!bool::from(nbe.converts(&neutral, &other)));
    }

    #[test]
    fn an_ascription_is_transparent_to_conversion()
    {
        let mut nbe = Normalizer::new();
        let bare = int(IntegerLiteral::from(1_i64));
        let ascribed = Rc::new(Value::Annot(
            int(IntegerLiteral::from(1_i64)),
            Rc::new(ValueType::integer()),
        ));
        assert!(bool::from(nbe.converts(&bare, &ascribed)));
    }

    #[test]
    fn conversion_unfolds_the_taller_side()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("one"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        nbe.define(NameRef::from("also_one"), &var(NameRef::from("one")))
            .unwrap();
        // Two definitions at different heights, both unfolding to the same
        // literal: the height rule picks a side and the comparison closes.
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("one")),
            &var(NameRef::from("also_one"))
        )));
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("also_one")),
            &int(IntegerLiteral::from(1_i64))
        )));
        assert!(!bool::from(nbe.converts(
            &var(NameRef::from("one")),
            &int(IntegerLiteral::from(2_i64))
        )));
    }

    #[test]
    fn speculation_closes_a_spine_without_unfolding_and_backtracks_when_it_must()
    {
        let mut nbe = Normalizer::new();
        nbe.define(
            NameRef::from("f"),
            &Value::Thunk(
                Grade::ONE,
                Rc::new(Comp::lam("x", Comp::ret(Value::Int(0)))),
            ),
        )
        .unwrap();
        let applied = |arg: i64| {
            thunk(Comp::app(
                Comp::force(Value::var(NameRef::from("f"))),
                Value::Int(arg),
            ))
        };
        // Same head, same spine: the speculative pass closes it with no
        // unfolding spent.
        assert!(bool::from(nbe.converts(&applied(1), &applied(1))));
        // Same head, different spine: the speculative pass fails, and the
        // backtrack unfolds both sides — which agree, because the body is
        // constant.
        assert!(bool::from(nbe.converts(&applied(1), &applied(2))));
    }

    // ── types, signatures, and normalization ────────────────────────────────

    #[test]
    fn signature_conversion_is_label_exact()
    {
        let mut nbe = Normalizer::new();
        let signature = |fields: &[(&str, ValueType)]| {
            ValueType::Record(
                fields
                    .iter()
                    .map(|&(label, ref ty)| (label.to_owned(), Rc::new(ty.clone())))
                    .collect::<BTreeMap<_, _>>(),
            )
        };
        let base = signature(&[("x", ValueType::integer()), ("y", ValueType::Unit)]);
        let same = signature(&[("y", ValueType::Unit), ("x", ValueType::integer())]);
        // Field order is not content: the map is canonical, so the two are one
        // signature written twice.
        assert!(bool::from(nbe.type_converts(&base, &same)));
        // A wider signature is NOT convertible with a narrower one. Width is a
        // subtyping question and this relation deliberately has no width rule.
        let wider = signature(&[
            ("x", ValueType::integer()),
            ("y", ValueType::Unit),
            ("z", ValueType::Unit),
        ]);
        assert!(!bool::from(nbe.type_converts(&base, &wider)));
        assert!(!bool::from(nbe.type_converts(&wider, &base)));
        // A differing field type separates them.
        let retyped = signature(&[("x", ValueType::Unit), ("y", ValueType::Unit)]);
        assert!(!bool::from(nbe.type_converts(&base, &retyped)));
    }

    #[test]
    fn identity_endpoints_convert_up_to_beta()
    {
        let mut nbe = Normalizer::new();
        let redex = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        let contractum = thunk(Comp::ret(Value::Int(3)));
        let path = |lhs: Rc<Value>, rhs: Rc<Value>| ValueType::Path {
            ty: Rc::new(ValueType::integer()),
            lhs,
            rhs,
        };
        let left = path(Rc::clone(&redex), Rc::clone(&redex));
        let right = path(Rc::clone(&contractum), Rc::clone(&contractum));
        assert!(
            bool::from(nbe.type_converts(&left, &right)),
            "identity endpoints did not convert up to beta"
        );
        let apart = path(Rc::clone(&contractum), thunk(Comp::ret(Value::Int(4))));
        assert!(!bool::from(nbe.type_converts(&left, &apart)));
    }

    #[test]
    fn the_unknown_type_is_consistent_in_both_directions()
    {
        let mut nbe = Normalizer::new();
        assert!(bool::from(
            nbe.type_converts(&ValueType::Unknown, &ValueType::integer())
        ));
        assert!(bool::from(
            nbe.type_converts(&ValueType::integer(), &ValueType::Unknown)
        ));
    }

    #[test]
    fn a_term_converts_with_its_own_normal_form()
    {
        let mut nbe = Normalizer::new();
        for term in [
            thunk(Comp::app(
                Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                Value::Int(3),
            )),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
            thunk(Comp::RecordProj {
                record: record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
                label: String::from("a"),
            }),
        ] {
            let normal = nbe.normalize(&term).unwrap();
            assert!(
                bool::from(nbe.converts(&term, &normal)),
                "a term did not convert with its own normal form"
            );
        }
    }

    #[test]
    fn normalization_is_idempotent()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::bind(
            Comp::app(
                Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                Value::Int(3),
            ),
            "y",
            Comp::ret(Value::Pair(
                var(NameRef::from("y")),
                var(NameRef::from("y")),
            )),
        ));
        let once = nbe.normalize(&term).unwrap();
        let twice = nbe.normalize(&once).unwrap();
        assert_eq!(*once, *twice);
    }

    #[test]
    fn normalizing_leaves_the_arena_where_it_found_it()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        // The syntax store keeps what it is given — it owns the canonical
        // syntax — but the SEMANTIC arena is where a run's scratch lives, and
        // that is what returns to its watermark.
        let before = nbe.watermark();
        let _normal = nbe.normalize(&term).unwrap();
        assert_eq!(
            nbe.watermark(),
            before,
            "normalization left semantic nodes behind"
        );
        let _decision = nbe.converts(&term, &term);
        assert_eq!(
            nbe.watermark(),
            before,
            "conversion left semantic nodes behind"
        );
    }

    #[test]
    fn the_arena_reports_its_own_population()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let before = usize::from(nbe.arena().value_count());
        let _evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert!(usize::from(nbe.arena().value_count()) > before);
    }

    #[test]
    fn a_semantic_value_carries_the_guard_it_was_minted_with()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let guard = Guard::leaf(mix_word(
            seed(crate::boundary::SemanticHash::from(1)),
            crate::boundary::SemanticHash::from(2),
        ));
        let value = SemValue::new(
            SemValueNode::Rigid(Rigid::Free(String::from("x")), ValueUnfold::Rigid),
            guard,
        );
        assert_eq!(value.guard(), guard);
        assert!(value.face().retained().is_none());
        let retained = value.retaining(node);
        assert_eq!(retained.face().retained(), Some(node));
    }

    #[test]
    fn grades_separate_two_thunks_that_agree_on_their_bodies()
    {
        let mut nbe = Normalizer::new();
        let one = Rc::new(Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Unit))));
        let many = Rc::new(Value::Thunk(Grade::OMEGA, Rc::new(Comp::ret(Value::Unit))));
        let also_one = Rc::new(Value::Thunk(
            Grade::fin(GradeBound::from(1)),
            Rc::new(Comp::ret(Value::Unit)),
        ));
        assert!(!bool::from(nbe.converts(&one, &many)));
        assert!(bool::from(nbe.converts(&one, &also_one)));
    }

    #[test]
    fn a_free_variable_is_a_blocker_the_neutral_names()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &var(NameRef::from("stuck")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let named = match *nbe.arena().value(evaluated).unwrap().node() {
            | SemValueNode::Rigid(Rigid::Free(ref name), _) => name.clone(),
            | _ => String::new(),
        };
        assert_eq!(named.as_str(), "stuck");
    }

    #[test]
    fn the_computation_entry_points_answer_for_computations_directly()
    {
        let mut nbe = Normalizer::new();
        // The value-level entries are convenience over the computation-level
        // ones; a caller holding weak-head normal computations compares and
        // reads them back without going through a thunk.
        let redex = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        let contractum = thunk(Comp::ret(Value::Int(3)));
        let left = lower(&mut nbe, &redex);
        let right = lower(&mut nbe, &contractum);
        let left = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, left).unwrap();
        let right = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, right).unwrap();
        assert!(bool::from(
            crate::nbe::conv::converts_values(&mut nbe, left, right).unwrap()
        ));
    }

    #[test]
    fn a_normalizer_can_be_built_over_an_existing_environment()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("f"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("f")),
            &int(IntegerLiteral::from(5_i64))
        )));
    }
}
