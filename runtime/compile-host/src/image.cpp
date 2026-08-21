#include "gandr/compile_host/image.hpp"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

namespace gandr::compile_host {
namespace {

/// The operand count a node kind requires, ignoring the constructor case whose
/// count its tag declares.
///
/// # Contract
/// - ensures: total; returns the fixed count for every kind but `Ctor`, whose
///   count is `ctor_arity` of its tag and is reported as absent here.
/// - panics: none.
[[nodiscard]] auto
fixed_operand_count(NodeKind kind) noexcept -> std::optional<std::size_t>
{
  switch (kind) {
    case NodeKind::Lit:
    case NodeKind::Var:
      return std::size_t{ 0 };
    case NodeKind::Dup:
    case NodeKind::Drop:
    case NodeKind::Cut:
      return std::size_t{ 1 };
    case NodeKind::Bind:
      return std::size_t{ 2 };
    case NodeKind::Case:
      return std::size_t{ 3 };
    case NodeKind::Ctor:
      return std::nullopt;
  }
  return std::nullopt;
}

} // namespace

auto
operand_binder_offset(NodeKind kind, std::size_t slot) noexcept -> std::uint32_t
{
  switch (kind) {
    case NodeKind::Bind:
      return slot == 1 ? 1U : 0U;
    case NodeKind::Case:
      return slot == 0 ? 0U : 1U;
    case NodeKind::Lit:
    case NodeKind::Var:
    case NodeKind::Ctor:
    case NodeKind::Dup:
    case NodeKind::Drop:
    case NodeKind::Cut:
      return 0U;
  }
  return 0U;
}

auto
image_is_wellformed(Image const& image) noexcept -> bool
{
  if (image.nodes.empty() || image.nodes.size() > max_image_nodes) {
    return false;
  }
  if (image.root.value >= image.nodes.size()) {
    return false;
  }
  // The root is the terminal cut, and a cut is a terminator: it is the one
  // command shape the positive core ends in, and it nests nowhere.
  if (image.nodes[image.root.value].kind != NodeKind::Cut) {
    return false;
  }

  // The scope depth each node is evaluated under. A node is reachable from
  // at most one site in a well-formed image, so one depth per node is
  // enough; the walk below computes it forward, because an operand always
  // precedes its user.
  std::vector<std::uint32_t> depth(image.nodes.size(), 0);
  std::vector<bool> reached(image.nodes.size(), false);
  reached[image.root.value] = true;

  for (std::size_t index = image.nodes.size(); index-- > 0;) {
    Node const& node = image.nodes[index];

    std::optional<std::size_t> const required = fixed_operand_count(node.kind);
    std::size_t const expected = required.has_value() ? *required : std::size_t{ ctor_arity(node.tag) };
    if (node.operands.size() != expected) {
      return false;
    }

    if (node.kind == NodeKind::Cut && NodeIndex{ static_cast<std::uint32_t>(index) } != image.root) {
      return false;
    }

    if (!reached[index]) {
      continue;
    }

    for (std::size_t slot = 0; slot < node.operands.size(); ++slot) {
      NodeIndex const operand = node.operands[slot];
      if (operand.value >= index) {
        return false;
      }
      if (reached[operand.value]) {
        // A shared operand would be evaluated once but named from two
        // scopes, and the emitter has no way to place it. Refusing
        // sharing here is what keeps the image a tree.
        return false;
      }
      reached[operand.value] = true;
      depth[operand.value] = depth[index] + operand_binder_offset(node.kind, slot);
    }

    if (node.kind == NodeKind::Var && node.binder >= depth[index]) {
      return false;
    }
  }

  for (std::size_t index = 0; index < image.nodes.size(); ++index) {
    if (!reached[index]) {
      return false;
    }
  }
  return true;
}

} // namespace gandr::compile_host
