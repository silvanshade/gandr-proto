; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/shell-host-escape-non-string.gandr
; b3sum: efac236c752b3f4d9beadd64d0e0634360568cefc84e63ac1e930877a6c7b287
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (bind
      (ret
        (annot
          (i 1)
          (tatom "String")))
      "%tmp0"
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
              (s "%s")
              (annot
                (var "%tmp0")
                (tatom "String"))))
          ("mode"
            (s "captured"))
          ("program"
            (s "printf")))))
    "%tmp1"
    (ret
      (var "%tmp1"))))
