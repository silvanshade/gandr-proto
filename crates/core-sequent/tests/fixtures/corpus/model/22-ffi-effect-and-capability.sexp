; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/22-ffi-effect-and-capability.gandr
; b3sum: 9780dc87582edf7ad5780d7d1878eb6a4ba4d4651a3f48a6c6231d55dd6a6854
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (perform
    (sig
      "sensor"
      (op
        "read"
        (trec
          ("channel"
            (tatom "i32")))
        (tatom "i64")))
    "read"
    (vrec
      ("channel"
        (n
          (i32 0))))))
