//! Built-in precedence-bounded grammar surface for Gandr tree-sitter nodes.

pub mod circuit;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the mutually recursive term-grammar forms have no linear call order"
    )
)]
pub mod term;
pub mod type_shell;

use gandr_theory_graphs::Assoc;
use gandr_theory_graphs::Prec;
use gandr_theory_graphs::PrecCycle;
use gandr_theory_graphs::PrecDag;
use gandr_theory_graphs::PrecSpec;

use crate::Pbg;
use crate::PbgError;
use crate::PrecName;
use crate::PrecTable;

/// Named precedence groups in deterministic dense order.
const PREC_GROUPS: &[(&str, Option<Assoc>)] = &[
    ("item.singleton", None),
    ("expression.atom", None),
    ("expression.postfix", Some(Assoc::Left)),
    ("expression.unary", None),
    ("expression.mul", Some(Assoc::Left)),
    ("expression.add", Some(Assoc::Left)),
    ("expression.cmp", Some(Assoc::Left)),
    ("expression.and", Some(Assoc::Left)),
    ("expression.or", Some(Assoc::Left)),
    ("expression.ret", Some(Assoc::Right)),
    ("pattern.atom", None),
    ("pattern.as", Some(Assoc::Left)),
    ("pattern.or", Some(Assoc::Left)),
    ("type.atom", None),
    ("type.application", None),
    ("type.product", Some(Assoc::Right)),
    ("type.sum", Some(Assoc::Right)),
    ("type.union", Some(Assoc::Right)),
    ("type.intersection", Some(Assoc::Right)),
    ("type.lazy_product", Some(Assoc::Right)),
    ("type.arrow", Some(Assoc::Right)),
];

/// Tighter-to-looser precedence edges by named group.
const PREC_EDGES: &[(&str, &str)] = &[
    ("expression.atom", "expression.postfix"),
    ("expression.postfix", "expression.unary"),
    ("expression.unary", "expression.mul"),
    ("expression.mul", "expression.add"),
    ("expression.add", "expression.cmp"),
    ("expression.cmp", "expression.and"),
    ("expression.and", "expression.or"),
    ("expression.or", "expression.ret"),
    ("pattern.atom", "pattern.as"),
    ("pattern.as", "pattern.or"),
    ("type.atom", "type.application"),
    ("type.application", "type.product"),
    ("type.product", "type.sum"),
    ("type.sum", "type.union"),
    ("type.sum", "type.intersection"),
    ("type.sum", "type.lazy_product"),
    ("type.union", "type.arrow"),
    ("type.intersection", "type.arrow"),
    ("type.lazy_product", "type.arrow"),
];

/// Exact deterministic list of committed tree-sitter named node kinds.
pub const TREE_SITTER_NAMED_KINDS: &[&str] = &[
    "acquire_statement",
    "and_expression",
    "annotation_expression",
    "argument",
    "arguments",
    "arm",
    "as_pattern",
    "at_type",
    "attribute",
    "attribute_block",
    "binary_expression",
    "bind_statement",
    "block",
    "block_comment",
    "boolean",
    "call_expression",
    "case_expression",
    "character",
    "close_expression",
    "co_expression",
    "co_field",
    "command",
    "command_name",
    "command_substitution",
    "constructor",
    "constructor_pattern",
    "def_function",
    "def_signature",
    "def_value",
    "double_quoted_string",
    "drop_expression",
    "dup_expression",
    "end_session_type",
    "environment_assignment",
    "escape_sequence",
    "expression_statement",
    "extern_block",
    "extern_function",
    "extern_type",
    "f_type",
    "file_descriptor",
    "forall_type",
    "force_expression",
    "fork_shared_statement",
    "fork_statement",
    "function_type",
    "grade",
    "hold_expression",
    "hole",
    "hole_name",
    "host_escape",
    "identifier",
    "if_expression",
    "instantiation_expression",
    "intersection_type",
    "lambda_expression",
    "lazy_product_type",
    "let_statement",
    "leta_statement",
    "line_comment",
    "list_expression",
    "list_operator",
    "list_pattern",
    "literal_pattern",
    "migrate_expression",
    "module_declaration",
    "mu_session_type",
    "negation",
    "number",
    "offer_expression",
    "offer_session_type",
    "or_expression",
    "or_pattern",
    "parameter",
    "parameters",
    "parenthesized_expression",
    "parenthesized_type",
    "pipeline",
    "primitive_type",
    "product_type",
    "projection_expression",
    "receive_session_type",
    "record_expression",
    "record_field",
    "record_pattern",
    "record_pattern_field",
    "record_type",
    "record_type_field",
    "record_update_expression",
    "recv_statement",
    "redirection",
    "redirection_operator",
    "release_statement",
    "rest_pattern",
    "ret_expression",
    "select_expression",
    "select_session_type",
    "send_expression",
    "send_session_type",
    "session_field",
    "shebang",
    "shell_block",
    "shell_list",
    "shell_word",
    "single_quoted_string",
    "source_file",
    "string",
    "subshell",
    "sum_type",
    "thunk_expression",
    "tuple_expression",
    "tuple_pattern",
    "type_abstraction",
    "type_application",
    "type_identifier",
    "type_variable",
    "typed_number",
    "u_type",
    "unary_expression",
    "union_type",
    "unit",
    "variable_expansion",
    "variable_name",
    "wildcard",
];

/// PBG-only construct kinds folded in by W4d and W4e.
///
/// These are surface forms — new datatype / control / declaration constructs
/// and the reserved / folded member forms inside them, plus the W4e shell
/// braced parameter form `${name}` — that the committed tree-sitter grammar
/// (`packages/tree-sitter-gandr`) does **not** produce.
/// They are the **parity exemption**: a rule may carry one of these as its
/// `provenance`, and an [`crate::Adaptation`] one as its `surface`, without it
/// being a committed tree-sitter named kind. Because the tree-sitter
/// differential harness ranges over [`TREE_SITTER_NAMED_KINDS`] only, none of
/// these ever appears in [`crate::named_kind_parity`] — parity coverage stays
/// exact for the existing tree-sitter kinds, and the W4d forms are covered by
/// the PBG-only corpus (`crates/surface-corpus/examples/surface/`) and this
/// registry instead. The set is disjoint from [`TREE_SITTER_NAMED_KINDS`].
///
/// Two groups: the new rule provenances (the constructs' own kinds) and the
/// inline / reserved / diverged member surfaces recorded as adaptations.
pub const PBG_ONLY_KINDS: &[&str] = &[
    // construct kinds (new rule provenances)
    "break_expression",
    "codata_declaration",
    "continue_expression",
    "data_declaration",
    "for_expression",
    "import_declaration",
    "circuit_declaration",
    "loop_expression",
    "operator_declaration",
    "rec_block",
    "sign_declaration",
    "while_expression",
    // reserved / folded / diverged member surfaces (adaptation surfaces)
    "bare_type_params",
    "braced_variable_expansion",
    "case_with_view",
    "circuit_body",
    "circuit_member",
    "circuit_signature",
    "codata_observation",
    "constructor_block_member",
    "data_generator",
    "def_rec",
    "feed_statement",
    "grade_prefix",
    "node_statement",
    "op_member",
    "parameterized_observation",
    "rule_member",
    "string_interpolation",
];

/// Build the checked built-in Gandr precedence-bounded grammar.
///
/// # Contract
/// - ensures: returns a PBG whose precedence DAG has incomparable item,
///   expression, pattern, and type bands except for the declared within-band
///   chains.
/// - provides: real structural forms over lexical tiles and recursive sorts;
///   tree-sitter named kinds appear only as rule provenance and in the parity
///   inventory, never as terminal-only coverage rules.
/// - fails: returns [`PbgError`] for precedence construction, rule validation,
///   Operator Form, Unique Tiles, Assumption 3, or duplicate-form violations.
/// - panics: none.
/// - intension: precedence ids are assigned by [`PREC_GROUPS`] order; every
///   committed named kind is covered by term or type-shell structural forms,
///   except the file root (see the parity inventory).
///
/// # Errors
/// Returns [`PbgError`] if the precedence DAG cannot be built or the assembled
/// rules fail checked PBG construction.
#[inline]
pub fn built_in() -> Result<Pbg, PbgError>
{
    let precs = built_in_prec_table()?;
    let mut rules = Vec::new();
    let term_rules = term::rules(&precs)?;
    rules.extend(term_rules);
    let type_shell_rules = type_shell::rules(&precs)?;
    rules.extend(type_shell_rules);
    let circuit_rules = circuit::rules(&precs)?;
    rules.extend(circuit_rules);
    Pbg::build_table(precs, rules)
}

/// Build the built-in precedence table.
///
/// # Contract
/// - ensures: returns the same named precedence graph used by [`built_in`].
/// - provides: a name lookup table for structural surface rule construction.
/// - fails: returns [`PbgError`] if the constant graph is malformed or cyclic.
/// - panics: none.
///
/// # Errors
/// Returns [`PbgError`] if group insertion, edge insertion, or DAG construction
/// fails.
#[inline]
pub fn built_in_prec_table() -> Result<PrecTable, PbgError>
{
    let mut spec = PrecSpec::new();
    for &(name, assoc) in PREC_GROUPS {
        spec.insert(name, assoc).map_err(PbgError::from)?;
    }
    for &(tighter, looser) in PREC_EDGES {
        let tighter_prec = lookup_prec(&spec, PrecName(tighter))?;
        let looser_prec = lookup_prec(&spec, PrecName(looser))?;
        spec.add_edge(tighter_prec, looser_prec)
            .map_err(PbgError::from)?;
    }
    let names = prec_table_names(&spec)?;
    let dag = PrecDag::build(&spec).map_err(|cycle| cycle_error(&spec, cycle))?;
    Ok(PrecTable::new(dag, names))
}

/// Look up a precedence id in a spec while building constants.
fn lookup_prec(
    spec: &PrecSpec,
    name: PrecName,
) -> Result<Prec, PbgError>
{
    for (prec, candidate, _assoc) in spec.groups() {
        if candidate == name.as_ref() {
            return Ok(prec);
        }
    }
    Err(PbgError::MissingPrec { name: name.0 })
}

/// Return static precedence-table names paired with their dense ids.
fn prec_table_names(spec: &PrecSpec) -> Result<Vec<(PrecName, Prec)>, PbgError>
{
    let mut names = Vec::new();
    for &(name, _assoc) in PREC_GROUPS {
        let name = PrecName(name);
        let prec = lookup_prec(spec, name)?;
        names.push((name, prec));
    }
    Ok(names)
}

/// Convert a graph cycle into the grammar error domain with named witnesses.
fn cycle_error(
    spec: &PrecSpec,
    cycle: PrecCycle,
) -> PbgError
{
    let mut witness = Vec::new();
    for prec in cycle.witness {
        witness.push(static_prec_name(spec, prec).0);
    }
    PbgError::PrecedenceCycle { witness }
}

/// Return the static name for a precedence id in the constant spec.
fn static_prec_name(
    spec: &PrecSpec,
    prec: Prec,
) -> PrecName
{
    let Some(name) = spec.name(prec)
    else {
        return PrecName("<invalid-precedence>");
    };
    for &(candidate, _assoc) in PREC_GROUPS {
        if candidate == name {
            return PrecName(candidate);
        }
    }
    PrecName("<unknown-precedence>")
}
