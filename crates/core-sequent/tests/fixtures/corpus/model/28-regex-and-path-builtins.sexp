; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/28-regex-and-path-builtins.gandr
; b3sum: 203f9134ee9c625e5e9adffa6c995fd58ebe5e470050f64a3f5a5d53914bbdb5
; lowering: gandr_pipeline::lower::lower_source_total
; items: 5

(c
  (app
    (app
      (recordproj
        (var "regex")
        "extract")
      (s "^(?<stem>.*)\\.(?<ext>gandr)$"))
    (s "examples/demo.gandr")))

(c
  (app
    (app
      (force
        (var "path.join"))
      (s "examples"))
    (s "demo.gandr")))

(c
  (app
    (force
      (var "path.basename"))
    (var "joined")))

(c
  (app
    (force
      (var "path.extension"))
    (var "joined")))

(v
  (vrec
    ("capture"
      (var "caps"))
    ("leaf"
      (var "leaf"))
    ("path_ext"
      (var "suffix"))))
