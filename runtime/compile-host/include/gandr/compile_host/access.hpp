// Indexing a contiguous range whose bound the caller has already established.
//
// # Contract
// - requires: nothing of this header; each entry states its own requirement.
// - ensures: no indexing helper here reads or writes outside a range.
// - provides: the one spelling for the third form of checked indexing the
//   host's totality rule admits.
// - panics: `proved_at` aborts in a checked build when its requirement does
//   not hold.

#ifndef GANDR_COMPILE_HOST_ACCESS_HPP
#define GANDR_COMPILE_HOST_ACCESS_HPP

#include <cassert>
#include <cstddef>
#include <utility>

namespace gandr::compile_host {

/// Indexes a contiguous range at a position the caller has already proved.
///
/// The totality rule admits three forms of indexing: a checked accessor, a
/// span with checked accessors, or a preceding bounds proof. The third form is
/// what a host walk actually has — the extent was compared a few lines above,
/// or the image validator refused every index that could reach here — and a
/// bare subscript cannot say so. This is that form, named: the proof stays
/// with the caller, and the access records that one exists.
///
/// It is not a substitute for the first two forms. Reach for it only where the
/// proof is in the same function or in an invariant the contract names, and
/// state which in the caller's own contract.
///
/// # Contract
/// - requires: `index` is less than `range.size()`, established before the
///   call and named in the caller's contract.
/// - ensures: the element at `index`, as whatever reference or proxy the
///   range's own subscript yields.
/// - panics: aborts in a checked build when the requirement does not hold, so
///   a wrong proof is a failing test rather than a silent read.
template<typename Range>
[[nodiscard]] constexpr auto
proved_at(Range&& range, std::size_t index) noexcept -> decltype(auto)
{
  assert(index < static_cast<std::size_t>(range.size()));
  // The suppression is the point of the function: every proved access in the
  // host routes through this one subscript, so there is one place to audit
  // rather than fifty.
  // NOLINTNEXTLINE(cppcoreguidelines-pro-bounds-avoid-unchecked-container-access)
  return std::forward<Range>(range)[index];
}

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_ACCESS_HPP
