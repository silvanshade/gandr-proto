; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/codata/codata-observations.gandr
; b3sum: f47b4842236923a8e0a838573d7d3af958ed091991bf839bce75fe5f80369832
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
