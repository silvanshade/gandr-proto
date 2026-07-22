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
//! module-select elaboration ([`crate::lower`], gated by [`is_module_member`]);
//! the one [`MODULE_BUILTINS`] table drives the recognition, the typing, and
//! the evaluation together, so the three never drift.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::ValueType;

use crate::boundary::MatchDecision;
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
/// primitive)`. The single source of truth for the lowerer's module-select
/// recognition ([`is_module_member`]), the typing prelude ([`prelude_ctx`]),
/// and the eval prelude ([`prelude_env`]); a member here both type-checks (its
/// declared type) and evaluates (its native computation), so the three faces
/// never drift.
///
/// MVP scope: the demonstrator module `prim` with the neutral combinators `id`
/// (`I`) and `const` (`K`) — pure, total, first-order — plus source-facing
/// `list` iteration and functional-update members, `record` access/update
/// members, `string` helpers, `regex.extract`, and `path` helpers. The
/// fixed-table arithmetic / comparison / boolean / list-concat operators are
/// the unqualified prelude bindings defined by the same table.
const MODULE_BUILTINS: &[(&str, &str, NativePrim)] = &[
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
    ("record", "get", NativePrim::Get),
    ("record", "insert", NativePrim::Insert),
    ("string", "escape", NativePrim::StringEscape),
    ("string", "contains", NativePrim::StringContains),
    ("string", "starts_with", NativePrim::StringStartsWith),
    ("string", "ends_with", NativePrim::StringEndsWith),
    ("string", "eq", NativePrim::StringEq),
    ("string", "split", NativePrim::StringSplit),
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

/// Whether `module.member` names a known module builtin — the lowerer's
/// module-select gate.
///
/// Decided on BOTH halves, so an unknown `module.member` (a genuine record
/// projection, once records exist) stays the structural-projection hole rather
/// than elaborating to an unbound qualified `Var`.
///
/// # Contract
/// - ensures: `true` iff `(module, member)` is an entry of [`MODULE_BUILTINS`].
/// - panics: none.
#[inline]
#[must_use]
pub fn is_module_member<
    'module,
    'member,
    M: Into<PreludeModuleName<'module>>,
    B: Into<PreludeMemberName<'member>>,
>(
    module: M,
    member: B,
) -> MatchDecision
{
    let module = module.into();
    let member = member.into();
    MatchDecision(
        MODULE_BUILTINS
            .iter()
            .any(|&(known_module, known_member, _)| {
                known_module == module.0 && known_member == member.0
            }),
    )
}

/// Whether `name` is a known builtin module.
///
/// Used by the lowerer to keep `M.member` where `M` is a known module on the
/// module path even when the member is unknown (a declined hole, the design
/// record: a module namespace is not a record value), rather than treating it
/// as a record projection (the module-selection contract).
///
/// # Contract
/// - ensures: `true` iff some entry of [`MODULE_BUILTINS`] has module `name`.
/// - panics: none.
#[inline]
#[must_use]
pub fn is_module<'name, N: Into<PreludeModuleName<'name>>>(name: N) -> MatchDecision
{
    let name = name.into();
    MatchDecision(
        MODULE_BUILTINS
            .iter()
            .any(|&(known_module, ..)| known_module == name.0),
    )
}
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
/// `Var` and both preludes bind a builtin under.
fn qualified<'module, 'member>(
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
