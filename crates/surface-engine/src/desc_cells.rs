//! The **description → cell layer** seam: stage-0 [`SignDesc`]s into the
//! content-addressed cell store of `gandr-theory-computads`.
//!
//! [`desc_elab`](crate::desc_elab) stops at the declaration table — it reads a
//! `data` / `codata` block into a [`SignDesc`] and runs
//! [`gandr_theory_levitation::check_desc`] over it. This module carries that
//! table one step further, to the artifact the fusion engines actually consume:
//! [`gandr_theory_computads::elaborate_data_desc`] turns each description's
//! constructors, `op` members, and `rule` faces into a
//! [`CellStore`] plus the typed declines for everything the cell layer cannot
//! hold.
//!
//! The two passes are deliberately separate rather than fused into
//! [`elaborate_data_descs`]: a caller that only wants the description table (the
//! generic consumers, the inspector) must not pay for cell elaboration, and the
//! cell layer's declines answer a different question from stage-0
//! well-formedness. A caller wanting both verdicts runs both passes and
//! concatenates their diagnostics — which is what the corpus harness's `desc`
//! mode does.
//!
//! # What the cell layer declines
//!
//! The command-pattern grammar's operation frame `f(p̄; c)` carries exactly one
//! return continuation, so an `op` member's [`BridgeArity`] is admitted only in
//! the degenerate one-monomial, one-output shape. A many-out `op divmod(m, n)
//! -> (q, r)` is a **well-formed description** and an **unrepresentable cell**,
//! and this module is where that pair of verdicts becomes two inspectable
//! diagnostics rather than a silent narrowing.
//!
//! [`BridgeArity`]: gandr_theory_levitation::BridgeArity
//! [`SignDesc`]: gandr_theory_levitation::SignDesc
//! [`elaborate_data_descs`]: crate::desc_elab::elaborate_data_descs

use gandr_theory_circuit_algebras::matching::MatchBudget;
use gandr_theory_circuit_algebras::matching::MatchCount;
use gandr_theory_coherent_resolutions::CompletionBudget;
use gandr_theory_coherent_resolutions::CompletionOutcome;
use gandr_theory_computads::CellId;
use gandr_theory_computads::CellStore;
use gandr_theory_computads::DeclinedCircuitIndex;
use gandr_theory_computads::DeclinedFaceIndex;
use gandr_theory_computads::ElaborateError;
use gandr_theory_computads::OpElaborateError;
use gandr_theory_computads::elaborate_data_desc;
use gandr_theory_levitation::CircuitDerivationError;
use gandr_theory_levitation::CircuitElaborationError;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::RedexOccurrence;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::TermPositionIndex;
use gandr_theory_levitation::WhiskeredCell;

use crate::boundary::DeclineReason;
use crate::circuit::embed::CircuitRuleCell;
use crate::circuit::embed::complete_circuit_rules;
use crate::cst_read::empty_surface_span;
use crate::desc_elab::ElabDiagnostic;

/// The cell-layer elaboration of a source's descriptions: one store per
/// description, plus every decline as a located diagnostic.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DescCells
{
    /// The final (or as-of-decline) cell store of each description, in
    /// declaration order, including cells derived by generic completion.
    pub stores: Vec<CellStore>,
    /// The **whiskered composite** each admitted circuit rule denotes, in
    /// description order — the boundary-language object a ruled circuit block
    /// lowers to, beside the cell its derived boundaries became.
    pub composites: Vec<WhiskeredCell>,
    /// Every cell-layer decline, in description order, located at its surface
    /// span where the description records one.
    pub diagnostics: Vec<ElabDiagnostic>,
    /// The **η cells** each description licensed, by their id in that
    /// description's store, in description order.
    pub eta: Vec<Vec<CellId>>,
    /// Where each circuit rule's diagram sits inside each circuit rule of its
    /// own description, in description order — the **redex-occurrence records**
    /// the embedding matcher computes, plus the generic overlap and replay
    /// evidence supplied to completion.
    pub circuit_sites: Vec<CircuitSite>,
    /// Why a description licensed no η cell, in description order, one entry
    /// per description that licensed none.
    ///
    /// This is **not** a diagnostic. Most declarations license no η law — a
    /// type with several constructors has no single constructor for a
    /// destructor to invert — so reporting one as a decline would make the
    /// ordinary case look like a failure. It is stated where a reader can ask
    /// for it and nowhere else.
    pub eta_declines: Vec<String>,
}

/// **Elaborate** each description into the cell store, collecting the stores
/// and every cell-layer decline.
///
/// # Contract
/// - requires: `descs` are stage-0 descriptions, typically from
///   [`elaborate_data_descs`]; they need not be well-formed (this pass is
///   independent of [`gandr_theory_levitation::check_desc`] and answers only
///   the cell layer's question).
/// - ensures: one [`CellStore`] per description, in the given order, each
///   holding a frame-defining cell per declared constructor, every admitted
///   `rule` cell, the η cell the declaration licenses, and any cells derived
///   from admitted circuit overlaps by generic completion; one diagnostic per
///   declined `op` member and per declined `rule` face, in description order
///   and within a description, operations before faces.
/// - ensures: `eta` addresses the η cells of each description's store, and
///   `eta_declines` states why a description licensed none — the latter is
///   reported rather than diagnosed, because licensing no η law is the ordinary
///   case.
/// - ensures: **the cell identities are a function of the source alone.** A
///   [`gandr_theory_computads::CellId`] is an insertion-order address. The
///   declaration phase inserts constructors, written faces, circuit rules, and
///   η cells in declaration order; generic completion then appends derived
///   cells in its deterministic overlap schedule. The same descriptions
///   therefore always lower to the same ids, and reordering two faces reorders
///   exactly their two declaration-phase cells. A face declared twice is one
///   cell: [`CellStore::insert`] deduplicates on structural identity, so the
///   store's length counts distinct cells.
/// - provides: the seam between the description layer and the fusion engines —
///   the first consumer of `gandr-theory-computads` outside that crate's own
///   tests.
/// - fails: never; total on any description.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a description whose single-output `op` and `rule` produce
///   cells is separated from one whose many-out `op` produces a decline by the
///   store's cell count and by the presence of a diagnostic naming the
///   operation; the determinism clause is separated from mere repeatability by
///   a source whose two faces are swapped, which moves exactly two ids.
/// - witness: `gandr-surface-engine` `tests/desc_cells.rs`
///   `a_declared_single_output_operation_lets_its_rule_become_a_cell`
/// - witness: `gandr-surface-engine` `tests/desc_cells.rs`
///   `a_many_out_operation_is_a_well_formed_description_and_an_unrepresentable_cell`
/// - witness: `gandr-surface-engine` `tests/desc_cells.rs`
///   `cell_identities_are_a_deterministic_function_of_the_source`
/// - witness: `gandr-surface-engine` `tests/desc_cells.rs`
///   `declaration_order_is_the_cell_order`
/// - witness: `gandr-surface-engine` `tests/desc_cells.rs`
///   `a_face_declared_twice_is_one_cell`
///
/// [`elaborate_data_descs`]: crate::desc_elab::elaborate_data_descs
#[inline]
#[must_use]
pub fn elaborate_desc_cells(descs: &[SignDesc]) -> DescCells
{
    let mut cells = DescCells::default();
    for desc in descs {
        let elaborated = elaborate_data_desc(desc);
        for &(index, error) in &elaborated.declined_opers {
            let name = desc
                .opers
                .get(usize::from(index))
                .map_or("<unknown>", |op| op.name.as_ref());
            cells.diagnostics.push(ElabDiagnostic::new(
                format!(
                    "operation `{name}` of `{}` {}",
                    desc.id.name.as_ref(),
                    op_decline_reason(error).as_ref()
                ),
                empty_surface_span(),
            ));
        }
        for &(index, ref error) in &elaborated.declined_faces {
            cells
                .diagnostics
                .push(face_decline_diagnostic(desc, index, error));
        }
        for &(index, ref error) in &elaborated.declined_circuits {
            cells
                .diagnostics
                .push(circuit_decline_diagnostic(desc, index, error));
        }
        let (sites, store) = circuit_sites(desc, elaborated.store, &elaborated.circuit_cell_ids);
        cells.circuit_sites.extend(sites);
        if let Some(ref declined) = elaborated.declined_eta {
            cells
                .eta_declines
                .push(format!("`{}`: {declined}", desc.id.name.as_ref()));
        }
        cells.eta.push(elaborated.eta);
        cells.composites.extend(elaborated.composites);
        cells.stores.push(store);
    }
    cells
}

/// **Where one circuit rule's diagram sits inside another's.**
///
/// One record per ordered pair of a description's circuit rules whose bodies
/// both read as diagrams, with the generic completion evidence for the
/// corresponding cell pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitSite
{
    /// The rule whose diagram was sought.
    pub pattern: Name,
    /// The rule whose body it was sought in.
    pub target: Name,
    /// How many embeddings the matcher admitted.
    pub admitted: MatchCount,
    /// How many generic confluence overlaps the pair supplied.
    pub overlap_count: usize,
    /// How many certificates for the pair replayed successfully.
    pub certificates_replayed: usize,
}

/// The **matching budget** the description route runs the embedding search
/// under.
///
/// A declared circuit body is author-sized and small, so the ceiling is set
/// well above any realistic body rather than tuned: what it is for is turning a
/// pathological search into a decline instead of a hang, and a decline here
/// drops the record rather than failing the elaboration, because a missing
/// redex-occurrence record is not a reason to refuse a description.
const CIRCUIT_MATCH_BUDGET: usize = 4_096_usize;

/// The **matching and completion seam** the description route runs for one
/// declaration.
///
/// It first asks the circuit matcher for every ordered body pair, then supplies
/// the admitted pairs and their generic cell ids to the coherent-resolution
/// completion API. The returned store is the completion store, so derived
/// cells and replayable certificates remain part of the route's artifact.
///
/// # Contract
/// - ensures: one record per ordered pair of circuit rules whose bodies both
///   read as diagrams, in declaration order, pattern-major; a pair whose search
///   declines, or either of whose bodies is not a diagram, contributes no
///   record. Each record carries the generic confluence-overlap count and the
///   count of its emitted certificates that replayed against the final
///   completion store.
/// - provides: the circuit-algebra crate's shipping consumer and the generic
///   completion instantiation site, above both theory crates.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — a description whose multi-root and reconvergent circuit
///   bodies embed separates matcher admissions from the reverse non-embedding
///   direction, and a replay count separates a completion certificate from a
///   matcher-only record.
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `the_description_route_runs_completion_through_the_matcher_seam`
#[inline]
fn circuit_sites(
    desc: &SignDesc,
    store: CellStore,
    circuit_cell_ids: &[Option<CellId>],
) -> (Vec<CircuitSite>, CellStore)
{
    let rules: Vec<CircuitRuleCell<'_>> = desc
        .circuits
        .iter()
        .zip(circuit_cell_ids.iter().copied())
        .map(|(rule, cell)| CircuitRuleCell {
            name: &rule.name,
            body: &rule.body,
            cell,
        })
        .collect();
    let completion = complete_circuit_rules(
        store,
        &rules,
        MatchBudget(CIRCUIT_MATCH_BUDGET),
        CompletionBudget::new(4_096_usize.into(), 4_096_usize.into(), 4_096_usize.into()),
    );
    let sites = completion
        .matches
        .into_iter()
        .map(|matched| CircuitSite {
            pattern: matched.pattern,
            target: matched.target,
            admitted: matched.admitted,
            overlap_count: matched.overlap_count,
            certificates_replayed: matched.certificates_replayed,
        })
        .collect();
    let store = match completion.outcome {
        | CompletionOutcome::Completed { store, .. }
        | CompletionOutcome::Declined { store, .. } => store,
    };
    (sites, store)
}

/// The human-readable reason an operation earned its cell-layer decline.
fn op_decline_reason(error: OpElaborateError) -> DeclineReason<'static>
{
    DeclineReason(match error {
        | OpElaborateError::NoOutput => {
            "declares no output port, so its operation frame's return continuation would carry \
             nothing"
        },
        | OpElaborateError::ManyOutput => {
            "declares a many-out result: the cell layer's operation frame `f(p̄; c)` carries \
             exactly one return continuation, so a multi-output arity has no frame in this \
             grammar (proposal-levitation.md §4.2)"
        },
        | OpElaborateError::AggregatedOutput => {
            "aggregates several monomials into one output port, which needs the commutative \
             monoid the Σ-zone firewall keeps out of the Π-layer"
        },
    })
}

/// The located diagnostic a declined **circuit rule** member earns.
///
/// The composite refusals render here rather than through a `Display` on the
/// theory type, following [`ElaborateError::NonLinear`]'s precedent: the
/// boundary-language refusal is structured data, and the sentence that reads it
/// is the surface's.
fn circuit_decline_diagnostic(
    desc: &SignDesc,
    index: DeclinedCircuitIndex,
    error: &ElaborateError,
) -> ElabDiagnostic
{
    let rule = desc.circuits.get(usize::from(index));
    let span = rule.map_or_else(empty_surface_span, |rule| rule.sphere.provenance);
    let name = rule.map_or("<unknown>", |rule| rule.name.as_ref());
    let message = match *error {
        | ElaborateError::NoCircuitComposite(ref refusal) => {
            format!("circuit rule `{name}` {}", composite_refusal(refusal))
        },
        | ElaborateError::NonLinear(ref refusal) => {
            format!("circuit rule `{name}` is refused: {refusal}")
        },
        | ElaborateError::LhsNotOperation => format!(
            "circuit rule `{name}`'s derived source boundary is not an operation application, so \
             there is no redex for the cell layer to cut against"
        ),
        | ElaborateError::EmptyOperation => format!(
            "circuit rule `{name}`'s derived source boundary applies an operation to no \
             arguments, so there is no matched producer to cut against"
        ),
        | ElaborateError::UnsupportedShape => format!(
            "circuit rule `{name}`'s derived boundaries have a term shape outside the supported \
             flattening fragment"
        ),
        | ElaborateError::UnrepresentableOperation => format!(
            "circuit rule `{name}`'s derived boundaries apply an operation whose declared arity \
             the cell layer declined"
        ),
    };
    ElabDiagnostic::new(message, span)
}

/// The sentence a boundary-language refusal reads as.
fn composite_refusal(error: &CircuitElaborationError) -> String
{
    match *error {
        | CircuitElaborationError::Derivation(CircuitDerivationError::CyclicWiring(ref port)) => {
            format!(
                "has a wiring that reaches port `{port}` from itself, so it denotes no composite"
            )
        },
        | CircuitElaborationError::Derivation(CircuitDerivationError::NodeBudget { budget }) => {
            format!(
                "has a wiring that unfolds past the derivation's node budget of {budget}, so its \
                 composite is declined rather than built"
            )
        },
        | CircuitElaborationError::ManyRedexOccurrences { ref occurrences } => {
            format!(
                "has a body whose declared output port unfolds to {} redex occurrences ({}), so \
                 it denotes no single whiskered composite: two occurrences at incomparable \
                 positions are the horizontal composite an earned shift-equivalence witness \
                 licenses, and two at comparable positions are the sequential composition the \
                 boundary language spells `then`",
                occurrences.len(),
                render_occurrences(occurrences)
            )
        },
        | CircuitElaborationError::PositionOffBoundary {
            ref rewrite,
            ref position,
        } => format!(
            "has an occurrence of `{rewrite}` at {} that its derived boundary does not address",
            render_position(position)
        ),
    }
}

/// Every occurrence, as `` `name` at [i, j] ``, in the order the reading
/// reached them.
fn render_occurrences(occurrences: &[RedexOccurrence]) -> String
{
    occurrences
        .iter()
        .map(|occurrence| {
            format!(
                "`{}` at {}",
                occurrence.rewrite,
                render_position(&occurrence.position)
            )
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// A position path, as `[i, j]`.
fn render_position(position: &[TermPositionIndex]) -> String
{
    format!(
        "[{}]",
        position
            .iter()
            .map(|index| usize::from(*index).to_string())
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// The located diagnostic a declined `rule` face earns.
///
/// The three fragment declines and the operation-gate decline carry static
/// prose; the linearity refusal renders its own diagnostic, which names the
/// copied hole and the respelling.
fn face_decline_diagnostic(
    desc: &SignDesc,
    index: DeclinedFaceIndex,
    error: &ElaborateError,
) -> ElabDiagnostic
{
    let span = desc
        .rules
        .get(usize::from(index))
        .map_or_else(empty_surface_span, |face| face.provenance);
    let name = desc.id.name.as_ref();
    let message = match *error {
        | ElaborateError::LhsNotOperation => {
            format!(
                "rule of `{name}` does not rewrite an operation application on its left-hand side"
            )
        },
        | ElaborateError::EmptyOperation => {
            format!(
                "rule of `{name}` applies an operation to no arguments, so there is no matched \
                 producer to cut against"
            )
        },
        | ElaborateError::UnsupportedShape => {
            format!("rule of `{name}` has a term shape outside the supported flattening fragment")
        },
        | ElaborateError::UnrepresentableOperation => {
            format!(
                "rule of `{name}` applies an operation whose declared arity the cell layer declined"
            )
        },
        | ElaborateError::NonLinear(ref refusal) => {
            format!("rule of `{name}` is refused: {refusal}")
        },
        // A written face has no wiring, so it can never be refused a composite.
        | ElaborateError::NoCircuitComposite(_) => {
            format!("rule of `{name}` is refused a boundary-language composite it never had")
        },
    };
    ElabDiagnostic::new(message, span)
}
