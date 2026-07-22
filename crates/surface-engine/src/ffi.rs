//! Foreign-function-interface declarations, extracted from `extern` blocks by
//! the lowerer (proposal-ffi.md §2, §4).
//!
//! An `extern "c" from "m" { … }` block declares a **foreign module**: a set of
//! opaque handle types (`type Db;`, §4.4) and bodyless foreign-function
//! signatures (`def cos(x: f64) -> f64;`, §2). The lowerer builds one
//! [`ForeignModule`] per block, elaborates each call `m.op(args)` to a
//! `perform m.op {payload}` against the module's [`ForeignModule::effect_sig`]
//! (§3.1 — "a foreign call is an effect op"), and surfaces the declarations on
//! [`crate::lower::Lowered::foreign`] so a native handler (`gandr-ffi`) can
//! resolve the C symbols and marshal the boundary.
//!
//! Only the MVP boundary type mapping (§4) is modelled: the six numeric atoms
//! (identity map, §4.1), `CStr` (a NUL-terminated `String` copy, §4.2), an
//! opaque pointer handle (§4.4, `u64`-carried in the MVP), and a `void` return.
//! Struct-by-value / `sret`, the sub-word integers, and the distinct
//! foreign-handle value node are named growth items (§7), out of the MVP.
//!
//! These declarations are the shared surface the interpreter execution path
//! (§5.1) consumes; they carry the C-ABI metadata (the [`CType`]s) that the
//! inline-carried [`EffectSig`] deliberately does not, so a handler keyed only
//! on the signature name and op still knows each argument's C type and the
//! library to resolve the symbol from.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::types::ValueType;

use crate::boundary::ForeignOperation;

/// A C-ABI boundary type (proposal-ffi.md §4): the C-level shape of one foreign
/// argument, one foreign result, or a result-less call.
///
/// The MVP boundary (§4.1 / §4.2 / §4.4): the six numeric atoms map by identity
/// to the fixed-width C scalars; `CStr` is a NUL-terminated copy of a gandr
/// `String`; `Ptr` is an opaque address-sized handle; `Void` is a result-less
/// call. Sub-word integers, `bool`/`c_char`, and struct-by-value are growth
/// items (§4.1 / §4.3), not represented here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CType
{
    /// `uint32_t` — the `u32` numeric atom (identity map, §4.1).
    U32,
    /// `uint64_t` — the `u64` numeric atom (identity map, §4.1).
    U64,
    /// `int32_t` — the `i32` numeric atom (identity map, §4.1).
    I32,
    /// `int64_t` — the `i64` numeric atom (identity map, §4.1).
    I64,
    /// `float` — the `f32` numeric atom (identity map, §4.1).
    F32,
    /// `double` — the `f64` numeric atom (identity map, §4.1).
    F64,
    /// `char*` — a NUL-terminated copy of a gandr `String`, freed after the
    /// call (§4.2). Arguments are borrowed-in; C must not retain them.
    CStr,
    /// An opaque address-sized handle / pointer (§4.4). MVP-carried as `u64`;
    /// the distinct foreign-handle value node with provenance is frozen-core
    /// growth (§4.4).
    Ptr,
    /// A result-less foreign call (`void`), replying `Unit`.
    Void,
}

impl CType
{
    /// The gandr value type a boundary value of this C type carries at the
    /// surface (proposal-ffi.md §4).
    ///
    /// # Contract
    /// - ensures: the six numeric atoms map to their rigid atom (identity map,
    ///   §4.1); `CStr` to `String` (the gandr value that crosses, §4.2); `Ptr`
    ///   to `u64` (the MVP handle carrier, §4.4); `Void` to `Unit` (a
    ///   result-less reply). This is the type the elaborated `perform`'s
    ///   payload field or reply is checked at.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn boundary_type(self) -> ValueType
    {
        match self {
            | Self::U32 => ValueType::u32(),
            | Self::I32 => ValueType::i32(),
            | Self::I64 => ValueType::i64(),
            | Self::F32 => ValueType::f32(),
            | Self::F64 => ValueType::f64(),
            | Self::CStr => ValueType::string(),
            // The MVP opaque handle is carried as `u64` (§4.4); the distinct
            // foreign-handle value node with provenance is frozen-core growth.
            | Self::U64 | Self::Ptr => ValueType::u64(),
            | Self::Void => ValueType::Unit,
        }
    }
}

/// One foreign-function parameter: its surface name and its C boundary type.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ForeignParam
{
    /// The parameter name (the record label the argument crosses under).
    pub name: String,
    /// The parameter's C boundary type.
    pub c_type: CType,
}

/// One declared foreign function `def op(params) -> result;` (proposal-ffi.md
/// §2). In the MVP the operation name is also the resolved C symbol name
/// (§5.1).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ForeignFn
{
    /// The operation / C symbol name.
    pub op: String,
    /// The parameters, in declaration order.
    pub params: Vec<ForeignParam>,
    /// The result boundary type (`Void` for a result-less signature).
    pub result: CType,
}

impl ForeignFn
{
    /// The effect operation `op : {params} ↠ result` this foreign function
    /// elaborates to (proposal-ffi.md §3.1).
    ///
    /// # Contract
    /// - ensures: the payload is the argument record keyed by parameter name
    ///   (each field at its [`CType::boundary_type`]); the reply is the result
    ///   boundary type. A call `m.op(args)` performs this op with the matching
    ///   record.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn effect_op(&self) -> EffectOp
    {
        let payload = ValueType::record(
            self.params
                .iter()
                .map(|param| (param.name.clone(), param.c_type.boundary_type())),
        );
        EffectOp::new((&self.op).into(), payload, self.result.boundary_type())
    }
}

/// A foreign module — one `extern "abi" from "library" { … }` block
/// (proposal-ffi.md §2), binding its members as the FFI contract module
/// members under the namespace named by its `library` string.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ForeignModule
{
    /// The module namespace (`m` in `m.cos`), taken from the `library` string;
    /// also the [`EffectSig::name`] the native handler dispatches on.
    pub name: String,
    /// The ABI string (`"c"` in the MVP; the slot where `"c-unwind"` / `"wasm"`
    /// grow, §2).
    pub abi: String,
    /// The library the symbols resolve from — the `dlopen`/`dlsym` target of
    /// the interpreter path (§5.1).
    pub library: String,
    /// The opaque handle types declared in the block (`type Db;`, §4.4).
    pub types: Vec<String>,
    /// The foreign functions declared in the block.
    pub functions: Vec<ForeignFn>,
}

impl ForeignModule
{
    /// The per-library effect signature (proposal-ffi.md §3.1): one operation
    /// per foreign function, named by the module, that a call `m.op` performs
    /// and the native FFI handler answers.
    ///
    /// # Contract
    /// - ensures: an [`EffectSig`] named [`Self::name`] with one [`EffectOp`]
    ///   per declared function (its [`ForeignFn::effect_op`]).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn effect_sig(&self) -> EffectSig
    {
        EffectSig::new(
            (&self.name).into(),
            self.functions.iter().map(ForeignFn::effect_op).collect(),
        )
    }

    /// Looks up a declared foreign function by operation name.
    ///
    /// # Contract
    /// - ensures: the function named `op` if the module declares it, else
    ///   `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn function<'operation, O: Into<ForeignOperation<'operation>>>(
        &self,
        operation: O,
    ) -> Option<&ForeignFn>
    {
        let operation = operation.into();
        self.functions
            .iter()
            .find(|function| function.op == operation.0)
    }
}
