; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/20-value-semantics-no-aliasing.gandr
; b3sum: b33fef56abee568774c970d1451db930434e6a24c23a25795a8fd49b0776dd7c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

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
      (var "r"))
    (vrec
      ("x"
        (i 9)))))

(v
  (var "r"))
