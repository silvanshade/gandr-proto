; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/data/non-exhaustive-match.gandr
; b3sum: fc8a36d28e1bec9291549680618c5c9746dcc64bdfc37499c1fed9d12decf37d
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (datacase
      (annot
        (ctor
          (did 0 "Maybe")
          0
          u)
        (tdata
          (did 0 "Maybe")
          (tatom "Integer")))
      (arm
        "_"
        (chole 0))
      (arm
        "x"
        (ret
          (var "x"))))))

(c
  (force
    (var "unwrap")))
