; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/identity/cong-value.gandr
; b3sum: 8c34000dba27ddf7f4f7c584633bd898596a0ea0272a972244830f6eb643bca5
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

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
              (vlist
                (var "x"))
              (vlist
                (var "y")))
            (row)))
        (wbase
          "x"
          (ret
            (here
              (annot
                (vlist
                  (var "x"))
                (tlist
                  (tatom "Integer"))))))))))

(c
  (app
    (force
      (var "cong_list"))
    (here
      (i 4))))
