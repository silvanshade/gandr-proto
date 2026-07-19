# scripts/lib/git.nu  (library module — `use`d, no shebang, no `main`)
#
# Shared git invocation for core gate scripts that shell out from a HOOK context.
# prek exports a RELATIVE `GIT_INDEX_FILE` into hook subprocesses (and git may
# inherit `GIT_DIR` / `GIT_WORK_TREE`), which re-resolve against a `git -C` target
# or the wrong tree — fatal inside a submodule mount whose `.git` is a gitlink file
# (ADR 4). Every git call in a hook-context gate goes through here so
# the ambient git environment is never trusted; Nushell removes a var set to null.
#
# The return is the same `{stdout, stderr, exit_code}` record shape as
# `^git … | complete`, so a call site swaps `(^git ARGS | complete)` for
# `(git-sanitized [ARGS])` with no other change.
export def git-sanitized [
    args: list<string> # the git argument vector, e.g. [rev-parse --show-toplevel]
]: nothing -> record<stdout: string, stderr: string, exit_code: int> {
    with-env {GIT_INDEX_FILE: null, GIT_DIR: null, GIT_WORK_TREE: null} {
        do { ^git ...$args } | complete
    }
}
