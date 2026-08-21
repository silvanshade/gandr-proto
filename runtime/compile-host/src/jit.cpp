#include "gandr/compile_host/jit.hpp"

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/pipeline.hpp"
#include "mlir/ExecutionEngine/CRunnerUtils.h"
#include "mlir/ExecutionEngine/ExecutionEngine.h"
#include "mlir/ExecutionEngine/OptUtils.h"
#include "mlir/IR/MLIRContext.h"

#include "llvm/Support/Error.h"
#include "llvm/Support/TargetSelect.h"

#include <chrono>
#include <cstddef>
#include <functional>
#include <memory>
#include <span>
#include <string>
#include <vector>

namespace gandr::compile_host {
namespace {

/// Initializes the native target once per process.
///
/// The execution engine needs a registered native target and asm printer; a
/// missing registration surfaces as an engine that will not build, far from
/// the omission.
void
ensure_native_target()
{
  static bool const initialized = [] {
    llvm::InitializeNativeTarget();
    llvm::InitializeNativeTargetAsmPrinter();
    return true;
  }();
  (void)initialized;
}

} // namespace

Expected<RunOutcome>
run_lowered_module(mlir::ModuleOp module, std::size_t heap_words)
{
  std::vector<std::int64_t> heap(heap_words, 0);
  return run_lowered_module_on(module, heap);
}

Expected<RunOutcome>
run_lowered_module_on(mlir::ModuleOp module, std::span<std::int64_t> heap)
{
  // The wall reaches execution as well as the passes: this entry is public,
  // so a caller could otherwise assemble the stages in an order that skips
  // it, and "verified before executed" would be a property of the usual call
  // path rather than of the host.
  Expected<void> const verified = verify_module(module);
  if (!verified.has_value()) {
    return std::unexpected(verified.error());
  }

  if (heap.size() < HeapLayout::arena_base) {
    return host_error(ErrorKind::LimitExceeded, "heap is too small to hold the reserved prefix the entry point writes");
  }

  ensure_native_target();

  // `transformer` is a non-owning reference, so the optimizing pipeline has
  // to outlive the engine construction rather than be built inline.
  std::function<llvm::Error(llvm::Module*)> const transformer = mlir::makeOptimizingTransformer(2, 0, nullptr);
  mlir::ExecutionEngineOptions options;
  options.transformer = transformer;
  auto engine = mlir::ExecutionEngine::create(module, options);
  if (!engine) {
    return host_error(
      ErrorKind::ExecutionFailed,
      "execution engine could not be created: " + llvm::toString(engine.takeError())
    );
  }

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
  std::string const symbol{ entry_point_name };
  llvm::Error invoked = (*engine)->invoke(symbol, descriptor_pointer, mlir::ExecutionEngine::result(produced));
  if (invoked) {
    return host_error(
      ErrorKind::ExecutionFailed,
      "entry point could not be invoked: " + llvm::toString(std::move(invoked))
    );
  }

  // The flag is read before the returned word, because a refused run returns
  // a word that was never an allocation and rendering it would be reading a
  // heap the program declined to build.
  if (heap_was_exhausted(heap)) {
    return host_error(ErrorKind::LimitExceeded, "compiled run refused an allocation that would not fit its heap");
  }

  std::optional<std::string> const rendered = render_value(heap, produced);
  if (!rendered.has_value()) {
    return host_error(ErrorKind::ResultUnreadable, "compiled run produced an unreadable value");
  }
  return RunOutcome{
    .value = *rendered,
    .ledger = read_ledger(heap),
    .allocated = allocated_words(heap),
  };
}

Expected<RunOutcome>
compile_and_run(Image const& image)
{
  Expected<std::pair<RunOutcome, RunTiming>> const timed = compile_and_run_timed(image);
  if (!timed.has_value()) {
    return std::unexpected(timed.error());
  }
  return timed->first;
}

Expected<RunOutcome>
compile_and_run_with_heap(Image const& image, std::size_t heap_words)
{
  std::unique_ptr<mlir::MLIRContext> const context = make_context();
  Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, image);
  if (!module.has_value()) {
    return std::unexpected(module.error());
  }
  Expected<void> const lowered = lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
  if (!lowered.has_value()) {
    return std::unexpected(lowered.error());
  }
  return run_lowered_module(module->get(), heap_words);
}

Expected<std::pair<RunOutcome, RunTiming>>
compile_and_run_timed(Image const& image)
{
  using Clock = std::chrono::steady_clock;

  Clock::time_point const compile_started = Clock::now();
  std::unique_ptr<mlir::MLIRContext> const context = make_context();
  Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, image);
  if (!module.has_value()) {
    return std::unexpected(module.error());
  }
  Expected<void> const lowered = lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
  if (!lowered.has_value()) {
    return std::unexpected(lowered.error());
  }
  Clock::time_point const compile_finished = Clock::now();

  Expected<RunOutcome> const outcome = run_lowered_module(module->get(), heap_words_for(image));
  Clock::time_point const execute_finished = Clock::now();
  if (!outcome.has_value()) {
    return std::unexpected(outcome.error());
  }

  RunTiming const timing{
    .compile_microseconds
    = std::chrono::duration_cast<std::chrono::microseconds>(compile_finished - compile_started).count(),
    .execute_microseconds
    = std::chrono::duration_cast<std::chrono::microseconds>(execute_finished - compile_finished).count(),
  };
  return std::pair<RunOutcome, RunTiming>{ *outcome, timing };
}

} // namespace gandr::compile_host
