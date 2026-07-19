#!/usr/bin/env nu

# Reject a push that would publish a commit without a good signature.
#
# This is the [pre-push] enforcement point for the project's sign-on-push model:
# commits may be authored unsigned (the AFK allowance), but the history that
# reaches the remote must be signed. It checks every commit the push publishes —
# the same range the message-lint gate uses (lib/push-range.nu), including the
# full history on a force-pushed rewrite.
#
# SIGNED-NESS is read from `git`'s `%G?` placeholder, which the repo's configured
# `gpg.format=ssh` + `gpg.ssh.allowedSignersFile` make meaningful:
#   G good · U good, unknown validity → ACCEPT (a good signature is present)
#   N none · B bad · E unverifiable · X expired · Y expired key · R revoked → REJECT
# (If `allowedSignersFile` is ever unset, validly-signed commits can read as
# unverifiable — fix the config rather than weakening this gate.)
#
# `use std/assert` (module-qualified, NOT the splat form which clobbers builtins).
use std/assert
use lib/push-range.nu [resolve-push-range push-range-revs]
# git-sanitized strips the hook-runner's injected git env (ADR-4; scripts/lib/git.nu).
use lib/git.nu [git-sanitized]

const GOOD_SIGNATURE = ["G" "U"]

def main []: nothing -> nothing {
    let plan = (resolve-push-range)
    let revs = (push-range-revs $plan)

    if ($revs | is-empty) {
        print "signed-commits OK: no commits to check"
        return
    }

    print $"signed-commits: checking ($revs | length) commit\(s) being pushed"

    # One `git log --no-walk` over the exact rev set: SHA, signature status, subject.
    let log_args = (['log' '--no-walk=unsorted' '--format=%H%x1f%G?%x1f%s'] | append $revs)
    let rows = (
        (git-sanitized $log_args).stdout
        | lines
        | where ($it | is-not-empty)
        | each {|line|
            let parts = ($line | split row (char --integer 0x1f))
            let sha = $parts.0?
            let status = $parts.1?
            let subject = $parts.2?
            {
                sha: (match $sha { null => "" _ => $sha })
                status: (match $status { null => "" _ => $status })
                subject: (match $subject { null => "" _ => $subject })
            }
        }
    )

    let unsigned = ($rows | where (not ($it.status in $GOOD_SIGNATURE)))

    if ($unsigned | is-empty) {
        print $"signed-commits OK: all ($revs | length) pushed commits carry a good signature"
    } else {
        print --stderr $"signed-commits FAILED: ($unsigned | length) of ($rows | length) pushed commits lack a good signature"
        print --stderr "  (status: N=none B=bad E=unverifiable X=expired Y=expired-key R=revoked)"
        for u in $unsigned {
            # INVARIANT: every row in `unsigned` came from `git log` over the push
            # set, so it has a resolvable SHA; assert before reporting it as one.
            assert ($u.sha | is-not-empty) "check-signed-commits: unsigned row missing its SHA"
            print --stderr $"  [($u.status)] ($u.sha | str substring 0..9)  ($u.subject)"
        }
        print --stderr "sign the range before pushing — e.g. `git rebase --root -f` re-creates every"
        print --stderr "commit signed (with commit.gpgsign on) — then retry the push."
        exit 1
    }
}
