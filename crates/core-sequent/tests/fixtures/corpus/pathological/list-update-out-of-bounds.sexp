; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/list-update-out-of-bounds.gandr
; b3sum: 546eea9eee276822af8a014219a7a90a3c0cd69712024a0812783e27dcb34610
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (app
    (app
      (app
        (force
          (var "list.set"))
        (vlist
          (i 1)
          (i 2)
          (i 3)))
      (i 9))
    (i 0)))
