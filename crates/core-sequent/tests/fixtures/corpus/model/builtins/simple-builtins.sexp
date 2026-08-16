; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/builtins/simple-builtins.gandr
; b3sum: 0b35c535767c6e66753c7c7b449d530f6e7d43c20e9760c7916545cc55bb5acc
; lowering: gandr_pipeline::lower::lower_source_total
; items: 8

(c
  (bind
    (app
      (app
        (force
          (var "lt"))
        (i 1))
      (i 2))
    "%tmp0"
    (app
      (force
        (var "bool.not"))
      (var "%tmp0"))))

(c
  (app
    (app
      (force
        (var "int.div"))
      (i 7))
    (i 2)))

(c
  (bind
    (app
      (force
        (var "neg"))
      (i 7))
    "%tmp1"
    (app
      (app
        (force
          (var "int.mod"))
        (var "%tmp1"))
      (i 2))))

(c
  (app
    (force
      (var "list.length"))
    (vlist
      (i 1)
      (i 2)
      (i 3))))

(c
  (app
    (app
      (force
        (var "list.get"))
      (vlist
        (i 10)
        (i 20)
        (i 30)))
    (i 1)))

(c
  (app
    (app
      (force
        (var "string.append"))
      (s "types are "))
    (s "machines")))

(c
  (app
    (force
      (var "string.length"))
    (var "slogan")))

(c
  (app
    (app
      (force
        (var "string.append"))
      (var "slogan"))
    (s "!")))
