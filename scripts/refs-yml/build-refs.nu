# Derive docs/spec/refs.yml (hayagriva) from the register extraction + curated
# citation fields. IDs and LOCATORS come verbatim from rows.nuon (the mechanical
# register extraction); TITLE/AUTHOR/DATE/VENUE come from data.nu (curated by
# reading the register citation column). The provenance header is prepended.
#
# Two-stage derivation orchestrated by `mise run docs:refs-yml`, which runs both
# stages with the canonical paths (a private temp file carries the intermediate
# rows.nuon). That task is the entry point (docs/workflow/scripting.md: typed
# scripts are reached through their named mise task, never inlined). To drive the
# stages by hand, set each stage's env vars, from the repository root:
#   1. REG_PATH=docs/research/bibliography-v2.md OUT_PATH=$tmp/rows.nuon \
#        nu scripts/refs-yml/extract.nu
#   2. ROWS_PATH=$tmp/rows.nuon OUT_PATH=docs/spec/refs.yml \
#        nu scripts/refs-yml/build-refs.nu
#
# Serialization is `to yaml` FOLLOWED BY oxfmt (see the tail of this script).
# `to yaml`'s string-quote style (single vs double) is non-deterministic across
# cold nu processes, so this script normalizes its own output through oxfmt —
# the SAME formatter treefmt runs on *.yml (treefmt.toml [formatter.oxfmt], with
# .oxfmtrc.json) — as its final step. oxfmt is idempotent on its canonical form,
# so the artifact is a stable fixed point: byte-reproducible from any cold run
# here and left unchanged by a later `treefmt` pass. Without this step refs.yml
# is byte-reproducible only by accident of a subsequent treefmt run.

use data.nu curated-entries

# Parse the register Locator column into structured hayagriva locator fields.
# The register convention: the FIRST token is the resolvable primary
# (doi:/arXiv:/<URL>); anything after it is secondary (alternates, flags,
# no-locator statements) and is preserved verbatim in loc_note.
def parse-locator [loc: string]: nothing -> record {
    let l = ($loc | str trim)
    mut out = {doi: null, arxiv: null, url: null, loc_note: null}
    if ($l | str starts-with 'doi:') {
        let d = ($l | parse --regex '^doi:(?<v>[^\s;]+)' | get v.0)
        let rest = ($l | str substring (4 + ($d | str length)).. | str trim)
        $out.doi = $d
        $out.loc_note = (if ($rest | is-empty) { null } else { $rest })
    } else if ($l | str starts-with 'arXiv:') {
        let a = ($l | parse --regex '^arXiv:(?<v>[^\s;]+)' | get v.0)
        let rest = ($l | str substring (6 + ($a | str length)).. | str trim)
        $out.arxiv = $a
        # co-primary `; doi:...` (R-44/49/50/51 form)
        let co = ($rest | parse --regex '^;\s*doi:(?<v>[^\s;]+)$')
        if ($co | is-not-empty) {
            $out.doi = ($co | get v.0)
        } else {
            $out.loc_note = (if ($rest | is-empty) { null } else { $rest })
        }
    } else if ($l | str starts-with '<') {
        let u = ($l | parse --regex '^<(?<v>[^>]+)>' | get v.0)
        let rest = ($l | str substring (2 + ($u | str length)).. | str trim)
        $out.url = $u
        $out.loc_note = (if ($rest | is-empty) { null } else { $rest })
    } else {
        # no structured primary (no-locator statement, slides-only, etc.)
        $out.loc_note = $l
    }
    $out
}

# Extract the leading iu refs.yml key from a section-R citation: `key` — ...
def iu-key [citation: string]: nothing -> any {
    let m = ($citation | parse --regex '^`(?<k>[^`]+)`')
    if ($m | is-empty) { null } else { $m | get k.0 }
}

def build []: nothing -> record {
    let rows = (open $env.ROWS_PATH)
    let curated = (curated-entries)
    let by_id = ($curated | reduce --fold {} {|it, acc| $acc | insert $it.id $it })

    mut result = {}
    for row in $rows {
        let id = $row.id
        # skip cross-reference placeholders (non-entries)
        if ($id in ['J-22' 'J-25d']) { continue }
        if not ($id in ($by_id | columns)) {
            error make {msg: $'no curated entry for register id ($id)'}
        }
        let c = ($by_id | get $id)
        let loc = (parse-locator $row.locator)

        # assemble serial-number
        mut sn = {}
        if ($loc.doi != null) { $sn = ($sn | insert doi $loc.doi) }
        if ($loc.arxiv != null) { $sn = ($sn | insert arxiv $loc.arxiv) }

        # assemble note: curated note (corrections/xref/gloss) + iu key (R) + locator remainder
        mut note_parts = []
        if (($c.note? | default null) != null) { $note_parts = ($note_parts | append $c.note) }
        if ($id | str starts-with 'R-') {
            let k = (iu-key $row.citation)
            if ($k != null) { $note_parts = ($note_parts | append $'iu refs.yml key: ($k)') }
        }
        if ($loc.loc_note != null) { $note_parts = ($note_parts | append $loc.loc_note) }
        let note = (if ($note_parts | is-empty) { null } else { $note_parts | str join ' — ' })

        # build entry in a stable field order
        mut e = {type: $c.type, title: $c.title}
        if (($c.author? | default null) != null) { $e = ($e | insert author $c.author) }
        if (($c.date? | default null) != null) { $e = ($e | insert date $c.date) }
        if (($c.genre? | default null) != null) { $e = ($e | insert genre $c.genre) }
        if (($c.publisher? | default null) != null) { $e = ($e | insert publisher $c.publisher) }
        if (($c.org? | default null) != null) { $e = ($e | insert organization $c.org) }
        if (($c.venue? | default null) != null) {
            let vt = ($c.vtype? | default 'proceedings')
            $e = ($e | insert parent {type: $vt, title: $c.venue})
        }
        if ($loc.url != null) { $e = ($e | insert url $loc.url) }
        if ($sn | is-not-empty) { $e = ($e | insert serial-number $sn) }
        if ($note != null) { $e = ($e | insert note $note) }

        $result = ($result | insert $id $e)
    }
    $result
}

const header = "# gandr central citation register — hayagriva bibliography (Typst `bibliography`).
#
# DERIVED ARTIFACT — DO NOT hand-edit entries here.
# Source of truth: docs/research/bibliography-v2.md
#   (gandr Consolidated Literature Register v2, gandr-fcw.10). The register is
#   authoritative; to change a citation, edit the register and re-derive.
#
# One entry per register REFERENCE row. The YAML key is the register row id
#   (the id-as-name scheme, A-1a … V-9), stable and unique. Titles, authors,
#   venues, years, DOIs, arXiv ids and URLs are transcribed verbatim from the
#   register; rows with no stable locator carry the register's note instead.
#
# Intentional non-entries: the cross-reference placeholder rows J-22 and J-25d
#   (they point at J-8 and J-19..J-21), and the register's Appendix 1/2
#   DECLINED / EXCLUDED candidate works (never register rows).
"

# Repository root, resolved from this script's own location (cwd-independent) so
# the oxfmt config below is found no matter where the generator is invoked.
const script_dir = (path self | path dirname)

let body = (build | to yaml)
$"($header)($body)" | save -f $env.OUT_PATH

# Normalize the quote style deterministically through the treefmt *.yml formatter
# (see the header). `to yaml` alone is not byte-reproducible across cold nu
# processes; oxfmt projects any quote style onto a single canonical fixed point.
if (which oxfmt | is-empty) {
    error make {msg: 'oxfmt not found on PATH — the refs.yml generator normalizes its output with oxfmt (the treefmt *.yml formatter); run inside the project mise environment'}
}
let oxfmtrc = ($script_dir | path join '..' '..' '.oxfmtrc.json' | path expand)
let fmt = (^oxfmt --config $oxfmtrc $env.OUT_PATH | complete)
if $fmt.exit_code != 0 {
    error make {msg: $'oxfmt normalization failed for ($env.OUT_PATH): ($fmt.stderr)'}
}
print $'wrote ($env.OUT_PATH)'
