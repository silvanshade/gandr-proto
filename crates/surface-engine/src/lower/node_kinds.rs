//! The single node-kind table (`A2-PLAN.md` §A2.1 scope item 3, §4.2).
//!
//! The lowerer keys on **node kinds**, never keyword spellings, and every
//! kind string it dispatches on lives here — mirroring the grammar's `KW`
//! table discipline, so the pending keyword bikeshed is a one-table diff in
//! this crate too. Kinds are `tree-sitter-gandr` node kinds
//! (`node-types.json`); anonymous-token kinds
//! (operators, `true`/`false`, `ω`) are node kinds in the same sense.
//!
//! A final section centralizes the few *surface names* (not node kinds) the
//! lowerer must compare: the `Inl`/`Inr` constructors and the `fst`/`snd`
//! lazy-product field names.

use crate::boundary::SyntaxField;
use crate::boundary::SyntaxKind;

// --- Structure ---------------------------------------------------------------

/// The root node of a parse.
pub const SOURCE_FILE: SyntaxKind = SyntaxKind("source_file");
pub const DEF_SIGNATURE: SyntaxKind = SyntaxKind("def_signature");
pub const DEF_VALUE: SyntaxKind = SyntaxKind("def_value");
pub const DEF_FUNCTION: SyntaxKind = SyntaxKind("def_function");
pub const DEF_REC: SyntaxKind = SyntaxKind("def_rec");
/// An open mutually-recursive declaration group `rec { … }`.
pub const REC_BLOCK: SyntaxKind = SyntaxKind("rec_block");
pub const CODATA_DECLARATION: SyntaxKind = SyntaxKind("codata_declaration");
pub const CODATA_OBSERVATION: SyntaxKind = SyntaxKind("codata_observation");
pub const COPATTERN_CLAUSE: SyntaxKind = SyntaxKind("copattern_clause");
pub const EXTERN_BLOCK: SyntaxKind = SyntaxKind("extern_block");
pub const EXTERN_TYPE: SyntaxKind = SyntaxKind("extern_type");
pub const EXTERN_FUNCTION: SyntaxKind = SyntaxKind("extern_function");
pub const DATA_DECLARATION: SyntaxKind = SyntaxKind("data_declaration");
pub const MODULE_DECLARATION: SyntaxKind = SyntaxKind("module_declaration");
pub const FIELD_ASCRIPTION: SyntaxField = SyntaxField("ascription");
pub const FIELD_MEMBER: SyntaxField = SyntaxField("member");
pub const ATTRIBUTE_BLOCK: SyntaxKind = SyntaxKind("attribute_block");
pub const ATTRIBUTE: SyntaxKind = SyntaxKind("attribute");
pub const FIELD_ATTRIBUTE: SyntaxField = SyntaxField("attribute");
pub const FIELD_PAYLOAD: SyntaxField = SyntaxField("payload");
pub const FIELD_NAME: SyntaxField = SyntaxField("name");
pub const FIELD_ARGUMENT: SyntaxField = SyntaxField("argument");
/// The erased residents carried by an instantiation slot.
pub const FIELD_INSTANTIATION: SyntaxField = SyntaxField("instantiation");
/// The expression modified by an instantiation slot.
pub const FIELD_TARGET: SyntaxField = SyntaxField("target");
pub const FIELD_ARGUMENTS: SyntaxField = SyntaxField("arguments");
pub const FIELD_CLAUSE: SyntaxField = SyntaxField("clause");
pub const FIELD_PATTERN: SyntaxField = SyntaxField("pattern");
pub const FIELD_OPERAND: SyntaxField = SyntaxField("operand");
pub const FIELD_PARAMETERS: SyntaxField = SyntaxField("parameters");
pub const FIELD_PARAMETER: SyntaxField = SyntaxField("parameter");
pub const FIELD_BODY: SyntaxField = SyntaxField("body");
pub const FIELD_CONSEQUENCE: SyntaxField = SyntaxField("consequence");
pub const FIELD_ALTERNATIVE: SyntaxField = SyntaxField("alternative");
pub const FIELD_RESULT: SyntaxField = SyntaxField("result");
pub const FIELD_TYPE: SyntaxField = SyntaxField("type");
pub const FIELD_VALUE: SyntaxField = SyntaxField("value");
pub const FIELD_SOURCE: SyntaxField = SyntaxField("source");
pub const FIELD_FIELD: SyntaxField = SyntaxField("field");
pub const FIELD_OBSERVATION: SyntaxField = SyntaxField("observation");
pub const FIELD_EXPRESSION: SyntaxField = SyntaxField("expression");
pub const FIELD_ABI: SyntaxField = SyntaxField("abi");
pub const FIELD_LIBRARY: SyntaxField = SyntaxField("library");
pub const FIELD_GRADE: SyntaxField = SyntaxField("grade");
pub const FIELD_FUNCTION: SyntaxField = SyntaxField("function");
pub const FIELD_CONSTRUCTOR: SyntaxField = SyntaxField("constructor");
pub const FIELD_LEFT: SyntaxField = SyntaxField("left");
pub const FIELD_RIGHT: SyntaxField = SyntaxField("right");
pub const FIELD_CONDITION: SyntaxField = SyntaxField("condition");
pub const LET_STATEMENT: SyntaxKind = SyntaxKind("let_statement");
pub const BIND_STATEMENT: SyntaxKind = SyntaxKind("bind_statement");
pub const EXPRESSION_STATEMENT: SyntaxKind = SyntaxKind("expression_statement");
pub const LETA_STATEMENT: SyntaxKind = SyntaxKind("leta_statement");
pub const RECV_STATEMENT: SyntaxKind = SyntaxKind("recv_statement");
pub const ACQUIRE_STATEMENT: SyntaxKind = SyntaxKind("acquire_statement");
pub const RELEASE_STATEMENT: SyntaxKind = SyntaxKind("release_statement");
pub const FORK_STATEMENT: SyntaxKind = SyntaxKind("fork_statement");
pub const FORK_SHARED_STATEMENT: SyntaxKind = SyntaxKind("fork_shared_statement");
pub const UNSUPPORTED_STATEMENTS: [SyntaxKind; 6] = [
    LETA_STATEMENT,
    RECV_STATEMENT,
    ACQUIRE_STATEMENT,
    RELEASE_STATEMENT,
    FORK_STATEMENT,
    FORK_SHARED_STATEMENT,
];

// --- Patterns --------------------------------------------------------------

/// A tuple pattern `(p, q, …)`.
pub const TUPLE_PATTERN: SyntaxKind = SyntaxKind("tuple_pattern");
pub const CONSTRUCTOR_PATTERN: SyntaxKind = SyntaxKind("constructor_pattern");
pub const WILDCARD: SyntaxKind = SyntaxKind("wildcard");
pub const IDENTIFIER: SyntaxKind = SyntaxKind("identifier");
pub const TYPE_VARIABLE: SyntaxKind = SyntaxKind("type_variable");
pub const CONSTRUCTOR: SyntaxKind = SyntaxKind("constructor");
pub const NUMBER: SyntaxKind = SyntaxKind("number");
pub const TYPED_NUMBER: SyntaxKind = SyntaxKind("typed_number");
pub const STRING: SyntaxKind = SyntaxKind("string");
pub const BOOLEAN: SyntaxKind = SyntaxKind("boolean");
pub const TRUE: SyntaxKind = SyntaxKind("true");
pub const FALSE: SyntaxKind = SyntaxKind("false");
pub const UNIT: SyntaxKind = SyntaxKind("unit");
pub const HOLE: SyntaxKind = SyntaxKind("hole");
pub const FIELD_HOLE_NAME: SyntaxField = SyntaxField("name");
pub const PARENTHESIZED_EXPRESSION: SyntaxKind = SyntaxKind("parenthesized_expression");
pub const TUPLE_EXPRESSION: SyntaxKind = SyntaxKind("tuple_expression");
pub const LIST_EXPRESSION: SyntaxKind = SyntaxKind("list_expression");
pub const ANNOTATION_EXPRESSION: SyntaxKind = SyntaxKind("annotation_expression");
pub const BLOCK: SyntaxKind = SyntaxKind("block");
pub const LAMBDA_EXPRESSION: SyntaxKind = SyntaxKind("lambda_expression");
pub const THUNK_EXPRESSION: SyntaxKind = SyntaxKind("thunk_expression");
pub const CASE_EXPRESSION: SyntaxKind = SyntaxKind("case_expression");
pub const ARM: SyntaxKind = SyntaxKind("arm");
pub const IF_EXPRESSION: SyntaxKind = SyntaxKind("if_expression");
pub const CO_EXPRESSION: SyntaxKind = SyntaxKind("co_expression");
pub const CO_FIELD: SyntaxKind = SyntaxKind("co_field");
pub const RECORD_EXPRESSION: SyntaxKind = SyntaxKind("record_expression");
pub const RECORD_UPDATE_EXPRESSION: SyntaxKind = SyntaxKind("record_update_expression");
pub const FIELD_BASE: SyntaxField = SyntaxField("base");
pub const RECORD_FIELD: SyntaxKind = SyntaxKind("record_field");
pub const PROJECTION_EXPRESSION: SyntaxKind = SyntaxKind("projection_expression");
pub const CALL_EXPRESSION: SyntaxKind = SyntaxKind("call_expression");
/// A postfix erased-instantiation slot `e[ι₁, …, ιₙ]`.
pub const INSTANTIATION_EXPRESSION: SyntaxKind = SyntaxKind("instantiation_expression");
/// One comma-delimited resident of an instantiation slot.
pub const INSTANTIATION_RESIDENT: SyntaxKind = SyntaxKind("instantiation_resident");
pub const FORCE_EXPRESSION: SyntaxKind = SyntaxKind("force_expression");
pub const RET_EXPRESSION: SyntaxKind = SyntaxKind("ret_expression");
pub const BINARY_EXPRESSION: SyntaxKind = SyntaxKind("binary_expression");
pub const UNARY_EXPRESSION: SyntaxKind = SyntaxKind("unary_expression");
pub const SHELL_BLOCK: SyntaxKind = SyntaxKind("shell_block");
pub const SHELL_LIST: SyntaxKind = SyntaxKind("shell_list");
pub const LIST_OPERATOR: SyntaxKind = SyntaxKind("list_operator");
pub const COMMAND: SyntaxKind = SyntaxKind("command");
pub const COMMAND_NAME: SyntaxKind = SyntaxKind("command_name");
pub const ARGUMENT: SyntaxKind = SyntaxKind("argument");
pub const SHELL_WORD: SyntaxKind = SyntaxKind("shell_word");
pub const SINGLE_QUOTED_STRING: SyntaxKind = SyntaxKind("single_quoted_string");
pub const DOUBLE_QUOTED_STRING: SyntaxKind = SyntaxKind("double_quoted_string");
pub const REDIRECTION: SyntaxKind = SyntaxKind("redirection");
pub const VARIABLE_EXPANSION: SyntaxKind = SyntaxKind("variable_expansion");
pub const COMMAND_SUBSTITUTION: SyntaxKind = SyntaxKind("command_substitution");
pub const HOST_ESCAPE: SyntaxKind = SyntaxKind("host_escape");
pub const SUBSHELL: SyntaxKind = SyntaxKind("subshell");
pub const PIPELINE: SyntaxKind = SyntaxKind("pipeline");
pub const AND_EXPRESSION: SyntaxKind = SyntaxKind("and_expression");
pub const OR_EXPRESSION: SyntaxKind = SyntaxKind("or_expression");
pub const ENVIRONMENT_ASSIGNMENT: SyntaxKind = SyntaxKind("environment_assignment");
pub const NEGATION: SyntaxKind = SyntaxKind("negation");
pub const PARAMETER: SyntaxKind = SyntaxKind("parameter");
pub const PRIMITIVE_TYPE: SyntaxKind = SyntaxKind("primitive_type");
pub const TYPE_IDENTIFIER: SyntaxKind = SyntaxKind("type_identifier");
pub const TYPE_APPLICATION: SyntaxKind = SyntaxKind("type_application");
pub const RECORD_TYPE: SyntaxKind = SyntaxKind("record_type");
pub const RECORD_TYPE_FIELD: SyntaxKind = SyntaxKind("record_type_field");
pub const F_TYPE: SyntaxKind = SyntaxKind("f_type");
pub const U_TYPE: SyntaxKind = SyntaxKind("u_type");
pub const FUNCTION_TYPE: SyntaxKind = SyntaxKind("function_type");
pub const PRODUCT_TYPE: SyntaxKind = SyntaxKind("product_type");
pub const SUM_TYPE: SyntaxKind = SyntaxKind("sum_type");
pub const LAZY_PRODUCT_TYPE: SyntaxKind = SyntaxKind("lazy_product_type");
pub const PARENTHESIZED_TYPE: SyntaxKind = SyntaxKind("parenthesized_type");
pub const OMEGA: SyntaxKind = SyntaxKind("ω");
pub const EXTRAS: [&str; 3] = ["line_comment", "block_comment", "shebang"];

// --- Operator table ---------------------------------------------------------

/// Binary operator-token kinds, with their prelude operator names.
///
/// Each operator elaborates to a forced prelude application
/// (`x * y` ⇒ `(force mul) x y`); see [`crate::prelude_ctx`] for the
/// operators' types. Mirrors the grammar's `BINARY_OPERATORS` table.
pub const BINARY_OPERATORS: [(&str, &str); 12] = [
    ("||", "or"),
    ("&&", "and"),
    ("==", "eq"),
    ("!=", "ne"),
    ("<", "lt"),
    ("<=", "le"),
    (">", "gt"),
    (">=", "ge"),
    ("++", "concat"),
    ("+", "add"),
    ("-", "sub"),
    ("*", "mul"),
];

/// The prelude operator unary `-` elaborates to.
pub const UNARY_NEG: &str = "neg";
pub const NAME_INL: &str = "Inl";
pub const NAME_INR: &str = "Inr";
pub const NAME_NIL: &str = "Nil";
pub const NAME_CONS: &str = "Cons";
pub const NAME_FST: &str = "fst";
pub const NAME_SND: &str = "snd";
pub const NAME_LIST_TYPE: &str = "List";
pub const NAME_PATH_TYPE: &str = "Path";
pub const NAME_HERE: &str = "here";
pub const NAME_WALK: &str = "walk";
pub const NAME_CSTR_TYPE: &str = "CStr";
pub const NAME_UNIT_TYPE: &str = "Unit";
pub const NAME_BOOLEAN_TYPE: &str = "Boolean";
pub const NAME_UNKNOWN_TYPE: &str = "Unknown";
pub const DISCARD_BINDER: &str = "_";
