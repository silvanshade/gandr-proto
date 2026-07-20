; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/shared-then-updated.gandr
; b3sum: f9666ed0427e96e49cf458e65c50b031b558c768a8d2bea79f4b5f2e29b60bf9
; lowering: gandr_pipeline::lower::lower_source_total
; items: 5

(v
  (vrec
    ("x"
      (i 1))
    ("y"
      (i 2))))

(v
  (var "r"))

(v
  (var "r"))

(c
  (app
    (app
      (native recordupdate)
      (var "r"))
    (vrec
      ("x"
        (i 99)))))

(v
  (var "a"))
