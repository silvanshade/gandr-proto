// The compile host's regression suite.
//
// One binary, one case per named check; CTest registers each case by name, so
// a failure names the property that broke rather than a file. The suite runs
// every case when given no selector.
//
// # Contract
// - requires: the fixtures directory, which CMake passes as
//   `GANDR_COMPILE_HOST_FIXTURES`.
// - ensures: a nonzero exit status if any selected case failed.
// - provides: the observable evidence for the slice's claims.
// - panics: none; a failed check is a printed line and an exit status.

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <map>
#include <span>
#include <string>
#include <string_view>
#include <vector>

#include "mlir/IR/Verifier.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

#include "gandr/compile_host/abi.h"
#include "gandr/compile_host/dialect.hpp"
#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/pipeline.hpp"
#include "gandr/compile_host/samples.hpp"
#include "gandr/compile_host/status.hpp"

namespace
{

using namespace gandr::compile_host;

/// How many failures the running case has recorded.
int failure_count = 0;

/// Records a failed check with its message.
void check(bool holds, std::string_view what)
{
    if (!holds) {
        failure_count += 1;
        std::fprintf(stderr, "  FAILED: %.*s\n", static_cast<int>(what.size()), what.data());
    }
}

/// Records a failed equality check, printing both sides.
void check_equal(std::string_view actual, std::string_view expected, std::string_view what)
{
    if (actual != expected) {
        failure_count += 1;
        std::fprintf(
            stderr,
            "  FAILED: %.*s\n    actual:   %.*s\n    expected: %.*s\n",
            static_cast<int>(what.size()),
            what.data(),
            static_cast<int>(actual.size()),
            actual.data(),
            static_cast<int>(expected.size()),
            expected.data());
    }
}

/// The fixtures directory CMake configured in.
[[nodiscard]] std::filesystem::path fixtures_root()
{
    return std::filesystem::path(GANDR_COMPILE_HOST_FIXTURES);
}

/// Reads the agreement fixture as name-to-value rows.
[[nodiscard]] std::map<std::string, std::string> read_agreement_fixture()
{
    std::map<std::string, std::string> rows;
    std::ifstream file(fixtures_root() / "positive-core-samples.txt");
    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line.front() == '#') {
            continue;
        }
        const std::size_t separator = line.find('\t');
        if (separator == std::string::npos) {
            continue;
        }
        rows.emplace(line.substr(0, separator), line.substr(separator + 1));
    }
    return rows;
}

/// Every named program, canonical set plus the accounted-work program.
[[nodiscard]] std::vector<Sample> all_samples()
{
    std::vector<Sample> samples = canonical_samples();
    samples.push_back(accounted_work_sample());
    return samples;
}

// ---------------------------------------------------------------------------
// The cases.
// ---------------------------------------------------------------------------

/// The arity a constructor tag declares is enforced by the verifier, not the
/// builder — the price the proving spike measured, pinned as a fixture.
void case_arity_verifier_rejects_a_malformed_constructor()
{
    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    mlir::OwningOpRef<mlir::ModuleOp> module = emit_malformed_arity_module(*context);
    check(static_cast<bool>(module), "the malformed constructor builds at all");
    check(
        count_dialect_operations(module.get(), "ctor") == 1,
        "the malformed module holds the constructor the builder accepted");

    const Expected<void> verdict = verify_module(module.get());
    check(!verdict.has_value(), "the verifier rejects the malformed constructor");
    if (!verdict.has_value()) {
        check(
            verdict.error().kind == ErrorKind::VerifierRejected,
            "the rejection is the verifier's, not another stage's");
        check(
            verdict.error().detail.find("declares arity 2 but was given 1") != std::string::npos,
            "the rejection names the declared arity and the supplied count");
    }

    // A well-formed constructor passes the same wall, so the fixture is
    // discriminating rather than a verifier that rejects everything.
    const std::vector<Sample> samples = canonical_samples();
    for (const Sample& sample : samples) {
        if (sample.name != "ctor") {
            continue;
        }
        Expected<mlir::OwningOpRef<mlir::ModuleOp>> healthy = emit_module(*context, sample.image);
        check(healthy.has_value(), "the well-formed constructor emits");
        if (healthy.has_value()) {
            check(verify_module(healthy->get()).has_value(), "the well-formed constructor verifies");
        }
    }
}

/// Duplication and discard survive canonicalization, and they survive it
/// because they declare memory effects rather than because nothing looked.
void case_canonicalization_preserves_accounted_work()
{
    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    mlir::OwningOpRef<mlir::ModuleOp> module = emit_accounted_work_witness_module(*context);

    check(count_dialect_operations(module.get(), "dup") == 1, "the witness starts with one duplication");
    check(count_dialect_operations(module.get(), "drop") == 1, "the witness starts with one discard");
    check(count_dialect_operations(module.get(), "lit") == 2, "the witness starts with two literals");

    // Neither operation's result is observed anywhere in the module, so a pure
    // operation here would be dead code.
    module->walk([](mlir::Operation* op) {
        const llvm::StringRef name = op->getName().getStringRef();
        if (name == "gandr.dup" || name == "gandr.drop") {
            check(op->use_empty(), "the accounted operation's result is unobserved");
            check(
                !mlir::isMemoryEffectFree(op),
                "the accounted operation declares an effect the optimizer must respect");
        }
    });

    const Expected<void> optimized =
        optimize_module(module.get(), Optimization::CanonicalizeAndDeduplicate);
    check(optimized.has_value(), "the witness survives the optimization pipeline");

    check(
        count_dialect_operations(module.get(), "dup") == 1,
        "canonicalization did not delete the duplication");
    check(
        count_dialect_operations(module.get(), "drop") == 1,
        "canonicalization did not delete the discard");
    check(
        count_dialect_operations(module.get(), "lit") == 2,
        "canonicalization did not delete the literal the accounted work reads");
}

/// The decoder is total: arbitrary bytes either decode into a well-formed
/// image or are refused, and never anything else.
void case_decoder_is_total_on_seed_corpus()
{
    const std::filesystem::path seeds = fixtures_root() / "fuzz-seeds";
    std::size_t examined = 0;
    std::size_t accepted = 0;
    std::size_t refused_at_a_limit = 0;

    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    for (const std::filesystem::directory_entry& entry : std::filesystem::directory_iterator(seeds)) {
        std::ifstream file(entry.path(), std::ios::binary);
        const std::vector<std::uint8_t> bytes(
            (std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());

        // A committed seed that no longer decodes is dead weight the campaign
        // would carry without anyone noticing, so each one is checked on its
        // own before the edits start.
        check(
            decode_image(bytes).has_value(),
            std::string("the committed seed ") + entry.path().filename().string() + " decodes");

        // The seed itself, then a deterministic sweep of single-byte edits, so
        // the case exercises the refusal paths as well as the accepting one.
        for (std::size_t position = 0; position <= bytes.size(); ++position) {
            std::vector<std::uint8_t> mutated = bytes;
            if (position < mutated.size()) {
                mutated[position] = static_cast<std::uint8_t>(mutated[position] ^ 0xA5U);
            }
            examined += 1;
            const std::optional<Image> decoded = decode_image(mutated);
            if (!decoded.has_value()) {
                continue;
            }
            accepted += 1;
            check(image_is_wellformed(*decoded), "a decoded image is well-formed");

            // A decoded image either emits and verifies, or is refused for a
            // stated reason. The nesting bound is the reason a well-formed
            // image can be refused here, and a seed sits on each side of it.
            const Expected<mlir::OwningOpRef<mlir::ModuleOp>> emitted =
                emit_module(*context, *decoded);
            if (emitted.has_value()) {
                check(verify_module(emitted->get()).has_value(), "a decoded image's module verifies");
            }
            else {
                check(
                    emitted.error().kind == ErrorKind::LimitExceeded,
                    "a decoded image is refused only by a stated limit");
                refused_at_a_limit += 1;
            }
        }
    }

    check(examined > 0, "the seed corpus is not empty");
    check(accepted > 0, "at least one input decodes, so the case is not vacuous");
    check(
        refused_at_a_limit > 0,
        "at least one seed sits past the nesting bound, so the limit path is exercised");
}

/// The wire form round-trips: an image encodes and decodes back to itself.
void case_encode_decode_round_trips_every_sample()
{
    for (const Sample& sample : all_samples()) {
        const std::vector<std::uint8_t> bytes = encode_image(sample.image);
        const std::optional<Image> decoded = decode_image(bytes);
        check(decoded.has_value(), "an encoded sample decodes");
        if (!decoded.has_value()) {
            continue;
        }
        check(decoded->nodes.size() == sample.image.nodes.size(), "the round trip keeps the node count");
        check(decoded->root == sample.image.root, "the round trip keeps the root");

        const Expected<RunOutcome> original = interpret_image(sample.image);
        const Expected<RunOutcome> restored = interpret_image(*decoded);
        check(original.has_value() && restored.has_value(), "both forms evaluate");
        if (original.has_value() && restored.has_value()) {
            check_equal(restored->value, original->value, "the round trip keeps the answer");
        }
    }
}

/// The compiled path agrees with the L machine, through the fixture the Rust
/// side pins.
void case_jit_agrees_with_the_fixture_on_every_sample()
{
    const std::map<std::string, std::string> expected = read_agreement_fixture();
    check(expected.size() == 8, "the fixture states every named program");

    for (const Sample& sample : all_samples()) {
        const auto row = expected.find(std::string(sample.name));
        check(row != expected.end(), "the fixture names this program");
        if (row == expected.end()) {
            continue;
        }
        const Expected<RunOutcome> outcome = compile_and_run(sample.image);
        check(outcome.has_value(), "the program compiles and runs");
        if (!outcome.has_value()) {
            std::fprintf(stderr, "    %s\n", outcome.error().detail.c_str());
            continue;
        }
        check_equal(outcome->value, row->second, "the compiled answer matches the L machine's");
    }
}

/// The compiled path agrees with the reference interpreter on generated
/// programs, which is where the seven named samples stop reaching.
void case_jit_agrees_with_the_reference_interpreter_under_generation()
{
    constexpr std::uint64_t case_count = 64;
    // Which node kinds the generated set actually exercised. A property that
    // never generated a dispatch would pass while proving nothing about one,
    // so the coverage is asserted rather than assumed.
    std::map<std::uint8_t, std::size_t> kinds_seen;

    for (std::uint64_t seed = 1; seed <= case_count; ++seed) {
        const Image image = generate_image(seed);
        check(image_is_wellformed(image), "the generator produced a well-formed image");
        for (const Node& node : image.nodes) {
            kinds_seen[static_cast<std::uint8_t>(node.kind)] += 1;
        }

        const Expected<RunOutcome> reference = interpret_image(image);
        const Expected<RunOutcome> compiled = compile_and_run(image);
        check(
            reference.has_value() == compiled.has_value(),
            "the two paths agree on whether the program runs");
        if (!reference.has_value() || !compiled.has_value()) {
            std::fprintf(stderr, "    seed %llu\n", static_cast<unsigned long long>(seed));
            continue;
        }
        check_equal(compiled->value, reference->value, "the two paths agree on the answer");
        check(
            compiled->ledger == reference->ledger,
            "the two paths account for the same duplications and discards");
        if (compiled->ledger != reference->ledger) {
            std::fprintf(
                stderr,
                "    seed %llu: compiled %lld/%lld, reference %lld/%lld\n",
                static_cast<unsigned long long>(seed),
                static_cast<long long>(compiled->ledger.duplications),
                static_cast<long long>(compiled->ledger.discards),
                static_cast<long long>(reference->ledger.duplications),
                static_cast<long long>(reference->ledger.discards));
        }
    }

    for (std::uint8_t kind = static_cast<std::uint8_t>(NodeKind::Lit);
         kind <= static_cast<std::uint8_t>(NodeKind::Cut);
         ++kind) {
        check(
            kinds_seen[kind] > 0,
            "the generated set exercised every positive-core node kind");
    }
}

/// The compiled code executes the accounting, rather than only declaring it.
void case_ledger_records_every_executed_duplication_and_discard()
{
    const Sample sample = accounted_work_sample();
    const Expected<RunOutcome> outcome = compile_and_run(sample.image);
    check(outcome.has_value(), "the accounted-work program compiles and runs");
    if (!outcome.has_value()) {
        std::fprintf(stderr, "    %s\n", outcome.error().detail.c_str());
        return;
    }
    check_equal(outcome->value, "(int 0)", "the answer mentions neither accounted operation");
    check(
        outcome->ledger == accounted_work_expectation,
        "the run recorded exactly one duplication and one discard");

    // The duplication and the discard in the seven-sample set are accounted
    // too, so the ledger is not a property of one program's shape.
    for (const Sample& named : canonical_samples()) {
        if (named.name != "dup" && named.name != "drop") {
            continue;
        }
        const Expected<RunOutcome> run = compile_and_run(named.image);
        check(run.has_value(), "the transition sample compiles and runs");
        if (!run.has_value()) {
            continue;
        }
        const std::int64_t expected = 1;
        check(
            (named.name == "dup" ? run->ledger.duplications : run->ledger.discards) == expected,
            "the transition sample accounted its own work");
    }
}

/// The compiled code carries its own bounds check: it refuses an allocation
/// that would not fit, at the same word the reference walk refuses it.
///
/// The sizes are measured rather than predicted. Each path is run once on the
/// static bound, its consumed arena is read back, and the exact heap is that
/// consumption plus the reserved prefix — so the boundary case is the real
/// last word rather than an arithmetic guess about it.
void case_compiled_allocation_is_bounded_by_its_heap()
{
    for (const Sample& sample : all_samples()) {
        const std::size_t generous = heap_words_for(sample.image);

        const Expected<RunOutcome> reference = interpret_image_with_heap(sample.image, generous);
        const Expected<RunOutcome> compiled = compile_and_run_with_heap(sample.image, generous);
        check(reference.has_value(), "the reference walk runs on the static bound");
        check(compiled.has_value(), "the compiled run runs on the static bound");
        if (!reference.has_value() || !compiled.has_value()) {
            continue;
        }
        check_equal(compiled->value, reference->value, "the two paths agree on the static bound");
        check(
            compiled->allocated <= reference->allocated,
            "the compiled path allocates no more than the reference walk");

        const std::size_t reference_exact = HeapLayout::arena_base + reference->allocated;
        const std::size_t compiled_exact = HeapLayout::arena_base + compiled->allocated;

        const Expected<RunOutcome> reference_at_exact =
            interpret_image_with_heap(sample.image, reference_exact);
        check(reference_at_exact.has_value(), "the reference walk fits its own exact heap");
        if (reference_at_exact.has_value()) {
            check_equal(
                reference_at_exact->value,
                reference->value,
                "the exact heap does not change the reference answer");
        }

        const Expected<RunOutcome> compiled_at_exact =
            compile_and_run_with_heap(sample.image, compiled_exact);
        check(compiled_at_exact.has_value(), "the compiled run fits its own exact heap");
        if (compiled_at_exact.has_value()) {
            check_equal(
                compiled_at_exact->value,
                compiled->value,
                "the exact heap does not change the compiled answer");
            check(
                compiled_at_exact->ledger == compiled->ledger,
                "the exact heap does not change the accounted work");
        }

        // One word short of what the run consumed. A program that allocates
        // nothing has no such case, and saying so is cheaper than pretending
        // it does.
        if (reference->allocated > 0) {
            const Expected<RunOutcome> starved =
                interpret_image_with_heap(sample.image, reference_exact - 1);
            check(!starved.has_value(), "the reference walk refuses one word short");
            if (!starved.has_value()) {
                check(
                    starved.error().kind == ErrorKind::LimitExceeded,
                    "the reference refusal names the limit, not another stage");
            }
        }
        if (compiled->allocated > 0) {
            const Expected<RunOutcome> starved =
                compile_and_run_with_heap(sample.image, compiled_exact - 1);
            check(!starved.has_value(), "the compiled run refuses one word short");
            if (!starved.has_value()) {
                check(
                    starved.error().kind == ErrorKind::LimitExceeded,
                    "the compiled refusal names the limit, not another stage");
            } else {
                std::fprintf(
                    stderr,
                    "    %.*s answered %s on a heap one word short\n",
                    static_cast<int>(sample.name.size()),
                    sample.name.data(),
                    starved->value.c_str());
            }
        }

        // A heap too small even for the reserved prefix is refused before the
        // entry point is invoked at all, because the prefix is what the
        // refusal itself is written into.
        const Expected<RunOutcome> unusable =
            compile_and_run_with_heap(sample.image, HeapLayout::arena_base - 1);
        check(!unusable.has_value(), "a heap below the reserved prefix is refused");
        if (!unusable.has_value()) {
            check(
                unusable.error().kind == ErrorKind::LimitExceeded,
                "the prefix refusal names the limit");
        }
    }
}

/// A refused run writes nothing past the extent it was given.
///
/// The heap handed to the entry point is a prefix of a longer buffer whose
/// remaining words carry a sentinel. The compiled code learns its bound from
/// the descriptor, so an unchecked allocation would run into the sentinel
/// region and be visible afterwards; a checked one cannot reach it.
void case_a_refused_run_writes_nothing_past_its_heap()
{
    constexpr std::int64_t sentinel = -987654321;
    constexpr std::size_t canary_words = 32;

    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    std::size_t exercised = 0;

    for (const Sample& sample : all_samples()) {
        const Expected<RunOutcome> measured = compile_and_run(sample.image);
        check(measured.has_value(), "the sample runs on the static bound");
        if (!measured.has_value() || measured->allocated == 0) {
            continue;
        }

        Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, sample.image);
        check(module.has_value(), "the sample emits");
        if (!module.has_value()) {
            continue;
        }
        const Expected<void> lowered =
            lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
        check(lowered.has_value(), "the sample lowers");
        if (!lowered.has_value()) {
            continue;
        }

        // One word short of what the run needs, so the last allocation is the
        // one that must be refused.
        const std::size_t offered = HeapLayout::arena_base + measured->allocated - 1;
        std::vector<std::int64_t> buffer(offered + canary_words, sentinel);
        const std::span<std::int64_t> heap(buffer.data(), offered);

        const Expected<RunOutcome> outcome = run_lowered_module_on(module->get(), heap);
        check(!outcome.has_value(), "the run on the short heap is refused");
        if (!outcome.has_value()) {
            check(
                outcome.error().kind == ErrorKind::LimitExceeded,
                "the refusal names the limit");
        }

        std::size_t disturbed = 0;
        for (std::size_t index = offered; index < buffer.size(); ++index) {
            if (buffer[index] != sentinel) {
                disturbed += 1;
            }
        }
        check(disturbed == 0, "no word past the offered heap was written");
        exercised += 1;
    }

    check(exercised > 0, "at least one sample allocates, so the case is not vacuous");
}

/// The C boundary owns the text it hands back, releases it, and reaches both
/// of the paths that produce one.
///
/// The two paths matter separately. A **literal** message is what the boundary
/// itself writes when it refuses a call; an **owned** message is what a run's
/// answer or a stage's detail carries out of the host. Both go through the one
/// nothrow allocation, and this case is what makes the claim observable rather
/// than a property of reading the source.
void case_the_c_boundary_owns_and_releases_every_message()
{
    // The owned path: a real run's rendered answer crosses the boundary.
    {
        const Sample sample = accounted_work_sample();
        const std::vector<std::uint8_t> bytes = encode_image(sample.image);
        GandrCompileHostOutcome outcome{};
        const std::int32_t status =
            gandr_compile_host_run(bytes.data(), bytes.size(), &outcome);
        check(status == GANDR_COMPILE_HOST_STATUS_OK, "a well-formed image runs across the boundary");
        check(status == outcome.status, "the return value repeats the outcome's status");
        check(outcome.text != nullptr, "a returned call always carries a message");
        if (outcome.text != nullptr) {
            check_equal(outcome.text, "(int 0)", "the owned message is the rendered answer");
        }
        check(outcome.duplications == 1 && outcome.discards == 1, "the ledger crosses too");

        gandr_compile_host_outcome_release(&outcome);
        check(outcome.text == nullptr, "release clears the pointer");
        // A second release is inert, which is what the cleared pointer buys.
        gandr_compile_host_outcome_release(&outcome);
        check(outcome.text == nullptr, "a second release changes nothing");
    }

    // The literal path: the decoder refuses, and the message is a static
    // string the boundary owns a copy of.
    {
        const std::vector<std::uint8_t> garbage{0xFF, 0xFF, 0xFF};
        GandrCompileHostOutcome outcome{};
        const std::int32_t status =
            gandr_compile_host_run(garbage.data(), garbage.size(), &outcome);
        check(
            status == GANDR_COMPILE_HOST_STATUS_MALFORMED_IMAGE,
            "undecodable bytes are refused at the decoder");
        check(outcome.text != nullptr, "a refusal carries its message");
        if (outcome.text != nullptr) {
            check_equal(
                outcome.text,
                "the byte image did not decode",
                "the literal message crosses intact");
        }
        gandr_compile_host_outcome_release(&outcome);
        check(outcome.text == nullptr, "the literal path releases like the owned one");
    }

    // The boundary's own refusals: a null outcome is a return value with
    // nothing written, and a null image with a length is a filled refusal.
    {
        check(
            gandr_compile_host_run(nullptr, 0, nullptr) == GANDR_COMPILE_HOST_STATUS_BAD_CALL,
            "a null outcome is refused by return value alone");

        GandrCompileHostOutcome outcome{};
        const std::int32_t status = gandr_compile_host_run(nullptr, 7, &outcome);
        check(status == GANDR_COMPILE_HOST_STATUS_BAD_CALL, "a null image with a length is refused");
        check(outcome.text != nullptr, "the refusal carries its message");
        gandr_compile_host_outcome_release(&outcome);
    }

    // The version is what the header declares, which is what a caller checks
    // before it calls anything else.
    check(
        gandr_compile_host_abi_version() == GANDR_COMPILE_HOST_ABI_VERSION,
        "the built library declares the header's version");

    // The sized entry reports exhaustion across the boundary rather than
    // answering, so the bounds check is visible to a foreign caller.
    {
        const Sample sample = accounted_work_sample();
        const std::vector<std::uint8_t> bytes = encode_image(sample.image);
        GandrCompileHostOutcome outcome{};
        const std::int32_t status = gandr_compile_host_run_with_heap(
            bytes.data(), bytes.size(), HeapLayout::arena_base, &outcome);
        check(
            status == GANDR_COMPILE_HOST_STATUS_LIMIT_EXCEEDED,
            "a heap with no arena is reported as a limit across the boundary");
        gandr_compile_host_outcome_release(&outcome);
    }
}

/// Lowering is complete: no operation of the gandr dialect survives it.
void case_lowering_leaves_no_dialect_operation_behind()
{
    const std::unique_ptr<mlir::MLIRContext> context = make_context();
    for (const Sample& sample : all_samples()) {
        Expected<mlir::OwningOpRef<mlir::ModuleOp>> module = emit_module(*context, sample.image);
        check(module.has_value(), "the program emits");
        if (!module.has_value()) {
            continue;
        }
        const Expected<void> lowered =
            lower_module(module->get(), Optimization::CanonicalizeAndDeduplicate);
        check(lowered.has_value(), "the program lowers");
        if (!lowered.has_value()) {
            std::fprintf(stderr, "    %s\n", lowered.error().detail.c_str());
            continue;
        }
        std::size_t survivors = 0;
        module->get()->walk([&survivors](mlir::Operation* op) {
            if (op->getName().getDialectNamespace() == "gandr") {
                survivors += 1;
            }
        });
        check(survivors == 0, "no gandr operation survived the lowering");
    }
}

/// The verifier wall opens the pipeline: a malformed module reaches neither
/// canonicalization nor lowering nor execution.
void case_verifier_runs_before_canonicalization_and_execution()
{
    const std::unique_ptr<mlir::MLIRContext> context = make_context();

    {
        mlir::OwningOpRef<mlir::ModuleOp> module = emit_malformed_arity_module(*context);
        const Expected<void> optimized =
            optimize_module(module.get(), Optimization::CanonicalizeAndDeduplicate);
        check(!optimized.has_value(), "optimization refuses a module the verifier rejects");
        if (!optimized.has_value()) {
            check(
                optimized.error().kind == ErrorKind::VerifierRejected,
                "the refusal is the verifier's, so no pass ran first");
        }
        check(
            count_dialect_operations(module.get(), "ctor") == 1,
            "the refused module is unchanged, so no pass touched it");
    }

    {
        mlir::OwningOpRef<mlir::ModuleOp> module = emit_malformed_arity_module(*context);
        const Expected<void> lowered =
            lower_module(module.get(), Optimization::CanonicalizeAndDeduplicate);
        check(!lowered.has_value(), "lowering refuses a module the verifier rejects");
        if (!lowered.has_value()) {
            check(
                lowered.error().kind == ErrorKind::VerifierRejected,
                "the refusal is the verifier's, so no lowering ran first");
        }
    }

    {
        // A structurally malformed image is refused one stage earlier still,
        // at the emitter, so nothing malformed is ever built into a module.
        Image malformed;
        malformed.nodes.push_back(
            Node{.kind = NodeKind::Var, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = {}});
        malformed.root = NodeIndex{0};
        check(!image_is_wellformed(malformed), "the malformed image fails the structural check");
        const Expected<RunOutcome> outcome = compile_and_run(malformed);
        check(!outcome.has_value(), "a malformed image never reaches execution");
        if (!outcome.has_value()) {
            check(
                outcome.error().kind == ErrorKind::MalformedImage,
                "the refusal names the image, not a later stage");
        }
    }
}

/// The registry of cases, in the order CTest declares them.
struct Case
{
    /// The case's name.
    std::string_view name;
    /// The case's body.
    void (*run)();
};

/// Every case the suite knows.
[[nodiscard]] std::vector<Case> registry()
{
    return {
        Case{"a_refused_run_writes_nothing_past_its_heap",
             case_a_refused_run_writes_nothing_past_its_heap},
        Case{"arity_verifier_rejects_a_malformed_constructor",
             case_arity_verifier_rejects_a_malformed_constructor},
        Case{"canonicalization_preserves_accounted_work", case_canonicalization_preserves_accounted_work},
        Case{"the_c_boundary_owns_and_releases_every_message",
             case_the_c_boundary_owns_and_releases_every_message},
        Case{"compiled_allocation_is_bounded_by_its_heap", case_compiled_allocation_is_bounded_by_its_heap},
        Case{"decoder_is_total_on_seed_corpus", case_decoder_is_total_on_seed_corpus},
        Case{"encode_decode_round_trips_every_sample", case_encode_decode_round_trips_every_sample},
        Case{"jit_agrees_with_the_fixture_on_every_sample", case_jit_agrees_with_the_fixture_on_every_sample},
        Case{"jit_agrees_with_the_reference_interpreter_under_generation",
             case_jit_agrees_with_the_reference_interpreter_under_generation},
        Case{"ledger_records_every_executed_duplication_and_discard",
             case_ledger_records_every_executed_duplication_and_discard},
        Case{"lowering_leaves_no_dialect_operation_behind", case_lowering_leaves_no_dialect_operation_behind},
        Case{"verifier_runs_before_canonicalization_and_execution",
             case_verifier_runs_before_canonicalization_and_execution},
    };
}

} // namespace

int main(int argc, char** argv)
{
    const std::vector<std::string_view> arguments(argv + 1, argv + argc);
    std::string_view selected;
    for (const std::string_view argument : arguments) {
        if (argument.starts_with("--case=")) {
            selected = argument.substr(std::string_view("--case=").size());
        }
    }

    int total_failures = 0;
    bool matched = false;
    for (const Case& registered : registry()) {
        if (!selected.empty() && registered.name != selected) {
            continue;
        }
        matched = true;
        failure_count = 0;
        std::fprintf(stderr, "case %.*s\n", static_cast<int>(registered.name.size()), registered.name.data());
        registered.run();
        total_failures += failure_count;
    }

    if (!matched) {
        std::fprintf(stderr, "no case named %.*s\n", static_cast<int>(selected.size()), selected.data());
        return EXIT_FAILURE;
    }
    return total_failures == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
