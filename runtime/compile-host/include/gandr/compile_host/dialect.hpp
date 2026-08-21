// The generated gandr dialect, and the arity declaration its verifier reads.
//
// # Contract
// - requires: an `mlir::MLIRContext` the dialect is loaded into before any
//   operation of it is built.
// - ensures: the operations, the opaque value type, and their verifiers are
//   the generated ones; nothing here re-states a declaration TableGen owns.
// - provides: the only dialect the emitter builds into.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_DIALECT_HPP
#define GANDR_COMPILE_HOST_DIALECT_HPP

#include "gandr/compile_host/image.hpp"
#include "mlir/Bytecode/BytecodeOpInterface.h"
#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/IR/OpImplementation.h"
#include "mlir/Interfaces/InferTypeOpInterface.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

#include <cstdint>
#include <optional>

// The generated declarations are compiled inside this translation unit, so the
// host's own warning wall reaches them. TableGen emits unused parameters in the
// operand-range helpers it generates for every operation; suppressing exactly
// that diagnostic across the generated span keeps the wall on hand-written code
// without relaxing it globally.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wconversion"
#pragma clang diagnostic ignored "-Wsign-conversion"

#include "GandrOpsDialect.h.inc"

#define GET_TYPEDEF_CLASSES
#include "GandrOpsTypes.h.inc"

#define GET_OP_CLASSES
#include "GandrOps.h.inc"

#pragma clang diagnostic pop

namespace gandr::compile_host::dialect {

/// The integer encoding of a constructor tag as the `gandr.ctor` attribute
/// carries it.
///
/// # Contract
/// - ensures: injective on `CtorTag`, and `tag_from_attribute` inverts it on
///   the image of this function.
/// - panics: none.
[[nodiscard]] auto
tag_to_attribute(CtorTag tag) noexcept -> std::uint32_t;

/// Recovers a constructor tag from the `gandr.ctor` attribute value.
///
/// # Contract
/// - ensures: inverts `tag_to_attribute` on its image.
/// - fails: returns `std::nullopt` for a value naming no declared tag, which
///   is what lets the verifier reject an invented tag rather than trusting it.
/// - panics: none.
[[nodiscard]] auto
tag_from_attribute(std::uint32_t value) noexcept -> std::optional<CtorTag>;

} // namespace gandr::compile_host::dialect

#endif // GANDR_COMPILE_HOST_DIALECT_HPP
