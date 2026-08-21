#include "gandr/compile_host/dialect.hpp"

// TableGen writes the type parser, the type printer and their dispatch switch
// into the .inc files this translation unit includes below, so two of the
// headers here are used by generated text the include analysis does not read.
#include "gandr/compile_host/image.hpp"
#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h" // NOLINT(misc-include-cleaner)
#include "mlir/IR/Region.h"
#include "mlir/Support/LLVM.h"

#include "llvm/ADT/TypeSwitch.h" // NOLINT(misc-include-cleaner)

#include <cstdint>
#include <optional>

// See the note in `dialect.hpp`: the generated definitions are held outside the
// host's own warning wall, which stays on for everything below them.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wconversion"
#pragma clang diagnostic ignored "-Wsign-conversion"

#include "GandrOpsDialect.cpp.inc"

#define GET_TYPEDEF_CLASSES
#include "GandrOpsTypes.cpp.inc"

#define GET_OP_CLASSES
#include "GandrOps.cpp.inc"

#pragma clang diagnostic pop

namespace gandr::compile_host::dialect {

void
GandrDialect::initialize()
{
  // MLIR's own AbstractType::get returns a value holding lambdas built in its
  // frame, which the static analyser reads as a stack-address escape. The
  // report names an MLIR header this project cannot edit, and it is reached
  // only through this one registration call.
  // NOLINTNEXTLINE(clang-analyzer-core.StackAddressEscape)
  addTypes<
#define GET_TYPEDEF_LIST
#include "GandrOpsTypes.cpp.inc"
    >();
  addOperations<
#define GET_OP_LIST
#include "GandrOps.cpp.inc"
    >();
}

auto
tag_to_attribute(CtorTag tag) noexcept -> std::uint32_t
{
  return static_cast<std::uint32_t>(static_cast<std::uint8_t>(tag));
}

auto
tag_from_attribute(std::uint32_t value) noexcept -> std::optional<CtorTag>
{
  switch (value) {
    case 0:
      return CtorTag::Unit;
    case 1:
      return CtorTag::Pair;
    case 2:
      return CtorTag::Inl;
    case 3:
      return CtorTag::Inr;
    default:
      return std::nullopt;
  }
}

/// Holds a constructor's operand count to the arity its tag declares.
///
/// This is the price the proving spike measured and named: the Rust core's
/// exhaustive match makes a wrong arity fail to compile, while here the
/// operation builds and only the verifier objects. The check therefore has to
/// run, which is why the pipeline runs it before anything else touches the
/// module.
auto
CtorOp::verify() -> mlir::LogicalResult
{
  std::optional<CtorTag> const tag = tag_from_attribute(getTag());
  if (!tag.has_value()) {
    return emitOpError() << "constructor tag " << getTag() << " names no declared constructor";
  }
  std::uint32_t const declared = ctor_arity(*tag);
  auto const supplied = static_cast<std::uint32_t>(getFields().size());
  if (declared != supplied) {
    return emitOpError()
        << "constructor tag "
        << getTag()
        << " declares arity "
        << declared
        << " but was given "
        << supplied
        << " arguments";
  }
  return mlir::success();
}

/// Holds a binder frame's region to the one-block, one-binding shape the
/// lowering assumes.
auto
BindOp::verify() -> mlir::LogicalResult
{
  mlir::Region& body = getBody();
  if (body.getNumArguments() != 1) {
    return emitOpError() << "binder frame region takes exactly one binding, found " << body.getNumArguments();
  }
  if (body.getArgument(0).getType() != getBound().getType()) {
    return emitOpError() << "binder frame binding type does not match the bound producer";
  }
  if (!mlir::isa<YieldOp>(body.front().getTerminator())) {
    return emitOpError() << "binder frame region does not end in a yield";
  }
  return mlir::success();
}

/// Holds a dispatch's two arms to the one-block, one-binding shape the
/// lowering assumes, and to the yield terminator the lowering reads the arm's
/// answer from.
auto
CaseOp::verify() -> mlir::LogicalResult
{
  for (mlir::Region* arm : { &getLeftArm(), &getRightArm() }) {
    if (arm->getNumArguments() != 1) {
      return emitOpError() << "dispatch arm takes exactly one binding, found " << arm->getNumArguments();
    }
    if (!mlir::isa<ValueType>(arm->getArgument(0).getType())) {
      return emitOpError() << "dispatch arm binding is not a positive-core value";
    }
    if (!mlir::isa<YieldOp>(arm->front().getTerminator())) {
      return emitOpError() << "dispatch arm does not end in a yield";
    }
  }
  return mlir::success();
}

} // namespace gandr::compile_host::dialect
