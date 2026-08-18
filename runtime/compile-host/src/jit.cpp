#include "gandr/compile_host/jit.hpp"

#include <chrono>
#include <functional>
#include <cstddef>
#include <memory>
#include <string>
#include <vector>

#include "mlir/ExecutionEngine/ExecutionEngine.h"
#include "mlir/ExecutionEngine/CRunnerUtils.h"
#include "mlir/ExecutionEngine/OptUtils.h"
#include "mlir/IR/MLIRContext.h"
#include "llvm/Support/Error.h"
#include "llvm/Support/TargetSelect.h"

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/pipeline.hpp"

namespace gandr::compile_host
{
namespace
{

/// Initializes the native target once per process.
///
/// The execution engine needs a registered native target and asm printer; a
/// missing registration surfaces as an engine that will not build, far from
/// the omission.
void ensure_native_target()
{
    static const bool initialized = [] {
        llvm::InitializeNativeTarget();
        llvm::InitializeNativeTargetAsmPrinter();
        return true;
    }();
    (void)initialized;
}

} // namespace

Expected<RunOutcome> run_lowered_module(mlir::ModuleOp module, std::size_t heap_words)
{
    // The wall reaches execution as well as the passes: this entry is public,
    // so a caller could otherwise assemble the stages in an order that skips
    // it, and "verified before executed" would be a property of the usual call
    // path rather than of the host.
    const Expected<void> verified = verify_module(module);
    if (!verified.has_value()) {
        return std::unexpected(verified.error());
    }

    ensure_native_target();

    // `transformer` is a non-owning reference, so the optimizing pipeline has
    // to outlive the engine construction rather than be built inline.
    const std::function<llvm::Error(llvm::Module*)> transformer =
        mlir::makeOptimizingTransformer(2, 0, nullptr);
    mlir::ExecutionEngineOptions options;
    options.transformer = transformer;
    auto engine = mlir::ExecutionEngine::create(module, options);
    if (!engine) {
        return host_error(
            ErrorKind::ExecutionFailed,
            "execution engine could not be created: " + llvm::toString(engine.takeError()));
    }

    std::vector<std::int64_t> heap(heap_words, 0);
    reset_heap(heap);

    StridedMemRefType<std::int64_t, 1> descriptor{};
    descriptor.basePtr = heap.data();
    descriptor.data = heap.data();
    descriptor.offset = 0;
    descriptor.sizes[0] = static_cast<std::int64_t>(heap.size());
    descriptor.strides[0] = 1;
    auto* descriptor_pointer = &descriptor;

    std::int64_t produced = 0;
    // `invoke` composes the C-interface and packed-argument prefixes itself,
    // so the plain function name is what it takes.
    const std::string symbol{entry_point_name};
    llvm::Error invoked =
        (*engine)->invoke(symbol, descriptor_pointer, mlir::ExecutionEngine::result(produced));
    if (invoked) {
        return host_error(
            ErrorKind::ExecutionFailed,
            "entry point could not be invoked: " + llvm::toString(std::move(invoked)));
    }

    const std::optional<std::string> rendered = render_value(heap, produced);
    if (!rendered.has_value()) {
        return host_error(ErrorKind::ResultUnreadable, "compiled run produced an unreadable value");
    }
    return RunOutcome{.value = *rendered, .ledger = read_ledger(heap)};
}

Expected<RunOutcome> compile_and_run(const Image& image)
{
    const Expected<std::pair<RunOutcome, RunTiming>> timed = compile_and_run_timed(image);
    if (!timed.has_value()) {
        return std::unexpected(timed.error());
    }
    return timed->first;
}

Expected<std::pair<RunOutcome, RunTiming>> compile_and_run_timed(const Image& image)
{
    using Clock = std::chrono::steady_clock;

    const Clock::time_point compile_started = Clock::now();
    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, image);
    if (!module.has_value()) {
        return std::unexpected(module.error());
    }
    const Expected<void> lowered = lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
    if (!lowered.has_value()) {
        return std::unexpected(lowered.error());
    }
    const Clock::time_point compile_finished = Clock::now();

    const Expected<RunOutcome> outcome = run_lowered_module(module->get(), heap_words_for(image));
    const Clock::time_point execute_finished = Clock::now();
    if (!outcome.has_value()) {
        return std::unexpected(outcome.error());
    }

    const RunTiming timing{
        .compile_microseconds = std::chrono::duration_cast<std::chrono::microseconds>(
                                    compile_finished - compile_started)
                                    .count(),
        .execute_microseconds = std::chrono::duration_cast<std::chrono::microseconds>(
                                    execute_finished - compile_finished)
                                    .count(),
    };
    return std::pair<RunOutcome, RunTiming>{*outcome, timing};
}

} // namespace gandr::compile_host
