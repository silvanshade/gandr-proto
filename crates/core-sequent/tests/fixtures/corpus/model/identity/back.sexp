; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/identity/back.gandr
; b3sum: 6521c01db912197e6bee50f83929460a40cce442f4ca63f79784ba0c5b343d50
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (here
    (i 7)))

(c
  (walk
    (var "p")
    (wmotive
      "x"
      "y"
      "q"
      (ctf
        (tpath
          (tatom "Integer")
          (var "y")
          (var "x"))
        (row)))
    (wbase
      "x"
      (ret
        (here
          (var "x"))))))
