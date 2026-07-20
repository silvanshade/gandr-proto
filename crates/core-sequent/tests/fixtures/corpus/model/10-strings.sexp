; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/10-strings.gandr
; b3sum: 805ff393350807b7e5e3536f835b70fdcda1b538aaefcb8cd1833cee8d80294b
; lowering: gandr_pipeline::lower::lower_source_total
; items: 7

(v
  (s "types are machines"))

(c
  (app
    (app
      (force
        (var "string.contains"))
      (var "slogan"))
    (s "machine")))

(c
  (app
    (app
      (force
        (var "string.eq"))
      (var "slogan"))
    (s "types are machines")))

(c
  (app
    (app
      (force
        (var "string.starts_with"))
      (var "slogan"))
    (s "types")))

(c
  (app
    (app
      (force
        (var "string.ends_with"))
      (var "slogan"))
    (s "machines")))

(c
  (app
    (force
      (var "string.escape"))
    (s "a+b.txt")))

(c
  (app
    (app
      (force
        (var "string.split"))
      (s "alpha,beta"))
    (s ",")))
