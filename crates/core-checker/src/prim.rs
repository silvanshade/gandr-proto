#![allow(
    unknown_lints,
    non_topologically_sorted_functions,
    reason = "cfg-alternative primitive helpers share names and the dispatcher family has no single unambiguous linear call order"
)]

//! Native (Rust-backed) builtin primitives — the registry behind
//! [`crate::syntax::Comp::Native`] (ADR-42; the MVP module layer's
//! native-builtin substrate).
//!
//! A builtin combinator cannot be written as a closed term over the v0 IR —
//! there is no recursion / fixpoint ([`crate::syntax::Comp::ListCase`] is
//! non-recursive) — so the iteration / table / arithmetic combinators are
//! realized in Rust. Because [`crate::syntax::Comp`] derives `Clone` / `Debug`
//! / `Eq` / `PartialEq` *and* is the CEK machine's runtime focus type, a native
//! node cannot carry a Rust `fn` / closure: a closure is not `Eq`, and a `fn`
//! pointer's address equality is not stable (the compiler may merge or
//! duplicate functions), which would corrupt the structural equality the
//! `checker ≡ machine` and `eval ≡ run` differentials rely on. So the node
//! carries an **opaque tag** — a [`NativePrim`] — and the Rust behavior lives
//! here in a by-value registry, exactly as [`crate::syntax::Comp::Perform`]
//! carries an operation *name* resolved against an inline signature rather than
//! a handler closure (ADR-33 D3). This keeps the IR inspectable and comparable.
//!
//! MVP scope: the registry started with two neutral
//! demonstrator combinators — `Id` (the identity `I`) and `Const` (the constant
//! `K`) — pure, total, first-order over the rigid `Integer` atom. It then also
//! took the fixed-table arithmetic / comparison / boolean / list-concat
//! primitives that the surface prelude lowers operators to, and
//! now the source-facing data / text / path combinators: higher-order list
//! combinators (`each` / `where` / `reduce` / `any` / `all`), pure list /
//! record combinators (`flatten` / `uniq` / `sort` / `get` / `insert`), record
//! and list functional-update helpers (`RecordUpdate`; `set` / `update_at` /
//! `insert_at` / `remove_at` / `push` / `append` / `concat` / `update_where`,
//! with `append` reusing `ListConcat` and `concat` reusing `Flatten`), string
//! helpers, `regex.extract`, and path helpers.
//! Update primitives return a FRESH value (the state-visibility red
//! line: no lvalue, no aliasing-visible mutation). The higher-order ones cannot
//! be closed terms over the v0 IR
//! (there is no core recursion / fixpoint), so — because the list argument
//! reaches [`NativePrim::apply`] as a **manifest** [`Value::List`] whose length
//! is therefore known — they **unroll** over that list into a closed CBPV term
//! (`force`, `bind`, `case`, application, and `ret`), which the CEK machine
//! then runs. New primitives are ADDITIONS at the frozen `native` node's slot
//! (ADR-42; `core-ir-contract.md` §0), not an IR-shape change.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;
use core::cmp::Ordering;
use std::path::Path;

#[cfg(feature = "gandr_feat_regex")]
use regex::Regex;

use crate::boundary::AppliedArgCount;
use crate::boundary::BooleanAtom;
use crate::boundary::I64Literal;
use crate::boundary::ListIndex;
use crate::boundary::NameRef;
use crate::boundary::PrimitiveArity;
use crate::boundary::PrimitivePredicate;
use crate::boundary::RegexLiteralText;
use crate::boundary::ShortCircuitFlag;
use crate::grade::Grade;
use crate::syntax::Comp;
use crate::syntax::NumLit;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// An opaque identifier for a Rust-backed builtin primitive — the tag carried
/// by [`crate::syntax::Comp::Native`].
///
/// The tag is `Copy` and compares by value, so a `Comp::Native` stays `Clone` /
/// `Debug` / `Eq` / `PartialEq` (the derive the machine and the differential
/// oracles require). Each variant knows its [`arity`](Self::arity), its
/// [`declared_type`](Self::declared_type), the
/// [`residual_type`](Self::residual_type) after a partial application, and how
/// it [`apply`](Self::apply)s once saturated.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NativePrim
{
    /// The identity combinator `I` — `Integer → F Integer`; `apply [v] = ret
    /// v`.
    Id,
    /// The constant combinator `K` — `Integer → Integer → F Integer`;
    /// `apply [a, _] = ret a` (returns its first argument, discards the
    /// second).
    Const,
    /// Same-tag numeric addition.
    Add,
    /// Same-tag numeric subtraction.
    Sub,
    /// Same-tag numeric multiplication.
    Mul,
    /// Numeric equality.
    Eq,
    /// Numeric inequality.
    Ne,
    /// Numeric less-than.
    Lt,
    /// Numeric less-than-or-equal.
    Le,
    /// Numeric greater-than.
    Gt,
    /// Numeric greater-than-or-equal.
    Ge,
    /// Boolean conjunction over the canonical `Bool = 1 + 1` encoding.
    And,
    /// Boolean disjunction over the canonical `Bool = 1 + 1` encoding.
    Or,
    /// Same-tag numeric negation.
    Neg,
    /// Homogeneous list concatenation.
    ListConcat,
    /// `each f xs` — map a pure closure `f : U(A → F B)` over a list, in order.
    /// Unrolls to `force(f) x₀ >>= r₀. … ret [r₀, …]`.
    Each,
    /// `where p xs` — filter a list by a pure predicate `p : U(A → F Bool)`,
    /// keeping order. Unrolls to a nested `case`-on-predicate
    /// chain that conses the kept elements (via [`Self::ListConcat`]).
    Where,
    /// `reduce f z xs` — left fold a list with a pure binary closure `f : U(A →
    /// B → F A)` from the seed `z`. Unrolls to `force(f) z x₀ >>=
    /// a₁. force(f) a₁ x₁ >>= a₂. …`.
    Reduce,
    /// `any p xs` — whether a pure predicate holds for some element,
    /// short-circuiting.
    Any,
    /// `all p xs` — whether a pure predicate holds for every element,
    /// short-circuiting.
    All,
    /// `flatten xs` — concatenate a manifest list of manifest lists;
    /// a pure one-step reduction (no closure).
    Flatten,
    /// `uniq xs` — drop later structural duplicates of a manifest list, keeping
    /// first occurrence; pure.
    Uniq,
    /// `sort xs` — sort a manifest list of homogeneous totally-ordered atoms
    /// (a bare integer literal, one of the sized-integer atoms
    /// `u32`/`u64`/`i32`/`i64`, or a string); pure. A
    /// non-orderable (a float — no total order over `NaN`) or heterogeneous
    /// list reduces to a gradual hole.
    Sort,
    /// `get r ℓ` — look a **dynamic** string label `ℓ` up in a manifest record
    /// `r`, returning an `Optional` (`A + 1`). The native layer's
    /// answer to the dynamic access the static
    /// [`crate::syntax::Comp::RecordProj`] cannot express.
    Get,
    /// `insert r ℓ v` — extend / override the field `ℓ` of a manifest record
    /// `r` with `v`, returning the extended record.
    Insert,
    /// `recordupdate r o` — the elaboration target of functional record update
    /// `#{ r | ℓ = v, … }` (value-semantics MVP,
    /// `proposal-value-semantics-mvp.md` §3.1). Overlays the manifest
    /// overrides record `o` onto the manifest base record `r`, returning a
    /// **fresh** record (override labels win, new labels extend) — the
    /// copy-and-update whole surface, holding the
    /// state-visibility red line (the base binding observes no change). Not a
    /// module builtin: the lowerer emits it directly, so it carries no surface
    /// name.
    RecordUpdate,
    /// `set xs i v` — replace the element at index `i` of a manifest list with
    /// `v`, returning a **fresh** list (list functional-update;
    /// `proposal-value-semantics-mvp.md` §3.2). An out-of-bounds index
    /// degenerates to a gradual hole. No lvalue / in-place assignment.
    Set,
    /// `update-at xs i f` — apply the pure closure `f : U(A → F A)` to the
    /// element at index `i`, returning a **fresh** list with that element
    /// replaced by the result. Higher-order: unrolls to
    /// `force(f) xs[i] >>= r. ret [ …, r, … ]`. Out-of-bounds ⇒ a gradual hole.
    UpdateAt,
    /// `insert-at xs i v` — insert `v` before index `i` of a manifest list
    /// (`i == len` appends), returning a **fresh**, longer list.
    /// An index past the end degenerates to a gradual hole.
    InsertAt,
    /// `remove-at xs i` — drop the element at index `i` of a manifest list,
    /// returning a **fresh**, shorter list. Out-of-bounds ⇒ a
    /// gradual hole.
    RemoveAt,
    /// `push xs v` — append `v` to the end of a manifest list, returning a
    /// **fresh**, longer list.
    Push,
    /// `update-where p f xs` — apply the pure closure `f : U(A → F A)` to every
    /// element for which the pure predicate `p : U(A → F Bool)` holds, keeping
    /// the rest unchanged and preserving order, returning a **fresh** list.
    /// Higher-order: unrolls to a per-element
    /// `case`-on-predicate chain (the transform-or-keep dual of
    /// [`Self::Where`]'s filter).
    UpdateWhere,
    /// `escape s` — quote regex metacharacters in `s` for literal matching.
    StringEscape,
    /// `contains s needle` — whether `s` contains the literal `needle`.
    StringContains,
    /// `starts_with s prefix` — whether `s` starts with `prefix`.
    StringStartsWith,
    /// `ends_with s suffix` — whether `s` ends with `suffix`.
    StringEndsWith,
    /// `eq s t` — string equality. The string counterpart of the
    /// numeric [`Self::Eq`], and the dispatch prim string-literal patterns
    /// elaborate through (`proposal-data-patterns.md` §17).
    StringEq,
    /// `split s sep` — split `s` on the literal separator `sep`.
    StringSplit,
    /// `extract pattern haystack` — first regex match's named captures as a
    /// record.
    RegexExtract,
    /// `join base child` — pure UTF-8 path join.
    PathJoin,
    /// `basename path` — final UTF-8 path component.
    PathBasename,
    /// `extension path` — final UTF-8 extension without the dot.
    PathExtension,
}

impl NativePrim
{
    /// How many arguments this primitive consumes before it reduces.
    ///
    /// # Contract
    /// - ensures: returns a positive arity — every builtin is a function, so a
    ///   nullary "native" (which would never meet an argument frame, hence
    ///   never reduce) is not representable.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn arity(self) -> PrimitiveArity
    {
        let arity = match self {
            | Self::Id
            | Self::Neg
            | Self::Flatten
            | Self::Uniq
            | Self::Sort
            | Self::StringEscape
            | Self::PathBasename
            | Self::PathExtension => 1,
            | Self::Const
            | Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Eq
            | Self::Ne
            | Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge
            | Self::And
            | Self::Or
            | Self::ListConcat
            | Self::Each
            | Self::Where
            | Self::Any
            | Self::All
            | Self::Get
            | Self::RecordUpdate
            | Self::Push
            | Self::RemoveAt
            | Self::StringContains
            | Self::StringStartsWith
            | Self::StringEndsWith
            | Self::StringEq
            | Self::StringSplit
            | Self::RegexExtract
            | Self::PathJoin => 2,
            | Self::Reduce
            | Self::Insert
            | Self::Set
            | Self::UpdateAt
            | Self::InsertAt
            | Self::UpdateWhere => 3,
        };
        arity.into()
    }

    /// The fully-curried declared type of this primitive — the type a
    /// source-level (argument-free) native node has, and the single source of
    /// truth the typing prelude (`gandr_pipeline`'s `prelude_ctx`) binds the
    /// builtin's name to.
    ///
    /// # Contract
    /// - ensures: returns a chain of [`arity`](Self::arity) `→` arrows ending
    ///   in a pure returner `F A`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn declared_type(self) -> CompType
    {
        match self {
            | Self::Id => CompType::arrow(
                ValueType::integer(),
                CompType::returner(ValueType::integer()),
            ),
            | Self::Const => CompType::arrow(
                ValueType::integer(),
                CompType::arrow(
                    ValueType::integer(),
                    CompType::returner(ValueType::integer()),
                ),
            ),
            | Self::Add | Self::Sub | Self::Mul => unknown_binary(ValueType::Unknown),
            | Self::Neg => {
                CompType::arrow(ValueType::Unknown, CompType::returner(ValueType::Unknown))
            },
            | Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge => {
                unknown_binary(bool_type())
            },
            | Self::And | Self::Or => {
                let bool_ty = bool_type();
                CompType::arrow(
                    bool_ty.clone(),
                    CompType::arrow(bool_ty.clone(), CompType::returner(bool_ty)),
                )
            },
            | Self::ListConcat => {
                let list_ty = ValueType::list(ValueType::Unknown);
                CompType::arrow(
                    list_ty.clone(),
                    CompType::arrow(list_ty.clone(), CompType::returner(list_ty)),
                )
            },
            // `each f xs : U(? → F ?) → List ? → F (List ?)`.
            | Self::Each => CompType::arrow(
                pure_closure(),
                CompType::arrow(list_unknown(), CompType::returner(list_unknown())),
            ),
            // `where p xs : U(? → F Bool) → List ? → F (List ?)`.
            | Self::Where => CompType::arrow(
                pred_closure(),
                CompType::arrow(list_unknown(), CompType::returner(list_unknown())),
            ),
            // `reduce f z xs : U(? → ? → F ?) → ? → List ? → F ?`.
            | Self::Reduce => CompType::arrow(
                binop_closure(),
                CompType::arrow(
                    ValueType::Unknown,
                    CompType::arrow(list_unknown(), CompType::returner(ValueType::Unknown)),
                ),
            ),
            // `any p xs`, `all p xs : U(? → F Bool) → List ? → F Bool`.
            | Self::Any | Self::All => CompType::arrow(
                pred_closure(),
                CompType::arrow(list_unknown(), CompType::returner(bool_type())),
            ),
            // `flatten xs`, `uniq xs`, `sort xs : List ? → F (List ?)`.
            | Self::Flatten | Self::Uniq | Self::Sort => {
                CompType::arrow(list_unknown(), CompType::returner(list_unknown()))
            },
            // `get r ℓ : {} → String → F (? + 1)` — the empty record `{}` is the
            // TOP of the width order, so any record fits it (ADR-45).
            | Self::Get => CompType::arrow(
                empty_record(),
                CompType::arrow(ValueType::string(), CompType::returner(option_type())),
            ),
            // `insert r ℓ v : {} → String → ? → F {}`.
            | Self::Insert => CompType::arrow(
                empty_record(),
                CompType::arrow(
                    ValueType::string(),
                    CompType::arrow(ValueType::Unknown, CompType::returner(empty_record())),
                ),
            ),
            // `recordupdate r o : {} → {} → F {}` — the empty record `{}` is the
            // TOP of the width order, so any record base / overrides fit it
            // (ADR-45). The MVP result types gradually as `{}` (the same
            // limitation the sibling `insert` builtin carries); the precise
            // "base type with ℓ retyped / widened" result of
            // `proposal-value-semantics-mvp.md` §3.1 needs type-directed
            // elaboration, deferred as an as-built note there.
            | Self::RecordUpdate => CompType::arrow(
                empty_record(),
                CompType::arrow(empty_record(), CompType::returner(empty_record())),
            ),
            // `set xs i v`, `insert-at xs i v : List ? → Integer → ? → F (List ?)`.
            | Self::Set | Self::InsertAt => CompType::arrow(
                list_unknown(),
                CompType::arrow(
                    ValueType::integer(),
                    CompType::arrow(ValueType::Unknown, CompType::returner(list_unknown())),
                ),
            ),
            // `update-at xs i f : List ? → Integer → U(? → F ?) → F (List ?)`.
            | Self::UpdateAt => CompType::arrow(
                list_unknown(),
                CompType::arrow(
                    ValueType::integer(),
                    CompType::arrow(pure_closure(), CompType::returner(list_unknown())),
                ),
            ),
            // `remove-at xs i : List ? → Integer → F (List ?)`.
            | Self::RemoveAt => CompType::arrow(
                list_unknown(),
                CompType::arrow(ValueType::integer(), CompType::returner(list_unknown())),
            ),
            // `push xs v : List ? → ? → F (List ?)`.
            | Self::Push => CompType::arrow(
                list_unknown(),
                CompType::arrow(ValueType::Unknown, CompType::returner(list_unknown())),
            ),
            // `update-where p f xs : U(? → F Bool) → U(? → F ?) → List ? → F (List ?)`.
            | Self::UpdateWhere => CompType::arrow(
                pred_closure(),
                CompType::arrow(
                    pure_closure(),
                    CompType::arrow(list_unknown(), CompType::returner(list_unknown())),
                ),
            ),
            | Self::StringEscape | Self::PathBasename | Self::PathExtension => {
                CompType::arrow(ValueType::string(), CompType::returner(ValueType::string()))
            },
            | Self::StringContains
            | Self::StringStartsWith
            | Self::StringEndsWith
            | Self::StringEq => CompType::arrow(
                ValueType::string(),
                CompType::arrow(ValueType::string(), CompType::returner(bool_type())),
            ),
            | Self::StringSplit => CompType::arrow(
                ValueType::string(),
                CompType::arrow(ValueType::string(), CompType::returner(list_unknown())),
            ),
            | Self::RegexExtract => CompType::arrow(
                ValueType::string(),
                CompType::arrow(ValueType::string(), CompType::returner(empty_record())),
            ),
            | Self::PathJoin => CompType::arrow(
                ValueType::string(),
                CompType::arrow(ValueType::string(), CompType::returner(ValueType::string())),
            ),
        }
    }

    /// The residual computation type after `applied` arguments have been
    /// consumed: the [`declared_type`](Self::declared_type) with `applied`
    /// leading arrows peeled.
    ///
    /// A source-level native has `applied = 0`, so this is the declared type;
    /// the partially-applied forms arise only mid-evaluation (the CEK machine
    /// accumulates arguments into the node), where peeling yields the residual
    /// function type — this is what keeps subject reduction sound over the
    /// node. An over-application (more arguments than arrows — unreachable
    /// from a well-typed source) degenerates to the un-peeled tail.
    ///
    /// # Contract
    /// - ensures: for `applied ≤ arity` exactly `applied` arrows are peeled;
    ///   the `applied = arity` case is the final returner `F A`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn residual_type<A>(
        self,
        applied: A,
    ) -> CompType
    where
        A: Into<AppliedArgCount>,
    {
        let applied = usize::from(applied.into());
        let mut ty = self.declared_type();
        for _ in 0 .. applied {
            ty = match ty {
                | CompType::Arrow(_, res) => (*res).clone(),
                | other => return other,
            };
        }
        ty
    }

    /// Reduces a **saturated** application: given exactly
    /// [`arity`](Self::arity) arguments, the computation the builtin steps
    /// to.
    ///
    /// # Contract
    /// - requires: `args.len() == self.arity()` (the CEK machine only calls
    ///   this once the node has accumulated a full argument list).
    /// - ensures: returns the result computation the builtin steps to — a `ret
    ///   v` for the scalar prims, and for the source-facing combinators an
    ///   **unrolled** closed CBPV term (`force` / `bind` / `case` / application
    ///   / `ret`) over the manifest list argument. A wrong-shape argument (a
    ///   non-manifest list / record / label, a non-orderable `sort`, an
    ///   argument-count mismatch) degenerates to a gradual hole, which
    ///   evaluates to a defined `Blame::Hole` rather than panicking, so `apply`
    ///   stays total.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn apply(
        self,
        args: &[Rc<Value>],
    ) -> Comp
    {
        match self {
            | Self::Id => match one_arg(args) {
                | Some(value) => Comp::ret(value.clone()),
                | None => Comp::hole(0),
            },
            | Self::Const => match two_args(args) {
                | Some((first, _second)) => Comp::ret(first.clone()),
                | None => Comp::hole(0),
            },
            | Self::Add => numeric_binary(args, numeric_add),
            | Self::Sub => numeric_binary(args, numeric_sub),
            | Self::Mul => numeric_binary(args, numeric_mul),
            | Self::Eq => numeric_compare(args, numeric_eq),
            | Self::Ne => numeric_compare(args, |lhs, rhs| {
                numeric_eq(lhs, rhs).map(PrimitivePredicate::negated)
            }),
            | Self::Lt => numeric_compare(args, numeric_lt),
            | Self::Le => numeric_compare(args, numeric_le),
            | Self::Gt => numeric_compare(args, numeric_gt),
            | Self::Ge => numeric_compare(args, numeric_ge),
            | Self::And => bool_binary(args, |lhs, rhs| bool::from(lhs) && bool::from(rhs)),
            | Self::Or => bool_binary(args, |lhs, rhs| bool::from(lhs) || bool::from(rhs)),
            | Self::Neg => numeric_unary(args, numeric_neg),
            | Self::ListConcat => list_concat(args),
            | Self::Each => native_each(args),
            | Self::Where => native_where(args),
            | Self::Reduce => native_reduce(args),
            | Self::Any => native_any(args),
            | Self::All => native_all(args),
            | Self::Flatten => native_flatten(args),
            | Self::Uniq => native_uniq(args),
            | Self::Sort => native_sort(args),
            | Self::Get => native_get(args),
            | Self::Insert => native_insert(args),
            | Self::RecordUpdate => native_record_update(args),
            | Self::Set => native_set(args),
            | Self::UpdateAt => native_update_at(args),
            | Self::InsertAt => native_insert_at(args),
            | Self::RemoveAt => native_remove_at(args),
            | Self::Push => native_push(args),
            | Self::UpdateWhere => native_update_where(args),
            | Self::StringEscape => native_string_escape(args),
            | Self::StringContains => {
                native_string_predicate(args, |haystack, needle| haystack.contains(needle))
            },
            | Self::StringStartsWith => {
                native_string_predicate(args, |haystack, prefix| haystack.starts_with(prefix))
            },
            | Self::StringEndsWith => {
                native_string_predicate(args, |haystack, suffix| haystack.ends_with(suffix))
            },
            | Self::StringEq => native_string_predicate(args, |lhs, rhs| lhs == rhs),
            | Self::StringSplit => native_string_split(args),
            | Self::RegexExtract => native_regex_extract(args),
            | Self::PathJoin => native_path_join(args),
            | Self::PathBasename => native_path_basename(args),
            | Self::PathExtension => native_path_extension(args),
        }
    }
}

/// Builds the gradual binary function type `Unknown -> Unknown -> F result`.
#[inline]
#[must_use]
fn unknown_binary(result: ValueType) -> CompType
{
    CompType::arrow(
        ValueType::Unknown,
        CompType::arrow(ValueType::Unknown, CompType::returner(result)),
    )
}

/// The gradual list type `List ?` — the shape the list combinators consume and
/// produce.
#[inline]
#[must_use]
fn list_unknown() -> ValueType
{
    ValueType::list(ValueType::Unknown)
}

/// The empty record `{}` — the TOP of the width order (ADR-45), so it is the
/// gradual "any record" shape the record combinators take as their subject.
#[inline]
#[must_use]
fn empty_record() -> ValueType
{
    ValueType::Record(BTreeMap::new())
}

/// The **pure** unary closure argument `U_ω(? → F ?)` of `each`.
///
/// The codomain is a *pure* returner `F ?` (empty effect row): the v0
/// combinators are pure higher-order functions, so an effectful closure fails
/// the row leg of subtyping (`⟨E⟩ ⊄ ⟨⟩`) — a shape mismatch rather than a
/// silently dropped effect. Effect-polymorphic combinators await the `+poly`
/// row variable (a residual).
#[inline]
#[must_use]
fn pure_closure() -> ValueType
{
    ValueType::thunk(
        Grade::OMEGA,
        CompType::arrow(ValueType::Unknown, CompType::returner(ValueType::Unknown)),
    )
}

/// The **pure** predicate closure argument `U_ω(? → F Bool)` of `where` / `any`
/// / `all` (see [`pure_closure`] on the pure-returner choice).
#[inline]
#[must_use]
fn pred_closure() -> ValueType
{
    ValueType::thunk(
        Grade::OMEGA,
        CompType::arrow(ValueType::Unknown, CompType::returner(bool_type())),
    )
}

/// The **pure** binary closure argument `U_ω(? → ? → F ?)` of `reduce` (see
/// [`pure_closure`] on the pure-returner choice).
#[inline]
#[must_use]
fn binop_closure() -> ValueType
{
    ValueType::thunk(
        Grade::OMEGA,
        CompType::arrow(
            ValueType::Unknown,
            CompType::arrow(ValueType::Unknown, CompType::returner(ValueType::Unknown)),
        ),
    )
}

/// Reads a canonical boolean value, ignoring source annotations.
#[inline]
#[must_use]
fn as_bool(value: &Value) -> Option<BooleanAtom>
{
    match unannotated(value) {
        | Value::Inj(crate::syntax::Side::Fst, payload)
            if matches!(unannotated(payload.as_ref()), Value::Unit) =>
        {
            Some(BooleanAtom::from(true))
        },
        | Value::Inj(crate::syntax::Side::Snd, payload)
            if matches!(unannotated(payload.as_ref()), Value::Unit) =>
        {
            Some(BooleanAtom::from(false))
        },
        | _ => None,
    }
}

/// Builds the canonical annotated boolean value (`true = inj1 ()`, `false =
/// inj2 ()`).
#[inline]
#[must_use]
fn bool_value<B>(value: B) -> Value
where
    B: Into<BooleanAtom>,
{
    let value = bool::from(value.into());
    let inj = if value {
        Value::inj1(Value::Unit)
    }
    else {
        Value::inj2(Value::Unit)
    };
    Value::annot(inj, bool_type())
}

/// The canonical boolean carrier (`1 + 1`) used by surface booleans and native
/// operators.
#[inline]
#[must_use]
fn bool_type() -> ValueType
{
    ValueType::sum(ValueType::Unit, ValueType::Unit)
}

/// Applies a total binary numeric primitive to two native arguments.
#[inline]
#[must_use]
fn numeric_binary(
    args: &[Rc<Value>],
    op: fn(Value, Value) -> Option<Value>,
) -> Comp
{
    match two_args(args) {
        | Some((lhs, rhs)) => {
            let lhs = unannotated(lhs);
            let rhs = unannotated(rhs);
            op(lhs, rhs).map_or_else(|| Comp::hole(0), Comp::ret)
        },
        | None => Comp::hole(0),
    }
}

/// Applies a total unary numeric primitive to one native argument.
#[inline]
#[must_use]
fn numeric_unary(
    args: &[Rc<Value>],
    op: fn(&Value) -> Option<Value>,
) -> Comp
{
    match one_arg(args) {
        | Some(value) => {
            let value = unannotated(value);
            op(&value).map_or_else(|| Comp::hole(0), Comp::ret)
        },
        | None => Comp::hole(0),
    }
}

/// Applies a total numeric comparison and wraps the boolean result canonically.
#[inline]
#[must_use]
fn numeric_compare<P>(
    args: &[Rc<Value>],
    op: P,
) -> Comp
where
    P: FnOnce(Value, Value) -> Option<PrimitivePredicate>,
{
    match two_args(args) {
        | Some((lhs, rhs)) => {
            let lhs = unannotated(lhs);
            let rhs = unannotated(rhs);
            op(lhs, rhs).map_or_else(|| Comp::hole(0), ret_bool)
        },
        | None => Comp::hole(0),
    }
}

/// Applies a total boolean binary primitive to canonical booleans.
#[inline]
#[must_use]
fn bool_binary<P>(
    args: &[Rc<Value>],
    op: P,
) -> Comp
where
    P: FnOnce(BooleanAtom, BooleanAtom) -> bool,
{
    match two_args(args) {
        | Some((lhs, rhs)) => match (as_bool(lhs), as_bool(rhs)) {
            | (Some(lhs), Some(rhs)) => ret_bool(op(lhs, rhs)),
            | _ => Comp::hole(0),
        },
        | None => Comp::hole(0),
    }
}

/// Adds two same-tag numeric values without implicit widening.
#[inline]
#[must_use]
fn numeric_add(
    lhs: Value,
    rhs: Value,
) -> Option<Value>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => checked_i64(
            I64Literal::from(lhs),
            I64Literal::from(rhs),
            i64::checked_add,
        )
        .map(|literal| Value::int(i64::from(literal))),
        | (Value::Num(lhs), Value::Num(rhs)) => num_add(lhs, rhs).map(Value::num),
        | _ => None,
    }
}

/// Subtracts two same-tag numeric values without implicit widening.
#[inline]
#[must_use]
fn numeric_sub(
    lhs: Value,
    rhs: Value,
) -> Option<Value>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => checked_i64(
            I64Literal::from(lhs),
            I64Literal::from(rhs),
            i64::checked_sub,
        )
        .map(|literal| Value::int(i64::from(literal))),
        | (Value::Num(lhs), Value::Num(rhs)) => num_sub(lhs, rhs).map(Value::num),
        | _ => None,
    }
}

/// Multiplies two same-tag numeric values without implicit widening.
#[inline]
#[must_use]
fn numeric_mul(
    lhs: Value,
    rhs: Value,
) -> Option<Value>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => checked_i64(
            I64Literal::from(lhs),
            I64Literal::from(rhs),
            i64::checked_mul,
        )
        .map(|literal| Value::int(i64::from(literal))),
        | (Value::Num(lhs), Value::Num(rhs)) => num_mul(lhs, rhs).map(Value::num),
        | _ => None,
    }
}

/// Negates one numeric value without changing its tag.
#[inline]
#[must_use]
fn numeric_neg(value: &Value) -> Option<Value>
{
    match value.clone() {
        | Value::Int(value) => value.checked_neg().map(Value::int),
        | Value::Num(value) => num_neg(value).map(Value::num),
        | _ => None,
    }
}

/// Compares two same-tag numeric values for equality.
#[inline]
#[must_use]
fn numeric_eq(
    lhs: Value,
    rhs: Value,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => Some(PrimitivePredicate::from(lhs == rhs)),
        | (Value::Num(lhs), Value::Num(rhs)) => num_eq(lhs, rhs),
        | _ => None,
    }
}

/// Compares two same-tag numeric values with `<`.
#[inline]
#[must_use]
fn numeric_lt(
    lhs: Value,
    rhs: Value,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => Some(PrimitivePredicate::from(lhs < rhs)),
        | (Value::Num(lhs), Value::Num(rhs)) => num_lt(lhs, rhs),
        | _ => None,
    }
}

/// Compares two same-tag numeric values with `<=`.
#[inline]
#[must_use]
fn numeric_le(
    lhs: Value,
    rhs: Value,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => Some(PrimitivePredicate::from(lhs <= rhs)),
        | (Value::Num(lhs), Value::Num(rhs)) => num_le(lhs, rhs),
        | _ => None,
    }
}

/// Compares two same-tag numeric values with `>`.
#[inline]
#[must_use]
fn numeric_gt(
    lhs: Value,
    rhs: Value,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => Some(PrimitivePredicate::from(lhs > rhs)),
        | (Value::Num(lhs), Value::Num(rhs)) => num_gt(lhs, rhs),
        | _ => None,
    }
}

/// Compares two same-tag numeric values with `>=`.
#[inline]
#[must_use]
fn numeric_ge(
    lhs: Value,
    rhs: Value,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (Value::Int(lhs), Value::Int(rhs)) => Some(PrimitivePredicate::from(lhs >= rhs)),
        | (Value::Num(lhs), Value::Num(rhs)) => num_ge(lhs, rhs),
        | _ => None,
    }
}

/// Applies a checked `i64` binary operation.
#[inline]
#[must_use]
fn checked_i64<P>(
    lhs: I64Literal,
    rhs: I64Literal,
    op: P,
) -> Option<I64Literal>
where
    P: FnOnce(i64, i64) -> Option<i64>,
{
    op(i64::from(lhs), i64::from(rhs)).map(I64Literal::from)
}

/// Adds two [`NumLit`] values with identical tags.
#[inline]
#[must_use]
fn num_add(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<NumLit>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => lhs.checked_add(rhs).map(NumLit::U32),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => lhs.checked_add(rhs).map(NumLit::U64),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => lhs.checked_add(rhs).map(NumLit::I32),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => lhs.checked_add(rhs).map(NumLit::I64),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => {
            Some(NumLit::f32(f32::from_bits(lhs) + f32::from_bits(rhs)))
        },
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => {
            Some(NumLit::f64(f64::from_bits(lhs) + f64::from_bits(rhs)))
        },
        | _ => None,
    }
}

/// Subtracts two [`NumLit`] values with identical tags.
#[inline]
#[must_use]
fn num_sub(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<NumLit>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => lhs.checked_sub(rhs).map(NumLit::U32),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => lhs.checked_sub(rhs).map(NumLit::U64),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => lhs.checked_sub(rhs).map(NumLit::I32),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => lhs.checked_sub(rhs).map(NumLit::I64),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => {
            Some(NumLit::f32(f32::from_bits(lhs) - f32::from_bits(rhs)))
        },
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => {
            Some(NumLit::f64(f64::from_bits(lhs) - f64::from_bits(rhs)))
        },
        | _ => None,
    }
}

/// Multiplies two [`NumLit`] values with identical tags.
#[inline]
#[must_use]
fn num_mul(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<NumLit>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => lhs.checked_mul(rhs).map(NumLit::U32),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => lhs.checked_mul(rhs).map(NumLit::U64),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => lhs.checked_mul(rhs).map(NumLit::I32),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => lhs.checked_mul(rhs).map(NumLit::I64),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => {
            Some(NumLit::f32(f32::from_bits(lhs) * f32::from_bits(rhs)))
        },
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => {
            Some(NumLit::f64(f64::from_bits(lhs) * f64::from_bits(rhs)))
        },
        | _ => None,
    }
}

/// Negates one [`NumLit`] without changing its tag.
#[inline]
#[must_use]
fn num_neg(value: NumLit) -> Option<NumLit>
{
    match value {
        | NumLit::U32(value) => 0_u32.checked_sub(value).map(NumLit::U32),
        | NumLit::U64(value) => 0_u64.checked_sub(value).map(NumLit::U64),
        | NumLit::I32(value) => value.checked_neg().map(NumLit::I32),
        | NumLit::I64(value) => value.checked_neg().map(NumLit::I64),
        | NumLit::F32(value) => Some(NumLit::f32(-f32::from_bits(value))),
        | NumLit::F64(value) => Some(NumLit::f64(-f64::from_bits(value))),
    }
}

/// Compares two [`NumLit`] values with identical tags for equality.
#[inline]
#[must_use]
fn num_eq(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => Some(PrimitivePredicate::from(lhs == rhs)),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => Some(PrimitivePredicate::from(lhs == rhs)),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => Some(PrimitivePredicate::from(lhs == rhs)),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => Some(PrimitivePredicate::from(lhs == rhs)),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => Some(PrimitivePredicate::from(
            f32::from_bits(lhs) == f32::from_bits(rhs),
        )),
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => Some(PrimitivePredicate::from(
            f64::from_bits(lhs) == f64::from_bits(rhs),
        )),
        | _ => None,
    }
}

/// Compares two [`NumLit`] values with identical tags using `<`.
#[inline]
#[must_use]
fn num_lt(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => Some(PrimitivePredicate::from(lhs < rhs)),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => Some(PrimitivePredicate::from(lhs < rhs)),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => Some(PrimitivePredicate::from(lhs < rhs)),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => Some(PrimitivePredicate::from(lhs < rhs)),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => Some(PrimitivePredicate::from(
            f32::from_bits(lhs) < f32::from_bits(rhs),
        )),
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => Some(PrimitivePredicate::from(
            f64::from_bits(lhs) < f64::from_bits(rhs),
        )),
        | _ => None,
    }
}

/// Compares two [`NumLit`] values with identical tags using `<=`.
#[inline]
#[must_use]
fn num_le(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => Some(PrimitivePredicate::from(lhs <= rhs)),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => Some(PrimitivePredicate::from(lhs <= rhs)),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => Some(PrimitivePredicate::from(lhs <= rhs)),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => Some(PrimitivePredicate::from(lhs <= rhs)),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => Some(PrimitivePredicate::from(
            f32::from_bits(lhs) <= f32::from_bits(rhs),
        )),
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => Some(PrimitivePredicate::from(
            f64::from_bits(lhs) <= f64::from_bits(rhs),
        )),
        | _ => None,
    }
}

/// Compares two [`NumLit`] values with identical tags using `>`.
#[inline]
#[must_use]
fn num_gt(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => Some(PrimitivePredicate::from(lhs > rhs)),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => Some(PrimitivePredicate::from(lhs > rhs)),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => Some(PrimitivePredicate::from(lhs > rhs)),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => Some(PrimitivePredicate::from(lhs > rhs)),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => Some(PrimitivePredicate::from(
            f32::from_bits(lhs) > f32::from_bits(rhs),
        )),
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => Some(PrimitivePredicate::from(
            f64::from_bits(lhs) > f64::from_bits(rhs),
        )),
        | _ => None,
    }
}

/// Compares two [`NumLit`] values with identical tags using `>=`.
#[inline]
#[must_use]
fn num_ge(
    lhs: NumLit,
    rhs: NumLit,
) -> Option<PrimitivePredicate>
{
    match (lhs, rhs) {
        | (NumLit::U32(lhs), NumLit::U32(rhs)) => Some(PrimitivePredicate::from(lhs >= rhs)),
        | (NumLit::U64(lhs), NumLit::U64(rhs)) => Some(PrimitivePredicate::from(lhs >= rhs)),
        | (NumLit::I32(lhs), NumLit::I32(rhs)) => Some(PrimitivePredicate::from(lhs >= rhs)),
        | (NumLit::I64(lhs), NumLit::I64(rhs)) => Some(PrimitivePredicate::from(lhs >= rhs)),
        | (NumLit::F32(lhs), NumLit::F32(rhs)) => Some(PrimitivePredicate::from(
            f32::from_bits(lhs) >= f32::from_bits(rhs),
        )),
        | (NumLit::F64(lhs), NumLit::F64(rhs)) => Some(PrimitivePredicate::from(
            f64::from_bits(lhs) >= f64::from_bits(rhs),
        )),
        | _ => None,
    }
}

/// Concatenates two list values, ignoring source annotations.
#[inline]
#[must_use]
fn list_concat(args: &[Rc<Value>]) -> Comp
{
    match two_args(args) {
        | Some((lhs, rhs)) => match (unannotated(lhs), unannotated(rhs)) {
            | (Value::List(mut lhs), Value::List(rhs)) => {
                lhs.extend(rhs);
                Comp::ret(Value::List(lhs))
            },
            | _ => Comp::hole(0),
        },
        | None => Comp::hole(0),
    }
}

/// Reads a manifest non-negative list index (a bare [`Value::Int`]), ignoring
/// annotations. A negative index (or a non-integer) has no valid position and
/// yields `None`, which the list-update builtins turn into a gradual hole.
#[inline]
#[must_use]
fn as_index(value: &Value) -> Option<ListIndex>
{
    match unannotated(value) {
        | Value::Int(index) => usize::try_from(index).ok().map(ListIndex::from),
        | _ => None,
    }
}

/// A canonical boolean returner value (`ret true` / `ret false`) as an
/// annotated computation, matching the native comparison / boolean primitives.
#[inline]
#[must_use]
fn ret_bool<P>(value: P) -> Comp
where
    P: Into<PrimitivePredicate>,
{
    Comp::ret(bool_value(BooleanAtom::from(bool::from(value.into()))))
}

/// The `Optional` value `Some payload` (`inj1 payload : ? + 1`).
#[inline]
#[must_use]
fn option_some(payload: Value) -> Value
{
    Value::annot(Value::inj1(payload), option_type())
}

/// The `Optional` value `None` (`inj2 () : ? + 1`).
#[inline]
#[must_use]
fn option_none() -> Value
{
    Value::annot(Value::inj2(Value::Unit), option_type())
}

// ─── Source-facing data / text / path combinators ────────────────────────────
//
// The higher-order list combinators (`each` / `where` / `reduce` / `any` /
// `all`) reach `apply` at saturation with the list argument already a manifest
// `Value::List`, so its length is known and they **unroll** it into a closed
// CBPV term the CEK machine runs. There is deliberately NO core recursion (the
// v0 IR has no fixpoint), which is exactly why these live in Rust. The
// generated binders carry a `$` sigil the
// surface identifier grammar (`[a-z_][A-Za-z0-9_]*`) cannot produce, so they
// never capture a source variable; the unrolls use only single-binder `bind`
// and disjoint-arm `case`, never a simultaneous two-binder form, so the
// `Split (x, x)` head==tail collision class cannot arise.

/// The `Optional` carrier `? + 1` returned by [`NativePrim::Get`] (`Some v =
/// inj1 v`, `None = inj2 ()`).
#[inline]
#[must_use]
fn option_type() -> ValueType
{
    ValueType::sum(ValueType::Unknown, ValueType::Unit)
}

/// The runtime cons `head :: tail` built as `ListConcat [head] tail`, so the
/// filtered tail (a runtime list value) is prepended without a structural
/// destructor (`where`).
#[inline]
#[must_use]
fn native_cons(
    head: Value,
    tail: Value,
) -> Comp
{
    let singleton = Value::List(alloc::vec![Rc::new(head)]);
    Comp::app(
        Comp::app(Comp::native(NativePrim::ListConcat), singleton),
        tail,
    )
}

/// `each f xs` — map the pure closure `f` over the manifest list `xs`, in
/// order: `force(f) x₀ >>= r₀. … force(f) xₙ >>= rₙ. ret [r₀, …, rₙ]`.
#[inline]
#[must_use]
fn native_each(args: &[Rc<Value>]) -> Comp
{
    let Some((closure, list)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let results = (0 .. items.len())
        .map(|index| Rc::new(Value::var(&gen_binder("each", index))))
        .collect();
    let mut body = Comp::ret(Value::List(results));
    for (index, item) in items.iter().enumerate().rev() {
        let call = Comp::app(Comp::force(closure.clone()), (**item).clone());
        body = Comp::bind(call, &gen_binder("each", index), body);
    }
    body
}

/// `where p xs` — keep the elements of the manifest list `xs` for which the
/// pure predicate `p` returns `true`, preserving order. Built right-to-left;
/// each level forces the predicate, binds the filtered tail ONCE, then
/// branches (`force p x >>= t. rest >>= ys. case t { inl _ ⇒ x :: ys | inr _
/// ⇒ ys }`). The tail is bound, not inlined into both `case` arms: inlining
/// makes substitution-based eval duplicate it per level into a Θ(2ⁿ) term
/// (OOM on ~30 elements — a review finding); the tail is pure and always
/// forced, so binding it before the branch is observationally identical and
/// unrolls linearly.
#[inline]
#[must_use]
fn native_where(args: &[Rc<Value>]) -> Comp
{
    let Some((pred, list)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let mut body = Comp::ret(Value::List(Vec::new()));
    for (index, item) in items.iter().enumerate().rev() {
        let element = (**item).clone();
        let test = Comp::app(Comp::force(pred.clone()), element.clone());
        let test_binder = gen_binder("wt", index);
        let tail_binder = gen_binder("wys", index);
        // The filtered tail is bound ONCE (`bind body to wys`) and BOTH arms
        // reference the bound variable — the `keep` arm conses onto it, the
        // drop arm returns it. Inlining `body` into the arms instead would let
        // substitution duplicate it per level → Θ(2ⁿ) (a review finding).
        let keep = native_cons(element, Value::var(&tail_binder));
        let discard = Comp::ret(Value::var(&tail_binder));
        let decide = Comp::case(Value::var(&test_binder), "$wp", keep, "$wq", discard);
        body = Comp::bind(test, &test_binder, Comp::bind(body, &tail_binder, decide));
    }
    body
}

/// `reduce f z xs` — left fold the manifest list `xs` with the pure binary
/// closure `f` from the seed `z`: `f z x₀ >>= a₀. f a₀ x₁ >>= a₁. … f aₙ₋₂
/// xₙ₋₁`. Built right-to-left with an explicit loop — NOT host recursion of
/// depth = list length (which would overflow the Rust stack while BUILDING
/// the term for a large manifest list — a review finding), and NOT core
/// recursion (the length is known). The empty list returns the seed;
/// otherwise the last element's returner `F Acc` IS the fold result (no
/// trailing bind).
#[inline]
#[must_use]
fn native_reduce(args: &[Rc<Value>]) -> Comp
{
    let Some((closure, seed, list)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let Some((final_item, prefix)) = items.split_last()
    else {
        return Comp::ret(seed.clone());
    };
    // The accumulator entering fold step `i`: the seed for `i = 0`, else the
    // value bound by the previous step (the `acc` binder `i - 1`).
    let acc = |index: usize| -> Value {
        if index == 0 {
            seed.clone()
        }
        else {
            Value::var(&gen_binder("acc", index.saturating_sub(1)))
        }
    };
    let step = |acc_val: Value, item: &Rc<Value>| -> Comp {
        Comp::app(
            Comp::app(Comp::force(closure.clone()), acc_val),
            (**item).clone(),
        )
    };
    let mut body = step(acc(prefix.len()), final_item);
    for (index, item) in prefix.iter().enumerate().rev() {
        body = Comp::bind(step(acc(index), item), &gen_binder("acc", index), body);
    }
    body
}

/// `any p xs` — whether the pure predicate `p` holds for some element,
/// short-circuiting on the first `true`.
#[inline]
#[must_use]
fn native_any(args: &[Rc<Value>]) -> Comp
{
    let Some((pred, list)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    quantify_chain(pred, &items, ShortCircuitFlag::from(true))
}

/// `all p xs` — whether the pure predicate `p` holds for every element,
/// short-circuiting on the first `false`.
#[inline]
#[must_use]
fn native_all(args: &[Rc<Value>]) -> Comp
{
    let Some((pred, list)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    quantify_chain(pred, &items, ShortCircuitFlag::from(false))
}

/// Builds the short-circuiting quantifier chain shared by [`native_any`] /
/// [`native_all`]. `short` is the boolean that short-circuits (`true` for
/// `any`, `false` for `all`); the empty list yields `!short` (the identity).
/// Built right-to-left with an explicit loop — NOT host recursion of depth =
/// list length (which would overflow the Rust stack on a large manifest list
/// — a review finding).
#[inline]
#[must_use]
fn quantify_chain(
    pred: &Value,
    items: &[Rc<Value>],
    short: ShortCircuitFlag,
) -> Comp
{
    // The `inl` arm is `true`, `inr` is `false`; for `any` a `true`
    // short-circuits (returns `true`) and a `false` falls through to the
    // tail, and dually for `all`.
    let mut body = ret_bool(short.identity_value());
    for (index, item) in items.iter().enumerate().rev() {
        let test = Comp::app(Comp::force(pred.clone()), (**item).clone());
        let binder = gen_binder("q", index);
        let (on_true, on_false) = if bool::from(short) {
            (ret_bool(true), body)
        }
        else {
            (body, ret_bool(false))
        };
        let arm = Comp::case(Value::var(&binder), "$qp", on_true, "$qq", on_false);
        body = Comp::bind(test, &binder, arm);
    }
    body
}

/// `flatten xs` — concatenate a manifest list of manifest lists into one list.
/// Pure and fully manifest, so it reduces in one step; a non-list element
/// degenerates to a gradual hole.
#[inline]
#[must_use]
fn native_flatten(args: &[Rc<Value>]) -> Comp
{
    let Some(list) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(outer) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let mut flat: Vec<Rc<Value>> = Vec::new();
    for inner in &outer {
        match as_list(inner.as_ref()) {
            | Some(elements) => flat.extend(elements),
            | None => return Comp::hole(0),
        }
    }
    Comp::ret(Value::List(flat))
}

/// `uniq xs` — drop later structural duplicates of a manifest list, keeping the
/// first occurrence. Equality is the derived structural `Value` equality (so a
/// bare `1` and an annotated `(1 : Integer)` are distinct — the MVP notion).
#[inline]
#[must_use]
fn native_uniq(args: &[Rc<Value>]) -> Comp
{
    let Some(list) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let mut seen: Vec<Rc<Value>> = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    Comp::ret(Value::List(seen))
}

/// `sort xs` — sort a manifest list of homogeneous, totally-ordered atoms
/// (integer literals, the integer numeric primitives, or strings). A
/// non-orderable element (a float — no total order over `NaN` — a pair, a
/// list, …) or a heterogeneous list degenerates to a gradual hole.
#[inline]
#[must_use]
fn native_sort(args: &[Rc<Value>]) -> Comp
{
    let Some(list) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    if items.len() <= 1 {
        return Comp::ret(Value::List(items));
    }
    let Some(first) = items.first()
    else {
        return Comp::hole(0);
    };
    let Some(kind) = order_kind(first.as_ref())
    else {
        return Comp::hole(0);
    };
    if !items
        .iter()
        .all(|item| order_kind(item.as_ref()) == Some(kind))
    {
        return Comp::hole(0);
    }
    let mut sorted = items;
    sorted.sort_by(|lhs, rhs| order_cmp(kind, lhs.as_ref(), rhs.as_ref()));
    Comp::ret(Value::List(sorted))
}

/// The [`OrderKind`] of a value, or `None` for a non-orderable atom (`sort`
/// rejects the whole list on any `None`).
#[inline]
#[must_use]
fn order_kind(value: &Value) -> Option<OrderKind>
{
    match unannotated(value) {
        | Value::Int(_) => Some(OrderKind::Int),
        | Value::Str(_) => Some(OrderKind::Str),
        | Value::Num(NumLit::U32(_)) => Some(OrderKind::U32),
        | Value::Num(NumLit::U64(_)) => Some(OrderKind::U64),
        | Value::Num(NumLit::I32(_)) => Some(OrderKind::I32),
        | Value::Num(NumLit::I64(_)) => Some(OrderKind::I64),
        | _ => None,
    }
}

/// The total order over two values known to share [`OrderKind`] `kind`; a
/// value that does not match (unreachable after the homogeneity check)
/// compares [`Ordering::Equal`], keeping the comparator total.
#[inline]
#[must_use]
fn order_cmp(
    kind: OrderKind,
    lhs: &Value,
    rhs: &Value,
) -> Ordering
{
    match (kind, unannotated(lhs), unannotated(rhs)) {
        | (OrderKind::Int, Value::Int(lhs), Value::Int(rhs)) => lhs.cmp(&rhs),
        | (OrderKind::Str, Value::Str(lhs), Value::Str(rhs)) => lhs.cmp(&rhs),
        | (OrderKind::U32, Value::Num(NumLit::U32(lhs)), Value::Num(NumLit::U32(rhs))) => {
            lhs.cmp(&rhs)
        },
        | (OrderKind::U64, Value::Num(NumLit::U64(lhs)), Value::Num(NumLit::U64(rhs))) => {
            lhs.cmp(&rhs)
        },
        | (OrderKind::I32, Value::Num(NumLit::I32(lhs)), Value::Num(NumLit::I32(rhs))) => {
            lhs.cmp(&rhs)
        },
        | (OrderKind::I64, Value::Num(NumLit::I64(lhs)), Value::Num(NumLit::I64(rhs))) => {
            lhs.cmp(&rhs)
        },
        | _ => Ordering::Equal,
    }
}

/// `get r ℓ` — look the dynamic string label `ℓ` up in the manifest record `r`,
/// returning `Some v` when the field is present and `None` otherwise. A
/// non-record subject or non-string label degenerates to a gradual hole.
#[inline]
#[must_use]
fn native_get(args: &[Rc<Value>]) -> Comp
{
    let Some((record, label)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(fields), Some(key)) = (as_record(record), as_str(label))
    else {
        return Comp::hole(0);
    };
    match fields.get(&key) {
        | Some(field) => Comp::ret(option_some((**field).clone())),
        | None => Comp::ret(option_none()),
    }
}

/// `insert r ℓ v` — extend / override the field `ℓ` of the manifest record `r`
/// with `v`, returning the extended record. A non-record subject or non-string
/// label degenerates to a gradual hole.
#[inline]
#[must_use]
fn native_insert(args: &[Rc<Value>]) -> Comp
{
    let Some((record, label, value)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut fields), Some(key)) = (as_record(record), as_str(label))
    else {
        return Comp::hole(0);
    };
    fields.insert(key, Rc::new(value.clone()));
    Comp::ret(Value::Record(fields))
}

/// Reads a manifest record value (the label→value map), ignoring annotations.
#[inline]
#[must_use]
fn as_record(value: &Value) -> Option<BTreeMap<String, Rc<Value>>>
{
    match unannotated(value) {
        | Value::Record(fields) => Some(fields),
        | _ => None,
    }
}

/// `recordupdate r o` — overlay the manifest overrides record `o` onto the
/// manifest base record `r`, returning a **fresh** record: override labels win,
/// labels present only in `r` survive, labels present only in `o` extend. A
/// non-record base or overrides degenerates to a gradual hole. The result is a
/// new [`Value::Record`] built from cloned fields, so the base binding observes
/// no change (the state-visibility red line).
#[inline]
#[must_use]
fn native_record_update(args: &[Rc<Value>]) -> Comp
{
    let Some((base, overrides)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut fields), Some(overlay)) = (as_record(base), as_record(overrides))
    else {
        return Comp::hole(0);
    };
    fields.extend(overlay);
    Comp::ret(Value::Record(fields))
}

/// `set xs i v` — replace the element at index `i` with `v`, returning a fresh
/// list. `get_mut` keeps the index bounds-checked without a partial index.
#[inline]
#[must_use]
fn native_set(args: &[Rc<Value>]) -> Comp
{
    let Some((list, index, value)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut items), Some(at)) = (as_list(list), as_index(index))
    else {
        return Comp::hole(0);
    };
    match items.get_mut(usize::from(at)) {
        | Some(slot) => {
            *slot = Rc::new(value.clone());
            Comp::ret(Value::List(items))
        },
        | None => Comp::hole(0),
    }
}

/// `update-at xs i f` — apply the pure closure `f` to the element at index `i`,
/// returning a fresh list. Unrolls to `force(f) xs[i] >>= $r. ret [ …, $r, … ]`
/// with the original elements except position `i`, which becomes the bound
/// result variable.
#[inline]
#[must_use]
fn native_update_at(args: &[Rc<Value>]) -> Comp
{
    let Some((list, index, closure)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut items), Some(at)) = (as_list(list), as_index(index))
    else {
        return Comp::hole(0);
    };
    let binder = gen_binder("uat", 0);
    let Some(target) = items.get_mut(usize::from(at))
    else {
        return Comp::hole(0);
    };
    let element = (**target).clone();
    *target = Rc::new(Value::var(&binder));
    let call = Comp::app(Comp::force(closure.clone()), element);
    Comp::bind(call, &binder, Comp::ret(Value::List(items)))
}

/// The totally-ordered atom kinds `sort` accepts — the integer literal, the
/// four integer numeric primitives, and the string atom. Floats are excluded
/// deliberately (IEEE `NaN` has no total order).
#[derive(Clone, Copy, Eq, PartialEq)]
enum OrderKind
{
    /// A bare integer literal ([`Value::Int`]).
    Int,
    /// A `u32` numeric primitive.
    U32,
    /// A `u64` numeric primitive.
    U64,
    /// An `i32` numeric primitive.
    I32,
    /// An `i64` numeric primitive.
    I64,
    /// A string atom ([`Value::Str`]).
    Str,
}

/// A collision-free generated binder `${prefix}{index}` (see the section
/// comment on the `$` sigil).
#[inline]
#[must_use]
fn gen_binder<'source, P, I>(
    prefix: P,
    index: I,
) -> String
where
    P: Into<NameRef<'source>>,
    I: Into<ListIndex>,
{
    let prefix = prefix.into();
    let index = usize::from(index.into());
    let mut name = String::from("$");
    name.push_str(prefix.as_ref());
    name.push_str(&index.to_string());
    name
}

/// `insert-at xs i v` — insert `v` before index `i` (`i == len` appends),
/// returning a fresh, longer list. An index past the end is a gradual hole.
#[inline]
#[must_use]
fn native_insert_at(args: &[Rc<Value>]) -> Comp
{
    let Some((list, index, value)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut items), Some(at)) = (as_list(list), as_index(index))
    else {
        return Comp::hole(0);
    };
    if at > ListIndex::from(items.len()) {
        return Comp::hole(0);
    }
    items.insert(usize::from(at), Rc::new(value.clone()));
    Comp::ret(Value::List(items))
}

/// `remove-at xs i` — drop the element at index `i`, returning a fresh, shorter
/// list. Out-of-bounds is a gradual hole.
#[inline]
#[must_use]
fn native_remove_at(args: &[Rc<Value>]) -> Comp
{
    let Some((list, index)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(mut items), Some(at)) = (as_list(list), as_index(index))
    else {
        return Comp::hole(0);
    };
    if at >= ListIndex::from(items.len()) {
        return Comp::hole(0);
    }
    let _removed = items.remove(usize::from(at));
    Comp::ret(Value::List(items))
}

/// Reads a manifest list value (the elements in order), ignoring annotations.
#[inline]
#[must_use]
fn as_list(value: &Value) -> Option<Vec<Rc<Value>>>
{
    match unannotated(value) {
        | Value::List(items) => Some(items),
        | _ => None,
    }
}

/// `push xs v` — append `v` to the end of the list, returning a fresh, longer
/// list.
#[inline]
#[must_use]
fn native_push(args: &[Rc<Value>]) -> Comp
{
    let Some((list, value)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(mut items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    items.push(Rc::new(value.clone()));
    Comp::ret(Value::List(items))
}

// ─── list functional-update builtins ────────────────────────────
//
// Each returns a FRESH `Value::List` (no lvalue, no in-place index assignment):
// the functional-update surface of `proposal-value-semantics-mvp.md` §3.2. The
// pure ones (`set` / `insert-at` / `remove-at` / `push`) reduce in one step
// over the manifest list; the higher-order ones (`update-at` / `update-where`)
// apply a gandr closure, so — like `each` / `where` — they UNROLL over the
// manifest list into a closed CBPV term the CEK machine runs (there is no core
// recursion). An out-of-bounds index or a non-manifest list degenerates to a
// gradual hole, so `apply` stays total. The generated binders carry a `$` sigil
// the surface identifier grammar cannot produce, so they never capture a source
// variable.

/// `update-where p f xs` — apply the pure closure `f` to every element the pure
/// predicate `p` accepts, keeping the rest, preserving order, returning a fresh
/// list. Built right-to-left like [`native_where`]: each level forces the
/// predicate, binds the processed tail ONCE, then branches — the `keep` arm
/// applies `f` and conses its result, the `drop` arm conses the element
/// unchanged. Binding the tail (not inlining it into both arms) keeps the
/// substitution-based unroll linear rather than Θ(2ⁿ) (the `where` lesson).
#[inline]
#[must_use]
fn native_update_where(args: &[Rc<Value>]) -> Comp
{
    let Some((pred, func, list)) = three_args(args)
    else {
        return Comp::hole(0);
    };
    let Some(items) = as_list(list)
    else {
        return Comp::hole(0);
    };
    let mut body = Comp::ret(Value::List(Vec::new()));
    for (index, item) in items.iter().enumerate().rev() {
        let element = (**item).clone();
        let test = Comp::app(Comp::force(pred.clone()), element.clone());
        let test_binder = gen_binder("uwt", index);
        let tail_binder = gen_binder("uwys", index);
        let result_binder = gen_binder("uwr", index);
        // The `keep` arm applies `f` to the element and conses the result onto
        // the (once-bound) processed tail; the `drop` arm conses the element
        // unchanged. Both reference the bound tail — never inline it.
        let transformed = Comp::bind(
            Comp::app(Comp::force(func.clone()), element.clone()),
            &result_binder,
            native_cons(Value::var(&result_binder), Value::var(&tail_binder)),
        );
        let kept = native_cons(element, Value::var(&tail_binder));
        let decide = Comp::case(Value::var(&test_binder), "$uwp", transformed, "$uwq", kept);
        body = Comp::bind(test, &test_binder, Comp::bind(body, &tail_binder, decide));
    }
    body
}

/// Borrows exactly three native arguments.
#[inline]
#[must_use]
fn three_args(args: &[Rc<Value>]) -> Option<(&Value, &Value, &Value)>
{
    let mut iter = args.iter();
    let first = iter.next()?;
    let second = iter.next()?;
    let third = iter.next()?;
    iter.next()
        .is_none()
        .then_some((first.as_ref(), second.as_ref(), third.as_ref()))
}

/// `escape s` — quote regex metacharacters in `s` for literal matching.
#[inline]
#[must_use]
fn native_string_escape(args: &[Rc<Value>]) -> Comp
{
    let Some(value) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(value) = as_str(value)
    else {
        return Comp::hole(0);
    };
    Comp::ret(Value::Str(escape_regex_literal(&value)))
}

/// Escapes regex metacharacters so the result matches `value` literally.
#[inline]
#[must_use]
fn escape_regex_literal<'source, T>(value: T) -> String
where
    T: Into<RegexLiteralText<'source>>,
{
    let value = value.into();
    let value = value.as_ref();
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(
            ch,
            '\\' | '.'
                | '+'
                | '*'
                | '?'
                | '('
                | ')'
                | '|'
                | '['
                | ']'
                | '{'
                | '}'
                | '^'
                | '$'
                | '#'
                | '&'
                | '-'
                | '~'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Applies a binary string predicate to two manifest strings.
#[inline]
#[must_use]
fn native_string_predicate<P>(
    args: &[Rc<Value>],
    pred: P,
) -> Comp
where
    P: FnOnce(&str, &str) -> bool,
{
    let Some((haystack, needle)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(haystack), Some(needle)) = (as_str(haystack), as_str(needle))
    else {
        return Comp::hole(0);
    };
    ret_bool(PrimitivePredicate::from(pred(
        haystack.as_str(),
        needle.as_str(),
    )))
}

/// `split s sep` — split a manifest string on a literal manifest separator.
#[inline]
#[must_use]
fn native_string_split(args: &[Rc<Value>]) -> Comp
{
    let Some((haystack, separator)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(haystack), Some(separator)) = (as_str(haystack), as_str(separator))
    else {
        return Comp::hole(0);
    };
    let parts = haystack
        .split(separator.as_str())
        .map(Value::string)
        .map(Rc::new)
        .collect();
    Comp::ret(Value::List(parts))
}

/// Reads a manifest string value (a dynamic record label), ignoring
/// annotations.
#[inline]
#[must_use]
fn as_str(value: &Value) -> Option<String>
{
    match unannotated(value) {
        | Value::Str(label) => Some(label),
        | _ => None,
    }
}

/// `extract pattern haystack` — return the first match's named captures as a
/// record. Without the `regex` feature, this total fallback is a gradual hole.
#[cfg(feature = "gandr_feat_regex")]
#[inline]
#[must_use]
fn native_regex_extract(args: &[Rc<Value>]) -> Comp
{
    let Some((pattern, haystack)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(pattern), Some(haystack)) = (as_str(pattern), as_str(haystack))
    else {
        return Comp::hole(0);
    };
    let Ok(regex) = Regex::new(&pattern)
    else {
        return Comp::hole(0);
    };
    let Some(captures) = regex.captures(&haystack)
    else {
        return Comp::hole(0);
    };
    let fields = regex
        .capture_names()
        .flatten()
        .filter_map(|name| {
            captures
                .name(name)
                .map(|value| (name.to_owned(), Rc::new(Value::string(value.as_str()))))
        })
        .collect();
    Comp::ret(Value::Record(fields))
}

/// `extract pattern haystack` — the regex-less total fallback: always a
/// gradual hole, since the native op is unavailable without the `regex`
/// feature.
#[cfg(not(feature = "gandr_feat_regex"))]
#[inline]
#[must_use]
fn native_regex_extract(_args: &[Rc<Value>]) -> Comp
{
    Comp::hole(0)
}

/// `join base child` — pure UTF-8 path join over manifest strings.
#[inline]
#[must_use]
fn native_path_join(args: &[Rc<Value>]) -> Comp
{
    let Some((base, child)) = two_args(args)
    else {
        return Comp::hole(0);
    };
    let (Some(base), Some(child)) = (as_str(base), as_str(child))
    else {
        return Comp::hole(0);
    };
    path_to_string(Path::new(&base).join(&child).as_path())
}

/// Borrows exactly two native arguments.
#[inline]
#[must_use]
fn two_args(args: &[Rc<Value>]) -> Option<(&Value, &Value)>
{
    let mut iter = args.iter();
    let lhs = iter.next()?;
    let rhs = iter.next()?;
    iter.next()
        .is_none()
        .then_some((lhs.as_ref(), rhs.as_ref()))
}

/// `basename path` — final UTF-8 path component.
#[inline]
#[must_use]
fn native_path_basename(args: &[Rc<Value>]) -> Comp
{
    let Some(path) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(path) = as_str(path)
    else {
        return Comp::hole(0);
    };
    match Path::new(&path).file_name().and_then(|name| name.to_str()) {
        | Some(name) => Comp::ret(Value::string(name)),
        | None => Comp::hole(0),
    }
}

/// `extension path` — final UTF-8 extension without the dot.
#[inline]
#[must_use]
fn native_path_extension(args: &[Rc<Value>]) -> Comp
{
    let Some(path) = one_arg(args)
    else {
        return Comp::hole(0);
    };
    let Some(path) = as_str(path)
    else {
        return Comp::hole(0);
    };
    match Path::new(&path).extension().and_then(|name| name.to_str()) {
        | Some(name) => Comp::ret(Value::string(name)),
        | None => Comp::hole(0),
    }
}

/// Borrows exactly one native argument.
#[inline]
#[must_use]
fn one_arg(args: &[Rc<Value>]) -> Option<&Value>
{
    let mut iter = args.iter();
    let value = iter.next()?;
    iter.next().is_none().then_some(value.as_ref())
}

/// Clones a value with any outer type annotations stripped.
#[inline]
#[must_use]
/// # Termination
/// - reason: primitive helpers consume finite arguments or pattern text.
/// - measure: remaining arguments, path components, or pattern characters.
/// - boundedness: primitive inputs are finite Rust values.
/// - input recursion: none.
fn unannotated(value: &Value) -> Value
{
    let mut current = value;
    while let Value::Annot(ref inner, _) = *current {
        current = inner.as_ref();
    }
    current.clone()
}

/// Converts a Rust path to a gandr string, or a hole when it is not UTF-8.
#[inline]
#[must_use]
fn path_to_string(path: &Path) -> Comp
{
    match path.to_str() {
        | Some(path) => Comp::ret(Value::string(path)),
        | None => Comp::hole(0),
    }
}
