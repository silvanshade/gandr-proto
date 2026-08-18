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

#include <cstddef>
#include <cstdint>
#include <optional>
#include <span>
#include <string>

#include "gandr/compile_host/image.hpp"

namespace gandr::compile_host
{

/// The heap cell tag a runtime value carries in its first word.
///
/// The tags are the constructor tags plus the scalar leaf, in a numbering the
/// emitted code and the reference interpreter both hard-code, because it is
/// the one representation decision the two paths must share for their outputs
/// to be comparable at all.
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
struct HeapLayout
{
    /// The word holding the bump-allocation cursor.
    static constexpr std::size_t bump_cursor = 0;
    /// The word counting executed duplications.
    static constexpr std::size_t duplication_ledger = 1;
    /// The word counting executed discards.
    static constexpr std::size_t discard_ledger = 2;
    /// The first word available to cell allocation.
    static constexpr std::size_t arena_base = 3;
};

/// The work a run accounted for, read back from the heap's ledger words.
struct WorkLedger
{
    /// How many duplications executed.
    std::int64_t duplications = 0;
    /// How many discards executed.
    std::int64_t discards = 0;

    friend constexpr bool operator==(const WorkLedger&, const WorkLedger&) = default;
};

/// Initializes a heap's reserved prefix for a fresh run.
///
/// # Contract
/// - requires: `heap.size()` is at least `HeapLayout::arena_base`.
/// - ensures: the bump cursor points at `HeapLayout::arena_base` and both
///   ledger words are zero.
/// - provides: the precondition the emitted entry point assumes.
/// - panics: none.
void reset_heap(std::span<std::int64_t> heap) noexcept;

/// Reads the work ledger out of a heap after a run.
///
/// # Contract
/// - requires: `heap.size()` is at least `HeapLayout::arena_base`.
/// - ensures: returns the two ledger words as executed counts.
/// - panics: none.
[[nodiscard]] WorkLedger read_ledger(std::span<const std::int64_t> heap) noexcept;

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
[[nodiscard]] std::optional<std::string> render_value(
    std::span<const std::int64_t> heap,
    std::int64_t root);

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
/// runs one arm, so the sum over-estimates rather than needing a check — which
/// is what lets the compiled code carry no bounds check of its own.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: a run of `image` allocates no more than the returned count.
/// - provides: the one sizing both the compiled path and the reference
///   interpreter use, so a heap difference can never be mistaken for a
///   disagreement between them.
/// - panics: none.
[[nodiscard]] std::size_t heap_words_for(const Image& image) noexcept;

/// The number of heap words a node of the given kind allocates when it runs.
///
/// # Contract
/// - ensures: total; the sum over an image's nodes bounds a run's allocation,
///   which is how the host sizes a heap for a generated program.
/// - panics: none.
[[nodiscard]] std::size_t node_allocation_words(NodeKind kind, CtorTag tag) noexcept;

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_VALUE_HPP
