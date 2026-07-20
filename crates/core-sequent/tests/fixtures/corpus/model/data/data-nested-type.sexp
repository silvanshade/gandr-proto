; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-nested-type.gandr
; b3sum: 296ec90103c47d1ab3927a9c37550ac8a3d18e3c6748218b1780e07573876fee
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (bind
      (ret
        (annot
          (vlist
            (ctor
              (did 0 "Maybe")
              1
              (i 1))
            (ctor
              (did 0 "Maybe")
              0
              u)
            (ctor
              (did 0 "Maybe")
              1
              (i 3)))
          (tlist
            (tdata
              (did 0 "Maybe")
              (tatom "Integer")))))
      "items"
      (ret
        (i 7)))))

(c
  (force
    (var "sum_first")))
