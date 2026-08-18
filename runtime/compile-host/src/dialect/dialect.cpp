#include "gandr/compile_host/dialect.hpp"

#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h"
#include "llvm/ADT/TypeSwitch.h"

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

namespace gandr::compile_host::dialect
{

void GandrDialect::initialize()
{
    addTypes<
#define GET_TYPEDEF_LIST
#include "GandrOpsTypes.cpp.inc"
        >();
    addOperations<
#define GET_OP_LIST
#include "GandrOps.cpp.inc"
        >();
}

std::uint32_t tag_to_attribute(CtorTag tag) noexcept
{
    return static_cast<std::uint32_t>(static_cast<std::uint8_t>(tag));
}

std::optional<CtorTag> tag_from_attribute(std::uint32_t value) noexcept
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
mlir::LogicalResult CtorOp::verify()
{
    const std::optional<CtorTag> tag = tag_from_attribute(getTag());
    if (!tag.has_value()) {
        return emitOpError() << "constructor tag " << getTag() << " names no declared constructor";
    }
    const std::uint32_t declared = ctor_arity(*tag);
    const auto supplied = static_cast<std::uint32_t>(getFields().size());
    if (declared != supplied) {
        return emitOpError() << "constructor tag " << getTag() << " declares arity " << declared
                             << " but was given " << supplied << " arguments";
    }
    return mlir::success();
}

/// Holds a binder frame's region to the one-block, one-binding shape the
/// lowering assumes.
mlir::LogicalResult BindOp::verify()
{
    mlir::Region& body = getBody();
    if (body.getNumArguments() != 1) {
        return emitOpError() << "binder frame region takes exactly one binding, found "
                             << body.getNumArguments();
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
mlir::LogicalResult CaseOp::verify()
{
    for (mlir::Region* arm : {&getLeftArm(), &getRightArm()}) {
        if (arm->getNumArguments() != 1) {
            return emitOpError() << "dispatch arm takes exactly one binding, found "
                                 << arm->getNumArguments();
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
