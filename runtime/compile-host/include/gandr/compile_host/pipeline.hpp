// The compilation pipeline, and the verifier wall that opens it.
//
// # Contract
// - requires: a module emitted by `emit_module` into a context from
//   `make_context`.
// - ensures: the verifier runs before any pass and before any execution; a
//   module that fails it never reaches canonicalization, lowering, or the JIT.
// - provides: the one route from a dialect module to an executable one.
// - fails: a typed error at whichever stage rejected.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_PIPELINE_HPP
#define GANDR_COMPILE_HOST_PIPELINE_HPP

#include <cstddef>

#include "mlir/IR/BuiltinOps.h"

#include "gandr/compile_host/status.hpp"

namespace gandr::compile_host
{

/// Whether the optimization passes run between the dialect and the lowering.
///
/// Canonicalization is a separate switch because the accounted-work witness
/// runs the module both ways and compares operation counts: the effect
/// declarations are what must make the two counts agree.
enum class Optimization : std::uint8_t
{
    /// Lower the module as emitted.
    None = 0,
    /// Run canonicalization and common-subexpression elimination first.
    CanonicalizeAndDeduplicate = 1,
};

/// Runs the mandatory verifier wall over a module.
///
/// This is the first thing every other entry in this header does, and the
/// reason it is public is that a caller may want the verdict without the rest
/// of the pipeline. The proving spike measured why the wall has to exist: a
/// constructor with the wrong operand count builds, and only a verifier
/// objects.
///
/// # Contract
/// - ensures: returns success exactly when `mlir::verify` accepts the module.
/// - provides: the wall itself.
/// - fails: `ErrorKind::VerifierRejected`, carrying the diagnostics the
///   verifier emitted.
/// - panics: none.
[[nodiscard]] Expected<void> verify_module(mlir::ModuleOp module);

/// Runs the optimization passes over a verified dialect module.
///
/// # Contract
/// - requires: nothing; the verifier runs first, inside.
/// - ensures: the module is still in the gandr dialect afterwards.
/// - provides: the observation point the accounted-work witness compares.
/// - fails: `ErrorKind::VerifierRejected` before the passes;
///   `ErrorKind::ConversionFailed` if a pass fails.
/// - panics: none.
[[nodiscard]] Expected<void> optimize_module(mlir::ModuleOp module, Optimization optimization);

/// Lowers a verified dialect module to the LLVM dialect.
///
/// The stages, in order: the verifier wall; the optimization passes, if any;
/// the structural lowering that replaces every gandr operation with control
/// flow, arithmetic and memory operations; the standard conversion to the LLVM
/// dialect; and the verifier again.
///
/// # Contract
/// - ensures: the module holds no operation of the gandr dialect afterwards.
/// - provides: the module the execution engine compiles.
/// - fails: `ErrorKind::VerifierRejected`, `ErrorKind::LoweringFailed`, or
///   `ErrorKind::ConversionFailed`, at whichever stage rejected.
/// - panics: none.
[[nodiscard]] Expected<void> lower_module(mlir::ModuleOp module, Optimization optimization);

/// Replaces every gandr operation in a module with its lowering.
///
/// Split out of `lower_module` so a caller can observe the intermediate
/// standard-dialect form; the pipeline entry above is what callers normally
/// use.
///
/// # Contract
/// - requires: the module verified.
/// - ensures: on success no operation of the gandr dialect remains.
/// - fails: `ErrorKind::LoweringFailed` for an operation with no lowering, or
///   `ErrorKind::LimitExceeded` past `max_emit_depth` of consumer nesting.
/// - panics: none.
[[nodiscard]] Expected<void> lower_dialect_operations(mlir::ModuleOp module);

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_PIPELINE_HPP
