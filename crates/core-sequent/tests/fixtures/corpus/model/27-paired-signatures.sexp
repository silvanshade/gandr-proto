; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/27-paired-signatures.gandr
; b3sum: 70f8462162f7a0c03c04e7aa199fbe10ec2bcd8ed8ca7ba72475082b9a248e1b
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(c
  (bind
    (app
      (force
        (var "prim.id"))
      (i 999))
    "%tmp0"
    (app
      (app
        (force
          (var "prim.const"))
        (i 40))
      (var "%tmp0"))))

(v
  (thunk
    gomega
    (abs
      "x"
      none
      (bind
        (app
          (app
            (force
              (var "add"))
            (var "x"))
          (i 2))
        "%tmp1"
        (ret
          (var "%tmp1"))))))

(c
  (app
    (force
      (var "plus_two"))
    (var "seed")))
