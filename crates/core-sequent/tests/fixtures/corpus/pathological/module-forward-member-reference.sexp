; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/module-forward-member-reference.gandr
; b3sum: 7e588b9f8fdb6d8e0635b6892c45f05d23cb48120fe967caa7de020ebbfdaff5
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (app
      (app
        (force
          (var "add"))
        (var "second"))
      (i 1))
    "first"
    (bind
      (ret
        (i 2))
      "second"
      (ret
        (vrec
          ("first"
            (var "first"))
          ("second"
            (var "second")))))))
