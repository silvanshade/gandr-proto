; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-enum.gandr
; b3sum: 7fccf2e13e71c29b1eae7448df9eed773a0660f870b3b02ba53573dd4d45f8ae
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (datacase
      (annot
        (ctor
          (did 0 "Color")
          1
          u)
        (tdata
          (did 0 "Color")))
      (arm
        "_"
        (ret
          (i 0)))
      (arm
        "_"
        (ret
          (i 1)))
      (arm
        "_"
        (ret
          (i 2))))))

(c
  (force
    (var "rank")))
