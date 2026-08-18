#include "gandr/compile_host/abi.h"

#include <cstring>
#include <exception>
#include <new>
#include <optional>
#include <span>
#include <string>
#include <vector>

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/status.hpp"

namespace
{

using gandr::compile_host::ErrorKind;
using gandr::compile_host::Expected;
using gandr::compile_host::Image;
using gandr::compile_host::RunOutcome;

/// The boundary status one host error kind maps to.
///
/// The mapping is spelled out rather than computed from the enumerator value,
/// so that reordering the C++ enum cannot silently renumber the C boundary.
[[nodiscard]] std::int32_t status_of(ErrorKind kind) noexcept
{
    switch (kind) {
    case ErrorKind::MalformedImage:
        return GANDR_COMPILE_HOST_STATUS_MALFORMED_IMAGE;
    case ErrorKind::VerifierRejected:
        return GANDR_COMPILE_HOST_STATUS_VERIFIER_REJECTED;
    case ErrorKind::LoweringFailed:
        return GANDR_COMPILE_HOST_STATUS_LOWERING_FAILED;
    case ErrorKind::ConversionFailed:
        return GANDR_COMPILE_HOST_STATUS_CONVERSION_FAILED;
    case ErrorKind::ExecutionFailed:
        return GANDR_COMPILE_HOST_STATUS_EXECUTION_FAILED;
    case ErrorKind::ResultUnreadable:
        return GANDR_COMPILE_HOST_STATUS_RESULT_UNREADABLE;
    case ErrorKind::LimitExceeded:
        return GANDR_COMPILE_HOST_STATUS_LIMIT_EXCEEDED;
    case ErrorKind::FixtureUnreadable:
        return GANDR_COMPILE_HOST_STATUS_FIXTURE_UNREADABLE;
    }
    return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
}

/// The one static empty string the out-of-memory path hands back.
///
/// It is a named function rather than a literal because release has to
/// recognize it: the boundary promises a non-null message, and the pointer
/// identity is what distinguishes the borrowed fallback from an owned buffer
/// without adding a second field to the struct.
[[nodiscard]] const char* borrowed_empty_text() noexcept
{
    static const char empty[] = "";
    return static_cast<const char*>(empty);
}

/// Copies a message into a buffer the caller releases.
///
/// A failed allocation yields `borrowed_empty_text` rather than propagating:
/// the boundary promises a non-null `text` on every call that returns, and
/// losing a message is a smaller failure than losing the status beside it.
[[nodiscard]] const char* own_text(const std::string& text) noexcept
{
    char* owned = new (std::nothrow) char[text.size() + 1];
    if (owned == nullptr) {
        return borrowed_empty_text();
    }
    std::memcpy(owned, text.data(), text.size());
    owned[text.size()] = '\0';
    return owned;
}

/// Whether a pointer `own_text` returned owns heap storage.
[[nodiscard]] bool is_owned_text(const char* text) noexcept
{
    return text != nullptr && text != borrowed_empty_text();
}

/// Fills an outcome from a host result.
void fill(GandrCompileHostOutcome& outcome, const Expected<RunOutcome>& result) noexcept
{
    if (result.has_value()) {
        outcome.status = GANDR_COMPILE_HOST_STATUS_OK;
        outcome.duplications = result->ledger.duplications;
        outcome.discards = result->ledger.discards;
        outcome.allocated_words = static_cast<std::uint64_t>(result->allocated);
        outcome.text = own_text(result->value);
        return;
    }
    outcome.status = status_of(result.error().kind);
    outcome.duplications = 0;
    outcome.discards = 0;
    outcome.allocated_words = 0;
    outcome.text = own_text(result.error().detail);
}

/// Fills an outcome for a call the boundary itself refused.
void refuse(GandrCompileHostOutcome& outcome, const char* detail) noexcept
{
    outcome.status = GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    outcome.duplications = 0;
    outcome.discards = 0;
    outcome.allocated_words = 0;
    outcome.text = own_text(std::string(detail));
}

/// Decodes the caller's bytes, or reports the refusal into the outcome.
[[nodiscard]] std::optional<Image> decode_or_refuse(
    const std::uint8_t* bytes,
    std::size_t length,
    GandrCompileHostOutcome& outcome) noexcept
{
    const std::span<const std::uint8_t> input(bytes, length);
    std::optional<Image> image = gandr::compile_host::decode_image(input);
    if (!image.has_value()) {
        outcome.status = GANDR_COMPILE_HOST_STATUS_MALFORMED_IMAGE;
        outcome.duplications = 0;
        outcome.discards = 0;
        outcome.allocated_words = 0;
        outcome.text = own_text(std::string("the byte image did not decode"));
    }
    return image;
}

} // namespace

extern "C" {

std::uint32_t gandr_compile_host_abi_version(void)
{
    return GANDR_COMPILE_HOST_ABI_VERSION;
}

std::int32_t gandr_compile_host_run(
    const std::uint8_t* bytes,
    std::size_t length,
    GandrCompileHostOutcome* outcome)
{
    if (outcome == nullptr) {
        return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
        refuse(*outcome, "the image pointer is null with a nonzero length");
        return outcome->status;
    }
    try {
        const std::optional<Image> image = decode_or_refuse(bytes, length, *outcome);
        if (!image.has_value()) {
            return outcome->status;
        }
        fill(*outcome, gandr::compile_host::compile_and_run(*image));
    } catch (const std::exception& raised) {
        refuse(*outcome, raised.what());
    } catch (...) {
        refuse(*outcome, "the host raised a non-standard exception");
    }
    return outcome->status;
}

std::int32_t gandr_compile_host_run_with_heap(
    const std::uint8_t* bytes,
    std::size_t length,
    std::uint64_t heap_words,
    GandrCompileHostOutcome* outcome)
{
    if (outcome == nullptr) {
        return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
        refuse(*outcome, "the image pointer is null with a nonzero length");
        return outcome->status;
    }
    try {
        const std::optional<Image> image = decode_or_refuse(bytes, length, *outcome);
        if (!image.has_value()) {
            return outcome->status;
        }
        fill(
            *outcome,
            gandr::compile_host::compile_and_run_with_heap(
                *image, static_cast<std::size_t>(heap_words)));
    } catch (const std::exception& raised) {
        refuse(*outcome, raised.what());
    } catch (...) {
        refuse(*outcome, "the host raised a non-standard exception");
    }
    return outcome->status;
}

std::int32_t gandr_compile_host_interpret(
    const std::uint8_t* bytes,
    std::size_t length,
    GandrCompileHostOutcome* outcome)
{
    if (outcome == nullptr) {
        return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
        refuse(*outcome, "the image pointer is null with a nonzero length");
        return outcome->status;
    }
    try {
        const std::optional<Image> image = decode_or_refuse(bytes, length, *outcome);
        if (!image.has_value()) {
            return outcome->status;
        }
        fill(*outcome, gandr::compile_host::interpret_image(*image));
    } catch (const std::exception& raised) {
        refuse(*outcome, raised.what());
    } catch (...) {
        refuse(*outcome, "the host raised a non-standard exception");
    }
    return outcome->status;
}

void gandr_compile_host_outcome_release(GandrCompileHostOutcome* outcome)
{
    if (outcome == nullptr) {
        return;
    }
    if (is_owned_text(outcome->text)) {
        delete[] const_cast<char*>(outcome->text);
    }
    outcome->text = nullptr;
}

} // extern "C"
