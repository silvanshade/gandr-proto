; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/06-sums-and-case.gandr
; b3sum: c0742fefcfc7f3aefd5b92a6e48d0a06610483b5e87fd54f1e70fd73b9590365
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (thunk
    gomega
    (case
      (annot
        (inj
          l
          (i 42))
        (tsum
          (tatom "Integer")
          (tatom "String")))
      "x"
      (bind
        (app
          (app
            (force
              (var "add"))
            (var "x"))
          (i 1))
        "%tmp0"
        (ret
          (var "%tmp0")))
      "s"
      (ret
        (i 0)))))

(c
  (force
    (var "pick")))
