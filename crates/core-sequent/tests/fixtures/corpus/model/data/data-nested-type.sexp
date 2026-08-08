; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-nested-type.gandr
; b3sum: 20012fdd8d05da33706f1b3ebd8663dee823312a304e40b635a4dc5ee55e24ce
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
