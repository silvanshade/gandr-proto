; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/module-duplicate-member.gandr
; b3sum: 2cdecd37e82cfe464474d4e66ee501382c146ac612a96f11e046f911238a394a
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (ret
      (i 1))
    "x"
    (bind
      (chole 0)
      "_"
      (ret
        (vrec
          ("x"
            (var "x")))))))
