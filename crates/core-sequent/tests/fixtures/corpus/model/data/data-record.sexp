; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-record.gandr
; b3sum: bcc6972616fa04b6aab3db061e06cd2397803c46732e15fd1461f38a16715b54
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
