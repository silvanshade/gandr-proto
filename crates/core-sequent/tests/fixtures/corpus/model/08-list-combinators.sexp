; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/08-list-combinators.gandr
; b3sum: 879aa43d0fbc1b80899f125a0390b9b778fe561bea06aa385bcda6efcd96a951
; lowering: gandr_pipeline::lower::lower_source_total
; items: 7

(c
  (app
    (app
      (force
        (var "list.each"))
      (thunk
        gomega
        (abs
          "x"
          none
          (app
            (app
              (force
                (var "mul"))
              (var "x"))
            (i 2)))))
    (vlist
      (i 1)
      (i 2)
      (i 3))))

(c
  (app
    (app
      (force
        (var "list.where"))
      (thunk
        gomega
        (abs
          "x"
          none
          (app
            (app
              (force
                (var "gt"))
              (var "x"))
            (i 2)))))
    (var "doubled")))

(c
  (app
    (app
      (app
        (force
          (var "list.reduce"))
        (thunk
          gomega
          (abs
            "acc"
            none
            (abs
              "x"
              none
              (app
                (app
                  (force
                    (var "add"))
                  (var "acc"))
                (var "x"))))))
      (i 0))
    (var "big")))

(c
  (app
    (app
      (force
        (var "list.any"))
      (thunk
        gomega
        (abs
          "x"
          none
          (app
            (app
              (force
                (var "gt"))
              (var "x"))
            (i 5)))))
    (var "big")))

(c
  (app
    (app
      (force
        (var "list.all"))
      (thunk
        gomega
        (abs
          "x"
          none
          (app
            (app
              (force
                (var "gt"))
              (var "x"))
            (i 1)))))
    (var "doubled")))

(c
  (app
    (force
      (var "list.flatten"))
    (vlist
      (vlist
        (i 3)
        (i 1))
      (vlist
        (i 2)
        (i 2)))))

(c
  (bind
    (app
      (force
        (var "list.uniq"))
      (var "flattened"))
    "%tmp0"
    (app
      (force
        (var "list.sort"))
      (var "%tmp0"))))
