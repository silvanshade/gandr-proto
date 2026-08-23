//! The differential gate: incremental validated resume ≡ from-scratch
//! re-typing (`incremental-pipeline.md` §"Checkpoints and the reuse rule"
//! through §"Derivation merging and identity stability").
//!
//! The theorem `gandr_core_incremental::checkpoint::resume` must satisfy:
//! for **every** edit, the incrementally-resumed per-item typing equals the
//! typing a full from-scratch re-type of the edited program produces. Adoption
//! (reusing a validated checkpoint) skips work; this gate proves the skips
//! never change the answer.
//!
//! The front end is a `ToySurface` — an in-tree test double implementing the
//! parser-agnostic [`ItemSource`] seam. It lowers a compact statement model
//! (integer / string literals, name references, integer-ascribed references,
//! and an optional item ascription) to the same core [`Item`]s a real parser
//! would, so the gate exercises the checkpoint engine without a parser
//! (`gandr-surface-engine` supplies the real front end, and gates it over real
//! source).
//!
//! # What each step of the gate asserts
//!
//! Every resume the gate performs runs the same four assertions, so a fixed
//! witness and a property-generated edit are held to one standard:
//!
//! - **Zero drift** — the resumed typings equal a from-scratch re-type.
//! - **Precision** — every adopted item's footprint is non-opaque, agrees with
//!   a fresh scan of its term, and reads only names bound to the same thing at
//!   the item's new position as at its old one. Conservatism is permitted;
//!   silent over-adoption is not.
//! - **The persisted round trip** — the resulting checkpoint set survives an
//!   encode/decode through the persistence codec unchanged, and the *decoded*
//!   set is what the next step of an edit sequence resumes from, so drift
//!   across persistence is gated rather than assumed.
//! - **One adoption flag per typing.**
//!
//! # The test classes
//!
//! `adoption`, `invalidation`, and `structure` are fixed witnesses for the
//! reuse rule, the re-typing a dependency change forces, and item-list edits.
//! `property` generates programs and edits — single edits and sequences of them
//! with a resume at every step. `teeth` proves the differential can fail by
//! seeding corruptions it must catch. `divergence` pins two inputs on which the
//! engine is known to be wrong today (`gandr-t8j6`).
//!
//! [`ItemSource`]: gandr_core_incremental::region::ItemSource
//! [`Item`]: gandr_core_incremental::region::Item

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code"
    )
)]

/// The differential gate for `gandr_core_incremental::checkpoint`.
#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::rc::Rc;

    use gandr_core_incremental::checkpoint::Checkpoints;
    use gandr_core_incremental::checkpoint::ItemTyping;
    use gandr_core_incremental::checkpoint::Resume;
    use gandr_core_incremental::checkpoint::checkpoint_program;
    use gandr_core_incremental::checkpoint::resume;
    use gandr_core_incremental::footprint::footprint_of;
    use gandr_core_incremental::persistence::decode_checkpoints;
    use gandr_core_incremental::persistence::encode_checkpoints;
    use gandr_core_incremental::region::Item;
    use gandr_core_incremental::region::ItemSource;
    use gandr_core_incremental::region::Program;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;
    use proptest::test_runner::TestCaseError;

    /// One definition body the toy surface can lower — enough scalar and
    /// reference shapes to drive body edits, type changes, and downstream
    /// errors without a parser.
    #[derive(Clone, Debug)]
    enum Body
    {
        /// An integer literal `n` (infers the rigid `Integer` atom).
        Int(i64),
        /// A string literal `s` (infers the rigid `String` atom) — the
        /// type-changing counterpart of [`Body::Int`].
        Str(String),
        /// A bare reference to a name — reads it from the ambient context, so
        /// its footprint is `{name}` and its type moves with the referent's.
        Ref(String),
        /// An integer-ascribed reference `(name : Integer)` — types iff `name`
        /// is `Integer`, so a change that retypes `name` to a non-integer makes
        /// this item ill-typed downstream.
        CheckInteger(String),
    }

    /// The ascription a statement carries — the third component of an item's
    /// identity, beside its name and its lowered term.
    ///
    /// **The vocabulary is deliberately restricted to atom types**, and the
    /// restriction is load-bearing rather than incidental. A type whose
    /// comparison consults definitional equality — a `Path`'s endpoints, or any
    /// type carrying a value position — reaches the over-adoption defect
    /// recorded as `gandr-t8j6`, so a generated program using one would fail
    /// nondeterministically and take the merge wall with it. Atom comparison is
    /// name equality and never consults the normalizer, so nothing reachable
    /// from here can enter that class. The defect itself is covered by the
    /// deterministic witnesses in `divergence`. **Widen this vocabulary when
    /// `gandr-t8j6` closes.**
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Ascription
    {
        /// The rigid `Integer` atom.
        Integer,
        /// The rigid `String` atom.
        Text,
    }

    /// One top-level statement: an optional definition name, an optional
    /// ascription, and its body.
    #[derive(Clone, Debug)]
    struct Stmt
    {
        /// The defined name (`def name = …`); [`None`] for a bare expression.
        name: Option<String>,
        /// The recorded ascription (`def name : ty = …`).
        ascription: Option<Ascription>,
        /// The statement body.
        body: Body,
    }

    /// How many adopted items one precision probe examined — the telemetry
    /// that keeps the probe from being silently vacuous.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
    struct ProbedAdoptionCount(usize);

    /// A position within a program or a checkpoint set, in source order.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct ItemPosition(usize);

    /// What one differential step observed.
    struct StepOutcome
    {
        /// The resume the step performed.
        resumed: Resume,
        /// The checkpoint set the next step resumes from — the one recovered
        /// from the persisted round trip, not the in-memory original.
        checkpoints: Checkpoints,
        /// How many adopted items the precision probe examined.
        probed: ProbedAdoptionCount,
    }

    /// The in-tree test double for the parser-agnostic [`ItemSource`] seam,
    /// standing in for the surface lane's real lowering front end so this gate
    /// runs without one. Its revision is a statement slice, and it has no
    /// input-independent failure residue, so its error is
    /// [`core::convert::Infallible`].
    struct ToySurface;

    impl ItemSource for ToySurface
    {
        type Error = core::convert::Infallible;
        type Revision = [Stmt];

        fn items(
            &self,
            revision: &[Stmt],
        ) -> Result<Program, Self::Error>
        {
            Ok(revision.iter().map(lower_stmt).collect())
        }
    }

    /// The toy front end's items for `revision`, with its uninhabited failure
    /// residue discharged by a total match rather than an unwrap.
    fn items_of(revision: &[Stmt]) -> Program
    {
        match ToySurface.items(revision) {
            | Ok(program) => program,
            | Err(never) => match never {},
        }
    }

    /// The core type one ascription denotes.
    fn ascription_type(ascription: Ascription) -> Ty
    {
        match ascription {
            | Ascription::Integer => Ty::Value(ValueType::integer()),
            | Ascription::Text => Ty::Value(ValueType::string()),
        }
    }

    /// Lowers one statement to a core [`Item`] — the toy analogue of the
    /// surface lane's CST → core lowering.
    fn lower_stmt(stmt: &Stmt) -> Item
    {
        let term = match stmt.body {
            | Body::Int(literal) => Term::Value(Value::int(literal)),
            | Body::Str(ref literal) => Term::Value(Value::string(literal)),
            | Body::Ref(ref name) => Term::Value(Value::var(name)),
            | Body::CheckInteger(ref name) => Term::Value(Value::Annot(
                Rc::new(Value::var(name)),
                Rc::new(ValueType::integer()),
            )),
        };
        Item::new(
            stmt.name.clone(),
            stmt.ascription.map(ascription_type),
            term,
        )
    }

    /// Builds an unascribed `def name = body` statement.
    fn def(
        name: impl Into<String>,
        body: Body,
    ) -> Stmt
    {
        Stmt {
            name: Some(name.into()),
            ascription: None,
            body,
        }
    }

    /// Builds an ascribed `def name : ascription = body` statement.
    fn ascribed_def(
        name: impl Into<String>,
        ascription: Ascription,
        body: Body,
    ) -> Stmt
    {
        Stmt {
            name: Some(name.into()),
            ascription: Some(ascription),
            body,
        }
    }

    /// Reports a differential failure as a proptest case failure, so a property
    /// run shrinks it and a fixed witness reports it directly.
    fn failure(message: String) -> TestCaseError
    {
        TestCaseError::fail(message)
    }

    /// Runs the gate: checkpoints `base`, resumes onto `edited`, and asserts
    /// every step obligation — zero drift, precision, and the persisted round
    /// trip. Returns the resume so a caller can additionally assert *which*
    /// items were adopted.
    fn gate(
        base: &[Stmt],
        edited: &[Stmt],
    ) -> Resume
    {
        let base_program = items_of(base);
        let edited_program = items_of(edited);
        let checkpoints = checkpoint_program(&base_program);
        match resume_step(&checkpoints, &edited_program) {
            | Ok(outcome) => outcome.resumed,
            | Err(error) => panic!("the differential step must hold: {error:?}"),
        }
    }

    /// One step of the differential: resume `base` onto `edited` and discharge
    /// every obligation the gate states, returning what the step observed.
    fn resume_step(
        base: &Checkpoints,
        edited: &Program,
    ) -> Result<StepOutcome, TestCaseError>
    {
        let resumed = resume(base, edited);
        let expected = from_scratch(edited);
        let actual: Vec<ItemTyping> = resumed.typings().cloned().collect();
        if actual != expected {
            return Err(failure(format!(
                "incremental resume must equal from-scratch re-typing\n  resumed:      {actual:?}\n  from scratch: {expected:?}"
            )));
        }
        if resumed.typings().len() != resumed.adopted().len() {
            return Err(failure("one adoption flag per typing".to_owned()));
        }
        let probed = probe_precision(base, edited, &resumed)?;
        let checkpoints = persisted_round_trip(resumed.checkpoints())?;
        Ok(StepOutcome {
            resumed,
            checkpoints,
            probed,
        })
    }

    /// The per-item typings a from-scratch re-type of `program` produces.
    fn from_scratch(program: &Program) -> Vec<ItemTyping>
    {
        checkpoint_program(program)
            .items
            .into_iter()
            .map(|checkpoint| checkpoint.typing)
            .collect()
    }

    /// The value type a checkpoint's typing contributes to the context.
    fn bound_type(typing: &ItemTyping) -> Option<ValueType>
    {
        match *typing {
            | ItemTyping::Definition {
                ty: Ty::Value(ref value_type),
                bound: true,
                ..
            } => Some(value_type.clone()),
            | _ => None,
        }
    }

    /// The bindings visible to the item at `position` — what the items before
    /// it thread into the typing context, in source order, a later definition
    /// shadowing an earlier one of the same name.
    ///
    /// This mirrors `thread_binding`: only a definition whose typing is a bound
    /// value type contributes, so an expression, a type error, a hole, and a
    /// bare computation type each contribute nothing.
    fn visible_bindings(
        checkpoints: &Checkpoints,
        position: ItemPosition,
    ) -> BTreeMap<String, ValueType>
    {
        let mut visible: BTreeMap<String, ValueType> = BTreeMap::new();
        for checkpoint in checkpoints.items.iter().take(position.0) {
            if let Some(name) = checkpoint.name.as_ref()
                && let Some(binding) = bound_type(&checkpoint.typing)
            {
                let _replaced = visible.insert(name.clone(), binding);
            }
        }
        visible
    }

    /// Where an item named `name` sat in the base checkpoint set, when exactly
    /// one base checkpoint carries that name.
    ///
    /// A duplicated or absent name yields [`None`], and the probe then declines
    /// to make a claim about that adoption rather than guessing which base
    /// checkpoint it came from.
    fn unique_base_position(
        base: &Checkpoints,
        name: Option<&String>,
    ) -> Option<ItemPosition>
    {
        let mut found: Option<ItemPosition> = None;
        for (index, checkpoint) in base.items.iter().enumerate() {
            if checkpoint.name.as_ref() == name {
                if found.is_some() {
                    return None;
                }
                found = Some(ItemPosition(index));
            }
        }
        found
    }

    /// The precision probe: every adopted item must have earned its adoption.
    ///
    /// # What it asserts
    ///
    /// For each adopted item: its footprint is not opaque, the footprint the
    /// engine recorded agrees with a fresh independent scan of its term, and —
    /// the substance — **every name it reads is bound to the same thing at its
    /// new position as it was at its old one.** That is the adoption rule's own
    /// licensing condition, re-derived from outside the engine: an item whose
    /// context agrees on everything it reads types identically, so its cached
    /// answer is still correct.
    ///
    /// Conservatism is permitted throughout: an item that could have been
    /// adopted and was not costs reuse and nothing else. What this rejects is
    /// the opposite — an adoption nothing licenses, which is silent
    /// over-adoption and yields a wrong answer.
    ///
    /// # Why the comparison is positional rather than a global changed set
    ///
    /// A global "names this edit changed" set is the obvious formulation and it
    /// produces false positives, which is worse than useless in a probe: the
    /// first one reads as a zero-drift defect. The measured case is the
    /// append fast path (`resume_appended`), which adopts an unchanged prefix
    /// on structural identity alone, without consulting any changed set —
    /// soundly, because items appended *after* a prefix cannot alter how
    /// that prefix types. A global set flags those adoptions; comparing the
    /// bindings visible *at each item's own position* does not, because it
    /// asks the question the soundness argument actually turns on.
    ///
    /// The positional form is also strictly more precise than a set: it catches
    /// a shadowing insertion that changes what a name resolves to at one
    /// position while leaving the program-wide binding table intact.
    ///
    /// # What this probe cannot see
    ///
    /// **A probe built this way checks the adoption rule's execution, not the
    /// rule.** Bindings are compared as the engine tracks them — by bound value
    /// type — because a probe that compared anything more would flag the
    /// type-stable dependent reuse the crate performs correctly. A defect in
    /// the rule itself is therefore invisible here by construction, and one
    /// exists: `gandr-t8j6`, where a definition's *value* is consulted during
    /// typing while only its type is tracked. Nothing but the
    /// incremental-equals-batch comparison catches that class, which is why the
    /// comparison, not this probe, is the gate's primary assertion.
    fn probe_precision(
        base: &Checkpoints,
        edited: &Program,
        resumed: &Resume,
    ) -> Result<ProbedAdoptionCount, TestCaseError>
    {
        let result = resumed.checkpoints();
        let mut probed = ProbedAdoptionCount::default();
        for (index, decision) in resumed.adopted().enumerate() {
            if !bool::from(decision) {
                continue;
            }
            let Some(item) = edited.items.get(index)
            else {
                return Err(failure(format!(
                    "the adoption flag at {index} has no edited item"
                )));
            };
            let Some(checkpoint) = result.items.get(index)
            else {
                return Err(failure(format!(
                    "the adoption flag at {index} has no checkpoint"
                )));
            };
            let footprint = footprint_of(&item.term);
            if checkpoint.footprint != footprint {
                return Err(failure(format!(
                    "the footprint recorded for the adopted item at {index} disagrees with a fresh scan of its term"
                )));
            }
            if footprint.opaque {
                return Err(failure(format!(
                    "an opaque footprint reads everything and must never be adopted, but the item at {index} was"
                )));
            }
            let Some(origin) = unique_base_position(base, item.name.as_ref())
            else {
                continue;
            };
            let visible_before = visible_bindings(base, origin);
            let visible_now = visible_bindings(result, ItemPosition(index));
            for name in &footprint.names {
                if visible_before.get(name) != visible_now.get(name) {
                    return Err(failure(format!(
                        "over-adoption: the item at {index} was adopted while reading `{name}`, which is bound to {:?} at its new position and was bound to {:?} at its old one",
                        visible_now.get(name),
                        visible_before.get(name)
                    )));
                }
            }
            probed.0 = probed.0.saturating_add(1);
        }
        Ok(probed)
    }

    /// Puts the persistence codec in the differential loop: encode, decode, and
    /// require the recovered checkpoint set to be the one that went in.
    ///
    /// The recovered set is what the caller resumes from next, so an edit
    /// sequence proves drift-freedom **across persistence** rather than only in
    /// memory. Every form the toy surface produces is codec-supported, so an
    /// encoding failure is a defect here, never a skip.
    fn persisted_round_trip(checkpoints: &Checkpoints) -> Result<Checkpoints, TestCaseError>
    {
        let bytes = match encode_checkpoints(checkpoints) {
            | Ok(bytes) => bytes,
            | Err(error) => {
                return Err(failure(format!(
                    "the codec must encode every checkpoint the toy surface produces, but reported {error:?}"
                )));
            },
        };
        let decoded = match decode_checkpoints(&bytes) {
            | Ok(decoded) => decoded,
            | Err(error) => {
                return Err(failure(format!(
                    "a canonical encoding must decode, but reported {error:?}"
                )));
            },
        };
        if decoded != *checkpoints {
            return Err(failure(
                "the persisted round trip must preserve the checkpoint set exactly".to_owned(),
            ));
        }
        Ok(decoded)
    }

    /// The reuse the trail-aware footprint buys.
    mod adoption
    {
        use super::*;

        /// A body edit that keeps the definition's *type* leaves its dependent
        /// adoptable: `target`'s literal changes but its type does not, so the
        /// dependent `d = target` is **adopted**, not re-typed. This is the
        /// §"Checkpoints and the reuse rule" / §"The edit loop" refinement:
        /// reuse keyed on whether the binding changed, not on whether an
        /// upstream item was edited.
        #[test]
        fn body_edit_adopts_the_type_stable_dependent()
        {
            let base = [
                def("target", Body::Int(1)),
                def("d", Body::Ref("target".to_owned())),
            ];
            let edited = [
                def("target", Body::Int(2)),
                def("d", Body::Ref("target".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert_eq!(2, resumed.adopted().len(), "two items");
            assert!(!adopted[0], "the edited definition `target` is re-typed");
            assert!(
                adopted[1],
                "the type-stable dependent `d = target` is adopted, not re-typed"
            );
        }

        /// Inserting a definition re-types only the insert; the untouched
        /// neighbours (which do not read it) are adopted.
        #[test]
        fn insertion_adopts_untouched_neighbours()
        {
            let base = [def("a", Body::Int(1)), def("c", Body::Int(3))];
            let edited = [
                def("a", Body::Int(1)),
                def("b", Body::Int(2)),
                def("c", Body::Int(3)),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert_eq!(adopted, vec![true, false, true], "only `b` is fresh");
            assert_eq!(
                2,
                usize::from(resumed.adopted_count()),
                "`a` and `c` reused"
            );
        }

        /// A no-op edit (identical source) adopts every item.
        #[test]
        fn noop_edit_adopts_everything()
        {
            let source = [
                def("a", Body::Int(1)),
                def("b", Body::Ref("a".to_owned())),
                def("c", Body::Ref("b".to_owned())),
            ];
            let resumed = gate(&source, &source);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(
                adopted.iter().all(|&adopted| adopted),
                "an identity edit reuses everything: {adopted:?}"
            );
        }
        /// An append leaves every prior checkpoint adopted and types only the
        /// new dirty tail against the accumulated prefix context.
        #[test]
        fn append_reuses_the_prefix_and_retypes_the_tail()
        {
            let base = [def("x", Body::Int(1)), def("y", Body::Ref("x".to_owned()))];
            let edited = [
                def("x", Body::Int(1)),
                def("y", Body::Ref("x".to_owned())),
                def("z", Body::Ref("y".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert_eq!(
                adopted,
                vec![true, true, false],
                "an append adopts the prefix and re-types only its dirty tail"
            );
        }

        /// The precision probe is meaningless if adoption never happens, and a
        /// generator or engine change could turn every probe into a no-op
        /// without any test noticing. This pins the probe to a real adoption:
        /// a type-stable body edit leaves exactly one item adopted, and the
        /// probe must have examined it.
        #[test]
        fn the_precision_probe_examines_real_adoptions()
        {
            let base = [
                def("target", Body::Int(1)),
                def("reader", Body::Ref("target".to_owned())),
            ];
            let edited = [
                def("target", Body::Int(2)),
                def("reader", Body::Ref("target".to_owned())),
            ];
            let checkpoints = checkpoint_program(&items_of(&base));
            let outcome = match resume_step(&checkpoints, &items_of(&edited)) {
                | Ok(outcome) => outcome,
                | Err(error) => panic!("the differential step must hold: {error:?}"),
            };

            assert_eq!(
                1, outcome.probed.0,
                "the probe must have examined the adopted `reader`"
            );
            assert_eq!(
                1,
                usize::from(outcome.resumed.adopted_count()),
                "exactly one item is adopted"
            );
        }
    }

    /// The re-typing a real dependency change forces.
    mod invalidation
    {
        use super::*;

        /// A type-changing edit to `x` re-types the downstream reader `y`: the
        /// footprint of `y` reads `x`, whose binding changed (Integer ⇒
        /// String), so its checkpoint is invalidated — and the resumed
        /// typing still equals from-scratch.
        #[test]
        fn type_change_retypes_the_dependent()
        {
            let base = [def("x", Body::Int(1)), def("y", Body::Ref("x".to_owned()))];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::Ref("x".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(!adopted[0], "the edited `x` is re-typed");
            assert!(
                !adopted[1],
                "the dependent `y` reads the changed binding `x`, so it is re-typed"
            );
            let typings: Vec<&ItemTyping> = resumed.typings().collect();
            match (typings[0], typings[1]) {
                | (
                    &ItemTyping::Definition {
                        name: ref x_name, ..
                    },
                    &ItemTyping::Definition {
                        name: ref y_name, ..
                    },
                ) => {
                    assert_eq!("x", x_name);
                    assert_eq!("y", y_name);
                },
                | other => panic!("expected two definitions, got {other:?}"),
            }
        }

        /// An edit that makes a downstream item ill-typed surfaces the error
        /// exactly as from-scratch would (the resume never masks a new error by
        /// adopting a stale success): `(x : Integer)` types while `x` is
        /// `Integer`, and errors once `x` becomes `String`.
        #[test]
        fn downstream_error_surfaces()
        {
            let base = [
                def("x", Body::Int(1)),
                def("y", Body::CheckInteger("x".to_owned())),
            ];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::CheckInteger("x".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(
                !adopted[1],
                "the dependent `y` is re-typed against the changed `x`"
            );
            let typings: Vec<&ItemTyping> = resumed.typings().collect();
            assert!(
                matches!(typings[1], ItemTyping::TypeError { .. }),
                "the ascription `(x : Integer)` fails once `x` is a String: {:?}",
                typings[1]
            );
        }

        /// Adding an ascription that the body does not satisfy re-types the
        /// item into an error, and the differential holds across it. The
        /// ascription is one of the three components of an item's identity, so
        /// changing it alone must invalidate that item's checkpoint.
        #[test]
        fn ascription_change_alone_invalidates_the_item()
        {
            let base = [def("a", Body::Int(1)), def("b", Body::Int(2))];
            let edited = [
                ascribed_def("a", Ascription::Text, Body::Int(1)),
                def("b", Body::Int(2)),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(
                !adopted[0],
                "the item whose ascription changed cannot keep its checkpoint"
            );
            assert!(
                adopted[1],
                "`b` reads nothing that changed, so it is reused"
            );
            let typings: Vec<&ItemTyping> = resumed.typings().collect();
            assert!(
                matches!(typings[0], ItemTyping::TypeError { .. }),
                "an integer literal does not satisfy a String ascription: {:?}",
                typings[0]
            );
        }

        /// A satisfied ascription still types, and its dependents behave as the
        /// unascribed case does — the ascription costs no reuse of its own.
        #[test]
        fn satisfied_ascription_types_and_keeps_dependents_adoptable()
        {
            let base = [
                ascribed_def("a", Ascription::Integer, Body::Int(1)),
                def("b", Body::Ref("a".to_owned())),
            ];
            let edited = [
                ascribed_def("a", Ascription::Integer, Body::Int(7)),
                def("b", Body::Ref("a".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(!adopted[0], "the edited `a` is re-typed");
            assert!(
                adopted[1],
                "`a` keeps its type under the ascription, so `b` stays adoptable"
            );
        }
    }

    /// Item-list edits.
    mod structure
    {
        use super::*;

        /// Deleting a definition matches from-scratch, and the survivors that
        /// do not read it are adopted.
        #[test]
        fn deletion_matches_from_scratch()
        {
            let base = [
                def("a", Body::Int(1)),
                def("b", Body::Int(2)),
                def("c", Body::Int(3)),
            ];
            let edited = [def("a", Body::Int(1)), def("c", Body::Int(3))];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert_eq!(adopted, vec![true, true], "both survivors reused");
        }

        /// Renaming a definition is a delete-plus-insert: the renamed item is
        /// fresh, and a definition it does not read is adopted. The gate holds
        /// regardless.
        #[test]
        fn rename_matches_from_scratch()
        {
            let base = [def("foo", Body::Int(1)), def("keep", Body::Int(9))];
            let edited = [def("bar", Body::Int(1)), def("keep", Body::Int(9))];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(adopted[1], "`keep` is adopted across the rename");
        }

        /// A **coordinated** rename — the definition and every reader renamed
        /// in one edit — invalidates and re-binds in a single step. The old
        /// name leaves scope and the new one enters it, so no item touching
        /// either may be adopted, while an item mentioning neither is.
        #[test]
        fn coordinated_rename_rebinds_every_reader()
        {
            let base = [
                def("old", Body::Int(1)),
                def("reader", Body::Ref("old".to_owned())),
                def("bystander", Body::Int(9)),
            ];
            let edited = [
                def("new", Body::Int(1)),
                def("reader", Body::Ref("new".to_owned())),
                def("bystander", Body::Int(9)),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(!adopted[0], "the renamed definition is fresh");
            assert!(
                !adopted[1],
                "the reader's term changed, so it cannot keep its checkpoint"
            );
            assert!(
                adopted[2],
                "`bystander` mentions neither name, so it is reused"
            );
            let typings: Vec<&ItemTyping> = resumed.typings().collect();
            assert!(
                matches!(typings[1], ItemTyping::Definition { .. }),
                "the reader re-binds to the new name rather than dangling: {:?}",
                typings[1]
            );
        }

        /// A coordinated rename that leaves a reader behind is a dangling
        /// reference, and the resume must report exactly what from-scratch
        /// does rather than adopting the stale success.
        #[test]
        fn uncoordinated_rename_leaves_a_dangling_reader()
        {
            let base = [
                def("old", Body::Int(1)),
                def("reader", Body::CheckInteger("old".to_owned())),
            ];
            let edited = [
                def("new", Body::Int(1)),
                def("reader", Body::CheckInteger("old".to_owned())),
            ];
            let resumed = gate(&base, &edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();

            assert!(
                !adopted[1],
                "the reader reads `old`, which left scope, so it is re-typed"
            );
        }

        /// Swapping two independent definitions changes the order without
        /// changing any binding. The differential holds, which is the whole
        /// claim — order sensitivity is where an identity keyed on position
        /// rather than on content would break.
        #[test]
        fn independent_swap_matches_from_scratch()
        {
            let base = [
                def("a", Body::Int(1)),
                def("b", Body::Str("s".to_owned())),
                def("c", Body::Int(3)),
            ];
            let edited = [
                def("c", Body::Int(3)),
                def("b", Body::Str("s".to_owned())),
                def("a", Body::Int(1)),
            ];
            let _resumed = gate(&base, &edited);
        }

        /// Swapping a definition past its own reader turns a valid program into
        /// a forward reference. The resume must surface that, not adopt the
        /// reader's cached success.
        #[test]
        fn swap_past_a_reader_matches_from_scratch()
        {
            let base = [
                def("x", Body::Int(1)),
                def("y", Body::CheckInteger("x".to_owned())),
            ];
            let edited = [
                def("y", Body::CheckInteger("x".to_owned())),
                def("x", Body::Int(1)),
            ];
            let resumed = gate(&base, &edited);
            let typings: Vec<&ItemTyping> = resumed.typings().collect();

            assert!(
                matches!(typings[0], ItemTyping::TypeError { .. }),
                "`y` now precedes `x`, so its reference is unbound: {:?}",
                typings[0]
            );
        }
    }

    /// Seeded corruptions the differential must catch.
    ///
    /// A differential that cannot fail proves nothing, and the failure mode is
    /// silent: a gate whose assertion has drifted out of reach keeps passing
    /// and keeps being cited. Each test here corrupts a checkpoint set in a way
    /// that is invisible to the adoption rule — the corrupted item is still
    /// adopted — and requires the comparison against from-scratch to detect it.
    /// These are permanent suite members, not one-off demonstrations.
    mod teeth
    {
        use super::*;

        /// A stale cached typing rides through an adoption, and the
        /// differential catches it.
        ///
        /// The corruption keeps the checkpoint's identity (name, ascription,
        /// term) intact and changes only the answer it caches, so every
        /// structural test the adoption rule performs still passes. Only the
        /// comparison against a from-scratch re-type can see it.
        #[test]
        fn a_stale_cached_typing_is_caught()
        {
            let source = [def("a", Body::Int(1)), def("b", Body::Int(2))];
            let program = items_of(&source);

            let honest = gate(&source, &source);
            assert_eq!(
                2,
                usize::from(honest.adopted_count()),
                "the uncorrupted identity edit adopts both items"
            );

            let mut checkpoints = checkpoint_program(&program);
            if let Some(checkpoint) = checkpoints.items.get_mut(0) {
                checkpoint.typing = ItemTyping::Definition {
                    name: "a".to_owned(),
                    ty: Ty::Value(ValueType::string()),
                    bound: true,
                };
            }
            let resumed = resume(&checkpoints, &program);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();
            let actual: Vec<ItemTyping> = resumed.typings().cloned().collect();

            assert!(
                adopted[0],
                "the corrupted checkpoint is still adopted, which is what makes it dangerous"
            );
            assert_ne!(
                actual,
                from_scratch(&program),
                "the differential must catch a stale cached typing riding through an adoption"
            );
        }

        /// A suppressed invalidation signal produces silent over-adoption, and
        /// the differential catches it.
        ///
        /// This is the target bug class stated as a corruption. `y` genuinely
        /// reads `x`, and `x`'s type changes from `Integer` to `String`. The
        /// corruption makes the base checkpoint for `x` *already* claim
        /// `String`, so when the engine re-types `x` and compares the fresh
        /// contribution against the cached one it sees no difference, never
        /// marks `x` as changed, and lets `y` keep a typing the new context
        /// refutes.
        ///
        /// **The corruption has to reach the invalidation signal, not the
        /// footprint.** A checkpoint's recorded footprint is carried for
        /// inspection and is not an input to the adoption decision: `resume`
        /// recomputes each edited item's footprint from its current term and
        /// tests *that*. Narrowing the stored footprint therefore changes
        /// nothing, which is a real robustness property of the engine — a
        /// persisted checkpoint cannot cause over-adoption through a stale
        /// footprint — and it is why this test corrupts the cached typing that
        /// feeds `note_binding_change` instead.
        #[test]
        fn a_suppressed_invalidation_signal_is_caught()
        {
            let base = [def("x", Body::Int(1)), def("y", Body::Ref("x".to_owned()))];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::Ref("x".to_owned())),
            ];
            let base_program = items_of(&base);
            let edited_program = items_of(&edited);

            let honest = gate(&base, &edited);
            assert_eq!(
                0,
                usize::from(honest.adopted_count()),
                "with an honest checkpoint the changed binding re-types both items"
            );

            let mut checkpoints = checkpoint_program(&base_program);
            if let Some(checkpoint) = checkpoints.items.get_mut(0) {
                checkpoint.typing = ItemTyping::Definition {
                    name: "x".to_owned(),
                    ty: Ty::Value(ValueType::string()),
                    bound: true,
                };
            }
            let resumed = resume(&checkpoints, &edited_program);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();
            let actual: Vec<ItemTyping> = resumed.typings().cloned().collect();

            assert!(
                adopted[1],
                "the suppressed signal lets `y` adopt across a changed binding"
            );
            assert_ne!(
                actual,
                from_scratch(&edited_program),
                "the differential must catch over-adoption caused by a missed invalidation"
            );
        }

        /// The engine's adoption decision reads the edited item's *current*
        /// footprint, never the one stored in the checkpoint — so a stale or
        /// narrowed stored footprint cannot cause over-adoption.
        ///
        /// This is the companion to the test above: it pins the robustness
        /// property that redirected that corruption, so a future change that
        /// started trusting the stored footprint would be caught here rather
        /// than silently widening the persisted trust surface.
        #[test]
        fn a_stored_footprint_is_not_an_adoption_input()
        {
            let base = [def("x", Body::Int(1)), def("y", Body::Ref("x".to_owned()))];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::Ref("x".to_owned())),
            ];
            let base_program = items_of(&base);
            let edited_program = items_of(&edited);

            let mut checkpoints = checkpoint_program(&base_program);
            if let Some(checkpoint) = checkpoints.items.get_mut(1) {
                let _removed = checkpoint.footprint.names.remove("x");
            }
            let resumed = resume(&checkpoints, &edited_program);
            let actual: Vec<ItemTyping> = resumed.typings().cloned().collect();

            assert!(
                !bool::from(
                    resumed
                        .adopted()
                        .nth(1)
                        .expect("the resumed program has two items")
                ),
                "`y` reads a changed binding, and the freshly scanned footprint says so whatever the checkpoint stored"
            );
            assert_eq!(
                actual,
                from_scratch(&edited_program),
                "a narrowed stored footprint changes no answer"
            );
        }
    }

    /// Inputs on which the engine is known to be wrong today.
    ///
    /// Each test here asserts the **divergence itself** — that `resume` and
    /// `checkpoint_program` disagree on this input — rather than asserting a
    /// wrong answer is right or being disabled. The reproduction therefore
    /// keeps running, so we know it still reproduces, and **each of these tests
    /// fails the moment the defect is fixed**, which is deliberate: it forces
    /// the rewrite into an ordinary equality assertion instead of relying on
    /// someone remembering these are here.
    ///
    /// The defect is `gandr-t8j6`. Both witnesses share one mechanism.
    /// `thread_binding` threads a definition's *unfolding rule* into the typing
    /// context beside its type binding, and the subtype checker mints its
    /// normalizer from that definition chain — so an item's typing can depend
    /// on a definition's **value**, which happens at a `Path` type's endpoints,
    /// compared by definitional equality. The adoption rule tracks only types.
    mod divergence
    {
        use super::*;

        /// The witness `def p = here(1)`, whose inferred type is
        /// `Path Integer 1 1`.
        fn path_witness() -> Item
        {
            Item::new(
                Some("p".to_owned()),
                None,
                Term::Value(Value::here(Value::int(1_i64))),
            )
        }

        /// `def one = literal`.
        fn one_bound_to(literal: Value) -> Item
        {
            Item::new(Some("one".to_owned()), None, Term::Value(literal))
        }

        /// The ascription `Path Integer one 1`, whose left endpoint is the
        /// value `one` reads from the context.
        fn path_to_one() -> Ty
        {
            Ty::Value(ValueType::path(
                ValueType::integer(),
                Value::var("one"),
                Value::int(1_i64),
            ))
        }

        /// Asserts that `resume` and `checkpoint_program` disagree on `edited`
        /// after `base`, and that the item at `index` is the one wrongly
        /// adopted.
        fn assert_diverges(
            base: &Program,
            edited: &Program,
            position: ItemPosition,
        )
        {
            let checkpoints = checkpoint_program(base);
            let resumed = resume(&checkpoints, edited);
            let adopted: Vec<bool> = resumed.adopted().map(bool::from).collect();
            let actual: Vec<ItemTyping> = resumed.typings().cloned().collect();

            assert!(
                adopted[position.0],
                "the item at {position:?} is adopted, which is what produces the wrong answer"
            );
            assert_ne!(
                actual,
                from_scratch(edited),
                "gandr-t8j6 is fixed: this witness now agrees with from-scratch, so rewrite it as an equality assertion"
            );
        }

        /// Gap one: a footprint is computed from the item's **term** alone, so
        /// a name occurring only in its **ascription** is read during checking
        /// and never recorded. Here `r`'s footprint is `{p}`; the `one` its
        /// ascription reads is invisible, so a value-only edit to `one` leaves
        /// `r` adoptable with a typing the edited program refutes.
        ///
        /// Fixing this alone does not close `gandr-t8j6` — see the companion
        /// witness, which reproduces with the name present in the footprint.
        #[test]
        fn known_divergence_ascription_footprint()
        {
            let program_for = |literal: Value| {
                Program::new(vec![
                    one_bound_to(literal),
                    path_witness(),
                    Item::new(
                        Some("r".to_owned()),
                        Some(path_to_one()),
                        Term::Value(Value::var("p")),
                    ),
                ])
            };
            assert_diverges(
                &program_for(Value::int(1_i64)),
                &program_for(Value::int(2_i64)),
                ItemPosition(2),
            );
        }

        /// Gap two: the changed-binding set compares only the bound value
        /// **type**, so an edit that changes a definition's value while keeping
        /// its type adds nothing to it. Here `r`'s term is the pair
        /// `(one, p)`, so its footprint genuinely contains `one` and is not
        /// opaque — and `r` is adopted anyway, because `one` never enters the
        /// changed set.
        ///
        /// Fixing the footprint scan alone therefore does not close
        /// `gandr-t8j6`; both gaps must close together.
        #[test]
        fn known_divergence_type_stable_unfolding()
        {
            let ascription = Some(Ty::Value(ValueType::Prod(
                Rc::new(ValueType::integer()),
                Rc::new(ValueType::path(
                    ValueType::integer(),
                    Value::var("one"),
                    Value::int(1_i64),
                )),
            )));
            let program_for = |literal: Value| {
                Program::new(vec![
                    one_bound_to(literal),
                    path_witness(),
                    Item::new(
                        Some("r".to_owned()),
                        ascription.clone(),
                        Term::Value(Value::Pair(
                            Rc::new(Value::var("one")),
                            Rc::new(Value::var("p")),
                        )),
                    ),
                ])
            };
            let base = program_for(Value::int(1_i64));
            let reader = base.items.get(2).cloned();
            if let Some(reader) = reader {
                let footprint = footprint_of(&reader.term);
                assert!(
                    footprint.names.contains("one"),
                    "the witness only means something if `one` is genuinely in the footprint"
                );
                assert!(
                    !footprint.opaque,
                    "the witness only means something if the footprint is not conservatively opaque"
                );
            }
            assert_diverges(&base, &program_for(Value::int(2_i64)), ItemPosition(2));
        }
    }

    /// The gate over property-generated random edits, singly and in sequences.
    mod property
    {
        use proptest::prelude::*;

        use super::*;

        /// An edit applied to a statement list.
        ///
        /// Every variant is index-guarded when applied: an index the edit
        /// cannot address makes that edit a no-op rather than a panic, which is
        /// what lets a sequence generated against one program length stay total
        /// as later edits change that length.
        #[derive(Clone, Debug)]
        enum Edit
        {
            /// Replace the statement at the index with a fresh one.
            Replace(usize, Stmt),
            /// Insert a fresh statement before the index.
            Insert(usize, Stmt),
            /// Delete the statement at the index.
            Delete(usize),
            /// Rename the definition at the index **and every reader of it** —
            /// one edit that invalidates the old binding and establishes the
            /// new one at the same time.
            Rename(usize, String),
            /// Exchange two statements, changing order without changing any
            /// binding.
            Swap(usize, usize),
            /// Set (or clear) the ascription of the statement at the index.
            Ascribe(usize, Option<Ascription>),
        }

        /// A program of one to six statements.
        fn program() -> impl Strategy<Value = Vec<Stmt>>
        {
            proptest::collection::vec(statement(), 1_usize .. 7_usize)
        }

        /// One definition name from the shared pool.
        ///
        /// The pool is wider than the one the body generator references
        /// (`d0..d5`), so a rename lands on an existing definition about three
        /// times in four and mints an unreferenced name otherwise — collisions
        /// and fresh names both stay common.
        fn name() -> impl Strategy<Value = String>
        {
            (0_usize .. 8_usize).prop_map(|index| format!("d{index}"))
        }

        /// One generated ascription, or none.
        ///
        /// **The generated type shapes are restricted to atoms, and the
        /// restriction is required rather than tidy.** A type whose comparison
        /// consults definitional equality — a `Path`'s endpoints, or any type
        /// carrying a value position — reaches the over-adoption defect
        /// `gandr-t8j6`, and a property run that wandered into it would fail
        /// nondeterministically and break the merge wall for whoever landed
        /// next. Atom comparison is name equality and never consults the
        /// normalizer, so nothing reachable from this generator can enter that
        /// class. The defect is covered instead by the deterministic witnesses
        /// in `divergence`. **Widen this generator when `gandr-t8j6` closes.**
        fn ascription() -> impl Strategy<Value = Option<Ascription>>
        {
            prop_oneof![
                Just(None),
                Just(Some(Ascription::Integer)),
                Just(Some(Ascription::Text)),
            ]
        }

        /// One statement: `def d{index} [: ty] = {body}`. The name pool
        /// (`d0..d5`) deliberately allows repeats and forward references,
        /// exercising the engine's conservative handling of both.
        fn statement() -> impl Strategy<Value = Stmt>
        {
            (0_usize .. 6_usize, ascription(), body()).prop_map(|(index, ascription, body)| Stmt {
                name: Some(format!("d{index}")),
                ascription,
                body,
            })
        }

        /// One definition body: an integer literal, a name reference, an
        /// integer-ascribed reference, or a string literal (so edits can change
        /// a definition's type).
        fn body() -> impl Strategy<Value = Body>
        {
            prop_oneof![
                (0_i64 .. 20_i64).prop_map(Body::Int),
                (0_usize .. 6_usize).prop_map(|index| Body::Ref(format!("d{index}"))),
                (0_usize .. 6_usize).prop_map(|index| Body::CheckInteger(format!("d{index}"))),
                "[a-z]{0,3}".prop_map(Body::Str),
            ]
        }

        /// How many statements a generated program holds — the span an edit's
        /// indices are drawn from.
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
        struct ProgramLength(usize);

        /// One edit sized to a program of `length` statements.
        fn edit(length: ProgramLength) -> impl Strategy<Value = Edit>
        {
            let span = length.0.max(1);
            prop_oneof![
                (0_usize .. span, statement()).prop_map(|(at, s)| Edit::Replace(at, s)),
                (0_usize ..= span, statement()).prop_map(|(at, s)| Edit::Insert(at, s)),
                (0_usize .. span).prop_map(Edit::Delete),
                (0_usize .. span, name()).prop_map(|(at, to)| Edit::Rename(at, to)),
                (0_usize .. span, 0_usize .. span)
                    .prop_map(|(first, second)| Edit::Swap(first, second)),
                (0_usize .. span, ascription()).prop_map(|(at, a)| Edit::Ascribe(at, a)),
            ]
        }

        /// A program paired with a random edit sized to it.
        fn program_and_edit() -> impl Strategy<Value = (Vec<Stmt>, Edit)>
        {
            program().prop_flat_map(|statements| {
                let length = statements.len();
                (Just(statements), edit(ProgramLength(length)))
            })
        }

        /// A program paired with a chain of one to four edits.
        ///
        /// The edits are sized to the *initial* program, and later edits change
        /// its length — so an index can fall out of range mid-sequence, where
        /// [`apply_edit`]'s guards make that edit a no-op. That is deliberate
        /// rather than tolerated: a sequence whose every step must be in range
        /// cannot generate a delete-then-address-the-hole shape at all.
        fn program_and_edit_sequence() -> impl Strategy<Value = (Vec<Stmt>, Vec<Edit>)>
        {
            program().prop_flat_map(|statements| {
                let length = statements.len();
                (
                    Just(statements),
                    proptest::collection::vec(edit(ProgramLength(length)), 1_usize .. 5_usize),
                )
            })
        }

        /// Applies `edit` to a clone of `statements`, returning the edited
        /// list.
        fn apply_edit(
            statements: &[Stmt],
            edit: &Edit,
        ) -> Vec<Stmt>
        {
            let mut edited = statements.to_vec();
            match *edit {
                | Edit::Replace(at, ref statement) => {
                    if let Some(slot) = edited.get_mut(at) {
                        slot.clone_from(statement);
                    }
                },
                | Edit::Insert(at, ref statement) => {
                    edited.insert(at.min(edited.len()), statement.clone());
                },
                | Edit::Delete(at) => {
                    if at < edited.len() {
                        let _removed = edited.remove(at);
                    }
                },
                | Edit::Rename(at, ref to) => {
                    let old = edited.get(at).and_then(|slot| slot.name.clone());
                    if let Some(old) = old {
                        for slot in &mut edited {
                            if slot.name.as_deref() == Some(old.as_str()) {
                                slot.name = Some(to.clone());
                            }
                            match slot.body {
                                | Body::Ref(ref mut read) | Body::CheckInteger(ref mut read) => {
                                    if *read == old {
                                        read.clone_from(to);
                                    }
                                },
                                | Body::Int(_) | Body::Str(_) => {},
                            }
                        }
                    }
                },
                | Edit::Swap(first, second) => {
                    if first < edited.len() && second < edited.len() {
                        edited.swap(first, second);
                    }
                },
                | Edit::Ascribe(at, ascription) => {
                    if let Some(slot) = edited.get_mut(at) {
                        slot.ascription = ascription;
                    }
                },
            }
            edited
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(400))]

            /// The gate over arbitrary programs and one edit: the incrementally
            /// resumed typings always equal a from-scratch re-type of the edited
            /// program — even when the edit introduces type errors, holes, name
            /// collisions, dangling references, or a coordinated rename — and
            /// every adoption survives the precision probe and the persisted
            /// round trip.
            #[test]
            fn incremental_equals_from_scratch((statements, concrete) in program_and_edit()) {
                let base = items_of(&statements);
                let edited_statements = apply_edit(&statements, &concrete);
                let edited = items_of(&edited_statements);

                let checkpoints = checkpoint_program(&base);
                let _outcome = resume_step(&checkpoints, &edited)?;
            }

            /// Zero drift under **sequences**: a chain of edits with a resume at
            /// every step, the running checkpoint set threaded forward through
            /// the persistence codec. Staleness that accumulates across resumes
            /// is invisible to single-edit coverage — each step here resumes
            /// from the previous step's *result*, never from a freshly computed
            /// base, so a checkpoint set that drifts a little per step is caught
            /// at the first step where it matters.
            #[test]
            fn edit_sequences_preserve_zero_drift(
                (statements, edits) in program_and_edit_sequence()
            ) {
                let mut current = statements;
                let mut checkpoints = checkpoint_program(&items_of(&current));
                for concrete in &edits {
                    current = apply_edit(&current, concrete);
                    let edited = items_of(&current);
                    let outcome = resume_step(&checkpoints, &edited)?;
                    checkpoints = outcome.checkpoints;
                }
            }
        }
    }
}
