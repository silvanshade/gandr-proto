#include "gandr/compile_host/samples.hpp"

#include "gandr/compile_host/image.hpp"

#include <cstddef>
#include <cstdint>
#include <utility>
#include <vector>

namespace gandr::compile_host {
namespace {

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
  auto
  push(Node node) -> NodeIndex
  {
    auto const index = static_cast<std::uint32_t>(image.nodes.size());
    image.nodes.push_back(std::move(node));
    return NodeIndex{ index };
  }

  /// Appends an integer literal.
  auto
  literal(std::int64_t value) -> NodeIndex
  {
    return push(Node{ .kind = NodeKind::Lit, .tag = CtorTag::Unit, .binder = 0, .literal = value, .operands = {} });
  }

  /// Appends a reference to the binder `levels_up` scopes out.
  auto
  variable(std::uint32_t levels_up) -> NodeIndex
  {
    return push(Node{ .kind = NodeKind::Var, .tag = CtorTag::Unit, .binder = levels_up, .literal = 0, .operands = {} });
  }

  /// Appends a constructor introduction.
  auto
  constructor(CtorTag tag, std::vector<NodeIndex> fields) -> NodeIndex
  {
    return push(Node{ .kind = NodeKind::Ctor, .tag = tag, .binder = 0, .literal = 0, .operands = std::move(fields) });
  }

  /// Appends a duplication.
  auto
  duplicate(NodeIndex source) -> NodeIndex
  {
    return push(Node{ .kind = NodeKind::Dup, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = { source } });
  }

  /// Appends a discard.
  auto
  discard(NodeIndex source) -> NodeIndex
  {
    return push(
      Node{ .kind = NodeKind::Drop, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = { source } }
    );
  }

  /// Appends a binder frame.
  auto
  bind(NodeIndex bound, NodeIndex body) -> NodeIndex
  {
    return push(
      Node{
        .kind = NodeKind::Bind,
        .tag = CtorTag::Unit,
        .binder = 0,
        .literal = 0,
        .operands = { bound, body }
    }
    );
  }

  /// Appends a dispatch.
  auto
  dispatch(NodeIndex scrutinee, NodeIndex left, NodeIndex right) -> NodeIndex
  {
    return push(
      Node{
        .kind = NodeKind::Case,
        .tag = CtorTag::Unit,
        .binder = 0,
        .literal = 0,
        .operands = { scrutinee, left, right }
    }
    );
  }

  /// Closes the image with the terminal cut.
  auto
  finish(NodeIndex produced) -> Image
  {
    image.root
      = push(Node{ .kind = NodeKind::Cut, .tag = CtorTag::Unit, .binder = 0, .literal = 0, .operands = { produced } });
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
  [[nodiscard]] auto
  next() noexcept -> std::uint64_t
  {
    state += 0x9E37'79B9'7F4A'7C15ULL;
    std::uint64_t mixed = state;
    mixed = (mixed ^ (mixed >> 30)) * 0xBF58'476D'1CE4'E5B9ULL;
    mixed = (mixed ^ (mixed >> 27)) * 0x94D0'49BB'1331'11EBULL;
    return mixed ^ (mixed >> 31);
  }

  /// Draws a value below `bound`.
  [[nodiscard]] auto
  below(std::uint32_t bound) noexcept -> std::uint32_t
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
[[nodiscard]] auto
generate_expression(
  Builder& builder,
  Generator& generator,
  std::uint32_t depth,
  std::uint32_t scope,
  std::size_t& budget
) -> NodeIndex;

/// Generates an expression that certainly evaluates to a sum injection.
///
/// Dispatch reads a cell tag, and the slice carries no type discipline that
/// would make dispatching on a non-injection mean anything. The generator
/// therefore only builds dispatches whose scrutinee is syntactically an
/// injection; the limitation is the slice's, not the generator's.
[[nodiscard]] auto
generate_injection(
  Builder& builder,
  Generator& generator,
  std::uint32_t depth,
  std::uint32_t scope,
  std::size_t& budget
) -> NodeIndex
{
  CtorTag const tag = (generator.below(2) == 0) ? CtorTag::Inl : CtorTag::Inr;
  NodeIndex const payload = generate_expression(builder, generator, depth, scope, budget);
  return builder.constructor(tag, { payload });
}

auto
generate_expression(
  Builder& builder,
  Generator& generator,
  std::uint32_t depth,
  std::uint32_t scope,
  std::size_t& budget
) -> NodeIndex
{
  bool const at_leaf = depth == 0 || budget < 8;
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
      NodeIndex const first = generate_expression(builder, generator, depth - 1, scope, budget);
      NodeIndex const second = generate_expression(builder, generator, depth - 1, scope, budget);
      return builder.constructor(CtorTag::Pair, { first, second });
    }
    case 4:
      return generate_injection(builder, generator, depth - 1, scope, budget);
    case 5: {
      NodeIndex const source = generate_expression(builder, generator, depth - 1, scope, budget);
      return (generator.below(2) == 0) ? builder.duplicate(source) : builder.discard(source);
    }
    case 6: {
      NodeIndex const bound = generate_expression(builder, generator, depth - 1, scope, budget);
      NodeIndex const body = generate_expression(builder, generator, depth - 1, scope + 1, budget);
      return builder.bind(bound, body);
    }
    default: {
      NodeIndex const scrutinee = generate_injection(builder, generator, depth - 1, scope, budget);
      NodeIndex const left = generate_expression(builder, generator, depth - 1, scope + 1, budget);
      NodeIndex const right = generate_expression(builder, generator, depth - 1, scope + 1, budget);
      return builder.dispatch(scrutinee, left, right);
    }
  }
}

} // namespace

auto
canonical_samples() -> std::vector<Sample>
{
  std::vector<Sample> samples;

  {
    Builder builder;
    NodeIndex const five = builder.literal(5);
    samples.push_back(Sample{ .name = "cut", .image = builder.finish(five) });
  }
  {
    Builder builder;
    NodeIndex const seven = builder.literal(7);
    NodeIndex const bound = builder.variable(0);
    NodeIndex const frame = builder.bind(seven, bound);
    samples.push_back(Sample{ .name = "bind", .image = builder.finish(frame) });
  }
  {
    Builder builder;
    NodeIndex const three = builder.literal(3);
    NodeIndex const injected = builder.constructor(CtorTag::Inl, { three });
    NodeIndex const left = builder.variable(0);
    NodeIndex const right = builder.variable(0);
    NodeIndex const dispatched = builder.dispatch(injected, left, right);
    samples.push_back(Sample{ .name = "case", .image = builder.finish(dispatched) });
  }
  {
    Builder builder;
    NodeIndex const one = builder.literal(1);
    NodeIndex const two = builder.literal(2);
    NodeIndex const paired = builder.constructor(CtorTag::Pair, { one, two });
    samples.push_back(Sample{ .name = "ctor", .image = builder.finish(paired) });
  }
  {
    Builder builder;
    NodeIndex const four = builder.literal(4);
    NodeIndex const duplicated = builder.duplicate(four);
    samples.push_back(Sample{ .name = "dup", .image = builder.finish(duplicated) });
  }
  {
    Builder builder;
    NodeIndex const nine = builder.literal(9);
    NodeIndex const discarded = builder.discard(nine);
    samples.push_back(Sample{ .name = "drop", .image = builder.finish(discarded) });
  }
  {
    Builder builder;
    NodeIndex const eight = builder.literal(8);
    NodeIndex const injected = builder.constructor(CtorTag::Inl, { eight });
    NodeIndex const scrutinee = builder.variable(0);
    NodeIndex const left = builder.variable(0);
    NodeIndex const right = builder.variable(0);
    NodeIndex const dispatched = builder.dispatch(scrutinee, left, right);
    NodeIndex const frame = builder.bind(injected, dispatched);
    samples.push_back(Sample{ .name = "compound", .image = builder.finish(frame) });
  }

  return samples;
}

auto
accounted_work_sample() -> Sample
{
  Builder builder;
  NodeIndex const four = builder.literal(4);
  NodeIndex const duplicated = builder.duplicate(four);
  NodeIndex const duplicate_binding = builder.variable(0);
  NodeIndex const discarded = builder.discard(duplicate_binding);
  NodeIndex const answer = builder.literal(0);
  NodeIndex const inner = builder.bind(discarded, answer);
  NodeIndex const outer = builder.bind(duplicated, inner);
  return Sample{ .name = "accounted-work", .image = builder.finish(outer) };
}

auto
generate_image(std::uint64_t seed) -> Image
{
  Builder builder;
  Generator generator{ .state = seed };
  std::size_t budget = generated_node_budget;
  NodeIndex const produced = generate_expression(builder, generator, max_generated_depth, 0, budget);
  return builder.finish(produced);
}

} // namespace gandr::compile_host
