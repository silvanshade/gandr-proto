; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/module-signature-mismatch.gandr
; b3sum: 96e369df38c5a5bd8e8cf69ba7787eb68589b0097f7d3e6aeed50e24b05de256
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (ret
      (i 1))
    "x"
    (ret
      (annot
        (vrec
          ("x"
            (var "x")))
        (trec
          ("x"
            (tatom "String")))))))
