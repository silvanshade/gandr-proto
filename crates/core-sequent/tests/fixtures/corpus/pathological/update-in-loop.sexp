; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/update-in-loop.gandr
; b3sum: 096b8a1692da17cadc8fb787961f8b8fda6acc623e867a92ae8d8939484846d7
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (annot
    (vlist
      (i 1)
      (i 2)
      (i 3)
      (i 4))
    (tlist
      (tatom "Integer"))))

(c
  (app
    (app
      (app
        (force
          (var "list.reduce"))
        (thunk
          gomega
          (abs
            "acc"
            none
            (abs
              "n"
              none
              (app
                (app
                  (force
                    (var "list.push"))
                  (var "acc"))
                (var "n"))))))
      (vlist))
    (var "steps")))
