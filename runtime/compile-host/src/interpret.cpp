#include "gandr/compile_host/interpret.hpp"

#include <cstddef>
#include <string>
#include <vector>

#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/value.hpp"

namespace gandr::compile_host
{
namespace
{

/// The interpreter's heap: the same layout the compiled path allocates in, so
/// the two paths' answers are comparable without a translation nobody owns.
struct Heap
{
    /// The words.
    std::vector<std::int64_t> words;

    /// Allocates `count` words and returns the base offset.
    [[nodiscard]] std::optional<std::int64_t> allocate(std::size_t count) noexcept
    {
        const std::int64_t cursor = words[HeapLayout::bump_cursor];
        if (cursor < 0) {
            return std::nullopt;
        }
        const auto base = static_cast<std::size_t>(cursor);
        if (base > words.size() || count > words.size() - base) {
            return std::nullopt;
        }
        words[HeapLayout::bump_cursor] = cursor + static_cast<std::int64_t>(count);
        return cursor;
    }

    /// Writes one word at an offset the caller allocated.
    void store(std::int64_t offset, std::int64_t value) noexcept
    {
        const auto index = static_cast<std::size_t>(offset);
        if (index < words.size()) {
            words[index] = value;
        }
    }

    /// Reads one word.
    [[nodiscard]] std::optional<std::int64_t> load(std::int64_t offset) const noexcept
    {
        if (offset < 0) {
            return std::nullopt;
        }
        const auto index = static_cast<std::size_t>(offset);
        if (index >= words.size()) {
            return std::nullopt;
        }
        return words[index];
    }
};

/// The environment of bindings in scope, innermost last.
using Environment = std::vector<std::int64_t>;

/// The cell tag a constructor tag builds.
[[nodiscard]] CellTag cell_tag_of(CtorTag tag) noexcept
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
[[nodiscard]] Expected<std::int64_t> evaluate(
    const Image& image,
    NodeIndex node_index,
    Environment& environment,
    Heap& heap,
    std::size_t depth)
{
    if (depth > max_emit_depth) {
        return host_error(ErrorKind::LimitExceeded, "image nests deeper than the host admits");
    }
    const Node& node = image.nodes[node_index.value];

    const auto allocate_cell = [&heap](std::size_t words) -> Expected<std::int64_t> {
        const std::optional<std::int64_t> base = heap.allocate(words);
        if (!base.has_value()) {
            heap.words[HeapLayout::exhaustion_flag] = 1;
            return host_error(
                ErrorKind::LimitExceeded,
                "reference run refused an allocation that would not fit its heap");
        }
        return *base;
    };

    switch (node.kind) {
    case NodeKind::Lit: {
        const Expected<std::int64_t> base = allocate_cell(2);
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
        return environment[environment.size() - 1 - node.binder];
    }
    case NodeKind::Ctor: {
        std::vector<std::int64_t> fields;
        fields.reserve(node.operands.size());
        for (const NodeIndex operand : node.operands) {
            const Expected<std::int64_t> field = evaluate(image, operand, environment, heap, depth + 1);
            if (!field.has_value()) {
                return field;
            }
            fields.push_back(*field);
        }
        const Expected<std::int64_t> base = allocate_cell(1 + fields.size());
        if (!base.has_value()) {
            return base;
        }
        heap.store(*base, static_cast<std::int64_t>(cell_tag_of(node.tag)));
        for (std::size_t field_index = 0; field_index < fields.size(); ++field_index) {
            heap.store(*base + 1 + static_cast<std::int64_t>(field_index), fields[field_index]);
        }
        return *base;
    }
    case NodeKind::Dup: {
        const Expected<std::int64_t> source =
            evaluate(image, node.operands[0], environment, heap, depth + 1);
        if (!source.has_value()) {
            return source;
        }
        const Expected<std::int64_t> base = allocate_cell(3);
        if (!base.has_value()) {
            return base;
        }
        heap.store(*base, static_cast<std::int64_t>(CellTag::Pair));
        heap.store(*base + 1, *source);
        heap.store(*base + 2, *source);
        heap.words[HeapLayout::duplication_ledger] += 1;
        return *base;
    }
    case NodeKind::Drop: {
        const Expected<std::int64_t> source =
            evaluate(image, node.operands[0], environment, heap, depth + 1);
        if (!source.has_value()) {
            return source;
        }
        const Expected<std::int64_t> base = allocate_cell(1);
        if (!base.has_value()) {
            return base;
        }
        heap.store(*base, static_cast<std::int64_t>(CellTag::Unit));
        heap.words[HeapLayout::discard_ledger] += 1;
        return *base;
    }
    case NodeKind::Bind: {
        const Expected<std::int64_t> bound =
            evaluate(image, node.operands[0], environment, heap, depth + 1);
        if (!bound.has_value()) {
            return bound;
        }
        environment.push_back(*bound);
        const Expected<std::int64_t> body =
            evaluate(image, node.operands[1], environment, heap, depth + 1);
        environment.pop_back();
        return body;
    }
    case NodeKind::Case: {
        const Expected<std::int64_t> scrutinee =
            evaluate(image, node.operands[0], environment, heap, depth + 1);
        if (!scrutinee.has_value()) {
            return scrutinee;
        }
        const std::optional<std::int64_t> tag = heap.load(*scrutinee);
        const std::optional<std::int64_t> payload = heap.load(*scrutinee + 1);
        if (!tag.has_value() || !payload.has_value()) {
            return host_error(ErrorKind::ResultUnreadable, "dispatch read past the heap");
        }
        const bool takes_left = static_cast<CellTag>(*tag) == CellTag::Inl;
        environment.push_back(*payload);
        const Expected<std::int64_t> arm =
            evaluate(image, node.operands[takes_left ? 1 : 2], environment, heap, depth + 1);
        environment.pop_back();
        return arm;
    }
    case NodeKind::Cut:
        return evaluate(image, node.operands[0], environment, heap, depth + 1);
    }
    return host_error(ErrorKind::MalformedImage, "image holds a node of no declared kind");
}

} // namespace

Expected<RunOutcome> interpret_image(const Image& image)
{
    return interpret_image_with_heap(image, heap_words_for(image));
}

Expected<RunOutcome> interpret_image_with_heap(const Image& image, std::size_t heap_words)
{
    if (!image_is_wellformed(image)) {
        return host_error(ErrorKind::MalformedImage, "image failed the structural check");
    }
    if (heap_words < HeapLayout::arena_base) {
        return host_error(
            ErrorKind::LimitExceeded,
            "heap is too small to hold the reserved prefix a run writes");
    }

    Heap heap;
    heap.words.assign(heap_words, 0);
    reset_heap(heap.words);

    Environment environment;
    const Expected<std::int64_t> root = evaluate(image, image.root, environment, heap, 0);
    if (!root.has_value()) {
        return std::unexpected(root.error());
    }

    const std::optional<std::string> rendered = render_value(heap.words, *root);
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
