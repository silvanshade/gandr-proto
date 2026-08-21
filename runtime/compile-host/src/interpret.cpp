#include "gandr/compile_host/interpret.hpp"

#include "gandr/compile_host/access.hpp"
#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/jit.hpp"
#include "gandr/compile_host/status.hpp"
#include "gandr/compile_host/value.hpp"

#include <cstddef>
#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <vector>

namespace gandr::compile_host {
namespace {

/// The interpreter's heap: the same layout the compiled path allocates in, so
/// the two paths' answers are comparable without a translation nobody owns.
struct Heap
{
  /// The words.
  std::vector<std::int64_t> words;

  /// Allocates `count` words and returns the base offset.
  [[nodiscard]] auto
  allocate(std::size_t count) noexcept -> std::optional<std::int64_t>
  {
    std::int64_t const cursor = proved_at(words, HeapLayout::bump_cursor);
    if (cursor < 0) {
      return std::nullopt;
    }
    auto const base = static_cast<std::size_t>(cursor);
    if (base > words.size() || count > words.size() - base) {
      return std::nullopt;
    }
    proved_at(words, HeapLayout::bump_cursor) = cursor + static_cast<std::int64_t>(count);
    return cursor;
  }

  /// Writes one word at an offset the caller allocated.
  void
  store(std::int64_t offset, std::int64_t value) noexcept
  {
    auto const index = static_cast<std::size_t>(offset);
    if (index < words.size()) {
      proved_at(words, index) = value;
    }
  }

  /// Reads one word.
  [[nodiscard]] auto
  load(std::int64_t offset) const noexcept -> std::optional<std::int64_t>
  {
    if (offset < 0) {
      return std::nullopt;
    }
    auto const index = static_cast<std::size_t>(offset);
    if (index >= words.size()) {
      return std::nullopt;
    }
    return proved_at(words, index);
  }
};

/// The environment of bindings in scope, innermost last.
using Environment = std::vector<std::int64_t>;

/// The cell tag a constructor tag builds.
[[nodiscard]] auto
cell_tag_of(CtorTag tag) noexcept -> CellTag
{
  switch (tag) {
    case CtorTag::Unit:
      return CellTag::Unit;
    case CtorTag::Pair:
      return CellTag::Pair;
    case CtorTag::Inl:
      return CellTag::Inl;
    case CtorTag::Inr:
      return CellTag::Inr;
  }
  return CellTag::Unit;
}

/// Evaluates one node, recursively over its operands.
///
/// The recursion tracks the image's consumer nesting and is bounded by
/// `max_emit_depth`, which the caller enforces before the walk begins and this
/// function rechecks on every descent — the interpreter runs on generated and
/// fuzzed images, so the bound is load-bearing rather than defensive.
[[nodiscard]] auto
evaluate(Image const& image, NodeIndex node_index, Environment& environment, Heap& heap, std::size_t depth)
  -> Expected<std::int64_t>
{
  if (depth > max_emit_depth) {
    return host_error(ErrorKind::LimitExceeded, "image nests deeper than the host admits");
  }
  Node const& node = proved_at(image.nodes, node_index.value);

  auto const allocate_cell = [&heap](std::size_t words) -> Expected<std::int64_t> {
    std::optional<std::int64_t> const base = heap.allocate(words);
    if (!base.has_value()) {
      proved_at(heap.words, HeapLayout::exhaustion_flag) = 1;
      return host_error(ErrorKind::LimitExceeded, "reference run refused an allocation that would not fit its heap");
    }
    return *base;
  };

  switch (node.kind) {
    case NodeKind::Lit: {
      Expected<std::int64_t> const base = allocate_cell(2);
      if (!base.has_value()) {
        return base;
      }
      heap.store(*base, static_cast<std::int64_t>(CellTag::Int));
      heap.store(*base + 1, node.literal);
      return *base;
    }
    case NodeKind::Var: {
      if (node.binder >= environment.size()) {
        return host_error(ErrorKind::MalformedImage, "variable names no binder in scope");
      }
      return proved_at(environment, environment.size() - 1 - node.binder);
    }
    case NodeKind::Ctor: {
      std::vector<std::int64_t> fields;
      fields.reserve(node.operands.size());
      for (NodeIndex const operand : node.operands) {
        Expected<std::int64_t> const field = evaluate(image, operand, environment, heap, depth + 1);
        if (!field.has_value()) {
          return field;
        }
        fields.push_back(*field);
      }
      Expected<std::int64_t> const base = allocate_cell(1 + fields.size());
      if (!base.has_value()) {
        return base;
      }
      heap.store(*base, static_cast<std::int64_t>(cell_tag_of(node.tag)));
      for (std::size_t field_index = 0; field_index < fields.size(); ++field_index) {
        heap.store(*base + 1 + static_cast<std::int64_t>(field_index), proved_at(fields, field_index));
      }
      return *base;
    }
    case NodeKind::Dup: {
      Expected<std::int64_t> const source = evaluate(image, proved_at(node.operands, 0), environment, heap, depth + 1);
      if (!source.has_value()) {
        return source;
      }
      Expected<std::int64_t> const base = allocate_cell(3);
      if (!base.has_value()) {
        return base;
      }
      heap.store(*base, static_cast<std::int64_t>(CellTag::Pair));
      heap.store(*base + 1, *source);
      heap.store(*base + 2, *source);
      proved_at(heap.words, HeapLayout::duplication_ledger) += 1;
      return *base;
    }
    case NodeKind::Drop: {
      Expected<std::int64_t> const source = evaluate(image, proved_at(node.operands, 0), environment, heap, depth + 1);
      if (!source.has_value()) {
        return source;
      }
      Expected<std::int64_t> const base = allocate_cell(1);
      if (!base.has_value()) {
        return base;
      }
      heap.store(*base, static_cast<std::int64_t>(CellTag::Unit));
      proved_at(heap.words, HeapLayout::discard_ledger) += 1;
      return *base;
    }
    case NodeKind::Bind: {
      Expected<std::int64_t> const bound = evaluate(image, proved_at(node.operands, 0), environment, heap, depth + 1);
      if (!bound.has_value()) {
        return bound;
      }
      environment.push_back(*bound);
      Expected<std::int64_t> const body = evaluate(image, proved_at(node.operands, 1), environment, heap, depth + 1);
      environment.pop_back();
      return body;
    }
    case NodeKind::Case: {
      Expected<std::int64_t> const scrutinee
        = evaluate(image, proved_at(node.operands, 0), environment, heap, depth + 1);
      if (!scrutinee.has_value()) {
        return scrutinee;
      }
      std::optional<std::int64_t> const tag = heap.load(*scrutinee);
      std::optional<std::int64_t> const payload = heap.load(*scrutinee + 1);
      if (!tag.has_value() || !payload.has_value()) {
        return host_error(ErrorKind::ResultUnreadable, "dispatch read past the heap");
      }
      bool const takes_left = static_cast<CellTag>(*tag) == CellTag::Inl;
      environment.push_back(*payload);
      Expected<std::int64_t> const arm
        = evaluate(image, proved_at(node.operands, takes_left ? 1 : 2), environment, heap, depth + 1);
      environment.pop_back();
      return arm;
    }
    case NodeKind::Cut:
      return evaluate(image, proved_at(node.operands, 0), environment, heap, depth + 1);
  }
  return host_error(ErrorKind::MalformedImage, "image holds a node of no declared kind");
}

} // namespace

auto
interpret_image(Image const& image) -> Expected<RunOutcome>
{
  return interpret_image_with_heap(image, heap_words_for(image));
}

auto
interpret_image_with_heap(Image const& image, std::size_t heap_words) -> Expected<RunOutcome>
{
  if (!image_is_wellformed(image)) {
    return host_error(ErrorKind::MalformedImage, "image failed the structural check");
  }
  if (heap_words < HeapLayout::arena_base) {
    return host_error(ErrorKind::LimitExceeded, "heap is too small to hold the reserved prefix a run writes");
  }

  Heap heap;
  heap.words.assign(heap_words, 0);
  reset_heap(heap.words);

  Environment environment;
  Expected<std::int64_t> const root = evaluate(image, image.root, environment, heap, 0);
  if (!root.has_value()) {
    return std::unexpected(root.error());
  }

  std::optional<std::string> const rendered = render_value(heap.words, *root);
  if (!rendered.has_value()) {
    return host_error(ErrorKind::ResultUnreadable, "reference run produced an unreadable value");
  }
  return RunOutcome{
    .value = *rendered,
    .ledger = read_ledger(heap.words),
    .allocated = allocated_words(heap.words),
  };
}

} // namespace gandr::compile_host
