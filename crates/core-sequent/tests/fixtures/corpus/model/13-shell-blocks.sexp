; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/13-shell-blocks.gandr
; b3sum: eba5dac387418b279866bda310df45b3af32c5f14918d4ebad1e4c7d6af48ee8
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (bind
      (ret
        (annot
          (s "one argv with spaces")
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
              (s "hello from the host\\n<%s>\\n")
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
