; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/18-nested-record-update.gandr
; b3sum: 3a7864fe5ba83b88e39512fd09e6259ed4fafd3f1754600aaebd6900630df052
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("inner"
      (vrec
        ("x"
          (i 1))
        ("y"
          (i 2))))
    ("tag"
      (s "t"))))

(c
  (bind
    (bind
      (recordproj
        (var "outer")
        "inner")
      "%tmp0"
      (app
        (app
          (native recordupdate)
          (var "%tmp0"))
        (vrec
          ("y"
            (i 9)))))
    "%tmp1"
    (app
      (app
        (native recordupdate)
        (var "outer"))
      (vrec
        ("inner"
          (var "%tmp1"))))))
