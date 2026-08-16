; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/builtins/int-division-by-zero.gandr
; b3sum: 8adb38da3092f54863b8042cde349f71ce0d1c0979ea1a6494f13b1f501ca804
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (app
    (app
      (force
        (var "int.div"))
      (i 1))
    (i 0)))
