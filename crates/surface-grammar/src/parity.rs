//! Tree-sitter named-kind parity and provenance inventory.
//!
//! Named kinds are not PBG semantics: the PBG's forms range over lexical tiles
//! and recursive sorts only. Every committed tree-sitter named kind is realised
//! either by real structural form(s) (whose rule `provenance` equals the kind
//! name) or, uniquely for the file root, by the top-level Item-sort item
//! sequence. This module is the standalone inventory table consumed by the
//! differential harness, projection, and
//! highlighting; it never defines the tile vocabulary.
//!
//! **Parity exemption (W4d).** The fold-in constructs — `data` /
//! `codata` datatypes, `def rec` + copatterns, `for` / `while` / `loop` /
//! `break` / `continue`, `import`, operator-fixity declarations, and their
//! reserved members — are PBG-only: the committed tree-sitter grammar does not
//! produce them. Their kinds live in [`crate::PBG_ONLY_KINDS`], NOT in
//! [`crate::TREE_SITTER_NAMED_KINDS`], so [`named_kind_parity`] never
//! enumerates them and the tree-sitter differential harness never expects them.
//! Parity coverage therefore stays exact for the existing tree-sitter kinds
//! while the PBG-only forms are covered by their own corpus tree
//! (`crates/surface-corpus/examples/surface/`). A rule realising a PBG-only
//! construct carries its `provenance` from [`crate::PBG_ONLY_KINDS`]; the two
//! registries are disjoint.
//!
//! **Projection re-scope (W5′).** `grammar.js` remains the
//! tree-sitter source; `packages/tree-sitter-gandr/src/grammar.json` is NOT
//! projected from the PBG. The PBG is an abstract structural model: tiles are
//! bare labels with no lexical regexes (some are placeholders no labeler token
//! carries), rules have no tree-sitter field concept (the generated
//! `node-types.json` declares 40 fields, all from `grammar.js` `field(…)`
//! annotations), and the surface is deliberately factored (the `def` / `let` /
//! `(`-atom families) where the committed grammar keeps distinct fielded named
//! nodes — so a faithful projection is not derivable from it. The two parsers
//! are instead kept honest by testing at this boundary: this inventory plus
//! the node-types drift gate (`tests/node_types_gate.rs`, which pins the
//! generated named kinds to [`crate::TREE_SITTER_NAMED_KINDS`], their PBG
//! realisation, and the six lowerer field names) for the kind/field surface,
//! and the differential parity harness for grouping.
//!
//! # The differential equivalence relation
//!
//! Two parsers, one language, forever: the load-bearing engineering risk of the
//! parser-interaction-core epic. The Rust front-end (`gandr_surface_parser`) is
//! **normative**; the committed tree-sitter grammar is a lossy view for
//! third-party editors, checked against the Rust parser — not the reverse.
//!
//! Exact CST isomorphism is too strong (the two grammars group differently).
//! The contract instead checks **two** projections, both grouping-insensitive
//! so a future PBG→`grammar.json` projection (declined this wave — see the
//! re-scope above) could pick any linear extension of the
//! precedence DAG without breaking parity:
//!
//! * **E1 — token stream.** `gandr_surface_parser::label`'s non-trivia token
//!   spans and tree-sitter's non-trivia leaf-token spans coincide, after
//!   mapping each labeler lexeme class through the manifest's
//!   `e1_declared_table`, modulo trivia. Catches all lexical drift. (Witness:
//!   `crates/surface-grammar/tests/token_stream_parity.rs`; the fuzz target
//!   `fuzz/fuzz_targets/parity.rs`.)
//! * **E2 — highlight span.** [`fn@crate::highlight`] and
//!   `gandr_surface_tree_sitter::highlight` produce an identical `Vec<HlSpan>`.
//!   Catches all drift observable to the artifact's only remaining consumer.
//!   (Witness: `crates/surface-grammar/tests/highlight_parity.rs`.)
//!
//! **Why NOT lowering (CST-core) equivalence — rejected.** Comparing the
//! lowered cores would force a parser-generic lowerer: a trait plus dynamic
//! dispatch on every node walk, forever, purely to serve a test. We compare
//! CSTs, not cores, so `surface-engine`'s `lower.rs` stays monomorphic. The two
//! CST projections (E1 lexical, E2 the only surviving consumer's view) bound
//! the drift that matters without that cost.
//!
//! Divergences are permitted only inside shell blocks — where the frozen PBG
//! tokenizes differently (all shell words as `command_name`, the host
//! `(string)` mold for shell strings, split `NAME=value`, command-substitution
//! interiors) — and are enumerated in the contract-fixtures manifest `parity`
//! section. The PBG-only surface corpus is exempt (above). Recovery inputs (an
//! `ERROR`/ `MISSING` tree, or a mold parse with obligations) are not parity
//! seeds: parity is a relation over programs both parsers fully accept.

use crate::surface::TREE_SITTER_NAMED_KINDS;

/// Named kinds realised by the file root rather than a structural form.
const FILE_ROOT_KINDS: &[&str] = &["source_file"];

/// How a committed tree-sitter named kind is realised in the PBG.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the parity contract fixes these two realisation classes"
)]
pub enum NamedKindRealization
{
    /// Generated by real structural rule(s) whose provenance equals the kind
    /// name, over lexical tiles and recursive sorts.
    StructuralForms,
    /// The source file root: the top-level Item-sort item sequence.
    FileRoot,
}

/// One row of the named-kind parity inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "an inventory row is exactly a kind and its realisation class"
)]
pub struct NamedKindEntry
{
    /// The committed tree-sitter named kind.
    pub kind: &'static str,
    /// How the kind is realised in the PBG.
    pub realization: NamedKindRealization,
}

/// Borrowed tree-sitter named-kind text.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed literally in the parity classification witnesses"
)]
pub struct NamedKind<'kind>(pub &'kind str);

/// Classify how a named kind is realised.
///
/// # Contract
/// - requires: `kind` is a committed tree-sitter named kind.
/// - ensures: returns [`NamedKindRealization::FileRoot`] for the file root and
///   [`NamedKindRealization::StructuralForms`] otherwise.
/// - provides: the parity/provenance classification, never a PBG rule.
/// - fails: never; unknown kinds classify as structural forms and are caught by
///   the coverage witness.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the file-root kind and any structural kind kill the class
///   branch.
/// - witness: `gandr_surface_grammar::contracts::named_kind_coverage_is_semantic`
#[inline]
#[must_use]
pub fn named_kind_realization(kind: NamedKind<'_>) -> NamedKindRealization
{
    if FILE_ROOT_KINDS.contains(&kind.0) {
        NamedKindRealization::FileRoot
    }
    else {
        NamedKindRealization::StructuralForms
    }
}

/// Return the full named-kind parity inventory in committed order.
///
/// # Contract
/// - requires: none.
/// - ensures: returns exactly one [`NamedKindEntry`] per committed named kind,
///   in [`TREE_SITTER_NAMED_KINDS`] order.
/// - provides: the standalone inventory table for parity/provenance consumers.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — a deterministic projection over the committed kind list;
///   exactness is witnessed by the semantic coverage test.
/// - witness: `gandr_surface_grammar::contracts::named_kind_coverage_is_semantic`
#[inline]
#[must_use]
pub fn named_kind_parity() -> Vec<NamedKindEntry>
{
    TREE_SITTER_NAMED_KINDS
        .iter()
        .map(|kind| NamedKindEntry {
            kind,
            realization: named_kind_realization(NamedKind(kind)),
        })
        .collect()
}
