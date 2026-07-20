; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/attributes/doc-and-deprecated.gandr
; b3sum: 04a3c257406eb0b833131f3c32a907a741e220cf06d143610eb21f2361dd2c7c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(v
  (thunk
    gomega
    (abs
      "x"
      none
      (bind
        (app
          (app
            (force
              (var "mul"))
            (var "x"))
          (var "x"))
        "%tmp0"
        (ret
          (var "%tmp0"))))))
