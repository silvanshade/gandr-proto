; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/29-modules.gandr
; b3sum: 5cd4b831042d8cf5ec4b8b3821520b8a2756ee1c002d3c224da2ecf50167aaff
; lowering: gandr_pipeline::lower::lower_source_total
; items: 4

(v
  (i 40))

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
              (s "module member ran\\n")))
          ("mode"
            (s "captured"))
          ("program"
            (s "printf"))))
      "%tmp0"
      (ret
        (var "%tmp0")))
    "reply"
    (bind
      (app
        (app
          (force
            (var "add"))
          (var "seed"))
        (i 1))
      "base"
      (bind
        (app
          (app
            (force
              (var "add"))
            (var "base"))
          (i 1))
        "answer"
        (bind
          (ret
            (vrec
              ("answer"
                (var "answer"))))
          "inner"
          (bind
            (bind
              (recordproj
                (var "inner")
                "answer")
              "%tmp1"
              (app
                (app
                  (force
                    (var "add"))
                  (var "%tmp1"))
                (i 1)))
            "total"
            (ret
              (annot
                (vrec
                  ("answer"
                    (var "answer"))
                  ("base"
                    (var "base"))
                  ("inner"
                    (var "inner"))
                  ("reply"
                    (var "reply"))
                  ("total"
                    (var "total")))
                (trec
                  ("inner"
                    (trec
                      ("answer"
                        (tatom "Integer"))))
                  ("reply"
                    (trec
                      ("exit_code"
                        (tatom "Integer"))
                      ("stderr"
                        (tatom "String"))
                      ("stdout"
                        (tatom "String"))))
                  ("total"
                    (tatom "Integer")))))))))))

(c
  (bind
    (recordproj
      (var "Facts")
      "total")
    "%tmp2"
    (app
      (app
        (force
          (var "add"))
        (var "%tmp2"))
      (i 1))))

(c
  (bind
    (bind
      (recordproj
        (var "Facts")
        "reply")
      "%tmp3"
      (recordproj
        (var "%tmp3")
        "exit_code"))
    "%tmp4"
    (bind
      (bind
        (bind
          (recordproj
            (var "Facts")
            "reply")
          "%tmp5"
          (recordproj
            (var "%tmp5")
            "stdout"))
        "%tmp6"
        (app
          (app
            (force
              (var "string.contains"))
            (var "%tmp6"))
          (s "member ran")))
      "%tmp7"
      (bind
        (recordproj
          (var "Facts")
          "total")
        "%tmp8"
        (ret
          (vrec
            ("after"
              (var "after"))
            ("code"
              (var "%tmp4"))
            ("saw"
              (var "%tmp7"))
            ("total"
              (var "%tmp8"))))))))
