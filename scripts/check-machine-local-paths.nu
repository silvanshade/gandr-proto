#!/usr/bin/env nu

# Publishable-history leak guard (core/PUBLISHABLE-HISTORY.md; ADR 4) — scan
# staged files for machine-local paths. Two pattern classes:
#
#   * absolute home paths — /Users/<name> or /home/<name> — ALWAYS flagged:
#     they embed a username and are contributor-concern by construction;
#   * tilde paths — home-relative — flagged UNLESS allowlisted: dot-directory
#     config references and generic documentation placeholders are legitimate,
#     personal development-tree layout is not.
#
# Wire via prek with pass_filenames (fragments/prek.base.toml):
#   entry = "mise exec -- nu .agents/core/scripts/check-machine-local-paths.nu"
#
# NOTE: messages avoid literal parens — `\(` inside a `$'...'` interpolation
# does not escape in Nushell (it opens an expression).

# Tilde-path allowlist: a hit is tolerated when it starts with one of these
# prefixes. Grow deliberately; prefer rephrasing docs over widening this list.
const TILDE_ALLOW: list<string> = [
    '~/.'                # dot-directory config/cache references, XDG-style
    '~/your-'            # generic documentation placeholder
    '~/path/to'          # generic documentation placeholder
    '~/beads-planning'   # beads' stock config template comments
    '~/work-planning'    # beads' stock config template comments
]

const ABS_RE = '(?P<hit>/(Users|home)/[A-Za-z0-9_.-]+[A-Za-z0-9_./-]*)'
const TILDE_RE = '(?P<hit>~/[A-Za-z0-9_.][A-Za-z0-9_./-]*)'

# Scan one file's content; one row per offending path occurrence. Unreadable
# or deleted staged paths scan as empty — absent content cannot leak. Binary
# staged files (fonts, images) scan as empty too: `open --raw` types them
# `binary`, and a binary payload cannot carry a machine-local text path.
export def scan-file [file: path]: nothing -> table<file: string, line: int, hit: string> {
    let content = (try { open --raw $file } catch { '' })
    let text = if ($content | describe | str starts-with 'string') { $content } else { '' }
    $text
    | lines
    | enumerate
    | each {|row|
        let abs = ($row.item | parse --regex $ABS_RE | get hit)
        let tilde = (
            $row.item
            | parse --regex $TILDE_RE
            | get hit
            | where {|h| not ($TILDE_ALLOW | any {|p| $h | str starts-with $p }) }
        )
        $abs | append $tilde | each {|h| {file: $file, line: ($row.index + 1), hit: $h} }
    }
    | flatten
}

def main [...files: string # staged files to scan for machine-local paths
]: nothing -> nothing {
    let offenders = ($files | each {|f| scan-file $f } | flatten)
    if ($offenders | is-empty) {
        print $'ok   no machine-local paths in ($files | length) staged files'
        return
    }
    for o in $offenders {
        print --stderr $'FAIL ($o.file):($o.line)  machine-local path: ($o.hit)'
    }
    print --stderr 'machine-local paths are contributor-concern; project history must stay publishable -- core/PUBLISHABLE-HISTORY.md'
    exit 1
}
