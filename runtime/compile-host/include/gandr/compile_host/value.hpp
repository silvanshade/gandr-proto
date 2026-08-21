// The runtime value representation the compiled slice builds, and its
// canonical rendering.
//
// # Contract
// - requires: a heap laid out by `HeapLayout`, whose reserved prefix carries
//   the bump pointer and the work ledger.
// - ensures: the compiled path and the reference interpreter build values in
//   the same layout, so agreement is compared on rendered values rather than
//   on two different encodings.
// - provides: the canonical rendering the agreement fixture is written in.
// - fails: `render_value` reports a typed absence for an out-of-range or
//   malformed cell rather than reading past the heap.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_VALUE_HPP
#define GANDR_COMPILE_HOST_VALUE_HPP

#include "gandr/compile_host/image.hpp"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <span>
#include <string>

namespace gandr::compile_host {

/// The heap cell tag a runtime value carries in its first word.
///
/// The tags are the constructor tags plus the scalar leaf, in a numbering the
/// emitted code and the reference interpreter both hard-code, because it is
/// the one representation decision the two paths must share for their outputs
/// to be comparable at all.
// The tag is stored as a heap word and compared against one, so its base type
// is the heap's word type rather than the smallest type its values need.
// NOLINTNEXTLINE(performance-enum-size)
enum class CellTag : std::int64_t
{
  /// An integer cell: `[Int, n]`.
  Int = 0,
  /// The unit cell: `[Unit]`.
  Unit = 1,
  /// A pair cell: `[Pair, first, second]`.
  Pair = 2,
  /// A left injection cell: `[Inl, payload]`.
  Inl = 3,
  /// A right injection cell: `[Inr, payload]`.
  Inr = 4,
};

/// The fixed word offsets of the heap's reserved prefix.
///
/// The prefix is what makes duplication and discard observable from outside
/// the compiled function: the two counters are the accounted work, and a
/// canonicalization that deleted either operation would leave them at zero.
/// The fourth word is how the compiled code reports that it refused to
/// allocate, since a function returning one machine word has no other channel.
struct HeapLayout
{
  /// The word holding the bump-allocation cursor.
  static constexpr std::size_t bump_cursor = 0;
  /// The word counting executed duplications.
  static constexpr std::size_t duplication_ledger = 1;
  /// The word counting executed discards.
  static constexpr std::size_t discard_ledger = 2;
  /// The word a run sets when an allocation would not fit.
  ///
  /// Zero means every allocation the run attempted fitted. Any other value
  /// means the run stopped at an allocation it refused, and its returned
  /// word is not a value.
  static constexpr std::size_t exhaustion_flag = 3;
  /// The first word available to cell allocation.
  static constexpr std::size_t arena_base = 4;
};

/// The work a run accounted for, read back from the heap's ledger words.
struct WorkLedger
{
  /// How many duplications executed.
  std::int64_t duplications = 0;
  /// How many discards executed.
  std::int64_t discards = 0;

  friend constexpr auto
  operator==(WorkLedger const&, WorkLedger const&) -> bool = default;
};

/// Initializes a heap's reserved prefix for a fresh run.
///
/// # Contract
/// - requires: `heap.size()` is at least `HeapLayout::arena_base`.
/// - ensures: the bump cursor points at `HeapLayout::arena_base`, both ledger
///   words are zero, and the exhaustion flag is clear.
/// - provides: the precondition the emitted entry point assumes.
/// - panics: none.
void
reset_heap(std::span<std::int64_t> heap) noexcept;

/// Reads the work ledger out of a heap after a run.
///
/// # Contract
/// - requires: `heap.size()` is at least `HeapLayout::arena_base`.
/// - ensures: returns the two ledger words as executed counts.
/// - panics: none.
[[nodiscard]] auto
read_ledger(std::span<std::int64_t const> heap) noexcept -> WorkLedger;

/// Whether a run stopped at an allocation that would not fit.
///
/// # Contract
/// - requires: nothing; a heap smaller than the reserved prefix reads as
///   exhausted, because such a heap cannot hold a run at all.
/// - ensures: true exactly when the compiled or interpreted run refused an
///   allocation, so a caller never reads the returned word as a value.
/// - provides: the one channel the compiled entry point has for reporting a
///   refusal, since its signature returns a single machine word.
/// - panics: none.
[[nodiscard]] auto
heap_was_exhausted(std::span<std::int64_t const> heap) noexcept -> bool;

/// The number of arena words a run consumed, above the reserved prefix.
///
/// # Contract
/// - requires: `heap.size()` is at least `HeapLayout::arena_base`.
/// - ensures: returns the bump cursor's advance over the reserved prefix, so
///   zero means the run allocated nothing.
/// - provides: the measurement the exact-heap witnesses size their heaps by,
///   rather than trusting the static bound.
/// - panics: none.
[[nodiscard]] auto
allocated_words(std::span<std::int64_t const> heap) noexcept -> std::size_t;

/// Renders a heap value in the canonical form the agreement fixture uses.
///
/// The grammar is `(int N)`, `(unit)`, `(pair V V)`, `(inl V)`, `(inr V)` —
/// the same shape the Rust L machine's terminal value is projected into, so
/// the two sides are compared as text rather than through a shared binary
/// encoding neither owns.
///
/// # Contract
/// - requires: `root` is a word offset into `heap`.
/// - ensures: a returned string is a closed s-expression in that grammar.
/// - provides: the one comparison surface the differential uses.
/// - fails: returns `std::nullopt` when a cell is out of range, carries an
///   unknown tag, or nests deeper than `max_render_depth`.
/// - panics: none.
[[nodiscard]] auto
render_value(std::span<std::int64_t const> heap, std::int64_t root) -> std::optional<std::string>;

/// The deepest value nesting the renderer walks before reporting failure.
///
/// The bound is what keeps rendering total on a heap the renderer did not
/// build; a cyclic or corrupted cell graph is refused rather than followed.
inline constexpr std::size_t max_render_depth = 256;

/// The slack `heap_words_for` adds above an image's own allocation bound.
///
/// The bound is exact, so the slack is not needed for correctness; it exists so
/// that a future node form allocating one more cell than its declaration says
/// fails a test rather than corrupting a neighbour.
inline constexpr std::size_t heap_headroom_words = 16;

/// The heap word count a run of `image` needs, with the reserved prefix.
///
/// Every node of a well-formed image is reached exactly once and evaluates at
/// most once, so the sum over its nodes bounds a run's allocation. A dispatch
/// runs one arm, so the sum over-estimates; the compiled code checks its own
/// allocations regardless, because this bound is the host's arithmetic rather
/// than anything the emitted program knows.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: a run of `image` allocates no more than the returned count.
/// - provides: the one sizing both the compiled path and the reference
///   interpreter use by default, so a heap difference can never be mistaken
///   for a disagreement between them.
/// - panics: none.
[[nodiscard]] auto
heap_words_for(Image const& image) noexcept -> std::size_t;

/// The number of heap words a node of the given kind allocates when it runs.
///
/// # Contract
/// - ensures: total; the sum over an image's nodes bounds a run's allocation,
///   which is how the host sizes a heap for a generated program.
/// - panics: none.
[[nodiscard]] auto
node_allocation_words(NodeKind kind, CtorTag tag) noexcept -> std::size_t;

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_VALUE_HPP
