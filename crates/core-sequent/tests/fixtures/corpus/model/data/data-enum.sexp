; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-enum.gandr
; b3sum: 1b3e6c0af08c3d6ee8e3d9494cb8074e12c1aab639343dbbab68433589ca8af3
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
