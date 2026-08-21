// The compile host's fuzz entry surface.
//
// One byte string in, one decode-emit-verify pass out, then a reference run.
//
// The reference walk is included and the JIT is not, and the split is about
// cost rather than coverage: the walk shares the image, the heap layout and
// the bounds discipline with the compiled path and costs microseconds, while
// building an execution engine per input would put a compiler on every fuzz
// iteration. A decoded image is bounded by construction, and a run that would
// outgrow its heap reports the refusal, so executing arbitrary well-formed
// images is now a bounded operation rather than a measurement of the sizing.
//
// # Contract
// - requires: the input on standard input, or file paths for deterministic
//   replay.
// - ensures: every input is either refused by the decoder or carried through
//   emission, verification and a reference run; no path may crash, hang, or
//   read out of bounds.
// - provides: the AFL++ target and the seed-replay smoke.
// - fails: a nonzero exit status only for a replay whose file cannot be read.
// - panics: none.

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/pipeline.hpp"
#include "gandr/compile_host/status.hpp"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/OwningOpRef.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <ios>
#include <iterator>
#include <memory>
#include <optional>
#include <print>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace {

// The host's names this program uses, declared one by one: a using-directive
// here would carry every future addition to the namespace with it.
using gandr::compile_host::decode_image;
using gandr::compile_host::Expected;
using gandr::compile_host::Image;
using gandr::compile_host::make_context;
using gandr::compile_host::RunOutcome;
using gandr::compile_host::verify_module;

/// Drives one byte string through the decoder, the emitter, and the verifier.
///
/// A refused decode is a normal outcome, not a finding: the surface under test
/// is that refusal and acceptance are the only two outcomes.
void
exercise(std::span<std::uint8_t const> bytes)
{
  std::optional<Image> const image = decode_image(bytes);
  if (!image.has_value()) {
    return;
  }
  std::unique_ptr<mlir::MLIRContext> const context = make_context();
  Expected<mlir::OwningOpRef<mlir::ModuleOp>> const module = emit_module(*context, *image);
  if (!module.has_value()) {
    return;
  }
  Expected<void> const verified = verify_module(module->get());
  if (!verified.has_value()) {
    return;
  }
  // The reference run. Its outcome is not asserted on — a generated program
  // may legitimately exhaust its heap or nest past the walk's bound — only
  // that reaching one of the two outcomes is all it ever does.
  Expected<RunOutcome> const outcome = interpret_image(*image);
  (void)outcome;
}

/// Reads a whole file as bytes.
[[nodiscard]] auto
read_file(std::string_view path) -> std::optional<std::vector<std::uint8_t>>
{
  std::ifstream file(std::string(path), std::ios::binary);
  if (!file) {
    return std::nullopt;
  }
  // Named iterators rather than a temporary pair: the inline form is a
  // declaration under the most vexing parse, and the parenthesis that used
  // to disambiguate it is not a parenthesis the format policy keeps.
  std::istreambuf_iterator<char> const first{ file };
  std::istreambuf_iterator<char> const last{};
  return std::vector<std::uint8_t>(first, last);
}

/// The largest input the entry reads, so a hostile length cannot exhaust
/// memory before the decoder's own bound applies.
constexpr std::size_t max_input_bytes = std::size_t{ 1 } << 20U;

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

  if (!arguments.empty()) {
    int status = EXIT_SUCCESS;
    for (std::string_view const path : arguments) {
      std::optional<std::vector<std::uint8_t>> const bytes = read_file(path);
      if (!bytes.has_value()) {
        std::println(stderr, "could not read {}", path);
        status = EXIT_FAILURE;
        continue;
      }
      exercise(*bytes);
    }
    return status;
  }

  std::vector<std::uint8_t> input;
  input.reserve(4096);
  int byte = 0;
  while ((byte = std::getchar()) != EOF && input.size() < max_input_bytes) {
    input.push_back(static_cast<std::uint8_t>(byte));
  }
  exercise(input);
  return EXIT_SUCCESS;
}
