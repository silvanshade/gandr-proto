; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/15-record-update.gandr
; b3sum: c303a2628cf3fdfc19c743d7d9ceb3bcccf808a33ddf6c00ef70dc477ae49bb0
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("x"
      (i 1))
    ("y"
      (i 2))))

(c
  (app
    (app
      (native recordupdate)
      (var "point"))
    (vrec
      ("x"
        (i 9)))))
