; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/data/data-maybe.gandr
; b3sum: 520c548f1b3afacb6ca05921f64bf34686510f575d1ec5fba504013601399e84
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
