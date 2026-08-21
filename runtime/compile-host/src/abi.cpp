#include "gandr/compile_host/abi.h"

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/interpret.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/status.hpp"

#include <cstdint>
#include <cstring>
#include <new>
#include <optional>
#include <span>
#include <string_view>

namespace {

using gandr::compile_host::ErrorKind;
using gandr::compile_host::Expected;
using gandr::compile_host::Image;
using gandr::compile_host::RunOutcome;

/// The boundary status one host error kind maps to.
///
/// The mapping is spelled out rather than computed from the enumerator value,
/// so that reordering the C++ enum cannot silently renumber the C boundary.
[[nodiscard]] auto
status_of(ErrorKind kind) noexcept -> std::int32_t
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
[[nodiscard]] auto
borrowed_empty_text() noexcept -> char const*
{
  // One NUL with static storage: a valid empty C string, and an address the
  // owned-buffer test can compare against. An array would be the same object
  // with a C declaration form the conventions do not want.
  static constexpr char empty = '\0';
  return &empty;
}

/// Copies a message into a buffer the caller releases.
///
/// The parameter is a **view** rather than a string, and that is the whole
/// point of it: every caller here is `noexcept`, so materializing a
/// `std::string` on the way in would put an allocation that can throw inside
/// a function that may not, and a failed one would call `std::terminate`
/// instead of reporting. A view allocates nothing, which leaves the nothrow
/// `char[]` below as the only allocation on the boundary.
///
/// A failed allocation yields `borrowed_empty_text` rather than propagating:
/// the boundary promises a non-null `text` on every call that returns, and
/// losing a message is a smaller failure than losing the status beside it.
[[nodiscard]] auto
own_text(std::string_view text) noexcept -> char const*
{
  // The boundary hands a foreign caller a raw pointer it releases through
  // gandr_compile_host_outcome_release; there is no smart pointer that
  // survives a C ABI, and gsl is not a dependency this boundary takes.
  // NOLINTNEXTLINE(cppcoreguidelines-owning-memory)
  char* owned = new (std::nothrow) char[text.size() + 1];
  if (owned == nullptr) {
    return borrowed_empty_text();
  }
  std::span<char> const buffer(owned, text.size() + 1);
  std::memcpy(buffer.data(), text.data(), text.size());
  buffer.back() = '\0';
  return owned;
}

// The boundary's no-abort promise rests on this: `own_text` is callable
// without throwing, so the `noexcept` functions below cannot terminate on the
// message path. A parameter that had to allocate would fail this.
static_assert(noexcept(own_text(std::string_view{})), "the message path must not throw");

/// Whether a pointer `own_text` returned owns heap storage.
[[nodiscard]] auto
is_owned_text(char const* text) noexcept -> bool
{
  return text != nullptr && text != borrowed_empty_text();
}

/// Fills an outcome from a host result.
void
fill(GandrCompileHostOutcome& outcome, Expected<RunOutcome> const& result) noexcept
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
void
refuse(GandrCompileHostOutcome& outcome, std::string_view detail) noexcept
{
  outcome.status = GANDR_COMPILE_HOST_STATUS_BAD_CALL;
  outcome.duplications = 0;
  outcome.discards = 0;
  outcome.allocated_words = 0;
  outcome.text = own_text(detail);
}

/// Decodes the caller's bytes, or reports the refusal into the outcome.
///
/// `noexcept` because the target is built `-fno-exceptions`: the decoder's
/// arena allocation cannot report a failure by throwing, so a caller has
/// nothing to catch. A malformed image is still a status, which is the only
/// failure of this function that is recoverable at all.
[[nodiscard]] auto
decode_or_refuse(std::uint8_t const* bytes, std::size_t length, GandrCompileHostOutcome& outcome) noexcept
  -> std::optional<Image>
{
  std::span<std::uint8_t const> const input(bytes, length);
  std::optional<Image> image = gandr::compile_host::decode_image(input);
  if (!image.has_value()) {
    outcome.status = GANDR_COMPILE_HOST_STATUS_MALFORMED_IMAGE;
    outcome.duplications = 0;
    outcome.discards = 0;
    outcome.allocated_words = 0;
    outcome.text = own_text("the byte image did not decode");
  }
  return image;
}

} // namespace

extern "C"
{

  auto
  gandr_compile_host_abi_version(void) -> std::uint32_t
  {
    return GANDR_COMPILE_HOST_ABI_VERSION;
  }

  auto
  gandr_compile_host_run(std::uint8_t const* bytes, std::size_t length, GandrCompileHostOutcome* outcome)
    -> std::int32_t
  {
    if (outcome == nullptr) {
      return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
      refuse(*outcome, "the image pointer is null with a nonzero length");
      return outcome->status;
    }
    std::optional<Image> const image = decode_or_refuse(bytes, length, *outcome);
    if (!image.has_value()) {
      return outcome->status;
    }
    fill(*outcome, gandr::compile_host::compile_and_run(*image));
    return outcome->status;
  }

  auto
  gandr_compile_host_run_with_heap(
    std::uint8_t const* bytes,
    std::size_t length,
    std::uint64_t heap_words,
    GandrCompileHostOutcome* outcome
  ) -> std::int32_t
  {
    if (outcome == nullptr) {
      return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
      refuse(*outcome, "the image pointer is null with a nonzero length");
      return outcome->status;
    }
    std::optional<Image> const image = decode_or_refuse(bytes, length, *outcome);
    if (!image.has_value()) {
      return outcome->status;
    }
    fill(*outcome, gandr::compile_host::compile_and_run_with_heap(*image, static_cast<std::size_t>(heap_words)));
    return outcome->status;
  }

  auto
  gandr_compile_host_interpret(std::uint8_t const* bytes, std::size_t length, GandrCompileHostOutcome* outcome)
    -> std::int32_t
  {
    if (outcome == nullptr) {
      return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    if (bytes == nullptr && length != 0) {
      refuse(*outcome, "the image pointer is null with a nonzero length");
      return outcome->status;
    }
    std::optional<Image> const image = decode_or_refuse(bytes, length, *outcome);
    if (!image.has_value()) {
      return outcome->status;
    }
    fill(*outcome, gandr::compile_host::interpret_image(*image));
    return outcome->status;
  }

  void
  gandr_compile_host_outcome_release(GandrCompileHostOutcome* outcome)
  {
    if (outcome == nullptr) {
      return;
    }
    if (is_owned_text(outcome->text)) {
      // `text` is `char const*` in the boundary struct so a caller cannot
      // write through it, and releasing it is the one operation that must.
      // NOLINTNEXTLINE(cppcoreguidelines-owning-memory,cppcoreguidelines-pro-type-const-cast)
      delete[] const_cast<char*>(outcome->text);
    }
    outcome->text = nullptr;
  }

} // extern "C"
