; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/24-shell-conditionals.gandr
; b3sum: 4910148f79912ae9b682cbfdbf6dabe66ceb03ad05b5593395fc7fc1d3a00fbf
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
              (s "-d")
              (s "/tmp")))
          ("mode"
            (s "captured"))
          ("program"
            (s "test"))))
      "%tmp0"
      (ret
        (var "%tmp0")))
    "probe"
    (force
      (annot
        (thunk
          gomega
          (bind
            (bind
              (recordproj
                (var "probe")
                "exit_code")
              "%tmp1"
              (app
                (app
                  (force
                    (var "eq"))
                  (var "%tmp1"))
                (i 0)))
            "%tmp2"
            (case
              (var "%tmp2")
              "_"
              (ret
                (s "present"))
              "_"
              (ret
                (s "absent")))))
        (tthunk
          gomega
          (ctf
            (tatom "String")
            (row)))))))
