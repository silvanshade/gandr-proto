; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/16-record-update-field-retype.gandr
; b3sum: 7e3741478dc48e09b52fc93d8275b14b4e093159b8d169b0e025cf2a0e664a1b
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("count"
      (i 3))
    ("label"
      (s "start"))))

(c
  (app
    (app
      (native recordupdate)
      (var "cell"))
    (vrec
      ("count"
        (s "three")))))
