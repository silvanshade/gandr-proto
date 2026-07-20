; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-maybe.gandr
; b3sum: 977e8565cfd2436f10a717cc5dc8f1d5542ccd9e74d4f26e2fefb12dc0a4ce9c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (datacase
      (annot
        (ctor
          (did 0 "Maybe")
          1
          (annot
            (i 3)
            (tatom "Integer")))
        (tdata
          (did 0 "Maybe")
          (tatom "Integer")))
      (arm
        "_"
        (ret
          (i 0)))
      (arm
        "x"
        (ret
          (var "x"))))))

(c
  (force
    (var "unwrap_or_zero")))
