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

    friend bool operator==(const RunOutcome&, const RunOutcome&) = default;
};

/// Compiles and runs a lowered module on a fresh heap.
///
/// # Contract
/// - requires: the module holds the entry point `entry_point_name`, already
///   lowered to the LLVM dialect.
/// - ensures: the heap is reset before the call, so the ledger counts one
///   run's work and nothing carried over.
/// - provides: the compiled half of the agreement differential.
/// - fails: `ErrorKind::ExecutionFailed` when the engine cannot be built or
///   the entry point cannot be resolved; `ErrorKind::ResultUnreadable` when
///   the produced value does not render.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> run_lowered_module(
    mlir::ModuleOp module,
    std::size_t heap_words);

/// Compiles a program image and runs it, end to end.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: every stage runs in order, verifier first.
/// - provides: the entry the CLI, the tests and the property harness all use,
///   so no caller can skip the wall by assembling the stages differently.
/// - fails: the first stage's typed error.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> compile_and_run(const Image& image);

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
