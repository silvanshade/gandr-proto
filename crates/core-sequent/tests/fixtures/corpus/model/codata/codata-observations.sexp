; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/codata/codata-observations.gandr
; b3sum: 4a283ac4759f7dae0c50de0680a5536cec3092d739963444abc3d97e72ea64bf
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("x"
      (thunk
        gomega
        (ret
          (i 3))))
    ("y"
      (thunk
        gomega
        (ret
          (i 4))))))

(c
  (bind
    (bind
      (recordproj
        (var "origin")
        "x")
      "%tmp0"
      (force
        (var "%tmp0")))
    "%tmp1"
    (bind
      (bind
        (recordproj
          (var "origin")
          "y")
        "%tmp2"
        (force
          (var "%tmp2")))
      "%tmp3"
      (app
        (app
          (force
            (var "add"))
          (var "%tmp1"))
        (var "%tmp3")))))
