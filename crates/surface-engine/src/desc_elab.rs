//! Levitation **stage-0 elaboration**: `data` / `codata` blocks → the
//! `gandr_theory_levitation` declaration table (proposal-levitation.md §3).
//!
//! This is the thin adapter between the parser's flat CST and the description
//! layer: it reads a parsed [`node_kinds::DATA_DECLARATION`] /
//! [`node_kinds::CODATA_DECLARATION`] and builds a [`DataDesc`], then runs
//! [`gandr_theory_levitation::check_desc`] over it. The bulk of the description
//! model and generic consumers live in `gandr-theory-levitation`; this module
//! provides the classification and conversion bridge consumed by the data and
//! codata lowerers.
//!
//! The grammar's `data` surface is a captured flat sequence in the checked PBG:
//! a declaration's members are inline tiles under one Meld, with only compound
//! field types and rule expressions nesting into sub-Melds. The reader is
//! therefore a small cursor over the declaration's significant children,
//! splitting members at bracket-depth-zero commas.
//!
//! # What stage 0 elaborates
//!
//! * constructors (`data`) and observations (`codata`) →
//!   [`gandr_theory_levitation::CtorDesc`] with a first-order
//!   [`gandr_theory_levitation::Code`] (fields fold right-nested; a
//!   self-referential field type is [`Code::Var`]);
//! * `op` members and parameterized observations →
//!   [`gandr_theory_levitation::OpDesc`] with a bridge arity;
//! * `rule` members → [`gandr_theory_levitation::CellFace`] with derived
//!   per-variable metadata;
//! * a **function-typed field** is declined at elaboration — it is outside the
//!   first-order fragment (proposal §8's `desc-higher-order-field`, pinning
//!   V2).

use gandr_core_checker::boundary::GradeBound;
use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::grade::Grade;
use gandr_surface_syntax::NodeId;
use gandr_theory_levitation::Attr;
use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::BridgeArity;
use gandr_theory_levitation::CellFace;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::OpDesc;
use gandr_theory_levitation::ParamDesc;
use gandr_theory_levitation::PrimTy;
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
use crate::cst_read::Cursor;
use crate::cst_read::Reader;
use crate::cst_read::empty_surface_span;
use crate::cst_read::grammar;
use crate::cst_read::split_at_top_level;
use crate::lower::node_kinds;
use crate::synnode::SynTree;

/// One elaboration **diagnostic** — an inspectable stage-0 decline (a
/// higher-order field, or a well-formedness failure surfaced by
/// [`gandr_theory_levitation::check_desc`]).
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
    pub descs: Vec<DataDesc>,
    /// The diagnostics: elaboration declines plus every well-formedness
    /// failure.
    pub diagnostics: Vec<ElabDiagnostic>,
}
/// **Elaborate** every `data` / `codata` declaration in `source` into the decl
/// table, collecting descriptions and diagnostics.
///
/// # Contract
/// - requires: `source` parses (an unparseable source yields an empty result).
/// - ensures: returns one [`DataDesc`] per datatype declaration, in source
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
    let mut elab = DescElab::default();
    let mut serial: u64 = 0;
    for item in tree.root().named_children() {
        let polarity = match item.kind() {
            | node_kinds::DATA_DECLARATION => DeclPolarity::Data,
            | node_kinds::CODATA_DECLARATION => DeclPolarity::Codata,
            | _ => continue,
        };
        let reader = Reader::new(pbg, cst);
        if let Some(desc) = reader.declaration(
            item.cst_node(),
            polarity,
            NominalSerial::from(serial),
            &mut elab,
        ) {
            for diagnostic in check_desc(&desc) {
                elab.diagnostics.push(ElabDiagnostic::new(
                    String::from(diagnostic.message),
                    diagnostic.span.unwrap_or_else(empty_surface_span),
                ));
            }
            elab.descs.push(desc);
            serial = serial.saturating_add(1);
        }
    }
    elab
}

/// The three parallel member lists of a declaration table under elaboration:
/// one member run appends into the constructor, observation (operation), or
/// cell list by its lead label, and the lists grow together across the block.
struct MemberLists<'lists>
{
    /// The constructor descriptors accumulated so far.
    ctors: &'lists mut Vec<CtorDesc>,
    /// The observation (operation) descriptors accumulated so far.
    ops: &'lists mut Vec<OpDesc>,
    /// The cell faces accumulated so far.
    cells: &'lists mut Vec<CellFace>,
}

impl<'tree> Reader<'tree>
{
    /// Read one datatype declaration node into a [`DataDesc`], appending any
    /// elaboration decline to `elab`.
    fn declaration(
        &self,
        node: NodeId,
        polarity: DeclPolarity,
        serial: NominalSerial,
        elab: &mut DescElab,
    ) -> Option<DataDesc>
    {
        let children = self.sig_children(node);
        let mut cursor = Cursor::new(self, &children);
        // `data` / `codata` lead.
        cursor.bump();
        let name_id = cursor.bump()?;
        let name = self.text(name_id).0.to_owned();
        let params = self.type_params(&mut cursor);
        if !cursor.eat(TileSpelling("{")).0 {
            return None;
        }
        let member_region = cursor.until_close_brace();
        let mut ctors: Vec<CtorDesc> = Vec::new();
        let mut ops: Vec<OpDesc> = Vec::new();
        let mut cells: Vec<CellFace> = Vec::new();
        let mut lists = MemberLists {
            ctors: &mut ctors,
            ops: &mut ops,
            cells: &mut cells,
        };
        for member in split_at_top_level(self, &member_region, TileSpelling(",")) {
            self.member(
                &member,
                TypeName::from(name.as_str()),
                polarity,
                &mut lists,
                elab,
            );
        }
        Some(DataDesc::new(
            NominalId::new(serial, name),
            params,
            ctors,
            ops,
            cells,
            polarity,
            Attrs::empty(),
        ))
    }

    /// Read the optional type-parameter list `( a, b, … )`, if the cursor is at
    /// one.
    fn type_params(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
    ) -> Vec<ParamDesc>
    {
        let mut params = Vec::new();
        if !cursor.eat(TileSpelling("(")).0 {
            return params;
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
                    params.push(ParamDesc::new(self.text(id).0, Grade::ONE, Attrs::empty()));
                    cursor.bump();
                },
            }
        }
        params
    }

    /// Elaborate one member run into the growing constructor / operation / cell
    /// lists.
    fn member(
        &self,
        run: &[NodeId],
        type_name: TypeName<'_>,
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
            | Some("op") => {
                if let Some(op) = self.op_member(run) {
                    lists.ops.push(op);
                }
            },
            | Some("rule") => {
                if let Some(cell) = self.rule_member(run) {
                    lists.cells.push(cell);
                }
            },
            | Some("constructor") => {
                if let Some(ctor) = self.ctor_member(run, type_name, elab) {
                    lists.ctors.push(ctor);
                }
            },
            // A lowercase-led member of a `codata` block: an observation.
            | _ if matches!(polarity, DeclPolarity::Codata) => {
                self.observation_member(run, type_name, lists.ctors, lists.ops, elab);
            },
            | _ => {},
        }
    }

    /// Elaborate a constructor member `C ( fields? ) ( : Result )? [ attrs ]?`.
    fn ctor_member(
        &self,
        run: &[NodeId],
        type_name: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Option<CtorDesc>
    {
        let mut cursor = Cursor::new(self, run);
        let name_id = cursor.bump()?;
        let name = self.text(name_id).0.to_owned();
        let fields = self.field_list(&mut cursor, type_name, elab);
        let code = Code::product_of(fields);
        // Optional GADT result annotation `: Result`.
        let result = if cursor.eat(TileSpelling(":")).0 {
            cursor.bump().map(|id| self.text(id).0.to_owned().into())
        }
        else {
            None
        };
        let attrs = self.attr_slot(&mut cursor);
        Some(CtorDesc::new(name, code, result, attrs))
    }

    /// Read a constructor / observation field list `( [grade?] name : Type, …
    /// )` into field codes.
    fn field_list(
        &self,
        cursor: &mut Cursor<'_, 'tree>,
        type_name: TypeName<'_>,
        elab: &mut DescElab,
    ) -> Vec<Code>
    {
        let mut fields = Vec::new();
        if !cursor.eat(TileSpelling("(")).0 {
            return fields;
        }
        loop {
            match cursor
                .peek()
                .and_then(|id| self.label(id).map(|label| label.0))
            {
                | Some(")") => {
                    cursor.bump();
                    break;
                },
                | Some(",") => {
                    cursor.bump();
                },
                | None if cursor.peek().is_none() => break,
                | _ => {
                    if let Some(code) = self.field(cursor, type_name, elab) {
                        fields.push(code);
                    }
                    else {
                        cursor.bump();
                    }
                },
            }
        }
        fields
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
            return Code::Var;
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
    /// [`OpDesc`].
    fn op_member(
        &self,
        run: &[NodeId],
    ) -> Option<OpDesc>
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
        Some(OpDesc::new(
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

    /// Elaborate a `rule lhs ~> rhs` member into a [`CellFace`] with derived
    /// per-variable metadata.
    fn rule_member(
        &self,
        run: &[NodeId],
    ) -> Option<CellFace>
    {
        // `rule <lhs> ~> <rhs>`: the lhs is the single expression node after the
        // `rule` lead; the rhs is the single node after the `~>` tile.
        let lead = run.first().copied()?;
        let lhs_id = run.get(1).copied()?;
        let arrow_pos = run
            .iter()
            .position(|&id| self.label(id).map(|label| label.0) == Some("~>"))?;
        let rhs_id = run.get(arrow_pos.saturating_add(1)).copied()?;
        let lhs = self.free_term(lhs_id);
        let rhs = self.free_term(rhs_id);
        let vars = derive_cell_var_meta(&lhs);
        let span = SurfaceSpan::new(self.span(lead).start, self.span(rhs_id).end);
        Some(CellFace::new(lhs, rhs, vars, span))
    }

    /// Read an expression node into a [`FreeTerm`]: an application
    /// `head(args)`, a bare constructor (nullary), or a variable.
    fn free_term(
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
        ops: &mut Vec<OpDesc>,
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
            ops.push(OpDesc::new(
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
        ctors.push(CtorDesc::new(name, code, None, Attrs::empty()));
    }
}

/// Build a bridge arity for an operation from its input and output ports: one
/// monomial per output, each reading all inputs (a well-formed Π-layer shape;
/// §4.2). A single (or absent) output uses [`BridgeArity::single_output`].
fn bridge_arity(
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
