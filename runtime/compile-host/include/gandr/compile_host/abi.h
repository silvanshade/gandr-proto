/* The compilation host's stable C boundary.
 *
 * This is the only surface a foreign caller uses. It is C rather than C++ on
 * purpose: the host is built against a discovered MLIR installation, and a
 * caller that had to share a C++ ABI with it would have to share that
 * installation too. Everything crossing here is plain old data.
 *
 * # Contract
 * - requires: `gandr_compile_host_abi_version` agrees with the caller's
 *   expectation before any other entry is called.
 * - ensures: no C++ exception escapes an entry point, and no entry point
 *   aborts; every failure arrives as a status in the outcome.
 * - provides: the image-in, answer-out boundary the Rust bridge drives.
 * - panics: none.
 */

#ifndef GANDR_COMPILE_HOST_ABI_H
#define GANDR_COMPILE_HOST_ABI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

/* The version of this boundary.
 *
 * It changes whenever a field, an entry point, or a status meaning changes.
 * A caller compares it before calling anything else, because a mismatched
 * struct layout across a dynamic boundary has no other symptom.
 */
#define GANDR_COMPILE_HOST_ABI_VERSION 1u

/* A successful run. */
#define GANDR_COMPILE_HOST_STATUS_OK 0

/* The failure statuses, one per host error kind, offset by one so that zero
 * stays the success value. The numbering follows `ErrorKind` and is part of
 * this boundary rather than of the C++ enum. */
#define GANDR_COMPILE_HOST_STATUS_MALFORMED_IMAGE    1
#define GANDR_COMPILE_HOST_STATUS_VERIFIER_REJECTED  2
#define GANDR_COMPILE_HOST_STATUS_LOWERING_FAILED    3
#define GANDR_COMPILE_HOST_STATUS_CONVERSION_FAILED  4
#define GANDR_COMPILE_HOST_STATUS_EXECUTION_FAILED   5
#define GANDR_COMPILE_HOST_STATUS_RESULT_UNREADABLE  6
#define GANDR_COMPILE_HOST_STATUS_LIMIT_EXCEEDED     7
#define GANDR_COMPILE_HOST_STATUS_FIXTURE_UNREADABLE 8

/* A status the host itself never produces: the caller passed a null pointer,
 * or an entry point met a condition it could not attribute to a stage. */
#define GANDR_COMPILE_HOST_STATUS_BAD_CALL 100

  /* What one run produced.
   *
   * `text` is the rendered value on success and the failure detail otherwise. It
   * is owned by the host and released by `gandr_compile_host_outcome_release`;
   * it is never null after a call that returned, so a caller always has a
   * message to report.
   */
  typedef struct GandrCompileHostOutcome
  {
    /* `GANDR_COMPILE_HOST_STATUS_OK` or one of the failure statuses. */
    int32_t status;
    /* How many duplications the run executed. */
    int64_t duplications;
    /* How many discards the run executed. */
    int64_t discards;
    /* The arena words the run consumed, above the reserved prefix. */
    uint64_t allocated_words;
    /* The rendered value, or the failure detail. NUL-terminated. */
    char const* text;
  } GandrCompileHostOutcome;

  /* The version of this boundary, as the built library implements it. */
  uint32_t
  gandr_compile_host_abi_version(void);

  /* Compiles and runs an encoded program image, sizing the heap from the image.
   *
   * # Contract
   * - requires: `bytes` addresses `length` readable bytes, or `length` is zero;
   *   `outcome` is not null.
   * - ensures: `outcome` is filled in and its `text` is owned by the host.
   * - provides: the ordinary entry: the heap is the host's own bound for the
   *   image, so the caller does not have to know the sizing rule.
   * - fails: through `outcome->status`; the return value repeats it.
   * - panics: none.
   */
  int32_t
  gandr_compile_host_run(uint8_t const* bytes, size_t length, GandrCompileHostOutcome* outcome);

  /* Compiles and runs an encoded program image on a heap of the caller's size.
   *
   * # Contract
   * - requires: as `gandr_compile_host_run`.
   * - ensures: the run refuses any allocation that would not fit `heap_words`,
   *   reporting `GANDR_COMPILE_HOST_STATUS_LIMIT_EXCEEDED` rather than writing
   *   outside the heap.
   * - provides: the entry that exercises the compiled bounds check from the
   *   other side of the boundary.
   * - fails: through `outcome->status`; the return value repeats it.
   * - panics: none.
   */
  int32_t
  gandr_compile_host_run_with_heap(
    uint8_t const* bytes,
    size_t length,
    uint64_t heap_words,
    GandrCompileHostOutcome* outcome
  );

  /* Interprets an encoded program image on the reference walk, sizing the heap
   * from the image.
   *
   * # Contract
   * - requires: as `gandr_compile_host_run`.
   * - ensures: the answer the reference interpreter produces, in the same
   *   outcome shape.
   * - provides: the differential's other side, reachable from a caller that has
   *   no MLIR of its own — so a bridge can compare the two host paths without
   *   reimplementing either.
   * - fails: through `outcome->status`; the return value repeats it.
   * - panics: none.
   */
  int32_t
  gandr_compile_host_interpret(uint8_t const* bytes, size_t length, GandrCompileHostOutcome* outcome);

  /* Releases what a filled outcome owns.
   *
   * # Contract
   * - requires: `outcome` was filled by one of the entries above, or is null.
   * - ensures: `outcome->text` is null afterwards, so a double release is inert.
   * - panics: none.
   */
  void
  gandr_compile_host_outcome_release(GandrCompileHostOutcome* outcome);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* GANDR_COMPILE_HOST_ABI_H */
