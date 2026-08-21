# The pinned MLIR/LLVM revision.
#
# This file is the single place the pin is written, and it is written by
# `mise run compile-host:pin-update` rather than by hand. Everything that needs
# the pin reads it from here: the CMake configure, which refuses a toolchain
# that is not the pinned version, and the mise tasks, which parse the same
# `set()` lines.
#
# The mechanism follows the Mojo compiler's (KGEN): one exact revision with its
# archive and sha256, fetched rather than submoduled, with local modifications
# carried as a curated Stacked Git series over the fetched tree and a dedicated
# update path. A submodule is refused — the repository is enormous and the
# archive fetch is far faster.
#
# What is NOT here yet is the source build. Until it lands, the local bootstrap
# satisfies the pin from an installed toolchain, and the pin's force is the
# equality check: a keg whose version is not `GANDR_MLIR_PIN_VERSION` is a
# fatal configure error rather than an accepted substitute. That is what makes
# this a pin rather than a preference, and it is why no host design may lean on
# the discovered-toolchain posture.

# The upstream tag. `llvmorg-<version>`.
set(GANDR_MLIR_PIN_REVISION "llvmorg-22.1.8")

# The version every consumer compares against: the compiling clang, the
# discovered MLIR, and the archive's own contents.
set(GANDR_MLIR_PIN_VERSION "22.1.8")

# The archive the source build fetches.
set(GANDR_MLIR_PIN_ARCHIVE
    "https://github.com/llvm/llvm-project/releases/download/llvmorg-22.1.8/llvm-project-22.1.8.src.tar.xz"
)

# The archive's sha256, measured by fetching it and hashing it rather than
# transcribed: `curl -L <archive> | shasum -a 256`. It agrees with the digest
# Homebrew's own llvm formula records for the same release.
set(GANDR_MLIR_PIN_SHA256 "922f1817a0df7b1489272d18134ee0087a8b068828f87ac63b9861b1a9965888")

# The curated patch series applied over the fetched archive, as a Stacked Git
# series. Empty means the pin is upstream unmodified, which is the state a
# reader should be able to confirm at a glance.
set(GANDR_MLIR_PIN_PATCHES "${CMAKE_CURRENT_LIST_DIR}/../mlir-patches")
