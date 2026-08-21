// The reference interpreter: a direct walk of a program image, independent of
// the compilation path.
//
// # Contract
// - requires: an image satisfying `image_is_wellformed`.
// - ensures: the same value rendering and the same work ledger the compiled
//   path produces, computed without MLIR.
// - provides: the differential partner the property harness compares the JIT
//   against on generated programs.
// - fails: `ErrorKind::LimitExceeded` past `max_emit_depth`.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_INTERPRET_HPP
#define GANDR_COMPILE_HOST_INTERPRET_HPP

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/status.hpp"

namespace gandr::compile_host {

/// Evaluates a program image directly.
///
/// The interpreter shares the heap layout and the value rendering with the
/// compiled path and shares nothing else: it walks the image rather than the
/// dialect, so a fault in the emitter or the lowering separates the two
/// answers. What it cannot catch is a fault in the shared layout, which is why
/// the canonical samples are additionally pinned against the Rust L machine.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: the outcome's rendering and ledger match the compiled path's on
///   every image both accept.
/// - fails: `ErrorKind::MalformedImage` for an image that does not verify;
///   `ErrorKind::LimitExceeded` past `max_emit_depth` or when an allocation
///   would not fit the heap.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome>
interpret_image(Image const& image);

/// Evaluates a program image directly on a heap of the caller's size.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: identical to `interpret_image` when `heap_words` is at least
///   what the walk needs; otherwise the walk stops at the first allocation
///   that does not fit and reports it, having written nothing outside the
///   heap.
/// - provides: the reference side of the bounds differential, so the compiled
///   path's refusal is compared against another implementation's rather than
///   against a rule stated once.
/// - fails: as `interpret_image`, `ErrorKind::LimitExceeded` included.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome>
interpret_image_with_heap(Image const& image, std::size_t heap_words);

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_INTERPRET_HPP
