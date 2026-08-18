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

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <iterator>
#include <string_view>
#include <vector>

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/pipeline.hpp"

namespace
{

using namespace gandr::compile_host;

/// Drives one byte string through the decoder, the emitter, and the verifier.
///
/// A refused decode is a normal outcome, not a finding: the surface under test
/// is that refusal and acceptance are the only two outcomes.
void exercise(std::span<const std::uint8_t> bytes)
{
    const std::optional<Image> image = decode_image(bytes);
    if (!image.has_value()) {
        return;
    }
    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    const Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, *image);
    if (!module.has_value()) {
        return;
    }
    const Expected<void> verified = verify_module(module->get());
    if (!verified.has_value()) {
        return;
    }
    // The reference run. Its outcome is not asserted on — a generated program
    // may legitimately exhaust its heap or nest past the walk's bound — only
    // that reaching one of the two outcomes is all it ever does.
    const Expected<RunOutcome> outcome = interpret_image(*image);
    (void)outcome;
}

/// Reads a whole file as bytes.
[[nodiscard]] std::optional<std::vector<std::uint8_t>> read_file(std::string_view path)
{
    std::ifstream file(std::string(path), std::ios::binary);
    if (!file) {
        return std::nullopt;
    }
    return std::vector<std::uint8_t>(
        (std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
}

/// The largest input the entry reads, so a hostile length cannot exhaust
/// memory before the decoder's own bound applies.
constexpr std::size_t max_input_bytes = 1 << 20;

} // namespace

int main(int argc, char** argv)
{
    const std::vector<std::string_view> arguments(argv + 1, argv + argc);

    if (!arguments.empty()) {
        int status = EXIT_SUCCESS;
        for (const std::string_view path : arguments) {
            const std::optional<std::vector<std::uint8_t>> bytes = read_file(path);
            if (!bytes.has_value()) {
                std::fprintf(stderr, "could not read %.*s\n", static_cast<int>(path.size()), path.data());
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
