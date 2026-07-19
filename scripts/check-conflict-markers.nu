#!/usr/bin/env nu

# Reject unresolved Git conflict markers before documentation formatters run.
# `assert` for the in-body runtime contracts (module-qualified, NOT `use std/assert
# *`: the splat shadows builtins like `length` this gate relies on).
use std/assert
# git-sanitized strips the hook-runner's injected git env (ADR-4; scripts/lib/git.nu).
use lib/git.nu [git-sanitized]

# Git writes conflict markers as EXACTLY seven marker characters, followed by
# end-of-line or a space + label (e.g. `<<<<<<< HEAD`, bare `=======`). Anchoring
# on `($|[ \t])` keeps real markers matched while rejecting longer decorative
# runs — notably tree-sitter corpus delimiters (`====…`/`----…`, 80 chars) and
# repeated shell operators, which are not conflicts.
const MARKER_PATTERN = r#'^(<<<<<<<|=======|>>>>>>>|\|\|\|\|\|\|\|)($|[ \t])'#

# Scan tracked files by default, or explicit paths when formatter guards pass them.
def main [
    ...paths: path # files to scan directly; defaults to tracked files in the current worktree
]: nothing -> nothing {
    let scan_paths = if ($paths | is-empty) { tracked-files } else { $paths }
    let findings = ($scan_paths | each {|file| scan-file $file } | flatten)
    for finding in $findings {
        # POST: the flattened scan table is renderable. The record type says the
        # fields exist, but only runtime data can prove path/line/text are useful
        # diagnostic coordinates rather than empty or non-marker values.
        assert (($finding.path | into string) | is-not-empty) 'main: conflict finding path must be non-empty'
        assert ($finding.line > 0) $'main: conflict finding line must be positive for ($finding.path)'
        assert ($finding.text =~ $MARKER_PATTERN) $'main: conflict finding text is not marker-shaped for ($finding.path):($finding.line)'
    }

    if ($findings | is-empty) {
        print $'conflict marker check OK: no conflict markers found in ($scan_paths | length) files'
    } else {
        print --stderr 'conflict marker check FAILED: unresolved Git conflict markers found'
        for finding in $findings {
            print --stderr $'($finding.path):($finding.line):($finding.text)'
        }
        exit 1
    }
}

# Return git-tracked file paths in the current worktree.
def tracked-files []: nothing -> list<path> {
    let root = (git-sanitized ['rev-parse' '--show-toplevel'])
    if $root.exit_code != 0 {
        error make {msg: ($'not a git worktree: ($root.stderr | str trim)' | str trim)}
    }

    let root_path = ($root.stdout | str trim)
    let listed = (git-sanitized ['-C' $root_path 'ls-files'])
    if $listed.exit_code != 0 {
        error make {msg: ($'git ls-files failed: ($listed.stderr | str trim)' | str trim)}
    }

    let tracked = (
        $listed.stdout
        | lines
        | where ($it | is-not-empty)
        | each {|path| $root_path | path join $path }
        | where ($it | path exists)
    )
    for path in $tracked {
        # POST: git emitted a non-empty tracked path that still exists after we
        # joined it to the repo root. `list<path>` cannot express either value
        # property, and a deleted-between-commands path must not reach scanners.
        assert (($path | into string) | is-not-empty) 'tracked-files: tracked path must be non-empty'
        assert ($path | path exists) $'tracked-files: tracked path must exist: ($path)'
    }
    $tracked
}

# Return conflict-marker findings for one existing regular file.
def scan-file [file: path]: nothing -> list<record<path: path, line: int, text: string>> {
    if not ($file | path exists) {
        error make {msg: $'file not found: ($file)'}
    }

    if (($file | path type) != 'file') {
        []
    } else {
        let display_path = try { $file | path relative-to (pwd) } catch { $file }
        # PRE: findings below capture this display path. Its path-ish type cannot
        # state that rendering it will name something, so assert the value before
        # it is copied into every diagnostic record.
        assert (($display_path | into string) | is-not-empty) $'scan-file: display path must be non-empty for ($file)'
        let body = (try { open --raw $file } catch {|err| error make {msg: $'failed to read ($file): ($err.msg)'} })

        $body
        | lines
        | enumerate
        | where item =~ $MARKER_PATTERN
        | each {|entry|
            let finding = {
                path: $display_path
                line: ($entry.index + 1)
                text: $entry.item
            }
            # INVARIANT: `where item =~ MARKER_PATTERN` and `enumerate` establish
            # marker text plus one-based line numbers dynamically; the return type
            # cannot encode either, so assert before the record leaves scan-file.
            assert (($finding.path | into string) | is-not-empty) 'scan-file: conflict finding path must be non-empty'
            assert ($finding.line > 0) $'scan-file: conflict finding line must be positive for ($finding.path)'
            assert ($finding.text =~ $MARKER_PATTERN) $'scan-file: conflict finding text is not marker-shaped for ($finding.path):($finding.line)'
            $finding
        }
    }
}
