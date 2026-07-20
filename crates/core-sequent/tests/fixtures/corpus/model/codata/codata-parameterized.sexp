; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/codata/codata-parameterized.gandr
; b3sum: c16cd16cf878f48584c1848bb13072c38e806fecb0d151994e8586ea524e2215
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (abs
      "n"
      none
      (ret
        (vrec
          ("doubled"
            (thunk
              gomega
              (app
                (app
                  (force
                    (var "add"))
                  (var "n"))
                (var "n")))))))))

(c
  (bind
    (app
      (force
        (var "scaled"))
      (i 21))
    "%tmp0"
    (bind
      (recordproj
        (var "%tmp0")
        "doubled")
      "%tmp1"
      (force
        (var "%tmp1")))))
