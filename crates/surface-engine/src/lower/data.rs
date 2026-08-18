//! Declared data: the positive-half elaboration (the declared-data design).
//!
//! A `data D(ā) { C₀, C₁(x: A), … }` block declares generative-nominal
//! **constructors** (1-cells). A constructor application `C(v̄)` introduces a
//! value of `D(ā)` — the [`gandr_core_term::syntax::Value::Ctor`]
//! tagged value — and a `case v { Cᵢ(x̄ᵢ) => eᵢ }` eliminates it through the
//! [`gandr_core_term::syntax::Comp::DataCase`] case-over-the-tag
//! (declared-data design Decisions 2/3).
//!
//! # The pipeline seam (declared-data design Decision 4)
//!
//! This module is the single seam that maps a
//! `gandr_theory_levitation::NominalId` (serial + name, minted by
//! [`crate::desc_elab::elaborate_data_descs`]) to the core-local
//! [`gandr_core_term::types::DataId`] the runtime former compares on.
//! The registry [`DataDecl`] holds the decl table the frozen core deliberately
//! does **not** carry — the constructor enumeration (names in tag order) — so
//! constructor application, `case` lowering, the `D(ā)` type former, and the
//! renderer all resolve against it.
//!
//! # MVP boundaries (declared-data design Decisions 6/7)
//!
//! * **Non-recursive.** A recursive datatype (a constructor field whose type is
//!   the type being declared) is declined off `SignDesc::is_recursive` until
//!   the value-level `μ⁺` rung lands.
//! * **Field discipline.** A constructor application under a `D(ā)` ascription
//!   checks each payload field against its declared type instantiated at `ā`
//!   ([`Lowerer::instantiated_field_types`]) — the soundness the frozen core's
//!   tag-only `rule_ctor` cannot enforce (declared-data design Decision 4/5). A
//!   field type outside the resolvable first-order fragment (an applied
//!   non-declared former) is checked permissively (`Unknown`), a residual.
//! * **Arity.** A nullary constructor's payload is `()`, a one-field
//!   constructor's is the field value directly, and a many-field constructor's
//!   is the right-nested product of its fields; a `case` arm destructures the
//!   product positionally into its field binders.
//! * **Ordered arms, one per head.** A `case` arm's pattern is compiled by
//!   [`super::pattern`], which admits constructor patterns nested to any depth,
//!   wildcard and binder sub-patterns, as-binders, and pattern holes. Two arms
//!   sharing one constructor head are declined by name, because reaching the
//!   second needs an arm body two branches can jump to and the core has no
//!   former for one.

use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use gandr_core_term::boundary::ConstructorTag;
use gandr_core_term::boundary::DataTypeName;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::DataId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::PrimTy;
use gandr_theory_levitation::ValueTypeRef;

use super::COut;
use super::Hoist;
use super::LowerError;
use super::LowerResult;
use super::Lowerer;
use super::VOut;
use super::entry;
use super::named_non_extra_children;
use super::node_kinds;
use super::pattern;
use super::pattern::ArmVerdict;
use super::required_field;
use super::types as ty_lower;
use crate::boundary::ConstructorArity;
use crate::boundary::ConstructorName;
use crate::boundary::MatchDecision;
use crate::boundary::MatchIndex;
use crate::boundary::NodeText;
use crate::boundary::TypeName;
use crate::desc_elab::elaborate_data_descs;
use crate::live_match;
use crate::origin::ElabKind;
use crate::origin::OriginNode;
use crate::synnode::SynNode;

/// One `case` arm, read into its compiled verdict with its body already
/// lowered where the arm has one to run.
///
/// # Contract
/// - ensures: `body` is `Some` exactly for a constructor-headed arm — an
///   indeterminate arm never runs a body and a declined arm has none to run.
struct CaseRow<'tree>
{
    /// What the compiler concluded about the arm's pattern.
    verdict: ArmVerdict<'tree>,
    /// The lowered arm body, for a constructor-headed arm.
    body: Option<COut>,
}

/// A `data D(ā) { … }` declaration's registered shape (declared-data design):
/// its minted core-local nominal id, its usable constructor names, and — the
/// decl table the frozen core deliberately does not carry — its type-parameter
/// names and each constructor's payload [`Code`], in declaration (tag) order.
///
/// The `params` and `ctor_codes` are the pipeline-seam field-discipline
/// substrate (declared-data design Decision 4/5): a constructor application
/// `C(v̄)` under a `D(ā)` ascription checks each payload field against its
/// declared type (`ctor_codes[tag]`) instantiated at `ā` (the parameters
/// `params` substituted by the ascription's arguments), the soundness the
/// core's tag-only `rule_ctor` cannot enforce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataDecl
{
    /// The minted core-local nominal id (serial + name) the runtime `Data`
    /// former compares on.
    pub id: DataId,
    /// The usable constructor names, index = tag (position in the decl table's
    /// `ctors` list).
    pub ctors: Vec<String>,
    /// The datatype's type-parameter names (`a`, `b` in `Either(a, b)`), in
    /// declaration order — the substitution domain for a constructor's field
    /// types at an ascribed instantiation.
    pub params: Vec<String>,
    /// Each constructor's payload [`Code`] (levitation stage-0 design
    /// first-order fragment), index = tag. A constructor of `n` fields
    /// carries a right-nested product of `n` field codes (`1` when
    /// nullary), interpreted to core field types by
    /// [`Lowerer::instantiated_field_types`].
    pub ctor_codes: Vec<Code>,
}

/// Prepared metadata for one declared-data constructor application.
pub(super) struct DataConstructorPlan
{
    /// Core-local nominal datatype identity.
    id: DataId,
    /// Constructor tag in declaration order.
    tag: ConstructorTag,
    /// Instantiated type of each payload field.
    field_types: Vec<ValueType>,
}

impl DataConstructorPlan
{
    /// Returns the payload field types in source order.
    pub(super) fn field_types(&self) -> &[ValueType]
    {
        &self.field_types
    }

    /// Assembles the tagged value after every field has lowered.
    pub(super) fn finish(
        self,
        call_node: SynNode<'_>,
        components: Vec<Value>,
        origins: Vec<OriginNode>,
    ) -> LowerResult<VOut>
    {
        VOut::from_legacy_value(
            &Value::ctor(
                self.id,
                usize::from(self.tag),
                Lowerer::nest_pair(components),
            ),
            OriginNode::new(entry(call_node, None), origins),
        )
    }
}

/// A constructor call recognized before its fields are lowered.
pub(super) struct DataConstructorCall<'tree>
{
    /// Whole call expression.
    pub call_node: SynNode<'tree>,
    /// Constructor name node.
    pub constructor: SynNode<'tree>,
    /// Payload arguments in source order.
    pub arguments: Vec<SynNode<'tree>>,
    /// Ascription arguments when this constructor belongs to the expected
    /// datatype.
    pub expected_args: Option<Vec<ValueType>>,
}

impl Lowerer<'_>
{
    /// Pre-pass: register every `data D(ā) { … }` block into the data registry
    /// (declared-data design Decision 4), mirroring the codata `collect_codata`
    /// pre-pass.
    ///
    /// Uses [`elaborate_data_descs`] over the source — the stage-0 elaborator
    /// that mints the `NominalId` serials — so this is the `NominalId → DataId`
    /// seam. `codata` blocks are skipped (they are the negative half). A
    /// recursive datatype is registered but its constructors are **not** usable
    /// (declared-data design Decision 6): they are left out of the registry, so
    /// a construction or match against them declines as an unknown
    /// constructor.
    ///
    /// # Contract
    /// - ensures: each non-recursive `data` block's constructors are registered
    ///   with their datatype's minted `DataId`, their tag (declaration order),
    ///   their payload [`Code`], and the datatype's parameter names; every
    ///   constructor name joins the constructor-resolution map.
    /// - panics: none.
    pub(super) fn collect_data(&mut self)
    {
        let elab = elaborate_data_descs(self.source);
        for desc in elab.descs {
            if desc.polarity != DeclPolarity::Data {
                continue;
            }
            // Non-recursive only (declared-data design Decision 6): a recursive datatype's
            // constructors are not made usable until the μ⁺ rung.
            if bool::from(desc.is_recursive()) {
                continue;
            }
            let data_name = desc.id.name.to_string();
            // Already seeded from a persisted declaration (an earlier
            // submission): keep its minted id (the `codata` bridge discipline),
            // do not re-mint.
            if self.data.contains_key(&data_name) {
                continue;
            }
            let id = DataId::new(desc.id.serial, &data_name);
            let mut ctor_names = Vec::with_capacity(desc.ctors.len());
            let mut ctor_codes = Vec::with_capacity(desc.ctors.len());
            for (tag, ctor) in desc.ctors.iter().enumerate() {
                let ctor_name = ctor.name.to_string();
                self.constructors
                    .insert(ctor_name.clone(), (data_name.clone(), tag));
                ctor_names.push(ctor_name);
                ctor_codes.push(ctor.code.clone());
            }
            let params = desc
                .params
                .iter()
                .map(|param| param.name.to_string())
                .collect();
            self.data.insert(data_name, DataDecl {
                id,
                ctors: ctor_names,
                params,
                ctor_codes,
            });
        }
    }

    /// Whether `constructor` names a declared data constructor (the injection
    /// dispatch gate deciding a `C(v̄)` call / bare `C` is a declared-data
    /// constructor rather than an out-of-fragment constructor).
    pub(super) fn is_data_constructor(
        &self,
        constructor: ConstructorName<'_>,
    ) -> MatchDecision
    {
        MatchDecision::from(self.constructors.contains_key(constructor.as_ref()))
    }

    /// The core-local `DataId` and tag of a declared constructor, cloned so the
    /// immutable `self.constructors` / `self.data` borrows are released before
    /// a `&mut self` sub-lowering.
    fn resolve_constructor(
        &self,
        constructor: ConstructorName<'_>,
    ) -> Option<(DataId, ConstructorTag)>
    {
        let ctor_entry = self.constructors.get(constructor.as_ref())?;
        let id = {
            let found = self.data.get(&ctor_entry.0)?;
            core::convert::identity(found)
        }
        .id
        .clone();
        Some((id, ctor_entry.1.into()))
    }

    /// Recognizes one declared-data constructor call without lowering fields.
    pub(super) fn data_constructor_call<'tree>(
        &self,
        node: SynNode<'tree>,
        expected: Option<&ValueType>,
    ) -> Option<DataConstructorCall<'tree>>
    {
        if node.kind() != node_kinds::CALL_EXPRESSION {
            return None;
        }
        let function = required_field(node, node_kinds::FIELD_FUNCTION).ok()?;
        if function.kind() != node_kinds::CONSTRUCTOR {
            return None;
        }
        let name = self.text(function).ok()?;
        let (constructor_id, _tag) = self.resolve_constructor(name.into())?;
        let arguments_node = required_field(node, node_kinds::FIELD_ARGUMENTS).ok()?;
        let arguments = named_non_extra_children(arguments_node);
        let expected_args = match expected {
            | Some(&ValueType::Data { ref id, ref args }) if *id == constructor_id => {
                Some(args.iter().map(|arg| (**arg).clone()).collect())
            },
            | Some(&ValueType::Data { .. }) | None => None,
            | Some(_) => return None,
        };
        Some(DataConstructorCall {
            call_node: node,
            constructor: function,
            arguments,
            expected_args,
        })
    }

    /// Resolves constructor identity, arity, and instantiated field types.
    pub(super) fn prepare_data_constructor(
        &self,
        call_node: SynNode<'_>,
        constructor: SynNode<'_>,
        arguments: &[SynNode<'_>],
        expected_args: Option<&[ValueType]>,
    ) -> LowerResult<DataConstructorPlan>
    {
        let name = self.text(constructor).map(NodeText::to_owned)?;
        let Some((id, tag)) = self.resolve_constructor(name.as_str().into())
        else {
            return Err(LowerError::Unsupported {
                kind: constructor.kind(),
                byte_range: constructor.byte_range(),
            });
        };
        let field_types = match expected_args {
            | Some(args) => self.instantiated_field_types(id.name(), tag, args),
            | None => {
                let arity = self.field_arity(id.name(), tag);
                alloc::vec![ValueType::Unknown; usize::from(arity)]
            },
        };
        if arguments.len() != field_types.len() {
            return Err(LowerError::Unsupported {
                kind: call_node.kind(),
                byte_range: call_node.byte_range(),
            });
        }
        Ok(DataConstructorPlan {
            id,
            tag,
            field_types,
        })
    }

    /// Lowers a declared-data constructor application `C(v̄)` to
    /// [`Value::Ctor`] (declared-data design Decision 2). The payload is the
    /// constructor's field-tuple: `()` for a nullary constructor, the
    /// single field value for a one-field constructor, and a right-nested
    /// [`Value::pair`] product for a many-field constructor (declared-data
    /// design's arity growth path; the payload mirror of the declared
    /// [`Code`] product).
    ///
    /// When `expected_args` is `Some(ā)`, the expected-type lowering request
    /// has confirmed this constructor belongs to the enclosing ascription
    /// `D(ā)`; each payload
    /// field is **checked** against its declared field type instantiated at
    /// `ā` (a [`Value::annot`] the core's tag-only `rule_ctor` then
    /// verifies; declared-data design Decision 4/5 field discipline), so
    /// `Some("hi") : Maybe(Integer)` is a clean type error rather than a
    /// silently-accepted payload. `None` lowers the fields unchecked (a
    /// bare / non-ascribed constructor, itself stuck under inference until
    /// an ascription reaches it).
    ///
    /// # Contract
    /// - ensures: for a declared constructor of the datatype's declared arity,
    ///   the term is `Value::Ctor { id, tag, payload }` with the payload the
    ///   field-tuple lowered in value position (computation arguments hoisted);
    ///   with `expected_args`, each field carries its instantiated-field-type
    ///   ascription.
    /// - fails: [`LowerError::Unsupported`] for an unknown constructor or an
    ///   argument count other than the constructor's declared field count.
    /// - panics: none.
    /// # Termination
    /// - reason: nested constructor fields lower only proper syntax
    ///   descendants.
    /// - measure: remaining source nodes beneath constructor arguments.
    /// - boundedness: the parsed CST is finite.
    pub(super) fn data_constructor(
        &mut self,
        call_node: SynNode<'_>,
        constructor: SynNode<'_>,
        arguments: &[SynNode<'_>],
        hoists: &mut Vec<Hoist>,
        expected_args: Option<&[ValueType]>,
    ) -> LowerResult<VOut>
    {
        super::recursive::data_constructor(
            self,
            call_node,
            constructor,
            arguments,
            hoists,
            expected_args,
        )
    }

    /// Right-nests the constructor payload components into the field-tuple the
    /// declared [`Code`] describes: `[]` ↦ `()`, `[v]` ↦ `v`, `[v₀, v₁, v₂]`
    /// ↦ `v₀ × (v₁ × v₂)` (the mirror of [`Code::product_of`]).
    fn nest_pair(components: Vec<Value>) -> Value
    {
        let mut iter = components.into_iter().rev();
        let Some(mut acc) = iter.next()
        else {
            return Value::Unit;
        };
        for value in iter {
            acc = Value::pair(value, acc);
        }
        acc
    }

    /// The declaration view the live pattern analysis reads: every declared
    /// datatype with its constructors, in declaration order, each carrying its
    /// declared field count.
    ///
    /// It is a snapshot taken once at the end of lowering rather than a borrow,
    /// so the analysis is a function of its arguments and can be recomputed
    /// from a checkpoint without re-entering the lowerer.
    pub(super) fn constructor_table(&self) -> live_match::ConstructorTable
    {
        let mut table = live_match::ConstructorTable::new();
        for (name, decl) in &self.data {
            let constructors = decl
                .ctors
                .iter()
                .enumerate()
                .map(|(tag, ctor)| live_match::DeclaredConstructor {
                    name: ctor.clone(),
                    arity: usize::from(
                        self.field_arity(name.as_str().into(), ConstructorTag::from(tag)),
                    ),
                })
                .collect();
            table.declare(TypeName::from(name.as_str()), constructors);
        }
        table
    }

    /// Records one `case` for the live pattern analysis: its lowered scrutinee,
    /// and every arm's pattern translated into the analysis's constraint
    /// language.
    ///
    /// Recording is unconditional and never fails. An arm whose pattern is
    /// outside what the lowerer can compile still contributes a branch here —
    /// which is the point: the analysis describes the match the programmer
    /// wrote, not the subset that happened to lower.
    ///
    /// Nothing here consults the declaration registry. Which datatype the match
    /// eliminates is settled once, at analysis time, against the registry as it
    /// stood when lowering finished — so a `case` written before the `data`
    /// block it matches on is resolved exactly as one written after it.
    pub(super) fn record_match_site(
        &mut self,
        node: SynNode<'_>,
        scrutinee: &Value,
    )
    {
        let branches: Vec<live_match::BranchPattern> = named_non_extra_children(node)
            .into_iter()
            .filter(|arm| arm.kind() == node_kinds::ARM)
            .filter_map(|arm| arm.child_by_field_name(node_kinds::FIELD_PATTERN))
            .map(|pattern| live_match::BranchPattern {
                shape: live_match::translate_pattern(pattern),
                byte_range: pattern.byte_range(),
            })
            .collect();
        // The index is per source item, so a consumer addressing a match by
        // (source item, match) needs nothing but the item's own source order —
        // and nothing about how many core items that source item becomes.
        let match_index = MatchIndex::from(
            self.match_sites
                .iter()
                .filter(|site| site.source_item == self.source_item)
                .count(),
        );
        self.match_sites.push(live_match::MatchSite {
            source_item: self.source_item,
            match_index,
            byte_range: node.byte_range(),
            scrutinee: scrutinee.clone(),
            branches,
        });
    }

    /// The declared field count of constructor `tag` of datatype `data_name`
    /// (its arity) — the length of the flattened payload [`Code`].
    pub(super) fn field_arity(
        &self,
        data_name: DataTypeName<'_>,
        tag: ConstructorTag,
    ) -> ConstructorArity
    {
        self.data
            .get(data_name.as_ref())
            .and_then(|decl| decl.ctor_codes.get(usize::from(tag)))
            .map_or(0, |code| Self::flatten_field_codes(code).len())
            .into()
    }

    /// The declared field types of constructor `tag` of datatype `data_name`,
    /// each instantiated at the ascribed type arguments `args` (the datatype's
    /// parameters substituted). A field the first-order code fragment cannot
    /// resolve to a precise core type becomes [`ValueType::Unknown`] — a
    /// permissive, soundness-preserving gap, never a false mismatch.
    fn instantiated_field_types(
        &self,
        data_name: DataTypeName<'_>,
        tag: ConstructorTag,
        args: &[ValueType],
    ) -> Vec<ValueType>
    {
        let Some(decl) = self.data.get(data_name.as_ref())
        else {
            return Vec::new();
        };
        let Some(code) = decl.ctor_codes.get(usize::from(tag))
        else {
            return Vec::new();
        };
        Self::flatten_field_codes(code)
            .iter()
            .map(|field| self.interpret_field_code(field, &decl.params, args))
            .collect()
    }

    /// Flattens a constructor's payload [`Code`] into its per-field codes,
    /// undoing the right-nesting [`Code::product_of`] built (`1` ↦ `[]`, a
    /// field ↦ `[field]`, `f × (g × h)` ↦ `[f, g, h]`). Iterative
    /// (iterative-traversal design): a right-nested product is walked down
    /// its `right` spine, never recursed. A non-recursive `data` block's
    /// constructor code is `1` / a field / a right-nested product of
    /// fields, so each `left` is one field leaf; any other shape
    /// contributes a single opaque field (interpreted permissively).
    fn flatten_field_codes(code: &Code) -> Vec<Code>
    {
        let mut fields = Vec::new();
        let mut current = code;
        loop {
            match *current {
                | Code::Unit => break,
                | Code::Prod(ref left, ref right) => {
                    fields.push((**left).clone());
                    current = right;
                },
                | ref other => {
                    fields.push(other.clone());
                    break;
                },
            }
        }
        fields
    }

    /// Interprets one field [`Code`] into the core [`ValueType`] the payload
    /// check ascribes, substituting the datatype's `params` by the ascribed
    /// `args`. Only a leaf [`Code::Field`] carries a field type; any other
    /// shape (a stray product / inline sum / atom-binding / recursive `var`) is
    /// out of the leaf-field fragment and interpreted permissively as
    /// [`ValueType::Unknown`].
    fn interpret_field_code(
        &self,
        code: &Code,
        params: &[String],
        args: &[ValueType],
    ) -> ValueType
    {
        match *code {
            | Code::Field(ref ty_ref, ..) => self.interpret_value_type_ref(ty_ref, params, args),
            | _ => ValueType::Unknown,
        }
    }

    /// Interprets a field's [`ValueTypeRef`] into a core [`ValueType`]: a
    /// parameter resolves to its ascribed argument, a primitive to its core
    /// type ([`Self::prim_to_value_type`]), a declared-datatype head to the
    /// [`ValueType::Data`] nominal handle (its arguments interpreted in turn),
    /// a bare non-primitive name to a rigid atom, and any other applied former
    /// (a `List(a)` field, out of this first cut) permissively to
    /// [`ValueType::Unknown`].
    fn interpret_value_type_ref(
        &self,
        ty_ref: &ValueTypeRef,
        params: &[String],
        args: &[ValueType],
    ) -> ValueType
    {
        match *ty_ref {
            | ValueTypeRef::Param(ref name) => params
                .iter()
                .position(|param| param.as_str() == name.as_ref())
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(ValueType::Unknown),
            | ValueTypeRef::Prim(prim) => Self::prim_to_value_type(prim),
            | ValueTypeRef::Ctor {
                ref head,
                args: ref ty_args,
            } => {
                let head = head.as_ref();
                match self.data.get(head) {
                    | Some(decl) => {
                        let interpreted = ty_args
                            .iter()
                            .map(|ty_arg| self.interpret_value_type_ref(ty_arg, params, args))
                            .collect();
                        ValueType::data(decl.id.clone(), interpreted)
                    },
                    // A bare non-declared, non-primitive name is a rigid atom;
                    // an applied non-declared former (`List(a)` in a field) is
                    // out of this first cut, interpreted permissively.
                    | None if ty_args.is_empty() => PrimTy::from_label(NameRef::from(head))
                        .map_or_else(|| ValueType::atom(head), Self::prim_to_value_type),
                    | None => ValueType::Unknown,
                }
            },
        }
    }

    /// Maps a description [`PrimTy`] to the core [`ValueType`] its surface
    /// spelling lowers to, mirroring `types::lower_primitive`: `Unit` ↦ `1`,
    /// `Boolean` ↦ `1 + 1`, `Unknown` ↦ the gradual top, every other primitive
    /// ↦ its rigid atom (so an `Integer` field checks a payload exactly as a `:
    /// Integer` ascription does).
    fn prim_to_value_type(prim: PrimTy) -> ValueType
    {
        match prim {
            | PrimTy::Unit => ValueType::Unit,
            | PrimTy::Boolean => ValueType::sum(ValueType::Unit, ValueType::Unit),
            | PrimTy::Unknown => ValueType::Unknown,
            | other => ValueType::atom(other.label().as_ref()),
        }
    }

    /// Lowers a bare declared-data constructor `C` (a nullary constructor in
    /// expression position, e.g. `None` / `Red`) to [`Value::Ctor`] with the
    /// unit payload (declared-data design). Returns `None` when `node` is not a
    /// declared constructor, so the caller can fall through to the
    /// out-of-fragment decline.
    ///
    /// # Contract
    /// - ensures: `Some(Ctor { id, tag, () })` for a declared **nullary**
    ///   constructor; `None` for a non-constructor / non-declared name, or a
    ///   declared constructor that carries fields (a partial application, out
    ///   of fragment — the caller declines it).
    /// - panics: none.
    pub(super) fn bare_data_constructor(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<Option<VOut>>
    {
        let name = self.text(node)?;
        let Some((id, tag)) = self.resolve_constructor(name.into())
        else {
            return Ok(None);
        };
        // A bare constructor is only a value when it is nullary; a fielded
        // constructor named without its arguments is a partial application, out
        // of the MVP fragment (declined, not silently given a `()` payload).
        if usize::from(self.field_arity(id.name(), tag)) != 0 {
            return Ok(None);
        }
        VOut::from_legacy_value(
            &Value::ctor(id, usize::from(tag), Value::Unit),
            OriginNode::leaf(entry(node, None)),
        )
        .map(Some)
    }

    /// The declared-data resolver the type lowering
    /// ([`ty_lower::lower_ty`] and its kin) threads to rewrite a declared
    /// datatype head to its [`ValueType::Data`] nominal handle at **any** depth
    /// (declared-data design) — `List(Maybe(a))`, `Maybe(a) -> X`, and record
    /// fields all intercept, not only a top-level ascription. A
    /// non-declared head resolves to `None` (a primitive / type variable /
    /// built-in former).
    fn data_resolver(&self) -> impl Fn(&str) -> Option<DataId> + '_
    {
        move |name: &str| self.data.get(name).map(|decl| decl.id.clone())
    }

    /// Lowers a type node with declared-datatype awareness at every depth
    /// (declared-data design): a declared datatype head — top-level or nested —
    /// becomes the [`ValueType::Data`] nominal handle, else the ordinary
    /// [`ty_lower::lower_ty`] result. The ascription / signature sites route
    /// through this so `def x : Maybe(Integer) = …`, `(v : Celsius)`, and a
    /// nested `List(Maybe(a))` all see the nominal type.
    ///
    /// # Contract
    /// - ensures: every declared-datatype reference in `ty_node`, at any depth,
    ///   is the `ValueType::Data` handle; every other node lowers as `lower_ty`
    ///   would.
    /// - panics: none.
    pub(super) fn lower_type_node(
        &self,
        ty_node: SynNode<'_>,
    ) -> LowerResult<Ty>
    {
        let resolve = self.data_resolver();
        ty_lower::lower_ty(self.source, ty_node, self.strictness, &resolve)
    }

    /// Lowers a type node inside a module signature, where the signature's
    /// already-elaborated manifest type components stand ahead of the ambient
    /// environment.
    ///
    /// # Contract
    /// - ensures: a name `manifest` answers expands to that answer at any
    ///   depth; every other name lowers as [`Self::lower_type_node`] lowers it.
    /// - panics: none.
    pub(super) fn lower_type_node_with_manifest(
        &self,
        ty_node: SynNode<'_>,
        manifest: ty_lower::ManifestTypes<'_>,
    ) -> LowerResult<Ty>
    {
        let resolve = self.data_resolver();
        ty_lower::lower_ty_manifest(self.source, ty_node, self.strictness, &resolve, manifest)
    }

    /// The value-sorted counterpart of [`Self::lower_type_node`]: a
    /// declared-datatype reference at any depth becomes the
    /// [`ValueType::Data`] nominal handle, else the ordinary
    /// [`ty_lower::lower_value_ty`] result. Routed from value-position type
    /// sites (a parameter annotation `f(m: Maybe(a))`).
    ///
    /// # Contract
    /// - ensures: every declared-datatype reference in `ty_node`, at any depth,
    ///   is the `ValueType::Data` handle; else the `lower_value_ty` result.
    /// - panics: none.
    pub(super) fn lower_value_type_node(
        &self,
        ty_node: SynNode<'_>,
    ) -> LowerResult<ValueType>
    {
        let resolve = self.data_resolver();
        ty_lower::lower_value_ty(self.source, ty_node, self.strictness, &resolve)
    }

    /// The computation-sorted counterpart of [`Self::lower_type_node`]: a
    /// declared-datatype reference at any depth (a returner `F Maybe(Integer)`,
    /// a `def` result `-> B`) becomes the [`ValueType::Data`] handle inside the
    /// computation type, else the ordinary [`ty_lower::lower_comp_ty`] result.
    ///
    /// # Contract
    /// - ensures: every declared-datatype reference in `ty_node`, at any depth,
    ///   is the `ValueType::Data` handle; else the `lower_comp_ty` result.
    /// - panics: none.
    pub(super) fn lower_comp_type_node(
        &self,
        ty_node: SynNode<'_>,
    ) -> LowerResult<CompType>
    {
        let resolve = self.data_resolver();
        ty_lower::lower_comp_ty(self.source, ty_node, self.strictness, &resolve)
    }

    /// Whether any arm of a `case` matches on a declared data constructor, so
    /// the `case` lowers to [`Comp::DataCase`] rather than the sum / list case.
    /// Syntactic — decidable at lowering against the constructor registry, with
    /// no type information.
    pub(super) fn case_arms_are_data(
        &self,
        node: SynNode<'_>,
    ) -> MatchDecision
    {
        named_non_extra_children(node)
            .into_iter()
            .filter(|arm| arm.kind() == node_kinds::ARM)
            .filter_map(|arm| arm.child_by_field_name(node_kinds::FIELD_PATTERN))
            .filter(|pattern| pattern.kind() == node_kinds::CONSTRUCTOR_PATTERN)
            .filter_map(|pattern| pattern.child_by_field_name(node_kinds::FIELD_CONSTRUCTOR))
            .any(|constructor| {
                self.text(constructor)
                    .is_ok_and(|name| self.is_data_constructor(name.into()).into())
            })
            .into()
    }

    /// Whether a `case` has no arms at all — the absurd match `case v {}` over
    /// an uninhabited datatype (declared-data design). It reveals no
    /// constructor family, so it is only meaningful as the declared-data
    /// eliminator (over an empty `Data` type), routed to
    /// [`Self::data_case`] which produces the arm-less `DataCase`.
    pub(super) fn case_arms_empty(node: SynNode<'_>) -> MatchDecision
    {
        (!named_non_extra_children(node)
            .into_iter()
            .any(|arm| arm.kind() == node_kinds::ARM))
        .into()
    }

    /// Lowers `case v { C₀(x̄₀) => t₀, … }` over declared constructors to
    /// [`Comp::DataCase`] (declared-data design Decision 3), arms placed by
    /// constructor tag.
    ///
    /// The arms are read through [`super::pattern`], which is where the
    /// ordered surface arms become the core's tag-indexed ones. This function
    /// owns only what is specific to the declared-data eliminator: resolving
    /// the one datatype every arm must share, walking the tags, and assembling
    /// the term.
    ///
    /// **The tag walk is the precedence rule.** For each tag the arms are read
    /// in source order and the first one that settles the tag takes it: a
    /// constructor arm naming that tag takes it with its body, and an
    /// indeterminate arm takes it with a stuck hole, because a hole standing
    /// where a test would neither matches nor refutes and so stops every later
    /// arm from being reachable. A tag no arm settles is a missing-arm hole,
    /// exactly as before.
    ///
    /// # Contract
    /// - ensures: on a data-constructor arm set over one datatype, the term is
    ///   `Comp::DataCase(scrut, arms)` with `arms.len()` = the datatype's
    ///   constructor count and each arm at its tag.
    /// - ensures: an arm carrying a pattern hole occupies the tags it shadows
    ///   with a `Comp::Hole` rather than being dropped, so those tags stop
    ///   falling through to the arms written after it.
    /// - fails: [`LowerError`] for a malformed arm, a cross-datatype arm set,
    ///   an unknown constructor, an argument count other than the constructor's
    ///   declared field count, a pattern form outside the compiled set, or
    ///   (strict) a missing constructor.
    /// - panics: none.
    /// # Termination
    /// - reason: each case arm lowers only its proper pattern and body
    ///   descendants.
    /// - measure: remaining source nodes beneath the case expression.
    /// - boundedness: the parsed CST is finite.
    /// - input recursion: none.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the staged hoist buffer is transaction-local: a failed arm drops it with the error (total mode skips the arm, strict mode propagates), so no effect escapes unhandled"
        )
    )]
    pub(super) fn data_case(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let scrut_node = required_field(node, node_kinds::FIELD_VALUE)?;
        let scrut = self.value_expr(scrut_node, &mut hoists)?;
        let scrut_value = scrut.readback_value()?;

        // Record the match for the live pattern analysis before the arms are
        // compiled away. The recording observes; it decides nothing about what
        // follows, and a `case` this analysis has something to say about lowers
        // exactly as one it does not.
        self.record_match_site(node, &scrut_value);

        let mut rows = self.case_rows(node)?;
        let Some(data_name) = rows.iter().find_map(|row| match row.verdict {
            | ArmVerdict::Head { ref data, .. } => Some(data.clone()),
            | ArmVerdict::Indeterminate(_) | ArmVerdict::Declined { .. } => None,
        })
        else {
            return self.data_case_without_constructors(node, &rows, scrut, hoists);
        };
        let ctor_count = usize::from(self.constructor_count(data_name.as_str().into()));

        // The arms in tag order. The `DataCase` origin children are the
        // scrutinee followed by each arm body in tag order.
        let mut arms: Vec<(String, Rc<Comp>)> = Vec::with_capacity(ctor_count);
        let mut arm_origins: Vec<OriginNode> = alloc::vec![scrut.origin];
        for slot in 0 .. ctor_count {
            let tag = ConstructorTag::from(slot);
            let (binder, body) = self.data_case_slot(node, &mut rows, tag)?;
            arms.push((
                binder,
                Rc::new({
                    let readback_comp = body.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ));
            arm_origins.push(body.origin);
        }

        let body = COut::from_legacy_comp(
            &Comp::DataCase(Rc::new(scrut_value), arms),
            OriginNode::new(entry(node, None), arm_origins),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }

    /// Reads every arm of `node` into its compiled verdict, in source order.
    ///
    /// Bodies are lowered here, in source order, and only for the arms that
    /// survive as constructor-headed — the order and the set together are what
    /// keep hole identifiers, and so lowered terms and checkpoints, exactly
    /// where they were for a `case` this compiler did not change the meaning
    /// of.
    ///
    /// # Contract
    /// - ensures: one row per `arm` child, in source order.
    /// - fails: strict mode propagates the first declined arm; total mode
    ///   carries the decline as the row's verdict for the tag walk to skip.
    /// - panics: none.
    fn case_rows<'tree>(
        &mut self,
        node: SynNode<'tree>,
    ) -> LowerResult<Vec<CaseRow<'tree>>>
    {
        let arm_nodes: Vec<SynNode<'tree>> = named_non_extra_children(node)
            .into_iter()
            .filter(|arm| arm.kind() == node_kinds::ARM)
            .collect();
        let mut rows: Vec<CaseRow<'tree>> = Vec::with_capacity(arm_nodes.len());
        let mut data_name: Option<String> = None;
        let mut claimed: BTreeSet<ConstructorTag> = BTreeSet::new();
        for arm_node in arm_nodes {
            let pattern = required_field(arm_node, node_kinds::FIELD_PATTERN)?;
            let plan = pattern::read_pattern(pattern);
            let mut verdict = self.classify_arm(plan)?;
            // The declared field count is the eliminator's own contract, so the
            // arity check belongs here rather than in the classifier.
            if let ArmVerdict::Head {
                ref data,
                tag,
                ref arguments,
                ..
            } = verdict
            {
                let arity = usize::from(self.field_arity(data.as_str().into(), tag));
                if arguments.len() != arity {
                    verdict = ArmVerdict::Declined {
                        kind: pattern.kind(),
                        byte_range: pattern.byte_range(),
                    };
                }
            }
            if let ArmVerdict::Declined { kind, byte_range } = verdict {
                if !bool::from(self.total()) {
                    return Err(ArmVerdict::declined_error(kind, byte_range));
                }
                rows.push(CaseRow {
                    verdict: ArmVerdict::Declined { kind, byte_range },
                    body: None,
                });
                continue;
            }
            let body = match verdict {
                | ArmVerdict::Head { .. } => {
                    let body_node = required_field(arm_node, node_kinds::FIELD_BODY)?;
                    Some(self.comp_expr(body_node)?)
                },
                // An indeterminate arm never runs its body, so the body is not
                // lowered: lowering it would mint hole identifiers and origin
                // entries for a term nothing can reach.
                | ArmVerdict::Indeterminate(_) | ArmVerdict::Declined { .. } => None,
            };
            // The datatype and the tag are settled after the body is lowered,
            // so an arm this eliminator cannot place still consumes the hole
            // identifiers and origin entries its body would have consumed —
            // which is what keeps a program with a stray or repeated arm
            // lowering to exactly the term it lowered to before.
            if let ArmVerdict::Head { ref data, tag, .. } = verdict {
                let owner = data_name.get_or_insert_with(|| data.clone()).clone();
                let stray = *data != owner;
                // Two arms sharing one constructor head are distinguishable
                // only by their arguments, and reaching the second needs an
                // arm body two branches can jump to. The core has no former
                // for that, so the later arm is declined by name rather than
                // compiled into a branch that silently never runs.
                let shadowed = !claimed.insert(tag);
                if stray || shadowed {
                    if !bool::from(self.total()) {
                        return Err(LowerError::Unsupported {
                            kind: arm_node.kind(),
                            byte_range: arm_node.byte_range(),
                        });
                    }
                    verdict = ArmVerdict::Declined {
                        kind: arm_node.kind(),
                        byte_range: arm_node.byte_range(),
                    };
                }
            }
            rows.push(CaseRow { verdict, body });
        }
        Ok(rows)
    }

    /// Settles one constructor tag against the arms, in source order.
    ///
    /// # Contract
    /// - ensures: the first arm that settles `tag` supplies it — a matching
    ///   constructor arm its body, an indeterminate arm a stuck hole — and a
    ///   tag no arm settles takes a missing-arm hole.
    /// - fails: strict mode fails on a duplicate arm for one constructor, a
    ///   cross-datatype arm, or a missing constructor.
    /// - panics: none.
    fn data_case_slot(
        &mut self,
        node: SynNode<'_>,
        rows: &mut [CaseRow<'_>],
        tag: ConstructorTag,
    ) -> LowerResult<(String, COut)>
    {
        for row in rows.iter_mut() {
            let head = match row.verdict {
                | ArmVerdict::Declined { .. } => continue,
                | ArmVerdict::Indeterminate(ref stuck) => {
                    let stuck = Self::stuck_slot(stuck)?;
                    return Ok((node_kinds::DISCARD_BINDER.to_owned(), stuck));
                },
                | ArmVerdict::Head {
                    tag: arm_tag,
                    ref arguments,
                    ref aliases,
                    node: ctor_node,
                    ..
                } => (arm_tag, arguments.clone(), aliases.clone(), ctor_node),
            };
            let (arm_tag, arguments, aliases, ctor_node) = head;
            if arm_tag != tag {
                continue;
            }
            let Some(body) = row.body.take()
            else {
                continue;
            };
            return self.data_case_arm(ctor_node, &arguments, &aliases, body);
        }
        let error = LowerError::MissingCaseArm {
            constructor: node_kinds::DISCARD_BINDER,
            byte_range: node.byte_range(),
        };
        if !bool::from(self.total()) {
            return Err(error);
        }
        self.missing_arm(node)
    }

    /// Compiles one constructor-headed arm into its `DataCase` slot.
    ///
    /// # Contract
    /// - ensures: the payload binder follows the arity discipline, the argument
    ///   patterns' nesting wraps the body outermost-test-first, and the
    ///   as-binders name the scrutinee around the whole arm.
    /// - fails: [`LowerError::Unsupported`] naming an argument form outside the
    ///   compiled set.
    /// - panics: none.
    fn data_case_arm(
        &mut self,
        ctor_node: SynNode<'_>,
        arguments: &[pattern::PatternPlan<'_>],
        aliases: &[(String, SynNode<'_>)],
        body: COut,
    ) -> LowerResult<(String, COut)>
    {
        let (components, nesting) = self.nest_arguments(arguments.to_vec())?;
        let wrapped = self.fold_nesting(nesting, body)?;
        let (payload, mut wrapped) = self.bind_payload(ctor_node, &components, wrapped)?;
        // An as-binder names the scrutinee itself, so it wraps the whole arm
        // rather than any one field; the innermost binder is applied first so
        // source order reads outward.
        for &(ref binder, alias_node) in aliases.iter().rev() {
            wrapped =
                Self::alias_value(alias_node, payload.as_str().into(), binder.clone(), wrapped)?;
        }
        Ok((payload, wrapped))
    }

    /// The `case` with no constructor-headed arm at all: either the absurd
    /// empty match, an arm set every one of whose arms is indeterminate, or an
    /// arm set total-mode lowering dropped entirely.
    ///
    /// # Contract
    /// - ensures: the absurd `case v {}` is the arm-less `DataCase`; an
    ///   indeterminate arm set is one stuck hole; anything else is the total
    ///   fallback hole.
    /// - fails: strict mode fails on an arm set with no compilable arm.
    /// - panics: none.
    fn data_case_without_constructors(
        &mut self,
        node: SynNode<'_>,
        rows: &[CaseRow<'_>],
        scrut: VOut,
        hoists: Vec<Hoist>,
    ) -> LowerResult<COut>
    {
        if Self::case_arms_empty(node).into() {
            // The absurd empty match `case v {}`: an arm-less `DataCase` over
            // the scrutinee's (uninhabited) data type, which the core checks
            // vacuously against the expected answer (declared-data design
            // Decision 3).
            let scrut_value = scrut.readback_value()?;
            let body = COut::from_legacy_comp(
                &Comp::DataCase(Rc::new(scrut_value), Vec::new()),
                OriginNode::new(entry(node, None), alloc::vec![scrut.origin]),
            )?;
            return Self::wrap_hoists(hoists, body, node);
        }
        // An arm set whose first settling arm is indeterminate settles nothing
        // for any scrutinee, so the whole match is stuck on that hole — no
        // datatype has to be resolved to say so.
        let indeterminate = rows.iter().find_map(|row| match row.verdict {
            | ArmVerdict::Indeterminate(ref stuck) => Some(stuck),
            | ArmVerdict::Head { .. } | ArmVerdict::Declined { .. } => None,
        });
        if let Some(stuck) = indeterminate {
            let stuck = Self::stuck_slot(stuck)?;
            return Self::wrap_hoists(hoists, stuck, node);
        }
        // Otherwise every arm dropped in total mode; fall back to a hole so
        // the pipeline stays total.
        let error = LowerError::Unsupported {
            kind: node.kind(),
            byte_range: node.byte_range(),
        };
        if !bool::from(self.total()) {
            return Err(error);
        }
        let hole = self.comp_hole(node, &error)?;
        Self::wrap_hoists(hoists, hole, node)
    }

    /// Wraps `body` in the nested `split`s that positionally destructure a
    /// many-field constructor's right-nested product payload into `binders`,
    /// returning the fresh payload variable the `case` arm binds and the
    /// wrapped body (the [`Comp::DataCase`] β-rule binds the whole product to
    /// that variable; declared-data design arity growth). Iterative
    /// (iterative-traversal design): the split levels are computed over an
    /// index and folded, no host recursion.
    ///
    /// # Contract
    /// - requires: `binders.len() >= 2` (the one/zero-field arms bind
    ///   directly).
    /// - ensures: the returned body evaluates `body` under `binders` bound to
    ///   the payload's positional components (`binders.len() - 1` `split`s).
    /// - panics: none.
    pub(super) fn destructure_payload(
        &mut self,
        arm_node: SynNode<'_>,
        binders: &[String],
        body: COut,
    ) -> LowerResult<(String, COut)>
    {
        let count = binders.len();
        let payload = self.fresh_name();
        let last_binder = count.saturating_sub(1);
        // Each split level `(scrut, fst, snd)`: split `scrut` as `(fst, snd)`.
        // The first scrutinee is the payload; each intermediate `snd` is a
        // fresh temp that scrutinizes the next level; the final level's `snd`
        // is the last field binder (the right end of the right-nested product).
        let mut levels: Vec<(String, String, String)> = Vec::with_capacity(last_binder);
        let mut scrut = payload.clone();
        for (index, fst) in binders.iter().enumerate() {
            if index == last_binder {
                break;
            }
            let snd = if index == last_binder.saturating_sub(1) {
                binders.last().cloned().unwrap_or_default()
            }
            else {
                self.fresh_name()
            };
            levels.push((scrut, fst.clone(), snd.clone()));
            scrut = snd;
        }
        let mut wrapped = body;
        for (scrut_name, fst, snd) in levels.into_iter().rev() {
            wrapped = COut::from_legacy_comp(
                &Comp::split(Value::var(&scrut_name), &fst, &snd, {
                    let readback_comp = wrapped.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                OriginNode::new(entry(arm_node, Some(ElabKind::SplitNest)), alloc::vec![
                    wrapped.origin
                ]),
            )?;
        }
        Ok((payload, wrapped))
    }
}
