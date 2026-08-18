#include "gandr/compile_host/samples.hpp"

#include <cstddef>
#include <utility>

namespace gandr::compile_host
{
namespace
{

/// A builder that appends nodes in dependency order.
///
/// An operand is always appended before its user, which is the invariant
/// `image_is_wellformed` checks; building through this type is what makes that
/// hold by construction rather than by inspection.
struct Builder
{
    /// The image under construction.
    Image image;

    /// Appends a node and returns its index.
    NodeIndex push(Node node)
    {
        const auto index = static_cast<std::uint32_t>(image.nodes.size());
        image.nodes.push_back(std::move(node));
        return NodeIndex{index};
    }

    /// Appends an integer literal.
    NodeIndex literal(std::int64_t value)
    {
        return push(Node{.kind = NodeKind::Lit, .tag = CtorTag::Unit, .binder = 0, .literal = value, .operands = {}});
    }

    /// Appends a reference to the binder `levels_up` scopes out.
    NodeIndex variable(std::uint32_t levels_up)
    {
        return push(
            Node{.kind = NodeKind::Var, .tag = CtorTag::Unit, .binder = levels_up, .literal = 0, .operands = {}});
    }

    /// Appends a constructor introduction.
    NodeIndex constructor(CtorTag tag, std::vector<NodeIndex> fields)
    {
        return push(
            Node{.kind = NodeKind::Ctor, .tag = tag, .binder = 0, .literal = 0, .operands = std::move(fields)});
    }

    /// Appends a duplication.
    NodeIndex duplicate(NodeIndex source)
    {
        return push(Node{.kind = NodeKind::Dup, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = {source}});
    }

    /// Appends a discard.
    NodeIndex discard(NodeIndex source)
    {
        return push(Node{.kind = NodeKind::Drop, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = {source}});
    }

    /// Appends a binder frame.
    NodeIndex bind(NodeIndex bound, NodeIndex body)
    {
        return push(
            Node{.kind = NodeKind::Bind, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = {bound, body}});
    }

    /// Appends a dispatch.
    NodeIndex dispatch(NodeIndex scrutinee, NodeIndex left, NodeIndex right)
    {
        return push(Node{
            .kind = NodeKind::Case,
            .tag = CtorTag::Unit,
            .binder = 0,
            .literal = 0,
            .operands = {scrutinee, left, right}});
    }

    /// Closes the image with the terminal cut.
    Image finish(NodeIndex produced)
    {
        image.root =
            push(Node{.kind = NodeKind::Cut, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = {produced}});
        return std::move(image);
    }
};

/// A deterministic generator state.
///
/// A counter-based mixing function rather than a stateful engine: the same
/// seed yields the same program on every platform, which is what makes a
/// property failure reproducible from its seed alone.
struct Generator
{
    /// The evolving state.
    std::uint64_t state = 0;

    /// Draws the next raw word.
    [[nodiscard]] std::uint64_t next() noexcept
    {
        state += 0x9E37'79B9'7F4A'7C15ULL;
        std::uint64_t mixed = state;
        mixed = (mixed ^ (mixed >> 30)) * 0xBF58'476D'1CE4'E5B9ULL;
        mixed = (mixed ^ (mixed >> 27)) * 0x94D0'49BB'1331'11EBULL;
        return mixed ^ (mixed >> 31);
    }

    /// Draws a value below `bound`.
    [[nodiscard]] std::uint32_t below(std::uint32_t bound) noexcept
    {
        if (bound == 0) {
            return 0;
        }
        return static_cast<std::uint32_t>(next() % bound);
    }
};

/// The node budget one generated program may spend.
constexpr std::size_t generated_node_budget = 96;

/// Generates one expression under a depth and scope, spending from `budget`.
///
/// The generator is total and its recursion is bounded twice over: by
/// `depth`, which never exceeds `max_generated_depth`, and by `budget`, which
/// strictly decreases at every constructor.
[[nodiscard]] NodeIndex generate_expression(
    Builder& builder,
    Generator& generator,
    std::uint32_t depth,
    std::uint32_t scope,
    std::size_t& budget);

/// Generates an expression that certainly evaluates to a sum injection.
///
/// Dispatch reads a cell tag, and the slice carries no type discipline that
/// would make dispatching on a non-injection mean anything. The generator
/// therefore only builds dispatches whose scrutinee is syntactically an
/// injection; the limitation is the slice's, not the generator's.
[[nodiscard]] NodeIndex generate_injection(
    Builder& builder,
    Generator& generator,
    std::uint32_t depth,
    std::uint32_t scope,
    std::size_t& budget)
{
    const CtorTag tag = (generator.below(2) == 0) ? CtorTag::Inl : CtorTag::Inr;
    const NodeIndex payload = generate_expression(builder, generator, depth, scope, budget);
    return builder.constructor(tag, {payload});
}

NodeIndex generate_expression(
    Builder& builder,
    Generator& generator,
    std::uint32_t depth,
    std::uint32_t scope,
    std::size_t& budget)
{
    const bool at_leaf = depth == 0 || budget < 8;
    if (budget > 0) {
        budget -= 1;
    }

    if (at_leaf) {
        if (scope > 0 && generator.below(2) == 0) {
            return builder.variable(generator.below(scope));
        }
        return builder.literal(static_cast<std::int64_t>(generator.below(64)));
    }

    switch (generator.below(8)) {
    case 0:
        return builder.literal(static_cast<std::int64_t>(generator.below(64)));
    case 1:
        if (scope > 0) {
            return builder.variable(generator.below(scope));
        }
        return builder.literal(static_cast<std::int64_t>(generator.below(64)));
    case 2:
        return builder.constructor(CtorTag::Unit, {});
    case 3: {
        const NodeIndex first = generate_expression(builder, generator, depth - 1, scope, budget);
        const NodeIndex second = generate_expression(builder, generator, depth - 1, scope, budget);
        return builder.constructor(CtorTag::Pair, {first, second});
    }
    case 4:
        return generate_injection(builder, generator, depth - 1, scope, budget);
    case 5: {
        const NodeIndex source = generate_expression(builder, generator, depth - 1, scope, budget);
        return (generator.below(2) == 0) ? builder.duplicate(source) : builder.discard(source);
    }
    case 6: {
        const NodeIndex bound = generate_expression(builder, generator, depth - 1, scope, budget);
        const NodeIndex body = generate_expression(builder, generator, depth - 1, scope + 1, budget);
        return builder.bind(bound, body);
    }
    default: {
        const NodeIndex scrutinee = generate_injection(builder, generator, depth - 1, scope, budget);
        const NodeIndex left = generate_expression(builder, generator, depth - 1, scope + 1, budget);
        const NodeIndex right = generate_expression(builder, generator, depth - 1, scope + 1, budget);
        return builder.dispatch(scrutinee, left, right);
    }
    }
}

} // namespace

std::vector<Sample> canonical_samples()
{
    std::vector<Sample> samples;

    {
        Builder builder;
        const NodeIndex five = builder.literal(5);
        samples.push_back(Sample{.name = "cut", .image = builder.finish(five)});
    }
    {
        Builder builder;
        const NodeIndex seven = builder.literal(7);
        const NodeIndex bound = builder.variable(0);
        const NodeIndex frame = builder.bind(seven, bound);
        samples.push_back(Sample{.name = "bind", .image = builder.finish(frame)});
    }
    {
        Builder builder;
        const NodeIndex three = builder.literal(3);
        const NodeIndex injected = builder.constructor(CtorTag::Inl, {three});
        const NodeIndex left = builder.variable(0);
        const NodeIndex right = builder.variable(0);
        const NodeIndex dispatched = builder.dispatch(injected, left, right);
        samples.push_back(Sample{.name = "case", .image = builder.finish(dispatched)});
    }
    {
        Builder builder;
        const NodeIndex one = builder.literal(1);
        const NodeIndex two = builder.literal(2);
        const NodeIndex paired = builder.constructor(CtorTag::Pair, {one, two});
        samples.push_back(Sample{.name = "ctor", .image = builder.finish(paired)});
    }
    {
        Builder builder;
        const NodeIndex four = builder.literal(4);
        const NodeIndex duplicated = builder.duplicate(four);
        samples.push_back(Sample{.name = "dup", .image = builder.finish(duplicated)});
    }
    {
        Builder builder;
        const NodeIndex nine = builder.literal(9);
        const NodeIndex discarded = builder.discard(nine);
        samples.push_back(Sample{.name = "drop", .image = builder.finish(discarded)});
    }
    {
        Builder builder;
        const NodeIndex eight = builder.literal(8);
        const NodeIndex injected = builder.constructor(CtorTag::Inl, {eight});
        const NodeIndex scrutinee = builder.variable(0);
        const NodeIndex left = builder.variable(0);
        const NodeIndex right = builder.variable(0);
        const NodeIndex dispatched = builder.dispatch(scrutinee, left, right);
        const NodeIndex frame = builder.bind(injected, dispatched);
        samples.push_back(Sample{.name = "compound", .image = builder.finish(frame)});
    }

    return samples;
}

Sample accounted_work_sample()
{
    Builder builder;
    const NodeIndex four = builder.literal(4);
    const NodeIndex duplicated = builder.duplicate(four);
    const NodeIndex duplicate_binding = builder.variable(0);
    const NodeIndex discarded = builder.discard(duplicate_binding);
    const NodeIndex answer = builder.literal(0);
    const NodeIndex inner = builder.bind(discarded, answer);
    const NodeIndex outer = builder.bind(duplicated, inner);
    return Sample{.name = "accounted-work", .image = builder.finish(outer)};
}

Image generate_image(std::uint64_t seed)
{
    Builder builder;
    Generator generator{.state = seed};
    std::size_t budget = generated_node_budget;
    const NodeIndex produced =
        generate_expression(builder, generator, max_generated_depth, 0, budget);
    return builder.finish(produced);
}

} // namespace gandr::compile_host
