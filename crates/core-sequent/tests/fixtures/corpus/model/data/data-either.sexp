; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-either.gandr
; b3sum: a13639fd4c91018a74e8edc296347a4710e347346d08cd39c8598cd431d90665
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
