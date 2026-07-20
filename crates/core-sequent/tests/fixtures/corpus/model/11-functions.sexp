; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/11-functions.gandr
; b3sum: 03f74a3b1d08bb60cdb5829d8fdf593b5a5ae1e07a4727bdc27fdc408e3c44b9
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(v
  (thunk
    gomega
    (abs
      "x"
      none
      (bind
        (app
          (app
            (force
              (var "add"))
            (var "x"))
          (i 1))
        "%tmp0"
        (ret
          (var "%tmp0"))))))

(v
  (thunk
    gomega
    (abs
      "x"
      none
      (bind
        (app
          (force
            (var "inc"))
          (var "x"))
        "y"
        (app
          (force
            (var "inc"))
          (var "y"))))))

(c
  (app
    (force
      (var "twice"))
    (i 40)))
