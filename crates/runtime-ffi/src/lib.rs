//! Least-authority native C ABI execution over the shared host-effect seam.
//!
//! [`FfiHost`] is constructed from the lowered foreign-module declarations. It
//! loads only those libraries, resolves only those declared operation names,
//! marshals only the declared boundary types, and returns a [`FfiAction`] for
//! the composing driver. No foreign pointer or borrowed C string leaves the
//! call boundary.
//!
//! The hermetic fixture and fixture-backed tests are enabled with
//! `cargo test -p gandr-runtime-ffi --features native-fixture`; default builds
//! do not compile the fixture.
extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::ffi::CString;
use std::ffi::CStr;
use std::ffi::c_char;
use std::ffi::c_void;
use std::path::Path;
use std::path::PathBuf;
use std::ptr;

use gandr_core_checker::boundary::OperationName;
use gandr_core_checker::effect;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::Value;
use gandr_runtime_effects::HostAction;
use gandr_runtime_effects::ShellHandler;
use gandr_surface_engine::ffi::CType;
use gandr_surface_engine::ffi::ForeignFn;
use gandr_surface_engine::ffi::ForeignModule;
use libffi::middle::Arg;
use libffi::middle::Cif;
use libffi::middle::CodePtr;
use libffi::middle::Type;
use libloading::Library;
use thiserror::Error;

/// Returns the build-script-produced path to the hermetic native test library.
///
/// This fixture locator is used by integration tests that need to copy the
/// exact library built for the current target.
#[cfg(feature = "native-fixture")]
#[inline]
#[must_use]
pub fn hermetic_testlib_path() -> PathBuf
{
    PathBuf::from(env!("GANDR_TESTLIB"))
}

/// A native boundary failure that remains inside the FFI driver.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum FfiError
{
    /// A declared library could not be loaded.
    #[error("ffi library `{library}` could not be loaded: {detail}")]
    Library
    {
        /// The requested library path.
        library: String,
        /// The loader's detail.
        detail: String,
    },
    /// A declared operation has no matching symbol in its library.
    #[error("ffi symbol `{symbol}` in `{library}` could not be loaded: {detail}")]
    Symbol
    {
        /// The requested symbol.
        symbol: String,
        /// The library containing the symbol.
        library: String,
        /// The loader's detail.
        detail: String,
    },
    /// A host payload did not match its declared foreign signature.
    #[error("ffi {signature}::{operation}: malformed payload: {detail}")]
    Marshal
    {
        /// The foreign signature name.
        signature: String,
        /// The operation name.
        operation: String,
        /// The violated boundary rule.
        detail: String,
    },
    /// A returned pointer was not a valid copied C string.
    #[error("ffi {signature}::{operation}: invalid returned C string: {detail}")]
    ReturnString
    {
        /// The foreign signature name.
        signature: String,
        /// The operation name.
        operation: String,
        /// The boundary failure.
        detail: String,
    },
}

/// The native dispatch verdict used by a composing driver.
#[derive(Debug)]
pub enum FfiAction
{
    /// Resume the machine with the foreign result.
    Resume(Value),
    /// Abort the run with a typed foreign boundary error.
    Fail(FfiError),
    /// The signature or operation is outside this host's authority.
    Decline,
}

/// A loaded foreign module and its live library handle.
struct LoadedModule
{
    /// The source declaration that authorized this module.
    declaration: ForeignModule,
    /// The live loader handle that owns resolved symbols.
    library: Library,
}

/// A least-authority C ABI handler built from declared foreign modules.
///
/// # Contract
/// - requires: each module uses the supported `c` ABI and names a loadable
///   dynamic library; each operation's declaration exactly describes its C ABI.
/// - ensures: only declared `(signature, operation)` pairs are resolved and
///   called; all arguments are copied or passed as declared and every result is
///   converted into an owned [`Value`].
/// - provides: [`FfiAction::Resume`] for a successful call and typed failure or
///   decline outcomes at the host boundary.
/// - fails: library, symbol, payload, and returned-string failures are reported
///   as [`FfiError`] values.
/// - panics: none.
/// - unsafe invariants: the loaded [`Library`] is retained for the lifetime of
///   this handler; the unsafe call is made only after exact declaration-based
///   type construction and all argument storage remains alive through it.
#[repr(transparent)]
pub struct FfiHost
{
    /// Loaded modules authorized by the source declarations.
    modules: Vec<LoadedModule>,
}

impl FfiHost
{
    /// Loads the declared foreign modules and no others.
    ///
    /// # Errors
    /// Returns [`FfiError::Library`] when a declared library cannot be loaded,
    /// or [`FfiError::Marshal`] for an unsupported ABI or duplicate module.
    #[inline]
    pub fn new(modules: Vec<ForeignModule>) -> Result<Self, FfiError>
    {
        let mut names = BTreeSet::new();
        let mut loaded = Vec::with_capacity(modules.len());
        for declaration in modules {
            let signature = declaration.name.clone();
            if !names.insert(signature.clone()) {
                return Err(FfiError::Marshal {
                    signature,
                    operation: String::new(),
                    detail: "duplicate foreign module".to_owned(),
                });
            }
            if declaration.abi != "c" {
                return Err(FfiError::Marshal {
                    signature: declaration.name,
                    operation: String::new(),
                    detail: format!("unsupported ABI `{}`", declaration.abi),
                });
            }
            let library = unsafe {
                // SAFETY: the loader owns the handle, and the handle is kept in
                // `LoadedModule` until every symbol call through this host ends.
                Library::new(declaration.library.as_str())
            }
            .map_err(|error| FfiError::Library {
                library: declaration.library.clone(),
                detail: error.to_string(),
            })?;
            loaded.push(LoadedModule {
                declaration,
                library,
            });
        }
        Ok(Self { modules: loaded })
    }

    /// Dispatches one host offer against the declared foreign registry.
    #[inline]
    #[must_use]
    pub fn dispatch(
        &self,
        op: &effect::host::HostOp,
    ) -> FfiAction
    {
        let Some(module) = self
            .modules
            .iter()
            .find(|module| module.declaration.name == op.sig.name().as_ref())
        else {
            return FfiAction::Decline;
        };
        let Some(function) = module.declaration.function(op.op.as_str())
        else {
            return FfiAction::Decline;
        };
        match Self::invoke(module, function, &op.payload) {
            | Ok(value) => FfiAction::Resume(value),
            | Err(error) => FfiAction::Fail(error),
        }
    }
    /// Invokes one declared foreign operation after validating its payload.
    fn invoke(
        module: &LoadedModule,
        function: &ForeignFn,
        payload: &Value,
    ) -> Result<Value, FfiError>
    {
        let fields = payload
            .as_record()
            .ok_or_else(|| Self::marshal(module, function, "expected an argument record"))?;
        if fields.len() != function.params.len() {
            return Err(Self::marshal(
                module,
                function,
                "argument record does not have declared arity",
            ));
        }
        let mut storage = Vec::with_capacity(function.params.len());
        for parameter in &function.params {
            let Some(value) = fields.get(&parameter.name)
            else {
                return Err(Self::marshal(
                    module,
                    function,
                    format!("missing field `{}`", parameter.name),
                ));
            };
            storage.push(Self::argument(module, function, parameter.c_type, value)?);
        }
        let types = function
            .params
            .iter()
            .map(|parameter| ffi_type(parameter.c_type))
            .collect::<Vec<_>>();
        let result_type = ffi_type(function.result);
        let cif = Cif::new(types, result_type);
        let arguments = storage.iter().map(ArgumentStorage::arg).collect::<Vec<_>>();
        let symbol = unsafe {
            // SAFETY: the symbol name is selected only from the declared
            // operation and the returned address is used with the exact CIF
            // assembled from the same declaration.
            module.library.get::<*mut c_void>(function.op.as_bytes())
        }
        .map_err(|error| FfiError::Symbol {
            symbol: function.op.clone(),
            library: module.declaration.library.clone(),
            detail: error.to_string(),
        })?;
        let code = CodePtr::from_ptr((*symbol).cast_const());
        call_result(&cif, code, &arguments, function.result, module, function)
    }
    /// Marshals one declared argument into owned call storage.
    fn argument(
        module: &LoadedModule,
        function: &ForeignFn,
        c_type: CType,
        value: &Value,
    ) -> Result<ArgumentStorage, FfiError>
    {
        let error = |detail| Self::marshal(module, function, detail);
        match c_type {
            | CType::U32 => match value_num(value) {
                | Some(NumLit::U32(value)) => Ok(ArgumentStorage::U32(value)),
                | _ => Err(error("expected u32")),
            },
            | CType::U64 => match value_num(value) {
                | Some(NumLit::U64(value)) => Ok(ArgumentStorage::U64(value)),
                | _ => Err(error("expected u64")),
            },
            | CType::I32 => match value_num(value) {
                | Some(NumLit::I32(value)) => Ok(ArgumentStorage::I32(value)),
                | _ => Err(error("expected i32")),
            },
            | CType::I64 => match value_num(value) {
                | Some(NumLit::I64(value)) => Ok(ArgumentStorage::I64(value)),
                | _ => Err(error("expected i64")),
            },
            | CType::F32 => match value_num(value) {
                | Some(NumLit::F32(bits)) => Ok(ArgumentStorage::F32(f32::from_bits(bits))),
                | _ => Err(error("expected f32")),
            },
            | CType::F64 => match value_num(value) {
                | Some(NumLit::F64(bits)) => Ok(ArgumentStorage::F64(f64::from_bits(bits))),
                | _ => Err(error("expected f64")),
            },
            | CType::CStr => {
                let text = value.as_str().ok_or_else(|| error("expected string"))?;
                let copied = CString::new(text.as_ref())
                    .map_err(|_error| error("string contains an interior NUL"))?;
                let pointer = copied.as_ptr();
                Ok(ArgumentStorage::CStr {
                    value: copied,
                    pointer,
                })
            },
            | CType::Ptr => {
                match value_num(value) {
                    | Some(NumLit::U64(value)) => Ok(ArgumentStorage::Ptr(
                        ptr::with_exposed_provenance_mut(usize::try_from(value).map_err(
                            |_error| error("pointer does not fit the host address width"),
                        )?),
                    )),
                    | _ => Err(error("expected u64 pointer handle")),
                }
            },
            | CType::Void => Err(error("void is valid only as a result type")),
        }
    }

    /// Builds a typed marshal error for one operation boundary.
    fn marshal(
        module: &LoadedModule,
        function: &ForeignFn,
        detail: impl Into<String>,
    ) -> FfiError
    {
        FfiError::Marshal {
            signature: module.declaration.name.clone(),
            operation: function.op.clone(),
            detail: detail.into(),
        }
    }
}

/// Owned argument storage kept alive during a native call.
#[derive(Debug)]
enum ArgumentStorage
{
    /// Unsigned 32-bit argument.
    U32(u32),
    /// Unsigned 64-bit argument.
    U64(u64),
    /// Signed 32-bit argument.
    I32(i32),
    /// Signed 64-bit argument.
    I64(i64),
    /// Single-precision argument.
    F32(f32),
    /// Double-precision argument.
    F64(f64),
    /// NUL-terminated string and stable pointer.
    CStr
    {
        /// Owned string storage.
        value: CString,
        /// Pointer into `value`.
        pointer: *const c_char,
    },
    /// Opaque pointer argument.
    Ptr(*mut c_void),
}

impl ArgumentStorage
{
    /// Converts owned storage into a libffi argument.
    fn arg(&self) -> Arg<'_>
    {
        match self {
            | &Self::U32(ref value) => libffi::middle::arg(value),
            | &Self::U64(ref value) => libffi::middle::arg(value),
            | &Self::I32(ref value) => libffi::middle::arg(value),
            | &Self::I64(ref value) => libffi::middle::arg(value),
            | &Self::F32(ref value) => libffi::middle::arg(value),
            | &Self::F64(ref value) => libffi::middle::arg(value),
            | &Self::CStr {
                ref value,
                ref pointer,
            } => {
                let _ = value;
                libffi::middle::arg(pointer)
            },
            | &Self::Ptr(ref value) => libffi::middle::arg(value),
        }
    }
}

/// Extracts a numeric literal without widening its declared type.
fn value_num(value: &Value) -> Option<NumLit>
{
    match value {
        | &Value::Num(number) => Some(number),
        | _ => None,
    }
}

/// Converts a declared boundary type to its libffi representation.
fn ffi_type(c_type: CType) -> Type
{
    match c_type {
        | CType::U32 => Type::u32(),
        | CType::U64 => Type::u64(),
        | CType::I32 => Type::i32(),
        | CType::I64 => Type::i64(),
        | CType::F32 => Type::f32(),
        | CType::F64 => Type::f64(),
        | CType::CStr | CType::Ptr => Type::pointer(),
        | CType::Void => Type::void(),
    }
}
/// Calls a declared C function through the exact CIF assembled for it.
fn call<R>(
    cif: &Cif,
    code: CodePtr,
    args: &[Arg<'_>],
) -> R
{
    // SAFETY: callers construct `cif`, `code`, and `args` from one declaration.
    unsafe { cif.call::<R>(code, args) }
}

/// Calls one symbol and converts its declared result into an owned value.
fn call_result(
    cif: &Cif,
    code: CodePtr,
    arguments: &[Arg<'_>],
    result: CType,
    module: &LoadedModule,
    function: &ForeignFn,
) -> Result<Value, FfiError>
{
    let signature = module.declaration.name.as_str();
    let operation = function.op.as_str();
    match result {
        | CType::U32 => Ok(Value::u32(call::<u32>(cif, code, arguments))),
        | CType::U64 => Ok(Value::u64(call::<u64>(cif, code, arguments))),
        | CType::I32 => Ok(Value::i32(call::<i32>(cif, code, arguments))),
        | CType::I64 => Ok(Value::i64(call::<i64>(cif, code, arguments))),
        | CType::F32 => Ok(Value::f32(call::<f32>(cif, code, arguments))),
        | CType::F64 => Ok(Value::f64(call::<f64>(cif, code, arguments))),
        | CType::Ptr => {
            let pointer: *const c_void = call(cif, code, arguments);
            let address = u64::try_from(pointer.addr()).map_err(|_error| FfiError::Marshal {
                signature: signature.to_owned(),
                operation: operation.to_owned(),
                detail: "returned pointer does not fit u64".to_owned(),
            })?;
            Ok(Value::u64(address))
        },
        | CType::CStr => {
            let pointer: *const c_void = call(cif, code, arguments);
            let pointer = pointer.cast::<c_char>();
            if pointer.is_null() {
                return Err(FfiError::ReturnString {
                    signature: signature.to_owned(),
                    operation: operation.to_owned(),
                    detail: "returned a null pointer".to_owned(),
                });
            }
            // SAFETY: the foreign declaration promises a non-null C string.
            let text = unsafe { CStr::from_ptr(pointer) }
                .to_str()
                .map_err(|_error| FfiError::ReturnString {
                    signature: signature.to_owned(),
                    operation: operation.to_owned(),
                    detail: "returned bytes are not UTF-8".to_owned(),
                })?;
            Ok(Value::string(text))
        },
        | CType::Void => Ok(Value::Unit),
    }
}

/// A combined FFI and shell run outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FfiShellOutcome
{
    /// The machine completed with a core evaluation.
    Completed(gandr_core_checker::outcome::Eval),
    /// The shell requested process termination.
    Exited
    {
        /// The process exit code.
        code: i64,
    },
    /// A shell operation failed.
    ShellFailed(gandr_runtime_effects::ShellError),
    /// A native operation failed.
    FfiFailed(FfiError),
}

/// Runs a hand-built computation with the declared FFI modules and shell host.
///
/// # Errors
///
/// Returns [`FfiError`] when a declared native library cannot be loaded.
#[inline]
pub fn run_program(
    comp: &gandr_core_checker::syntax::Comp,
    modules: Vec<ForeignModule>,
) -> Result<FfiShellOutcome, FfiError>
{
    run_program_with_prelude(comp, &[], modules)
}

/// Runs a computation with ambient bindings, declared FFI modules, and shell
/// host.
///
/// # Errors
///
/// Returns [`FfiError`] when a declared native library cannot be loaded.
#[inline]
pub fn run_program_with_prelude(
    comp: &gandr_core_checker::syntax::Comp,
    prelude: &[(String, Value)],
    modules: Vec<ForeignModule>,
) -> Result<FfiShellOutcome, FfiError>
{
    let host = FfiHost::new(modules)?;
    let mut driver = CombinedDriver {
        ffi: host,
        shell: ShellHandler::new(),
        early: None,
    };
    let eval =
        gandr_core_sequent::machine::run_comp_with_prelude_and_host(comp, prelude, &mut driver);
    Ok(driver.early.unwrap_or(FfiShellOutcome::Completed(eval)))
}

/// A source preparation or file-read failure for the native run entry.
#[derive(Debug, Error)]
pub enum FfiRunError
{
    /// The source file could not be read as UTF-8.
    #[error("cannot read `{path}`: {detail}", path = path.display())]
    Read
    {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying read failure.
        detail: String,
    },
    /// The source could not be lowered, linked, or type-checked.
    #[error(transparent)]
    Prepare(#[from] gandr_surface_engine::run::RunError),
}

/// Lowers, links, type-checks, and runs source under the combined native/shell
/// host.
///
/// # Errors
///
/// Returns [`FfiRunError`] when source preparation fails.
#[inline]
pub fn run_source<'source, S>(source: S) -> Result<FfiShellOutcome, FfiRunError>
where
    S: Into<gandr_surface_engine::boundary::PipelineSource<'source>>,
{
    let prepared = gandr_surface_engine::run::prepare_source(source)?;
    Ok(run_prepared(prepared))
}

/// Runs a prepared source through the combined native and shell hosts.
fn run_prepared(prepared: gandr_surface_engine::run::PreparedSource) -> FfiShellOutcome
{
    match run_program_with_prelude(
        &prepared.comp,
        prepared.prelude.as_bindings(),
        prepared.foreign,
    ) {
        | Ok(outcome) => outcome,
        | Err(error) => FfiShellOutcome::FfiFailed(error),
    }
}

/// Reads, prepares, and runs one source file under the combined native/shell
/// host.
///
/// # Errors
///
/// Returns [`FfiRunError`] when reading or source preparation fails.
#[inline]
pub fn run_source_file(path: &Path) -> Result<FfiShellOutcome, FfiRunError>
{
    let source = std::fs::read_to_string(path).map_err(|error| FfiRunError::Read {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    run_source(source.as_str())
}

/// Composes native FFI and shell handlers for one machine run.
struct CombinedDriver
{
    /// Native handler.
    ffi: FfiHost,
    /// Shell handler.
    shell: ShellHandler,
    /// Outcome that terminates evaluation early.
    early: Option<FfiShellOutcome>,
}

impl effect::host::HostHandler for CombinedDriver
{
    fn handle<'source, O>(
        &mut self,
        sig: &effect::EffectSig,
        op: O,
        payload: &Value,
    ) -> effect::host::HostReply
    where
        O: Into<OperationName<'source>>,
    {
        let host_op = effect::host::HostOp::new(sig.clone(), op.into(), payload.clone());
        match self.shell.dispatch(&host_op) {
            | HostAction::Resume(value) => effect::host::HostReply::Resume(value),
            | HostAction::Exit(code) => {
                self.early = Some(FfiShellOutcome::Exited { code });
                effect::host::HostReply::Unhandled
            },
            | HostAction::Fail(error) => {
                self.early = Some(FfiShellOutcome::ShellFailed(error));
                effect::host::HostReply::Unhandled
            },
            | HostAction::Decline => match self.ffi.dispatch(&host_op) {
                | FfiAction::Resume(value) => effect::host::HostReply::Resume(value),
                | FfiAction::Fail(error) => {
                    self.early = Some(FfiShellOutcome::FfiFailed(error));
                    effect::host::HostReply::Unhandled
                },
                | FfiAction::Decline => effect::host::HostReply::Unhandled,
            },
        }
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::effect::EffectSig;
    #[cfg(feature = "native-fixture")]
    use gandr_surface_engine::ffi::ForeignParam;

    use super::*;

    #[cfg(feature = "native-fixture")]
    fn module(function: ForeignFn) -> ForeignModule
    {
        ForeignModule {
            name: "testlib".to_owned(),
            abi: "c".to_owned(),
            library: hermetic_testlib_path().display().to_string(),
            types: Vec::new(),
            functions: vec![function],
        }
    }

    fn offer<'source, O>(
        operation: O,
        payload: Value,
    ) -> effect::host::HostOp
    where
        O: Into<OperationName<'source>>,
    {
        effect::host::HostOp::new(
            EffectSig::new("testlib".into(), vec![]),
            operation.into(),
            payload,
        )
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn native_i32_call_returns_declared_value()
    {
        let function = ForeignFn {
            op: "gandr_add_i32".to_owned(),
            params: vec![
                ForeignParam {
                    name: "left".to_owned(),
                    c_type: CType::I32,
                },
                ForeignParam {
                    name: "right".to_owned(),
                    c_type: CType::I32,
                },
            ],
            result: CType::I32,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([
            ("left".to_owned(), Value::i32(4_i32)),
            ("right".to_owned(), Value::i32(5_i32)),
        ]);
        assert!(matches!(
            host.dispatch(&offer("gandr_add_i32", payload)),
            FfiAction::Resume(Value::Num(NumLit::I32(9_i32)))
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn native_i64_call_returns_declared_value()
    {
        let function = ForeignFn {
            op: "gandr_test_add".to_owned(),
            params: vec![
                ForeignParam {
                    name: "left".to_owned(),
                    c_type: CType::I64,
                },
                ForeignParam {
                    name: "right".to_owned(),
                    c_type: CType::I64,
                },
            ],
            result: CType::I64,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([
            ("left".to_owned(), Value::i64(21)),
            ("right".to_owned(), Value::i64(21)),
        ]);
        assert!(matches!(
            host.dispatch(&offer("gandr_test_add", payload)),
            FfiAction::Resume(Value::Num(NumLit::I64(42)))
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn numeric_identity_operations_preserve_declared_types()
    {
        let cases = [
            (
                "gandr_identity_u32",
                CType::U32,
                Value::u32(7),
                NumLit::U32(7),
            ),
            (
                "gandr_identity_u64",
                CType::U64,
                Value::u64(11),
                NumLit::U64(11),
            ),
            (
                "gandr_identity_f32",
                CType::F32,
                Value::f32(1.5),
                NumLit::F32(1.5f32.to_bits()),
            ),
            (
                "gandr_identity_f64",
                CType::F64,
                Value::f64(2.5_f64),
                NumLit::F64(2.5f64.to_bits()),
            ),
        ];
        for (operation, c_type, input, expected) in cases {
            let function = ForeignFn {
                op: operation.to_owned(),
                params: vec![ForeignParam {
                    name: "value".to_owned(),
                    c_type,
                }],
                result: c_type,
            };
            let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
            let payload = Value::record([("value".to_owned(), input)]);
            assert!(matches!(
                host.dispatch(&offer(operation, payload)),
                FfiAction::Resume(Value::Num(actual)) if actual == expected
            ));
        }
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn copied_returned_c_string_is_a_value()
    {
        let function = ForeignFn {
            op: "gandr_greeting".to_owned(),
            params: Vec::new(),
            result: CType::CStr,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let result = host.dispatch(&offer(
            "gandr_greeting",
            Value::record(Vec::<(String, Value)>::new()),
        ));
        assert!(matches!(
            result,
            FfiAction::Resume(Value::Str(text)) if text.as_str() == "hello from testlib"
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn invalid_returned_c_string_is_a_typed_boundary_failure()
    {
        let function = ForeignFn {
            op: "gandr_invalid_string".to_owned(),
            params: Vec::new(),
            result: CType::CStr,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        assert!(matches!(
            host.dispatch(&offer(
                "gandr_invalid_string",
                Value::record(Vec::<(String, Value)>::new())
            )),
            FfiAction::Fail(FfiError::ReturnString { detail, .. })
                if detail == "returned bytes are not UTF-8"
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn void_result_is_unit()
    {
        let function = ForeignFn {
            op: "gandr_void".to_owned(),
            params: Vec::new(),
            result: CType::Void,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        assert!(matches!(
            host.dispatch(&offer(
                "gandr_void",
                Value::record(Vec::<(String, Value)>::new())
            )),
            FfiAction::Resume(Value::Unit)
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn c_string_argument_reaches_foreign_function()
    {
        let function = ForeignFn {
            op: "gandr_test_strlen".to_owned(),
            params: vec![ForeignParam {
                name: "text".to_owned(),
                c_type: CType::CStr,
            }],
            result: CType::U64,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        assert!(matches!(
            host.dispatch(&offer(
                "gandr_test_strlen",
                Value::record([("text".to_owned(), Value::string("hello"))])
            )),
            FfiAction::Resume(Value::Num(NumLit::U64(5)))
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn opaque_pointer_argument_and_result_round_trip()
    {
        let function = ForeignFn {
            op: "gandr_identity_ptr".to_owned(),
            params: vec![ForeignParam {
                name: "value".to_owned(),
                c_type: CType::Ptr,
            }],
            result: CType::Ptr,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([("value".to_owned(), Value::u64(0x1234))]);
        assert!(matches!(
            host.dispatch(&offer("gandr_identity_ptr", payload)),
            FfiAction::Resume(Value::Num(NumLit::U64(0x1234)))
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn colliding_foreign_signature_reaches_ffi_boundary()
    {
        let foreign = ForeignModule {
            name: effect::host::PROC.to_owned(),
            abi: "c".to_owned(),
            library: hermetic_testlib_path().display().to_string(),
            types: Vec::new(),
            functions: vec![ForeignFn {
                op: effect::host::PROC_EXIT.to_owned(),
                params: vec![ForeignParam {
                    name: "code".to_owned(),
                    c_type: CType::I32,
                }],
                result: CType::Void,
            }],
        };
        let lowered_signature = foreign.effect_sig();
        let lowered_payload = Value::record([("code".to_owned(), Value::i32(7_i32))]);
        let mut driver = CombinedDriver {
            ffi: FfiHost::new(vec![foreign]).expect("fixture loads"),
            shell: ShellHandler::new(),
            early: None,
        };
        let reply = effect::host::HostHandler::handle(
            &mut driver,
            &lowered_signature,
            effect::host::PROC_EXIT,
            &lowered_payload,
        );
        assert!(matches!(
            reply,
            effect::host::HostReply::Resume(Value::Unit)
        ));
        assert!(driver.early.is_none());
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn machine_preserves_colliding_foreign_signature_for_ffi_fallback()
    {
        let foreign = ForeignModule {
            name: effect::host::PROC.to_owned(),
            abi: "c".to_owned(),
            library: hermetic_testlib_path().display().to_string(),
            types: Vec::new(),
            functions: vec![ForeignFn {
                op: effect::host::PROC_EXIT.to_owned(),
                params: vec![ForeignParam {
                    name: "code".to_owned(),
                    c_type: CType::I32,
                }],
                result: CType::Void,
            }],
        };
        let signature = foreign.effect_sig();
        let computation = gandr_core_checker::syntax::Comp::perform(
            signature,
            effect::host::PROC_EXIT,
            Value::record([("code".to_owned(), Value::i32(7_i32))]),
        );
        assert!(matches!(
            run_program(&computation, vec![foreign]),
            Ok(FfiShellOutcome::Completed(_))
        ));
    }
    #[test]
    fn undeclared_operation_declines()
    {
        let host = FfiHost::new(Vec::new()).expect("empty registry loads");
        assert!(matches!(
            host.dispatch(&offer(
                "missing",
                Value::record(Vec::<(String, Value)>::new())
            )),
            FfiAction::Decline
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn interior_nul_is_a_typed_boundary_failure()
    {
        let function = ForeignFn {
            op: "gandr_test_strlen".to_owned(),
            params: vec![ForeignParam {
                name: "text".to_owned(),
                c_type: CType::CStr,
            }],
            result: CType::U64,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([("text".to_owned(), Value::string("a\0b"))]);
        assert!(matches!(
            host.dispatch(&offer("gandr_test_strlen", payload)),
            FfiAction::Fail(FfiError::Marshal { .. })
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn wrong_argument_type_is_a_typed_boundary_failure()
    {
        let function = ForeignFn {
            op: "gandr_test_add".to_owned(),
            params: vec![ForeignParam {
                name: "left".to_owned(),
                c_type: CType::I64,
            }],
            result: CType::I64,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([("left".to_owned(), Value::string("not an integer"))]);
        assert!(matches!(
            host.dispatch(&offer("gandr_test_add", payload)),
            FfiAction::Fail(FfiError::Marshal { .. })
        ));
    }

    #[cfg(feature = "native-fixture")]
    #[test]
    fn wrong_argument_arity_is_a_typed_boundary_failure()
    {
        let function = ForeignFn {
            op: "gandr_test_add".to_owned(),
            params: vec![
                ForeignParam {
                    name: "left".to_owned(),
                    c_type: CType::I64,
                },
                ForeignParam {
                    name: "right".to_owned(),
                    c_type: CType::I64,
                },
            ],
            result: CType::I64,
        };
        let host = FfiHost::new(vec![module(function)]).expect("fixture loads");
        let payload = Value::record([("left".to_owned(), Value::i64(21))]);
        assert!(matches!(
            host.dispatch(&offer("gandr_test_add", payload)),
            FfiAction::Fail(FfiError::Marshal { .. })
        ));
    }
}
