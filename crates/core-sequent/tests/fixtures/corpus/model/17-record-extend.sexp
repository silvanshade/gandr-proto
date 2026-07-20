; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/17-record-extend.gandr
; b3sum: d486760be2bbc9a8a51b6b9be8a6a7b5b23a9fe3c8d7b034362e97b3da0136a3
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("a"
      (i 1))))

(c
  (app
    (app
      (native recordupdate)
      (var "base"))
    (vrec
      ("b"
        (i 2)))))
