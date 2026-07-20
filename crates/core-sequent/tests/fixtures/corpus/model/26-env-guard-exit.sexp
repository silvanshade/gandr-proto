; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/26-env-guard-exit.gandr
; b3sum: 82cd26c24ffe3edabd3d46fbc4bd9cb8d9079bcb9c6c5d1b64b74fb6d162bda8
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (perform
      (sig
        "Env"
        (op
          "get"
          (tatom "String")
          (tatom "String"))
        (op
          "path"
          tunit
          (tlist
            (tatom "String"))))
      "get"
      (s "GANDR_CORPUS_UNSET_VARIABLE_ZZQ"))
    "missing"
    (bind
      (force
        (annot
          (thunk
            gomega
            (bind
              (app
                (app
                  (force
                    (var "string.eq"))
                  (var "missing"))
                (s ""))
              "%tmp0"
              (case
                (var "%tmp0")
                "_"
                (ret
                  (i 3))
                "_"
                (ret
                  (i 7)))))
          (tthunk
            gomega
            (ctf
              (tatom "Integer")
              (row)))))
      "code"
      (perform
        (sig
          "Proc"
          (op
            "exit"
            (tatom "Integer")
            tunit))
        "exit"
        (var "code")))))
