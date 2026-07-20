; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/identity/then.gandr
; b3sum: 61708d2545cdf2708ce3687cad44c77e766d6394d7552ec364cb28de0d5486d0
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(v
  (here
    (i 7)))

(v
  (here
    (i 7)))

(c
  (app
    (walk
      (var "r")
      (wmotive
        "u"
        "v"
        "q"
        (ctarrow
          (tthunk
            (gfin 1)
            (ctf
              (tpath
                (tatom "Integer")
                (i 7)
                (var "u"))
              (row)))
          (ctf
            (tpath
              (tatom "Integer")
              (i 7)
              (var "v"))
            (row))))
      (wbase
        "u"
        (abs
          "w"
          none
          (force
            (var "w")))))
    (thunk
      gomega
      (ret
        (var "p")))))
