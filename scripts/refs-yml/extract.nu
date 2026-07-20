# Extract the bibliography-v2 register table rows into a structured table.
# Faithful mechanical extraction only (no lossy parsing): splits the markdown
# table on `|` and trims. Column order in the register:
#   Id | Citation | Feature area | Sites & holdings | Locator | Prov

def extract-register [path: string]: nothing -> table {
    let lines = (open --raw $path | decode utf-8 | lines)
    mut section = ""
    mut rows = []
    for line in $lines {
        let sec_match = ($line | parse --regex '^## (?<s>[A-V])\. ')
        if ($sec_match | is-not-empty) {
            $section = ($sec_match | get s.0)
            continue
        }
        if ($line =~ '^\| [A-V]-[0-9]') {
            let cells = ($line | split row '|' | each {|c| $c | str trim })
            # cells.0 is leading empty, then 6 columns, then trailing empty
            $rows = ($rows | append {
                section: $section
                id: ($cells | get 1)
                citation: ($cells | get 2)
                feature: ($cells | get 3)
                sites: ($cells | get 4)
                locator: ($cells | get 5)
                prov: ($cells | get 6)
            })
        }
    }
    $rows
}

extract-register $env.REG_PATH | to nuon | save -f $env.OUT_PATH
