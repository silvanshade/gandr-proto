; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/large-list-append-vs-set.gandr
; b3sum: 79ad415d68a0dd476dfab0d27d6fbe591764ed62bb52a516a14f955cb49bff2c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(v
  (annot
    (vlist
      (i 0)
      (i 1)
      (i 2)
      (i 3)
      (i 4)
      (i 5)
      (i 6)
      (i 7)
      (i 8)
      (i 9))
    (tlist
      (tatom "Integer"))))

(c
  (app
    (app
      (force
        (var "list.append"))
      (var "xs"))
    (vlist
      (i 10)
      (i 11)
      (i 12))))

(c
  (app
    (app
      (app
        (force
          (var "list.set"))
        (var "grown"))
      (i 11))
    (i 99)))
