; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/identity/here-formation.gandr
; b3sum: 1cf6f3e2c4b8b9595d51375adb52540940bf2ff58b19aa04f39ce3527d0eb5b8
; lowering: gandr_pipeline::lower::lower_source_total
; items: 4

(v
  (here
    (i 4)))

(v
  (thunk
    gomega
    (abs
      "p"
      none
      (ret
        (var "p")))))

(v
  (annot
    (here
      (i 4))
    (tpath
      (tatom "Integer")
      (i 4)
      (i 4))))

(c
  (app
    (force
      (var "cast"))
    (var "four_eq_four")))
