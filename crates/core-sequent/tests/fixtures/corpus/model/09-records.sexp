; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/09-records.gandr
; b3sum: 858645eac2b8e2ab9a570ead54ddf013e02efab3b0c6dc8d0b593d5702db937c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(v
  (vrec
    ("name"
      (s "gandr"))
    ("stars"
      (i 1))))

(c
  (app
    (app
      (app
        (force
          (var "record.insert"))
        (var "base"))
      (s "kind"))
    (s "language")))

(c
  (app
    (app
      (force
        (var "record.get"))
      (var "extended"))
    (s "kind")))
