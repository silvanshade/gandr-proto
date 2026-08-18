/* A deliberately incomplete compilation-host boundary.
 *
 * It declares the same ABI version and exports the ordinary run entry, and it
 * exports **no release entry at all**. Nothing here computes anything: the
 * library exists so that a caller's symbol-resolution order is observable.
 *
 * The failure it witnesses is specific. A caller that resolved the release
 * entry only *after* invoking a run would, against this library, allocate the
 * outcome's text and then fail to find the function that frees it — leaking on
 * the way to reporting a bindable-library error. Resolving release first turns
 * that into a refusal before anything is allocated, and this library is what
 * makes the difference testable rather than argued.
 *
 * # Contract
 * - requires: nothing; every entry is total.
 * - ensures: the version matches the real boundary, so a caller reaches the
 *   symbol resolution rather than stopping at a version mismatch.
 * - provides: the boundary-order witness in the Rust bridge's test suite.
 * - fails: never; `gandr_compile_host_run` reports a bad call.
 * - panics: none.
 */

#include <stddef.h>
#include <stdint.h>

#include "gandr/compile_host/abi.h"

uint32_t gandr_compile_host_abi_version(void)
{
    return GANDR_COMPILE_HOST_ABI_VERSION;
}

int32_t gandr_compile_host_run(
    const uint8_t* bytes,
    size_t length,
    GandrCompileHostOutcome* outcome)
{
    (void)bytes;
    (void)length;
    if (outcome == NULL) {
        return GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    }
    /* A caller that reaches this has already failed the property under test:
     * it invoked a run against a library whose release entry it had not
     * resolved. The static text is owned by nobody, so nothing leaks even
     * then, and the status says what happened. */
    outcome->status = GANDR_COMPILE_HOST_STATUS_BAD_CALL;
    outcome->duplications = 0;
    outcome->discards = 0;
    outcome->allocated_words = 0;
    outcome->text = "this boundary exports no release entry";
    return outcome->status;
}
