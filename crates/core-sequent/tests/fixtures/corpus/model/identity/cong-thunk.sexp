; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/identity/cong-thunk.gandr
; b3sum: d3800a6f78975036f884af917dffa7298611287471494557111472a007c071d2
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(v
  (thunk
    gomega
    (abs
      "x"
      none
      (bind
        (app
          (app
            (force
              (var "add"))
            (var "x"))
          (var "x"))
        "%tmp0"
        (ret
          (var "%tmp0"))))))

(v
  (thunk
    gomega
    (abs
      "p"
      (some
        (tpath
          (tatom "Integer")
          (i 4)
          (i 4)))
      (walk
        (var "p")
        (wmotive
          "x"
          "y"
          "q"
          (ctf
            (tpath
              tunknown
              (thunk
                gomega
                (app
                  (force
                    (var "double"))
                  (var "x")))
              (thunk
                gomega
                (app
                  (force
                    (var "double"))
                  (var "y"))))
            (row)))
        (wbase
          "x"
          (ret
            (here
              (thunk
                gomega
                (app
                  (force
                    (var "double"))
                  (var "x"))))))))))

(c
  (app
    (force
      (var "cong_double"))
    (here
      (i 4))))
