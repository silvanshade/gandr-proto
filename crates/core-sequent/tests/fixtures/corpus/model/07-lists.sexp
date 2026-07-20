; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/07-lists.gandr
; b3sum: fabb813667d70cadbcb56dbc0104c8a5eb4f7d4e4720bbaa545ee69822ab2d5c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (annot
    (vlist
      (i 1)
      (i 2)
      (i 3))
    (tlist
      (tatom "Integer"))))

(c
  (app
    (app
      (force
        (var "concat"))
      (vlist
        (i 0)))
    (var "xs")))
