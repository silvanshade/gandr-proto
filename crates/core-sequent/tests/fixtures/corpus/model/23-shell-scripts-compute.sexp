; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/23-shell-scripts-compute.gandr
; b3sum: 3e327de3e651707176b013d451102e0e1c57b95ca5572265a147cb4e7302d6db
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (bind
      (perform
        (sig
          "Exec"
          (op
            "exec"
            (trec
              ("args"
                (tlist
                  (tatom "String")))
              ("mode"
                (tatom "String"))
              ("program"
                (tatom "String")))
            (trec
              ("exit_code"
                (tatom "Integer"))
              ("stderr"
                (tatom "String"))
              ("stdout"
                (tatom "String")))))
        "exec"
        (vrec
          ("args"
            (vlist
              (s "hello from gandr\\n")))
          ("mode"
            (s "captured"))
          ("program"
            (s "printf"))))
      "%tmp0"
      (ret
        (var "%tmp0")))
    "banner"
    (bind
      (recordproj
        (var "banner")
        "stdout")
      "%tmp1"
      (app
        (app
          (force
            (var "string.contains"))
          (var "%tmp1"))
        (s "gandr")))))
