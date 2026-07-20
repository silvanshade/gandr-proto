; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/19-list-update-ops.gandr
; b3sum: 893774541a2872f03be8f0a0a049ab360d67fc22f89b1ab9e46198bf72584df2
; lowering: gandr_pipeline::lower::lower_source_total
; items: 9

(v
  (annot
    (vlist
      (i 10)
      (i 20)
      (i 30))
    (tlist
      (tatom "Integer"))))

(c
  (app
    (app
      (app
        (force
          (var "list.set"))
        (var "xs"))
      (i 1))
    (i 99)))

(c
  (app
    (app
      (app
        (force
          (var "list.update_at"))
        (var "xs"))
      (i 0))
    (thunk
      gomega
      (abs
        "n"
        none
        (app
          (app
            (force
              (var "add"))
            (var "n"))
          (i 1))))))

(c
  (app
    (app
      (app
        (force
          (var "list.insert_at"))
        (var "xs"))
      (i 1))
    (i 15)))

(c
  (app
    (app
      (force
        (var "list.remove_at"))
      (var "xs"))
    (i 0)))

(c
  (app
    (app
      (force
        (var "list.push"))
      (var "xs"))
    (i 40)))

(c
  (app
    (app
      (force
        (var "list.append"))
      (var "xs"))
    (vlist
      (i 40)
      (i 50))))

(c
  (app
    (force
      (var "list.concat"))
    (vlist
      (var "xs")
      (vlist
        (i 40)
        (i 50)))))

(c
  (app
    (app
      (app
        (force
          (var "list.update_where"))
        (thunk
          gomega
          (abs
            "n"
            none
            (app
              (app
                (force
                  (var "gt"))
                (var "n"))
              (i 15)))))
      (thunk
        gomega
        (abs
          "n"
          none
          (app
            (app
              (force
                (var "mul"))
              (var "n"))
            (i 2)))))
    (var "xs")))
