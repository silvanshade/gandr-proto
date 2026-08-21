#include "gandr/compile_host/value.hpp"

#include "gandr/compile_host/access.hpp"
#include "gandr/compile_host/image.hpp"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <utility>
#include <vector>

namespace gandr::compile_host {
namespace {

/// One step of the renderer's explicit traversal: either a cell still to be
/// read, or literal text already decided.
///
/// The renderer walks a heap it did not necessarily build, so it uses an
/// explicit stack rather than recursion: a corrupted cell graph then costs a
/// depth check rather than the process.
struct RenderStep
{
  /// Whether this step reads a cell.
  bool is_cell = false;
  /// The cell's word offset, when `is_cell`.
  std::int64_t cell = 0;
  /// The nesting depth this step sits at.
  std::size_t depth = 0;
  /// The text to append, when not `is_cell`.
  std::string text;
};

/// Reads one heap word, refusing an out-of-range offset.
[[nodiscard]] auto
read_word(std::span<std::int64_t const> heap, std::int64_t offset) noexcept -> std::optional<std::int64_t>
{
  if (offset < 0) {
    return std::nullopt;
  }
  auto const index = static_cast<std::size_t>(offset);
  if (index >= heap.size()) {
    return std::nullopt;
  }
  return proved_at(heap, index);
}

} // namespace

void
reset_heap(std::span<std::int64_t> heap) noexcept
{
  if (heap.size() < HeapLayout::arena_base) {
    return;
  }
  proved_at(heap, HeapLayout::bump_cursor) = static_cast<std::int64_t>(HeapLayout::arena_base);
  proved_at(heap, HeapLayout::duplication_ledger) = 0;
  proved_at(heap, HeapLayout::discard_ledger) = 0;
  proved_at(heap, HeapLayout::exhaustion_flag) = 0;
}

auto
read_ledger(std::span<std::int64_t const> heap) noexcept -> WorkLedger
{
  if (heap.size() < HeapLayout::arena_base) {
    return WorkLedger{};
  }
  return WorkLedger{
    .duplications = proved_at(heap, HeapLayout::duplication_ledger),
    .discards = proved_at(heap, HeapLayout::discard_ledger),
  };
}

auto
heap_was_exhausted(std::span<std::int64_t const> heap) noexcept -> bool
{
  if (heap.size() < HeapLayout::arena_base) {
    return true;
  }
  return proved_at(heap, HeapLayout::exhaustion_flag) != 0;
}

auto
allocated_words(std::span<std::int64_t const> heap) noexcept -> std::size_t
{
  if (heap.size() < HeapLayout::arena_base) {
    return 0;
  }
  std::int64_t const cursor = proved_at(heap, HeapLayout::bump_cursor);
  if (cursor <= static_cast<std::int64_t>(HeapLayout::arena_base)) {
    return 0;
  }
  return static_cast<std::size_t>(cursor) - HeapLayout::arena_base;
}

auto
render_value(std::span<std::int64_t const> heap, std::int64_t root) -> std::optional<std::string>
{
  std::string rendered;
  std::vector<RenderStep> pending;
  pending.push_back(RenderStep{ .is_cell = true, .cell = root, .depth = 0, .text = {} });

  while (!pending.empty()) {
    RenderStep const step = std::move(pending.back());
    pending.pop_back();

    if (!step.is_cell) {
      rendered += step.text;
      continue;
    }
    if (step.depth > max_render_depth) {
      return std::nullopt;
    }

    std::optional<std::int64_t> const tag_word = read_word(heap, step.cell);
    if (!tag_word.has_value()) {
      return std::nullopt;
    }

    auto const push_text = [&pending](std::string text) -> void {
      pending.push_back(RenderStep{ .is_cell = false, .cell = 0, .depth = 0, .text = std::move(text) });
    };
    auto const push_cell = [&pending, &step](std::int64_t offset) -> void {
      pending.push_back(RenderStep{ .is_cell = true, .cell = offset, .depth = step.depth + 1, .text = {} });
    };

    if (*tag_word < 0 || *tag_word > static_cast<std::int64_t>(CellTag::Inr)) {
      return std::nullopt;
    }
    switch (static_cast<CellTag>(*tag_word)) {
      case CellTag::Int: {
        std::optional<std::int64_t> const payload = read_word(heap, step.cell + 1);
        if (!payload.has_value()) {
          return std::nullopt;
        }
        rendered += "(int ";
        rendered += std::to_string(*payload);
        rendered += ")";
        break;
      }
      case CellTag::Unit:
        rendered += "(unit)";
        break;
      case CellTag::Pair: {
        std::optional<std::int64_t> const first = read_word(heap, step.cell + 1);
        std::optional<std::int64_t> const second = read_word(heap, step.cell + 2);
        if (!first.has_value() || !second.has_value()) {
          return std::nullopt;
        }
        rendered += "(pair ";
        push_text(")");
        push_cell(*second);
        push_text(" ");
        push_cell(*first);
        break;
      }
      case CellTag::Inl:
      case CellTag::Inr: {
        std::optional<std::int64_t> const payload = read_word(heap, step.cell + 1);
        if (!payload.has_value()) {
          return std::nullopt;
        }
        rendered += (static_cast<CellTag>(*tag_word) == CellTag::Inl) ? "(inl " : "(inr ";
        push_text(")");
        push_cell(*payload);
        break;
      }
    }
  }

  return rendered;
}

auto
heap_words_for(Image const& image) noexcept -> std::size_t
{
  std::size_t words = HeapLayout::arena_base;
  for (Node const& node : image.nodes) {
    words += node_allocation_words(node.kind, node.tag);
  }
  return words + heap_headroom_words;
}

auto
node_allocation_words(NodeKind kind, CtorTag tag) noexcept -> std::size_t
{
  switch (kind) {
    case NodeKind::Lit:
      return 2;
    case NodeKind::Ctor:
      return std::size_t{ 1 } + ctor_arity(tag);
    case NodeKind::Dup:
      return 3;
    case NodeKind::Drop:
      return 1;
    case NodeKind::Var:
    case NodeKind::Bind:
    case NodeKind::Case:
    case NodeKind::Cut:
      return 0;
  }
  return 0;
}

} // namespace gandr::compile_host
