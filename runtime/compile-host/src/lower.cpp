#include "gandr/compile_host/dialect.hpp"
#include "gandr/compile_host/emit.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/pipeline.hpp"
#include "gandr/compile_host/status.hpp"
#include "gandr/compile_host/value.hpp"
#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlowOps.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/MemRef/IR/MemRef.h"
#include "mlir/IR/Block.h"
#include "mlir/IR/Builders.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/IRMapping.h"
#include "mlir/IR/Operation.h"
#include "mlir/IR/Region.h"
#include "mlir/IR/Value.h"
#include "mlir/Support/LLVM.h"

#include <cstddef>
#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <string_view>

namespace gandr::compile_host {
namespace {

/// The name the dialect entry point is moved to while its lowered replacement
/// takes the exported name.
constexpr std::string_view staged_entry_name = "gandr_positive_core_dialect";

/// The lowering's per-walk state.
///
/// The insertion point moves as dispatches split the control flow, so the
/// builder — rather than any block the caller remembers — is what says where
/// the next operation belongs.
// NOLINTBEGIN(cppcoreguidelines-avoid-const-or-ref-data-members):
// A per-walk borrow rather than a value: the state exists for the duration of
// one walk, is never copied, assigned, or stored, and the builder it holds is
// the caller's. A pointer would say the same thing and admit null.
struct LowerState
{
  /// The builder for the replacement function.
  mlir::OpBuilder& builder;
  /// The replacement function's heap argument.
  mlir::Value heap;
  /// The heap's word count, read from the argument in the entry block so it
  /// dominates every allocation site that compares against it.
  mlir::Value heap_limit;
  /// Dialect values to their lowered machine-word replacements.
  mlir::IRMapping mapping;
  /// The one block every refused allocation branches to, built on first use.
  mlir::Block* refusal = nullptr;
};

// NOLINTEND(cppcoreguidelines-avoid-const-or-ref-data-members)

/// Converts a machine-word offset into the index type memory operations take.
[[nodiscard]] auto
word_index(LowerState& state, mlir::Value offset) -> mlir::Value
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  return mlir::arith::IndexCastOp::create(builder, location, builder.getIndexType(), offset).getResult();
}

/// Materializes a machine-word constant.
[[nodiscard]] auto
word_constant(LowerState& state, std::int64_t value) -> mlir::Value
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  return mlir::arith::ConstantOp::create(builder, location, builder.getI64Type(), builder.getI64IntegerAttr(value))
    .getResult();
}

/// Loads the heap word at `base + offset`.
[[nodiscard]] auto
load_word(LowerState& state, mlir::Value base, std::int64_t offset) -> mlir::Value
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Value address = base;
  if (offset != 0) {
    address = mlir::arith::AddIOp::create(builder, location, base, word_constant(state, offset)).getResult();
  }
  return mlir::memref::LoadOp::create(builder, location, state.heap, mlir::ValueRange{ word_index(state, address) })
    .getResult();
}

/// Stores `value` at the heap word `base + offset`.
void
store_word(LowerState& state, mlir::Value base, std::int64_t offset, mlir::Value value)
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Value address = base;
  if (offset != 0) {
    address = mlir::arith::AddIOp::create(builder, location, base, word_constant(state, offset)).getResult();
  }
  mlir::memref::StoreOp::create(builder, location, value, state.heap, mlir::ValueRange{ word_index(state, address) });
}

/// Reads the heap argument's word count as a machine word.
///
/// The count comes from the memref descriptor the caller passed, so the
/// compiled code learns its own bound from its argument rather than from a
/// number the host baked in at compile time. A heap sized by one caller and a
/// program compiled for another therefore still checks correctly.
[[nodiscard]] auto
heap_word_count(LowerState& state) -> mlir::Value
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Value const axis = mlir::arith::ConstantOp::create(builder, location, builder.getIndexAttr(0)).getResult();
  mlir::Value const extent = mlir::memref::DimOp::create(builder, location, state.heap, axis).getResult();
  return mlir::arith::IndexCastOp::create(builder, location, builder.getI64Type(), extent).getResult();
}

/// The block a refused allocation branches to, created once per function.
///
/// The block sets the heap's exhaustion flag and returns. The flag is the
/// channel: the entry point returns one machine word, so a refusal cannot be
/// distinguished from an answer in the return value alone, and a caller that
/// read the word without reading the flag would read a heap offset that was
/// never allocated.
[[nodiscard]] auto
refusal_block(LowerState& state) -> mlir::Block*
{
  if (state.refusal != nullptr) {
    return state.refusal;
  }
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::OpBuilder::InsertionGuard const guard(builder);

  mlir::Region* region = builder.getInsertionBlock()->getParent();
  mlir::Block* refusal = builder.createBlock(region, region->end(), {}, {});
  builder.setInsertionPointToEnd(refusal);
  store_word(
    state,
    word_constant(state, static_cast<std::int64_t>(HeapLayout::exhaustion_flag)),
    0,
    word_constant(state, 1)
  );
  mlir::func::ReturnOp::create(builder, location, mlir::ValueRange{ word_constant(state, 0) });

  state.refusal = refusal;
  return refusal;
}

/// Bump-allocates `words` heap words and returns the base offset.
///
/// The allocation is checked against the heap's own extent before the cursor
/// moves, so no store the compiled code performs can address a word outside
/// the caller's heap. A refusal leaves the cursor where it was and branches to
/// the function's refusal block; the reference interpreter refuses at the same
/// point, which is what makes the short-heap differential a comparison rather
/// than two independent conventions.
[[nodiscard]] auto
allocate_cell(LowerState& state, std::int64_t words) -> mlir::Value
{
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Value const cursor_slot = word_constant(state, static_cast<std::int64_t>(HeapLayout::bump_cursor));
  mlir::Value const base = load_word(state, cursor_slot, 0);
  mlir::Value const advanced
    = mlir::arith::AddIOp::create(builder, location, base, word_constant(state, words)).getResult();
  mlir::Value const fits
    = mlir::arith::CmpIOp::create(builder, location, mlir::arith::CmpIPredicate::sle, advanced, state.heap_limit)
        .getResult();

  mlir::Block* refusal = refusal_block(state);
  mlir::Block* attempted = builder.getInsertionBlock();
  mlir::Region* region = attempted->getParent();
  mlir::Block* fitted = builder.createBlock(region, region->end(), {}, {});

  builder.setInsertionPointToEnd(attempted);
  mlir::cf::CondBranchOp::create(builder, location, fits, fitted, mlir::ValueRange{}, refusal, mlir::ValueRange{});

  builder.setInsertionPointToEnd(fitted);
  store_word(state, cursor_slot, 0, advanced);
  return base;
}

/// Increments one of the heap's ledger words.
///
/// This is where duplication and discard stop being declarations and become
/// observable work: the increment is the effect the dialect's memory-effect
/// annotation announces, and the regression witness reads the result back.
void
record_work(LowerState& state, std::size_t ledger_word)
{
  mlir::Value const slot = word_constant(state, static_cast<std::int64_t>(ledger_word));
  mlir::Value const current = load_word(state, slot, 0);
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Value const incremented
    = mlir::arith::AddIOp::create(builder, location, current, word_constant(state, 1)).getResult();
  store_word(state, slot, 0, incremented);
}

/// The cell tag a constructor tag builds.
[[nodiscard]] auto
cell_tag_word(CtorTag tag) noexcept -> std::int64_t
{
  switch (tag) {
    case CtorTag::Unit:
      return static_cast<std::int64_t>(CellTag::Unit);
    case CtorTag::Pair:
      return static_cast<std::int64_t>(CellTag::Pair);
    case CtorTag::Inl:
      return static_cast<std::int64_t>(CellTag::Inl);
    case CtorTag::Inr:
      return static_cast<std::int64_t>(CellTag::Inr);
  }
  return static_cast<std::int64_t>(CellTag::Unit);
}

[[nodiscard]] auto
lower_region(mlir::Region& region, mlir::Value argument, LowerState& state, std::size_t depth) -> Expected<mlir::Value>;

/// Lowers the operations of one dialect block, in order.
///
/// Returns the machine word the block's terminator produces: the yielded value
/// for a consumer region, and the cut's produced value for the entry block.
///
/// # Termination
/// - reason: a dialect operation carries regions of dialect operations, so
///   lowering a block is defined by lowering the blocks nested inside it; the
///   mutual pair with `lower_region` mirrors that nesting.
/// - measure: `max_emit_depth - depth`, which strictly decreases on every edge
///   of the pair because each one passes `depth + 1`.
/// - boundedness: `max_emit_depth` is a compile-time constant and the entry
///   test refuses anything above it.
/// - input recursion: yes, over a module built from a decoded image whose
///   depth an adversary chooses; the constant bound is what makes that safe,
///   and the explicit worklist the conventions prefer is owed rather than
///   present.
// NOLINTBEGIN(misc-no-recursion,readability-function-cognitive-complexity):
// The recursion carries a termination contract above. The complexity is the
// dialect-operation dispatch, one arm per operation, held together so the
// lowering of a block reads in one place.
[[nodiscard]] auto
lower_operations(mlir::Block& block, LowerState& state, std::size_t depth) -> Expected<mlir::Value>
{
  if (depth > max_emit_depth) {
    return host_error(ErrorKind::LimitExceeded, "module nests deeper than the host lowers");
  }

  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();

  for (mlir::Operation& op : block.getOperations()) {
    if (auto literal = mlir::dyn_cast<dialect::LitOp>(op)) {
      mlir::Value const cell = allocate_cell(state, 2);
      store_word(state, cell, 0, word_constant(state, static_cast<std::int64_t>(CellTag::Int)));
      store_word(state, cell, 1, word_constant(state, literal.getValue()));
      state.mapping.map(literal.getResult(), cell);
      continue;
    }
    if (auto constructor = mlir::dyn_cast<dialect::CtorOp>(op)) {
      std::optional<CtorTag> const tag = dialect::tag_from_attribute(constructor.getTag());
      if (!tag.has_value()) {
        return host_error(ErrorKind::LoweringFailed, "constructor names no declared tag");
      }
      // Widen before adding: the sum of an arity and one is computed in the
      // wide type rather than in the tag's own 32-bit one.
      auto const width = 1 + static_cast<std::int64_t>(ctor_arity(*tag));
      mlir::Value const cell = allocate_cell(state, width);
      store_word(state, cell, 0, word_constant(state, cell_tag_word(*tag)));
      std::int64_t field_offset = 1;
      for (mlir::Value const field : constructor.getFields()) {
        store_word(state, cell, field_offset, state.mapping.lookup(field));
        field_offset += 1;
      }
      state.mapping.map(constructor.getResult(), cell);
      continue;
    }
    if (auto duplication = mlir::dyn_cast<dialect::DupOp>(op)) {
      mlir::Value const source = state.mapping.lookup(duplication.getSource());
      mlir::Value const cell = allocate_cell(state, 3);
      store_word(state, cell, 0, word_constant(state, static_cast<std::int64_t>(CellTag::Pair)));
      store_word(state, cell, 1, source);
      store_word(state, cell, 2, source);
      record_work(state, HeapLayout::duplication_ledger);
      state.mapping.map(duplication.getResult(), cell);
      continue;
    }
    if (auto discard = mlir::dyn_cast<dialect::DropOp>(op)) {
      // The discarded producer is still evaluated: the operand is
      // already a lowered value, and dropping it is accounted rather
      // than elided.
      mlir::Value const cell = allocate_cell(state, 1);
      store_word(state, cell, 0, word_constant(state, static_cast<std::int64_t>(CellTag::Unit)));
      record_work(state, HeapLayout::discard_ledger);
      state.mapping.map(discard.getResult(), cell);
      continue;
    }
    if (auto frame = mlir::dyn_cast<dialect::BindOp>(op)) {
      // A binder frame's continuation is a straight-line region, so the
      // region disappears into a value binding and no control flow is
      // created. This is the measured half of the region-lowering cost:
      // a consumer that only binds costs nothing beyond the mapping.
      mlir::Value const bound = state.mapping.lookup(frame.getBound());
      Expected<mlir::Value> const answer = lower_region(frame.getBody(), bound, state, depth + 1);
      if (!answer.has_value()) {
        return answer;
      }
      state.mapping.map(frame.getResult(), *answer);
      continue;
    }
    if (auto dispatch = mlir::dyn_cast<dialect::CaseOp>(op)) {
      // A dispatch is the half that does cost control flow: the tag is
      // read, two arms become blocks, and the answers meet at a join
      // block's argument.
      mlir::Value const scrutinee = state.mapping.lookup(dispatch.getScrutinee());
      mlir::Value const tag = load_word(state, scrutinee, 0);
      mlir::Value const payload = load_word(state, scrutinee, 1);
      mlir::Value const takes_left = mlir::arith::CmpIOp::create(
                                       builder,
                                       location,
                                       mlir::arith::CmpIPredicate::eq,
                                       tag,
                                       word_constant(state, static_cast<std::int64_t>(CellTag::Inl))
      )
                                       .getResult();

      mlir::Region* region = builder.getInsertionBlock()->getParent();
      mlir::Block* left_block = builder.createBlock(region, region->end(), {}, {});
      mlir::Block* right_block = builder.createBlock(region, region->end(), {}, {});
      mlir::Block* join_block = builder.createBlock(region, region->end(), { builder.getI64Type() }, { location });

      builder.setInsertionPointAfterValue(takes_left);
      mlir::cf::CondBranchOp::create(
        builder,
        location,
        takes_left,
        left_block,
        mlir::ValueRange{},
        right_block,
        mlir::ValueRange{}
      );

      builder.setInsertionPointToEnd(left_block);
      Expected<mlir::Value> const left = lower_region(dispatch.getLeftArm(), payload, state, depth + 1);
      if (!left.has_value()) {
        return left;
      }
      mlir::cf::BranchOp::create(builder, location, join_block, mlir::ValueRange{ *left });

      builder.setInsertionPointToEnd(right_block);
      Expected<mlir::Value> const right = lower_region(dispatch.getRightArm(), payload, state, depth + 1);
      if (!right.has_value()) {
        return right;
      }
      mlir::cf::BranchOp::create(builder, location, join_block, mlir::ValueRange{ *right });

      builder.setInsertionPointToEnd(join_block);
      state.mapping.map(dispatch.getResult(), join_block->getArgument(0));
      continue;
    }
    if (auto yielded = mlir::dyn_cast<dialect::YieldOp>(op)) {
      return state.mapping.lookup(yielded.getResult());
    }
    if (auto cut = mlir::dyn_cast<dialect::CutOp>(op)) {
      return state.mapping.lookup(cut.getProduced());
    }
    return host_error(
      ErrorKind::LoweringFailed,
      std::string("no lowering for operation ") + op.getName().getStringRef().str()
    );
  }

  return host_error(ErrorKind::LoweringFailed, "block ends without a yield or a cut");
}

// NOLINTEND(misc-no-recursion,readability-function-cognitive-complexity)

/// Lowers one region's entry block under an argument.
///
/// # Termination
/// - reason: the other half of the mutual pair above; a region is lowered by
///   lowering its entry block.
/// - measure: `max_emit_depth - depth`, shared with `lower_operations` and
///   strictly decreasing on every edge of the pair.
/// - boundedness: `max_emit_depth` is a compile-time constant, tested by
///   `lower_operations` on entry.
/// - input recursion: yes, with the same bound and the same owed worklist as
///   `lower_operations`.
// NOLINTBEGIN(misc-no-recursion):
// The other half of the mutual pair, under the same termination contract.
auto
lower_region(mlir::Region& region, mlir::Value argument, LowerState& state, std::size_t depth) -> Expected<mlir::Value>
{
  if (region.empty()) {
    return host_error(ErrorKind::LoweringFailed, "consumer region holds no block");
  }
  mlir::Block& body = region.front();
  if (body.getNumArguments() != 1) {
    return host_error(ErrorKind::LoweringFailed, "consumer region takes no single binding");
  }
  state.mapping.map(body.getArgument(0), argument);
  return lower_operations(body, state, depth);
}

// NOLINTEND(misc-no-recursion)

} // namespace

auto
lower_dialect_operations(mlir::ModuleOp module) -> Expected<void>
{
  auto staged = module.lookupSymbol<mlir::func::FuncOp>(std::string(entry_point_name));
  if (!staged) {
    return host_error(ErrorKind::LoweringFailed, "module holds no entry point to lower");
  }
  if (staged.getBody().empty()) {
    return host_error(ErrorKind::LoweringFailed, "entry point has no body");
  }

  staged.setSymName(std::string(staged_entry_name));

  mlir::OpBuilder builder(module.getContext());
  mlir::Location const location = builder.getUnknownLoc();
  builder.setInsertionPointToEnd(module.getBody());
  auto lowered = mlir::func::FuncOp::create(builder, location, std::string(entry_point_name), staged.getFunctionType());
  lowered->setAttr("llvm.emit_c_interface", builder.getUnitAttr());
  mlir::Block* entry = lowered.addEntryBlock();
  builder.setInsertionPointToEnd(entry);

  LowerState state{
    .builder = builder,
    .heap = entry->getArgument(0),
    .heap_limit = {},
    .mapping = {},
    .refusal = nullptr,
  };
  state.heap_limit = heap_word_count(state);
  Expected<mlir::Value> const produced = lower_operations(staged.getBody().front(), state, 0);
  if (!produced.has_value()) {
    staged.erase();
    lowered.erase();
    return std::unexpected(produced.error());
  }
  mlir::func::ReturnOp::create(builder, location, mlir::ValueRange{ *produced });

  staged.erase();
  return Expected<void>{};
}

} // namespace gandr::compile_host
