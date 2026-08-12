//! Levitation **stage-0 elaboration**: `data` / `codata` blocks → the
//! `gandr_theory_levitation` declaration table (the levitation design's
//! description layer).
//!
//! This is the thin adapter between the parser's flat CST and the description
//! layer: it reads a parsed [`node_kinds::DATA_DECLARATION`] /
//! [`node_kinds::CODATA_DECLARATION`] and builds a [`SignDesc`], then runs
//! [`gandr_theory_levitation::check_desc`] over it. The bulk of the description
//! model and generic consumers live in `gandr-theory-levitation`; this module
//! provides the classification and conversion bridge consumed by the data and
//! codata lowerers.
//!
//! The grammar's `data` surface is a captured flat sequence in the checked PBG:
//! a declaration's members are inline tiles under one Meld, with only compound
//! field types and rule expressions nesting into sub-Melds. The reader is
//! therefore a small cursor over the declaration's significant children,
//! splitting members at their `;` terminators (with the retired `,` separator
//! admissible, so a stale declaration parses whole and reaches the migration
//! declines below).
//!
//! # What stage 0 elaborates
//!
//! * the **nested generator block** — THE one data-declaration form: the family
//!   head binds its parameters once as typed binders `(a : Type, …)` and
//!   carries the index arity `: Idx -> Type` (`: Type` when unindexed), and
//!   every generator member is a judgment `Ctor : (binders) --> Result` whose
//!   result head is the family applied to the parameter variables in order —
//!   **head uniformity**, checked here (an instantiated head is declined:
//!   instantiation is uninferable);
//! * constructors (`data`) and observations (`codata`) →
//!   [`gandr_theory_levitation::CtorDesc`] with a first-order
//!   [`gandr_theory_levitation::Code`] (fields fold right-nested; a
//!   self-referential field type is [`Code::Var`]);
//! * `op` members and parameterized observations →
//!   [`gandr_theory_levitation::OperDesc`] with a bridge arity;
//! * `rule` members → [`gandr_theory_levitation::RuleFace`] with derived
//!   per-variable metadata, the face written `lhs ==> rhs`;
//! * a **function-typed field** is declined at elaboration — it is outside the
//!   first-order fragment (proposal §8's `desc-higher-order-field`, pinning
//!   V2);
//! * the **retired head** — bare parameters `data Maybe(a)`, or the head
//!   without its annotation — declines the whole declaration with the
//!   respelling hint; the grammar keeps both admissible precisely so this
//!   decline can name the respelling;
//! * the **retired field-tuple constructor member** `Ctor(fields?)` declines
//!   with the generator-judgment respelling; a constructor-led member of a
//!   `codata` block declines (observations are lowercase-led);
//! * a `rule` member spelling its face with the **retired** `~>` is declined at
//!   elaboration and told the respelling — the block-form ruling made `==>` the
//!   face former at every position, and the grammar keeps `~>` admissible in
//!   the arrow slot precisely so this decline can name it.

use gandr_core_checker::boundary::GradeBound;
use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::grade::Grade;
use gandr_surface_syntax::NodeId;
use gandr_theory_levitation::Attr;
use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::BridgeArity;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::OperDesc;
use gandr_theory_levitation::ParamDesc;
use gandr_theory_levitation::PrimTy;
use gandr_theory_levitation::RuleFace;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::SortRef;
use gandr_theory_levitation::SurfaceSpan;
use gandr_theory_levitation::ValueTypeRef;
use gandr_theory_levitation::boundary::NominalSerial;
use gandr_theory_levitation::check_desc;
use gandr_theory_levitation::wellformed::derive_cell_var_meta;

use crate::boundary::MatchDecision;
use crate::boundary::PipelineSource;
use crate::boundary::TileSpelling;
use crate::boundary::TypeName;
use crate::circuit::desc as circuit_desc;
use crate::circuit::shape::Shape;
use crate::cst_read::BracketLabel;
use crate::cst_read::Cursor;
use crate::cst_read::Reader;
use crate::cst_read::empty_surface_span;
use crate::cst_read::grammar;
use crate::cst_read::is_closer;
use crate::cst_read::is_opener;
use crate::cst_read::member_runs;
use crate::cst_read::split_at_top_level;
use crate::lower::node_kinds;
use crate::synnode::SynTree;

/// The rewrite-face former of a `rule` member, ruled at `==>` for every
/// position by the block form
/// (`spec:surface-language/circuit-cells.md`).
const RULE_FACE_ARROW: &str = "==>";
/// The retired rewrite-face former. It still lexes and still parses in the
/// member's arrow slot, so a stale face reaches this elaborator whole and is
/// declined by name rather than repaired by the parser.
const RETIRED_RULE_FACE_ARROW: &str = "~>";

/// One elaboration **diagnostic** — an inspectable stage-0 decline (a
/// higher-order field, a retired face arrow, or a well-formedness failure
/// surfaced by [`gandr_theory_levitation::check_desc`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ElabDiagnostic
{
    /// A human-readable description of the decline.
    pub message: String,
    /// The surface span the decline was located at.
    pub span: SurfaceSpan,
}

impl ElabDiagnostic
{
    /// A diagnostic with the given message and span.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        message: String,
        span: SurfaceSpan,
    ) -> Self
    {
        Self { message, span }
    }
}

/// The result of elaborating a source's datatype declarations: the built
/// descriptions and every diagnostic, in declaration order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DescElab
{
    /// The elaborated descriptions, one per `data` / `codata` declaration.
    pub descs: Vec<SignDesc>,
    /// The diagnostics: elaboration declines plus every well-formedness
    /// failure.
    pub diagnostics: Vec<ElabDiagnostic>,
}
/// **Elaborate** every `data` / `codata` declaration in `source` into the decl
/// table, collecting descriptions and diagnostics.
///
/// # Contract
/// - requires: `source` parses (an unparseable source yields an empty result).
/// - ensures: returns one [`SignDesc`] per datatype declaration, in source
///   order, plus every elaboration decline and well-formedness diagnostic;
///   non-declaration items are ignored (the elaborator reads only the
///   description surface).
/// - fails: never; total on any input.
///
/// # Adequacy
/// - hypothesis: L3 — a nullary enum, a parameterized recursive datatype, and a
///   declined higher-order field are elaborated / declined distinctly.
/// - witness: `desc_elab` integration tests
///   (`crates/gandr-surface-engine/tests/desc_elab.rs`).
#[inline]
#[must_use]
pub fn elaborate_data_descs<'source, S>(source: S) -> DescElab
where
    S: Into<PipelineSource<'source>>,
{
    let source = source.into();
    let Ok(tree) = SynTree::parse(source.0)
    else {
        return DescElab::default();
    };
    let Some(pbg) = grammar()
    else {
        return DescElab::default();
    };
    let cst = tree.cst();
    let reader = Reader::new(pbg, cst);
    let shape = Shape { reader: &reader };
    let mut elab = DescElab::default();
    let mut serial = NominalSerial::from(0_u64);
    let mut circuit_forms = CircuitFormPresence(false);
    for item in tree.root().named_children() {
        // The circuit block form's two item leads take the circuit route: a
        // `sign` block is a declaration table of its own, and a top-level
        // `oper` / `rule` is a singleton one.
        let circuit = match item.kind() {
            | node_kinds::SIGN_DECLARATION => {
                circuit_forms = CircuitFormPresence(true);
                circuit_desc::sign_desc(shape, item.cst_node(), serial, &mut elab.diagnostics)
            },
            | node_kinds::CIRCUIT_DECLARATION => {
                circuit_forms = CircuitFormPresence(true);
                circuit_desc::circuit_declaration_desc(
                    shape,
                    item.cst_node(),
                    serial,
                    &mut elab.diagnostics,
                )
            },
            | _ => None,
        };
        if let Some(desc) = circuit {
            check_and_push(desc, &mut elab, &mut serial);
            continue;
        }
        let polarity = match item.kind() {
            | node_kinds::DATA_DECLARATION => DeclPolarity::Data,
            | node_kinds::CODATA_DECLARATION => DeclPolarity::Codata,
            | _ => continue,
        };
        if let Some(desc) = reader.declaration(item.cst_node(), polarity, serial, &mut elab) {
            check_and_push(desc, &mut elab, &mut serial);
        }
    }
    // The circuit surface check is the *other* reading of the same blocks — the
    // arrow-kind confirmation and the name-set fold that binds a body's
    // internal wires — and this is its production caller. It runs once over the
    // whole parse rather than per item, because the arrow confirmation resolves
    // applied heads against a file-level scope, and only when a circuit form is
    // present so no other program pays for it.
    if circuit_forms.0 {
        let items = reader.sig_children(cst.root());
        elab.diagnostics
            .extend(crate::circuit::check_over(&reader, &items).diagnostics);
    }
    elab
}

/// Whether a source carries any circuit block form, so the surface check runs
/// exactly when there is something for it to read.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CircuitFormPresence(bool);

/// Run the declaration table over one elaborated description, then record it.
fn check_and_push(
    desc: SignDesc,
    elab: &mut DescElab,
    serial: &mut NominalSerial,
)
{
    for diagnostic in check_desc(&desc) {
        elab.diagnostics.push(ElabDiagnostic::new(
            String::from(diagnostic.message),
            diagnostic.span.unwrap_or_else(empty_surface_span),
        ));
    }
    elab.descs.push(desc);
    *serial = NominalSerial::from(u64::from(*serial).saturating_add(1));
}

/// The index arity a family head's annotation declares — the count of `->`
/// steps along the right spine of `: Idx -> Type` (`: Type` when unindexed,
/// arity zero).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct IndexArity(usize);

/// One datatype declaration's head, as its generator members are checked
/// against it: the parameters bound once at the head, and the index arity the
/// head's annotation declares. The nested generator block makes the three
/// side conditions of the separation argument structural — head uniformity
/// (every generator's result head is the family applied to the parameter
/// variables), one head per block, and family-wide positivity — and this is
/// the record the uniformity check reads.
struct FamilyHead
{
    /// The head's parameters, in binding order.
    params: Vec<ParamDesc>,
    /// The declared index arity.
    indices: IndexArity,
}

/// The three parallel member lists of a declaration table under elaboration:
/// one member run appends into the constructor, operation, or rule-face list
/// by its lead label, and the lists grow together across the block.
struct MemberLists<'lists>
{
    /// The constructor descriptors accumulated so far.
    ctors: &'lists mut Vec<CtorDesc>,
    /// The operation descriptors accumulated so far.
    opers: &'lists mut Vec<OperDesc>,
    /// The rule faces accumulated so far.
    rules: &'lists mut Vec<RuleFace>,
}

impl<'tree> Reader<'tree>
{
    /// Read one datatype declaration node into a [`SignDesc`], appending any
    /// elaboration decline to `elab`.
    ///
    /// The head is the nested generator block's: typed parameter binders plus
    /// the mandatory index-arity annotation. A retired head — a bare
    /// parameter, or the annotation missing — declines the WHOLE declaration
    /// with the respelling hint (the family arity is unreadable without it,
    /// so no member can be checked).
    fn declaration(
        &self,
        node: NodeId,
        polarity: DeclPolarity,
        serial: NominalSerial,
        elab: &mut DescElab,
    ) -> Option<SignDesc>
    {
        let children = self.sig_children(node);
        let mut cursor = Cursor::new(self, &children);
        // `data` / `codata` lead.
        cursor.bump();
        let name_id = cursor.bump()?;
        let name = self.text(name_id).0.to_owned();
        let family = TypeName::from(name.as_str());
        let params = self.head_params(&mut cursor, family, elab)?;
        let indices = self.head_annotation(&mut cursor, family, elab)?;
        let head = FamilyHead { params, indices };
        if !cursor.eat(TileSpelling("{")).0 {
            return None;
        }
        let member_region = cursor.until_close_brace();
        let mut ctors: Vec<CtorDesc> = Vec::new();
        let mut opers: Vec<OperDesc> = Vec::new();
        let mut rules: Vec<RuleFace> = Vec::new();
        let mut lists = MemberLists {
            ctors: &mut ctors,
            opers: &mut opers,
            rules: &mut rules,
        };
        for member in member_runs(self, &member_region) {
            self.member(&member, family, &head, polarity, &mut lists, elab);
        }
        Some(SignDesc::new(
            NominalId::new(serial, name),
            head.params,
            ctors,
            opers,
            rules,
            polarity,
            Attrs::empty(),
        ))
    }

    /// Read the family head's parameter list `( a : Type, … )`: every
    /// parameter a **typed binder**. A bare parameter — the retired
    /// `data Maybe(a)` head, kept admissible by the grammar for exactly this
    /// decline — declines the whole declaration with the respelling hint.
    fn head_params(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
        family: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Option<Vec<ParamDesc>>
    {
        let mut params = Vec::new();
        if !cursor.eat(TileSpelling("(")).0 {
            return Some(params);
        }
        while let Some(id) = cursor.peek() {
            match self.label(id).map(|label| label.0) {
                | Some(")") => {
                    cursor.bump();
                    break;
                },
                | Some(",") => {
                    cursor.bump();
                },
                | _ => {
                    let param_id = cursor.bump()?;
                    let param = self.text(param_id).0;
                    if !cursor.eat(TileSpelling(":")).0 {
                        elab.diagnostics.push(ElabDiagnostic::new(
                            format!(
                                "the family `{family}`'s parameter `{param}` carries no type: \
                                 the head binds every parameter as a typed binder and carries \
                                 the index arity — respell it as `{family}({param} : Type, …) : \
                                 Type {{ … }}` (`: Type` when unindexed, `: Idx -> Type` when \
                                 indexed)",
                                family = family.0
                            ),
                            self.span(param_id),
                        ));
                        return None;
                    }
                    // The binder's type node (a tile or a compound Meld).
                    cursor.bump()?;
                    params.push(ParamDesc::new(param, Grade::ONE, Attrs::empty()));
                },
            }
        }
        Some(params)
    }

    /// Read the family head's index-arity annotation `: Idx -> Type` (`:
    /// Type` when unindexed). The annotation is **mandatory**: an unannotated
    /// head — the retired spelling, kept admissible by the grammar for
    /// exactly this decline — declines the whole declaration with the
    /// respelling hint.
    fn head_annotation(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
        family: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Option<IndexArity>
    {
        if !cursor.eat(TileSpelling(":")).0 {
            let span = cursor
                .peek()
                .map_or_else(empty_surface_span, |id| self.span(id));
            elab.diagnostics.push(ElabDiagnostic::new(
                format!(
                    "the family `{family}`'s head carries no index-arity annotation: respell \
                     it as `{family}(…) : Type {{ … }}` (`: Type` when unindexed, `: Idx -> \
                     Type` when indexed)",
                    family = family.0
                ),
                span,
            ));
            return None;
        }
        let annotation = cursor.bump()?;
        Some(self.index_arity(annotation))
    }

    /// Count a head annotation's index arity: the `->` steps along its right
    /// spine (`Nat -> Nat -> Type` indexes twice; `Type` is unindexed).
    fn index_arity(
        &self,
        annotation: NodeId,
    ) -> IndexArity
    {
        let mut arity = 0_usize;
        let mut node = annotation;
        while self.is_function_type(node).0 {
            arity = arity.saturating_add(1);
            let Some(next) = self.sig_children(node).last().copied()
            else {
                break;
            };
            node = next;
        }
        IndexArity(arity)
    }

    /// Elaborate one member run into the growing constructor / operation / cell
    /// lists.
    fn member(
        &self,
        run: &[NodeId],
        type_name: TypeName<'_>,
        head: &FamilyHead,
        polarity: DeclPolarity,
        lists: &mut MemberLists<'_>,
        elab: &mut DescElab,
    )
    {
        let Some(&first) = run.first()
        else {
            return;
        };
        match self.label(first).map(|label| label.0) {
            | Some("oper") => {
                if let Some(op) = self.oper_member(run) {
                    lists.opers.push(op);
                }
            },
            // The retired operation-member lead: after the respell, `oper` is
            // the 1-cell member and `op` is the operator-fixity declaration
            // only, so a stale member is told what to write rather than
            // silently accepted (the retired-`~>` precedent).
            | Some("op") => {
                elab.diagnostics.push(ElabDiagnostic::new(
                    "operation member lead `op` is retired; respell this member with `oper` \
                     (`op` is the operator-fixity declaration)"
                        .to_owned(),
                    self.span(first),
                ));
            },
            | Some("rule") => {
                if let Some(cell) = self.rule_member(run, elab) {
                    lists.rules.push(cell);
                }
            },
            | Some("constructor") => {
                // A constructor-led member of a `codata` block has no reading:
                // observations are lowercase-led, and generators belong to the
                // `data` block.
                if matches!(polarity, DeclPolarity::Codata) {
                    elab.diagnostics.push(ElabDiagnostic::new(
                        format!(
                            "constructor member `{}` has no place in a `codata` block: a codata \
                             block declares lowercase-led observations (`head : a`); \
                             constructors are the generator members of a `data` block",
                            self.text(first).0
                        ),
                        self.span(first),
                    ));
                    return;
                }
                // The generator judgment `Ctor : …` — THE one
                // constructor-declaration form — discriminates against the
                // retired field-tuple tail one tile after the constructor
                // name.
                if run
                    .get(1)
                    .and_then(|&id| self.label(id))
                    .map(|label| label.0)
                    == Some(":")
                {
                    if let Some(ctor) = self.generator_member(run, type_name, head, elab) {
                        lists.ctors.push(ctor);
                    }
                }
                else {
                    elab.diagnostics.push(ElabDiagnostic::new(
                        format!(
                            "the constructor-block member `{}` is retired; respell it as a \
                             generator judgment — `{} : Result` when it declares no fields, `{} \
                             : (binders) --> Result` when it does — whose result head is the \
                             family applied to its parameter variables",
                            self.text(first).0,
                            self.text(first).0,
                            self.text(first).0
                        ),
                        self.span(first),
                    ));
                }
            },
            // A lowercase-led member of a `codata` block: an observation.
            | _ if matches!(polarity, DeclPolarity::Codata) => {
                self.observation_member(run, type_name, lists.ctors, lists.opers, elab);
            },
            | _ => {},
        }
    }

    /// Elaborate a generator member `Ctor : Side ( --> Result )? [ attrs ]?` —
    /// THE one constructor-declaration form of the nested generator block.
    ///
    /// The side ladder mirrors the circuit signature's: no arrow means the
    /// side IS the result (`Nil : Vec(a, 0)` declares no fields); an arrow
    /// makes the side the payload and the post-arrow type the result — a
    /// parenthesized binder telescope (`Cons : (n : Nat, x : a) --> Vec(a, n)`)
    /// or a bare single-field sort (`Succ : Nat --> Nat`).
    ///
    /// **Head uniformity is enforced here, executably**: the result head is
    /// the family applied to the parameter variables in order, with the index
    /// arguments admitted unread (index expressions are admitted syntactically
    /// as far as the `Type` sort reaches; their semantics are the
    /// arity-substitution lane's). A bare result head is exact only for an
    /// unparameterized, unindexed family. A violation — a foreign head, the
    /// wrong argument count, or an instantiated parameter — declines the
    /// member: instantiation is uninferable.
    fn generator_member(
        &self,
        run: &[NodeId],
        type_name: TypeName<'_>,
        head: &FamilyHead,
        elab: &mut DescElab,
    ) -> Option<CtorDesc>
    {
        let mut cursor = Cursor::new(self, run);
        let name_id = cursor.bump()?;
        let name = self.text(name_id).0.to_owned();
        // The `:` lead the dispatch matched on.
        cursor.bump();
        let (fields, result_id) = if cursor.eat(TileSpelling("(")).0 {
            let fields = self.generator_telescope(&mut cursor, type_name, elab);
            match cursor.bump() {
                | Some(arrow) if self.label(arrow).map(|label| label.0) == Some("-->") => {
                    let result = cursor.bump()?;
                    (fields, result)
                },
                | _ => {
                    elab.diagnostics.push(ElabDiagnostic::new(
                        format!(
                            "generator `{name}` declares a binder telescope but no `-->` result: \
                             the result head is the family `{type_name}` applied to its \
                             parameter variables — write `{name} : (binders) --> {type_name}(…)`",
                            type_name = type_name.0
                        ),
                        self.span(name_id),
                    ));
                    return None;
                },
            }
        }
        else {
            let side = cursor.bump()?;
            if cursor
                .peek()
                .and_then(|id| self.label(id).map(|label| label.0))
                == Some("-->")
            {
                cursor.bump();
                let result = cursor.bump()?;
                let field = self.field_type_code(side, type_name, Grade::ONE, elab);
                (vec![field], result)
            }
            else {
                (Vec::new(), side)
            }
        };
        let attrs = self.attr_slot(&mut cursor);
        self.check_result_uniformity(
            result_id,
            NameRef::from(name.as_str()),
            type_name,
            head,
            elab,
        )?;
        let result: Name = type_name.0.into();
        Some(CtorDesc::new(name, Code::product_of(fields), result, attrs))
    }

    /// Read a generator's parenthesized telescope `( entry, … )` into field
    /// codes: named binders `[grade?] name : Type` (the constructor-field
    /// shape) or bare-sort ports, the same sugar ladder the circuit parameter
    /// side climbs.
    fn generator_telescope(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
        type_name: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Vec<Code>
    {
        let mut fields = Vec::new();
        let mut interior: Vec<NodeId> = Vec::new();
        let mut depth: u32 = 0;
        while let Some(id) = cursor.bump() {
            let label = self.label(id).map(|label| label.0);
            if depth == 0 && label == Some(")") {
                break;
            }
            match label {
                | Some(bracket) if is_opener(BracketLabel(bracket)).0 => {
                    depth = depth.saturating_add(1);
                },
                | Some(bracket) if is_closer(BracketLabel(bracket)).0 => {
                    depth = depth.saturating_sub(1);
                },
                | _ => {},
            }
            interior.push(id);
        }
        for entry in split_at_top_level(self, &interior, TileSpelling(",")) {
            // A bare-sort port is one type node; anything longer is the named
            // binder `[grade?] name : Type`.
            if let &[single] = entry.as_slice() {
                fields.push(self.field_type_code(single, type_name, Grade::ONE, elab));
            }
            else {
                let mut entry_cursor = Cursor::new(self, &entry);
                if let Some(code) = self.field(&mut entry_cursor, type_name, elab) {
                    fields.push(code);
                }
            }
        }
        fields
    }

    /// Enforce head uniformity on one generator's result: the result head is
    /// the family applied to the parameter variables in order, with the index
    /// arguments admitted unread. A violation appends the decline and returns
    /// [`None`].
    fn check_result_uniformity(
        &self,
        result_id: NodeId,
        ctor: NameRef<'_>,
        family: TypeName<'_>,
        head: &FamilyHead,
        elab: &mut DescElab,
    ) -> Option<()>
    {
        let ctor: &str = ctor.as_ref();
        let family = family.0;
        let expected = head.params.len().saturating_add(head.indices.0);
        let decline = |message: String, elab: &mut DescElab| {
            elab.diagnostics
                .push(ElabDiagnostic::new(message, self.span(result_id)));
        };
        // A bare result head — the family name with no argument list — is
        // exact only for an unparameterized, unindexed family.
        if !self.is_meld(result_id).0 {
            let text = self.text(result_id).0;
            if text == family && expected == 0 {
                return Some(());
            }
            if text == family {
                decline(
                    format!(
                        "generator `{ctor}`'s bare result head takes no arguments, but the \
                         family `{family}` takes {expected} ({} parameter(s) + {} index(es)) — \
                         write the result as `{family}` applied to its parameter variables in \
                         order",
                        head.params.len(),
                        head.indices.0
                    ),
                    elab,
                );
            }
            else {
                decline(
                    format!(
                        "generator `{ctor}`'s result head is `{text}`, not the family \
                         `{family}`: every generator of `{family}` constructs `{family}`"
                    ),
                    elab,
                );
            }
            return None;
        }
        let inner = self.sig_children(result_id);
        let Some((&head_id, tail)) = inner.split_first()
        else {
            decline(
                format!(
                    "generator `{ctor}`'s result is not the family `{family}` applied to its \
                     parameter variables"
                ),
                elab,
            );
            return None;
        };
        if self.text(head_id).0 != family {
            decline(
                format!(
                    "generator `{ctor}`'s result head is `{}`, not the family `{family}`: \
                     every generator of `{family}` constructs `{family}` — a generator of a \
                     specialized family has no eliminator schema and no place in one \
                     description",
                    self.text(head_id).0
                ),
                elab,
            );
            return None;
        }
        let Some((&open_id, arg_region)) = tail.split_first()
        else {
            decline(
                format!(
                    "generator `{ctor}`'s result head is the family `{family}` but applies it \
                     to nothing: the family takes {expected} argument(s) ({} parameter(s) + {} \
                     index(es))",
                    head.params.len(),
                    head.indices.0
                ),
                elab,
            );
            return None;
        };
        if self.label(open_id).map(|label| label.0) != Some("(") {
            decline(
                format!(
                    "generator `{ctor}`'s result is not the family `{family}` applied to its \
                     parameter variables in order"
                ),
                elab,
            );
            return None;
        }
        let arg_region = match arg_region.split_last() {
            | Some((close, args)) if self.label(*close).map(|label| label.0) == Some(")") => args,
            | _ => arg_region,
        };
        let args = split_at_top_level(self, arg_region, TileSpelling(","));
        if args.len() != expected {
            decline(
                format!(
                    "generator `{ctor}`'s result head takes {} argument(s); the family \
                     `{family}` takes {expected} ({} parameter(s) + {} index(es))",
                    args.len(),
                    head.params.len(),
                    head.indices.0
                ),
                elab,
            );
            return None;
        }
        // The parameter arguments are the parameter VARIABLES in binding
        // order, never instantiations; the index arguments that follow are
        // admitted unread.
        for (arg, param) in args.iter().zip(head.params.iter()) {
            let is_variable = match arg.as_slice() {
                | &[arg_id] => {
                    self.label(arg_id).map(|label| label.0) == Some("type_variable")
                        && self.text(arg_id).0 == param.name.as_ref()
                },
                | _ => false,
            };
            if !is_variable {
                decline(
                    format!(
                        "generator `{ctor}`'s result instantiates parameter `{}`; instantiation \
                         is uninferable — the result head applies the family `{family}` to its \
                         parameter variables in order",
                        param.name
                    ),
                    elab,
                );
                return None;
            }
        }
        Some(())
    }

    /// Read one field `[grade?] name : Type` into a field code (or a
    /// [`Code::Var`] recursive occurrence).
    fn field(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
        type_name: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Option<Code>
    {
        // Optional grade prefix (`1` / `ω`).
        let grade = match cursor
            .peek()
            .and_then(|id| self.label(id).map(|label| label.0))
        {
            | Some("number") => {
                let id = cursor.bump()?;
                let value = self.text(id).0.parse::<u64>().unwrap_or(1);
                Some(Grade::fin(GradeBound::from(value)))
            },
            | Some("ω") => {
                cursor.bump();
                Some(Grade::OMEGA)
            },
            | _ => None,
        };
        // The field name, then `:`.
        cursor.bump()?;
        if !cursor.eat(TileSpelling(":")).0 {
            return None;
        }
        let type_id = cursor.bump()?;
        Some(self.field_type_code(type_id, type_name, grade.unwrap_or(Grade::ONE), elab))
    }

    /// Turn a field's type node into a code: [`Code::Var`] for a recursive
    /// occurrence of `type_name`, else a [`Code::Field`] (declining a
    /// function-typed field as higher-order).
    fn field_type_code(
        &self,
        type_id: NodeId,
        type_name: TypeName<'_>,
        grade: Grade,
        elab: &mut DescElab,
    ) -> Code
    {
        if self.is_function_type(type_id).0 {
            elab.diagnostics.push(ElabDiagnostic::new(
                format!(
                    "field of function type `{}` is outside the first-order code fragment \
                     `{{1, var, ×, σ}}` (proposal-levitation.md §3, V2)",
                    self.text(type_id).0
                ),
                self.span(type_id),
            ));
        }
        if self.type_head(type_id).as_deref() == Some(type_name.0) {
            return Code::var(type_name.0);
        }
        Code::field(self.type_ref(type_id), grade, Attrs::empty())
    }

    /// The head name of a type node: a bare token's text, or a type
    /// application's leading tile text.
    fn type_head(
        &self,
        type_id: NodeId,
    ) -> Option<String>
    {
        if self.is_meld(type_id).0 {
            let inner = self.sig_children(type_id);
            let head = inner.first().copied()?;
            Some(self.text(head).0.to_owned())
        }
        else {
            Some(self.text(type_id).0.to_owned())
        }
    }

    /// Whether a type node is a function type `A -> B` (a Meld carrying a
    /// top-level `->` tile) — the higher-order shape excluded from the
    /// fragment.
    fn is_function_type(
        &self,
        type_id: NodeId,
    ) -> MatchDecision
    {
        MatchDecision(
            self.is_meld(type_id).0
                && self
                    .sig_children(type_id)
                    .into_iter()
                    .any(|child| self.label(child).map(|label| label.0) == Some("->")),
        )
    }

    /// Read a non-recursive field type into a [`ValueTypeRef`].
    fn type_ref(
        &self,
        type_id: NodeId,
    ) -> ValueTypeRef
    {
        if self.is_meld(type_id).0 {
            let inner = self.sig_children(type_id);
            let head = inner
                .first()
                .map_or_else(String::new, |&id| self.text(id).0.to_owned());
            let args: Vec<ValueTypeRef> = inner
                .iter()
                .copied()
                .skip(1)
                .filter(|&id| !matches!(self.label(id).map(|label| label.0), Some("(" | ")" | ",")))
                .map(|id| self.type_ref(id))
                .collect();
            return ValueTypeRef::Ctor {
                head: head.into(),
                args: args.into_boxed_slice(),
            };
        }
        let text = self.text(type_id).0;
        if self.label(type_id).map(|label| label.0) == Some("type_variable") {
            ValueTypeRef::Param(text.into())
        }
        else if let Some(prim) = PrimTy::from_label(NameRef::from(text)) {
            ValueTypeRef::Prim(prim)
        }
        else {
            ValueTypeRef::Ctor {
                head: text.into(),
                args: Box::default(),
            }
        }
    }

    /// Read the optional per-symbol attribute slot `[ name, … ]`.
    fn attr_slot(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
    ) -> Attrs
    {
        if !cursor.eat(TileSpelling("[")).0 {
            return Attrs::empty();
        }
        let mut markers = Vec::new();
        while let Some(id) = cursor.peek() {
            match self.label(id).map(|label| label.0) {
                | Some("]") => {
                    cursor.bump();
                    break;
                },
                | Some(",") => {
                    cursor.bump();
                },
                | _ => {
                    markers.push(Attr::marker(self.text(id).0));
                    cursor.bump();
                },
            }
        }
        Attrs::new(markers)
    }

    /// Elaborate an `op name ( params ) ( -> Result )?` member into an
    /// [`OperDesc`].
    fn oper_member(
        &self,
        run: &[NodeId],
    ) -> Option<OperDesc>
    {
        let mut cursor = Cursor::new(self, run);
        cursor.bump(); // `op`
        let name_id = cursor.bump()?;
        let name = self.text(name_id).0.to_owned();
        let inputs = self.op_params(&mut cursor);
        let outputs = if cursor.eat(TileSpelling("->")).0 {
            self.op_result(&mut cursor)
        }
        else {
            Vec::new()
        };
        Some(OperDesc::new(
            name,
            bridge_arity(inputs, outputs),
            Attrs::empty(),
        ))
    }

    /// Read an operation's parameter list `( name : Type, … )` into input
    /// ports.
    fn op_params(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
    ) -> Vec<SortRef>
    {
        let mut ports = Vec::new();
        if !cursor.eat(TileSpelling("(")).0 {
            return ports;
        }
        while let Some(id) = cursor.peek() {
            match self.label(id).map(|label| label.0) {
                | Some(")") => {
                    cursor.bump();
                    break;
                },
                | Some(",") => {
                    cursor.bump();
                },
                | _ => {
                    if let Some(port) = self.named_port(cursor) {
                        ports.push(port);
                    }
                    else {
                        cursor.bump();
                    }
                },
            }
        }
        ports
    }

    /// Read one `name : Type` port.
    fn named_port(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
    ) -> Option<SortRef>
    {
        let name_id = cursor.peek()?;
        let name = self.text(name_id).0.to_owned();
        cursor.bump();
        if !cursor.eat(TileSpelling(":")).0 {
            return None;
        }
        let type_id = cursor.bump()?;
        Some(SortRef::new(name, self.text(type_id).0))
    }

    /// Read an operation's result: a single type (one anonymous output port) or
    /// a named multi-out tuple `( q : A, r : B )`.
    fn op_result(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
    ) -> Vec<SortRef>
    {
        if cursor
            .peek()
            .and_then(|id| self.label(id).map(|label| label.0))
            == Some("(")
        {
            return self.op_params(cursor);
        }
        cursor
            .bump()
            .map(|id| vec![SortRef::new("out", self.text(id).0)])
            .unwrap_or_default()
    }

    /// Elaborate a `rule lhs ==> rhs` member into a [`RuleFace`] with derived
    /// per-variable metadata, declining the retired `~>` spelling by name.
    ///
    /// The block-form ruling
    /// (`spec:surface-language/circuit-cells.md` §"The block form,
    /// ruled") makes `==>` the rewrite-face former at every position. The
    /// grammar still admits `~>` in this slot so a stale face arrives here
    /// whole: the decline can then quote the member and name the respelling,
    /// which a parse repair over a lone token could not.
    fn rule_member(
        &self,
        run: &[NodeId],
        elab: &mut DescElab,
    ) -> Option<RuleFace>
    {
        // `rule <lhs> ==> <rhs>`: the lhs is the single expression node after
        // the `rule` lead; the rhs is the single node after the face arrow.
        let lead = run.first().copied()?;
        let lhs_id = run.get(1).copied()?;
        let arrow_pos = run.iter().position(|&id| {
            matches!(
                self.label(id).map(|label| label.0),
                Some(RULE_FACE_ARROW | RETIRED_RULE_FACE_ARROW)
            )
        })?;
        let arrow_id = run.get(arrow_pos).copied()?;
        if self.label(arrow_id).map(|label| label.0) == Some(RETIRED_RULE_FACE_ARROW) {
            elab.diagnostics.push(ElabDiagnostic::new(
                format!(
                    "rule face arrow `{RETIRED_RULE_FACE_ARROW}` is retired; respell this face \
                     with `{RULE_FACE_ARROW}`"
                ),
                self.span(arrow_id),
            ));
            return None;
        }
        let rhs_id = run.get(arrow_pos.saturating_add(1)).copied()?;
        let lhs = self.free_term(lhs_id);
        let rhs = self.free_term(rhs_id);
        let vars = derive_cell_var_meta(&lhs);
        let span = SurfaceSpan::new(self.span(lead).start, self.span(rhs_id).end);
        Some(RuleFace::new(lhs, rhs, vars, span))
    }

    /// Read an expression node into a [`FreeTerm`]: an application
    /// `head(args)`, a bare constructor (nullary), or a variable.
    pub(crate) fn free_term(
        &self,
        id: NodeId,
    ) -> FreeTerm
    {
        if self.is_meld(id).0 {
            let inner = self.sig_children(id);
            let Some(&head) = inner.first()
            else {
                return FreeTerm::var(self.text(id).0);
            };
            let head_text = self.text(head).0.to_owned();
            let args: Vec<FreeTerm> = inner
                .iter()
                .copied()
                .skip(1)
                .filter(|&child| {
                    !matches!(
                        self.label(child).map(|label| label.0),
                        Some("(" | ")" | ",")
                    )
                })
                .map(|child| self.free_term(child))
                .collect();
            if self.label(head).map(|label| label.0) == Some("constructor") {
                return FreeTerm::Ctor(head_text.into(), args.into_boxed_slice());
            }
            return FreeTerm::Op(head_text.into(), args.into_boxed_slice());
        }
        let text = self.text(id).0;
        if self.label(id).map(|label| label.0) == Some("constructor") {
            FreeTerm::Ctor(text.into(), Box::default())
        }
        else {
            FreeTerm::Var(text.into())
        }
    }

    /// Elaborate a codata observation member `[grade?] name (params?) : Type`
    /// into a constructor-shaped entry (no params) or an operation (params).
    fn observation_member(
        &self,
        run: &[NodeId],
        type_name: TypeName<'_>,
        ctors: &mut Vec<CtorDesc>,
        opers: &mut Vec<OperDesc>,
        elab: &mut DescElab,
    )
    {
        let mut cursor = Cursor::new(self, run);
        // Optional grade prefix.
        let grade = match cursor
            .peek()
            .and_then(|id| self.label(id).map(|label| label.0))
        {
            | Some("number") => {
                let value = cursor
                    .bump()
                    .and_then(|id| self.text(id).0.parse::<u64>().ok())
                    .unwrap_or(1);
                Grade::fin(GradeBound::from(value))
            },
            | Some("ω") => {
                cursor.bump();
                Grade::OMEGA
            },
            | _ => Grade::ONE,
        };
        let Some(name_id) = cursor.bump()
        else {
            return;
        };
        let name = self.text(name_id).0.to_owned();
        // A parameterized observation carries an input list: elaborate as an op.
        if cursor
            .peek()
            .and_then(|id| self.label(id).map(|label| label.0))
            == Some("(")
        {
            let inputs = self.op_params(&mut cursor);
            let outputs = if cursor.eat(TileSpelling(":")).0 {
                cursor
                    .bump()
                    .map(|id| vec![SortRef::new(name.clone(), self.text(id).0)])
                    .unwrap_or_default()
            }
            else {
                Vec::new()
            };
            opers.push(OperDesc::new(
                name,
                bridge_arity(inputs, outputs),
                Attrs::empty(),
            ));
            return;
        }
        if !cursor.eat(TileSpelling(":")).0 {
            return;
        }
        let Some(type_id) = cursor.bump()
        else {
            return;
        };
        let code = self.field_type_code(type_id, type_name, grade, elab);
        ctors.push(CtorDesc::new(name, code, type_name.0, Attrs::empty()));
    }
}

/// Build a bridge arity for an operation from its input and output ports: one
/// monomial per output, each reading all inputs (a well-formed Π-layer shape;
/// §4.2). A single (or absent) output uses [`BridgeArity::single_output`].
pub(crate) fn bridge_arity(
    inputs: Vec<SortRef>,
    outputs: Vec<SortRef>,
) -> BridgeArity
{
    let input_count = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
    if outputs.len() <= 1 {
        let output = outputs
            .into_iter()
            .next()
            .unwrap_or_else(|| SortRef::new("out", "Unit"));
        return BridgeArity::single_output(inputs, output);
    }
    let mut factors: Vec<u32> = Vec::new();
    let mut source: Vec<u32> = Vec::new();
    let mut dest: Vec<u32> = Vec::new();
    for (index, _output) in outputs.iter().enumerate() {
        factors.push(input_count);
        source.extend(0 .. input_count);
        dest.push(u32::try_from(index).unwrap_or(u32::MAX));
    }
    BridgeArity::new(inputs, factors, source, dest, outputs)
}
