// Stage one: a program image becomes a module in the gandr dialect.
//
// # Contract
// - requires: a context with the gandr dialect and the standard dialects the
//   later stages need already loaded — `make_context` does that.
// - ensures: the emitted module holds exactly one function, named
//   `entry_point_name`, whose body is dialect operations ending in a cut.
// - provides: the module the verifier wall receives.
// - fails: a malformed image, or a nesting depth past `max_emit_depth`.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_EMIT_HPP
#define GANDR_COMPILE_HOST_EMIT_HPP

#include <cstddef>
#include <string_view>

#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/OwningOpRef.h"

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/status.hpp"

namespace gandr::compile_host
{

/// The name of the function every stage of the pipeline carries.
inline constexpr std::string_view entry_point_name = "gandr_positive_core";

/// The deepest consumer nesting the emitter and the lowering will walk.
///
/// Both stages descend the region tree, so the bound is what makes them total
/// on an image a generator or a fuzzer produced rather than on one the host
/// wrote.
inline constexpr std::size_t max_emit_depth = 64;

/// Builds a context with every dialect the pipeline needs.
///
/// # Contract
/// - ensures: the gandr dialect and the func, arith, control-flow, memref, ub
///   and llvm dialects are loaded, and the LLVM translation interfaces are
///   registered.
/// - provides: the only context construction the host uses, so no stage can
///   fail for a dialect nobody loaded.
/// - panics: none.
[[nodiscard]] std::unique_ptr<mlir::MLIRContext> make_context();

/// Emits a program image as a module in the gandr dialect.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`; the emitter rechecks
///   rather than trusting the caller.
/// - ensures: the module's single function has the signature
///   `(memref<?xi64>) -> i64` and carries the C interface attribute the
///   execution engine needs.
/// - provides: stage one of the pipeline.
/// - fails: `ErrorKind::MalformedImage` for an image that does not verify
///   structurally; `ErrorKind::LimitExceeded` past `max_emit_depth`.
/// - panics: none.
[[nodiscard]] Expected<mlir::OwningOpRef<mlir::ModuleOp>> emit_module(
    mlir::MLIRContext& context,
    const Image& image);

/// Emits a module whose single constructor is given the wrong operand count.
///
/// The verifier wall's negative fixture. It is built here rather than in a
/// test because building it requires bypassing the arity the tag declares,
/// which is exactly the property the fixture exists to demonstrate is checked
/// by a pass rather than by the builder.
///
/// # Contract
/// - ensures: the returned module builds, and `mlir::verify` rejects it.
/// - provides: the malformed-arity fixture the verifier test drives.
/// - panics: none.
[[nodiscard]] mlir::OwningOpRef<mlir::ModuleOp> emit_malformed_arity_module(
    mlir::MLIRContext& context);

/// Builds the accounted-work witness module directly.
///
/// A duplication and a discard whose results nothing observes. Under the
/// declared memory effects canonicalization must keep both; declaring either
/// pure lets it delete them, and the operation counts are what say which
/// happened. The module is built here rather than emitted from an image
/// because the image language has no way to leave a producer's result unused,
/// while the dialect does — and the dialect is the surface under test.
///
/// # Contract
/// - ensures: the module verifies, and holds exactly one duplication, one
///   discard, and two literals.
/// - provides: the canonicalization witness's input.
/// - panics: none.
[[nodiscard]] mlir::OwningOpRef<mlir::ModuleOp> emit_accounted_work_witness_module(
    mlir::MLIRContext& context);

/// Counts the operations of the gandr dialect in a module, by mnemonic.
///
/// # Contract
/// - ensures: the count covers every nested region.
/// - provides: the observation the canonicalization witness compares before
///   and after the pass pipeline.
/// - panics: none.
[[nodiscard]] std::size_t count_dialect_operations(
    mlir::ModuleOp module,
    std::string_view mnemonic);

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_EMIT_HPP
