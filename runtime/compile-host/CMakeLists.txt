cmake_minimum_required(VERSION 3.28)

project(
  gandr-compile-host
  VERSION 0.1.0
  DESCRIPTION "gandr's C++26 MLIR compilation host"
  LANGUAGES CXX C
)

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

option(
  GANDR_COMPILE_HOST_ALLOW_TOOLCHAIN_MISMATCH
  "Configure even when the compiling clang and the discovered MLIR come from different installations"
  OFF
)
option(GANDR_COMPILE_HOST_BUILD_TESTS "Build the compile host's test binaries" ON)
option(GANDR_COMPILE_HOST_BUILD_FUZZ "Build the fuzz entry surface" ON)

# The pin. One revision, one version, one sha256, in one file that the mise
# tasks parse and this configure includes.
include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/mlir-pin.cmake)

# MLIR is *pinned*. Until the source build lands, the local bootstrap satisfies
# the pin from an installed toolchain — `MLIR_DIR` is supplied by the invoking
# task — and the pin's force is the equality check below: an installation that
# is not the pinned version is refused rather than accepted as a substitute.
find_package(MLIR REQUIRED CONFIG)

if(NOT LLVM_PACKAGE_VERSION VERSION_EQUAL GANDR_MLIR_PIN_VERSION)
  message(
    FATAL_ERROR
    "the MLIR at ${MLIR_INSTALL_PREFIX} is ${LLVM_PACKAGE_VERSION} but this tree pins ${GANDR_MLIR_PIN_REVISION} (${GANDR_MLIR_PIN_VERSION}); install the pinned toolchain or move the pin with 'mise run compile-host:pin-update'"
  )
endif()
message(STATUS "compile-host: pinned to ${GANDR_MLIR_PIN_REVISION}, satisfied by ${MLIR_INSTALL_PREFIX}")

list(APPEND CMAKE_MODULE_PATH "${MLIR_CMAKE_DIR}" "${LLVM_CMAKE_DIR}")
include(TableGen)
include(AddLLVM)
include(AddMLIR)
include(HandleLLVMOptions)

# The toolchain-match check. An `-frtti` MLIR consumed by a differently built
# clang is an ODR hazard that surfaces as a link or run-time failure far from
# its cause, so the mismatch is refused at configure time. The MLIR half is
# already held to the pin above, so agreeing here means both agree with it.
if(NOT CMAKE_CXX_COMPILER_VERSION VERSION_EQUAL LLVM_PACKAGE_VERSION)
  set(_gandr_toolchain_message
      "compiling C++ compiler is version ${CMAKE_CXX_COMPILER_VERSION} but the discovered MLIR is ${LLVM_PACKAGE_VERSION}"
  )
  if(GANDR_COMPILE_HOST_ALLOW_TOOLCHAIN_MISMATCH)
    message(WARNING "${_gandr_toolchain_message} (allowed by GANDR_COMPILE_HOST_ALLOW_TOOLCHAIN_MISMATCH)")
  else()
    message(
      FATAL_ERROR
      "${_gandr_toolchain_message}; point CMAKE_CXX_COMPILER at the clang++ that ships beside the discovered MLIR"
    )
  endif()
endif()

get_filename_component(_gandr_compiler_real "${CMAKE_CXX_COMPILER}" REALPATH)
get_filename_component(_gandr_mlir_prefix_real "${MLIR_INSTALL_PREFIX}" REALPATH)
string(FIND "${_gandr_compiler_real}" "${_gandr_mlir_prefix_real}" _gandr_prefix_position)
if(NOT _gandr_prefix_position EQUAL 0)
  message(
    WARNING
    "the compiling clang (${_gandr_compiler_real}) does not live under the discovered MLIR prefix (${_gandr_mlir_prefix_real})"
  )
endif()

include_directories(SYSTEM ${LLVM_INCLUDE_DIRS} ${MLIR_INCLUDE_DIRS})
add_definitions(${LLVM_DEFINITIONS})

# The dialect declarations. `add_mlir_dialect` is deliberately not used: it
# registers a dependency on MLIR's own `mlir-headers` target, which exists only
# inside an MLIR build tree and not in a consumer project.
set(LLVM_TARGET_DEFINITIONS ${CMAKE_CURRENT_SOURCE_DIR}/src/dialect/GandrOps.td)
mlir_tablegen(GandrOps.h.inc -gen-op-decls EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
mlir_tablegen(GandrOps.cpp.inc -gen-op-defs EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
mlir_tablegen(GandrOpsTypes.h.inc -gen-typedef-decls -typedefs-dialect=gandr EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
mlir_tablegen(GandrOpsTypes.cpp.inc -gen-typedef-defs -typedefs-dialect=gandr EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
mlir_tablegen(GandrOpsDialect.h.inc -gen-dialect-decls -dialect=gandr EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
mlir_tablegen(GandrOpsDialect.cpp.inc -gen-dialect-defs -dialect=gandr EXTRA_INCLUDES ${MLIR_INCLUDE_DIRS})
add_public_tablegen_target(GandrCompileHostDialectIncGen)

add_library(
  gandr-compile-host-core
  STATIC
  src/decode.cpp
  src/dialect/dialect.cpp
  src/emit.cpp
  src/image.cpp
  src/interpret.cpp
  src/jit.cpp
  src/lower.cpp
  src/pipeline.cpp
  src/samples.cpp
  src/value.cpp
)
add_dependencies(gandr-compile-host-core GandrCompileHostDialectIncGen)
target_include_directories(
  gandr-compile-host-core
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_BINARY_DIR}
)

# Link the aggregate shared libraries, never the component archives. Mixing
# the two gives the process two copies of MLIR's type-storage registries, and
# the first `Type::get` on a dialect type then crashes inside the uniquer with
# no diagnostic. The one exception is the execution engine: `libMLIR.dylib`
# does not export it, so its static archive is linked on top of the shared
# library, and nothing in `MLIRConfig.cmake` says so.
target_link_libraries(gandr-compile-host-core PUBLIC MLIR LLVM MLIRExecutionEngine)

set(GANDR_COMPILE_HOST_WARNINGS -Wall -Wextra -Wconversion -Wsign-conversion -Werror)

# Exceptions and RTTI follow the keg rather than a preference, because a
# translation unit that disagrees with the MLIR it links is an ODR hazard whose
# symptom appears far from its cause.
#
# The keg reports both. `LLVM_ENABLE_EH` is OFF on every LLVM this host is
# built against, so `-fno-exceptions` is what matching it means and the C
# boundary's promise is narrowed to say so: a recoverable host or validation
# failure is a status, and process-fatal resource exhaustion aborts exactly as
# it does inside LLVM's own fatal-error path.
#
# `LLVM_ENABLE_RTTI` is ON for the ordinary distribution kegs, and that is the
# lawful departure `docs/workflow/cpp.md` records: the MLIR-facing targets keep
# RTTI because the keg has it. It is recorded here, at the flag site, and it
# never propagates above this layer because there is nothing above it that does
# not link MLIR.
if(LLVM_ENABLE_EH)
  message(
    FATAL_ERROR
    "the discovered MLIR at ${MLIR_INSTALL_PREFIX} was built with exceptions enabled; this host is built -fno-exceptions to match a keg built without them"
  )
endif()
set(GANDR_COMPILE_HOST_POSTURE -fno-exceptions)
if(NOT LLVM_ENABLE_RTTI)
  list(APPEND GANDR_COMPILE_HOST_POSTURE -fno-rtti)
endif()
list(APPEND GANDR_COMPILE_HOST_WARNINGS ${GANDR_COMPILE_HOST_POSTURE})
message(STATUS "compile-host: posture ${GANDR_COMPILE_HOST_POSTURE} (keg RTTI=${LLVM_ENABLE_RTTI}, EH=${LLVM_ENABLE_EH})")
target_compile_options(gandr-compile-host-core PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})

# The C boundary, built as a shared library so a foreign caller resolves it at
# run time. Keeping it separate from the static core is what lets the Rust
# workspace build with no MLIR at all: nothing links this, and a caller that
# cannot find it reports an unavailable host rather than failing to compile.
add_library(gandr-compile-host-abi SHARED src/abi.cpp)
target_link_libraries(gandr-compile-host-abi PRIVATE gandr-compile-host-core)
target_compile_options(gandr-compile-host-abi PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})
set_target_properties(gandr-compile-host-abi PROPERTIES CXX_VISIBILITY_PRESET default)

# The same boundary as an archive, for a caller that binds at link time rather
# than through a symbol lookup. It is the same `src/abi.cpp` and the same core
# behind it, so there is one boundary implementation and two ways to reach it.
#
# What the archive deliberately does NOT do is restate the link discipline
# above. It carries `gandr-compile-host-core` as a PUBLIC dependency, and the
# core carries `MLIR`, `LLVM` and `MLIRExecutionEngine`, so a consumer that asks
# CMake for this target's link interface gets the aggregate shared libraries and
# never the component archives. That rule lives here and is not copied into any
# consumer's build script.
add_library(gandr-compile-host-abi-static STATIC src/abi.cpp)
target_link_libraries(gandr-compile-host-abi-static PUBLIC gandr-compile-host-core)
target_compile_options(gandr-compile-host-abi-static PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})

# The link line a foreign build system needs, written by the build that owns the
# discipline rather than reconstructed by the consumer. One flag per line.
file(
  GENERATE
  OUTPUT ${CMAKE_BINARY_DIR}/gandr-compile-host-link.txt
  CONTENT
    "$<TARGET_FILE:gandr-compile-host-abi-static>\n$<TARGET_FILE:gandr-compile-host-core>\n$<JOIN:$<TARGET_FILE:MLIR>,>\n$<JOIN:$<TARGET_FILE:LLVM>,>\n$<JOIN:$<TARGET_FILE:MLIRExecutionEngine>,>\n"
)

# A deliberately incomplete boundary: it declares the version and the ordinary
# run entry and exports no release. It exists so the Rust side's resolution
# order is witnessed rather than argued — a caller that resolved the release
# entry only after a run had already allocated the outcome's text would leak
# against exactly this library, and the witness is what makes that a failure.
add_library(gandr-compile-host-abi-partial SHARED tests/partial_abi.c)
set_target_properties(
  gandr-compile-host-abi-partial
  PROPERTIES C_VISIBILITY_PRESET default C_STANDARD 17 C_STANDARD_REQUIRED ON
)
target_include_directories(gandr-compile-host-abi-partial PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)

add_executable(gandr-compile-host tools/main.cpp)
target_link_libraries(gandr-compile-host PRIVATE gandr-compile-host-core)
target_compile_options(gandr-compile-host PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})

if(GANDR_COMPILE_HOST_BUILD_FUZZ)
  add_executable(gandr-compile-host-fuzz tools/fuzz_entry.cpp)
  target_link_libraries(gandr-compile-host-fuzz PRIVATE gandr-compile-host-core)
  target_compile_options(gandr-compile-host-fuzz PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})
endif()

if(GANDR_COMPILE_HOST_BUILD_TESTS)
  enable_testing()
  add_executable(gandr-compile-host-tests tests/main.cpp)
  # The boundary is linked into the test binary as well as built as the shared
  # library a foreign caller resolves, so its own paths are exercised by named
  # cases rather than only across the dynamic boundary.
  target_link_libraries(gandr-compile-host-tests PRIVATE gandr-compile-host-core gandr-compile-host-abi)
  target_compile_options(gandr-compile-host-tests PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})
  target_compile_definitions(
    gandr-compile-host-tests
    PRIVATE GANDR_COMPILE_HOST_FIXTURES="${CMAKE_CURRENT_SOURCE_DIR}/fixtures"
  )

  set(GANDR_COMPILE_HOST_TEST_CASES
      a_refused_run_writes_nothing_past_its_heap
      arity_verifier_rejects_a_malformed_constructor
      the_c_boundary_owns_and_releases_every_message
      canonicalization_preserves_accounted_work
      compiled_allocation_is_bounded_by_its_heap
      decoder_is_total_on_seed_corpus
      encode_decode_round_trips_every_sample
      jit_agrees_with_the_fixture_on_every_sample
      jit_agrees_with_the_reference_interpreter_under_generation
      ledger_records_every_executed_duplication_and_discard
      lowering_leaves_no_dialect_operation_behind
      verifier_runs_before_canonicalization_and_execution
  )
  foreach(case IN LISTS GANDR_COMPILE_HOST_TEST_CASES)
    add_test(NAME ${case} COMMAND gandr-compile-host-tests --case=${case})
  endforeach()
endif()
