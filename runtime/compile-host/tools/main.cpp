// The compile host's command-line face.
//
// # Contract
// - requires: one of the modes below, or none, in which case the sample run is
//   the default.
// - ensures: every mode prints machine-readable lines and returns a nonzero
//   exit status if any stage failed.
// - provides: the observable surface the mise tasks drive.
// - panics: none; a failure is a printed diagnostic and an exit status.

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/pipeline.hpp"
#include "gandr/compile_host/samples.hpp"
#include "gandr/compile_host/status.hpp"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/OwningOpRef.h"

#include "llvm/Support/raw_ostream.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <ios>
#include <memory>
#include <print>
#include <string>
#include <string_view>
#include <system_error>
#include <utility>
#include <vector>

namespace {

using namespace gandr::compile_host;

/// Prints one outcome line in the fixture's format.
void
print_outcome(std::string_view name, RunOutcome const& outcome)
{
  std::println("{}\t{}\t{}\t{}", name, outcome.value, outcome.ledger.duplications, outcome.ledger.discards);
}

/// Prints a typed failure to the error stream.
void
print_error(std::string_view name, HostError const& error)
{
  std::println(stderr, "{}: {}: {}", name, error_kind_name(error.kind), error.detail);
}

/// Every program the host names, in a stable order.
[[nodiscard]] auto
all_samples() -> std::vector<Sample>
{
  std::vector<Sample> samples = canonical_samples();
  samples.push_back(accounted_work_sample());
  return samples;
}

/// Runs every named program through the compiled path.
[[nodiscard]] auto
run_samples() -> int
{
  int status = EXIT_SUCCESS;
  for (Sample const& sample : all_samples()) {
    Expected<RunOutcome> const outcome = compile_and_run(sample.image);
    if (!outcome.has_value()) {
      print_error(sample.name, outcome.error());
      status = EXIT_FAILURE;
      continue;
    }
    print_outcome(sample.name, *outcome);
  }
  return status;
}

/// Runs every named program through the reference interpreter.
[[nodiscard]] auto
interpret_samples() -> int
{
  int status = EXIT_SUCCESS;
  for (Sample const& sample : all_samples()) {
    Expected<RunOutcome> const outcome = interpret_image(sample.image);
    if (!outcome.has_value()) {
      print_error(sample.name, outcome.error());
      status = EXIT_FAILURE;
      continue;
    }
    print_outcome(sample.name, *outcome);
  }
  return status;
}

/// Reports the compile and execute cost of every named program.
[[nodiscard]] auto
report_timings() -> int
{
  int status = EXIT_SUCCESS;
  for (Sample const& sample : all_samples()) {
    Expected<std::pair<RunOutcome, RunTiming>> const timed = compile_and_run_timed(sample.image);
    if (!timed.has_value()) {
      print_error(sample.name, timed.error());
      status = EXIT_FAILURE;
      continue;
    }
    std::println("{}\t{}\t{}", sample.name, timed->second.compile_microseconds, timed->second.execute_microseconds);
  }
  return status;
}

/// Prints a named program's module at a chosen stage.
[[nodiscard]] auto
dump_module(std::string_view name, bool lowered) -> int
{
  for (Sample const& sample : all_samples()) {
    if (sample.name != name) {
      continue;
    }
    std::unique_ptr<mlir::MLIRContext> const context = make_context();
    Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, sample.image);
    if (!module.has_value()) {
      print_error(name, module.error());
      return EXIT_FAILURE;
    }
    if (lowered) {
      Expected<void> const result = lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
      if (!result.has_value()) {
        print_error(name, result.error());
        return EXIT_FAILURE;
      }
    } else {
      Expected<void> const result = verify_module(module->get());
      if (!result.has_value()) {
        print_error(name, result.error());
        return EXIT_FAILURE;
      }
    }
    module->get()->print(llvm::outs());
    llvm::outs() << "\n";
    return EXIT_SUCCESS;
  }
  std::println(stderr, "no sample named {}", name);
  return EXIT_FAILURE;
}

/// Writes the encoded form of every named program into a directory.
[[nodiscard]] auto
write_seeds(std::string_view directory) -> int
{
  std::error_code creation;
  std::filesystem::create_directories(std::filesystem::path(directory), creation);
  if (creation) {
    std::println(stderr, "could not create {}", directory);
    return EXIT_FAILURE;
  }
  for (Sample const& sample : all_samples()) {
    std::vector<std::uint8_t> const bytes = encode_image(sample.image);
    std::filesystem::path const path = std::filesystem::path(directory) / (std::string(sample.name) + ".bin");
    std::ofstream out(path, std::ios::binary | std::ios::trunc);
    if (!out) {
      std::println(stderr, "could not write {}", path.string());
      return EXIT_FAILURE;
    }
    out.write(reinterpret_cast<char const*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
  }
  return EXIT_SUCCESS;
}

/// The usage text, printed for an unrecognized mode.
void
print_usage()
{
  std::println(
    stderr,
    "usage: gandr-compile-host [MODE]\n  --run-samples          compile and run every named program (default)\n  "
    "--interpret-samples    evaluate every named program with the reference interpreter\n  --timings              "
    "report per-program compile and execute microseconds\n  --dump-dialect=NAME    print a program's verified dialect "
    "module\n  --dump-lowered=NAME    print a program's lowered module\n  --write-seeds=DIR      write every named "
    "program's encoded image into DIR"
  );
}

} // namespace

auto
main(int argc, char** argv) -> int
{
  // A span over the whole vector, then everything after the program name:
  // argc is not guaranteed positive, so the tail is taken only when there
  // is one.
  std::span<char* const> const raw(argv, argc < 1 ? 0U : static_cast<std::size_t>(argc));
  std::span<char* const> const tail = raw.empty() ? raw : raw.subspan(1);
  std::vector<std::string_view> const arguments(tail.begin(), tail.end());
  if (arguments.empty()) {
    return run_samples();
  }
  std::string_view const mode = arguments.front();

  if (mode == "--run-samples") {
    return run_samples();
  }
  if (mode == "--interpret-samples") {
    return interpret_samples();
  }
  if (mode == "--timings") {
    return report_timings();
  }
  if (mode.starts_with("--dump-dialect=")) {
    return dump_module(mode.substr(std::string_view("--dump-dialect=").size()), false);
  }
  if (mode.starts_with("--dump-lowered=")) {
    return dump_module(mode.substr(std::string_view("--dump-lowered=").size()), true);
  }
  if (mode.starts_with("--write-seeds=")) {
    return write_seeds(mode.substr(std::string_view("--write-seeds=").size()));
  }

  print_usage();
  return EXIT_FAILURE;
}
