; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/identity/k-derivation.gandr
; b3sum: 68f1092fd32a8fef36d25b8feefb84ce28626f76435b675cd1891fa4b58eec11
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(v
  (thunk
    gomega
    (abs
      "n"
      none
      (abs
        "p"
        none
        (abs
          "q"
          none
          (case
            (var "p")
            "_"
            (chole 0)
            "_"
            (chole 1)))))))
