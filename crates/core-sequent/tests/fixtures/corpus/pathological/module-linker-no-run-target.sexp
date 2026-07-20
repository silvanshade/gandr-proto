; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/module-linker-no-run-target.gandr
; b3sum: 75eb0cb968f9a4638840236a64f9a9d9e3972c44f6a069a83f9cf1b99d78f4af
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (ret
      (i 1))
    "x"
    (ret
      (vrec
        ("x"
          (var "x"))))))
