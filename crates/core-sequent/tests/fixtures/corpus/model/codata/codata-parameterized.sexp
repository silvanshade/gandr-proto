; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/codata/codata-parameterized.gandr
; b3sum: d17088ba0271e81ac05bafce63c4eb3a71328933de4a3229c7af8b8c7ac66864
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
