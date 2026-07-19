# scripts/lib/push-range.nu  (library module — `use`d, no shebang, no `main`)
#
# Shared resolution of the commit range a `git push` will publish, for the
# [pre-push] gates. prek mirrors pre-commit and exports the push endpoints as
# `PRE_COMMIT_FROM_REF` (remote tip being updated) and `PRE_COMMIT_TO_REF` (local
# tip being pushed). This module turns them into ONE plan that every pre-push
# gate shares, so the message-lint gate and the signature gate check exactly the
# same set of commits. It handles three awkward cases identically:
#   - a brand-new remote ref sets FROM to all-zeros (no remote tip yet);
#   - a manual/standalone run leaves both unset;
#   - a FORCE-PUSHED REWRITE makes FROM and TO share NO commit (every hash
#     changes), so there is no merge-base — the gates must lint/check the full
#     pushed history rather than a `FROM..TO` range.
#
# `use std/assert` (module-qualified, NOT `use std/assert *`: the splat form
# imports the assertion variants unqualified and clobbers builtins like `length`).
use std/assert
# git-sanitized strips the hook-runner's injected git env (ADR-4); every git call
# below goes through it rather than a bare `^git … | complete`.
use git.nu [git-sanitized]

# Resolve what the push publishes: a `from..to` range when the endpoints share
# history; the full `root..to` history when disjoint (a force-pushed rewrite);
# the lone last commit when no lower bound exists at all.
export def resolve-push-range []: nothing -> record<mode: string, from: string, to: string, root: string> {
    let raw_to_env = $env.PRE_COMMIT_TO_REF?
    let to_env = (match $raw_to_env { null => "" _ => $raw_to_env } | str trim)
    let to = (if ($to_env | is-empty) { "HEAD" } else { $to_env })

    # A push endpoint is usable iff it is present and not the all-zeros sentinel
    # git uses for a not-yet-existing remote ref.
    let raw_from_env = $env.PRE_COMMIT_FROM_REF?
    let from_env = (match $raw_from_env { null => "" _ => $raw_from_env } | str trim)
    let from_is_real = (($from_env | is-not-empty) and ($from_env !~ '^0+$'))
    let from = (if $from_is_real { $from_env } else { fallback-lower-bound })

    let root = (root-commit $to)

    let plan = (if (($from | is-not-empty) and (shares-history $from $to)) {
        {mode: "range", from: $from, to: $to, root: $root}
    } else if ($root | is-not-empty) {
        {mode: "full", from: "", to: $to, root: $root}
    } else {
        {mode: "last", from: "", to: $to, root: ""}
    })

    # POST: each mode carries the ref it needs — `range` a lower bound, `full` a
    # root — so no consumer is ever handed an empty anchor.
    assert ($plan.mode in ["range" "full" "last"]) $"resolve-push-range: unknown mode ($plan.mode)"
    assert (($plan.mode != "range") or ($plan.from | is-not-empty)) "resolve-push-range: range mode requires a non-empty lower bound"
    assert (($plan.mode != "full") or ($plan.root | is-not-empty)) "resolve-push-range: full mode requires a non-empty root"
    $plan
}

# The commit SHAs the push will publish, for gates that inspect each commit (the
# signature gate). `range`: FROM..TO; `full`: ROOT..TO plus the root itself
# (which the range excludes); `last`: the single tip.
export def push-range-revs [
    plan: record<mode: string, from: string, to: string, root: string>
]: nothing -> list<string> {
    let revs = (match $plan.mode {
        "range" => (rev-list-range $"($plan.from)..($plan.to)")
        "full" => ((rev-list-range $"($plan.root)..($plan.to)") | append $plan.root)
        "last" => [(resolve-rev $plan.to)]
        _ => []
    })
    $revs | where ($it | is-not-empty)
}

# The commits in a `a..b` rev-range, or [] on error. The range is built as ONE
# quoted string so nushell's `..` operator never eats it (a bare `$a..$b` would).
def rev-list-range [range: string]: nothing -> list<string> {
    let r = (git-sanitized ['rev-list' $range])
    if ($r.exit_code == 0) {
        $r.stdout | lines | where ($it | is-not-empty)
    } else {
        []
    }
}

# Resolve a single rev to its full SHA, or "" on error.
def resolve-rev [rev: string]: nothing -> string {
    let r = (git-sanitized ['rev-parse' $rev])
    if ($r.exit_code == 0) { $r.stdout | str trim } else { "" }
}

# True when `from` and `to` share a common ancestor — i.e. a merge-base exists.
# A force-pushed rewrite yields none, which commitlint@21 treats as a hard error.
def shares-history [from: string, to: string]: nothing -> bool {
    let base = (git-sanitized ['merge-base' $from $to])
    (($base.exit_code == 0) and (($base.stdout | str trim) | is-not-empty))
}

# The first root (parent-less) commit reachable from `to`, or "" if none.
def root-commit [to: string]: nothing -> string {
    let roots = (git-sanitized ['rev-list' '--max-parents=0' $to])
    if ($roots.exit_code == 0) {
        let found = ($roots.stdout | lines | where ($it | is-not-empty))
        let first_root = $found.0?
        match $first_root { null => "" _ => $first_root }
    } else {
        ""
    }
}

# Resolve a lower bound when the push endpoints are absent: the branch's upstream
# tip, then the previous commit, then "" to signal there is no usable bound.
def fallback-lower-bound []: nothing -> string {
    let upstream = (git-sanitized ['rev-parse' '--verify' '--quiet' '@{upstream}'])
    let upstream_sha = ($upstream.stdout | str trim)
    if (($upstream.exit_code == 0) and ($upstream_sha | is-not-empty)) {
        $upstream_sha
    } else {
        let prev = (git-sanitized ['rev-parse' '--verify' '--quiet' 'HEAD~1'])
        let prev_sha = ($prev.stdout | str trim)
        if (($prev.exit_code == 0) and ($prev_sha | is-not-empty)) {
            $prev_sha
        } else {
            ""
        }
    }
}
