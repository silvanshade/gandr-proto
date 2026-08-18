// The execution stage: an LLVM-dialect module becomes a callable, and the
// callable runs on a host-owned heap.
//
// # Contract
// - requires: a module `lower_module` accepted.
// - ensures: the entry point is invoked on the caller's heap, and the run's
//   result and ledger are read back from that heap.
// - provides: the executed-value side of the differential.
// - fails: a typed error when the engine cannot be built, the entry point
//   cannot be resolved, or the produced value cannot be read.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_JIT_HPP
#define GANDR_COMPILE_HOST_JIT_HPP

#include <cstdint>
#include <span>
#include <string>
#include <vector>

#include "mlir/IR/BuiltinOps.h"

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/status.hpp"
#include "gandr/compile_host/value.hpp"

namespace gandr::compile_host
{

/// What one execution produced.
struct RunOutcome
{
    /// The produced value, rendered canonically.
    std::string value;
    /// The work the run accounted for.
    WorkLedger ledger;
    /// The arena words the run consumed, above the reserved prefix.
    ///
    /// The two paths are compared on the value and the ledger and never on
    /// this count: canonicalization folds pure producers, so the compiled path
    /// may allocate strictly fewer words than the reference walk for the same
    /// program. The differential asserts that ordering rather than equality.
    std::size_t allocated = 0;

    friend bool operator==(const RunOutcome&, const RunOutcome&) = default;
};

/// Compiles and runs a lowered module on a fresh heap.
///
/// # Contract
/// - requires: the module holds the entry point `entry_point_name`, already
///   lowered to the LLVM dialect.
/// - ensures: the heap is reset before the call, so the ledger counts one
///   run's work and nothing carried over; a run whose allocations do not fit
///   `heap_words` stops at the first one that does not, having written nothing
///   outside the heap.
/// - provides: the compiled half of the agreement differential.
/// - fails: `ErrorKind::LimitExceeded` when `heap_words` cannot hold the
///   reserved prefix, or when the run refused an allocation;
///   `ErrorKind::ExecutionFailed` when the engine cannot be built or the entry
///   point cannot be resolved; `ErrorKind::ResultUnreadable` when the produced
///   value does not render.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> run_lowered_module(
    mlir::ModuleOp module,
    std::size_t heap_words);

/// Compiles and runs a lowered module on a heap the caller owns.
///
/// The extent the compiled code checks against is the one in the descriptor
/// built from `heap`, so a caller passing a subspan of a larger buffer bounds
/// the run to that subspan — which is how the surrounding words can be
/// inspected afterwards for a write that should not have happened.
///
/// # Contract
/// - requires: the module holds the entry point `entry_point_name`, already
///   lowered to the LLVM dialect.
/// - ensures: `heap` is reset before the call and no word outside it is
///   written by the run.
/// - provides: the heap-owning entry the bounds witness and any embedding
///   caller use.
/// - fails: as `run_lowered_module`.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> run_lowered_module_on(
    mlir::ModuleOp module,
    std::span<std::int64_t> heap);

/// Compiles a program image and runs it, end to end.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: every stage runs in order, verifier first; the heap is sized by
///   `heap_words_for`.
/// - provides: the entry the CLI, the tests and the property harness all use,
///   so no caller can skip the wall by assembling the stages differently.
/// - fails: the first stage's typed error.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> compile_and_run(const Image& image);

/// Compiles a program image and runs it on a heap of the caller's size.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: identical to `compile_and_run` when `heap_words` is at least
///   what the run needs; otherwise the run reports exhaustion rather than
///   writing outside the heap.
/// - provides: the sized entry the bounds witnesses drive, so the check is
///   exercised at the exact word rather than only far from it.
/// - fails: the first stage's typed error, `ErrorKind::LimitExceeded`
///   included.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> compile_and_run_with_heap(
    const Image& image,
    std::size_t heap_words);

/// The measured cost of one end-to-end compilation and run.
struct RunTiming
{
    /// Microseconds from a fresh context to a lowered module.
    std::int64_t compile_microseconds = 0;
    /// Microseconds spent building the engine and calling the entry point.
    std::int64_t execute_microseconds = 0;
};

/// Compiles and runs a program image, reporting the stage timings beside the
/// outcome.
///
/// # Contract
/// - ensures: the outcome is identical to `compile_and_run`'s; the timings are
///   wall-clock and are reported, never asserted on.
/// - provides: the host's own cost measurement, so a number in a report has a
///   command behind it.
/// - fails: as `compile_and_run`.
/// - panics: none.
[[nodiscard]] Expected<std::pair<RunOutcome, RunTiming>> compile_and_run_timed(const Image& image);

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_JIT_HPP
