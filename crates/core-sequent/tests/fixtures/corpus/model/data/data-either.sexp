; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-either.gandr
; b3sum: 658144b2e4d27bc81a050e19aae58dbfdbe11ee5f795b73dc598fac7ae21795d
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (datacase
      (annot
        (ctor
          (did 0 "Either")
          0
          (annot
            (i 5)
            (tatom "Integer")))
        (tdata
          (did 0 "Either")
          (tatom "Integer")
          (tatom "String")))
      (arm
        "x"
        (ret
          (var "x")))
      (arm
        "y"
        (ret
          (i 0))))))

(c
  (force
    (var "from_left")))
