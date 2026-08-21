#include "gandr/compile_host/emit.hpp"

#include "gandr/compile_host/access.hpp"
#include "gandr/compile_host/dialect.hpp"
#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/status.hpp"
#include "mlir/Conversion/ArithToLLVM/ArithToLLVM.h"
#include "mlir/Conversion/ControlFlowToLLVM/ControlFlowToLLVM.h"
#include "mlir/Conversion/FuncToLLVM/ConvertFuncToLLVM.h"
#include "mlir/Conversion/MemRefToLLVM/MemRefToLLVM.h"
#include "mlir/Conversion/UBToLLVM/UBToLLVM.h"
#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlow.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/Dialect/MemRef/IR/MemRef.h"
#include "mlir/Dialect/UB/IR/UBOps.h"
#include "mlir/IR/Block.h"
#include "mlir/IR/Builders.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/BuiltinTypeInterfaces.h"
#include "mlir/IR/OwningOpRef.h"
#include "mlir/IR/Value.h"
#include "mlir/Target/LLVMIR/Dialect/Builtin/BuiltinToLLVMIRTranslation.h"
#include "mlir/Target/LLVMIR/Dialect/LLVMIR/LLVMToLLVMIRTranslation.h"

#include <cstddef>
#include <cstdint>
#include <expected>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

namespace gandr::compile_host {
namespace {

/// The dialect value type every producer in an emitted module carries.
[[nodiscard]] auto
value_type(mlir::MLIRContext& context) -> mlir::Type
{
  return dialect::ValueType::get(&context);
}

/// The emitted entry point's signature: the caller's heap in, the produced
/// value's heap offset out.
[[nodiscard]] auto
entry_signature(mlir::MLIRContext& context) -> mlir::FunctionType
{
  mlir::Builder builder(&context);
  mlir::Type const heap = mlir::MemRefType::get({ mlir::ShapedType::kDynamic }, builder.getI64Type());
  return builder.getFunctionType({ heap }, { builder.getI64Type() });
}

/// The emitter's per-walk state.
// NOLINTBEGIN(cppcoreguidelines-avoid-const-or-ref-data-members):
// A per-walk borrow rather than a value: the state exists for the duration of
// one walk, is never copied, assigned, or stored, and the builder it holds is
// the caller's. A pointer would say the same thing and admit null.
struct EmitState
{
  /// The builder, whose insertion point stays at the end of the block the
  /// next operation belongs to.
  mlir::OpBuilder& builder;
  /// The image being walked.
  Image const& image;
  /// The bindings in scope, innermost last, as dialect values.
  std::vector<mlir::Value> environment;
};

// NOLINTEND(cppcoreguidelines-avoid-const-or-ref-data-members)

/// Emits one image node, recursively over its operands.
///
/// # Termination
/// - reason: the image is a tree and the emission of a node is defined by the
///   emission of its operands, so the walk has the shape of the data.
/// - measure: `max_emit_depth - depth`, which strictly decreases on every
///   recursive edge because each one passes `depth + 1`.
/// - boundedness: `max_emit_depth` is a compile-time constant and the entry
///   test refuses anything above it, so the stack is bounded by a constant
///   rather than by the image.
/// - input recursion: yes, over a decoded image whose depth an adversary
///   chooses; the constant bound is what makes that safe, and the explicit
///   worklist the conventions prefer is owed rather than present.
// NOLINTBEGIN(misc-no-recursion,readability-function-cognitive-complexity):
// The recursion carries a termination contract above, which is the obligation
// the first check stands in for. The complexity is one switch over the node
// kinds with a short arm each: splitting it scatters a tabular dispatch across
// functions that would each have to re-establish the same state.
[[nodiscard]] auto
emit_node(EmitState& state, NodeIndex node_index, std::size_t depth) -> Expected<mlir::Value>
{
  if (depth > max_emit_depth) {
    return host_error(ErrorKind::LimitExceeded, "image nests deeper than the host admits");
  }

  Node const& node = proved_at(state.image.nodes, node_index.value);
  mlir::OpBuilder& builder = state.builder;
  mlir::Location const location = builder.getUnknownLoc();
  mlir::Type const produced = value_type(*builder.getContext());

  switch (node.kind) {
    case NodeKind::Lit: {
      auto op = dialect::LitOp::create(builder, location, produced, node.literal);
      return op.getResult();
    }
    case NodeKind::Var: {
      if (node.binder >= state.environment.size()) {
        return host_error(ErrorKind::MalformedImage, "variable names no binder in scope");
      }
      return proved_at(state.environment, state.environment.size() - 1 - node.binder);
    }
    case NodeKind::Ctor: {
      std::vector<mlir::Value> fields;
      fields.reserve(node.operands.size());
      for (NodeIndex const operand : node.operands) {
        Expected<mlir::Value> const field = emit_node(state, operand, depth + 1);
        if (!field.has_value()) {
          return field;
        }
        fields.push_back(*field);
      }
      auto op = dialect::CtorOp::create(builder, location, produced, dialect::tag_to_attribute(node.tag), fields);
      return op.getResult();
    }
    case NodeKind::Dup: {
      Expected<mlir::Value> const source = emit_node(state, proved_at(node.operands, 0), depth + 1);
      if (!source.has_value()) {
        return source;
      }
      auto op = dialect::DupOp::create(builder, location, produced, *source);
      return op.getResult();
    }
    case NodeKind::Drop: {
      Expected<mlir::Value> const source = emit_node(state, proved_at(node.operands, 0), depth + 1);
      if (!source.has_value()) {
        return source;
      }
      auto op = dialect::DropOp::create(builder, location, produced, *source);
      return op.getResult();
    }
    case NodeKind::Bind: {
      Expected<mlir::Value> const bound = emit_node(state, proved_at(node.operands, 0), depth + 1);
      if (!bound.has_value()) {
        return bound;
      }
      auto frame = dialect::BindOp::create(builder, location, produced, *bound);
      mlir::Block* body = builder.createBlock(&frame.getBody(), frame.getBody().end(), { produced }, { location });
      state.environment.push_back(body->getArgument(0));
      Expected<mlir::Value> const answer = emit_node(state, proved_at(node.operands, 1), depth + 1);
      state.environment.pop_back();
      if (!answer.has_value()) {
        return answer;
      }
      dialect::YieldOp::create(builder, location, *answer);
      builder.setInsertionPointAfter(frame);
      return frame.getResult();
    }
    case NodeKind::Case: {
      Expected<mlir::Value> const scrutinee = emit_node(state, proved_at(node.operands, 0), depth + 1);
      if (!scrutinee.has_value()) {
        return scrutinee;
      }
      auto dispatch = dialect::CaseOp::create(builder, location, produced, *scrutinee);
      for (std::size_t arm_index = 0; arm_index < 2; ++arm_index) {
        mlir::Region& arm = (arm_index == 0) ? dispatch.getLeftArm() : dispatch.getRightArm();
        mlir::Block* body = builder.createBlock(&arm, arm.end(), { produced }, { location });
        state.environment.push_back(body->getArgument(0));
        Expected<mlir::Value> const answer = emit_node(state, proved_at(node.operands, 1 + arm_index), depth + 1);
        state.environment.pop_back();
        if (!answer.has_value()) {
          return answer;
        }
        dialect::YieldOp::create(builder, location, *answer);
      }
      builder.setInsertionPointAfter(dispatch);
      return dispatch.getResult();
    }
    case NodeKind::Cut:
      return emit_node(state, proved_at(node.operands, 0), depth + 1);
  }
  return host_error(ErrorKind::MalformedImage, "image holds a node of no declared kind");
}

// NOLINTEND(misc-no-recursion,readability-function-cognitive-complexity)

} // namespace

auto
make_context() -> std::unique_ptr<mlir::MLIRContext>
{
  mlir::DialectRegistry registry;
  registry.insert<
    dialect::GandrDialect,
    mlir::arith::ArithDialect,
    mlir::cf::ControlFlowDialect,
    mlir::func::FuncDialect,
    mlir::memref::MemRefDialect,
    mlir::ub::UBDialect,
    mlir::LLVM::LLVMDialect>();
  // Without these the one-shot conversion to the LLVM dialect *succeeds and
  // does nothing*: the pass finds no pattern interface on the dialects it
  // meets and leaves them in place, so the failure surfaces only later, as a
  // module that will not translate.
  mlir::arith::registerConvertArithToLLVMInterface(registry);
  mlir::cf::registerConvertControlFlowToLLVMInterface(registry);
  mlir::registerConvertFuncToLLVMInterface(registry);
  mlir::registerConvertMemRefToLLVMInterface(registry);
  mlir::ub::registerConvertUBToLLVMInterface(registry);

  mlir::registerBuiltinDialectTranslation(registry);
  mlir::registerLLVMDialectTranslation(registry);

  auto context = std::make_unique<mlir::MLIRContext>(registry);
  context->loadAllAvailableDialects();
  return context;
}

auto
emit_module(mlir::MLIRContext& context, Image const& image) -> Expected<mlir::OwningOpRef<mlir::ModuleOp>>
{
  if (!image_is_wellformed(image)) {
    return host_error(ErrorKind::MalformedImage, "image failed the structural check");
  }

  mlir::OpBuilder builder(&context);
  mlir::Location const location = builder.getUnknownLoc();
  mlir::OwningOpRef<mlir::ModuleOp> module = mlir::ModuleOp::create(builder, location);

  builder.setInsertionPointToEnd(module->getBody());
  auto entry = mlir::func::FuncOp::create(builder, location, std::string(entry_point_name), entry_signature(context));
  entry->setAttr("llvm.emit_c_interface", builder.getUnitAttr());
  mlir::Block* body = entry.addEntryBlock();
  builder.setInsertionPointToEnd(body);

  EmitState state{ .builder = builder, .image = image, .environment = {} };
  Expected<mlir::Value> const produced = emit_node(state, image.root, 0);
  if (!produced.has_value()) {
    return std::unexpected(produced.error());
  }
  dialect::CutOp::create(builder, location, *produced);

  return module;
}

auto
emit_malformed_arity_module(mlir::MLIRContext& context) -> mlir::OwningOpRef<mlir::ModuleOp>
{
  mlir::OpBuilder builder(&context);
  mlir::Location const location = builder.getUnknownLoc();
  mlir::OwningOpRef<mlir::ModuleOp> module = mlir::ModuleOp::create(builder, location);

  builder.setInsertionPointToEnd(module->getBody());
  auto entry = mlir::func::FuncOp::create(builder, location, std::string(entry_point_name), entry_signature(context));
  entry->setAttr("llvm.emit_c_interface", builder.getUnitAttr());
  mlir::Block* body = entry.addEntryBlock();
  builder.setInsertionPointToEnd(body);

  mlir::Type const produced = value_type(context);
  auto only_field = dialect::LitOp::create(builder, location, produced, std::int64_t{ 1 });
  // The pair tag declares two fields and is given one. The builder accepts
  // it; the operation verifier is the only thing that does not.
  auto malformed = dialect::CtorOp::create(
    builder,
    location,
    produced,
    dialect::tag_to_attribute(CtorTag::Pair),
    mlir::ValueRange{ only_field.getResult() }
  );
  dialect::CutOp::create(builder, location, malformed.getResult());

  return module;
}

auto
emit_accounted_work_witness_module(mlir::MLIRContext& context) -> mlir::OwningOpRef<mlir::ModuleOp>
{
  mlir::OpBuilder builder(&context);
  mlir::Location const location = builder.getUnknownLoc();
  mlir::OwningOpRef<mlir::ModuleOp> module = mlir::ModuleOp::create(builder, location);

  builder.setInsertionPointToEnd(module->getBody());
  auto entry = mlir::func::FuncOp::create(builder, location, std::string(entry_point_name), entry_signature(context));
  entry->setAttr("llvm.emit_c_interface", builder.getUnitAttr());
  mlir::Block* body = entry.addEntryBlock();
  builder.setInsertionPointToEnd(body);

  mlir::Type const produced = value_type(context);
  auto accounted = dialect::LitOp::create(builder, location, produced, std::int64_t{ 4 });
  dialect::DupOp::create(builder, location, produced, accounted.getResult());
  dialect::DropOp::create(builder, location, produced, accounted.getResult());
  auto answer = dialect::LitOp::create(builder, location, produced, std::int64_t{ 0 });
  dialect::CutOp::create(builder, location, answer.getResult());

  return module;
}

auto
count_dialect_operations(mlir::ModuleOp module, std::string_view mnemonic) -> std::size_t
{
  std::size_t total = 0;
  std::string const qualified = std::string("gandr.") + std::string(mnemonic);
  module->walk([&total, &qualified](mlir::Operation* op) -> void {
    if (op->getName().getStringRef() == qualified) {
      total += 1;
    }
  });
  return total;
}

} // namespace gandr::compile_host
