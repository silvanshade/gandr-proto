; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/module-malformed-recovery.gandr
; b3sum: 4efa12efffdee72ee9293f6725e5927b8aa7ef2fa88a3926f904c9e2aa3730fd
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (ret
      (vhole 0))
    "broken"
    (bind
      (ret
        (i 2))
      "ok"
      (ret
        (vrec
          ("broken"
            (var "broken"))
          ("ok"
            (var "ok")))))))
