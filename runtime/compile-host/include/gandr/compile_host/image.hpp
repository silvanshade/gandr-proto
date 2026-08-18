// The positive-core program image: the plain-old-data boundary the host
// compiles from.
//
// # Contract
// - requires: a well-formed image, as `image_is_wellformed` decides it.
// - ensures: the representation is flat and index-addressed, so an image is
//   copyable, serializable, and free of owning pointers.
// - provides: the only input shape `emit_module` accepts.
// - fails: `decode_image` reports a typed failure; nothing here throws.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_IMAGE_HPP
#define GANDR_COMPILE_HOST_IMAGE_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <span>
#include <string_view>
#include <vector>

namespace gandr::compile_host
{

/// A node's index in an `Image`'s flat arena.
///
/// # Contract
/// - requires: the index addresses a node strictly before the referring node,
///   which is what makes an image acyclic without a separate check.
/// - panics: none.
struct NodeIndex
{
    /// The zero-based position in `Image::nodes`.
    std::uint32_t value = 0;

    friend constexpr bool operator==(NodeIndex, NodeIndex) = default;
};

/// The positive-core node kinds.
///
/// Six of these are the transitions this slice lowers; `Lit` and `Var` are the
/// producer leaves they operate over. The excluded block — codata and
/// call-by-need, effects, the host seam, reified continuations, and the
/// unconditional jump — has no representation here by construction.
enum class NodeKind : std::uint8_t
{
    /// An integer literal producer.
    Lit = 0,
    /// A reference to a binder introduced by an enclosing `Bind` or `Case`.
    Var = 1,
    /// A positive constructor introduction.
    Ctor = 2,
    /// The grade structural duplication.
    Dup = 3,
    /// The grade structural discard.
    Drop = 4,
    /// A sequencing consumer: the `mu-tilde` binder frame.
    Bind = 5,
    /// A constructor dispatch consumer.
    Case = 6,
    /// The terminal cut against the top-level consumer.
    Cut = 7,
};

/// The constructor tags this slice admits.
///
/// The arity each tag declares is `ctor_arity`, and the verifier holds an
/// emitted constructor to it. The set is the positive-core subset of the L
/// machine's own constructor tags.
enum class CtorTag : std::uint8_t
{
    /// The unit value; arity zero.
    Unit = 0,
    /// An eager pair; arity two.
    Pair = 1,
    /// The left sum injection; arity one.
    Inl = 2,
    /// The right sum injection; arity one.
    Inr = 3,
};

/// The number of producer arguments a constructor tag declares.
///
/// # Contract
/// - ensures: total over `CtorTag`, with no wildcard arm, so a new tag does
///   not compile until it declares its arity.
/// - provides: the arity the operation verifier compares an emitted
///   constructor's operand count against.
/// - panics: none.
[[nodiscard]] constexpr std::uint32_t ctor_arity(CtorTag tag) noexcept
{
    switch (tag) {
    case CtorTag::Unit:
        return 0;
    case CtorTag::Pair:
        return 2;
    case CtorTag::Inl:
    case CtorTag::Inr:
        return 1;
    }
    return 0;
}

/// The printable name of a constructor tag.
///
/// # Contract
/// - ensures: total, and the spelling matches the canonical value rendering
///   the agreement fixture uses.
/// - panics: none.
[[nodiscard]] constexpr std::string_view ctor_tag_name(CtorTag tag) noexcept
{
    switch (tag) {
    case CtorTag::Unit:
        return "unit";
    case CtorTag::Pair:
        return "pair";
    case CtorTag::Inl:
        return "inl";
    case CtorTag::Inr:
        return "inr";
    }
    return "unit";
}

/// One node of a program image.
///
/// The payload fields are interpreted per `kind`; a field a kind does not use
/// is zero. Keeping one flat record rather than a variant is what makes the
/// image a plain-old-data structure a foreign caller can hand over without
/// serialization.
struct Node
{
    /// Which positive-core form this node is.
    NodeKind kind = NodeKind::Lit;
    /// The constructor tag, for `NodeKind::Ctor`.
    CtorTag tag = CtorTag::Unit;
    /// How many binders separate the reference from the one it names, for
    /// `NodeKind::Var`. Zero is the innermost binder, so this is a de Bruijn
    /// index counted inwards rather than a level counted outwards; the
    /// evaluator reads it as `environment[size - 1 - binder]`.
    std::uint32_t binder = 0;
    /// The integer payload, for `NodeKind::Lit`.
    std::int64_t literal = 0;
    /// The operand list: constructor arguments; the duplicated or discarded
    /// producer; the bound producer then the body for `Bind`; the scrutinee
    /// then the two arm bodies for `Case`; the produced value for `Cut`.
    std::vector<NodeIndex> operands;
};

/// A complete program image: a flat node arena plus its root.
///
/// # Contract
/// - requires: `root` addresses a node in `nodes`, every operand addresses a
///   strictly earlier node, and every `Var` binder level is in scope at its
///   use site. `image_is_wellformed` decides all three.
/// - provides: the input `emit_module` and `interpret_image` both consume, so
///   the compiled and reference paths never disagree about the program.
/// - panics: none.
struct Image
{
    /// The node arena, in dependency order.
    std::vector<Node> nodes;
    /// The root node's index.
    NodeIndex root;
};

/// Whether an image satisfies the structural preconditions the emitter
/// assumes.
///
/// # Contract
/// - ensures: returns true exactly when the root is in range, every operand
///   points strictly backwards, every node's operand count matches its kind
///   (and its constructor tag's declared arity), and every `Var` names a
///   binder in scope.
/// - provides: the gate the decoder and the emitter both run, so a malformed
///   byte image cannot reach the builder.
/// - fails: returns false; it never reports which clause failed, because the
///   only consumers are a boolean gate.
/// - panics: none.
[[nodiscard]] bool image_is_wellformed(const Image& image) noexcept;

/// The number of binders a node's operand at `slot` is evaluated under,
/// relative to the node's own scope depth.
///
/// # Contract
/// - ensures: total; returns one for a `Bind` body and for each `Case` arm
///   body, and zero everywhere else.
/// - panics: none.
[[nodiscard]] std::uint32_t operand_binder_offset(NodeKind kind, std::size_t slot) noexcept;

/// Decodes a byte string into a program image.
///
/// The decoder is the fuzz entry surface: it is total over arbitrary bytes and
/// reports a typed absence rather than constructing a malformed image.
///
/// # Contract
/// - requires: nothing; `bytes` may be arbitrary.
/// - ensures: a returned image satisfies `image_is_wellformed`.
/// - provides: the property generator's and the fuzz target's shared entry.
/// - fails: returns `std::nullopt` for truncated, over-large, or structurally
///   invalid input.
/// - panics: none.
[[nodiscard]] std::optional<Image> decode_image(std::span<const std::uint8_t> bytes);

/// Encodes a program image into the byte form `decode_image` accepts.
///
/// # Contract
/// - requires: `image` satisfies `image_is_wellformed`.
/// - ensures: `decode_image(encode_image(i))` yields an image equal to `i`.
/// - provides: the fuzz corpus seeds, written from the canonical samples.
/// - panics: none.
[[nodiscard]] std::vector<std::uint8_t> encode_image(const Image& image);

/// The largest node count the decoder admits.
///
/// The bound exists so a hostile byte string cannot ask the host to allocate
/// an arbitrary arena; it is far above any program this slice compiles.
inline constexpr std::size_t max_image_nodes = 4096;

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_IMAGE_HPP
