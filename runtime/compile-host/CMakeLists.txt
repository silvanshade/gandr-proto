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

# MLIR is *found*, never fetched: it is an installed toolchain whose headers,
# libraries and TableGen generator must match the compiling clang exactly.
# `MLIR_DIR` is supplied by the invoking task, which discovers the installation
# rather than hard-coding one.
find_package(MLIR REQUIRED CONFIG)

list(APPEND CMAKE_MODULE_PATH "${MLIR_CMAKE_DIR}" "${LLVM_CMAKE_DIR}")
include(TableGen)
include(AddLLVM)
include(AddMLIR)
include(HandleLLVMOptions)

# The toolchain-match check. An `-frtti` MLIR consumed by a differently built
# clang is an ODR hazard that surfaces as a link or run-time failure far from
# its cause, so the mismatch is refused at configure time.
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
target_compile_options(gandr-compile-host-core PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})

# The C boundary, built as a shared library so a foreign caller resolves it at
# run time. Keeping it separate from the static core is what lets the Rust
# workspace build with no MLIR at all: nothing links this, and a caller that
# cannot find it reports an unavailable host rather than failing to compile.
add_library(gandr-compile-host-abi SHARED src/abi.cpp)
target_link_libraries(gandr-compile-host-abi PRIVATE gandr-compile-host-core)
target_compile_options(gandr-compile-host-abi PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})
set_target_properties(gandr-compile-host-abi PROPERTIES CXX_VISIBILITY_PRESET default)

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
  target_link_libraries(gandr-compile-host-tests PRIVATE gandr-compile-host-core)
  target_compile_options(gandr-compile-host-tests PRIVATE ${GANDR_COMPILE_HOST_WARNINGS})
  target_compile_definitions(
    gandr-compile-host-tests
    PRIVATE GANDR_COMPILE_HOST_FIXTURES="${CMAKE_CURRENT_SOURCE_DIR}/fixtures"
  )

  set(GANDR_COMPILE_HOST_TEST_CASES
      a_refused_run_writes_nothing_past_its_heap
      arity_verifier_rejects_a_malformed_constructor
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
