; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/data/non-exhaustive-match.gandr
; b3sum: ad82b5c956112ddb4c3dae71729b2b9131ee2d4d14b98c31106e36f35352e9c3
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
