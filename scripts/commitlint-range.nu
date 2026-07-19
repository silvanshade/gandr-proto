#!/usr/bin/env nu

# Lint the conventional-commit messages a `git push` is about to publish.
#
# This is the [pre-push] message backstop for commitlint — the commit-msg hook
# already lints each message as it is written, but rebases, cherry-picks, and
# `--no-verify` commits can land messages that never passed it, and a
# history-rewriting force-push re-publishes the whole range. This gate re-checks
# the exact set of commits the push will add to the remote.
#
# WHY A SCRIPT (not a one-line `commitlint --from` in prek.toml): the prior
# in-config form was `commitlint --verbose --from` with NO ref and the default
# `pass_filenames = true`, so prek appended the push's CHANGED FILES as
# positionals; `--from` greedily consumed the first filename as a git ref and
# commitlint built a garbage `<file>..HEAD` range, failing EVERY push. The fix is
# `pass_filenames = false` plus an explicitly resolved range.
#
# The push range (incl. the force-pushed-rewrite "no merge-base" case) is
# resolved by lib/push-range.nu, shared with the signature gate so both check the
# same commits.
use lib/push-range.nu [resolve-push-range]
# git-sanitized strips the hook-runner's injected git env (ADR-4; scripts/lib/git.nu).
use lib/git.nu [git-sanitized]

def main []: nothing -> nothing {
    let plan = (resolve-push-range)
    match $plan.mode {
        "range" => {
            print $"commitlint: linting commit range ($plan.from)..($plan.to)"
            # The external's nonzero exit propagates as this script's exit code,
            # which is what prek reads to pass/fail the push.
            ^mise x -- commitlint --verbose --from $plan.from --to $plan.to
        }
        "full" => {
            print $"commitlint: rewrite/no shared base — linting the full pushed history up to ($plan.to)"
            # `root..TO` excludes the root commit, so lint the root separately. A
            # failure in the first call aborts before the second (fail-closed),
            # which is the correct behaviour for a gate.
            ^mise x -- commitlint --verbose --from $plan.root --to $plan.to
            (git-sanitized ['log' '-1' '--format=%B' $plan.root]).stdout | ^mise x -- commitlint --verbose
        }
        "last" => {
            print "commitlint: no push range available — linting the last commit"
            ^mise x -- commitlint --verbose --last
        }
    }
}
