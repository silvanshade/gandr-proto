// The named programs the host compiles: one per transition, one compound, and
// the accounted-work regression program.
//
// # Contract
// - requires: nothing.
// - ensures: every sample satisfies `image_is_wellformed`.
// - provides: the programs the agreement fixture is written over, and the
//   generator the property harness draws from.
// - panics: none.

#ifndef GANDR_COMPILE_HOST_SAMPLES_HPP
#define GANDR_COMPILE_HOST_SAMPLES_HPP

#include "gandr/compile_host/image.hpp"
#include "gandr/compile_host/value.hpp"

#include <cstdint>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace gandr::compile_host {

/// One named program.
struct Sample
{
  /// The name the fixture keys the expected value by.
  std::string_view name;
  /// The program.
  Image image;
};

/// The canonical sample set: one program per positive-core transition, plus a
/// compound program threading a binder frame, a dispatch, and a constructor.
///
/// These are the seven programs the proving spike ran, restated as in-tree
/// images. The Rust L machine's answers to the same seven are pinned in
/// `fixtures/positive-core-samples.txt`, and a gated Rust test keeps that file
/// equal to what the machine produces.
///
/// # Contract
/// - ensures: exactly seven samples, in a stable order, each well-formed.
/// - provides: the agreement surface.
/// - panics: none.
[[nodiscard]] auto
canonical_samples() -> std::vector<Sample>;

/// The accounted-work regression program: a duplication and a discard whose
/// results are never observed.
///
/// Marking either operation pure would let canonicalization delete it, and the
/// program's answer would not change — which is exactly the failure mode the
/// proving spike measured. The witness is the ledger, not the answer.
///
/// # Contract
/// - ensures: the program's value is an integer that does not mention either
///   operation's result, and a correct run's ledger records one duplication
///   and one discard.
/// - provides: the regression witness's program.
/// - panics: none.
[[nodiscard]] auto
accounted_work_sample() -> Sample;

/// The work a correct run of `accounted_work_sample` accounts for.
inline constexpr WorkLedger accounted_work_expectation{ .duplications = 1, .discards = 1 };

/// Generates a well-formed program image pseudo-randomly.
///
/// The generator is deterministic in `seed`, which is what makes a property
/// failure reproducible from the report alone. It is deliberately biased
/// toward the boundary cases the decision surfaces have: nested dispatches,
/// nullary constructors, and duplications whose results are discarded.
///
/// # Contract
/// - ensures: the returned image satisfies `image_is_wellformed` and nests no
///   deeper than `max_generated_depth`.
/// - provides: the property harness's inputs and the fuzz corpus seeds.
/// - panics: none.
[[nodiscard]] auto
generate_image(std::uint64_t seed) -> Image;

/// The deepest consumer nesting the generator produces.
inline constexpr std::uint32_t max_generated_depth = 6;

} // namespace gandr::compile_host

#endif // GANDR_COMPILE_HOST_SAMPLES_HPP
