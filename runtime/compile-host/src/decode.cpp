#include "gandr/compile_host/access.hpp"
#include "gandr/compile_host/image.hpp"

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <span>
#include <utility>
#include <vector>

namespace gandr::compile_host {
namespace {

/// The image wire version this host reads and writes.
///
/// An older reader meets an unknown version and refuses precisely, rather than
/// interpreting a later layout as this one.
constexpr std::uint8_t image_version = 1;

/// A bounded cursor over a byte string.
///
/// Every read is checked against the remaining length, so the decoder is total
/// on arbitrary input — which is the property the fuzz entry surface exists to
/// keep true.
struct Cursor
{
  /// The bytes still to be read.
  std::span<std::uint8_t const> rest;

  /// Reads one byte.
  [[nodiscard]] auto
  take_u8() noexcept -> std::optional<std::uint8_t>
  {
    if (rest.empty()) {
      return std::nullopt;
    }
    std::uint8_t const byte = rest.front();
    rest = rest.subspan(1);
    return byte;
  }

  /// Reads a little-endian unsigned integer of `Width` bytes into the
  /// unsigned type `T`. The reconstruction is by shift rather than by
  /// reinterpretation, so the wire form does not depend on the host's byte
  /// order.
  template<typename T, std::size_t Width>
  [[nodiscard]] auto
  take_le() noexcept -> std::optional<T>
  {
    if (rest.size() < Width) {
      return std::nullopt;
    }
    std::uint64_t accumulated = 0;
    for (std::size_t byte_index = 0; byte_index < Width; ++byte_index) {
      accumulated |= static_cast<std::uint64_t>(proved_at(rest, byte_index))
                  << (8 * static_cast<std::uint64_t>(byte_index));
    }
    rest = rest.subspan(Width);
    return static_cast<T>(accumulated);
  }
};

/// Appends a little-endian unsigned integer of `Width` bytes.
template<std::size_t Width>
void
put_le(std::vector<std::uint8_t>& bytes, std::uint64_t value)
{
  for (std::size_t byte_index = 0; byte_index < Width; ++byte_index) {
    bytes.push_back(static_cast<std::uint8_t>((value >> (8 * static_cast<std::uint64_t>(byte_index))) & 0xFFU));
  }
}

/// The largest operand count a node may declare in the wire form.
///
/// The bound is the widest node kind's count; a larger declaration is refused
/// rather than allocated.
constexpr std::uint8_t max_wire_operands = 3;

} // namespace

auto
decode_image(std::span<std::uint8_t const> bytes) -> std::optional<Image>
{
  Cursor cursor{ bytes };

  std::optional<std::uint8_t> const version = cursor.take_u8();
  if (!version.has_value() || *version != image_version) {
    return std::nullopt;
  }

  std::optional<std::uint16_t> const node_count = cursor.take_le<std::uint16_t, 2>();
  if (!node_count.has_value() || *node_count == 0 || static_cast<std::size_t>(*node_count) > max_image_nodes) {
    return std::nullopt;
  }

  Image image;
  image.nodes.reserve(*node_count);
  for (std::uint16_t node_index = 0; node_index < *node_count; ++node_index) {
    std::optional<std::uint8_t> const kind = cursor.take_u8();
    std::optional<std::uint8_t> const tag = cursor.take_u8();
    std::optional<std::uint32_t> const binder = cursor.take_le<std::uint32_t, 4>();
    std::optional<std::uint64_t> const literal = cursor.take_le<std::uint64_t, 8>();
    std::optional<std::uint8_t> const operand_count = cursor.take_u8();
    if (
      !kind.has_value()
      || !tag.has_value()
      || !binder.has_value()
      || !literal.has_value()
      || !operand_count.has_value()) {
      return std::nullopt;
    }
    if (*kind > static_cast<std::uint8_t>(NodeKind::Cut)) {
      return std::nullopt;
    }
    if (*tag > static_cast<std::uint8_t>(CtorTag::Inr)) {
      return std::nullopt;
    }
    if (*operand_count > max_wire_operands) {
      return std::nullopt;
    }

    Node node;
    node.kind = static_cast<NodeKind>(*kind);
    node.tag = static_cast<CtorTag>(*tag);
    node.binder = *binder;
    std::memcpy(&node.literal, &*literal, sizeof(node.literal));
    node.operands.reserve(*operand_count);
    for (std::uint8_t operand_index = 0; operand_index < *operand_count; ++operand_index) {
      std::optional<std::uint32_t> const operand = cursor.take_le<std::uint32_t, 4>();
      if (!operand.has_value()) {
        return std::nullopt;
      }
      node.operands.push_back(NodeIndex{ *operand });
    }
    image.nodes.push_back(std::move(node));
  }

  image.root = NodeIndex{ static_cast<std::uint32_t>(*node_count - 1) };
  if (!image_is_wellformed(image)) {
    return std::nullopt;
  }
  return image;
}

auto
encode_image(Image const& image) -> std::vector<std::uint8_t>
{
  std::vector<std::uint8_t> bytes;
  bytes.push_back(image_version);
  put_le<2>(bytes, static_cast<std::uint64_t>(image.nodes.size()));
  for (Node const& node : image.nodes) {
    bytes.push_back(static_cast<std::uint8_t>(node.kind));
    bytes.push_back(static_cast<std::uint8_t>(node.tag));
    put_le<4>(bytes, static_cast<std::uint64_t>(node.binder));
    std::uint64_t literal_bits = 0;
    std::memcpy(&literal_bits, &node.literal, sizeof(literal_bits));
    put_le<8>(bytes, literal_bits);
    bytes.push_back(static_cast<std::uint8_t>(node.operands.size()));
    for (NodeIndex const operand : node.operands) {
      put_le<4>(bytes, static_cast<std::uint64_t>(operand.value));
    }
  }
  return bytes;
}

} // namespace gandr::compile_host
