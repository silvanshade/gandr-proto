//! The entity-attribute layer (proposal-attributes.md; ADR "entity
//! attributes"; epic `wyrd-5sit`, MVP bead `wyrd-f0a9`).
//!
//! An **entity attribute** is a named, typed datum attached to a declaration
//! entity through the leading `@[…]` marker ([`crate::lower`], grammar §2).
//! This module owns the three landed disciplines the layer composes:
//!
//! 1. a **schema registry** ([`REGISTRY`]) binding attribute names to value
//!    types, the way ADR-42's prelude binds native builtins to their types;
//! 2. a **checker path** that types each payload against its schema with the
//!    ordinary bidirectional checker — driven through the **typing machine**
//!    (iterative, ADR-47), never the recursive checker, so the pass is robust
//!    on generated input exactly as [`crate::goals`] / [`crate::diag`] are;
//! 3. the **inert side table** — [`run`] resolves every raw attribute into a
//!    [`ResolvedAttr`] keyed by its item's stable id, plus one [`AttrFinding`]
//!    per malformed attribute for the diagnostics surface.
//!
//! # Storage identity — the stable-`NodeId` stand-in
//!
//! The proposal keys the side table by the entity's **stable `NodeId`**
//! (ADR-50, commit `8bfccbe`). The pipeline's item identity today is the item's
//! **index** in [`Lowered::items`] — the same key the `Report`'s goal,
//! diagnostic, and mark projections already localize by (`item : usize`). This
//! module keys on that index ([`ResolvedAttr::node`]); the arena-of-`NodeId`
//! graduation is a lossless re-key, not a redesign (the ECS-attachment path,
//! proposal §4.3).
//!
//! # Hash-neutrality (the `wyrd-q5r0` invariant, inert MVP)
//!
//! Every MVP schema is [`AttrTier::Inert`]: an attribute lives **only** in this
//! side table and never enters an item's core-IR term ([`Lowered::items`] is
//! untouched by this pass), so an item's content-address — computed over its
//! core-IR syntax — is unchanged by adding, removing, or editing an attribute.
//! The semantic tier (proposal §4.2), which reflects an attribute into the core
//! IR and so participates in identity, is reserved and unbuilt.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::control::Dir;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::machine;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::ValueType;

use crate::boundary::AttributeName;
use crate::boundary::EditDistance;
use crate::boundary::SourceRange;
use crate::lower::Lowered;

/// An attribute's **identity tier** (proposal-attributes.md §4.2): whether it
/// changes what the entity *is*.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "codecs", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum AttrTier
{
    /// Inert metadata that never enters the content-address (the default and
    /// the whole MVP): a doc string, a deprecation note, a manifest coordinate.
    Inert,
    /// A transformation of the entity, reflected into the core IR and so
    /// hash-participating (deriving, operator fixity, FFI gates). Reserved,
    /// unbuilt; no MVP schema is semantic.
    Semantic,
}

/// An attribute schema's **arity** (proposal-attributes.md §3.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttrArity
{
    /// At most one occurrence per entity; a repeat is a
    /// [`AttrFinding::Duplicate`].
    Single,
    /// Any number of occurrences per entity (e.g. one `dependency` per
    /// dependency of a unit).
    Repeatable,
}

/// One built-in attribute schema: a name bound to a value-type schema, an
/// identity tier, and an arity (proposal-attributes.md §3.1).
///
/// The schema type is a function pointer rather than a stored [`ValueType`] so
/// the [`REGISTRY`] is a plain `const` — [`ValueType`] carries `Rc`/`String`
/// and is not itself const-constructible.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct AttrSchema
{
    /// The attribute name the marker writes (`doc`, `deprecated`, …).
    pub name: &'static str,
    /// The identity tier (every MVP schema is [`AttrTier::Inert`]).
    pub tier: AttrTier,
    /// The arity (single-valued or repeatable).
    pub arity: AttrArity,
    /// Builds the schema's value type — the type the payload is checked
    /// against.
    schema: fn() -> ValueType,
}

impl AttrSchema
{
    /// The schema's value type (the type a payload checks against).
    ///
    /// # Contract
    /// - ensures: returns the [`ValueType`] this schema types payloads against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn schema_type(&self) -> ValueType
    {
        (self.schema)()
    }
}

/// The `deprecated` schema `#{ since : String, note : String }`
/// (proposal-attributes.md §3.1).
fn schema_deprecated() -> ValueType
{
    ValueType::record([
        ("note".to_owned(), ValueType::string()),
        ("since".to_owned(), ValueType::string()),
    ])
}

/// The `package` manifest schema `#{ name : String, version : String }`
/// (proposal-packages.md §7.2–§7.3).
fn schema_package() -> ValueType
{
    ValueType::record([
        ("name".to_owned(), ValueType::string()),
        ("version".to_owned(), ValueType::string()),
    ])
}

/// The `dependency` manifest schema
/// `#{ name : String, alias : String, constraint : String }` — a repeatable
/// coordinate, one per dependency (proposal-packages.md §7.3).
fn schema_dependency() -> ValueType
{
    ValueType::record([
        ("alias".to_owned(), ValueType::string()),
        ("constraint".to_owned(), ValueType::string()),
        ("name".to_owned(), ValueType::string()),
    ])
}

/// The `toolchain` manifest schema `#{ gandr : String }`
/// (proposal-packages.md §7.2–§7.3).
fn schema_toolchain() -> ValueType
{
    ValueType::record([("gandr".to_owned(), ValueType::string())])
}

/// The `authors` manifest schema `[String]` (proposal-packages.md §7.3).
fn schema_authors() -> ValueType
{
    ValueType::list(ValueType::string())
}

/// The MVP built-in attribute registry — every entry is inert.
///
/// Proposal-attributes.md §3.1 and proposal-packages.md §7: the entity and
/// manifest schemas share this one registry, the ADR-42 prelude-binding
/// substrate the attribute names resolve through. The manifest schemas
/// (`package` / `dependency` / `toolchain` / `name` / `license` / `authors`,
/// bead `wyrd-c7bx`) are the **inert descriptive + coordinate** fields of
/// proposal-packages.md §7.6's MVP column; the identity-bearing fields (exposed
/// signature, required capabilities) gate on the unbuilt semantic tier, and the
/// `resolved` dependency-address participation gates on resolution.
///
/// The manifest's intended host is the `wyrd-wpa0` M1-lite **module root**,
/// which is not landed; until it is, a manifest schema validates on a top-level
/// `def` item (the unit-root stand-in the grammar provides today). The schemas
/// and their checker path are therefore complete; the module-root attachment
/// and the local lock record are the reported `wyrd-wpa0` gap.
pub const REGISTRY: &[AttrSchema] = &[
    AttrSchema {
        name: "doc",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: ValueType::string,
    },
    AttrSchema {
        name: "deprecated",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: schema_deprecated,
    },
    // --- Manifest schemas (proposal-packages.md §7; bead `wyrd-c7bx`) ---------
    AttrSchema {
        name: "name",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: ValueType::string,
    },
    AttrSchema {
        name: "license",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: ValueType::string,
    },
    AttrSchema {
        name: "authors",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: schema_authors,
    },
    AttrSchema {
        name: "package",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: schema_package,
    },
    AttrSchema {
        name: "dependency",
        tier: AttrTier::Inert,
        arity: AttrArity::Repeatable,
        schema: schema_dependency,
    },
    AttrSchema {
        name: "toolchain",
        tier: AttrTier::Inert,
        arity: AttrArity::Single,
        schema: schema_toolchain,
    },
];

/// The maximum edit distance a [`AttrFinding::Unknown`] did-you-mean suggestion
/// tolerates (proposal-attributes.md §3.2 — the ADR-42 unknown-member story).
const MAX_SUGGESTION_DISTANCE: usize = 3;

/// Resolves and types the entity attributes of a lowered file against `base`
/// (proposal-attributes.md §3–§5).
///
/// Each raw attribute resolves against the [`REGISTRY`]; an unknown name, a
/// single-valued duplicate, a missing payload, a non-value payload, or a
/// payload that fails to type is a [`AttrFinding`]; every remaining attribute
/// is a [`ResolvedAttr`] carrying its checked payload. Payload typing runs on
/// the iterative typing machine (ADR-47), so this pass never recurses on the
/// host stack.
///
/// # Contract
/// - requires: `base` is the typing context the file was lowered against (e.g.
///   [`crate::prelude_ctx`] plus session definitions), so payloads type in the
///   same context the items do.
/// - ensures: returns one [`ResolvedAttr`] per well-formed attribute (known
///   schema, non-duplicate, value-fragment payload — including a payload that
///   fails to *type*, so a renderer still sees the attachment) and one
///   [`AttrFinding`] per malformed attribute, both in source order.
/// - provides: the inert side table (§4.1) and the diagnostics inputs (§3.2);
///   the items' core-IR terms are never read or mutated (hash-neutral, §4.2).
/// - panics: none.
#[inline]
#[must_use]
pub fn run(
    lowered: &Lowered,
    base: &Ctx,
) -> AttrPass
{
    let mut resolved: Vec<ResolvedAttr> = Vec::new();
    let mut findings: Vec<AttrFinding> = Vec::new();
    // Single-arity duplicate tracking, keyed by (item id, schema name).
    let mut seen: BTreeSet<(usize, &'static str)> = BTreeSet::new();
    for raw in &lowered.attributes {
        let Some(schema) = lookup(&raw.name)
        else {
            findings.push(AttrFinding::Unknown {
                name: raw.name.clone(),
                suggestion: suggest(&raw.name),
                span: raw.name_range.clone(),
            });
            continue;
        };
        if matches!(schema.arity, AttrArity::Single) && !seen.insert((raw.item, schema.name)) {
            findings.push(AttrFinding::Duplicate {
                name: raw.name.clone(),
                span: raw.name_range.clone(),
            });
            continue;
        }
        let Some(payload) = raw.payload.as_ref()
        else {
            findings.push(AttrFinding::MissingPayload {
                name: raw.name.clone(),
                span: raw.name_range.clone(),
            });
            continue;
        };
        if !payload.is_value_fragment {
            findings.push(AttrFinding::NonValuePayload {
                name: raw.name.clone(),
                span: payload.range.clone(),
            });
            continue;
        }
        if let Some(error) = check_payload(base, &payload.value, &schema.schema_type()) {
            findings.push(AttrFinding::IllTypedPayload {
                name: raw.name.clone(),
                error,
                span: payload.range.clone(),
            });
        }
        // Project the attachment even when the payload is ill-typed: the value
        // is present and a renderer reads it alongside the diagnostic (the
        // renderer-firewall model, §5).
        resolved.push(ResolvedAttr {
            node: raw.item,
            schema: schema.name.to_owned(),
            payload: payload.value.clone(),
            tier: schema.tier,
            span: raw.block_range.clone(),
        });
    }
    AttrPass { resolved, findings }
}
/// Looks up a schema by attribute name.
///
/// # Contract
/// - ensures: returns the registry entry whose name equals `name`, or [`None`].
/// - panics: none.
#[inline]
#[must_use]
pub fn lookup<'name, N: Into<AttributeName<'name>>>(name: N) -> Option<&'static AttrSchema>
{
    let name = name.into();
    REGISTRY.iter().find(|schema| schema.name == name.0)
}

/// One resolved, projected attribute: an entry of the inert side table
/// (proposal-attributes.md §4.1) and the source of one `Report.attributes` row
/// (§5).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ResolvedAttr
{
    /// The annotated item's stable id (its index in [`Lowered::items`]; see the
    /// module doc's storage-identity note).
    pub node: usize,
    /// The resolved schema's name (the `SchemaRef`).
    pub schema: String,
    /// The checked value-fragment payload.
    pub payload: Value,
    /// The schema's identity tier (inert for every MVP schema).
    pub tier: AttrTier,
    /// The `@[…]` block's byte range (the projection span).
    pub span: SourceRange,
}

/// One malformed attribute, for the diagnostics surface (proposal-attributes.md
/// §3.2). [`crate::diag`] maps each into a source-ranged `Diagnostic`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttrFinding
{
    /// The name resolves to no registry entry, with a did-you-mean over the
    /// registry (`UnknownAttribute`).
    Unknown
    {
        /// The unresolved attribute name.
        name: String,
        /// The nearest registry name within [`MAX_SUGGESTION_DISTANCE`], if
        /// any.
        suggestion: Option<String>,
        /// The name token's byte range.
        span: SourceRange,
    },
    /// A single-valued attribute repeated on one entity (`DuplicateAttribute`).
    Duplicate
    {
        /// The duplicated attribute name.
        name: String,
        /// The repeat's name-token byte range.
        span: SourceRange,
    },
    /// A schema that requires a payload was written as a bare marker.
    MissingPayload
    {
        /// The attribute name.
        name: String,
        /// The name token's byte range.
        span: SourceRange,
    },
    /// The payload is not in the value fragment — a computation payload
    /// (proposal-attributes.md §3.3, "attribute purity is locality": a payload
    /// cannot be an `F`-computation).
    NonValuePayload
    {
        /// The attribute name.
        name: String,
        /// The payload's byte range.
        span: SourceRange,
    },
    /// The payload fails to check against the schema — the ordinary type error
    /// of the record/scalar/list rules, surfaced at the payload node.
    IllTypedPayload
    {
        /// The attribute name.
        name: String,
        /// The checker's first type error.
        error: TypeError,
        /// The payload's byte range.
        span: SourceRange,
    },
}

/// The attribute pass's output: the resolved side table plus the findings for
/// the diagnostics surface.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AttrPass
{
    /// The resolved attributes, in source order (the inert side table and the
    /// `Report.attributes` projection source).
    pub resolved: Vec<ResolvedAttr>,
    /// One finding per malformed attribute, in source order.
    pub findings: Vec<AttrFinding>,
}

/// The nearest registry name to `name` within [`MAX_SUGGESTION_DISTANCE`], for
/// the `UnknownAttribute` did-you-mean.
fn suggest<'name>(name: impl Into<AttributeName<'name>>) -> Option<String>
{
    let name = name.into();
    REGISTRY
        .iter()
        .map(|schema| (levenshtein(name, schema.name).into().0, schema.name))
        .filter(|&(distance, _)| distance <= MAX_SUGGESTION_DISTANCE)
        .min_by_key(|&(distance, _)| distance)
        .map(|(_distance, candidate)| candidate.to_owned())
}

/// Types one payload value against its schema on the iterative typing machine
/// (ADR-47), returning the first [`TypeError`] or [`None`] when it checks.
///
/// This is the "no new typing rule" path (proposal-attributes.md §3.1): a
/// payload is a value, and the record/scalar/list rules already type it —
/// driven through the machine exactly as [`crate::diag`] drives items, so spans
/// and error shapes match the rest of the surface.
fn check_payload(
    base: &Ctx,
    value: &Value,
    schema: &ValueType,
) -> Option<TypeError>
{
    let mut state =
        machine::State::new_value(base.clone(), value.clone(), Dir::Check(schema.clone()));
    loop {
        match machine::step(state) {
            | machine::Outcome::Step(next) => state = next,
            | machine::Outcome::Error { error, .. } => return Some(error),
            // `Done`, and any future non-error terminal outcome (`Outcome` is
            // non-exhaustive upstream), mean the payload checks.
            | _ => return None,
        }
    }
}

/// The Levenshtein edit distance between two strings — a total, allocation-only
/// two-row dynamic program (no indexing, saturating arithmetic).
fn levenshtein<'lhs, 'rhs>(
    lhs: impl Into<AttributeName<'lhs>>,
    rhs: impl Into<AttributeName<'rhs>>,
) -> impl Into<EditDistance>
{
    let lhs = lhs.into();
    let rhs = rhs.into();
    let rhs_chars: Vec<char> = rhs.chars().collect();
    // `prev[j]` is the distance between the empty prefix of `lhs` and the
    // `j`-length prefix of `rhs`.
    let mut prev: Vec<usize> = (0 ..= rhs_chars.len()).collect();
    for (row, lhs_char) in lhs.chars().enumerate() {
        let mut curr: Vec<usize> = Vec::with_capacity(prev.len());
        // The first column: deleting `row + 1` characters of `lhs`.
        curr.push(row.saturating_add(1));
        for (col, rhs_char) in rhs_chars.iter().enumerate() {
            let up = prev
                .get(col.saturating_add(1))
                .copied()
                .unwrap_or(usize::MAX);
            let left = curr.get(col).copied().unwrap_or(usize::MAX);
            let diagonal = prev.get(col).copied().unwrap_or(usize::MAX);
            let substitution = usize::from(lhs_char != *rhs_char);
            curr.push(
                up.saturating_add(1)
                    .min(left.saturating_add(1))
                    .min(diagonal.saturating_add(substitution)),
            );
        }
        prev = curr;
    }
    EditDistance(prev.last().copied().unwrap_or(lhs.len()))
}
