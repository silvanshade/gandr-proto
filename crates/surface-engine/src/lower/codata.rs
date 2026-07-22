//! Codata: the negative-half elaboration (proposal-codata-corecursion §2–4).
//!
//! A `codata C { π₁: B₁, … }` block declares named **observations** — the dual
//! of a `data` block's field-tuple constructors (a constructor says what a
//! value *contains*; an observation says what a value *answers*). A copattern
//! definition `def rec f(…) -> C { .πᵢ => eᵢ }` introduces a value of `C` by
//! saying how it responds to each observation.
//!
//! # One algorithm, one node, one lowering (the MVP slice)
//!
//! * **The lhs problem** ([`Lowerer::build_cosplit`], codata design §4.1).
//!   Patterns and copatterns elaborate by *one* engine — the generalization of
//!   the planned Maranget matrix (declared-data design §4.2) to a
//!   left-hand-side problem that mixes application copatterns (ordinary
//!   argument patterns) and projection copatterns (`.π`). The codata MVP
//!   exercises the projection-copattern axis: every clause leads with `.π`, so
//!   the engine partitions the clauses by observation (the `Cosplit` step). The
//!   data-only fragment degenerates to exactly the planned Maranget matrix —
//!   the pattern-matrix work extends this engine rather than introducing a
//!   second one. Coverage (every observation answered exactly once) is a
//!   *separate phase* from productivity (codata design §4.1) — the productivity
//!   ladder (guardedness, sizes) is `guarded-corecursion work`/beyond and runs
//!   on the already-elaborated tree.
//! * **The `Cosplit` node** ([`Cosplit`], codata design §4.2, C3). One
//!   elaboration- level case-tree node: a record of branches keyed by
//!   observation, reducing only when observed.
//! * **Route (a) lowering** ([`Lowerer::lower_cosplit`], codata design
//!   §4.2/§3.1). The `Cosplit` lowers to the **record of graded thunks** `#{ πᵢ
//!   = thunk_ω tᵢ }` over the existing record former; an observation `s.π` is
//!   `RecordProj` + `force` ([`Lowerer::codata_observation`]). **Zero
//!   frozen-core spend** — no new core construct; the carrier is the existing
//!   record, `U_ω` thunk, and call-by-need memo machinery. Route (b) (a labeled
//!   n-ary negative product) stays reserved (codata design §4.2), taken only on
//!   sequent-kernel evidence.
//!
//! # MVP boundaries (recorded honestly)
//!
//! * **No nominal opacity.** The MVP carrier is the *structural* record type
//!   `#{ π: U_ω B }`; the codata type `C` is a synonym for its carrier, not a
//!   minted nominal id. Nominal tagging is the single frozen-core touch
//!   declared-data design schedules for declared data (shared, both polarities)
//!   — deferred with the data-block work (`datatype-description work`), out of
//!   this zero-core slice.
//! * **No corecursion.** The `fix self` desugaring of a *recursive* copattern
//!   definition (codata design §5.1) and the guardedness rung (§5.2) are
//!   `guarded-corecursion work`. This slice lowers the copattern clauses to the
//!   `Cosplit` record-of-thunks with **no** `fix` binder: a non-recursive
//!   codata value (a finite record of observations) evaluates end-to-end; a
//!   self-referential body observes an unbound name (honest stuckness until
//!   `fix` lands).
//! * **The `_` default arm** (codata design §3.2) parses and is recorded, but
//!   is a reserved slot — it does not fill unanswered observations in the MVP,
//!   so a definition relying on it fails coverage.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::grade::Grade;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;

use super::COut;
use super::EOut;
use super::LowerError;
use super::LowerResult;
use super::LoweredItem;
use super::Lowerer;
use super::VOut;
use super::entry;
use super::node_kinds;
use super::required_field;
use crate::boundary::NodeText;
use crate::boundary::ObservationName;
use crate::boundary::ObservationPresence;
use crate::boundary::TypeName;
use crate::origin::ElabKind;
use crate::origin::OriginNode;
use crate::synnode::SynNode;

/// One declared observation `π : B` of a [`CodataDecl`] (codata design §2). The
/// result `B` is a *computation* type (an observation is a demand); a value-
/// typed surface `π : A` is stored as `π : F A`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ObservationDecl
{
    /// The observation name.
    name: String,
    /// The observation's result computation type `B`.
    result: CompType,
}

/// A `codata C { … }` declaration's registered shape (codata design §2): its
/// usable observations, in declaration order. Reserved members (grade-prefixed,
/// parameterized, or `rule` 2-cells) are parse-and-decline and are not stored.
///
/// Carried on [`super::Lowered::codata`] so a REPL [`crate::session::Session`]
/// persists `codata` blocks across submissions — the negative-declaration
/// analogue of the `extern`-module bridge (each interactive line is lowered
/// separately, so a `def rec f() -> C` line must see the `codata C` block a
/// prior line declared).
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodataDecl
{
    /// The usable observations, in declaration order.
    observations: Vec<ObservationDecl>,
}

impl CodataDecl
{
    /// The declared observation names (for seeding the projection-
    /// disambiguation set from a persisted declaration).
    pub(super) fn observation_names(&self) -> impl Iterator<Item = &str>
    {
        self.observations.iter().map(|obs| obs.name.as_str())
    }

    /// Whether `observation` is declared by this codata type.
    fn declares(
        &self,
        observation: ObservationName<'_>,
    ) -> ObservationPresence
    {
        ObservationPresence(
            self.observations
                .iter()
                .any(|obs| obs.name == observation.0),
        )
    }

    /// The MVP structural carrier `#{ π: U_ω B }` (codata design §3.1): the
    /// record of graded thunks a value of this codata type inhabits. Used
    /// as the codata definition's ascription — it enforces each
    /// observation's result type and (by the record former's width+depth
    /// discipline) surfaces a *missing* observation as a type mismatch
    /// naming the field (the coverage diagnostic).
    fn carrier(&self) -> ValueType
    {
        ValueType::record(self.observations.iter().map(|obs| {
            (
                obs.name.clone(),
                ValueType::thunk(Grade::OMEGA, obs.result.clone()),
            )
        }))
    }
}

/// One arm of a [`Cosplit`]: an observation and the clause that answers it.
struct CosplitArm<'tree>
{
    /// The observed observation name.
    observation: String,
    /// The clause body `e` (the observation's delayed answer).
    body: SynNode<'tree>,
    /// The clause node (the `Cosplit` arm's origin anchor).
    clause: SynNode<'tree>,
}

/// The copattern case-tree node (codata design §4.2, C3): a record of branches
/// keyed by observation, reducing only when observed. Built by the lhs-problem
/// engine ([`Lowerer::build_cosplit`]) from a coverage-checked clause list,
/// then lowered by route (a) ([`Lowerer::lower_cosplit`]) to the
/// record-of-thunks carrier. The MVP is depth-1 (each clause is a single
/// projection copattern); nested copatterns (`.tail.head => …`) are the
/// documented generalization.
#[repr(transparent)]
struct Cosplit<'tree>
{
    /// The arms, one per answered observation (deduplicated, coverage-checked).
    arms: Vec<CosplitArm<'tree>>,
}

impl Lowerer<'_>
{
    /// Pre-pass: register one `codata C { … }` block into the codata registry
    /// (codata design §2). Mirrors the `extern`-block pre-pass — a `codata`
    /// block is a declaration, not a runnable item, so it contributes no
    /// [`LoweredItem`]; it records the observation set copattern
    /// definitions are elaborated and coverage-checked against, regardless
    /// of source order.
    ///
    /// # Contract
    /// - ensures: `C` is registered with its usable observations (each result
    ///   type lowered, value-sugar `π: A` stored as `π: F A`); every usable
    ///   observation name joins the projection-disambiguation set; reserved
    ///   members (grade / parameter / `rule`) are declined.
    /// - fails: [`LowerError`] in strict mode for a malformed member; total
    ///   mode drops the offending member.
    /// - panics: none.
    pub(super) fn collect_codata(
        &mut self,
        node: SynNode<'_>,
        out: &mut BTreeMap<String, CodataDecl>,
    ) -> LowerResult<()>
    {
        let name_node = required_field(node, node_kinds::FIELD_NAME)?;
        let codata_name = {
            let text = self.text(name_node)?;
            core::convert::identity(text)
        }
        .to_owned();
        let mut observations = Vec::new();
        for member in node.named_children() {
            if member.kind() != node_kinds::CODATA_OBSERVATION {
                continue;
            }
            // Reserved slots (codata design §2): parameterized / graded observations
            // and `rule` 2-cells parse and are declined — registered as present
            // (they still occupy the block) but not lowered to the carrier.
            if member.is_reserved_observation().0 {
                continue;
            }
            let (Some(obs_name_node), Some(ty_node)) = (
                member.child_by_field_name(node_kinds::FIELD_NAME),
                member.child_by_field_name(node_kinds::FIELD_TYPE),
            )
            else {
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::MalformedNode {
                    kind: member.kind(),
                    byte_range: member.byte_range(),
                });
            };
            let obs_name = {
                let text = self.text(obs_name_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            // An observation is a demand, so its result is a computation type;
            // the value-typed surface `π: A` is the sugar `π: F A` (codata design §2).
            // The declared-data-aware seam rewrites a declared datatype in the
            // observation's result type to its nominal handle at any depth
            // (declared-data design).
            let result = match self.lower_type_node(ty_node)? {
                | Ty::Value(value) => CompType::returner(value),
                | Ty::Comp(comp) => comp,
                // `Ty` is non-exhaustive upstream; an unknown sort degrades to
                // the gradual returner (the sort-free fallback of the total
                // type lowering).
                | _ => CompType::returner(ValueType::Unknown),
            };
            self.observations.insert(obs_name.clone());
            observations.push(ObservationDecl {
                name: obs_name,
                result,
            });
        }
        let decl = CodataDecl { observations };
        // Register for this source's own lookup, and emit for the caller to
        // persist across submissions (the `extern`-bridge analogue).
        self.codata.insert(codata_name.clone(), decl.clone());
        out.insert(codata_name, decl);
        Ok(())
    }

    /// Lowers a copattern definition `def rec f(params?) -> C { .π => e, … }`
    /// to its route-(a) carrier (codata design §4.2/§5.1, minus the `fix`
    /// binder that is `guarded-corecursion work`).
    ///
    /// The copattern clauses elaborate through the lhs-problem engine to a
    /// [`Cosplit`], which lowers to the record of graded thunks. A nullary
    /// definition binds `f` directly to that record value; a parameterized one
    /// binds `f` to `thunk_ω { fn(params) { ret record } }` (a codata value
    /// indexed by its parameters). The ascription is `C`'s carrier (or the
    /// curried thunk type over it), enforcing observation result types and
    /// surfacing missing coverage.
    ///
    /// # Contract
    /// - ensures: on a definition whose `-> C` names a declared codata type,
    ///   the term is the record-of-thunks carrier (nullary) or a thunked
    ///   function returning it (parameterized), ascribed to `C`'s carrier; an
    ///   unknown `C` lowers structurally with no ascription.
    /// - fails: [`LowerError`] in strict mode for a coverage violation (unknown
    ///   / duplicate / missing observation) or malformed clause; total mode
    ///   degrades (drop / last-wins / carrier-surfaced).
    /// - panics: none.
    pub(super) fn lower_copattern_def(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(LoweredItem, OriginNode)>
    {
        let name_node = required_field(node, node_kinds::FIELD_NAME)?;
        let name = {
            let text = self.text(name_node)?;
            core::convert::identity(text)
        }
        .to_owned();
        let codata_name = node
            .child_by_field_name(node_kinds::FIELD_RESULT)
            .and_then(|result| self.head_type_name(result));
        // Clone the declaration to release the immutable `self.codata` borrow
        // before the `&mut self` clause lowering.
        let decl = codata_name
            .as_deref()
            .and_then(|codata| self.codata.get(codata))
            .cloned();

        let clauses = node.children_by_field_name(node_kinds::FIELD_CLAUSE);
        let cosplit = self.build_cosplit(
            node,
            &clauses,
            codata_name.as_deref().map(Into::into),
            decl.as_ref(),
        )?;
        let record = self.lower_cosplit(node, &cosplit)?;

        let params = match node.child_by_field_name(node_kinds::FIELD_PARAMETERS) {
            | Some(params_node) => self.parameters(params_node)?,
            | None => Vec::new(),
        };
        let carrier = decl.as_ref().map(CodataDecl::carrier);

        if params.is_empty() {
            // A nullary copattern definition *is* a codata value: bind the name
            // directly to the record-of-thunks (no `def f()` outer thunk), so an
            // observation `f.π` projects-and-forces the record with no extra
            // force of `f` itself.
            let ascription = carrier.map(Ty::Value);
            return Ok((
                LoweredItem {
                    name: Some(name),
                    ascription,
                    term: Term::Value({
                        let readback_value = record.readback_value()?;
                        core::convert::identity(readback_value)
                    }),
                },
                record.origin,
            ));
        }

        // A parameterized copattern definition is a codata value indexed by its
        // parameters: `thunk_ω { fn(params) { ret record } }` (the def-function
        // sugar shape, codata design §5.1's λ minus the `fix self`).
        let sugar_entry = entry(node, Some(ElabKind::Cosplit));
        let ret = COut::from_legacy_comp(
            &Comp::Ret(Rc::new({
                let readback_value = record.readback_value()?;
                core::convert::identity(readback_value)
            })),
            OriginNode::new(entry(node, Some(ElabKind::RetCoercion)), vec![
                record.origin,
            ]),
        )?;
        let abs_params: Vec<(String, Option<ValueType>)> = params
            .iter()
            .map(|entry| {
                let (ref param_name, _) = *entry;
                (param_name.clone(), None)
            })
            .collect();
        let body = Self::curry_abs(abs_params, ret, &sugar_entry, Some(ElabKind::Cosplit))?;
        let thunk = VOut::from_legacy_value(
            &Value::thunk(Grade::OMEGA, {
                let readback_comp = body.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
            OriginNode::new(sugar_entry, vec![body.origin]),
        )?;
        // Ascription `U_ω (A₁ → … → F carrier)` when every parameter is
        // annotated and the codata type is known (the def-function sugar's
        // derived ascription over the returner of the carrier).
        let ascription = carrier.and_then(|carrier| {
            Self::derived_ascription(&params, Some(CompType::returner(carrier)))
        });
        Ok((
            LoweredItem {
                name: Some(name),
                ascription,
                term: Term::Value({
                    let readback_value = thunk.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            },
            thunk.origin,
        ))
    }

    /// The lhs-problem engine (codata design §4.1): partition the copattern
    /// clauses by observation into a [`Cosplit`], coverage-checked against
    /// `C`'s declared observations. Coverage is the without-K analogue of a
    /// `case`'s exhaustiveness; it is a *separate phase* from productivity
    /// (codata design §4.1).
    ///
    /// The MVP is depth-1 — every clause is a single projection copattern
    /// `.π => e` — so the engine's `Cosplit` step is the whole compilation. An
    /// unknown observation (not declared by `C`), a duplicate (an observation
    /// answered twice), or a missing observation (an unanswered one) is the
    /// coverage diagnosis; strict mode rejects, total mode degrades.
    fn build_cosplit<'tree>(
        &self,
        node: SynNode<'tree>,
        clauses: &[SynNode<'tree>],
        codata_name: Option<TypeName<'_>>,
        decl: Option<&CodataDecl>,
    ) -> LowerResult<Cosplit<'tree>>
    {
        let mut arms: Vec<CosplitArm<'tree>> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for &clause in clauses {
            if clause.kind() != node_kinds::COPATTERN_CLAUSE {
                continue;
            }
            let Some(body) = clause.child_by_field_name(node_kinds::FIELD_BODY)
            else {
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::MalformedNode {
                    kind: clause.kind(),
                    byte_range: clause.byte_range(),
                });
            };
            let Some(obs_node) = clause.child_by_field_name(node_kinds::FIELD_OBSERVATION)
            else {
                // A `_ => e` default arm (codata design §3.2): reserved parse-and-
                // decline. It is recognized but does not fill unanswered
                // observations in the MVP, so a definition relying on it fails
                // the missing-observation check below.
                continue;
            };
            let observation = {
                let text = self.text(obs_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            if let Some(decl) = decl
                && !decl.declares(observation.as_str().into()).0
            {
                // Coverage: an observation the codata type does not declare.
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::UnknownObservation {
                    observation,
                    codata: codata_name.map_or_else(String::new, |name| name.0.to_owned()),
                    byte_range: clause.byte_range(),
                });
            }
            if !seen.insert(observation.clone()) {
                // Coverage: an observation answered more than once.
                if bool::from(self.total()) {
                    if let Some(existing) =
                        arms.iter_mut().find(|arm| arm.observation == observation)
                    {
                        existing.body = body;
                        existing.clause = clause;
                    }
                    continue;
                }
                return Err(LowerError::DuplicateObservation {
                    observation,
                    byte_range: clause.byte_range(),
                });
            }
            arms.push(CosplitArm {
                observation,
                body,
                clause,
            });
        }
        // Coverage: every declared observation must be answered. In total mode
        // the omission is left to the carrier ascription, whose record former
        // rejects the short record with a mismatch naming the missing field.
        if let Some(decl) = decl
            && !bool::from(self.total())
            && let Some(missing) = decl
                .observations
                .iter()
                .find(|obs| !seen.contains(&obs.name))
        {
            return Err(LowerError::MissingObservation {
                observation: missing.name.clone(),
                codata: codata_name.map_or_else(String::new, |name| name.0.to_owned()),
                byte_range: node.byte_range(),
            });
        }
        Ok(Cosplit { arms })
    }

    /// Route (a) (codata design §4.2): lower a [`Cosplit`] to the
    /// record-of-thunks carrier `#{ πᵢ = thunk_ω tᵢ }`. Each observation's
    /// clause body lowers in *computation position* (the observation is a
    /// demand, so a value body is `ret`-coerced), then is delayed as a
    /// graded thunk `U_ω B` — the field the existing record former holds
    /// and the observation `s.π` later `RecordProj`s and `force`s. Fields
    /// go in canonical (sorted) label order, the order the checker /
    /// machine / mark descend a record (mirrors [`Lowerer::record_expr`]).
    fn lower_cosplit(
        &mut self,
        node: SynNode<'_>,
        cosplit: &Cosplit<'_>,
    ) -> LowerResult<VOut>
    {
        let mut fields: BTreeMap<String, Rc<Value>> = BTreeMap::new();
        let mut origins: BTreeMap<String, OriginNode> = BTreeMap::new();
        for arm in &cosplit.arms {
            let body = self.comp_expr(arm.body)?;
            let thunk = VOut::from_legacy_value(
                &Value::thunk(Grade::OMEGA, {
                    let readback_comp = body.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                OriginNode::new(entry(arm.clause, Some(ElabKind::Cosplit)), vec![
                    body.origin,
                ]),
            )?;
            fields.insert(
                arm.observation.clone(),
                Rc::new({
                    let readback_value = thunk.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            );
            origins.insert(arm.observation.clone(), thunk.origin);
        }
        VOut::from_legacy_value(
            &Value::Record(fields),
            OriginNode::new(
                entry(node, Some(ElabKind::Cosplit)),
                origins.into_values().collect(),
            ),
        )
    }

    /// Lowers a codata observation `s.π` to `let t <- RecordProj(s, π);
    /// force t` (codata design §3.1). The observed target `s` lowers in value
    /// position (a computation target — e.g. `f(x).π` — is hoisted); the
    /// projection reads the delayed observation body `t : U_ω B`, and
    /// `force t : B` performs the observation. The [`Lowerer::projection`]
    /// dispatch routes here when the field is a declared observation.
    ///
    /// # Contract
    /// - ensures: the returned computation is `RecordProj(s, π)` bound and
    ///   forced, of the observation's result type `B`; a computation target is
    ///   hoisted around it.
    /// - panics: none.
    ///
    /// # Termination
    /// - reason: mutual expression lowering follows the finite syntax tree.
    /// - measure: proper syntax descendants beneath the observation target.
    /// - boundedness: the CST is finite and recursive calls descend.
    pub(super) fn codata_observation(
        &mut self,
        node: SynNode<'_>,
        target_node: SynNode<'_>,
        observation: String,
    ) -> LowerResult<EOut>
    {
        let elab = Some(ElabKind::Observe);
        let mut hoists = Vec::new();
        let record = self.value_expr(target_node, &mut hoists)?;
        let proj = COut::from_legacy_comp(
            &Comp::RecordProj {
                record: Rc::new({
                    let readback_value = record.readback_value()?;
                    core::convert::identity(readback_value)
                }),
                label: observation,
            },
            OriginNode::new(entry(node, elab), vec![record.origin]),
        )?;
        let binder = self.fresh_name();
        let forced = COut::from_legacy_comp(
            &Comp::Force(Rc::new(Value::var(&binder))),
            OriginNode::leaf(entry(node, elab)),
        )?;
        let observe = COut::from_legacy_comp(
            &Comp::Bind(
                Rc::new({
                    let readback_comp = proj.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                binder,
                Rc::new({
                    let readback_comp = forced.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ),
            OriginNode::new(entry(node, elab), vec![proj.origin, forced.origin]),
        )?;
        Ok(EOut::Comp({
            let wrapped = Self::wrap_hoists(hoists, observe, node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Whether `field` is a declared observation of some `codata` block — the
    /// [`Lowerer::projection`] gate deciding a `.π` projection is a codata
    /// observation (project-and-force) rather than a plain record projection.
    pub(super) fn is_observation(
        &self,
        field: ObservationName<'_>,
    ) -> ObservationPresence
    {
        ObservationPresence(self.observations.contains(field.0))
    }

    /// The head type name of a codata definition's `-> C` result: a bare
    /// `type_identifier` (`Point`) or the constructor of a `type_application`
    /// (`Stream(Integer)` → `Stream`). Other result shapes are not codata
    /// types.
    fn head_type_name(
        &self,
        result: SynNode<'_>,
    ) -> Option<String>
    {
        match result.kind() {
            | node_kinds::TYPE_IDENTIFIER => self.text(result).ok().map(NodeText::to_owned),
            | node_kinds::TYPE_APPLICATION => result
                .child_by_field_name(node_kinds::FIELD_CONSTRUCTOR)
                .and_then(|constructor| self.text(constructor).ok())
                .map(NodeText::to_owned),
            | _ => None,
        }
    }
}
