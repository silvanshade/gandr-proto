#include "gandr/compile_host/pipeline.hpp"

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/status.hpp"
#include "mlir/Conversion/ConvertToLLVM/ToLLVMPass.h"
#include "mlir/Conversion/ReconcileUnrealizedCasts/ReconcileUnrealizedCasts.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/Diagnostics.h"
#include "mlir/IR/Verifier.h"
#include "mlir/Pass/PassManager.h"
#include "mlir/Support/LLVM.h"
#include "mlir/Transforms/Passes.h"

#include <string>

namespace gandr::compile_host {
namespace {

/// Collects the diagnostics a scoped operation emits, so a failure carries
/// what the verifier or a pass actually said rather than a bare verdict.
class DiagnosticCollector
{
public:
  /// Installs the handler on `context` for this object's lifetime.
  explicit DiagnosticCollector(mlir::MLIRContext& context)
    : context_(context)
  {
    handler_id_ = context_.getDiagEngine().registerHandler([this](mlir::Diagnostic& diagnostic) -> mlir::LogicalResult {
      if (!collected_.empty()) {
        collected_ += "; ";
      }
      collected_ += diagnostic.str();
      return mlir::success();
    });
  }

  DiagnosticCollector(DiagnosticCollector const&) = delete;
  DiagnosticCollector(DiagnosticCollector&&) = delete;
  auto
  operator=(DiagnosticCollector const&) -> DiagnosticCollector& = delete;
  auto
  operator=(DiagnosticCollector&&) -> DiagnosticCollector& = delete;

  ~DiagnosticCollector() { context_.getDiagEngine().eraseHandler(handler_id_); }

  /// What was collected, joined.
  [[nodiscard]] auto
  collected() const noexcept -> std::string const&
  {
    return collected_;
  }

private:
  /// The context the handler is installed on.
  mlir::MLIRContext& context_;
  /// The handler's registration.
  mlir::DiagnosticEngine::HandlerID handler_id_ = 0;
  /// The joined diagnostic text.
  std::string collected_;
};

} // namespace

auto
verify_module(mlir::ModuleOp module) -> Expected<void>
{
  DiagnosticCollector collector(*module.getContext());
  if (mlir::failed(mlir::verify(module))) {
    return host_error(
      ErrorKind::VerifierRejected,
      collector.collected().empty() ? std::string("module failed verification") : collector.collected()
    );
  }
  return Expected<void>{};
}

auto
optimize_module(mlir::ModuleOp module, Optimization optimization) -> Expected<void>
{
  // The wall opens the pipeline. A module that has not been verified is
  // never handed to a pass: canonicalization on a malformed module is
  // undefined territory, and the arity fixture is exactly the malformed
  // module a builder happily produces.
  Expected<void> const verified = verify_module(module);
  if (!verified.has_value()) {
    return verified;
  }
  if (optimization == Optimization::None) {
    return Expected<void>{};
  }

  DiagnosticCollector collector(*module.getContext());
  mlir::PassManager manager(module.getContext());
  manager.addPass(mlir::createCanonicalizerPass());
  manager.addPass(mlir::createCSEPass());
  if (mlir::failed(manager.run(module))) {
    return host_error(
      ErrorKind::ConversionFailed,
      collector.collected().empty() ? std::string("optimization pipeline failed") : collector.collected()
    );
  }
  return Expected<void>{};
}

auto
lower_module(mlir::ModuleOp module, Optimization optimization) -> Expected<void>
{
  Expected<void> const optimized = optimize_module(module, optimization);
  if (!optimized.has_value()) {
    return optimized;
  }

  Expected<void> const structural = lower_dialect_operations(module);
  if (!structural.has_value()) {
    return structural;
  }

  DiagnosticCollector collector(*module.getContext());
  if (mlir::failed(mlir::verify(module))) {
    return host_error(
      ErrorKind::LoweringFailed,
      collector.collected().empty() ? std::string("lowered module failed verification") : collector.collected()
    );
  }

  mlir::PassManager manager(module.getContext());
  manager.addPass(mlir::createConvertToLLVMPass());
  manager.addPass(mlir::createReconcileUnrealizedCastsPass());
  if (mlir::failed(manager.run(module))) {
    return host_error(
      ErrorKind::ConversionFailed,
      collector.collected().empty() ? std::string("conversion to the LLVM dialect failed") : collector.collected()
    );
  }

  if (mlir::failed(mlir::verify(module))) {
    return host_error(
      ErrorKind::ConversionFailed,
      collector.collected().empty() ? std::string("converted module failed verification") : collector.collected()
    );
  }

  // The one-shot conversion reports success when it converted nothing, so
  // the pipeline checks its postcondition rather than its verdict: the entry
  // point must now be an LLVM-dialect function.
  if (!module.lookupSymbol<mlir::LLVM::LLVMFuncOp>(std::string(entry_point_name))) {
    return host_error(ErrorKind::ConversionFailed, "conversion left the entry point outside the LLVM dialect");
  }
  return Expected<void>{};
}

} // namespace gandr::compile_host
