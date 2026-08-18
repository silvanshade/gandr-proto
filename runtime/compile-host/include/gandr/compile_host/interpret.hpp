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

namespace gandr::compile_host
{

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
///   `ErrorKind::LimitExceeded` past `max_emit_depth`;
///   `ErrorKind::ResultUnreadable` when the heap ran out of room.
/// - panics: none.
[[nodiscard]] Expected<RunOutcome> interpret_image(const Image& image);

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_INTERPRET_HPP
