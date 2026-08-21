// The host's typed failure surface.
//
// # Contract
// - requires: nothing.
// - ensures: every fallible host entry returns `Expected<T>`; no host path
//   throws, aborts, or reports a failure by returning a sentinel value.
// - provides: the one error vocabulary the CLI maps to exit classes.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_STATUS_HPP
#define GANDR_COMPILE_HOST_STATUS_HPP

#include <expected>
#include <string>
#include <string_view>
#include <utility>

namespace gandr::compile_host {

/// What kind of failure a host stage reported.
///
/// The kinds are the stage boundaries: a caller can tell a malformed input
/// from a rejected module from a compilation failure without parsing a
/// message.
enum class ErrorKind : std::uint8_t
{
  /// The byte input did not decode into a well-formed image.
  MalformedImage = 0,
  /// The emitted module failed the mandatory verifier wall.
  VerifierRejected = 1,
  /// A lowering stage could not translate an operation.
  LoweringFailed = 2,
  /// The conversion pipeline to the LLVM dialect failed.
  ConversionFailed = 3,
  /// The execution engine could not be created or could not resolve the
  /// entry point.
  ExecutionFailed = 4,
  /// A run produced a heap the renderer could not read as a value.
  ResultUnreadable = 5,
  /// A host-side resource limit was reached: nesting depth, heap size, or
  /// image size.
  LimitExceeded = 6,
  /// A file the host was asked to read could not be read.
  FixtureUnreadable = 7,
};

/// A typed host failure: its kind and a message naming the site.
struct HostError
{
  /// Which stage boundary rejected.
  ErrorKind kind = ErrorKind::MalformedImage;
  /// A message naming what was rejected and why.
  std::string detail;
};

/// The host's fallible-result alias.
template<typename T>
using Expected = std::expected<T, HostError>;

/// Builds a typed failure.
///
/// # Contract
/// - ensures: the returned value is an `std::unexpected` carrying `kind` and
///   `detail`.
/// - panics: none.
[[nodiscard]] inline std::unexpected<HostError>
host_error(ErrorKind kind, std::string detail)
{
  return std::unexpected(HostError{ kind, std::move(detail) });
}

/// The stable short name of an error kind.
///
/// # Contract
/// - ensures: total, and the spellings are the ones the CLI prints, so a
///   caller can match on them.
/// - panics: none.
[[nodiscard]] constexpr std::string_view
error_kind_name(ErrorKind kind) noexcept
{
  switch (kind) {
    case ErrorKind::MalformedImage:
      return "malformed-image";
    case ErrorKind::VerifierRejected:
      return "verifier-rejected";
    case ErrorKind::LoweringFailed:
      return "lowering-failed";
    case ErrorKind::ConversionFailed:
      return "conversion-failed";
    case ErrorKind::ExecutionFailed:
      return "execution-failed";
    case ErrorKind::ResultUnreadable:
      return "result-unreadable";
    case ErrorKind::LimitExceeded:
      return "limit-exceeded";
    case ErrorKind::FixtureUnreadable:
      return "fixture-unreadable";
  }
  return "unknown";
}

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_STATUS_HPP
