; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/wide-record-update.gandr
; b3sum: e3305117aade1bfb6f082e02fdc73dd939eb8a9373e542b038ac361810595eec
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("a"
      (i 1))
    ("b"
      (i 2))
    ("c"
      (i 3))
    ("d"
      (i 4))
    ("e"
      (i 5))
    ("f"
      (i 6))
    ("g"
      (i 7))
    ("h"
      (i 8))))

(c
  (app
    (app
      (native recordupdate)
      (var "wide"))
    (vrec
      ("e"
        (i 99)))))
