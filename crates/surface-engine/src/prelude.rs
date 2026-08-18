//! Elaboration / REPL prelude bindings.
//!
//! Names a bare program can elaborate against, plus the fixed-table values the
//! machine resolves them to.
//!
//! Two faces, kept in lock-step off native primitive tags:
//!
//! - [`prelude_ctx`] — the **typing** prelude: a [`Ctx`] binding every name a
//!   bare program may mention without a `def`. The fixed-table operators
//!   ([`crate::lower::node_kinds::BINARY_OPERATORS`] +
//!   [`crate::lower::node_kinds::UNARY_NEG`]) that `binary_expression` /
//!   `unary_expression` elaborate to are eval-bound by [`prelude_env`], and the
//!   module-qualified native builtins come from [`MODULE_BUILTINS`].
//! - [`prelude_env`] — the **eval** prelude: a [`Prelude`] binding those
//!   fixed-table operators and module builtins to native values consumed by the
//!   L machine's prelude focus.
//!
//! The module-qualified names (`prim.id`, …) are produced by the lowerer's
//! module-select elaboration ([`crate::lower`]). **This module no longer
//! decides whether a name is recognized**: [`crate::recognition`] reads
//! [`MODULE_BUILTINS`] once to seed the outermost visible scope, and the
//! lowerer resolves paths against that scope. The one table still drives the
//! recognition seed, the typing, and the evaluation together, so the three
//! never drift.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::ctx::Ctx;
use gandr_core_term::grade::Grade;
use gandr_core_term::prim::NativePrim;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::ValueType;

use crate::boundary::PreludeMemberName;
use crate::boundary::PreludeModuleName;
/// Evaluation bindings consumed by the L machine's prelude focus.
///
/// This preserves the predecessor `Prelude` contract exactly: construction
/// performs no validation, binding order is retained, and later duplicate
/// names shadow earlier names when the machine prepares its focus.
///
/// # Contract
/// - ensures: preserves binding order and values byte-for-byte.
/// - provides: a nominal boundary over the L machine's `(name, value)` slice.
/// - panics: none.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prelude(Vec<(String, Value)>);

impl Prelude
{
    /// Build a prelude from ordered name/value bindings.
    ///
    /// # Contract
    /// - ensures: preserves `bindings` byte-for-byte and in order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_bindings(bindings: Vec<(String, Value)>) -> Self
    {
        Self(bindings)
    }

    /// Borrow the ordered bindings expected by the L machine.
    ///
    /// # Contract
    /// - ensures: returns every stored binding in insertion order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_bindings(&self) -> &[(String, Value)]
    {
        self.0.as_slice()
    }
}

/// The module-qualified native builtins of the MVP prelude — `(module, member,
/// primitive)`. The single source of truth for the outermost scope's
/// recognition seed ([`crate::recognition::Recognition::new`]), the typing
/// prelude ([`prelude_ctx`]), and the eval prelude ([`prelude_env`]); a member
/// here both type-checks (its declared type) and evaluates (its native
/// computation), so the three faces never drift.
///
/// MVP scope: the demonstrator module `prim` with the neutral combinators `id`
/// (`I`) and `const` (`K`) — pure, total, first-order — plus source-facing
/// `list` iteration and functional-update members, `record` access/update
/// members, `string` helpers, `regex.extract`, and `path` helpers. The
/// band-01-rung-07 completions add the `bool` (`not`) and `int` (`div`,
/// `mod`) modules and the list / string read-side and construction members.
/// The fixed-table arithmetic / comparison / boolean / list-concat operators
/// are the unqualified prelude bindings defined by the same table.
pub(crate) const MODULE_BUILTINS: &[(&str, &str, NativePrim)] = &[
    ("prim", "id", NativePrim::Id),
    ("prim", "const", NativePrim::Const),
    ("list", "each", NativePrim::Each),
    ("list", "where", NativePrim::Where),
    ("list", "reduce", NativePrim::Reduce),
    ("list", "any", NativePrim::Any),
    ("list", "all", NativePrim::All),
    ("list", "flatten", NativePrim::Flatten),
    ("list", "uniq", NativePrim::Uniq),
    ("list", "sort", NativePrim::Sort),
    // List functional-update builtins (`proposal-value-semantics-mvp.md` §3.2)
    // each return a fresh list, never an
    // lvalue. `append` reuses the `++`/`concat` operator prim (two-list
    // concatenation); `concat` reuses `flatten` (a list of lists, the Haskell
    // `concat`), so those two names add no prim.
    ("list", "set", NativePrim::Set),
    ("list", "update_at", NativePrim::UpdateAt),
    ("list", "insert_at", NativePrim::InsertAt),
    ("list", "remove_at", NativePrim::RemoveAt),
    ("list", "push", NativePrim::Push),
    ("list", "append", NativePrim::ListConcat),
    ("list", "concat", NativePrim::Flatten),
    ("list", "update_where", NativePrim::UpdateWhere),
    // List read-side builtins (band-01-rung-07): the element count and the
    // `Optional`-returning indexed read. The member is `get` — the same
    // Optional-read name `record.get` carries — because `at` is a reserved
    // word of the surface grammar (the session-types vocabulary).
    ("list", "length", NativePrim::ListLength),
    ("list", "get", NativePrim::ListAt),
    ("record", "get", NativePrim::Get),
    ("record", "insert", NativePrim::Insert),
    ("string", "escape", NativePrim::StringEscape),
    ("string", "contains", NativePrim::StringContains),
    ("string", "starts_with", NativePrim::StringStartsWith),
    ("string", "ends_with", NativePrim::StringEndsWith),
    ("string", "eq", NativePrim::StringEq),
    ("string", "split", NativePrim::StringSplit),
    // String construction (band-01-rung-07): concatenation (the string
    // counterpart of the list `++` operator) and the scalar-value length.
    ("string", "append", NativePrim::StringAppend),
    ("string", "length", NativePrim::StringLength),
    // Boolean negation (band-01-rung-07) over the canonical `Bool = 1 + 1`.
    ("bool", "not", NativePrim::Not),
    // Truncating integer division and remainder (band-01-rung-07); a zero
    // divisor evaluates to a defined hole blame, never a panic.
    ("int", "div", NativePrim::Div),
    ("int", "mod", NativePrim::Mod),
    #[cfg(feature = "regex")]
    ("regex", "extract", NativePrim::RegexExtract),
    ("path", "join", NativePrim::PathJoin),
    ("path", "basename", NativePrim::PathBasename),
    ("path", "extension", NativePrim::PathExtension),
];

/// The unqualified fixed-table operator builtins — `(prelude-name, primitive)`.
/// Kept separate from [`MODULE_BUILTINS`] because source operators elaborate to
/// bare names (`add`, `concat`, …), while `prim.id` / `prim.const` remain
/// module-qualified.
const OPERATOR_BUILTINS: &[(&str, NativePrim)] = &[
    ("or", NativePrim::Or),
    ("and", NativePrim::And),
    ("eq", NativePrim::Eq),
    ("ne", NativePrim::Ne),
    ("lt", NativePrim::Lt),
    ("le", NativePrim::Le),
    ("gt", NativePrim::Gt),
    ("ge", NativePrim::Ge),
    ("concat", NativePrim::ListConcat),
    ("add", NativePrim::Add),
    ("sub", NativePrim::Sub),
    ("mul", NativePrim::Mul),
    ("neg", NativePrim::Neg),
];

/// The typing context for the prelude.
///
/// One binding per fixed-table operator
/// ([`crate::lower::node_kinds::BINARY_OPERATORS`] +
/// [`crate::lower::node_kinds::UNARY_NEG`]) plus one per module builtin
/// ([`MODULE_BUILTINS`]), each a graded thunk so the elaborated `force` is
/// well-typed.
///
/// Operators:
///
/// - `add`, `sub`, `mul`, `neg`: `Unknown` operands/results
/// - `eq`, `ne`, `lt`, `le`, `gt`, `ge`: `Unknown → Unknown → F (1 + 1)`
/// - `and`, `or`: `(1 + 1) → (1 + 1) → F (1 + 1)`
/// - `concat`: `List Unknown → List Unknown → F (List Unknown)`
///
/// Module builtins type at `U_ω (NativePrim::declared_type)` (the design
/// record), the same type [`prelude_env`] binds the eval value at — so typing
/// and evaluation agree.
///
/// # Contract
/// - ensures: returns a `Ctx` binding every operator name and every
///   `module.member` builtin name, each as a graded thunk.
/// - provides: the typing context operator / module-select elaboration targets.
/// - panics: none.
#[inline]
#[must_use]
pub fn prelude_ctx() -> Ctx
{
    let mut ctx = Ctx::new();
    for &(name, prim) in OPERATOR_BUILTINS {
        ctx.bind(
            name.to_owned(),
            ValueType::thunk(Grade::OMEGA, prim.declared_type()),
        );
    }
    for &(module, member, prim) in MODULE_BUILTINS {
        ctx.bind(
            qualified(module, member),
            ValueType::thunk(Grade::OMEGA, prim.declared_type()),
        );
    }
    ctx
}

/// The flat qualified name `"module.member"` — the key the lowerer emits as a
/// `Var`, both preludes bind a builtin under, and the outermost scope's
/// [`crate::recognition::Recognized::PreludeMember`] binding carries.
pub(crate) fn qualified<'module, 'member>(
    module: impl Into<PreludeModuleName<'module>>,
    member: impl Into<PreludeMemberName<'member>>,
) -> String
{
    let module = module.into();
    let member = member.into();
    let mut name = module.0.to_owned();
    name.push('.');
    name.push_str(member.0);
    name
}

/// The eval-side prelude.
///
/// Fixed-table operators and module builtins are bound to thunks wrapping
/// their native computations, so the L machine's prelude focus resolves a
/// qualified name to its value.
///
/// # Contract
/// - ensures: returns a [`Prelude`] binding exactly the [`OPERATOR_BUILTINS`]
///   and [`MODULE_BUILTINS`] names, each to `thunk_ω (native prim)` whose type
///   is the same `U_ω (prim.declared_type)` [`prelude_ctx`] assigns the name.
/// - panics: none.
#[inline]
#[must_use]
pub fn prelude_env() -> Prelude
{
    let operator_bindings = OPERATOR_BUILTINS.iter().map(|&(name, prim)| {
        (
            name.to_owned(),
            Value::thunk(Grade::OMEGA, Comp::native(prim)),
        )
    });
    let module_bindings = MODULE_BUILTINS.iter().map(|&(module, member, prim)| {
        (
            qualified(module, member),
            Value::thunk(Grade::OMEGA, Comp::native(prim)),
        )
    });
    let bindings: Vec<(String, Value)> = operator_bindings.chain(module_bindings).collect();
    Prelude::from_bindings(bindings)
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use gandr_core_sequent::machine::run_comp_with_prelude;
    use gandr_core_term::outcome::Eval;

    use super::*;

    /// The wrapper preserves insertion order, and the L-machine focus resolves
    /// duplicate names from the latest binding as the predecessor prelude did.
    #[test]
    fn ordered_bindings_shadow_from_the_right()
    {
        let bindings = vec![
            (
                "x".to_owned(),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(1))),
            ),
            (
                "x".to_owned(),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(2))),
            ),
        ];
        let prelude = Prelude::from_bindings(bindings.clone());
        assert_eq!(
            bindings.as_slice(),
            prelude.as_bindings(),
            "the wrapper preserves every binding in insertion order"
        );
        assert_eq!(
            Eval::Value(Comp::ret(Value::int(2))),
            run_comp_with_prelude(&Comp::force(Value::var("x")), prelude.as_bindings()),
            "the later duplicate binding shadows the earlier binding"
        );
    }
}
