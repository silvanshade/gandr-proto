; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-record.gandr
; b3sum: 5bb80e1eeb004448196513a13e4c03746af33d22a10a79d627f26aeaffc11f8a
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (datacase
      (annot
        (ctor
          (did 0 "Color3")
          0
          (pair
            (annot
              (i 10)
              (tatom "Integer"))
            (pair
              (annot
                (i 20)
                (tatom "Integer"))
              (annot
                (i 30)
                (tatom "Integer")))))
        (tdata
          (did 0 "Color3")))
      (arm
        "%tmp0"
        (split
          (var "%tmp0")
          "r"
          "%tmp1"
          none
          (split
            (var "%tmp1")
            "g"
            "b"
            none
            (ret
              (var "g"))))))))

(c
  (force
    (var "green_channel")))
